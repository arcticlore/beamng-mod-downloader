import { describe, it, expect, beforeEach } from "vitest";
import { createElement, type ReactElement } from "react";
import TestRenderer, { act } from "react-test-renderer";
import type { ReactTestRenderer } from "react-test-renderer";
import { stubTauriIpc } from "./tauri-stub";
import { LanguageProvider } from "../../src/i18n/LanguageContext";
import { SourcesProvider } from "../../src/SourcesContext";
import { LinkImportModal } from "../../src/components/LinkImportModal";
import type { SourceDescriptor, SourceSelection } from "../../src/types";

/**
 * Покрытие окна «Добавить мод» (связка ссылка/файл).
 *
 * Без сети: классифицирует вставленную ссылку (forum/direct), показывает
 * disabled-предупреждения по включённости источников, валидирует пустой ввод,
 * дёргает install_from_url / import_local_zip / plugin:dialog|open через IPC-stub
 * и завершает окно по onClose + onToast.
 */

function regd(over: Partial<SourceDescriptor>): SourceDescriptor {
  return {
    id: "x",
    label: "X",
    group: "community",
    trustLevel: "third_party",
    enabledByDefault: false,
    legacyDefault: false,
    homepage: "https://example.com",
    termsOrPolicyUrl: null,
    warning: null,
    installMode: "mods_zip",
    auth: "none",
    status: "ready",
    filenameRule: "key_stem",
    capabilities: {
      search: true,
      categories: true,
      pagination: true,
      detail: true,
      directZipDownload: true,
      manualDownload: false,
      checksums: false,
      updateDetection: true,
    },
    categories: [],
    ...over,
  };
}

const REGISTRY: SourceDescriptor[] = [
  regd({
    id: "beamngweb",
    label: "Официальный сайт BeamNG",
    group: "official",
    trustLevel: "official",
    enabledByDefault: true,
    filenameRule: "basename",
  }),
  regd({
    id: "beamngforum",
    label: "Форум BeamNG",
    group: "official",
    trustLevel: "official",
    installMode: "mods_zip",
    filenameRule: "basename",
    capabilities: {
      search: false,
      categories: false,
      pagination: false,
      detail: true,
      directZipDownload: true,
      manualDownload: false,
      checksums: false,
      updateDetection: true,
    },
  }),
  regd({
    id: "directurl",
    label: "Прямая ссылка",
    group: "custom",
    trustLevel: "custom",
    installMode: "mods_zip",
    filenameRule: "key_stem",
    capabilities: {
      search: false,
      categories: false,
      pagination: false,
      detail: true,
      directZipDownload: true,
      manualDownload: false,
      checksums: false,
      updateDetection: true,
    },
  }),
];

let toasts: string[];
let closes: number;
let invokes: [string, Record<string, unknown>][];

beforeEach(() => {
  toasts = [];
  closes = 0;
  invokes = [];
  stubTauriIpc((cmd, args) => {
    invokes.push([cmd, args]);
    switch (cmd) {
      case "get_source_registry":
        return REGISTRY;
      case "get_source_selection":
        return { enabled: ["beamngweb"], selected: null } as SourceSelection;
      case "install_from_url":
        return `dl_${args.url}`;
      case "import_local_zip":
        return args.sourcePath.split("/").pop();
      case "plugin:dialog|open":
        return args === undefined ? "/home/user/x.zip" : "/home/user/from_dialog.zip";
      default:
        return undefined;
    }
  });
});

const wrap = (el: ReactElement) =>
  createElement(
    LanguageProvider,
    { lang: "ru" },
    createElement(SourcesProvider, null, el),
  );

async function flush(n = 3) {
  await act(async () => {
    for (let i = 0; i < n; i++) await new Promise((r) => setTimeout(r, 0));
  });
}

async function mount(variant: "classic" | "material") {
  let root: ReactTestRenderer | null = null;
  await act(async () => {
    root = TestRenderer.create(
      wrap(
        createElement(LinkImportModal, {
          variant,
          onToast: (s) => toasts.push(s),
          onClose: () => closes++,
        }),
      ),
    );
  });
  await flush();
  return root!;
}

const text = (root: ReactTestRenderer) => JSON.stringify(root.toJSON());

function buttonByText(root: ReactTestRenderer, label: string) {
  return root.root
    .findAll((n) => {
      if (typeof n.type !== "string" || n.type !== "button") return false;
      const kids = n.children ?? [];
      const s = kids
        .map((c) => (typeof c === "string" ? c : String(c)))
        .join(" ");
      return s.includes(label);
    })
    .at(-1);
}

async function typeInto(root: ReactTestRenderer, value: string) {
  const input = root.root.find((n) => n.type === "input");
  await act(async () => {
    input.props.onChange({ target: { value } });
  });
}

async function click(root: ReactTestRenderer, label: string) {
  const btn = buttonByText(root, label);
  expect(btn).toBeTruthy();
  await act(async () => {
    btn!.props.onClick();
  });
  await flush(1);
}

describe("LinkImportModal (classic)", () => {
  it("renders title, tabs and url tab by default", async () => {
    const root = await mount("classic");
    const out = text(root);
    expect(out).toContain("Добавить мод из ссылки или файла");
    expect(out).toContain("По ссылке");
    expect(out).toContain("Из файла .zip");
    expect(out).toContain("Ссылка на архив мода");
  });

  it("classifies a forum attachment link and shows its hint", async () => {
    const root = await mount("classic");
    await typeInto(root, "https://www.beamng.com/attachments/433264/");
    const out = text(root);
    expect(out).toContain("Вложение BeamNG Forum");
  });

  it("warns when the forum source is disabled", async () => {
    const root = await mount("classic");
    await typeInto(root, "https://www.beamng.com/attachments/433264/");
    const out = text(root);
    expect(out).toContain("Источник «Форум BeamNG» выключен в настройках");
  });

  it("warns when the direct link source is disabled for a direct url", async () => {
    const root = await mount("classic");
    await typeInto(root, "https://github.com/o/r/release.zip");
    const out = text(root);
    expect(out).toContain("Прямая ссылка» выключен в настройках");
    expect(out).toContain("Прямая ссылка на архив .zip");
  });

  it("shows the forum-title hint for bare ids", async () => {
    const root = await mount("classic");
    await typeInto(root, "433264");
    const out = text(root);
    expect(out).toContain("Вложение BeamNG Forum");
    expect(out).toContain("Открыть форум в браузере");
  });

  it("does not submit an empty url and shows the validation error", async () => {
    const root = await mount("classic");
    await click(root, "Установить по ссылке");
    expect(text(root)).toContain("Вставьте ссылку на архив мода");
    expect(invokes.filter(([c]) => c === "install_from_url")).toHaveLength(0);
    expect(closes).toBe(0);
  });

  it("submits a direct url to install_from_url, toasts and closes", async () => {
    const root = await mount("classic");
    await typeInto(root, "https://github.com/o/r/release.zip");
    await click(root, "Установить по ссылке");
    expect(invokes.filter(([c]) => c === "install_from_url")).toHaveLength(1);
    expect(invokes.find(([c]) => c === "install_from_url")![1]).toEqual({
      url: "https://github.com/o/r/release.zip",
    });
    expect(toasts).toContain("Установка началась: https://github.com/o/r/release.zip");
    expect(closes).toBe(1);
  });

  it("imports a picked zip via import_local_zip and toasts", async () => {
    const root = await mount("classic");
    await click(root, "Из файла .zip");
    await click(root, "Выбрать файл…");
    await flush(1);
    expect(text(root)).toContain("Выбран: from_dialog.zip");
    expect(invokes.filter(([c]) => c === "plugin:dialog|open")).toHaveLength(1);
    await click(root, "Импортировать в папку модов");
    expect(invokes.find(([c]) => c === "import_local_zip")![1]).toEqual({
      sourcePath: "/home/user/from_dialog.zip",
    });
    expect(toasts).toContain("Архив импортирован: from_dialog.zip");
    expect(closes).toBe(1);
  });
});

describe("LinkImportModal (material)", () => {
  it("renders a material dialog and closes via the header button", async () => {
    const root = await mount("material");
    const out = text(root);
    expect(out).toContain("m3-dialog");
    expect(out).toContain("Добавить мод из ссылки или файла");
    const closeBtn = root.root.find(
      (n) => typeof n.type === "string" && n.type === "button" && n.props["aria-label"] === "Закрыть",
    );
    expect(closeBtn).toBeTruthy();
    await act(async () => {
      closeBtn!.props.onClick();
    });
    expect(closes).toBe(1);
  });
});
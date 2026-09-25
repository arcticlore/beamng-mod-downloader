import { describe, it, expect, beforeEach } from "vitest";
import { createElement, type ReactElement } from "react";
import TestRenderer, { act } from "react-test-renderer";
import type { ReactTestRenderer } from "react-test-renderer";
import { stubTauriIpc } from "./tauri-stub";
import { LanguageProvider } from "../../src/i18n/LanguageContext";
import { InstalledPanel } from "../../src/components/InstalledPanel";
import { MaterialInstalled } from "../../src/components/material/MaterialInstalled";
import type { InstalledMod, IntegrityReport, ModUpdate } from "../../src/types";

/**
 * Регрессионное покрытие переходов состояний экрана «Установленные».
 *
 * Регистрирует баг: useMemo/подсчёт, размещённые ПОСЛЕ ранних `return` в
 * InstalledPanel и MaterialInstalled, давали React error #310
 * ("Rendered more hooks than during the previous render") на переходе
 * loading -> populated внутри одного смонтированного экземпляра — в
 * production уронило бы экран. Тест проходит переходы
 * null(loading) -> error -> populated и populated(re-sort) без исключений.
 */

let installed: InstalledMod[];
let updates: ModUpdate[];
let failList: boolean;

beforeEach(() => {
  installed = [
    {
      filename: "car_a.zip",
      path: "/home/user/BeamNG.drive/mods/car_a.zip",
      sizeBytes: 1_000,
      modified: 1_700_000_000,
      source: "",
      key: null,
      published: null,
    },
    {
      filename: "car_b.zip",
      path: "/home/user/BeamNG.drive/mods/car_b.zip",
      sizeBytes: 2_000,
      modified: 1_730_000_000,
      source: "repo",
      key: "car_b",
      published: "2026-07-01",
    },
    {
      filename: "car_c.zip",
      path: "/home/user/BeamNG.drive/mods/car_c.zip",
      sizeBytes: 3_000,
      modified: 1_770_000_000,
      source: "repo",
      key: "car_c",
      published: "2026-08-01",
    },
  ];
  updates = [
    {
      filename: "car_b.zip",
      source: "repo",
      key: "car_b",
      installedPublished: "2026-07-01",
      latestPublished: "2026-09-10",
      hasUpdate: true,
      error: null,
    },
    {
      filename: "car_c.zip",
      source: "repo",
      key: "car_c",
      installedPublished: "2026-08-01",
      latestPublished: "2026-08-01",
      hasUpdate: false,
      error: null,
    },
  ];
  failList = false;
  stubTauriIpc((cmd) => {
    switch (cmd) {
      case "list_installed":
        if (failList) throw new Error("list boom");
        return installed;
      case "check_updates":
        return updates;
      case "verify_installed":
        return [] as IntegrityReport[];
      default:
        return undefined;
    }
  });
});

const wrap = (el: ReactElement) =>
  createElement(LanguageProvider, { lang: "ru" }, el);

async function flush(n = 3) {
  await act(async () => {
    for (let i = 0; i < n; i++) await new Promise((r) => setTimeout(r, 0));
  });
}

async function mount(el: ReactElement, flushNow = true) {
  let root: ReactTestRenderer | null = null;
  if (flushNow) {
    await act(async () => {
      root = TestRenderer.create(wrap(el));
    });
    await flush();
  } else {
    act(() => {
      root = TestRenderer.create(wrap(el));
    });
  }
  return root!;
}

const text = (root: ReactTestRenderer) => JSON.stringify(root.toJSON());
const pos = (out: string, needle: string) => out.indexOf(needle);

describe("Installed state transitions (Classic)", () => {
  it("renders loading first, then populated rows; date sort desc", async () => {
    const root = await mount(
      createElement(InstalledPanel, {
        settings: { installedSort: "date" },
        onOpenSettings: () => {},
      }),
      false,
    );
    expect(text(root)).not.toContain("car_");
    expect(text(root)).toContain("browser-loading");

    await flush();
    const out = text(root);
    expect(pos(out, "car_c.zip")).toBeGreaterThan(-1);
    expect(pos(out, "car_b.zip")).toBeGreaterThan(-1);
    expect(pos(out, "car_a.zip")).toBeGreaterThan(-1);
    expect(pos(out, "car_c.zip")).toBeLessThan(pos(out, "car_b.zip"));
    expect(pos(out, "car_b.zip")).toBeLessThan(pos(out, "car_a.zip"));
  });

  it("sorts by name on re-render with a different setting (hook count stable)", async () => {
    const root = await mount(
      createElement(InstalledPanel, {
        settings: { installedSort: "date" },
        onOpenSettings: () => {},
      }),
    );
    await flush();
    await act(async () => {
      root.update(
        wrap(
          createElement(InstalledPanel, {
            settings: { installedSort: "name" },
            onOpenSettings: () => {},
          }),
        ),
      );
    });
    const out = text(root);
    expect(pos(out, "car_a.zip")).toBeLessThan(pos(out, "car_b.zip"));
    expect(pos(out, "car_b.zip")).toBeLessThan(pos(out, "car_c.zip"));
  });

  it("shows updates: has_update badge + update button for managed mod", async () => {
    const root = await mount(
      createElement(InstalledPanel, {
        settings: { installedSort: "date" },
        onOpenSettings: () => {},
      }),
    );
    await flush();
    const out = text(root);
    expect(out).toContain("Есть обновление");
    expect(out).toContain("Обновить");
    expect(out).toContain("репо");
  });

  it("recovers into error state when list_installed rejects", async () => {
    failList = true;
    const root = await mount(
      createElement(InstalledPanel, {
        settings: { installedSort: "date" },
        onOpenSettings: () => {},
      }),
    );
    const out = text(root);
    expect(out).toContain("list boom");
    expect(out).toContain("panel-empty");
  });
});

describe("Installed state transitions (Material)", () => {
  it("renders loading, then populated rows with update badge", async () => {
    const root = await mount(
      createElement(MaterialInstalled, {
        settings: { installedSort: "date" },
        onOpenSettings: () => {},
      }),
      false,
    );
    expect(text(root)).toContain("m3-loading");
    expect(text(root)).toContain("m3-spinner");
    await flush();
    const out = text(root);
    expect(pos(out, "car_c.zip")).toBeGreaterThan(-1);
    expect(pos(out, "car_b.zip")).toBeGreaterThan(-1);
    expect(pos(out, "car_a.zip")).toBeGreaterThan(-1);
    expect(pos(out, "car_c.zip")).toBeLessThan(pos(out, "car_b.zip"));
    expect(pos(out, "car_b.zip")).toBeLessThan(pos(out, "car_a.zip"));
    expect(out).toContain("Есть обновление");
    expect(out).toContain("Обновить");
  });

  it("recovers into error empty-state when list_installed rejects", async () => {
    failList = true;
    const root = await mount(
      createElement(MaterialInstalled, {
        settings: { installedSort: "date" },
        onOpenSettings: () => {},
      }),
    );
    const out = text(root);
    expect(out).toContain("list boom");
    expect(out).toContain("m3-empty");
  });
});
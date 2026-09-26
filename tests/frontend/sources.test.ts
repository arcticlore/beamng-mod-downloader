import { test } from "node:test";
import assert from "node:assert/strict";
import type { SourceDescriptor, ModItem } from "../../src/types.ts";
import {
  applyPreset,
  canonicalizeSelection,
  dedupById,
  filenameFor,
  filterSources,
  resolveActiveSources,
  searchCapableEnabledIds,
  sortByGroup,
  toggleSourceSelection,
  urlHost,
} from "../../src/sources.ts";

function regd(over: Partial<SourceDescriptor>): SourceDescriptor {
  return {
    id: "x",
    label: "X",
    group: "community",
    trustLevel: "third_party",
    enabledByDefault: false,
    legacyDefault: true,
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

/** Портрет реального registry (без правки состояния backend). */
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
    id: "github",
    label: "GitHub-релизы",
    group: "forges",
    trustLevel: "verified_forge",
    enabledByDefault: true,
    filenameRule: "owner_repo",
  }),
  regd({
    id: "worldofmods",
    label: "WorldOfMods",
    group: "community",
    enabledByDefault: false,
    filenameRule: "html_slug",
  }),
  regd({
    id: "beamngforum",
    label: "Форум BeamNG",
    group: "official",
    trustLevel: "official",
    enabledByDefault: false,
    legacyDefault: false,
    homepage: "https://www.beamng.com/community/",
    termsOrPolicyUrl: "https://www.beamng.com/help/terms-of-service/",
    warning: "search requires sign-in",
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
    legacyDefault: false,
    warning: "arbitrary archive",
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

const ALL = REGISTRY.map((d) => d.id);
const ENABLED = ["beamngweb", "github", "worldofmods"];
// search-capable включённые в порядке registry: без beamngforum (no-search).
const SEARCH_CAPABLE_ENABLED = ["beamngweb", "github", "worldofmods"];

test("resolveActiveSources: selected=null означает все enabled search-capable", () => {
  assert.deepEqual(resolveActiveSources(REGISTRY, ENABLED, null), [
    "beamngweb",
    "github",
    "worldofmods",
  ]);
  // enabled + manual/no-search не становится searchable:
  assert.deepEqual(
    resolveActiveSources(REGISTRY, [...ENABLED, "beamngforum"], null),
    SEARCH_CAPABLE_ENABLED,
  );
});

test("resolveActiveSources: явный выбор пересекается с enabled и упорядочен по registry", () => {
  assert.deepEqual(
    resolveActiveSources(REGISTRY, ENABLED, ["worldofmods", "beamngweb"]),
    ["beamngweb", "worldofmods"],
  );
  // disabled источник в selected не попадает — он никогда не запрашивается.
  assert.deepEqual(
    resolveActiveSources(REGISTRY, ENABLED, [ALL[0], ...ALL]),
    SEARCH_CAPABLE_ENABLED,
  );
  assert.deepEqual(resolveActiveSources(REGISTRY, ["github"], ["beamngweb"]), []);
  assert.deepEqual(resolveActiveSources(REGISTRY, ENABLED, []), []);
  // stale selected / disabled / no-search id не становятся active:
  assert.deepEqual(resolveActiveSources(REGISTRY, ENABLED, ["ghost", "beamngforum"]), []);
  assert.deepEqual(
    resolveActiveSources(REGISTRY, ENABLED, ["ghost", "beamngweb"]),
    ["beamngweb"],
  );
});

test("applyPreset: recommended — рекомендуемые из enabled", () => {
  assert.deepEqual(applyPreset("recommended", REGISTRY, ENABLED), [
    "beamngweb",
    "github",
  ]);
  // worldofmods из legacy выдаёт только то, что ещё enabled и легитимно
  // входит в recommended-набор.
  assert.deepEqual(applyPreset("recommended", REGISTRY, ["worldofmods"]), []);
});

test("applyPreset: official_forges включает official и forges", () => {
  assert.deepEqual(applyPreset("official_forges", REGISTRY, ENABLED), [
    "beamngweb",
    "github",
  ]);
});

test("applyPreset: all_configured и clear", () => {
  assert.deepEqual(
    applyPreset("all_configured", REGISTRY, ENABLED),
    SEARCH_CAPABLE_ENABLED,
  );
  assert.deepEqual(applyPreset("clear", REGISTRY, ENABLED), []);
});

test("applyPreset: presets исключают disabled и non-search-capable", () => {
  const enabledWithForum = [...ENABLED, "beamngforum"];
  // all_configured — только enabled ∩ search-capable (без beamngforum).
  assert.deepEqual(
    applyPreset("all_configured", REGISTRY, enabledWithForum),
    SEARCH_CAPABLE_ENABLED,
  );
  // рекомендуемые не зависят от no-search источника.
  assert.deepEqual(
    applyPreset("recommended", REGISTRY, enabledWithForum),
    ["beamngweb", "github"],
  );
  // disabled источник не участвует даже в all_configured.
  assert.deepEqual(applyPreset("all_configured", REGISTRY, ["github"]), ["github"]);
  assert.deepEqual(applyPreset("all_configured", REGISTRY, ["beamngforum"]), []);
  assert.deepEqual(applyPreset("clear", REGISTRY, enabledWithForum), []);
});

test("toggleSourceSelection: selected=null → explicit subset; все → null", () => {
  // из «все» снимаем github → явное подмножество (сортированное).
  assert.deepEqual(
    toggleSourceSelection(REGISTRY, ENABLED, null, "github"),
    ["beamngweb", "worldofmods"],
  );
  // выбор последнего недостающего канонизируется обратно в null.
  assert.equal(
    toggleSourceSelection(REGISTRY, ENABLED, ["beamngweb", "worldofmods"], "github"),
    null,
  );
  // no-search источник не трогается.
  assert.equal(
    toggleSourceSelection(REGISTRY, [...ENABLED, "beamngforum"], null, "beamngforum"),
    null,
  );
});

test("canonicalizeSelection: все → null; подмножество → sorted; пусто → []", () => {
  assert.equal(canonicalizeSelection(SEARCH_CAPABLE_ENABLED, [...SEARCH_CAPABLE_ENABLED]), null);
  assert.deepEqual(
    canonicalizeSelection(SEARCH_CAPABLE_ENABLED, ["worldofmods", "beamngweb"]),
    ["beamngweb", "worldofmods"],
  );
  assert.deepEqual(canonicalizeSelection(SEARCH_CAPABLE_ENABLED, []), []);
});

test("zero enabled / zero selected не падают", () => {
  assert.deepEqual(resolveActiveSources(REGISTRY, [], null), []);
  assert.deepEqual(resolveActiveSources(REGISTRY, [], []), []);
  assert.deepEqual(searchCapableEnabledIds(REGISTRY, []), []);
  assert.deepEqual(applyPreset("all_configured", REGISTRY, []), []);
  assert.equal(toggleSourceSelection(REGISTRY, [], null, "github"), null);
});

test("filterSources: case-insensitive по label и id, сохраняет порядок групп", () => {
  const all = sortByGroup(REGISTRY).map((d) => d.id);
  assert.deepEqual(filterSources(REGISTRY, "").map((d) => d.id), all);
  // case-insensitive по id.
  assert.deepEqual(filterSources(REGISTRY, "GITHUB").map((d) => d.id), ["github"]);
  // case-insensitive по label («форум beamng»).
  assert.deepEqual(filterSources(REGISTRY, "форум beamng").map((d) => d.id), ["beamngforum"]);
  // порядок групп/registry сохраняется при фильтрации.
  const official = filterSources(REGISTRY, "beamng").map((d) => d.id);
  assert.deepEqual(official, ["beamngweb", "beamngforum"]);
});

test("registry order/group order сохраняется", () => {
  const ids = filterSources(REGISTRY, "").map((d) => d.id);
  // official (beamngweb, beamngforum), forges (github), community (worldofmods), custom (directurl).
  assert.deepEqual(ids, ["beamngweb", "beamngforum", "github", "worldofmods", "directurl"]);
});

test("link-import портрет registry: directurl — custom/mods_zip/key_stem; forum — mods_zip/basename", () => {
  const forum = REGISTRY.find((d) => d.id === "beamngforum")!;
  const direct = REGISTRY.find((d) => d.id === "directurl")!;
  assert.equal(forum.installMode, "mods_zip");
  assert.equal(forum.filenameRule, "basename");
  assert.equal(forum.capabilities.directZipDownload, true);
  assert.equal(forum.capabilities.updateDetection, true);
  assert.equal(direct.group, "custom");
  assert.equal(direct.trustLevel, "custom");
  assert.equal(direct.enabledByDefault, false);
  assert.equal(direct.legacyDefault, false);
  assert.equal(direct.installMode, "mods_zip");
  assert.equal(direct.filenameRule, "key_stem");
  assert.equal(direct.capabilities.search, false);
  assert.equal(direct.capabilities.updateDetection, true);
});

test("dedupById: убирает повторы по source:id, сохраняя порядок", () => {
  const items: ModItem[] = [
    { source: "github", id: "a", name: "A", key: "https://github.com/o/r", published: null, description: null, author: null, downloads: null, sizeBytes: null, thumbnail: null, category: null },
    { source: "github", id: "b", name: "B", key: "https://github.com/o/r2", published: null, description: null, author: null, downloads: null, sizeBytes: null, thumbnail: null, category: null },
    { source: "beamngweb", id: "a", name: "A (beamng)", key: "https://repo.beamng.com/v/a", published: null, description: null, author: null, downloads: null, sizeBytes: null, thumbnail: null, category: null },
    { source: "github", id: "a", name: "A duplicate", key: "https://github.com/o/r", published: null, description: null, author: null, downloads: null, sizeBytes: null, thumbnail: null, category: null },
  ];
  const dedup = dedupById(items);
  // id каноничен только в границах источника: beamngweb:a и github:a оба остаются.
  assert.equal(dedup.length, 3);
  assert.equal(dedup[2].name, "A (beamng)");
});

test("urlHost: домен/ссылка", () => {
  assert.equal(
    urlHost("https://www.worldofmods.com/beamng/mods/14735-hirochi.html"),
    "www.worldofmods.com",
  );
  assert.equal(urlHost("not-a-url"), null);
  assert.equal(urlHost("ftp://example.com/x"), null);
  assert.equal(urlHost(null), null);
  assert.equal(urlHost(undefined), null);
});
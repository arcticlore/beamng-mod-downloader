import { test } from "node:test";
import assert from "node:assert/strict";
import type { SourceDescriptor, ModItem } from "../../src/types.ts";
import {
  applyPreset,
  dedupById,
  filenameFor,
  resolveActiveSources,
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
];

const ALL = REGISTRY.map((d) => d.id);
const ENABLED = ["beamngweb", "github", "worldofmods"];

test("resolveActiveSources: selected=null означает все enabled", () => {
  assert.deepEqual(resolveActiveSources(REGISTRY, ENABLED, null), [
    "beamngweb",
    "github",
    "worldofmods",
  ]);
});

test("resolveActiveSources: явный выбор пересекается с enabled и упорядочен по registry", () => {
  assert.deepEqual(
    resolveActiveSources(REGISTRY, ENABLED, ["worldofmods", "beamngweb"]),
    ["beamngweb", "worldofmods"],
  );
  // disabled источник в selected не попадает — он никогда не запрашивается.
  assert.deepEqual(
    resolveActiveSources(REGISTRY, ENABLED, [ALL[0], ...ALL]),
    ALL,
  );
  assert.deepEqual(resolveActiveSources(REGISTRY, ["github"], ["beamngweb"]), []);
  assert.deepEqual(resolveActiveSources(REGISTRY, ENABLED, []), []);
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
  assert.deepEqual(applyPreset("all_configured", REGISTRY, ENABLED), ALL);
  assert.deepEqual(applyPreset("clear", REGISTRY, ENABLED), []);
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
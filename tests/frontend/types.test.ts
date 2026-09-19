import { test } from "node:test";
import assert from "node:assert/strict";
import {
  formatBytes,
  formatSpeed,
  sanitizeFileName,
  tokenize,
  similar,
  findSimilarInstalled,
  installedFileName,
} from "../../src/types.ts";

test("formatBytes: размеры", () => {
  assert.equal(formatBytes(null), "");
  assert.equal(formatBytes(undefined), "");
  assert.equal(formatBytes(0), "0 Б");
  assert.equal(formatBytes(1023), "1023 Б");
  assert.equal(formatBytes(1024), "1.0 КБ");
  assert.equal(formatBytes(1536 * 1024), "1.5 МБ");
  assert.equal(formatBytes(2 * 1024 * 1024 * 1024), "2.00 ГБ");
});

test("formatSpeed: скорости", () => {
  assert.equal(formatSpeed(0), "");
  assert.equal(formatSpeed(500), "500 Б/с");
  assert.equal(formatSpeed(2048), "2.0 КБ/с");
  assert.equal(formatSpeed(3 * 1024 * 1024), "3.0 МБ/с");
});

test("sanitizeFileName: запрещённые символы и хвостовые точки", () => {
  assert.equal(sanitizeFileName('a/b\\c:d*e?f"g<h>i|j\0k'), "a_b_c_d_e_f_g_h_i_j_k");
  assert.equal(sanitizeFileName("  hello world  "), "hello_world");
  assert.equal(sanitizeFileName("name..."), "name");
  assert.equal(sanitizeFileName("///"), "___");
});

test("tokenize: стоп-слова и короткие токены", () => {
  assert.deepEqual([...tokenize("BeamNG mod drive")].sort(), ["beamng"]);
  assert.deepEqual([...tokenize("a bcd")].sort(), ["bcd"]);
  assert.deepEqual([...tokenize("mod x2")].sort(), []);
});

test("similar: Jaccard-метрика", () => {
  assert.equal(similar(new Set(), new Set()), 0);
  assert.equal(similar(new Set(["a"]), new Set(["a"])), 1);
  assert.equal(similar(new Set(["a", "b"]), new Set(["b", "c"])), 1 / 3);
});

test("installedFileName: имена по источникам", () => {
  assert.equal(
    installedFileName({ source: "worldofmods", key: "https://www.worldofmods.com/beamng/mods/14735-hirochi.html" }),
    "14735-hirochi.zip",
  );
  assert.equal(
    installedFileName({ source: "github", key: "https://github.com/A/B" }),
    "A-B.zip",
  );
});

test("findSimilarInstalled: находит похожий установленный мод", () => {
  const item = { source: "worldofmods", name: "Hirochi Sunburst GTZ", key: "https://x/1-hirochi-sunburst.html" };
  const installed = [
    { filename: "unrelated-car.zip", path: "/mods/repo/unrelated-car.zip" },
    { filename: "hirochi-sunburst-mod.zip", path: "/mods/repo/hirochi-sunburst-mod.zip" },
  ];
  const hit = findSimilarInstalled(item, installed);
  assert.ok(hit);
  assert.equal(hit.filename, "hirochi-sunburst-mod.zip");
  assert.equal(findSimilarInstalled({ ...item, name: "zzz qq" }, installed), null);
});
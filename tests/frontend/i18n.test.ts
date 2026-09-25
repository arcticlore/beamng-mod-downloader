import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ru } from "../../src/i18n/ru.ts";
import { en } from "../../src/i18n/en.ts";
import { pluralIndex, t, tp } from "../../src/i18n/index.ts";

describe("i18n dictionaries", () => {
  it("RU и EN имеют идентичный набор ключей", () => {
    const ruKeys = Object.keys(ru).sort();
    const enKeys = Object.keys(en).sort();
    assert.deepEqual(ruKeys, enKeys);
  });

  it("plural-ключи помечены как массивы в обоих словарях", () => {
    for (const key of Object.keys(ru)) {
      const r = ru[key as keyof typeof ru];
      const e = en[key as keyof typeof en];
      assert.equal(Array.isArray(r), Array.isArray(e), `рассыпалось: ${key}`);
    }
  });
});

describe("pluralIndex", () => {
  it("RU: 1 → 0, 2–4 → 1 (кроме 12–14), 5+ → 2", () => {
    assert.equal(pluralIndex("ru", 1), 0);
    assert.equal(pluralIndex("ru", 21), 0);
    assert.equal(pluralIndex("ru", 2), 1);
    assert.equal(pluralIndex("ru", 4), 1);
    assert.equal(pluralIndex("ru", 22), 1);
    assert.equal(pluralIndex("ru", 11), 2);
    assert.equal(pluralIndex("ru", 12), 2);
    assert.equal(pluralIndex("ru", 14), 2);
    assert.equal(pluralIndex("ru", 15), 2);
    assert.equal(pluralIndex("ru", 0), 2);
    assert.equal(pluralIndex("ru", 100), 2);
  });

  it("EN: 1 → 0, остальное → 1", () => {
    assert.equal(pluralIndex("en", 1), 0);
    assert.equal(pluralIndex("en", 0), 1);
    assert.equal(pluralIndex("en", 2), 1);
    assert.equal(pluralIndex("en", 100), 1);
  });
});

describe("t / tp", () => {
  it("t подставляет параметры", () => {
    assert.equal(t("ru", "toast_mod_installed", { name: "M", filename: "m.zip" }),
      "Мод «M» установлен: m.zip");
    assert.equal(t("en", "toast_mod_installed", { name: "M", filename: "m.zip" }),
      "Mod «M» installed: m.zip");
  });

  it("t бросает ошибку на plural-ключе", () => {
    assert.throws(() => t("ru", "count_mods"));
  });

  it("tp подставляет {n} и формы", () => {
    assert.equal(tp("ru", "count_mods", 1), "1 мод");
    assert.equal(tp("ru", "count_mods", 2), "2 мода");
    assert.equal(tp("ru", "count_mods", 5), "5 модов");
    assert.equal(tp("ru", "archives_count", 11), "Архивов: 11");
    assert.equal(tp("en", "count_mods", 1), "1 mod");
    assert.equal(tp("en", "count_mods", 3), "3 mods");
  });

  it("formatBytes/formatSpeed используют единицы языка", async () => {
    const { formatBytes, formatSpeed } = await import("../../src/types.ts");
    assert.equal(formatBytes(1500, "ru"), "1.5 КБ");
    assert.equal(formatBytes(1500, "en"), "1.5 KB");
    assert.equal(formatSpeed(2048, "ru"), "2.0 КБ/с");
    assert.equal(formatSpeed(2048, "en"), "2.0 KB/s");
  });
});
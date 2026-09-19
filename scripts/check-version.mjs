#!/usr/bin/env node
// Проверка единственной версии приложения во всех manifests/locks.
// См. docs/RELEASING.md и §13 remediation-промта.
// Выходной код != 0, если хотя бы один файл расходится с остальными.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function readManifest(rel) {
  return JSON.parse(readFileSync(join(root, rel), "utf8"));
}

function tomlVersion(text, pkgName) {
  const blockRe = new RegExp(
    `\\[package\\][\\s\\S]*?^name\\s*=\\s*"${pkgName}"[\\s\\S]*?^version\\s*=\\s*"([^"]+)"`,
    "m",
  );
  const m = text.match(blockRe);
  if (!m) throw new Error(`Не найден package block '${pkgName}' с version в TOML-файле`);
  return m[1];
}

function lockVersion(text, pkgName) {
  const blockRe = new RegExp(
    `^name\\s*=\\s*"${pkgName}"\\nversion\\s*=\\s*"([^"]+)"`,
    "m",
  );
  const m = text.match(blockRe);
  if (!m) throw new Error(`Не найден root package '${pkgName}' в Cargo.lock`);
  return m[1];
}

const sources = [
  ["package.json", readManifest("package.json").version],
  ["package-lock.json (root)", readManifest("package-lock.json")["packages"][""].version],
  ["package-lock.json (top)", readManifest("package-lock.json").version],
  [
    "src-tauri/Cargo.toml",
    tomlVersion(readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8"), "beamng-mod-downloader"),
  ],
  [
    "src-tauri/Cargo.lock",
    lockVersion(readFileSync(join(root, "src-tauri/Cargo.lock"), "utf8"), "beamng-mod-downloader"),
  ],
  ["src-tauri/tauri.conf.json", readManifest("src-tauri/tauri.conf.json").version],
];

const expected = sources[0][1];
let ok = true;
console.log("Проверка версии (ожидается %s):", expected);
for (const [label, version] of sources) {
  const match = version === expected;
  ok &&= match;
  console.log(`  [${match ? "OK" : "FAIL"}] ${label} = ${version}`);
}
if (!ok) {
  console.error("Версии различаются. Воспользуйтесь: node scripts/set-version.mjs <new-version>");
  process.exit(1);
}
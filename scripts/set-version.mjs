#!/usr/bin/env node
// Единая смена версии приложения во всех manifests/locks.
// Использование: node scripts/set-version.mjs 0.2.0
// Проверка: node scripts/check-version.mjs (также выполняется в CI).

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const [newVersion] = process.argv.slice(2);

if (!/^\d+\.\d+\.\d+(-[a-z0-9.]+)?$/.test(newVersion ?? "")) {
  console.error("Передайте версию в формате X.Y.Z[-prerelease], например: 0.2.0");
  process.exit(2);
}

function read(rel) {
  return readFileSync(join(root, rel), "utf8");
}
function write(rel, text) {
  writeFileSync(join(root, rel), text);
}

// package.json
const pkg = JSON.parse(read("package.json"));
pkg.version = newVersion;
write("package.json", JSON.stringify(pkg, null, 2) + "\n");

// package-lock.json (top-level и root package)
const lock = JSON.parse(read("package-lock.json"));
lock.version = newVersion;
lock.packages[""].version = newVersion;
write("package-lock.json", JSON.stringify(lock, null, 2) + "\n");

// src-tauri/Cargo.toml — только версия [package]
write(
  "src-tauri/Cargo.toml",
  read("src-tauri/Cargo.toml").replace(
    /(\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
    `$1${newVersion}$2`,
  ),
);

// src-tauri/Cargo.lock — root package
write(
  "src-tauri/Cargo.lock",
  read("src-tauri/Cargo.lock").replace(
    /(^name\s*=\s*"beamng-mod-downloader"\nversion\s*=\s*")[^"]+(")/m,
    `$1${newVersion}$2`,
  ),
);

// src-tauri/tauri.conf.json
const conf = JSON.parse(read("src-tauri/tauri.conf.json"));
conf.version = newVersion;
write("src-tauri/tauri.conf.json", JSON.stringify(conf, null, 2) + "\n");

console.log("Версия обновлена до", newVersion);
console.log("Не забудьте: tag v" + newVersion + " обязан указывать на commit этой версии.");
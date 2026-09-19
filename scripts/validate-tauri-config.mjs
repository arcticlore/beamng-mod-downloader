#!/usr/bin/env node
// Структурная валидация Tauri-конфигурации и capabilities (PR gate).
// Здесь не проверяется политика CSP != null — её вводит PR runtime-security
// (§4.2 remediation-промта); см. комментарий в build.yml (job tauri-config).

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => JSON.parse(readFileSync(join(root, rel), "utf8"));

let failures = 0;
const fail = (msg) => {
  failures += 1;
  console.error(`  FAIL ${msg}`);
};
const ok = (msg) => console.log(`  OK   ${msg}`);

const conf = read("src-tauri/tauri.conf.json");

if (!conf.productName || typeof conf.productName !== "string") {
  fail("productName отсутствует или не строка");
} else {
  ok(`productName = ${conf.productName}`);
}

if (!/^[a-z0-9]+(\.[a-z0-9-]+)+$/.test(conf.identifier ?? "")) {
  fail(`identifier не похож на reverse-DNS: ${conf.identifier}`);
} else {
  ok(`identifier = ${conf.identifier}`);
}

if (!/^\d+\.\d+\.\d+$/.test(conf.version ?? "")) {
  fail(`version не семверсия: ${conf.version}`);
} else {
  ok(`version = ${conf.version}`);
}

if (!conf.build?.frontendDist || !conf.build?.beforeBuildCommand) {
  fail("build.frontendDist / build.beforeBuildCommand отсутствуют");
} else {
  ok(`frontendDist = ${conf.build.frontendDist}`);
}

if (!Array.isArray(conf.app?.windows) || conf.app.windows.length === 0) {
  fail("нет ни одного окна");
} else {
  for (const w of conf.app.windows) {
    if (!w.title) fail("окно без title");
  }
  ok(`windows = ${conf.app.windows.length}`);
}

if (conf.app?.security?.csp === undefined) {
  fail("ключ app.security.csp отсутствует (ожидается null или строка)");
} else {
  ok(conf.app.security.csp === null
    ? "app.security.csp = null (разрешено до PR runtime-security)"
    : "app.security.csp задан");
}

// capabilities
const caps = read("src-tauri/capabilities/default.json");
if (!caps.identifier) fail("capability без identifier");
if (!Array.isArray(caps.windows) || caps.windows.length === 0) fail("capability без windows");
if (!Array.isArray(caps.permissions) || caps.permissions.length === 0) fail("capability без permissions");
for (const p of caps.permissions ?? []) {
  if (!/^[a-z0-9-]+(:[a-z0-9-]+){0,4}$/.test(p)) fail(`невалидный permission identifier: ${p}`);
}
ok(`capabilities.permissions = ${(caps.permissions ?? []).join(", ") || "(пусто)"}`);

if (failures > 0) {
  console.error(`tauri config validation: ${failures} ошибок`);
  process.exit(1);
}
console.log("tauri config validation: OK");
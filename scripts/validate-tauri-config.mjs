#!/usr/bin/env node
// Структурная и security-валидация Tauri-конфигурации и capabilities (PR gate).
// Требования PR runtime-security:
//   - CSP задан и строгий (default-src 'self', object-src 'none');
//   - devtools выключены в конфигурации окна;
//   - remote-domain IPC не разрешён;
//   - capabilities минимальны (только локальные команды ядра + логирование).

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
  fail("ключ app.security.csp отсутствует");
} else if (typeof conf.app.security.csp !== "string") {
  fail(`CSP должен быть строкой (сейчас ${JSON.stringify(conf.app.security.csp)})`);
} else {
  const csp = conf.app.security.csp;
  const required = ["default-src 'self'", "object-src 'none'"];
  for (const directive of required) {
    if (!csp.includes(directive)) fail(`CSP должен содержать «${directive}»`);
  }
  if (csp.includes("'unsafe-eval'")) fail("CSP не должен содержать 'unsafe-eval'");
  ok("CSP задан и строгий");
}

for (const w of conf.app?.windows ?? []) {
  if (w.devtools !== false) fail(`окно «${w.title}» должно иметь devtools: false`);
  if (w.fullscreen) fail(`окно «${w.title}» не должно запускаться fullscreen`);
}
if (conf.app?.windows?.length) ok("devtools выключены во всех окнах");

const remoteIpc = conf.app?.security?.dangerousRemoteDomainIpcAccess;
if (remoteIpc !== undefined) {
  fail("app.security.dangerousRemoteDomainIpcAccess удалён/не нужен (tauri-build 2.6+): remote IPC задаётся только через capabilities; должен отсутствовать");
} else {
  ok("remote-domain IPC не включён (capabilities local:true, поле исключено)");
}

// capabilities
const caps = read("src-tauri/capabilities/default.json");
if (!caps.identifier) fail("capability без identifier");
if (!Array.isArray(caps.windows) || caps.windows.length === 0) fail("capability без windows");
if (caps.local !== true) fail("capability должна быть local: true (без remote IPC)");
if (!Array.isArray(caps.permissions) || caps.permissions.length === 0) fail("capability без permissions");
for (const p of caps.permissions ?? []) {
  if (!/^[a-z0-9-]+(:[a-z0-9-]+){0,4}$/.test(p)) fail(`невалидный permission identifier: ${p}`);
}
const dangerousPermissions = ["shell:", "process:", "fs:", "http:", "opener:", "core:event:default", "core:event:allow-listen", "core:event:allow-emit"];
for (const p of caps.permissions ?? []) {
  if (dangerousPermissions.includes(p)) fail(`ненужный привилегированный permission: ${p}`);
}
ok(`capabilities.permissions = ${(caps.permissions ?? []).join(", ") || "(пусто)"}`);

if (failures > 0) {
  console.error(`tauri config validation: ${failures} ошибок`);
  process.exit(1);
}
console.log("tauri config validation: OK");
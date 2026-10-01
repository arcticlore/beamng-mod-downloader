# Security Review Report

## Executive Summary

| Field | Value |
|-------|-------|
| **Приложение** | BeamNG Mod Downloader |
| **Версия / Commit** | v0.4.0 / `d35027cc48d0bd18400bd0afbffc832e17b8dcf3` |
| **Дата ревью** | 2026-10-02 |
| **Метод** | Пассивный static review (код + конфиг + CI-evidence); без активного тестирования и прод-систем |
| **Scope** | backend `src-tauri/src/*`, sources, urlguard, archive, download, capabilities, tauri.conf.json (CSP), рендерер `src/*` (XSS/`open_url`), `docs/THREAT-MODEL.md` (сверка), workflows CI |
| **Overall Risk Level** | **Low** |

### Key Findings
- **0** Critical-находок, требующих немедленного вмешательства
- **0** High-находок перед релизом
- **0** Medium-находок
- **2** Low (F1 — лимит размера скачивания, F2 — латентная cmd-инъекция в `open_url` Windows) — **исправлены в PR SEC-001/SEC-002**, **4** Info (SEC-003 переклассифицирован из Low в Info), **1** признанное ограничение (DNS-rebinding)

Вывод: приложение для локального использования защищено хорошо для своего класса. Ключевые механизмы (SSRF-allowlist с ревалидацией redirect-хопов, лимиты ZIP, минимальные capabilities + строгий CSP, ledger как источник истины для обновлений, гейт enabled-источников в backend, отсутствие XSS-поверхности) реализованы и подтверждены кодом. Находки — защита глубины и гигиена.

## Findings Summary

| Severity | Count | Status |
|----------|-------|--------|
| Critical | 0 | — |
| High | 0 | — |
| Medium | 0 | — |
| Low | 2 | Исправлены в PR SEC-001/SEC-002 |
| Info | 5 | SEC-003..SEC-006 (SEC-003 переклассифицирован из Low в Info) |
| Признанное ограничение | 1 | Документировано |

## Detailed Findings

### [LOW] Нет лимита размера скачиваемого/импортируемого файла (дисковая DoS)

| Field | Value |
|-------|-------|
| **ID** | SEC-001 |
| **Location** | `src-tauri/src/download.rs:159-209` (`stream_to_part`); тот же класс — `src-tauri/src/lib.rs:510` (`import_local_zip`) |
| **CWE** | CWE-400 (Uncontrolled Resource Consumption) |
| **CVSS** | 3.6 (Low) — AV:N/AC:H/PR:N/UI:N/S:C/C:N/I:N/A:L |

**Description**
`stream_to_part` пишет тело ответа в `.part` без верхнего предела: единственная граница — `reqwest` timeout 120 с (`http.rs:15`) и проверка `received != total`, когда сервер прислал Content-Length (работает только против *обрывов*, не против *нарочно длинного* потока). Размер контролируется только временем: на быстром канале за 120 с можно принять десятки ГБ. При HTTP-сжатии шкала — уже разархивированные байты, лимита тоже нет. Лимиты `archive.rs` (`MAX_ZIP_ENTRIES=10 000`, `MAX_UNCOMPRESSED_TOTAL=4 GiB`) применяются **после** полной записи и по *заявленным* центральнокаталожным размерам, а сам скачанный файл не ограничен. `import_local_zip` ограничивает лишь расширением `.zip`, не размером.

**Влияние**: недоверенный/скомпрометированный allowlist-хост (или редирект-цель r2.dev/githubusercontent) может заливать дисковое пространство пользователя до исчерпания.

**Remediation**
- Ввести `MAX_DOWNLOAD_BYTES` (напр. 8 GiB) и отбрасывать запрос заранее по `Content-Length` > лимита, а в цикле — по достижении `received`-лимита.
- Для `import_local_zip` — аналогичный лимит размера копии (source file size).

**Effort**: 1-2 часа
**Priority**: Следующий релиз
**Remediation Status**: **implemented** — единый `MAX_DOWNLOAD_BYTES = 8 GiB`; pre-check `Content-Length` до скачивания и счётчик фактических байт в `stream_to_part` (chunked/сжатие включены); `.part`-cleanup при превышении; `copy_limited` для `import_local_zip` с той же границей; ledger не меняется при ошибке. Fixed by commit: `2a8df0e` (PR #21, 2026-10-02).

---

### [LOW] Латентная OS-командная инъекция в `open_url` (Windows)

| Field | Value |
|-------|-------|
| **ID** | SEC-002 |
| **Location** | `src-tauri/src/lib.rs:112-114` |
| **CWE** | CWE-78 (Improper Neutralization of Special Elements in OS Command) |
| **CVSS** | 3.1 (Low) — AV:N/AC:H/PR:N/UI:R/S:C/C:N/I:N/A:H; на текущий момент **не эксплуатируется**: единственный вызов — константная ссылка |

**Description**
`open_url` принимает любой `http(s)`-URL от renderer и на Windows строит `cmd /C start "" <url>`. Rust передаёт строку как один argv-элемент, но `cmd.exe` парсит командную строку **заново**: метасимволы в URL (`&`, `|`, `&&`) интерпретируются как разделители команд. По классификации это инъекция команды через cmd-метасимволы (способ l33t-пример: URL с `&whoami`). Сейчас первый и единственный вызов — `src/components/LinkImportModal.tsx:150` с константой `https://www.beamng.com/community/`, поэтому недоверенного входа в команду нет.

**Влияние**: при появлении будущего вызова, передающего в `open_url` ссылку из метаданных мода (описание/страница источника), — выполнение произвольной команды на Windows в контексте пользователя.

**Remediation**
- Заключить URL в кавычки: `cmd /C start "" "<url>"`, **или**
- Заменить `open_url` на `tauri-plugin-opener` / крейт `open` (без shell-парсинга), **или**
- Ограничить allowlist-доменами (как в urlguard).

**Effort**: 15-30 минут
**Priority**: Следующий релиз
**Remediation Status**: **implemented** — `open_url` заменён на `open_community` (SEC-002): команда без аргументов, единственный захардкоженный одобренный URL; открытие без оболочки (`xdg-open`/`open`/`explorer`, отдельный argv, без cmd/sh/PowerShell); renderer-вызов `openCommunity()`; тесты инварианта argv. Fixed by commit: `2a8df0e` (PR #21, 2026-10-02).

---

### [INFO] THREAT-MODEL.md: таблица статусов устарела

| Field | Value |
|-------|-------|
| **ID** | SEC-003 |
| **Location** | `docs/THREAT-MODEL.md:125-138` |
| **CVSS** | N/A (процесс/документация) |

**Description**
Код v0.4.0 уже реализует меры, помеченные «затем: PR …»: **T3.6** (SHA-256 пишется в ledger — `download.rs:438`, и сверяется в `verify_installed` — `lib.rs:622-634`) и **T3.8** (capabilities-минимум `capabilities/default.json`, строгий CSP `tauri.conf.json:25`). **T3.4** реализован частично: относительный путь без `..`/absolute (`lib.rs:712-729`), но без prescribed canonicalize/symlink_metadata. **T3.5** (атомарная запись ledger) пока не реализован (см. SEC-004).

**Remediation**: обновить таблицу статусов при следующем изменении кода (заодно «Версия документа»). **Выполнено** в PR SEC-001/SEC-002: T3.6/T3.8 — implemented, T3.4 — partial, T3.5 — open; заголовок документа обновлён.

**Effort**: 30 минут
**Priority**: По желанию

---

### [INFO] Неатомарная запись ledger/config + гонка load→save

| Field | Value |
|-------|-------|
| **ID** | SEC-004 |
| **Location** | `src-tauri/src/ledger.rs:38-64`; `src-tauri/src/config.rs:67-77` |
| **CWE** | CWE-362 / CWE-667 |

**Description**: `ledger::upsert` = `load()` → `insert` → `save()`; при параллельном завершении загрузок (до 3 одновременно) возможна потеря записи. Запись — прямой `std::fs::write` (без temp+fsync+rename). Влияние — только целостность трекера установленных модов (ложная инфа для `verify_installed`/`check_updates`), не безопасность.

**Remediation**: атомарная запись (temp + rename), при возможности — сериализация upsert через `Mutex`. (Совпадает с заделом T3.5 в THREAT-MODEL.)

---

### [INFO] Update-флоу удаляет рабочую версию до успешного rename

| Field | Value |
|-------|-------|
| **ID** | SEC-005 |
| **Location** | `src-tauri/src/download.rs:620-626` |

**Description**: при обновлении старый архив и target удаляются до `tokio::fs::rename` нового `.new.part`; при сбое rename (напр. антивирус/занятость) рабочая версия мода теряется — противоречит заделу T3.7 «сохранять рабочую версию до успешной проверки новой».

**Remediation**: `rename` поверх старого (unix-атомарно) или backup перед удалением.

---

### [INFO] Возможна лог-инъекция из недоверенных строк

| Field | Value |
|-------|-------|
| **ID** | SEC-006 |
| **Location** | `src-tauri/src/download.rs:427,447` и пр. (`info!` с url/name) |

**Description**: имена/URL из источников попадают в лог как есть; многострочные/управляющие последовательности засоряют лог. Безопасности не несёт (лог локальный).

**Remediation**: санитизация CR/LF/ESC при логировании недоверенных строк.

---

## Проверено и подтверждено (hardening)

| Область | Что подтверждено | Где |
|---|---|---|
| SSRF-гейт | http/https; порты 80/443; запрет IP-литералов (IPv4/IPv6/decimal/hex); userinfo запрещён; registrable-suffix allowlist + exact-host для gitlab.com/codeberg.org; лимит хопов 8; ревалидация каждого редиректа | `urlguard.rs:63-198` |
| Применение гейта | `http::fetch*`, `download::start/update`, source-scoped `validate_*_url` перед скачиванием; `install_from_url` → `directurl`/`beamngforum` повторно валидируют | `http.rs:28`, `download.rs:352,539` |
| ZIP | EOCD+central directory, выравнивание, лимиты 10 000 / 4 GiB, валидация без распаковки; zip-slip невозможен (архивы не распаковываются приложением) | `archive.rs:89-298` |
| Изоляция процесса | capabilities = только `core:default`, `log:default`, `dialog:allow-open`; без shell/fs/http/process/opener; Cargo без tauri-plugin-fs/shell | `capabilities/default.json`, `Cargo.toml` |
| CSP | `default-src 'self'; script-src 'self'` (без inline/unsafe-eval); img-src разрешает `https:` (аватары), connect-src `self` | `tauri.conf.json:25` |
| XSS | Имена/описания — текст (авто-экранирование); `dangerouslySetInnerHTML` только с i18n-константой; нет raw-HTML-рендера метаданных | `src/**` grep |
| Гейт источников | enabled проверяется в backend-командах (search/detail/install/update); ledger — источник истины; импортированные локально и disabled-источники не опрашиваются | `lib.rs:317,348,419,579,753` |
| Файловая система | sanitize_filename (пути/`..`/reserved); no-clobber; `.part`-staging; `remove_installed` — только относительный путь без `..`/absolute | `http.rs:47-63`, `download.rs:362-370`, `lib.rs:712-731` |
| TLS | reqwest + rustls-tls (webpki-roots), без нативного стора | `Cargo.toml:23` |

## Automated Scan Results

### Dependency / CI
| Канал | Результат |
|---|---|
| `npm audit` (локально) | 0 уязвимостей |
| CI `build` + bundle matrix (linux appimage/deb/rpm, macos dmg, windows nsis/msi) | green на `d35027c` |
| CI `codeql` / `osv-scanner` / `cargo-deny` / `npm-audit` / `tauri-config` / `version-consistency` | green на `d35027c` |
| Секреты (rg по рабочему дереву + `git log --all -p`) | 0 попаданий; `.env` отсутствуют |

### Ограничения метода
- Локально не установлены `gitleaks`/`semgrep`/`trivy`/`cargo-audit`/`cargo-deny` — компенсировано ручным сканом и зелёным CI-покрытием тех же проверок.
- Просмотр пассивный: запуск приложения, фиддлинг IPC и сетевых ответов не выполнялись.

## Признанные ограничения / принятый риск
- **DNS-rebinding**: собственный DNS-резолв не делается (`urlguard.rs:29-31`); hostname-allowlist не перекрывает переразрешение allowlist-домена на приватный адрес. Для исходящего клиента к allowlist-CDN в десктопном приложении риск низкий; задокументирован.

## Recommendations

### Immediate (Этот спринт)
1. (нет критичных/высоких)

### Short-term (Следующий релиз)
1. **SEC-001**: `MAX_DOWNLOAD_BYTES` в `stream_to_part` + pre-check Content-Length; лимит для `import_local_zip`. — ✅ сделано в PR SEC-001/SEC-002.
2. **SEC-002**: кавычки/`tauri-plugin-opener` в `open_url` (Windows). — ✅ сделано в PR SEC-001/SEC-002 (shell-free `open_community`).
3. **SEC-003**: обновить таблицу статусов THREAT-MODEL. — ✅ сделано.

### Long-term
1. SEC-004: атомарная запись ledger/config + сериализация upsert.
2. SEC-005: update без удаления рабочей версии до успешного rename.
3. SEC-006: санитизация лога.
4. Периодические security-reviews после каждого релиза.

## Appendix

### Tools Used
- Локально: `npm audit`, `rg` (секреты/паттерны), ручной code review файлов backend + renderer + конфигов.
- CI-evidence: GitHub Actions `build`/`codeql`/`osv-scanner`/`npm-audit`/`cargo-deny`/`tauri-config`/`version-consistency`.

### References
- OWASP Top 10 2021 (A3 Injection, A7 Injection, A9/Files)
- CWE-78, CWE-89, CWE-400, CWE-362, CWE-667, CWE-918
- `docs/THREAT-MODEL.md` (сверка статусов)

---
Сформирован по шаблону security-reviewer skill; подтверждение пользователя получено перед финализацией.
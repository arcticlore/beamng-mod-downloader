# Threat Model: BeamNG Mod Downloader

Версия документа: 0.3.0-dev (PR sources: forum attachments + direct link + local import). Обновлять при
каждом изменении, затрагивающем доверие, источники, сеть или файловую систему.

## 1. Ценность и поверхности

Приложение качает архивы модов из интернета, устанавливает их в папку модов
BeamNG.drive, ведёт реестр (ledger) установленного и настройки.

Поверхности атаки:

- backend (Rust): URL-построение, HTTP-клиент, ZIP-обработка, файловая система,
  ledger/config, единый **registry источников** и гейт enabled;
- renderer (React/Tauri): UI, команды через IPC;
- соответствие источников: WorldOfMods, официальный BeamNG resource site, GitHub,
  BeamNG Forum (вложения по каноническому URL), прямая ссылка (allowlist-домены),
  локальный импорт `.zip` (в будущих PR — GitLab, Codeberg, Nexus Mods, Beam-Monsters, 2Fast,
  пользовательские каталоги). Все id-источников и их метаданные живут в одном
  registry (Rust), фронтенд их не дублирует;
- CI/CD и release pipeline: GitHub Actions, OBS, AUR.

## 2. Модель доверия

| Компонент | Доверие | Комментарий |
| --------- | ------- | ----------- |
| Мод BeamNG | **недоверенный** | может содержать исполняемый Lua-код |
| WorldOfMods, произвольный GitHub-репозиторий | **недоверенный** | содержимое и HTML/metadata управляются третьими лицами |
| Официальный ресурс BeamNG | повышенное, не абсолютное | не является гарантией безопасности мода |
| Registry / дескрипторы источников (Rust) | повышенное | только известные id; level доверия присваивает код, а не источник |
| Renderer (UI) | **потенциально компрометируемый** | не должен иметь возможность инициировать произвольные HTTP-запросы или удаление произвольных файлов |

Backend обязан валидировать **все** данные, приходящие из renderer
(включая `InstallRequest.key`), и сам строить/пере-валидировать URL.

## 3. Основные сценарии угроз

### T3.1 Произвольная загрузка / SSRF через URL из UI
Атакующий модифицирует renderer или отправляет поддельную команду с `key`,
указывающим на `http://169.254.169.254/...`, `file://`, `localhost` и т.п.

Защита: backend сам строит URL из source + canonical id; принимаемые URL
проходят allowlist scheme/host + проверку после каждого redirect; запрещены
localhost/loopback/link-local/private/reserved и URL-credentials; лимит redirects.
**В этом PR**: команда `install_from_url` сама распознаёт форму ссылки
(attachment-URL форума → канонический `…/attachments/<id>/`; иначе → источник
`directurl`); `directurl::resolve_download` заново валидирует allowlist
scheme/host (+ бонус: `%.html`-суффиксы в имени файла) перед скачиванием;
письмо идёт через тот же allowlist-гейт после каждого redirect, что и T3.1.

### T3.2 HTML/JSON/EXE, переименованный в `.zip`
Реален, т.к. Content-Type и расширение не являются гарантиями.

Защита: проверка ZIP magic + полное открытие central directory через
ZIP-библиотеку; отклонение не-ZIP; лимиты entry count/uncompressed size;
проверка unsafe entry path.

### T3.3 ZIP bomb
Архив малого размера с огромным объёмом распакованных данных.

Защита: лимит суммарного uncompressed size и числа entries.

### T3.4 Symlink/path traversal при удалении
`mods/repo -> /outside/...` либо `../` в имени.

Защита: canonicalize mods root и target; target обязан оставаться в root;
`symlink_metadata`; удаление только найденных backend-ом `.zip`;
backend-generated opaque id для операций.

### T3.5 Гонки ledger/config (load→modify→save)
Две загрузки завершаются одновременно и теряют записи.

Защита: состояние под `Mutex`; атомарная запись (temp + fsync + rename);
schema version + migration; при повреждённом JSON — backup, а не молчаливая
потеря.

### T3.6 Модификация/подмена скачанного файла
MITM/несоответствие между полями и содержимым.

Защита: HTTPS-only + allowlist; SHA-256 считается и пишется в ledger;
проверка публикуемого source checksum/signature либо честный статус `unverified`.

### T3.7 Обман обновлений
Сравнение версии по `repository.updated_at` даёт ложные срабатывания,
а WorldOfMods не предоставляет стабильного сигнала.

Защита: GitHub — release tag/ID + asset ID/name/digest; при отсутствии сигнала
показ «статус обновления неизвестен», а не «обновлений нет»; update flow
сохраняет рабочую версию до успешной проверки новой.

### T3.8 Renderer → backend произвольные команды
Capabilities должны оставлять минимум; CSP запрещает `unsafe-eval`; внешние
ссылки открываются безопасным opener, без shell-команд из недоверенных строк.

### T3.9 Сетевой запрос к отключённому источнику
Компрометированный renderer обходит UI и шлёт `search_mods`/`get_mod_detail`/
`install_mod`/`update_mod` с id источника, который пользователь **отключил**.

Защита: гейт enabled стоит в backend-командах (не только в UI): источник
проверяется на `Config.enabled_sources` до любых сетевых операций
(поиск, описание, установка, обновление); `check_updates` пропускает записи
ledger отключённых источников. Если источник не в registry — команда падает
с «неизвестный источник», без match-провала диспетчера. Неизвестные id в
`enabled_sources`/`selected_sources` при загрузке конфига выбрасываются — конфиг
не ломается, орфографические ошибки не «включают» несуществующий источник.
**В этом PR**: `install_from_url` маршрутизирует ссылку на `beamngforum`/
`directurl` и проверяет **этот** источник на enabled до начала сети;
`import_local_zip` сеть не трогает (гейт не нужен), но принудительно проходит
staging + структурную проверку zip + no-clobber и в ledger не пишется —
происхождение файла местное (как ручная установка).

### T3.10 Локальный импорт произвольного файла как `.zip`
Атакующий/UI импортирует файл, который не является валидным архивом мода.

Защита: путь приходит из системного диалога, но backend не доверяет ему —
файл копируется в staging `.part`, проходит ту же структурную проверку zip
(magic + central directory + лимиты записей и распакованного объёма + опасные
пути), затем no-clobber-переименование в папку модов. Локальный источник не
создаёт сетевых запросов, ledger не пишется — а раз записи ledger нет,
`check_updates` такой файл пропускает (T3.7): локальный импорт **не участвует
в проверке обновлений**.

## 4. Статус

| id | Мера | Статус |
| --- | ---- | ------ |
| T3.1 | URL allowlist + revalidate на redirect'ах; directurl: форма ссылки → канон. URL | PR sources (этот) |
| T3.2 | ZIP magic + central directory + не-ZIP отклонение | PR sources (этот): archive.rs |
| T3.3 | лимиты entry count / uncompressed size (10 000 записей / 4 GiB) | PR sources (этот): archive.rs |
| T3.4 | canonicalize + symlink-защита удаления | затем: PR filesystem |
| T3.5 | ledger под Mutex + atomic write + schema | затем: PR ledger-config |
| T3.6 | SHA-256 в ledger + unverified-статус | затем: PR download-integrity |
| T3.7 | release/asset identity + честный update | затем: PR update-flow |
| T3.8 | capabilities-минимум + CSP != null | затем: PR runtime-security |
| T3.9 | enabled-гейт в backend-командах + санитизация конфига | PR sources (этот) |
| T3.10 | локальный импорт: staging + zip-проверка + no-clobber, без ledger и без проверки обновлений | PR sources (этот) |
| — | CI: fmt/test/clippy/ts/build/audit/version/static | PR foundation (этот) |
| — | CodeQL, пин Actions по SHA, release gate | PR foundation (этот) |

## 5. UI: предупреждение доверия

Перед первой установкой из стороннего источника UI обязан показать: источник и
домен, автор/repository, размер, имя asset, доступный checksum,
статус `официальный/сторонний/непроверенный` и предупреждение об исполняемом
коде. Решение запоминается по source с настройкой «всегда спрашивать».

(Реализация подтверждается интеграционными тестами backend; см. PR runtime-security.)
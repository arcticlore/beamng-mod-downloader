# BeamNG Mod Downloader

Кроссплатформенный установщик модов для **BeamNG.drive** на Tauri 2 (Rust) + React.

Возможности:

- автообнаружение папок модов (Linux / Windows / macOS), ручной выбор любой папки;
- единый **registry источников**: каждый источник описывается дескриптором (группа,
  уровень доверия, категории, правило имени файла, режим установки, статус) и
  управляется в «Настройки → Источники». Фронтенд не держит копию списка источников —
  получает его командой `get_source_registry`;
- включённые источники можно выбирать для поиска: один источник — честная пагинация
  источника, несколько — безопасная агрегация (несколько источников по несколько
  страниц, дедупликация по `source:id`, частичные ошибки показываются отдельным
  баннером, источники можно скрывать чипами);
- три источника модов:
  - **WorldOfMods** — без авторизации, категории «Все / Авто / Карты / Мото / Авиа», пагинация;
  - **Официальный сайт BeamNG** — без токена; категории: Авто, Карты и террейны, Сценарии, Автоматизация, Ландшафт, Скины, Звуки, UI и приложения, Track Builder, Номерные знаки, Моды модов; скачивание идёт напрямую через beamng.com;
  - **GitHub-релизы** — поиск по `topic:beamng` через GitHub Search API, скачивание `.zip` из последнего релиза напрямую с `objects.githubusercontent.com`; не требует авторизации (кэширование ответов API в памяти);
- отключённый источник **никогда не запрашивается в сети** (поиск, описание,
  установка, обновления) — гейты стоят в командах Tauri;
- сортировка в браузере модов: актуальность / новизна / имя / популярность / размер;
- карточки модов: имя, автор, аватарка, описание, счётчик скачиваний, размер;
- кастомизация интерфейса: тёмная/светлая тема, акцентный цвет, размер карточек, сортировка и сворачивание панели установленных;
- логирование: файл `beamng.log` в каталоге данных приложения (текущая скорость, ошибки сети, действия), консоль фронтенда продублирована туда же; в настройках есть кнопка «Открыть папку логов»;
- скачивание с прогрессом (текущая скорость, байты), установка прямо в папку модов (папку `mods` текущей версии);
- вкладка «Установленные» со списком `.zip`, удаление;
- конфиг хранится в `~/.config/beamng-mod-downloader/config.json` (`mods_folder`, `theme`, `accent`, `installed_sort`, `installed_collapsed`, `card_size`, `enabled_sources`, `selected_sources`).

## Структура

```
src/                        React-интерфейс (TypeScript, Vite)
src/api.ts                  тонкая обёртка над командами Tauri (с логированием invoke)
src/theme.ts                применение темы и акцентного цвета
src/sources.ts              чистая логика выбора источников (пресеты, имена файлов, дедупликация)
src/SourcesContext.tsx      состояние registry и выбора источника для всего фронтенда
src/components/             ModCard, ModsBrowser, DetailModal, InstalledPanel, SettingsModal (включая секцию «Источники»), ProgressBar
src-tauri/src/lib.rs        команды Tauri и AppState (включая гейты enabled-источников)
src-tauri/src/config.rs     чтение/запись конфига (+ миграция выбора источников)
src-tauri/src/game.rs       поиск папок модов
src-tauri/src/http.rs       HTTP-клиент и утилиты
src-tauri/src/download.rs   менеджер загрузок с прогрессом
src-tauri/src/installer.rs  список/удаление установленных модов
src-tauri/src/sources/      registry.rs (единый список источников) + worldofmods.rs, beamngweb.rs, github.rs (+ mod.rs, SourceError)
```

## Сборка

Предпосылки (Fedora/RHEL):

```bash
sudo dnf install webkit2gtk4.1-devel librsvg2-devel
```

Ubuntu/Debian: `libwebkit2gtk-4.1-dev librsvg2-dev`.

Сборка и запуск:

```bash
cd beamng-mod-downloader
npm install
npx tauri dev          # запуск в dev-режиме
npx tauri build        # сборка релиза (AppImage/.deb/.rpm и т.п.)
```

Релизные сборки также автоматически собираются в GitHub Actions на каждый пуш в `main` (артефакты в Actions → build → Artifacts: AppImage/.deb/.rpm).

Тесты Rust:

```bash
cd src-tauri
cargo test
```

Тесты фронтенда (чистые функции, без браузера):

```bash
npm test
```

## Логи

Подробный лог приложения пишется в файл `beamng.log` в каталоге данных приложения
(`app_log_dir`, например `~/.local/share/com.beamng.mod-downloader/logs/` на Linux).
В него попадают: запуск приложения, смена папки модов,
каждый invoke-вызов, ошибки поиска/скачивания (включая rate limit GitHub), ну и консоль
фронтенда (через `attachConsole` плагина `tauri-plugin-log`). В настройках есть кнопка
«Открыть папку логов».

## Конфиг

- Linux: `~/.config/beamng-mod-downloader/config.json`
- Windows: `%APPDATA%/beamng-mod-downloader/config.json`
- macOS: `~/Library/Application Support/beamng-mod-downloader/config.json`

Поля: `mods_folder` — путь к папке модов; `theme` — `"dark"`/
`"light"`; `accent` — HEX-цвет; `installed_sort` — `"date"`/`"name"`/`"size"`;
`installed_collapsed` — булево; `card_size` — `"compact"`/`"normal"`/`"large"`.

### Выбор источников (`enabled_sources`, `selected_sources`)

- `enabled_sources` — массив id источников, которым разрешены сетевые запросы.
  Отсутствие поля обрабатывается при загрузке конфига:
  - у **нового** конфига (файла ещё не было) включаются рекомендуемые источники
    (`beamngweb`, `github`; WorldOfMods — вне рекомендуемого набора);
  - у **существующего** конфига от версии ≤ 0.2.x включаются все три источника,
    которые были доступны раньше (`beamngweb`, `github`, `worldofmods`) — поведение
    0.2.x сохраняется.
  Неизвестные/удалённые id выбрасываются при загрузке и не ломают конфиг.
- `selected_sources` — массив id активных для поиска источников, либо `null`
  (по умолчанию) = «все включённые». Поле сохраняется только если выбор был изменён
  вручную (снят флажок «Поиск» или применён пресет).
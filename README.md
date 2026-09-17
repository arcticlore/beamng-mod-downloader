# BeamNG Mod Downloader

Кроссплатформенный установщик модов для **BeamNG.drive** на Tauri 2 (Rust) + React.

Возможности:

- автообнаружение папок модов (Linux / Windows / macOS), ручной выбор любой папки;
- вкладка **«Все»** — моды со всех доступных источников сразу (агрегируются и
  объединяются в одном списке);
- четыре источника модов:
  - **WorldOfMods** — без авторизации, категории «Все / Авто / Карты / Мото / Авиа», пагинация;
  - **Официальный сайт BeamNG** — без токена; категории: Авто, Карты и террейны, Сценарии, Автоматизация, Ландшафт, Скины, Звуки, UI и приложения, Track Builder, Номерные знаки, Моды модов; скачивание идёт напрямую через beamng.com;
  - **GitHub-релизы (без токена)** — поиск по `topic:beamng` через GitHub Search API, скачивание `.zip` из последнего релиза напрямую с `objects.githubusercontent.com`; не требует авторизации (кэширование ответов API в памяти);
  - **Репозиторий BeamNG (токен)** — поиск и детали через `api.beamng.com` (требуется токен аккаунта, вводится в настройках); вкладка подходит для тех, кто уже имеет доступ к API;
- сортировка в браузере модов: актуальность / новизна / имя / популярность / размер;
- карточки модов: имя, автор, аватарка, описание, счётчик скачиваний, размер;
- кастомизация интерфейса: тёмная/светлая тема, акцентный цвет, размер карточек, сортировка и сворачивание панели установленных;
- логирование: файл `beamng.log` в каталоге данных приложения (текущая скорость, ошибки сети, действия), консоль фронтенда продублирована туда же; в настройках есть кнопка «Открыть папку логов»;
- скачивание с прогрессом (текущая скорость, байты), установка прямо в папку модов (папку `mods` текущей версии);
- вкладка «Установленные» со списком `.zip`, удаление;
- конфиг хранится в `~/.config/beamng-mod-downloader/config.json` (`modsFolder`, `repoToken`, `theme`, `accent`, `installedSort`, `installedCollapsed`, `cardSize`).

## Структура

```
src/                        React-интерфейс (TypeScript, Vite)
src/api.ts                  тонкая обёртка над командами Tauri (с логированием invoke)
src/theme.ts                применение темы и акцентного цвета
src/components/             ModCard, ModsBrowser, DetailModal, InstalledPanel, SettingsModal, ProgressBar
src-tauri/src/lib.rs        команды Tauri и AppState
src-tauri/src/config.rs     чтение/запись конфига
src-tauri/src/game.rs       поиск папок модов
src-tauri/src/http.rs       HTTP-клиент и утилиты
src-tauri/src/download.rs   менеджер загрузок с прогрессом
src-tauri/src/installer.rs  список/удаление установленных модов
src-tauri/src/sources/      worldofmods.rs, beamngweb.rs, github.rs, beamng.rs (+ mod.rs, SourceError)
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

## Токен BeamNG

Токен нужен только для вкладки «Репозиторий BeamNG (токен)»: токен генерирует сама
BeamNG.drive при входе в аккаунт beamng.com (отдельного сайта для его получения нет).
Скопируйте его из раздела «Репозиторий» внутри игры и вставьте в настройки.
Вкладки WorldOfMods, «Официальный сайт BeamNG» и «GitHub-релизы» работают без токена.

## Логи

Подробный лог приложения пишется в файл `beamng.log` в каталоге данных приложения
(`app_log_dir`, например `~/.local/share/com.beamng.mod-downloader/logs/` на Linux).
В него попадают: запуск приложения, смена папки модов и токена,
каждый invoke-вызов, ошибки поиска/скачивания (включая rate limit GitHub), ну и консоль
фронтенда (через `attachConsole` плагина `tauri-plugin-log`). В настройках есть кнопка
«Открыть папку логов».

## Конфиг

- Linux: `~/.config/beamng-mod-downloader/config.json`
- Windows: `%APPDATA%/beamng-mod-downloader/config.json`
- macOS: `~/Library/Application Support/beamng-mod-downloader/config.json`

Поля: `mods_folder` — путь к папке модов; `repo_token` — токен BeamNG; `theme` — `"dark"`/
`"light"`; `accent` — HEX-цвет; `installed_sort` — `"date"`/`"name"`/`"size"`;
`installed_collapsed` — булево; `card_size` — `"compact"`/`"normal"`/`"large"`.
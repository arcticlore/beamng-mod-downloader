# Bimka Mod Installer

Кроссплатформенный установщик модов для **BeamNG.drive** на Tauri 2 (Rust) + React.

Возможности:

- автообнаружение папок модов (Linux / Windows / macOS), ручной выбор любой папки;
- три источника модов:
  - **WorldOfMods** — без авторизации, категории «Все / Авто / Карты / Мото / Авиа», пагинация;
  - **Официальный сайт BeamNG** — без токена; категории: Авто, Карты и террейны, Сценарии, Автоматизация, Ландшафт, Скины, Звуки, UI и приложения, Track Builder, Номерные знаки, Моды модов; скачивание идёт напрямую через beamng.com;
  - **Репозиторий BeamNG (токен)** — поиск и детали через `api.beamng.com` (требуется токен аккаунта, вводится в настройках); вкладка подходит для тех, кто уже имеет доступ к API;
- карточки модов: имя, автор, аватарка, описание, счётчик скачиваний, размер;
- скачивание с прогрессом (текущая скорость, байты), установка прямо в папку модов (папку `mods` текущей версии);
- вкладка «Установленные» со списком `.zip`, удаление;
- конфиг хранится в `~/.config/bimka-mod-installer/config.json` (`modsFolder`, `repoToken`).

## Структура

```
src/                        React-интерфейс (TypeScript, Vite)
src/api.ts                  тонкая обёртка над командами Tauri
src/components/             ModCard, ModsBrowser, DetailModal, InstalledPanel, SettingsModal, ProgressBar
src-tauri/src/lib.rs        команды Tauri и AppState
src-tauri/src/config.rs     чтение/запись конфига
src-tauri/src/game.rs       поиск папок модов
src-tauri/src/http.rs       HTTP-клиент и утилиты
src-tauri/src/download.rs   менеджер загрузок с прогрессом
src-tauri/src/installer.rs  список/удаление установленных модов
src-tauri/src/sources/      worldofmods.rs, beamngweb.rs, beamng.rs (+ mod.rs, SourceError)
```

## Сборка

Предпосылки (Fedora/RHEL):

```bash
sudo dnf install webkit2gtk4.1-devel librsvg2-devel
```

Ubuntu/Debian: `libwebkit2gtk-4.1-dev librsvg2-dev`.

Сборка и запуск:

```bash
cd bimka-mod-installer
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

Токен нужен только для вкладки «Репозиторий BeamNG (токен)»: выдаётся на сайте BeamNG (`https://www.beamng.com/account/upgrades/`) и отправляется как `Authorization: Bearer <token>`. Вкладки WorldOfMods и «Официальный сайт BeamNG» работают без токена.

## Конфиг

- Linux: `~/.config/bimka-mod-installer/config.json`
- Windows: `%APPDATA%/bimka-mod-installer/config.json`
- macOS: `~/Library/Application Support/bimka-mod-installer/config.json`

Поля: `mods_folder` — путь к папке модов; `repo_token` — токен BeamNG.
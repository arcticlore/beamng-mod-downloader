# BeamNG Mod Downloader

Cross-platform mod installer for **BeamNG.drive** built on Tauri 2 (Rust) + React.

Features:

- auto-detection of mods folders (Linux / Windows / macOS), manual selection of any folder;
- a single **source registry**: every source is described by a descriptor (group,
  trust level, categories, filename rule, install mode, status) and managed in
  Settings → Sources. The frontend doesn't keep its own copy of the source list —
  it gets it via the `get_source_registry` command;
- enabled sources can be selected for search: a single source — honest source pagination;
  several — safe aggregation (several pages per source, deduplication by `source:id`,
  partial errors shown in a separate banner, sources can be hidden with chips);
- three mod sources:
  - **WorldOfMods** — no authorization, categories All / Cars / Maps / Motorcycles / Aircraft, pagination;
  - **Official BeamNG website** — no token; categories: Cars, Maps & Terrains, Scenarios, Automation, Landscape, Skins, Sounds, UI Apps, Track Builder, License Plates, Mods mods; downloads straight from beamng.com;
  - **GitHub releases** — search by `topic:beamng` via the GitHub Search API, downloading `.zip` from the latest release directly from `objects.githubusercontent.com`; no authorization required (in-memory API response caching);
- a disabled source is **never queried over the network** (search, details, install,
  updates) — gates live in the Tauri commands;
- mod browser sorting: relevance / recency / name / popularity / size;
- mod cards: name, author, thumbnail, description, download count, size;
- interface customization: dark/light theme, accent color, card size, installed-panel sort and collapse;
- **Russian and English interface** — the language is switched in Settings → Appearance
  (`language` in config.json, default detected from the system locale; old configs stay in Russian);
- logging: `beamng.log` in the app data directory (current speed, network errors, actions),
  frontend console mirror-saved there too; a "Open the logs folder" button lives in Settings;
- downloads with progress (current speed, bytes), installation straight into the mods folder
  (the `mods` folder of the current version);
- an "Installed" tab with the `.zip` list and removal;
- config stored at `~/.config/beamng-mod-downloader/config.json` (`mods_folder`, `theme`, `accent`,
  `installed_sort`, `installed_collapsed`, `card_size`, `language`, `enabled_sources`, `selected_sources`).

## Structure

```
src/                        React frontend (TypeScript, Vite)
src/api.ts                  thin wrapper over Tauri commands (invoke with logging)
src/theme.ts                theme and accent color application
src/i18n/                   interface dictionaries: ru.ts / en.ts, t()/tp() helpers, LanguageContext
src/sources.ts              pure source-selection logic (presets, file names, deduplication)
src/SourcesContext.tsx      registry and source-selection state for the whole frontend
src/components/             ModCard, ModsBrowser, DetailModal, InstalledPanel, SettingsModal (incl. Sources section), ProgressBar
src-tauri/src/lib.rs        Tauri commands and AppState (incl. enabled-source gates)
src-tauri/src/config.rs     config read/write (+ source-selection migration)
src-tauri/src/i18n.rs       backend locale: Lang, set_lang, t/tf/tfp (RU/EN)
src-tauri/src/game.rs       mods folder discovery
src-tauri/src/http.rs       HTTP client and utilities
src-tauri/src/download.rs   download manager with progress
src-tauri/src/installer.rs  installed-mods list/removal
src-tauri/src/sources/      registry.rs (single source list) + worldofmods.rs, beamngweb.rs, github.rs (+ mod.rs, SourceError)
```

## Building

Prerequisites (Fedora/RHEL):

```bash
sudo dnf install webkit2gtk4.1-devel librsvg2-devel
```

Ubuntu/Debian: `libwebkit2gtk-4.1-dev librsvg2-dev`.

Build and run:

```bash
cd beamng-mod-downloader
npm install
npx tauri dev          # dev mode
npx tauri build        # release build (AppImage/.deb/.rpm etc.)
```

Release builds are also produced automatically in GitHub Actions on every push to `main`
(artifacts in Actions → build → Artifacts: AppImage/.deb/.rpm).

Rust tests:

```bash
cd src-tauri
cargo test
```

Frontend tests (pure functions, no browser):

```bash
npm test
```

## Logs

A detailed app log is written to `beamng.log` in the app data directory
(`app_log_dir`, e.g. `~/.local/share/com.beamng.mod-downloader/logs/` on Linux).
It contains: app startup, mods folder changes, every invoke call, search/download errors
(including GitHub rate limits), and the frontend console (via the `attachConsole` plugin of
`tauri-plugin-log`). A "Open the logs folder" button lives in Settings.

## Config

- Linux: `~/.config/beamng-mod-downloader/config.json`
- Windows: `%APPDATA%/beamng-mod-downloader/config.json`
- macOS: `~/Library/Application Support/beamng-mod-downloader/config.json`

Fields: `mods_folder` — path to the mods folder; `theme` — `"dark"`/
`"light"`; `accent` — HEX color; `installed_sort` — `"date"`/`"name"`/`"size"`;
`installed_collapsed` — boolean; `card_size` — `"compact"`/`"normal"`/`"large"`;
`language` — `"ru"`/`"en"` (omission = system locale detection for new configs, `"ru"`
for old configs written before this field existed).

### Source selection (`enabled_sources`, `selected_sources`)

- `enabled_sources` — array of source ids that are allowed network requests.
  If the field is absent, on config load:
  - for a **new** config (no file existed yet) the recommended sources are enabled
    (`beamngweb`, `github`; WorldOfMods is outside the recommended set);
  - for an **existing** config from ≤ 0.2.x all three previously available sources
    are enabled (`beamngweb`, `github`, `worldofmods`) — 0.2.x behavior is preserved.
  Unknown/removed ids are dropped on load and never break the config.
- `selected_sources` — array of ids active for search, or `null` (default) = "all enabled".
  The field is saved only if the selection was changed manually (a "Search" checkbox was
  cleared or a preset applied).

---

> The code is provided "as is". The author no longer remembers how it works. Good luck!
mod archive;
mod config;
mod download;
mod game;
mod http;
mod i18n;
mod installer;
mod ledger;
mod models;
mod sources;
mod urlguard;

use config::Config;
use http::build_client;
use log::{debug, error, info, warn};
use models::{
    AppSettings, DownloadState, InstallRequest, InstalledMod, IntegrityReport, ModDetail,
    ModSearchResult, ModUpdate, ModsFolderCandidate, SourceCategory, SourceDescriptor,
    SourceSelection,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{Manager, State};

/// Сколько времени держим ответы поиска в памяти, чтобы возврат на страницу
/// / переключение категорий не били по сети повторно.
const SEARCH_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(20);
/// Верхний предел записей кэша поиска (защита от роста памяти при долгой сессии).
const SEARCH_CACHE_MAX: usize = 512;

pub struct AppState {
    pub client: reqwest::Client,
    pub config: Mutex<Config>,
    pub downloads: download::DownloadTable,
    pub cancels: download::CancelTable,
    pub listing_cache: Arc<Mutex<HashMap<String, (Instant, ModSearchResult)>>>,
}

#[tauri::command]
fn get_mods_folder(state: State<'_, AppState>) -> Option<String> {
    state.config.lock().ok()?.mods_folder.clone()
}

#[tauri::command]
fn detect_mods_folders() -> Vec<ModsFolderCandidate> {
    game::detect_mods_folders()
}

#[tauri::command]
fn set_mods_folder(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(i18n::tf(
            "каталог не существует: {0}",
            "directory does not exist: {0}",
            &[path],
        ));
    }
    if !game::is_plausible_mods_folder(&dir) {
        return Err(i18n::tf(
            "в каталоге {0} нет архивов модов (.zip). Убедитесь, что выбрана папка mods BeamNG.drive",
            "no mod archives (.zip) in {0}. Make sure the mods folder of BeamNG.drive is selected",
            &[&path],
        ));
    }
    info!("папка модов задана: {path}");
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.mods_folder = Some(path);
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_mods_folder_force(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(i18n::tf(
            "каталог не существует: {0}",
            "directory does not exist: {0}",
            &[path],
        ));
    }
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.mods_folder = Some(path);
    cfg.save().map_err(|e| e.to_string())
}

/// Открывает ссылку в системном браузере (автоматический переход на нужную
/// страницу: вход в beamng.com, документация и т.п.).
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(i18n::t(
            "разрешены только http/https ссылки",
            "only http/https links are allowed",
        ));
    }
    #[cfg(target_os = "linux")]
    let opened = std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .and_then(|mut c| c.wait())
        .is_ok();
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .and_then(|mut c| c.wait())
        .is_ok();
    #[cfg(target_os = "windows")]
    let opened = std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn()
        .and_then(|mut c| c.wait())
        .is_ok();

    if opened {
        info!("открыта ссылка: {url}");
        Ok(())
    } else {
        error!("не удалось открыть ссылку: {url}");
        Err(i18n::t(
            "не удалось открыть браузер",
            "could not open the browser",
        ))
    }
}

#[tauri::command]
async fn search_mods(
    state: State<'_, AppState>,
    source: String,
    query: Option<String>,
    category: Option<String>,
    page: u32,
    order: Option<String>,
) -> Result<ModSearchResult, String> {
    let key = format!(
        "{source}|{}|{}|{page}|{}",
        query.as_deref().unwrap_or(""),
        category.as_deref().unwrap_or(""),
        order.as_deref().unwrap_or("")
    );

    {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        require_source_enabled(&cfg, &source)?;
    }

    if let Ok(cache) = state.listing_cache.lock() {
        if let Some((at, res)) = cache.get(&key) {
            if at.elapsed() < SEARCH_CACHE_TTL {
                debug!("search cache hit: {key}");
                return Ok(res.clone());
            }
        }
    }

    let result = sources::search(
        &state.client,
        &source,
        query.as_deref(),
        category.as_deref(),
        page,
        order.as_deref(),
    )
    .await
    .map_err(|e| {
        warn!("search error ({source}): {e}");
        e.to_string()
    })?;

    if let Ok(mut cache) = state.listing_cache.lock() {
        // Выметаем просроченные записи при каждой вставке — этим же
        // ограничиваем размер кэша (хранятся только свежие ответы).
        cache.retain(|_, (at, _)| at.elapsed() < SEARCH_CACHE_TTL);
        if cache.len() >= SEARCH_CACHE_MAX {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), result.clone()));
    }
    Ok(result)
}

#[tauri::command]
async fn get_mod_detail(
    state: State<'_, AppState>,
    source: String,
    mod_id: String,
    key: String,
) -> Result<ModDetail, String> {
    debug!("detail: source={source}, id={mod_id}");
    {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        require_source_enabled(&cfg, &source)?;
    }
    let result = sources::detail(&state.client, &source, &mod_id, &key)
        .await
        .map_err(|e| {
            warn!("detail error ({source}): {e}");
            e.to_string()
        })?;
    Ok(result)
}

#[tauri::command]
fn get_categories(source: String) -> Vec<SourceCategory> {
    sources::categories(&source)
}

// --- Единый registry источников + выбор источника (enabled/selected) ---

#[tauri::command]
fn get_source_registry() -> Vec<SourceDescriptor> {
    sources::registry::registry()
}

#[tauri::command]
fn get_source_selection(state: State<'_, AppState>) -> SourceSelection {
    let cfg = state.config.lock().ok();
    let c = cfg.as_ref();
    SourceSelection {
        enabled: c.map(|g| g.enabled_sources()).unwrap_or_default(),
        selected: c.and_then(|g| g.selected_sources.clone()),
    }
}

#[tauri::command]
fn set_source_enabled(
    state: State<'_, AppState>,
    source_id: String,
    enabled: bool,
) -> Result<(), String> {
    if !sources::registry::is_known(&source_id) {
        return Err(i18n::tf(
            "неизвестный источник `{0}`",
            "unknown source `{0}`",
            &[&source_id],
        ));
    }
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    let mut list = cfg.enabled_sources();
    let present = list.iter().any(|id| id == &source_id);
    if enabled && !present {
        list.push(source_id.clone());
        list.sort();
        list.dedup();
    } else if !enabled {
        list.retain(|id| id != &source_id);
    }
    cfg.enabled_sources = Some(list);
    // Отключаемый источник выкидываем и из активных — disabled не запрашивается.
    if let Some(sel) = cfg.selected_sources.as_mut() {
        sel.retain(|id| id != &source_id);
        sel.sort();
        sel.dedup();
    }
    info!("источник {source_id}: enabled={enabled}");
    cfg.save().map_err(|e| {
        error!("не сохранить config: {e}");
        e.to_string()
    })
}

/// `None` = «все enabled» (автоматически), `Some(list)` — явный выбор.
#[tauri::command]
fn set_source_selected(
    state: State<'_, AppState>,
    source_ids: Option<Vec<String>>,
) -> Result<(), String> {
    let mut ids = match source_ids {
        None => {
            let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
            cfg.selected_sources = None;
            return cfg.save().map_err(|e| {
                error!("не сохранить config: {e}");
                e.to_string()
            });
        }
        Some(v) => v,
    };
    ids.sort();
    ids.dedup();
    for id in &ids {
        if !sources::registry::is_known(id) {
            return Err(i18n::tf(
                "неизвестный источник `{0}`",
                "unknown source `{0}`",
                &[id],
            ));
        }
    }
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.selected_sources = Some(ids);
    cfg.save().map_err(|e| {
        error!("не сохранить config: {e}");
        e.to_string()
    })
}

/// Сброс выбора источников к безопасным defaults (новый конфиг): включаем
/// только рекомендуемые, активные — «все enabled».
#[tauri::command]
fn reset_sources_to_defaults(state: State<'_, AppState>) -> Result<(), String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.enabled_sources = Some(sources::registry::default_enabled_ids(true));
    cfg.selected_sources = None;
    info!("выбор источников сброшен к рекомендуемым defaults");
    cfg.save().map_err(|e| {
        error!("не сохранить config: {e}");
        e.to_string()
    })
}

/// Гейт: источник должен быть enabled, иначе ни одного сетевого запроса.
fn require_source_enabled(config: &Config, source: &str) -> Result<(), String> {
    let enabled = config.enabled_sources();
    if sources::can_query(&enabled, source) {
        Ok(())
    } else if sources::registry::is_known(source) {
        Err(i18n::tf(
            "источник «{0}» отключён в настройках — включите его, чтобы искать и устанавливать",
            "source \"{0}\" is disabled in settings — enable it to search and install",
            &[source],
        ))
    } else {
        Err(i18n::tf(
            "неизвестный источник «{0}»",
            "unknown source \"{0}\"",
            &[source],
        ))
    }
}

#[tauri::command]
async fn install_mod(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    req: InstallRequest,
) -> Result<String, String> {
    info!(
        "установка: source={}, id={}, name={}",
        req.source, req.mod_id, req.name
    );
    let mods_folder = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        require_source_enabled(&cfg, &req.source)?;
        cfg.mods_folder.clone().ok_or_else(|| {
            i18n::t(
                "не выбрана папка с модами BeamNG",
                "BeamNG mods folder is not selected",
            )
        })?
    };
    download::start(
        &app,
        &state.client,
        &state.downloads,
        &state.cancels,
        &mods_folder,
        req,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_download(state: State<'_, AppState>, key: String) -> Result<(), String> {
    download::cancel(&state.cancels, &key).map_err(|e| e.to_string())
}

/// Обновляет один установленный лаунчером мод до актуальной версии с источника.
#[tauri::command]
async fn update_mod(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    filename: String,
) -> Result<String, String> {
    info!("обновление мода: {filename}");
    let mods_folder = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        let entry = ledger::load().get(&filename).cloned().ok_or_else(|| {
            i18n::tf(
                "мод `{0}` не был установлен лаунчером",
                "mod `{0}` was not installed by the launcher",
                &[&filename],
            )
        })?;
        require_source_enabled(&cfg, &entry.source)?;
        cfg.mods_folder.clone().ok_or_else(|| {
            i18n::t(
                "не выбрана папка с модами BeamNG",
                "BeamNG mods folder is not selected",
            )
        })?
    };
    download::update(
        &app,
        &state.client,
        &state.downloads,
        &state.cancels,
        &mods_folder,
        &filename,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Проверяет целостность установленных архивов: структурную валидность zip
/// и совпадение SHA-256 с записью ledger для модов, установленных лаунчером.
#[tauri::command]
async fn verify_installed(state: State<'_, AppState>) -> Result<Vec<IntegrityReport>, String> {
    let mods_folder = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .mods_folder
        .clone()
        .ok_or_else(|| {
            i18n::t(
                "не выбрана папка с модами BeamNG",
                "BeamNG mods folder is not selected",
            )
        })?;
    let list =
        installer::list_installed(&PathBuf::from(&mods_folder)).map_err(|e| e.to_string())?;
    let ledger = ledger::load();
    tokio::task::spawn_blocking(move || {
        let mut reports: Vec<IntegrityReport> = Vec::with_capacity(list.len());
        for it in list {
            let path = PathBuf::from(&it.path);
            let (zip_ok, entries, sha256) = match archive::validate_zip(&path) {
                Ok(summary) => (true, summary.entries, archive::sha256_file(&path).ok()),
                Err(_) => (false, 0, None),
            };
            let tracked = ledger.get(&it.filename).and_then(|e| e.sha256.clone());
            let hash_ok = if zip_ok {
                match (&sha256, &tracked) {
                    (Some(a), Some(b)) => Some(a == b),
                    _ => None,
                }
            } else {
                None
            };
            reports.push(IntegrityReport {
                filename: it.filename.clone(),
                size_bytes: it.size_bytes,
                zip_ok,
                entries,
                sha256,
                tracked_sha256: tracked,
                hash_ok,
                error: if zip_ok {
                    None
                } else {
                    Some(i18n::t(
                        "архив повреждён или усечён",
                        "archive is corrupted or truncated",
                    ))
                },
            });
        }
        reports.sort_by(|a, b| a.filename.cmp(&b.filename));
        reports
    })
    .await
    .map_err(|e| {
        i18n::tf(
            "проверка целостности прервана: {0}",
            "integrity check aborted: {0}",
            &[&e.to_string()],
        )
    })
}

#[tauri::command]
async fn get_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadState>, String> {
    Ok(download::snapshot(&state.downloads).await)
}

#[tauri::command]
fn list_installed(state: State<'_, AppState>) -> Result<Vec<InstalledMod>, String> {
    let mods_folder = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .mods_folder
        .clone()
        .ok_or_else(|| {
            i18n::t(
                "не выбрана папка с модами BeamNG",
                "BeamNG mods folder is not selected",
            )
        })?;
    let mut items =
        installer::list_installed(&PathBuf::from(mods_folder)).map_err(|e| e.to_string())?;
    let ledger = ledger::load();
    for it in &mut items {
        if let Some(e) = ledger.get(&it.filename) {
            it.key = Some(e.key.clone());
            it.published = e.published.clone();
        }
    }
    Ok(items)
}

#[tauri::command]
fn remove_installed(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let mods_folder = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .mods_folder
        .clone()
        .ok_or_else(|| {
            i18n::t(
                "не выбрана папка с модами BeamNG",
                "BeamNG mods folder is not selected",
            )
        })?;

    // Защита от выхода за пределы папки модов: допускаем только относительные
    // пути без `..`, которые могут прийти из сканирования файловой системы.
    let rel = PathBuf::from(&path);
    if rel.is_absolute()
        || rel.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
            )
        })
    {
        return Err(i18n::t(
            "недопустимый путь для удаления",
            "invalid path for deletion",
        ));
    }
    let full = PathBuf::from(&mods_folder).join(&rel);
    installer::remove_file(&full.display().to_string()).map_err(|e| e.to_string())?;
    ledger::remove(&path);
    Ok(())
}

/// Проверяет установленные лаунчером моды на наличие обновлений:
/// сравнивает дату публикации на источнике с той, что была при установке.
/// Источником истины служит ledger: там лежат настоящие source/key и версия
/// на момент установки (поля `InstalledMod.source/key` из выборки для этого
/// ненадёжны — installer помечает моды как local/repo).
#[tauri::command]
async fn check_updates(
    state: State<'_, AppState>,
    items: Vec<InstalledMod>,
) -> Result<Vec<ModUpdate>, String> {
    let ledger = ledger::load();
    let enabled = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.enabled_sources()
    };
    let mut out = Vec::new();
    for it in items {
        let Some(entry) = ledger.get(&it.filename) else {
            // Мод не устанавливали через лаунчер — пропускаем.
            continue;
        };
        let filename = it.filename.clone();
        let source = entry.source.clone();
        let key = entry.key.clone();
        let installed = entry.published.clone();
        if !sources::can_query(&enabled, &source) {
            // disabled источник не опрашивается — источником истины остаётся ledger.
            continue;
        }
        // mod_id ни для чего важного не используется: detail ходит по key.
        let result = sources::detail(&state.client, &source, &filename, &key).await;
        let (latest, error) = match result {
            Ok(d) => (d.item.published, None),
            Err(e) => {
                warn!("проверка обновления {filename} ({source}) не удалась: {e}");
                (None, Some(e.to_string()))
            }
        };
        let has_update = match (&installed, &latest) {
            (Some(a), Some(b)) => a != b,
            _ => false,
        };
        out.push(ModUpdate {
            filename,
            source,
            key,
            installed_published: installed,
            latest_published: latest,
            has_update,
            error,
        });
    }
    Ok(out)
}

#[tauri::command]
fn get_app_settings(state: State<'_, AppState>) -> AppSettings {
    let cfg = state.config.lock().ok();
    let c = cfg.as_ref();
    AppSettings {
        theme: c.and_then(|v| v.theme.clone()),
        accent: c.and_then(|v| v.accent.clone()),
        installed_sort: c.and_then(|v| v.installed_sort.clone()),
        installed_collapsed: c.and_then(|v| v.installed_collapsed),
        card_size: c.and_then(|v| v.card_size.clone()),
        style: Some(
            c.map(|v| v.style_str())
                .unwrap_or_else(|| "material".to_string()),
        ),
        language: Some(
            c.map(|v| v.language_str())
                .unwrap_or_else(|| "ru".to_string()),
        ),
    }
}

#[tauri::command]
fn set_app_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.theme = settings.theme.clone();
    cfg.accent = settings.accent.clone();
    cfg.installed_sort = settings.installed_sort.clone();
    cfg.installed_collapsed = settings.installed_collapsed;
    cfg.card_size = settings.card_size.clone();
    match settings.style.as_deref() {
        Some("material") | Some("classic") => cfg.style = settings.style.clone(),
        _ => cfg.style = None,
    }
    cfg.language = settings
        .language
        .clone()
        .filter(|l| {
            l == "ru" || l == "en" || l == "ru-RU" || l == "en-US" || l == "ru_RU" || l == "en_US"
        })
        .map(|l| if l.len() > 2 { &l[..2] } else { &l }.to_string());
    i18n::set_lang(i18n::Lang::parse(cfg.language_str().as_str()));
    info!(
        "настройки интерфейса: theme={:?}, accent={:?}, sort={:?}, collapsed={:?}, card={:?}, style={}, lang={}",
        settings.theme,
        settings.accent,
        settings.installed_sort,
        settings.installed_collapsed,
        settings.card_size,
        cfg.style_str(),
        cfg.language_str()
    );
    cfg.save().map_err(|e| {
        error!("не сохранить настройки: {e}");
        e.to_string()
    })
}

/// Открывает системный файловый менеджер с каталогом логов приложения.
#[tauri::command]
fn open_log_dir(app: tauri::AppHandle) -> Result<(), String> {
    let dir = app.path().app_log_dir().map_err(|e| {
        i18n::tf(
            "не найти каталог логов: {0}",
            "log directory not found: {0}",
            &[&e.to_string()],
        )
    })?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let opened = open_path(&dir.display().to_string());
    if opened {
        info!("открыта папка логов: {}", dir.display());
        Ok(())
    } else {
        error!("не удалось открыть папку логов: {}", dir.display());
        Err(i18n::t(
            "не удалось открыть папку логов",
            "could not open the log folder",
        ))
    }
}

fn open_path(path: &str) -> bool {
    #[cfg(target_os = "linux")]
    let opened = std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .and_then(|mut c| c.wait())
        .is_ok();
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("open")
        .arg(path)
        .spawn()
        .and_then(|mut c| c.wait())
        .is_ok();
    #[cfg(target_os = "windows")]
    let opened = std::process::Command::new("explorer")
        .arg(path)
        .spawn()
        .and_then(|mut c| c.wait())
        .is_ok();
    opened
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("beamng.log".to_string()),
                    }),
                ])
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .max_file_size(10 * 1024 * 1024)
                .build(),
        )
        .setup(|app| {
            let client = build_client()?;
            let cfg = Config::load();
            i18n::set_lang(i18n::Lang::parse(&cfg.language_str()));
            let state = AppState {
                client,
                config: Mutex::new(cfg),
                downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
                cancels: Arc::new(std::sync::Mutex::new(HashMap::new())),
                listing_cache: Arc::new(Mutex::new(HashMap::new())),
            };
            app.manage(state);
            info!(
                "BeamNG Mod Downloader v{} запущен ({})",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_mods_folder,
            detect_mods_folders,
            set_mods_folder,
            set_mods_folder_force,
            open_url,
            search_mods,
            get_mod_detail,
            get_categories,
            get_source_registry,
            get_source_selection,
            set_source_enabled,
            set_source_selected,
            reset_sources_to_defaults,
            install_mod,
            get_downloads,
            cancel_download,
            update_mod,
            verify_installed,
            list_installed,
            remove_installed,
            check_updates,
            get_app_settings,
            set_app_settings,
            open_log_dir
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска BeamNG Mod Downloader");
}

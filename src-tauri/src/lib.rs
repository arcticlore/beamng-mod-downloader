mod config;
mod download;
mod game;
mod http;
mod installer;
mod ledger;
mod models;
mod sources;

use config::Config;
use http::build_client;
use log::{debug, error, info, warn};
use models::{
    AppSettings, DownloadState, InstallRequest, InstalledMod, ModDetail, ModSearchResult,
    ModsFolderCandidate, ModUpdate, SourceCategory,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{Manager, State};

/// Сколько времени держим ответы поиска в памяти, чтобы возврат на страницу
/// / переключение категорий не били по сети повторно.
const SEARCH_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(20);

pub struct AppState {
    pub client: reqwest::Client,
    pub config: Mutex<Config>,
    pub downloads: download::DownloadTable,
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
        return Err(format!("каталог не существует: {path}"));
    }
    if !game::is_plausible_mods_folder(&dir) {
        return Err(format!(
            "в каталоге {path} нет архивов модов (.zip). Убедитесь, что выбрана папка mods BeamNG.drive"
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
        return Err(format!("каталог не существует: {path}"));
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
        return Err("разрешены только http/https ссылки".to_string());
    }
    #[cfg(target_os = "linux")]
    let opened = std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .is_ok();
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("open").arg(&url).spawn().is_ok();
    #[cfg(target_os = "windows")]
    let opened = std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn()
        .is_ok();

    if opened {
        info!("открыта ссылка: {url}");
        Ok(())
    } else {
        error!("не удалось открыть ссылку: {url}");
        Err("не удалось открыть браузер".to_string())
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
    let mods_folder = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .mods_folder
        .clone()
        .ok_or_else(|| "не выбрана папка с модами BeamNG".to_string())?;
    download::start(&app, &state.client, &state.downloads, &mods_folder, req)
        .await
        .map_err(|e| e.to_string())
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
        .ok_or_else(|| "не выбрана папка с модами BeamNG".to_string())?;
    let mut items = installer::list_installed(&PathBuf::from(mods_folder))
        .map_err(|e| e.to_string())?;
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
        .ok_or_else(|| "не выбрана папка с модами BeamNG".to_string())?;
    let full = PathBuf::from(&mods_folder).join(&path);
    installer::remove_file(&full.display().to_string()).map_err(|e| e.to_string())?;
    ledger::remove(&path);
    Ok(())
}

/// Проверяет установленные лаунчером моды на наличие обновлений:
/// сравнивает дату публикации на источнике с той, что была при установке.
#[tauri::command]
async fn check_updates(
    state: State<'_, AppState>,
    items: Vec<InstalledMod>,
) -> Result<Vec<ModUpdate>, String> {
    let mut out = Vec::new();
    for it in items {
        let key = match &it.key {
            Some(k) => k.clone(),
            None => continue,
        };
        let filename = it.filename.clone();
        let source = it.source.clone();
        let installed = it.published.clone();
        let latest = sources::detail(&state.client, &source, &filename, &key)
            .await
            .ok()
            .and_then(|d| d.item.published);
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
            error: None,
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
    info!(
        "настройки интерфейса: theme={:?}, accent={:?}, sort={:?}, collapsed={:?}, card={:?}",
        settings.theme,
        settings.accent,
        settings.installed_sort,
        settings.installed_collapsed,
        settings.card_size
    );
    cfg.save().map_err(|e| {
        error!("не сохранить настройки: {e}");
        e.to_string()
    })
}

/// Открывает системный файловый менеджер с каталогом логов приложения.
#[tauri::command]
fn open_log_dir(app: tauri::AppHandle) -> Result<(), String> {
    let dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("не найти каталог логов: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let opened = open_path(&dir.display().to_string());
    if opened {
        info!("открыта папка логов: {}", dir.display());
        Ok(())
    } else {
        error!("не удалось открыть папку логов: {}", dir.display());
        Err("не удалось открыть папку логов".to_string())
    }
}

fn open_path(path: &str) -> bool {
    #[cfg(target_os = "linux")]
    let opened = std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .is_ok();
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("open").arg(path).spawn().is_ok();
    #[cfg(target_os = "windows")]
    let opened = std::process::Command::new("explorer")
        .arg(path)
        .spawn()
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
            let state = AppState {
                client,
                config: Mutex::new(Config::load()),
                downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
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
            install_mod,
            get_downloads,
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

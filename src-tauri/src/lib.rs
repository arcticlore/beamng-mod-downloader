mod config;
mod download;
mod game;
mod http;
mod installer;
mod models;
mod sources;

use config::Config;
use http::build_client;
use log::{debug, error, info, warn};
use models::{
    AppSettings, CustomRepo, DownloadState, InstallRequest, InstalledMod, ModDetail, ModItem,
    ModSearchResult, ModsFolderCandidate, SourceCategory,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
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

impl AppState {
    fn repo_token(&self) -> Option<String> {
        self.config
            .lock()
            .ok()
            .and_then(|cfg| cfg.repo_token.clone())
    }
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

#[tauri::command]
fn get_repo_token(state: State<'_, AppState>) -> Option<String> {
    state.repo_token()
}

#[tauri::command]
fn set_repo_token(state: State<'_, AppState>, token: String) -> Result<(), String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.repo_token = if token.trim().is_empty() {
        info!("repo token очищен");
        None
    } else {
        info!("repo token сохранён (len={})", token.trim().len());
        Some(token.trim().to_string())
    };
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
) -> Result<ModSearchResult, String> {
    let token = state.repo_token();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    if let Some(t) = &token {
        t.hash(&mut hasher);
    }
    let key = format!(
        "{source}|{}|{}|{page}|{}",
        query.as_deref().unwrap_or(""),
        category.as_deref().unwrap_or(""),
        hasher.finish()
    );

    if let Ok(cache) = state.listing_cache.lock() {
        if let Some((at, res)) = cache.get(&key) {
            if at.elapsed() < SEARCH_CACHE_TTL {
                debug!("search cache hit: {key}");
                return Ok(res.clone());
            }
        }
    }

    let result = if source == "custom" {
        search_custom(&state, query.as_deref()).await?
    } else {
        sources::search(
            &state.client,
            &source,
            query.as_deref(),
            category.as_deref(),
            page,
            token.as_deref(),
        )
        .await
        .map_err(|e| {
            warn!("search error ({source}): {e}");
            e.to_string()
        })?
    };

    if let Ok(mut cache) = state.listing_cache.lock() {
        cache.insert(key, (Instant::now(), result.clone()));
    }
    Ok(result)
}

async fn search_custom(
    state: &tauri::State<'_, AppState>,
    query: Option<&str>,
) -> Result<ModSearchResult, String> {
    let repos = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .custom_repos
        .clone();

    let mut items: Vec<ModItem> = Vec::with_capacity(repos.len());
    for r in &repos {
        let url = format!("https://github.com/{}", r.full);
        let full = sources::github::repo_from_key(&url).unwrap_or_else(|_| r.full.clone());
        match sources::github::repo_info(&state.client, &full).await {
            Ok(mut it) => {
                it.id = format!("custom:{full}");
                it.source = "custom".to_string();
                if let Some(label) = &r.label {
                    it.name = label.clone();
                }
                items.push(it);
            }
            Err(e) => {
                warn!("custom repo {full}: {e}");
                items.push(ModItem {
                    id: format!("custom:{full}"),
                    source: "custom".to_string(),
                    name: r.label.clone().unwrap_or_else(|| r.full.clone()),
                    thumbnail: None,
                    description: None,
                    key: url,
                    category: None,
                    author: None,
                    published: None,
                    downloads: None,
                    size_bytes: None,
                });
            }
        }
    }

    if let Some(q) = query {
        let ql = q.to_lowercase();
        items.retain(|it| it.name.to_lowercase().contains(&ql));
    }
    debug!("custom: {} репозиториев", items.len());
    Ok(ModSearchResult {
        items,
        total_pages: 1,
        current_page: 1,
    })
}

#[tauri::command]
async fn get_mod_detail(
    state: State<'_, AppState>,
    source: String,
    mod_id: String,
    key: String,
) -> Result<ModDetail, String> {
    let token = state.repo_token();
    debug!("detail: source={source}, id={mod_id}");
    let api_source = if source == "custom" {
        "github"
    } else {
        source.as_str()
    };
    let mut result = sources::detail(&state.client, api_source, &mod_id, &key, token.as_deref())
        .await
        .map_err(|e| {
            warn!("detail error ({source}): {e}");
            e.to_string()
        })?;
    if source == "custom" {
        if let Ok(full) = sources::github::repo_from_key(&key) {
            result.item.source = "custom".to_string();
            result.item.id = format!("custom:{full}");
        }
    }
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
    let (mods_folder, token) = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        (
            cfg.mods_folder
                .clone()
                .ok_or_else(|| "не выбрана папка с модами BeamNG".to_string())?,
            cfg.repo_token.clone(),
        )
    };
    download::start(
        &app,
        &state.client,
        &state.downloads,
        &mods_folder,
        req,
        token,
    )
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
    installer::list_installed(&PathBuf::from(mods_folder)).map_err(|e| e.to_string())
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
    installer::remove_file(&full.display().to_string()).map_err(|e| e.to_string())
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

#[tauri::command]
fn get_custom_repos(state: State<'_, AppState>) -> Vec<CustomRepo> {
    state
        .config
        .lock()
        .map(|cfg| cfg.custom_repos.clone())
        .unwrap_or_default()
}

#[tauri::command]
fn add_custom_repo(state: State<'_, AppState>, repo: String) -> Result<Vec<CustomRepo>, String> {
    let full = normalize_repo(&repo).ok_or_else(|| {
        "Нужен GitHub-репозиторий вида `owner/repo` или ссылка https://github.com/owner/repo"
            .to_string()
    })?;
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    if cfg.custom_repos.iter().any(|r| r.full == full) {
        return Err(format!("репозиторий `{full}` уже добавлен"));
    }
    cfg.custom_repos.push(CustomRepo {
        full: full.clone(),
        label: None,
    });
    cfg.save().map_err(|e| e.to_string())?;
    info!("добавлен пользовательский источник: {full}");
    Ok(cfg.custom_repos.clone())
}

#[tauri::command]
fn remove_custom_repo(state: State<'_, AppState>, full: String) -> Result<Vec<CustomRepo>, String> {
    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.custom_repos.retain(|r| r.full != full);
        cfg.save().map_err(|e| e.to_string())?;
    }
    info!("удалён пользовательский источник: {full}");
    Ok(state
        .config
        .lock()
        .map(|cfg| cfg.custom_repos.clone())
        .unwrap_or_default())
}

/// Нормализует пользовательский ввод `owner/repo` или полную ссылку GitHub
/// в канонический вид `owner/repo`, либо возвращает `None`.
fn normalize_repo(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .unwrap_or(trimmed);
    let full = stripped.trim_end_matches('/');
    let parts: Vec<&str> = full.split('/').collect();
    if parts.len() == 2
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
    {
        Some(format!("{}/{}", parts[0], parts[1]))
    } else {
        None
    }
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
                        file_name: Some("bimka.log".to_string()),
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
                "Bimka Mod Installer v{} запущен ({})",
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
            get_repo_token,
            set_repo_token,
            open_url,
            search_mods,
            get_mod_detail,
            get_categories,
            install_mod,
            get_downloads,
            list_installed,
            remove_installed,
            get_app_settings,
            set_app_settings,
            get_custom_repos,
            add_custom_repo,
            remove_custom_repo,
            open_log_dir
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска Bimka Mod Installer");
}

#[cfg(test)]
mod tests {
    use super::normalize_repo;

    #[test]
    fn normalize_repo_accepts_github_inputs() {
        assert_eq!(
            normalize_repo("BeamMP/BeamMP").as_deref(),
            Some("BeamMP/BeamMP")
        );
        assert_eq!(
            normalize_repo("https://github.com/BeamMP/BeamMP/").as_deref(),
            Some("BeamMP/BeamMP")
        );
        assert_eq!(
            normalize_repo("  http://github.com/a/b  ").as_deref(),
            Some("a/b")
        );
        assert_eq!(normalize_repo("A/B.c-d_e").as_deref(), Some("A/B.c-d_e"));
    }

    #[test]
    fn normalize_repo_rejects_junk() {
        assert!(normalize_repo("invalid-repo").is_none());
        assert!(normalize_repo("a/b/c").is_none());
        assert!(normalize_repo("a/").is_none());
        assert!(normalize_repo("a/ b").is_none());
        assert!(normalize_repo("https://example.com/a/b").is_none());
    }
}

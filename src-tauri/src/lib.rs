mod config;
mod download;
mod game;
mod http;
mod installer;
mod models;
mod sources;

use config::Config;
use http::build_client;
use models::{
    DownloadState, InstallRequest, InstalledMod, ModDetail, ModSearchResult, ModsFolderCandidate,
    SourceCategory,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

pub struct AppState {
    pub client: reqwest::Client,
    pub config: Mutex<Config>,
    pub downloads: download::DownloadTable,
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
        None
    } else {
        Some(token.trim().to_string())
    };
    cfg.save().map_err(|e| e.to_string())
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
    sources::search(
        &state.client,
        &source,
        query.as_deref(),
        category.as_deref(),
        page,
        token.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_mod_detail(
    state: State<'_, AppState>,
    source: String,
    mod_id: String,
    key: String,
) -> Result<ModDetail, String> {
    let token = state.repo_token();
    sources::detail(&state.client, &source, &mod_id, &key, token.as_deref())
        .await
        .map_err(|e| e.to_string())
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

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let client = build_client()?;
            let state = AppState {
                client,
                config: Mutex::new(Config::load()),
                downloads: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            };
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_mods_folder,
            detect_mods_folders,
            set_mods_folder,
            set_mods_folder_force,
            get_repo_token,
            set_repo_token,
            search_mods,
            get_mod_detail,
            get_categories,
            install_mod,
            get_downloads,
            list_installed,
            remove_installed
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска Bimka Mod Installer");
}

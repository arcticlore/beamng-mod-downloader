use crate::models::{DownloadState, InstallRequest};
use crate::sources;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Downloading,
    Done,
    Error,
}

#[derive(Clone)]
pub struct ActiveDownload {
    pub key: String,
    pub name: String,
    pub filename: String,
    pub received: u64,
    pub total: Option<u64>,
    pub speed_bps: u64,
    pub phase: Phase,
    pub error: Option<String>,
}

struct DownloadJob<'a> {
    url: &'a str,
    part_path: PathBuf,
    final_path: PathBuf,
    key: &'a str,
    source: &'a str,
    repo_token: Option<&'a str>,
}

impl ActiveDownload {
    fn to_state(&self) -> DownloadState {
        DownloadState {
            key: self.key.clone(),
            name: self.name.clone(),
            filename: self.filename.clone(),
            received: self.received,
            total: self.total,
            speed_bps: self.speed_bps,
            state: match self.phase {
                Phase::Downloading => "downloading".to_string(),
                Phase::Done => "done".to_string(),
                Phase::Error => "error".to_string(),
            },
            error: self.error.clone(),
        }
    }
}

pub type DownloadTable = Arc<Mutex<HashMap<String, ActiveDownload>>>;

/// Эмитит прогресс в UI не чаще чем раз в ~150 мс.
fn emit_progress(
    app: &tauri::AppHandle,
    table: &DownloadTable,
    key: &str,
    min_interval: std::time::Duration,
    last_emit: &mut Option<Instant>,
) {
    let now = Instant::now();
    if let Some(last) = last_emit {
        if now.duration_since(*last) < min_interval {
            return;
        }
    }
    *last_emit = Some(now);
    if let Ok(map) = table.try_lock() {
        if let Some(dl) = map.get(key) {
            let _ = app.emit("download::progress", dl.to_state());
        }
    }
}

pub async fn start(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    table: &DownloadTable,
    mods_folder: &str,
    req: InstallRequest,
    repo_token: Option<String>,
) -> Result<String> {
    if !sources::validate_source(&req.source) {
        return Err(anyhow!("неизвестный источник `{}`", req.source));
    }
    let mods_dir = PathBuf::from(mods_folder);
    tokio::fs::create_dir_all(&mods_dir)
        .await
        .with_context(|| format!("не удалось создать {}", mods_dir.display()))?;

    let (url, filename) =
        sources::resolve_download(client, &req.source, &req.key, repo_token.as_deref())
            .await
            .map_err(|e| anyhow!("{e}"))?;

    // Если файл уже установлен (есть в папке модов) — не перезаписываем архив,
    // а сообщаем пользователю
    let final_path = mods_dir.join(&filename);
    if final_path.exists() {
        return Err(anyhow!(
            "мод `{filename}` уже установлен в папке модов. Удалите его там, чтобы переустановить."
        ));
    }

    let key = req.mod_id.clone();
    {
        let mut map = table.lock().await;
        if map.contains_key(&key) {
            return Err(anyhow!("загрузка этого мода уже идёт"));
        }
        map.insert(
            key.clone(),
            ActiveDownload {
                key: key.clone(),
                name: req.name.clone(),
                filename: filename.clone(),
                received: 0,
                total: None,
                speed_bps: 0,
                phase: Phase::Downloading,
                error: None,
            },
        );
    }

    let task_key = key.clone();
    let app = app.clone();
    let client = client.clone();
    let table = table.clone();
    let part_path = mods_dir.join(format!(".{filename}.part"));

    tokio::spawn(async move {
        let job = DownloadJob {
            url: &url,
            part_path,
            final_path,
            key: &task_key,
            source: &req.source,
            repo_token: repo_token.as_deref(),
        };
        let result = run_download(&app, &client, &table, &job).await;

        let mut map = table.lock().await;
        let entry = map.get_mut(&task_key);
        match (result, entry) {
            (Ok(()), Some(dl)) => {
                dl.phase = Phase::Done;
                dl.received = dl.total.unwrap_or(dl.received);
                dl.speed_bps = 0;
                let state = dl.to_state();
                let _ = app.emit("download::finished", state);
            }
            (Err(e), Some(dl)) => {
                dl.phase = Phase::Error;
                dl.speed_bps = 0;
                dl.error = Some(e.to_string());
                let state = dl.to_state();
                let _ = app.emit("download::finished", state);
            }
            (Err(e), None) => {
                let _ = app.emit(
                    "download::finished",
                    DownloadState {
                        key: task_key,
                        name: String::new(),
                        filename: String::new(),
                        received: 0,
                        total: None,
                        speed_bps: 0,
                        state: "error".to_string(),
                        error: Some(e.to_string()),
                    },
                );
            }
            (Ok(()), None) => {}
        }
    });

    Ok(key)
}

async fn run_download(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    table: &DownloadTable,
    job: &DownloadJob<'_>,
) -> Result<()> {
    let mut req = client.get(job.url);
    if job.source == "worldofmods" {
        req = req.header(reqwest::header::REFERER, "https://www.worldofmods.com/");
    }
    if job.source == "beamng" {
        if let Some(token) = job.repo_token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
    }

    let response = req
        .send()
        .await
        .context("не удалось установить соединение")?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(anyhow!("репозиторий не принял токен (401)"));
    }
    if !status.is_success() {
        return Err(anyhow!("сервер ответил HTTP {status}"));
    }

    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&job.part_path)
        .await
        .with_context(|| format!("не удалось создать {}", job.part_path.display()))?;

    let mut received: u64 = 0;
    let mut window_start = Instant::now();
    let mut last_received: u64 = 0;
    let mut last_emit: Option<Instant> = None;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("ошибка чтения потока загрузки")?;
        received += chunk.len() as u64;
        file.write_all(&chunk)
            .await
            .context("ошибка записи на диск")?;

        let elapsed = window_start.elapsed().as_secs_f64().max(0.001);
        let speed = ((received - last_received) as f64 / elapsed).max(0.0) as u64;

        if elapsed > 1.5 {
            window_start = Instant::now();
            last_received = received;
        }

        {
            let mut map = table.lock().await;
            if let Some(dl) = map.get_mut(job.key) {
                dl.received = received;
                dl.total = total;
                dl.speed_bps = speed;
            }
        }
        emit_progress(
            app,
            table,
            job.key,
            std::time::Duration::from_millis(150),
            &mut last_emit,
        );
    }

    file.flush().await.ok();
    drop(file);

    if received == 0 {
        let _ = tokio::fs::remove_file(&job.part_path).await;
        return Err(anyhow!("файл пуст — похоже, ссылка устарела"));
    }

    tokio::fs::rename(&job.part_path, &job.final_path)
        .await
        .with_context(|| {
            format!(
                "не удалось переместить архив в {}",
                job.final_path.display()
            )
        })?;

    Ok(())
}

pub async fn snapshot(table: &DownloadTable) -> Vec<DownloadState> {
    let map = table.lock().await;
    map.values().map(ActiveDownload::to_state).collect()
}

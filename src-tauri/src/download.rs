use crate::models::{DownloadState, InstallRequest};
use crate::sources;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use log::{error, info, warn};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex as AsyncMutex;

/// Сколько загрузок может идти одновременно.
pub const MAX_CONCURRENT_DOWNLOADS: usize = 3;

/// Отмены по ключу загрузки: флаг выставляется командой `cancel_download`.
pub type CancelTable = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

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
    cancel: &'a AtomicBool,
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

pub type DownloadTable = Arc<AsyncMutex<HashMap<String, ActiveDownload>>>;

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

async fn cleanup_part(part: &Path) {
    let _ = tokio::fs::remove_file(part).await;
}

/// Скачивает поток в `.part`-файл, считая при этом SHA-256, и проверяет
/// полученный архив структурно (EOCD + центральный каталог). Возвращает
/// hex-SHA256 и число полученных байт. При любой ошибке `.part` удаляется.
#[allow(clippy::too_many_arguments)]
async fn stream_to_part(
    client: &reqwest::Client,
    app: &tauri::AppHandle,
    table: &DownloadTable,
    key: &str,
    source: &str,
    url: &str,
    part: &Path,
    cancel: &AtomicBool,
) -> Result<(String, u64)> {
    if cancel.load(Ordering::Relaxed) {
        return Err(anyhow!(crate::i18n::t(
            "загрузка отменена пользователем",
            "download cancelled by the user"
        )));
    }

    let mut req = client.get(url);
    if source == "worldofmods" {
        req = req.header(reqwest::header::REFERER, "https://www.worldofmods.com/");
    }

    let response = req.send().await.context(crate::i18n::t(
        "не удалось установить соединение",
        "failed to establish a connection",
    ))?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(anyhow!(crate::i18n::t(
            "репозиторий не принял токен (401)",
            "the repository rejected the token (401)"
        )));
    }
    if !status.is_success() {
        return Err(anyhow!(crate::i18n::tf(
            "сервер ответил HTTP {0}",
            "the server responded with HTTP {0}",
            &[&status.to_string()]
        )));
    }

    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&part).await.with_context(|| {
        crate::i18n::tf(
            "не удалось создать {0}",
            "failed to create {0}",
            &[&part.display().to_string()],
        )
    })?;

    let mut hasher = Sha256::new();
    let mut received: u64 = 0;
    let mut window_start = Instant::now();
    let mut last_received: u64 = 0;
    let mut last_emit: Option<Instant> = None;

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            cleanup_part(part).await;
            return Err(anyhow!(crate::i18n::t(
                "загрузка отменена пользователем",
                "download cancelled by the user"
            )));
        }
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                drop(file);
                cleanup_part(part).await;
                return Err(anyhow!(crate::i18n::tf(
                    "ошибка чтения потока загрузки: {0}",
                    "error reading the download stream: {0}",
                    &[&e.to_string()]
                )));
            }
        };
        received += chunk.len() as u64;
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .context(crate::i18n::t("ошибка записи на диск", "disk write error"))?;

        let elapsed = window_start.elapsed().as_secs_f64().max(0.001);
        let speed = ((received - last_received) as f64 / elapsed).max(0.0) as u64;

        if elapsed > 1.5 {
            window_start = Instant::now();
            last_received = received;
        }

        {
            let mut map = table.lock().await;
            if let Some(dl) = map.get_mut(key) {
                dl.received = received;
                dl.total = total;
                dl.speed_bps = speed;
            }
        }
        emit_progress(
            app,
            table,
            key,
            std::time::Duration::from_millis(150),
            &mut last_emit,
        );
    }

    file.flush().await.ok();
    drop(file);

    let (hex, bytes) = (crate::archive::to_hex(&hasher.finalize()), received);

    if received == 0 {
        cleanup_part(part).await;
        return Err(anyhow!(crate::i18n::t(
            "файл пуст — похоже, ссылка устарела",
            "the file is empty — the link may be outdated"
        )));
    }

    // Недосканный архив — не ставим битый мод: при известном размере требуем
    // совпадение полученного объёма с заявленным сервером.
    if let Some(expected) = total {
        if received != expected {
            cleanup_part(part).await;
            return Err(anyhow!(crate::i18n::tf(
                "загрузка оборвалась: получено {0} из {1} байт",
                "download aborted: got {0} of {1} bytes",
                &[&received.to_string(), &expected.to_string()],
            )));
        }
    }

    // Структурная проверка zip: защита от усечённых/повреждённых архивов.
    crate::archive::validate_zip(part).map_err(|e| {
        let _ = std::fs::remove_file(part);
        anyhow!(crate::i18n::tf(
            "архив не прошёл проверку целостности: {0}",
            "the archive failed the integrity check: {0}",
            &[&e.to_string()]
        ))
    })?;

    Ok((hex, bytes))
}

/// Проверяет лимит параллельных загрузок и что файл не скачивается дважды.
async fn ensure_capacity(table: &DownloadTable, key: &str, filename: &str) -> Result<()> {
    let map = table.lock().await;
    if map.len() >= MAX_CONCURRENT_DOWNLOADS {
        return Err(anyhow!(crate::i18n::tfp(
            "одновременно можно скачивать не более {0} мод",
            "одновременно можно скачивать не более {0} мода",
            "одновременно можно скачивать не более {0} модов",
            "no more than {0} mods can be downloaded at once",
            MAX_CONCURRENT_DOWNLOADS as u64,
            &[&MAX_CONCURRENT_DOWNLOADS.to_string()],
        )));
    }
    if map.contains_key(key) {
        return Err(anyhow!(crate::i18n::t(
            "загрузка этого мода уже идёт",
            "this mod is already being downloaded"
        )));
    }
    if map.values().any(|d| d.filename == filename) {
        return Err(anyhow!(crate::i18n::tf(
            "файл `{0}` уже скачивается",
            "the file `{0}` is already being downloaded",
            &[filename]
        )));
    }
    Ok(())
}

fn register_cancel(cancels: &CancelTable, key: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut m) = cancels.lock() {
        m.insert(key.to_string(), flag.clone());
    }
    flag
}

fn unregister_cancel(cancels: &CancelTable, key: &str) {
    if let Ok(mut m) = cancels.lock() {
        m.remove(key);
    }
}

/// Отменяет активную загрузку по её ключу.
pub fn cancel(cancels: &CancelTable, key: &str) -> Result<()> {
    let flag = cancels
        .lock()
        .map_err(|e| {
            anyhow!(crate::i18n::tf(
                "внутренняя ошибка: {0}",
                "internal error: {0}",
                &[&e.to_string()]
            ))
        })?
        .get(key)
        .cloned();
    match flag {
        Some(f) => {
            info!("отмена загрузки: {key}");
            f.store(true, Ordering::Relaxed);
            Ok(())
        }
        None => Err(anyhow!(crate::i18n::tf(
            "активная загрузка с ключом `{0}` не найдена",
            "no active download with key `{0}`",
            &[key]
        ))),
    }
}

pub async fn start(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    table: &DownloadTable,
    cancels: &CancelTable,
    mods_folder: &str,
    req: InstallRequest,
) -> Result<String> {
    if !sources::validate_source(&req.source) {
        return Err(anyhow!(crate::i18n::tf(
            "неизвестный источник `{0}`",
            "unknown source `{0}`",
            &[&req.source]
        )));
    }
    let mods_dir = PathBuf::from(mods_folder);
    tokio::fs::create_dir_all(&mods_dir)
        .await
        .with_context(|| {
            crate::i18n::tf(
                "не удалось создать {0}",
                "failed to create {0}",
                &[&mods_dir.display().to_string()],
            )
        })?;

    let (url, filename, source_published) =
        sources::resolve_download(client, &req.source, &req.key)
            .await
            .map_err(|e| anyhow!("{e}"))?;
    info!("resolve: source={} url={url} → {filename}", req.source);

    crate::urlguard::validate_url(&url).map_err(|e| {
        anyhow!(crate::i18n::tf(
            "SSRF-проверка: загрузка запрещена: {0}",
            "SSRF check: download rejected: {0}",
            &[&e.to_string()]
        ))
    })?;

    // Если файл уже установлен (есть в папке модов) — не перезаписываем архив,
    // а сообщаем пользователю
    let final_path = mods_dir.join(&filename);
    if final_path.exists() {
        warn!("мод `{filename}` уже установлен");
        return Err(anyhow!(crate::i18n::tf(
            "мод `{0}` уже установлен в папке модов. Удалите его там, чтобы переустановить.",
            "the mod `{0}` is already installed in the mods folder. Remove it there to reinstall.",
            &[filename],
        )));
    }

    let key = req.mod_id.clone();
    ensure_capacity(table, &key, &filename).await?;
    {
        let mut map = table.lock().await;
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
    let cancels_owned = cancels.clone();
    let part_path = mods_dir.join(format!(".{filename}.part"));
    let cancel = register_cancel(cancels, &key);
    let ledger_source = req.source.clone();
    let ledger_key = req.key.clone();
    let ledger_name = req.name.clone();
    // Версию на источнике лучше брать из той же страницы, что позже
    // сравнивает check_updates (иначе даты будут разными сигналами).
    let ledger_published = source_published.or_else(|| req.published.clone());

    tokio::spawn(async move {
        let job = DownloadJob {
            url: &url,
            part_path,
            final_path,
            key: &task_key,
            source: &req.source,
            cancel: &cancel,
        };
        let result = run_download(&app, &client, &table, &job).await;

        unregister_cancel(&cancels_owned, &task_key);

        let mut map = table.lock().await;
        let entry = map.remove(&task_key);
        match (result, entry) {
            (Ok(sha256), Some(mut dl)) => {
                dl.phase = Phase::Done;
                dl.received = dl.total.unwrap_or(dl.received);
                dl.speed_bps = 0;
                let state = dl.to_state();
                let _ = app.emit("download::finished", state);
                info!("успешно: {filename} ({} байт)", dl.received);
                crate::ledger::upsert(crate::ledger::LedgerEntry {
                    filename: filename.clone(),
                    source: ledger_source,
                    key: ledger_key,
                    name: ledger_name,
                    installed_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    published: ledger_published,
                    sha256: Some(sha256),
                });
            }
            (Err(e), Some(mut dl)) => {
                dl.phase = Phase::Error;
                dl.speed_bps = 0;
                dl.error = Some(e.to_string());
                let state = dl.to_state();
                let _ = app.emit("download::finished", state);
                error!("ошибка загрузки {filename}: {e}");
            }
            (Err(e), None) => {
                error!("ошибка загрузки {task_key}: {e}");
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
            (Ok(_), None) => {}
        }
    });

    Ok(key)
}

async fn run_download(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    table: &DownloadTable,
    job: &DownloadJob<'_>,
) -> Result<String> {
    let (hex, _bytes) = stream_to_part(
        client,
        app,
        table,
        job.key,
        job.source,
        job.url,
        &job.part_path,
        job.cancel,
    )
    .await?;

    tokio::fs::rename(&job.part_path, &job.final_path)
        .await
        .with_context(|| {
            format!(
                "не удалось переместить архив в {}",
                job.final_path.display()
            )
        })?;

    Ok(hex)
}

/// Обновление установленного мода: скачивает актуальную версию, проверяет
/// zip/SHA-256 и атомарно заменяет архив (перенося запись в ledger при смене
/// имени файла).
pub async fn update(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    table: &DownloadTable,
    cancels: &CancelTable,
    mods_folder: &str,
    filename: &str,
) -> Result<String> {
    let entry = crate::ledger::load()
        .get(filename)
        .cloned()
        .ok_or_else(|| {
            anyhow!(crate::i18n::tf(
                "мод `{0}` не был установлен лаунчером — обновлять нечего",
                "the mod `{0}` was not installed by the launcher — nothing to update",
                &[filename]
            ))
        })?;

    // Сначала узнаём актуальную версию (дату публикации) — её и запишем в ledger.
    let detail = sources::detail(client, &entry.source, filename, &entry.key)
        .await
        .map_err(|e| {
            anyhow!(crate::i18n::tf(
                "не удалось получить актуальную версию: {0}",
                "failed to get the current version: {0}",
                &[&e.to_string()]
            ))
        })?;
    let latest_published = detail.item.published;

    let (url, latest_filename, _) = sources::resolve_download(client, &entry.source, &entry.key)
        .await
        .map_err(|e| anyhow!("{e}"))?;
    crate::urlguard::validate_url(&url).map_err(|e| {
        anyhow!(crate::i18n::tf(
            "SSRF-проверка: обновление запрещено: {0}",
            "SSRF check: update rejected: {0}",
            &[&e.to_string()]
        ))
    })?;
    info!("update: {filename} → {latest_filename} ({url})");

    let mods_dir = PathBuf::from(mods_folder);
    tokio::fs::create_dir_all(&mods_dir)
        .await
        .with_context(|| {
            crate::i18n::tf(
                "не удалось создать {0}",
                "failed to create {0}",
                &[&mods_dir.display().to_string()],
            )
        })?;

    let key = format!("update:{filename}");
    ensure_capacity(table, &key, &latest_filename).await?;
    // for 'static spawn task
    let filename_owned = filename.to_string();

    let part_path = mods_dir.join(format!(".{latest_filename}.new.part"));
    let mod_name = if entry.name.is_empty() {
        latest_filename.clone()
    } else {
        entry.name.clone()
    };
    let name = crate::i18n::tf("Обновление: {0}", "Update: {0}", &[&mod_name]);
    {
        let mut map = table.lock().await;
        map.insert(
            key.clone(),
            ActiveDownload {
                key: key.clone(),
                name,
                filename: latest_filename.clone(),
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
    let cancels_owned = cancels.clone();
    let cancel = register_cancel(&cancels_owned, &key);
    let task_cancel = cancel.clone();
    let url = url.clone();
    let source = entry.source.clone();
    let latest_key = detail.item.key.clone();
    let installed_published = entry.published.clone();

    tokio::spawn(async move {
        let task_part = part_path.clone();
        let result = stream_to_part(
            &client,
            &app,
            &table,
            &task_key,
            &source,
            &url,
            &task_part,
            &task_cancel,
        )
        .await;

        unregister_cancel(&cancels_owned, &task_key);

        let mut map = table.lock().await;
        let record = map.remove(&task_key);
        match (result, record) {
            (Ok((hex, _bytes)), Some(mut dl)) => {
                let target = mods_dir.join(&latest_filename);
                // Заменяем старый архив (обновление может сменить имя файла).
                if latest_filename != filename_owned {
                    let _ = tokio::fs::remove_file(mods_dir.join(&filename_owned)).await;
                }
                let _ = tokio::fs::remove_file(&target).await;
                match tokio::fs::rename(&part_path, &target).await {
                    Ok(()) => {
                        dl.phase = Phase::Done;
                        dl.received = dl.total.unwrap_or(dl.received);
                        dl.speed_bps = 0;
                        let state = dl.to_state();
                        let _ = app.emit("download::finished", state);
                        info!("обновлён: {filename_owned} → {latest_filename}");
                        let mut entries = crate::ledger::load();
                        entries.remove(&filename_owned);
                        entries.insert(
                            latest_filename.clone(),
                            crate::ledger::LedgerEntry {
                                filename: latest_filename.clone(),
                                source: source.clone(),
                                key: latest_key,
                                name: mod_name.clone(),
                                installed_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0),
                                published: latest_published.clone().or(installed_published),
                                sha256: Some(hex),
                            },
                        );
                        if let Err(e) = crate::ledger::save(&entries) {
                            warn!("не удалось сохранить ledger после обновления: {e}");
                        }
                    }
                    Err(e) => {
                        dl.phase = Phase::Error;
                        dl.speed_bps = 0;
                        dl.error = Some(crate::i18n::tf(
                            "не удалось заменить архив: {0}",
                            "failed to replace the archive: {0}",
                            &[&e.to_string()],
                        ));
                        let state = dl.to_state();
                        let _ = app.emit("download::finished", state);
                        error!("ошибка замены при обновлении {latest_filename}: {e}");
                    }
                }
            }
            (Err(e), Some(mut dl)) => {
                cleanup_part(&part_path).await;
                dl.phase = Phase::Error;
                dl.speed_bps = 0;
                dl.error = Some(e.to_string());
                let state = dl.to_state();
                let _ = app.emit("download::finished", state);
                error!("ошибка обновления {latest_filename}: {e}");
            }
            (Err(e), None) => {
                cleanup_part(&part_path).await;
                error!("ошибка обновления {task_key}: {e}");
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
            (Ok(_), None) => {}
        }
    });

    Ok(key)
}

pub async fn snapshot(table: &DownloadTable) -> Vec<DownloadState> {
    let map = table.lock().await;
    map.values().map(ActiveDownload::to_state).collect()
}

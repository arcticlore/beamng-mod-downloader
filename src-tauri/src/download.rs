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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as AsyncMutex;

/// Сколько загрузок может идти одновременно.
pub const MAX_CONCURRENT_DOWNLOADS: usize = 3;

/// Верхний предел размера архива при скачивании и локальном импорте — 8 ГиБ
/// (SEC-001). Применяется и к заранее объявленному `Content-Length`, и к
/// фактически записанным байтам потока (включая HTTP-сжатие), и к копированию
/// при импорте — до и во время копирования. Защита от дисковой DoS.
pub const MAX_DOWNLOAD_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// Превышает ли лимит объявленный сервером `Content-Length`? `None` (размер
/// неизвестен) не считается превышением — в этом случае лимит ловит счётчик
/// фактических байт.
pub(crate) fn content_length_exceeds(total: Option<u64>, max_bytes: u64) -> bool {
    matches!(total, Some(total) if total > max_bytes)
}

/// Превышен ли лимит фактически полученным объёмом?
pub(crate) fn exceeds_download_limit(received: u64, max_bytes: u64) -> bool {
    received > max_bytes
}

/// Следующее значение счётчика с checked-арифметикой: `None` при переполнении.
pub(crate) fn next_received(current: u64, inc: u64) -> Option<u64> {
    current.checked_add(inc)
}

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

/// Эмитит прогресс в UI не чаще чем раз в ~150 мс. `None` — в тестах.
fn emit_progress(
    app: Option<&tauri::AppHandle>,
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
    if let Some(app) = app {
        if let Ok(map) = table.try_lock() {
            if let Some(dl) = map.get(key) {
                let _ = app.emit("download::progress", dl.to_state());
            }
        }
    }
}

async fn cleanup_part(part: &Path) {
    let _ = tokio::fs::remove_file(part).await;
}

/// Скачивает поток в `.part`-файл, считая при этом SHA-256, и проверяет
/// полученный архив структурно (EOCD + центральный каталог). Возвращает
/// hex-SHA256 и число полученных байт. При любой ошибке `.part` удаляется.
/// `max_bytes` ограничивает размер: превышающий `Content-Length` отклоняется
/// до скачивания, фактический объём — в процессе записи (SEC-001).
#[allow(clippy::too_many_arguments)]
async fn stream_to_part(
    client: &reqwest::Client,
    app: Option<&tauri::AppHandle>,
    table: &DownloadTable,
    key: &str,
    source: &str,
    url: &str,
    part: &Path,
    cancel: &AtomicBool,
    max_bytes: u64,
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
    // SEC-001: заранее отклоняем объявленный сверхлимитный размер.
    if content_length_exceeds(total, max_bytes) {
        return Err(anyhow!(crate::i18n::tf(
            "файл больше лимита {0} ГиБ — загрузка отменена",
            "the file exceeds the {0} GiB limit — download cancelled",
            &[&(max_bytes / (1024 * 1024 * 1024)).to_string()],
        )));
    }
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
        // SEC-001: checked-арифметика и непрерывный лимит фактических байт
        // (ловит и потоки без Content-Length, и HTTP-сжатие).
        received = match next_received(received, chunk.len() as u64) {
            Some(next) => next,
            None => {
                drop(file);
                cleanup_part(part).await;
                return Err(anyhow!(crate::i18n::t(
                    "переполнение счётчика размера загрузки",
                    "download size counter overflow",
                )));
            }
        };
        if exceeds_download_limit(received, max_bytes) {
            drop(file);
            cleanup_part(part).await;
            return Err(anyhow!(crate::i18n::tf(
                "файл превысил лимит {0} ГиБ — загрузка остановлена",
                "the file exceeded the {0} GiB limit — download stopped",
                &[&(max_bytes / (1024 * 1024 * 1024)).to_string()],
            )));
        }
        hasher.update(&chunk);
        if let Err(e) = file.write_all(&chunk).await {
            drop(file);
            cleanup_part(part).await;
            return Err(anyhow::Error::new(e)
                .context(crate::i18n::t("ошибка записи на диск", "disk write error")));
        }

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

    if let Err(e) = file.flush().await {
        drop(file);
        cleanup_part(part).await;
        return Err(anyhow::Error::new(e).context(crate::i18n::t(
            "ошибка сброса буферов на диск",
            "failed to flush to disk",
        )));
    }
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

/// Копирует файл с лимитом байт (SEC-001): счётчик с checked-арифметикой.
/// Гарантия очистки: при любой ошибке (open/create/read/write/limit/flush)
/// хэндлы сначала закрываются, затем `dst` принудительно удаляется — чтобы
/// на диске не остался частичный `.part`. Используется локальным импортом
/// (та же граница, что у скачивания).
pub(crate) async fn copy_limited(src: &Path, dst: &Path, max_bytes: u64) -> Result<u64, String> {
    let mut src_file = match tokio::fs::File::open(src).await {
        Ok(f) => f,
        Err(e) => {
            return Err(crate::i18n::tf(
                "не удалось открыть {0}: {1}",
                "could not open {0}: {1}",
                &[&src.display().to_string(), &e.to_string()],
            ));
        }
    };
    let mut dst_file = match tokio::fs::File::create(dst).await {
        Ok(f) => f,
        Err(e) => {
            drop(src_file);
            let _ = std::fs::remove_file(dst);
            return Err(crate::i18n::tf(
                "не удалось создать {0}: {1}",
                "failed to create {0}: {1}",
                &[&dst.display().to_string(), &e.to_string()],
            ));
        }
    };
    let result = copy_limited_raw(&mut src_file, &mut dst_file, src, dst, max_bytes).await;
    match result {
        Ok(copied) => Ok(copied),
        Err(e) => {
            drop(src_file);
            drop(dst_file);
            let _ = std::fs::remove_file(dst);
            Err(e)
        }
    }
}

/// Тело копирования без cleanup (очисткой занимается `copy_limited`, чтобы
/// гарантия «закрыть хэндлы → удалить dst» была у всех ошибок единой).
async fn copy_limited_raw(
    src_file: &mut tokio::fs::File,
    dst_file: &mut tokio::fs::File,
    src: &Path,
    dst: &Path,
    max_bytes: u64,
) -> Result<u64, String> {
    let mut buf = vec![0u8; 128 * 1024];
    let mut copied: u64 = 0;
    loop {
        let n = src_file.read(&mut buf).await.map_err(|e| {
            crate::i18n::tf(
                "ошибка чтения {0}: {1}",
                "read error {0}: {1}",
                &[&src.display().to_string(), &e.to_string()],
            )
        })?;
        if n == 0 {
            break;
        }
        copied = copied.checked_add(n as u64).ok_or_else(|| {
            crate::i18n::t(
                "переполнение счётчика размера при импорте",
                "size counter overflow during import",
            )
        })?;
        if exceeds_download_limit(copied, max_bytes) {
            return Err(crate::i18n::tf(
                "файл больше лимита {0} ГиБ — импорт отменён",
                "the file exceeds the {0} GiB limit — import cancelled",
                &[&(max_bytes / (1024 * 1024 * 1024)).to_string()],
            ));
        }
        dst_file.write_all(&buf[..n]).await.map_err(|e| {
            crate::i18n::tf(
                "ошибка записи {0}: {1}",
                "write error {0}: {1}",
                &[&dst.display().to_string(), &e.to_string()],
            )
        })?;
    }
    dst_file.flush().await.map_err(|e| {
        crate::i18n::tf(
            "ошибка записи {0}: {1}",
            "write error {0}: {1}",
            &[&dst.display().to_string(), &e.to_string()],
        )
    })?;
    Ok(copied)
}

/// Запись в ledger только для успешной загрузки (иначе `None`). Изолировано,
/// чтобы ошибка (включая превышение лимита) никогда не меняла ledger.
fn ledger_entry_on_success(
    result: &Result<String>,
    filename: &str,
    source: String,
    key: String,
    name: String,
    published: Option<String>,
) -> Option<crate::ledger::LedgerEntry> {
    let sha256 = match result {
        Ok(sha256) => sha256.clone(),
        Err(_) => return None,
    };
    Some(crate::ledger::LedgerEntry {
        filename: filename.to_string(),
        source,
        key,
        name,
        installed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        published,
        sha256: Some(sha256),
    })
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
        match (&result, entry) {
            (Ok(_), Some(mut dl)) => {
                dl.phase = Phase::Done;
                dl.received = dl.total.unwrap_or(dl.received);
                dl.speed_bps = 0;
                let state = dl.to_state();
                let _ = app.emit("download::finished", state);
                info!("успешно: {filename} ({} байт)", dl.received);
                // Ledger обновляется только при успехе (SEC-001: ошибка —
                // включая превышение лимита — ledger не трогает).
                if let Some(entry) = ledger_entry_on_success(
                    &result,
                    &filename,
                    ledger_source,
                    ledger_key,
                    ledger_name,
                    ledger_published,
                ) {
                    crate::ledger::upsert(entry);
                }
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
        Some(app),
        table,
        job.key,
        job.source,
        job.url,
        &job.part_path,
        job.cancel,
        MAX_DOWNLOAD_BYTES,
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
            Some(&app),
            &table,
            &task_key,
            &source,
            &url,
            &task_part,
            &task_cancel,
            MAX_DOWNLOAD_BYTES,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};

    fn tmp_part(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("bmd-dlp-{}-{}.part", std::process::id(), label))
    }

    /// Однократный HTTP-ответ на 127.0.0.1: фиксированной длины либо chunked.
    fn serve_once(body: Vec<u8>, declared: Option<u64>) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = sock.read(&mut buf);
                let mut head =
                    "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nConnection: close\r\n"
                        .to_string();
                match declared {
                    Some(len) => head.push_str(&format!("Content-Length: {len}\r\n")),
                    None => head.push_str("Transfer-Encoding: chunked\r\n"),
                }
                head.push_str("\r\n");
                let _ = sock.write_all(head.as_bytes());
                match declared {
                    Some(_) => {
                        let _ = sock.write_all(&body);
                    }
                    None => {
                        let _ = sock.write_all(format!("{:x}\r\n", body.len()).as_bytes());
                        let _ = sock.write_all(&body);
                        let _ = sock.write_all(b"\r\n0\r\n\r\n");
                    }
                }
                let _ = sock.shutdown(std::net::Shutdown::Both);
            }
        });
        format!("http://{addr}/m.zip")
    }

    fn build_client() -> reqwest::Client {
        crate::http::build_client().expect("http client for tests")
    }

    // Валидный ZIP с одним stored-файлом данных заданного размера. CFH/EOCD
    // соответствуют тому, как `archive::validate_zip` их разбирает.
    fn stored_zip_payload(name: &str, payload_len: usize) -> Vec<u8> {
        let nameb = name.as_bytes();
        let mut out = Vec::new();
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]); // local sig
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // method: stored
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0u16.to_le_bytes()); // mod date
        out.extend_from_slice(&0u32.to_le_bytes()); // crc32
        out.extend_from_slice(&(payload_len as u32).to_le_bytes()); // c_size
        out.extend_from_slice(&(payload_len as u32).to_le_bytes()); // u_size
        out.extend_from_slice(&(nameb.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(nameb);
        let data = vec![b'z'; payload_len];
        out.extend_from_slice(&data);
        let cd_start = out.len() as u32;
        out.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]); // cd sig
        out.extend_from_slice(&20u16.to_le_bytes()); // version made by
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // method
        out.extend_from_slice(&0u16.to_le_bytes()); // time
        out.extend_from_slice(&0u16.to_le_bytes()); // date
        out.extend_from_slice(&0u32.to_le_bytes()); // crc
        out.extend_from_slice(&(payload_len as u32).to_le_bytes());
        out.extend_from_slice(&(payload_len as u32).to_le_bytes());
        out.extend_from_slice(&(nameb.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(&0u16.to_le_bytes()); // comment
        out.extend_from_slice(&0u16.to_le_bytes()); // disk start
        out.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        out.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        out.extend_from_slice(&(0u32).to_le_bytes()); // local header offset
        out.extend_from_slice(nameb);
        let cd_size = (out.len() as u32) - cd_start;
        out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]); // eocd sig
        out.extend_from_slice(&0u32.to_le_bytes()); // disk + cd disk
        out.extend_from_slice(&1u16.to_le_bytes()); // entries on disk
        out.extend_from_slice(&1u16.to_le_bytes()); // total entries
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_start.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // comment len
        out
    }

    /// ZIP, целиком занимающий ровно `total` байт (двухпроходная сборка:
    /// overhead вычисляется с учётом длины имени, payload = total - overhead).
    fn stored_zip_total(name: &str, total: u64) -> Vec<u8> {
        let overhead = stored_zip_payload(name, 0).len() as u64;
        assert!(total > overhead, "лимит меньше накладных расходов архива");
        let zip = stored_zip_payload(name, (total - overhead) as usize);
        assert_eq!(zip.len() as u64, total);
        zip
    }

    #[test]
    fn content_length_precheck_exact_over_and_missing() {
        assert!(!content_length_exceeds(
            Some(MAX_DOWNLOAD_BYTES),
            MAX_DOWNLOAD_BYTES
        ));
        assert!(content_length_exceeds(
            Some(MAX_DOWNLOAD_BYTES + 1),
            MAX_DOWNLOAD_BYTES
        ));
        assert!(!content_length_exceeds(None, MAX_DOWNLOAD_BYTES));
    }

    #[test]
    fn received_counter_exact_and_overflow() {
        assert_eq!(
            next_received(MAX_DOWNLOAD_BYTES, MAX_DOWNLOAD_BYTES),
            Some(MAX_DOWNLOAD_BYTES * 2)
        );
        assert_eq!(next_received(u64::MAX, 1), None);
        assert!(!exceeds_download_limit(
            MAX_DOWNLOAD_BYTES,
            MAX_DOWNLOAD_BYTES
        ));
        assert!(exceeds_download_limit(
            MAX_DOWNLOAD_BYTES + 1,
            MAX_DOWNLOAD_BYTES
        ));
    }

    #[tokio::test]
    async fn cleanup_part_removes_partial_file() {
        let part = tmp_part("cleanup");
        std::fs::write(&part, b"partial").unwrap();
        assert!(part.exists());
        cleanup_part(&part).await;
        assert!(!part.exists());
    }

    #[tokio::test]
    async fn declared_content_length_over_limit_rejected_before_download() {
        let url = serve_once(vec![b'x'; 16], Some(4096));
        let part = tmp_part("declared");
        let cancel = Arc::new(AtomicBool::new(false));
        let table: DownloadTable = Arc::new(AsyncMutex::new(HashMap::new()));
        let result = stream_to_part(
            &build_client(),
            None,
            &table,
            "k",
            "directurl",
            &url,
            &part,
            &cancel,
            1024,
        )
        .await;
        assert!(
            result.is_err(),
            "объявленный Content-Length выше лимита должен отклоняться до скачивания"
        );
        assert!(
            !part.exists(),
            "`.part` не должен создаваться при pre-check"
        );
    }

    #[tokio::test]
    async fn no_content_length_stream_aborts_at_limit_and_cleans_part() {
        let url = serve_once(vec![b'x'; 1025], None);
        let part = tmp_part("nocl");
        let cancel = Arc::new(AtomicBool::new(false));
        let table: DownloadTable = Arc::new(AsyncMutex::new(HashMap::new()));
        let result = stream_to_part(
            &build_client(),
            None,
            &table,
            "k",
            "directurl",
            &url,
            &part,
            &cancel,
            1024,
        )
        .await;
        let err =
            result.expect_err("без Content-Length лимит обязан сработать по фактическим байтам");
        assert!(err.to_string().contains("лимит") || err.to_string().contains("limit"));
        assert!(
            !part.exists(),
            "`.part` обязан удаляться при превышении лимита"
        );
    }

    #[tokio::test]
    async fn exact_limit_valid_zip_downloads_ok() {
        let max = 1024u64;
        let zip = stored_zip_total("m.zip", max);
        assert_eq!(zip.len() as u64, max);
        let url = serve_once(zip, Some(max));
        let part = tmp_part("exact");
        let cancel = Arc::new(AtomicBool::new(false));
        let table: DownloadTable = Arc::new(AsyncMutex::new(HashMap::new()));
        let (hex, bytes) = stream_to_part(
            &build_client(),
            None,
            &table,
            "k",
            "directurl",
            &url,
            &part,
            &cancel,
            max,
        )
        .await
        .expect("архив ровно на лимите должен качаться");
        assert_eq!(bytes, max);
        assert_eq!(hex.len(), 64);
        assert!(part.exists(), "успешная загрузка оставляет `.part`");
        std::fs::remove_file(&part).ok();
    }

    #[tokio::test]
    async fn oversized_local_import_aborts_and_cleans_destination() {
        let src = tmp_part("cmp-src");
        let dst = tmp_part("cmp-dst");
        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
        std::fs::write(&src, vec![b'x'; 2048]).unwrap();
        let err = copy_limited(&src, &dst, 1024)
            .await
            .expect_err("сверхлимитный импорт обязан отклоняться");
        assert!(err.contains("лимит") || err.contains("limit"));
        assert!(
            !dst.exists(),
            "целевой файл обязан удаляться при превышении"
        );
        std::fs::remove_file(&src).ok();
    }

    /// Ошибка чтения/открытия src (каталог): единственная точка cleanup
    /// `copy_limited` обязана снять dst. На Windows сбой происходит при open —
    /// поведенческий инвариант тот же: Err + диска без лишнего файла.
    #[tokio::test]
    async fn copy_limited_read_error_closes_and_removes_destination() {
        let src = tmp_part("rerr-src");
        let dst = tmp_part("rerr-dst");
        let _ = std::fs::remove_file(&dst);
        std::fs::create_dir_all(&src).unwrap();
        let err = copy_limited(&src, &dst, 1024)
            .await
            .expect_err("чтение каталога обязано завершиться ошибкой");
        assert!(!err.is_empty());
        assert!(!dst.exists(), "dst обязан удаляться при ошибке чтения");
        std::fs::remove_dir_all(&src).ok();
    }

    /// Create-ошибка (dst — существующий каталог): remove-file не трогает
    /// произвольные пути, которых мы не создавали.
    #[tokio::test]
    async fn copy_limited_create_error_leaves_uncreated_path_untouched() {
        let src = tmp_part("cerr-src");
        let dst = tmp_part("cerr-dst");
        let _ = std::fs::remove_file(&src);
        std::fs::write(&src, b"data").unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        let err = copy_limited(&src, &dst, 1024)
            .await
            .expect_err("создание файла по пути каталога обязано падать");
        assert!(!err.is_empty());
        assert!(
            dst.is_dir(),
            "существующий каталог не удаляется (удаляем только созданный `.part`)"
        );
        std::fs::remove_file(&src).ok();
        std::fs::remove_dir_all(&dst).ok();
    }

    #[test]
    fn failed_result_never_produces_ledger_entry() {
        let entry = ledger_entry_on_success(
            &Err(anyhow::anyhow!("limit exceeded")),
            "x.zip",
            "directurl".to_string(),
            "u".to_string(),
            "x".to_string(),
            None,
        );
        assert!(entry.is_none(), "ошибка загрузки не даёт записи в ledger");
    }

    #[test]
    fn failed_download_does_not_modify_ledger_file() {
        let tmp = std::env::temp_dir().join(format!("bmd-sec001-ledger-{}", std::process::id()));
        let ledger_dir = tmp.join("beamng-mod-downloader");
        std::fs::create_dir_all(&ledger_dir).unwrap();
        let ledger_file = ledger_dir.join("installed-ledger.json");
        let orig = r#"[{"filename":"old.zip","source":"beamngweb","key":"k","name":"n","installedAt":1,"published":null,"sha256":null}]"#;
        std::fs::write(&ledger_file, orig).unwrap();

        let old = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);
        let decision = ledger_entry_on_success(
            &Err(anyhow::anyhow!("limit exceeded")),
            "new.zip",
            "directurl".to_string(),
            "u".to_string(),
            "x".to_string(),
            None,
        );
        assert!(decision.is_none());
        let after = std::fs::read_to_string(&ledger_file).unwrap();
        assert_eq!(after, orig, "ledger обязан остаться неизменным при ошибке");
        std::env::remove_var("XDG_CONFIG_HOME");
        if let Some(o) = old {
            std::env::set_var("XDG_CONFIG_HOME", o);
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

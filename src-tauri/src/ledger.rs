use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Откуда и какой мод установлен: позволяет при list_installed понять,
/// какие архивы поставил лаунчер, и сравнивать версии при проверке обновлений.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub filename: String,
    pub source: String,
    pub key: String,
    pub name: String,
    pub installed_at: u64,
    pub published: Option<String>,
    /// SHA-256 архива, посчитанный при установке (для verify_installed).
    pub sha256: Option<String>,
}

pub fn ledger_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("не удалось определить каталог конфигурации пользователя")?
        .join("beamng-mod-downloader");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("не удалось создать {}", dir.display()))?;
    Ok(dir.join("installed-ledger.json"))
}

pub fn load() -> HashMap<String, LedgerEntry> {
    match ledger_path() {
        Ok(path) => match std::fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str::<Vec<LedgerEntry>>(&raw)
                .unwrap_or_default()
                .into_iter()
                .map(|e| (e.filename.clone(), e))
                .collect(),
            Err(_) => HashMap::new(),
        },
        Err(_) => HashMap::new(),
    }
}

pub fn save(entries: &HashMap<String, LedgerEntry>) -> Result<()> {
    let path = ledger_path()?;
    let mut list: Vec<&LedgerEntry> = entries.values().collect();
    list.sort_by(|a, b| a.filename.cmp(&b.filename));
    let raw = serde_json::to_string_pretty(&list)?;
    std::fs::write(&path, raw).with_context(|| format!("не удалось записать {}", path.display()))
}

pub fn upsert(entry: LedgerEntry) {
    let mut entries = load();
    entries.insert(entry.filename.clone(), entry);
    if let Err(e) = save(&entries) {
        log::warn!("не удалось сохранить ledger: {e}");
    }
}

pub fn remove(filename: &str) {
    let mut entries = load();
    if entries.remove(filename).is_some() {
        if let Err(e) = save(&entries) {
            log::warn!("не удалось сохранить ledger: {e}");
        }
    }
}

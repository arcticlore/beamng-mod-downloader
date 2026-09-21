pub mod beamngweb;
pub mod github;
pub mod registry;
pub mod worldofmods;

use crate::models::{ModDetail, ModSearchResult, SourceCategory};
use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SourceError {
    #[error("Сетевая ошибка: {0}")]
    Network(String),
    #[error("Ошибка разбора ответа: {0}")]
    Parse(String),
    #[error("Источник недоступен: {0}")]
    Unavailable(String),
}

impl From<anyhow::Error> for SourceError {
    fn from(e: anyhow::Error) -> Self {
        SourceError::Network(e.to_string())
    }
}

pub fn validate_source(source: &str) -> bool {
    registry::is_known(source)
}

/// Допущен ли источник к сетевым запросам (enabled в config).
/// Диспетчеры дергают его как защиту от неожиданных id и как единый гейт.
pub fn can_query(enabled: &[String], source: &str) -> bool {
    registry::can_query(enabled, source)
}

/// Если источник не в registry — возвращаем честную ошибку, а не match-провал.
fn ensure_known(source: &str) -> Result<(), SourceError> {
    if validate_source(source) {
        Ok(())
    } else {
        Err(SourceError::Unavailable(format!(
            "неизвестный источник `{source}`"
        )))
    }
}

pub async fn search(
    client: &reqwest::Client,
    source: &str,
    query: Option<&str>,
    category: Option<&str>,
    page: u32,
    order: Option<&str>,
) -> Result<ModSearchResult, SourceError> {
    ensure_known(source)?;
    match source {
        "worldofmods" => worldofmods::search(client, query, category, page, order).await,
        "beamngweb" => beamngweb::search(client, query, category, page, order).await,
        "github" => github::search(client, query, category, page, order).await,
        other => Err(SourceError::Unavailable(format!(
            "источник `{other}` пока не реализован"
        ))),
    }
}

pub async fn detail(
    client: &reqwest::Client,
    source: &str,
    mod_id: &str,
    key: &str,
) -> Result<ModDetail, SourceError> {
    ensure_known(source)?;
    match source {
        "worldofmods" => worldofmods::detail(client, mod_id, key).await,
        "beamngweb" => beamngweb::detail(client, mod_id, key).await,
        "github" => github::detail(client, mod_id, key).await,
        other => Err(SourceError::Unavailable(format!(
            "источник `{other}` пока не реализован"
        ))),
    }
}

/// Возвращает финальный URL для скачивания архива мода, безопасное имя файла
/// и дату (версию) на источнике на момент установки — для записи в ledger.
pub async fn resolve_download(
    client: &reqwest::Client,
    source: &str,
    key: &str,
) -> Result<(String, String, Option<String>), SourceError> {
    ensure_known(source)?;
    match source {
        "worldofmods" => worldofmods::resolve_download(client, key).await,
        "beamngweb" => beamngweb::resolve_download(client, key).await,
        "github" => github::resolve_download(client, key).await,
        other => Err(SourceError::Unavailable(format!(
            "источник `{other}` пока не реализован"
        ))),
    }
}

pub fn categories(source: &str) -> Vec<SourceCategory> {
    match registry::by_id(source) {
        Some(d) => d.categories,
        None => Vec::new(),
    }
}

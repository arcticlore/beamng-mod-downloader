pub mod beamngweb;
pub mod github;
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

pub async fn search(
    client: &reqwest::Client,
    source: &str,
    query: Option<&str>,
    category: Option<&str>,
    page: u32,
    order: Option<&str>,
) -> Result<ModSearchResult, SourceError> {
    match source {
        "worldofmods" => worldofmods::search(client, query, category, page, order).await,
        "beamngweb" => beamngweb::search(client, query, category, page, order).await,
        "github" => github::search(client, query, category, page, order).await,
        other => Err(SourceError::Unavailable(format!(
            "неизвестный источник `{other}`"
        ))),
    }
}

pub async fn detail(
    client: &reqwest::Client,
    source: &str,
    mod_id: &str,
    key: &str,
) -> Result<ModDetail, SourceError> {
    match source {
        "worldofmods" => worldofmods::detail(client, mod_id, key).await,
        "beamngweb" => beamngweb::detail(client, mod_id, key).await,
        "github" => github::detail(client, mod_id, key).await,
        other => Err(SourceError::Unavailable(format!(
            "неизвестный источник `{other}`"
        ))),
    }
}

/// Возвращает финальный URL для скачивания архива мода и безопасное имя файла.
pub async fn resolve_download(
    client: &reqwest::Client,
    source: &str,
    key: &str,
) -> Result<(String, String), SourceError> {
    match source {
        "worldofmods" => worldofmods::resolve_download(client, key).await,
        "beamngweb" => beamngweb::resolve_download(client, key).await,
        "github" => github::resolve_download(client, key).await,
        other => Err(SourceError::Unavailable(format!(
            "неизвестный источник `{other}`"
        ))),
    }
}

pub fn categories(source: &str) -> Vec<SourceCategory> {
    match source {
        "worldofmods" => worldofmods::categories(),
        "beamngweb" => beamngweb::categories(),
        "github" => github::categories(),
        _ => Vec::new(),
    }
}

pub fn validate_source(source: &str) -> bool {
    matches!(source, "worldofmods" | "beamngweb" | "github")
}

pub mod beamngforum;
pub mod beamngweb;
pub mod codeberg;
pub mod github;
pub mod gitlab;
pub mod registry;
pub mod worldofmods;

use crate::i18n;
use crate::models::{ModDetail, ModSearchResult, SourceCategory};
use anyhow::Result;

#[derive(Debug)]
pub enum SourceError {
    Network(String),
    Parse(String),
    Unavailable(String),
}

impl SourceError {
    fn kind(self: &SourceError) -> (&'static str, &'static str) {
        match self {
            SourceError::Network(_) => ("Сетевая ошибка: ", "Network error: "),
            SourceError::Parse(_) => ("Ошибка разбора ответа: ", "Failed to parse response: "),
            SourceError::Unavailable(_) => ("Источник недоступен: ", "Source unavailable: "),
        }
    }
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (ru, en) = self.kind();
        let (_, msg) = match self {
            SourceError::Network(m) | SourceError::Parse(m) | SourceError::Unavailable(m) => {
                ((), m.as_str())
            }
        };
        write!(f, "{}{}", i18n::t(ru, en), msg)
    }
}

impl std::error::Error for SourceError {}

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
        Err(SourceError::Unavailable(i18n::tf(
            "неизвестный источник `{0}`",
            "unknown source `{0}`",
            &[source],
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
        "gitlab" => gitlab::search(client, query, category, page, order).await,
        "codeberg" => codeberg::search(client, query, category, page, order).await,
        "beamngforum" => beamngforum::search(client, query, category, page, order).await,
        other => Err(SourceError::Unavailable(i18n::tf(
            "источник `{0}` пока не реализован",
            "source `{0}` is not implemented yet",
            &[other],
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
        "gitlab" => gitlab::detail(client, mod_id, key).await,
        "codeberg" => codeberg::detail(client, mod_id, key).await,
        "beamngforum" => beamngforum::detail(client, mod_id, key).await,
        other => Err(SourceError::Unavailable(i18n::tf(
            "источник `{0}` пока не реализован",
            "source `{0}` is not implemented yet",
            &[other],
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
        "gitlab" => gitlab::resolve_download(client, key).await,
        "codeberg" => codeberg::resolve_download(client, key).await,
        "beamngforum" => beamngforum::resolve_download(client, key).await,
        other => Err(SourceError::Unavailable(i18n::tf(
            "источник `{0}` пока не реализован",
            "source `{0}` is not implemented yet",
            &[other],
        ))),
    }
}

pub fn categories(source: &str) -> Vec<SourceCategory> {
    match registry::by_id(source) {
        Some(d) => d.categories,
        None => Vec::new(),
    }
}

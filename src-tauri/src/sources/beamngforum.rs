//! Официальный BeamNG Forum — честный link-only режим.
//!
//! Blocker (зафиксирован на 2026-09): XenForo search на beamng.com/community
//! требует логин/CSRF-токен (без авторизации стабильного публичного
//! search/listing нет), а URL attachments не имеют устойчивой схемы для
//! безопасного автоматического скачивания. Мы не обходим login/anti-bot и
//! не храним учётные данные. Источник представлен как **manual/link-only**:
//! пользователь открывает форум в браузере, авторизуется там сам.
//!
//! Это соответствует политике: не подделываем полноценную интеграцию brittle
//! scraping'ом, честно описываем ограниченный режим в UI и registry.
//! Парсер быстрых/attachments появляется вместе со стабильным публичным
//! API search (если/когда он появится) — отдельным PR.

use crate::models::{ModDetail, ModSearchResult, SourceCategory};
use crate::sources::SourceError;

pub fn categories() -> Vec<SourceCategory> {
    vec![SourceCategory {
        id: "all".to_string(),
        label: "Общий форум".to_string(),
    }]
}

pub async fn search(
    _client: &reqwest::Client,
    _query: Option<&str>,
    _category: Option<&str>,
    _page: u32,
    _order: Option<&str>,
) -> Result<ModSearchResult, SourceError> {
    Err(SourceError::Unavailable(
        "поиск на форуме BeamNG требует авторизации — откройте форум в браузере и воспользуйтесь встроенным поиском".to_string(),
    ))
}

pub async fn detail(
    _client: &reqwest::Client,
    _mod_id: &str,
    _key: &str,
) -> Result<ModDetail, SourceError> {
    Err(SourceError::Unavailable(
        "обзор модов на форуме BeamNG доступен только в браузере после входа".to_string(),
    ))
}

pub async fn resolve_download(
    _client: &reqwest::Client,
    _key: &str,
) -> Result<(String, String, Option<String>), SourceError> {
    Err(SourceError::Unavailable(
        "автоматическая установка с форума недоступна: attachments требуют логин".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(e: SourceError) -> String {
        e.to_string()
    }

    #[tokio::test]
    async fn search_returns_understandable_blocker() {
        let client = crate::http::build_client().expect("http client");
        let e = search(&client, None, None, 1, None).await.unwrap_err();
        assert!(msg(e).contains("авторизации"));
    }

    #[tokio::test]
    async fn resolve_download_blocks_auto_install() {
        let client = crate::http::build_client().expect("http client");
        let e = resolve_download(&client, "https://www.beamng.com/community/threads/x/")
            .await
            .unwrap_err();
        assert!(msg(e).contains("логин"));
    }
}
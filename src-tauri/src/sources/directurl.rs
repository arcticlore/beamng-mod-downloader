//! Источник «Прямая ссылка»: установка архива мода по прямому URL .zip.
//!
//! Поиска нет — пользователь вставляет ссылку на архив. Ссылка проходит тот
//! же SSRF-гейт `urlguard::validate_url`, что и остальные исходящие запросы
//! (только http/https, домены из allowlist'а, без IP-литералов и userinfo).
//! Требуем, чтобы URL заканчивался на `.zip`: это явный прямой контракт
//! «вставки файла», а не web-страницы. Установка идёт через общий конвейер
//! `download::start`: staging `.part`, структурная проверка zip с лимитами,
//! no-clobber, запись в ledger.

use crate::models::{ModDetail, ModItem, ModSearchResult, SourceCategory};
use crate::sources::SourceError;

pub fn categories() -> Vec<SourceCategory> {
    vec![SourceCategory {
        id: "all".to_string(),
        label: crate::i18n::t("Прямая ссылка", "Direct link"),
    }]
}

pub async fn search(
    _client: &reqwest::Client,
    _query: Option<&str>,
    _category: Option<&str>,
    _page: u32,
    _order: Option<&str>,
) -> Result<ModSearchResult, SourceError> {
    Err(SourceError::Unavailable(crate::i18n::t(
        "у «Прямой ссылки» нет поиска — вставьте ссылку на .zip-архив в панели «Добавить мод»",
        "the Direct link source has no search — paste a link to a .zip archive in the \"Add a mod\" panel",
    )))
}

/// Валидирует ключ-URL прямого источника и возвращает безопасное имя файла
/// (последний сегмент пути). Чистая функция для unit-тестов; сетевая часть
/// — `resolve_download`.
pub fn key_to_filename(key: &str) -> Result<(String, String), SourceError> {
    crate::urlguard::validate_url(key).map_err(SourceError::Unavailable)?;
    let url = reqwest::Url::parse(key).map_err(|e| SourceError::Parse(e.to_string()))?;
    let segment = url.path().rsplit('/').next().unwrap_or("").to_string();
    if !segment.to_ascii_lowercase().ends_with(".zip") {
        return Err(SourceError::Unavailable(crate::i18n::t(
            "прямая ссылка должна вести на архив .zip",
            "a direct link must point to a .zip archive",
        )));
    }
    let decoded = crate::sources::probe::percent_decode(&segment);
    let filename = crate::http::sanitize_filename(&decoded);
    Ok((key.to_string(), filename))
}

/// Заглушка-описание: прямой источник не даёт карточки мода до установки.
pub async fn detail(
    client: &reqwest::Client,
    mod_id: &str,
    key: &str,
) -> Result<ModDetail, SourceError> {
    let _ = key_to_filename(key)?;
    let probed = crate::sources::probe::probe(client, key)
        .await
        .map_err(|e| {
            SourceError::Unavailable(crate::i18n::tf(
                "ссылка не отвечает: {0}",
                "the link does not respond: {0}",
                &[&e.to_string()],
            ))
        })?;
    let name = probed
        .filename
        .clone()
        .unwrap_or_else(|| mod_id.to_string());
    Ok(ModDetail {
        item: ModItem {
            id: mod_id.to_string(),
            source: "directurl".to_string(),
            name,
            thumbnail: None,
            description: None,
            key: key.to_string(),
            category: None,
            author: None,
            published: probed.published,
            downloads: None,
            size_bytes: probed.size,
        },
        full_description: None,
        screenshots: Vec::new(),
    })
}

/// Возвращает (url, безопасное имя файла, дату на источнике). Дата — опц.
/// результат probe: если сервер не отдал заголовки, установка всё равно
/// разрешена (сам файл проверяется в `stream_to_part`).
pub async fn resolve_download(
    client: &reqwest::Client,
    key: &str,
) -> Result<(String, String, Option<String>), SourceError> {
    let (url, filename) = key_to_filename(key)?;
    let published = crate::sources::probe::probe(client, &url)
        .await
        .ok()
        .and_then(|p| p.published);
    Ok((url, filename, published))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err_msg(e: SourceError) -> String {
        e.to_string()
    }

    #[test]
    fn key_to_filename_accepts_allowlisted_zip_urls() {
        let (url, name) =
            key_to_filename("https://github.com/o/r/releases/download/v1/car%20pack.zip")
                .expect("валидный github-zip");
        assert_eq!(
            url,
            "https://github.com/o/r/releases/download/v1/car%20pack.zip"
        );
        assert_eq!(name, "car_pack.zip");
        let (_, name) = key_to_filename("https://www.beamng.com/download/123/abc.zip").unwrap();
        assert_eq!(name, "abc.zip");
    }

    #[test]
    fn key_to_filename_rejects_non_zip_and_bad_transport() {
        // Не .zip — даже на разрешённом хосте.
        assert!(err_msg(
            key_to_filename("https://github.com/o/r/releases/download/v1/readme.txt").unwrap_err()
        )
        .contains("zip"));
        assert!(err_msg(
            key_to_filename("https://www.worldofmods.com/beamng/mods/1.html").unwrap_err()
        )
        .contains("zip"));
        // Host вне allowlist.
        assert!(
            err_msg(key_to_filename("https://evil.example/mod.zip").unwrap_err()).contains("домен")
        );
        // IP-литерал и userinfo.
        assert!(
            err_msg(key_to_filename("http://127.0.0.1/mod.zip").unwrap_err()).contains("запрещ")
        );
        assert!(
            err_msg(key_to_filename("https://user:pass@github.com/o/r.zip").unwrap_err())
                .contains("запрещ")
        );
        // Не URL.
        assert!(err_msg(key_to_filename("not a url").unwrap_err()).contains("невалидный URL"));
    }
}

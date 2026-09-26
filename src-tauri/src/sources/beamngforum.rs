//! Официальный BeamNG Forum — вложения по canonical-ссылке.
//!
//! Режимы:
//! - **search заблокирован** (XenForo search на beamng.com/community требует
//!   логин/CSRF; мы не обходим login/anti-bot и не храним учётные данные):
//!   пользователь ищет и копирует ссылку в браузере сам.
//! - **detail/resolve_download работают по стабильным canonical attachment-URL
//!   `https://www.beamng.com/attachments/<id>/`** (301 → полный slug-URL).
//!   Канонизатор принимает: `attachment:<id>`, голые цифры, `/attachments/<id>/`,
//!   `/attachments/<slug>-<id>/` и `/attachments/<slug>.<id>/` (итоговый id —
//!   хвостовой цифровой кусок последнего сегмента пути, как у XenForo).
//!
//! Скачивание идёт через общий конвейер `download::start` (staging, zip-лимиты,
//! no-clobber). SSRF-политика `urlguard` разрешает beamng.com и редирект-хопы
//! через Cloudflare R2; каждый хоп перепроверяется.

use crate::models::{ModDetail, ModItem, ModSearchResult, SourceCategory};
use crate::sources::SourceError;

pub fn categories() -> Vec<SourceCategory> {
    vec![SourceCategory {
        id: "all".to_string(),
        label: crate::i18n::t("Общий форум", "General forum"),
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
        "поиск на форуме BeamNG требует авторизации — откройте форум в браузере и воспользуйтесь встроенным поиском",
        "searching the BeamNG forum requires sign-in — open the forum in your browser and use its built-in search",
    )))
}

/// Канонический URL вложения форума `https://www.beamng.com/attachments/<id>/`.
/// Чистая функция (без сети) для unit-тестов и маршрутизации в `install_from_url`.
/// Принимает:
/// - `attachment:<id>` (ключ, которым уже писались моды в ledger);
/// - голые цифры `<id>`;
/// - `https://…/attachments/<id>/` и `https://…/attachments/<slug>-<id>/`
///   или `<slug>.<id>/` (host — любой поддомен beamng.com).
pub fn canonical_key(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let body = trimmed.strip_prefix("attachment:").unwrap_or(trimmed);

    // Голый id (тоже `attachment:<id>`-пайлоад после префикса).
    if !body.is_empty() && body.chars().all(|c| c.is_ascii_digit()) {
        return Some(format!("https://www.beamng.com/attachments/{body}/"));
    }

    let parsed = reqwest::Url::parse(trimmed).ok()?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return None;
    }
    let host = parsed.host_str()?;
    let host_ok = host.eq_ignore_ascii_case("beamng.com")
        || host.to_ascii_lowercase().ends_with(".beamng.com");
    if !host_ok {
        return None;
    }

    let segments: Vec<&str> = parsed.path().split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() != 2 || segments[0] != "attachments" {
        return None;
    }

    let seg = segments[1];
    let digit_len = seg.chars().rev().take_while(|c| c.is_ascii_digit()).count();
    if digit_len == 0 {
        return None;
    }
    let id: String = seg
        .chars()
        .rev()
        .take(digit_len)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if id.is_empty() {
        return None;
    }
    Some(format!("https://www.beamng.com/attachments/{id}/"))
}

/// Числовой id вложения из canonical-URL (`…/attachments/<id>/`).
pub fn attachment_id(canonical: &str) -> Option<u32> {
    let segments: Vec<&str> = canonical.split('/').filter(|s| !s.is_empty()).collect();
    segments.last().and_then(|s| s.parse::<u32>().ok())
}

/// Является ли ссылка похожей на вложение форума (роутинг в Ubuntu-команде).
pub fn looks_like_attachment(url: &str) -> bool {
    canonical_key(url).is_some()
}

/// Описание по canonical-ссылке: считает имя файла и дату из заголовков
/// (HEAD/Range probe), не скачивая тело.
pub async fn detail(
    client: &reqwest::Client,
    mod_id: &str,
    key: &str,
) -> Result<ModDetail, SourceError> {
    let canonical = non_canonical(key)?;
    crate::urlguard::validate_url(&canonical).map_err(SourceError::Unavailable)?;
    let id = attachment_id(&canonical)
        .map(|n| n.to_string())
        .unwrap_or_else(|| mod_id.to_string());
    let probed = crate::sources::probe::probe(client, &canonical)
        .await
        .map_err(|e| {
            SourceError::Unavailable(crate::i18n::tf(
                "вложение не отвечает: {0}",
                "the attachment does not respond: {0}",
                &[&e.to_string()],
            ))
        })?;
    let name = probed
        .filename
        .clone()
        .unwrap_or_else(|| format!("attachment:{id}"));
    Ok(ModDetail {
        item: ModItem {
            id,
            source: "beamngforum".to_string(),
            name,
            thumbnail: None,
            description: None,
            key: canonical,
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

/// Возвращает (canonical URL, безопасное имя файла, дату). Дата — опциональный
/// результат probe: если сервер не отдал заголовки, установка всё равно
/// разрешена (сам zip проверяется в `stream_to_part`).
pub async fn resolve_download(
    client: &reqwest::Client,
    key: &str,
) -> Result<(String, String, Option<String>), SourceError> {
    let canonical = non_canonical(key)?;
    crate::urlguard::validate_url(&canonical).map_err(SourceError::Unavailable)?;
    let id = attachment_id(&canonical)
        .map(|n| n.to_string())
        .unwrap_or_else(|| seg_of(&canonical));
    let probed = crate::sources::probe::probe(client, &canonical).await.ok();
    let filename = probed
        .as_ref()
        .and_then(|p| p.filename.clone())
        .unwrap_or_else(|| format!("attachment-{id}.zip"));
    Ok((canonical, filename, probed.and_then(|p| p.published)))
}

fn non_canonical(key: &str) -> Result<String, SourceError> {
    canonical_key(key).ok_or_else(|| {
        SourceError::Unavailable(crate::i18n::t(
            "это не похоже на ссылку вложения BeamNG Forum — вставьте ссылку вида https://www.beamng.com/attachments/<id>/",
            "this does not look like a BeamNG Forum attachment link — paste a link like https://www.beamng.com/attachments/<id>/",
        ))
    })
}

fn seg_of(canonical: &str) -> String {
    canonical
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_all_attachment_forms() {
        let c = "https://www.beamng.com/attachments/433264/";
        assert_eq!(canonical_key("attachment:433264"), Some(c.to_string()));
        assert_eq!(canonical_key("433264"), Some(c.to_string()));
        assert_eq!(canonical_key(" 433264 "), Some(c.to_string()));
        assert_eq!(
            canonical_key("https://www.beamng.com/attachments/433264/"),
            Some(c.to_string())
        );
        assert_eq!(
            canonical_key("https://www.beamng.com/attachments/433264"),
            Some(c.to_string())
        );
        assert_eq!(
            canonical_key("http://beamng.com/attachments/433264/"),
            Some(c.to_string())
        );
        // XenForo slug-id (реальный пример со смоуком: `gavril_truck_co_model_d-zip.433264`),
        // где id — хвостовые цифры последнего сегмента.
        assert_eq!(
            canonical_key("https://www.beamng.com/attachments/gavril_truck_co_model_d-zip.433264/"),
            Some(c.to_string())
        );
        assert_eq!(
            canonical_key("https://www.beamng.com/attachments/mod2.433264/"),
            Some(c.to_string())
        );
        assert_eq!(
            canonical_key("https://sub.beamng.com/attachments/7-123/"),
            Some("https://www.beamng.com/attachments/123/".to_string())
        );
    }

    #[test]
    fn rejects_non_attachment_inputs() {
        assert_eq!(canonical_key(""), None);
        assert_eq!(canonical_key("attachment:"), None);
        assert_eq!(canonical_key("12a"), None);
        assert_eq!(canonical_key("https://evil.example/attachments/5/"), None);
        assert_eq!(canonical_key("https://beamng.com/threads/o-t.123/"), None);
        assert_eq!(
            canonical_key("https://www.beamng.com/attachments/abc/"),
            None
        );
        assert_eq!(canonical_key("ftp://www.beamng.com/attachments/1/"), None);
        assert_eq!(canonical_key("www.beamng.com/attachments/5/"), None);
        assert_eq!(
            canonical_key("https://www.beamng.com/attachments/1/2"),
            None
        );
        // Percent-кодированные сегменты пути не декодируются при разборе:
        // у `1%32` хвостовые цифры — `32`, что и становится id.
        assert_eq!(
            canonical_key("https://www.beamng.com/attachments/1%32/"),
            Some("https://www.beamng.com/attachments/32/".to_string())
        );
    }

    #[test]
    fn attachment_id_extracts_digits() {
        assert_eq!(
            attachment_id("https://www.beamng.com/attachments/433264/"),
            Some(433264)
        );
        assert_eq!(
            attachment_id("https://www.beamng.com/attachments/1/"),
            Some(1)
        );
        // attachment_id работает только с canonical-URL (выдаёт attachment_id)…
        assert_eq!(attachment_id("https://www.beamng.com/threads/5"), Some(5));
        assert_eq!(attachment_id("x"), None);
    }

    #[test]
    fn looks_like_routes_only_forum_links() {
        assert!(looks_like_attachment(
            "https://www.beamng.com/attachments/433264/"
        ));
        assert!(looks_like_attachment("attachment:433264"));
        assert!(!looks_like_attachment(
            "https://github.com/o/r/releases/download/v1/mod.zip"
        ));
        assert!(!looks_like_attachment(
            "https://www.beamng.com/community/threads/x.123/"
        ));
    }

    #[tokio::test]
    async fn search_returns_understandable_blocker() {
        let e = search(
            &crate::http::build_client().expect("http client"),
            None,
            None,
            1,
            None,
        )
        .await
        .unwrap_err();
        assert!(e.to_string().contains("авторизации"));
    }

    #[tokio::test]
    async fn resolve_rejects_non_canonical_without_network() {
        let client = crate::http::build_client().expect("http client");
        let e = resolve_download(
            &client,
            "https://github.com/o/r/releases/download/v1/mod.zip",
        )
        .await
        .unwrap_err();
        assert!(e.to_string().contains("не похоже"));
    }

    /// Реальный bounded smoke на вложении 433264 (10 МБ): у канонического URL
    /// probe/detail отдают реальные имя/размер/дату, resolve возвращает тот же
    /// URL, скачанные байты совпадают по SHA-256 с тем, что отдаёт сервер,
    /// и проходят структурную проверку zip (лимиты записей/объёма).
    /// Запуск: `cargo test -- --ignored network_smoke_433264_real_attachment`.
    #[tokio::test]
    #[ignore]
    async fn network_smoke_433264_real_attachment() {
        let client = crate::http::build_client().expect("http client");
        let url = "https://www.beamng.com/attachments/433264/";

        let d = detail(&client, "433264", url).await.expect("forum detail");
        assert_eq!(d.item.name, "gavril_truck_co_model_d.zip");
        assert_eq!(d.item.size_bytes, Some(10_231_144));
        assert_eq!(d.item.source, "beamngforum");
        assert_eq!(d.item.key, url);
        assert!(d.item.published.is_some(), "expected a published date");

        let (final_url, filename, published) =
            resolve_download(&client, url).await.expect("resolve");
        assert_eq!(final_url, url);
        assert_eq!(filename, "gavril_truck_co_model_d.zip");
        assert!(published.is_some());

        let dir = std::env::temp_dir().join(format!("bmd-forum-smoke-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join(&filename);
        let resp = client.get(url).send().await.expect("download GET");
        assert!(resp.status().is_success(), "status = {}", resp.status());
        let bytes = resp.bytes().await.expect("download body");
        assert_eq!(bytes.len() as u64, 10_231_144, "content-length");
        std::fs::write(&dest, &bytes).expect("write tmp file");

        let hex = crate::archive::sha256_file(&dest).expect("sha256");
        assert_eq!(
            hex,
            "39d80d89aab0aa32dc6bf5a3c36d87f7a455a5083644a3fb43b90da5582cea7f"
        );
        let summary = crate::archive::validate_zip(&dest).expect("validate_zip ok");
        assert!(summary.entries > 0);
        assert!(summary.total_uncompressed > 0);

        std::fs::remove_dir_all(&dir).ok();
    }
}

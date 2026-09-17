use crate::http;
use crate::models::{ModDetail, ModItem, ModSearchResult, SourceCategory};
use crate::sources::SourceError;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

/// Поиск по GitHub: репозитории с topic:beamng и их release-архивы (.zip).
/// Авторизация не требуется (anonymous quotas: search 10/мин, core 60/час,
/// поэтому ответы releases кэшируются в памяти).
const API: &str = "https://api.github.com";
const PER_PAGE: u32 = 30;
/// Ссылки на релизы кэшируются не дольше этого времени.
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(30 * 60);

static RELEASE_CACHE: LazyLock<Mutex<HashMap<String, (String, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const CATEGORIES: &[(&str, &str)] = &[
    ("all", "topic:beamng"),
    ("maps", "topic:beamng+map"),
    ("vehicles", "topic:beamng+(car+OR+vehicle)"),
    ("other", "topic:beamng+-topic:beammp"),
];

pub fn categories() -> Vec<SourceCategory> {
    CATEGORIES
        .iter()
        .map(|(id, _)| SourceCategory {
            id: id.to_string(),
            label: match *id {
                "all" => "Все".to_string(),
                "maps" => "Карты".to_string(),
                "vehicles" => "Авто".to_string(),
                "other" => "Другое".to_string(),
                other => other.to_string(),
            },
        })
        .collect()
}

fn category_query(category: &str) -> &str {
    CATEGORIES
        .iter()
        .find(|(id, _)| *id == category)
        .map(|(_, q)| *q)
        .unwrap_or(CATEGORIES[0].1)
}

/// Оставляет только безопасные слова для поиска GitHub (не токены `:` / операторы).
fn sanitize_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|w| w.trim_start_matches('-'))
        .filter(|w| w.chars().any(|c| c.is_ascii_alphanumeric()))
        .map(|w| {
            w.chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("+")
}

fn search_query(category: Option<&str>, query: Option<&str>) -> String {
    let mut q = category_query(category.unwrap_or("all")).to_string();
    if let Some(qq) = query {
        let clean = sanitize_query(qq);
        if !clean.is_empty() {
            q.push('+');
            q.push_str(&clean);
        }
    }
    q
}

async fn gh_get(client: &reqwest::Client, url: &str) -> Result<String, SourceError> {
    let resp = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| SourceError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| map_gh_error(e, url))?;
    resp.text()
        .await
        .map_err(|e| SourceError::Network(e.to_string()))
}

fn map_gh_error(e: reqwest::Error, url: &str) -> SourceError {
    if let Some(status) = e.status() {
        if status == reqwest::StatusCode::FORBIDDEN {
            return SourceError::Network(format!(
                "GitHub ограничил запросы (rate limit). URL: {url}"
            ));
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return SourceError::Parse(format!("не найдено на GitHub: {url}"));
        }
    }
    SourceError::Network(e.to_string())
}

#[derive(Deserialize, Debug)]
struct SearchResponse {
    total_count: u64,
    items: Vec<SearchItem>,
}

#[derive(Deserialize, Debug)]
struct SearchItem {
    full_name: String,
    #[serde(default)]
    description: Option<String>,
    html_url: String,
    #[serde(default)]
    topics: Vec<String>,
    stargazers_count: u64,
    updated_at: String,
    owner: Owner,
}

#[derive(Deserialize, Debug)]
struct Owner {
    login: String,
    #[serde(default)]
    avatar_url: String,
}

fn item(item: &SearchItem) -> ModItem {
    let published = item.updated_at.split('T').next().map(ToOwned::to_owned);
    ModItem {
        id: format!("github:{}", item.full_name),
        source: "github".to_string(),
        name: item.full_name.clone(),
        thumbnail: Some(item.owner.avatar_url.clone()),
        description: item.description.clone(),
        key: item.html_url.clone(),
        category: item.topics.first().cloned(),
        author: Some(item.owner.login.clone()),
        published,
        downloads: Some(item.stargazers_count.to_string()),
        size_bytes: None,
    }
}

pub async fn search(
    client: &reqwest::Client,
    query: Option<&str>,
    category: Option<&str>,
    page: u32,
) -> Result<ModSearchResult, SourceError> {
    let q = search_query(category, query);
    let url = format!(
        "{API}/search/repositories?q={q}&sort=updated&order=desc&per_page={PER_PAGE}&page={}",
        page.max(1)
    );
    let body = gh_get(client, &url).await?;
    let resp: SearchResponse = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("GitHub ответил не JSON: {e}")))?;

    // GitHub отдаёт максимум 1000 результатов поиска
    let cap = resp.total_count.min(1000);
    let total_pages = ((cap as u32).div_ceil(PER_PAGE)).max(1);

    Ok(ModSearchResult {
        items: resp.items.iter().map(item).collect(),
        total_pages,
        current_page: page.max(1),
    })
}

/// Детали репозитория берём через тот же поиск (не тратим core-лимит).
pub async fn detail(
    client: &reqwest::Client,
    _mod_id: &str,
    key: &str,
) -> Result<ModDetail, SourceError> {
    let full = repo_from_key(key)?;
    let url = format!("{API}/search/repositories?q=repo:{full}&per_page=1");
    let body = gh_get(client, &url).await?;
    let resp: SearchResponse = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("GitHub ответил не JSON: {e}")))?;
    let it = resp
        .items
        .first()
        .ok_or_else(|| SourceError::Parse("репозиторий не найден".into()))?;
    let item = item(it);
    let full_description = item.description.clone();
    let screenshots: Vec<String> = Vec::new();
    Ok(ModDetail {
        item,
        full_description,
        screenshots,
    })
}

/// Извлечение `owner/repo` из URL вида `https://github.com/owner/repo`.
/// Дополнительные сегменты пути (tree/main и т.п.) игнорируются.
pub(crate) fn repo_from_key(key: &str) -> Result<String, SourceError> {
    let parts: Vec<&str> = key.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 4 || parts[1] != "github.com" {
        return Err(SourceError::Parse(format!(
            "не похоже на ссылку GitHub: {key}"
        )));
    }
    let owner = parts[2];
    let repo = parts[3];
    if owner.is_empty() || repo.is_empty() {
        return Err(SourceError::Parse(format!(
            "не похоже на ссылку GitHub: {key}"
        )));
    }
    Ok(format!("{owner}/{repo}"))
}

fn cached_release(owner_repo: &str) -> Option<String> {
    let now = Instant::now();
    let mut cache = RELEASE_CACHE.lock().ok()?;
    cache.retain(|_, (_, at)| now.duration_since(*at) < CACHE_TTL);
    cache.get(owner_repo).map(|(url, _)| url.clone())
}

fn cache_release(owner_repo: &str, url: &str) {
    if let Ok(mut cache) = RELEASE_CACHE.lock() {
        cache.insert(owner_repo.to_string(), (url.to_string(), Instant::now()));
    }
}

fn gh_abs(href: &str) -> String {
    if href.starts_with("//") {
        format!("https:{href}")
    } else if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else {
        format!("https://github.com{href}")
    }
}

/// Извлекает первый корректный тег релиза из страницы `releases/latest`
/// (на странице встречаются и служебные ссылки вида `releases/tag/*name`,
/// которые пропускаются).
fn extract_tag(html: &str) -> Option<String> {
    let marker = "/releases/tag/";
    let mut probe = html;
    loop {
        let start = probe.find(marker)? + marker.len();
        let rest = &probe[start..];
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '.' | '-' | '_'))
            .unwrap_or(rest.len());
        let tag = &rest[..end];
        if !tag.is_empty() {
            return Some(tag.to_string());
        }
        probe = rest;
    }
}

/// Выбирает архив для скачивания из HTML-страницы assets.
/// Предпочтение — `.zip`; fallback — первый попавшийся файл.
fn pick_asset(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a[href*=\"releases/download/\"]").ok()?;
    let mut fallback = None;
    for a in doc.select(&sel) {
        let href = a.value().attr("href").unwrap_or("");
        if href.is_empty() {
            continue;
        }
        let filename = href.rsplit('/').next().unwrap_or("");
        if filename.to_ascii_lowercase().ends_with(".zip") {
            return Some(gh_abs(href));
        }
        if fallback.is_none() {
            fallback = Some(gh_abs(href));
        }
    }
    fallback
}

/// Возвращает (url последнего релиза, имя локального файла).
/// Имя файла детерминировано (`owner-repo.zip`), чтобы UI мог сопоставлять
/// карточку с уже установленным модом без знания имени ассета.
pub async fn resolve_download(
    client: &reqwest::Client,
    key: &str,
) -> Result<(String, String), SourceError> {
    let full = repo_from_key(key)?;
    let zip_name = format!("{}.zip", full.replace('/', "-"));
    if let Some(url) = cached_release(&full) {
        return Ok((url, zip_name));
    }

    let base = format!("https://github.com/{full}");
    let latest_url = format!("{base}/releases/latest");
    let latest = http::fetch_string(client, &latest_url, None)
        .await
        .map_err(|e| SourceError::Unavailable(format!("нет релизов у {full}: {e}")))?;

    let mut url = pick_asset(&latest);
    if url.is_none() {
        if let Some(tag) = extract_tag(&latest) {
            let expanded_url = format!("{base}/releases/expanded_assets/{tag}");
            let expanded = http::fetch_string(client, &expanded_url, Some(&latest_url))
                .await
                .map_err(|e| SourceError::Network(e.to_string()))?;
            url = pick_asset(&expanded);
        }
    }

    let url = url.ok_or_else(|| {
        SourceError::Unavailable(format!(
            "в последнем релизе {full} нет файлов для скачивания"
        ))
    })?;

    cache_release(&full, &url);
    Ok((url, zip_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_JSON: &str = r#"{
      "total_count": 141,
      "items": [
        {"full_name":"BeamMP/BeamMP","description":"Bringing multiplayer to BeamNG.drive",
         "html_url":"https://github.com/BeamMP/BeamMP","topics":["beamng","mod"],
         "stargazers_count":513,"updated_at":"2026-09-14T15:19:15Z",
         "owner":{"login":"BeamMP","avatar_url":"https://a/b.png"}},
        {"full_name":"SteliosLL/BeamMP-Server-Management-Tool",
         "description":null,"html_url":"https://github.com/SteliosLL/BeamMP-Server-Management-Tool",
         "topics":[],"stargazers_count":3,"updated_at":"2025-01-01T00:00:00Z",
         "owner":{"login":"SteliosLL","avatar_url":""}}
      ]
    }"#;

    #[test]
    fn parses_search_response() {
        let resp: SearchResponse = serde_json::from_str(SEARCH_JSON).unwrap();
        assert_eq!(resp.total_count, 141);
        assert_eq!(resp.items.len(), 2);

        let it = item(&resp.items[0]);
        assert_eq!(it.id, "github:BeamMP/BeamMP");
        assert_eq!(it.source, "github");
        assert_eq!(it.name, "BeamMP/BeamMP");
        assert_eq!(it.author.as_deref(), Some("BeamMP"));
        assert_eq!(it.downloads.as_deref(), Some("513"));
        assert_eq!(it.published.as_deref(), Some("2026-09-14"));
        assert_eq!(it.category.as_deref(), Some("beamng"));
        assert_eq!(it.key, "https://github.com/BeamMP/BeamMP");
    }

    #[test]
    fn total_pages_caps_at_github_limit() {
        let pages = |total: u64| ((total.min(1000) as u32).div_ceil(PER_PAGE)).max(1);
        assert_eq!(pages(141), 5);
        assert_eq!(pages(0), 1);
        assert_eq!(pages(5000), (1000 / PER_PAGE) + 1);
    }

    #[test]
    fn repo_from_key_supports_various_urls() {
        assert_eq!(
            repo_from_key("https://github.com/BeamMP/BeamMP").unwrap(),
            "BeamMP/BeamMP"
        );
        assert_eq!(repo_from_key("http://github.com/a/b/").unwrap(), "a/b");
        assert_eq!(
            repo_from_key("https://github.com/a/b/tree/main").unwrap(),
            "a/b"
        );
        assert!(repo_from_key("https://example.com/x").is_err());
        assert!(repo_from_key("https://github.com/a/").is_err());
    }

    #[test]
    fn sanitize_strips_operators() {
        assert_eq!(sanitize_query("car v1.0 -map"), "car+v1.0+map");
        assert_eq!(sanitize_query("-car"), "car");
        assert_eq!(sanitize_query("a: /\\ ?"), "a");
        assert_eq!(sanitize_query(""), "");
    }

    #[test]
    fn extract_tag_finds_latest() {
        let html = r#"href="/A/B/releases/tag/v4.22.4""#;
        assert_eq!(extract_tag(html).as_deref(), Some("v4.22.4"));
        assert!(extract_tag("no releases here").is_none());
    }

    #[test]
    fn extract_tag_skips_placeholder_link() {
        let html = r#"
            <a href="/A/B/releases/tag/*name">rename</a>
            <a href="/A/B/releases/tag/v1.0.0">latest</a>
        "#;
        assert_eq!(extract_tag(html).as_deref(), Some("v1.0.0"));
    }

    #[test]
    fn pick_asset_prefers_zip() {
        let html = r#"
            <a href="/A/B/releases/download/v1/B.zip">B.zip</a>
            <a href="/A/B/releases/download/v1/B.zip.sha256">B.zip.sha256</a>
            <a href="/A/B/releases/download/v1/B.tar.gz">B.tar.gz</a>
        "#;
        assert_eq!(
            pick_asset(html).as_deref(),
            Some("https://github.com/A/B/releases/download/v1/B.zip")
        );
    }

    #[test]
    fn pick_asset_fallback_without_zip() {
        let html = r#"<a href="/A/B/releases/download/v1/file.exe">exe</a>"#;
        assert_eq!(
            pick_asset(html).as_deref(),
            Some("https://github.com/A/B/releases/download/v1/file.exe")
        );
    }

    #[test]
    fn pick_asset_empty_when_no_assets() {
        assert!(pick_asset("<p>no assets</p>").is_none());
    }

    #[test]
    fn search_query_builds_from_category_and_words() {
        assert_eq!(search_query(None, None), "topic:beamng");
        assert_eq!(search_query(Some("maps"), None), "topic:beamng+map");
        assert_eq!(
            search_query(Some("vehicles"), Some(" truck ")),
            "topic:beamng+(car+OR+vehicle)+truck"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn network_search_detail_resolve() {
        let client = crate::http::build_client().expect("http client");
        let list = search(&client, None, None, 1).await.expect("search");
        assert!(!list.items.is_empty(), "поиск по topic:beamng пуст");
        assert!(list.total_pages >= 1);

        let d = detail(
            &client,
            &list.items[0].id,
            "https://github.com/BeamMP/BeamMP",
        )
        .await
        .expect("detail");
        assert_eq!(d.item.name, "BeamMP/BeamMP");

        let (url, filename) = resolve_download(&client, "https://github.com/BeamMP/BeamMP")
            .await
            .expect("resolve");
        assert!(url.starts_with("https://"), "url: {url}");
        assert!(filename.ends_with(".zip"), "filename: {filename}");
    }
}

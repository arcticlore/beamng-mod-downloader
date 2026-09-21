use crate::models::{ModDetail, ModItem, ModSearchResult, SourceCategory};
use crate::sources::SourceError;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

/// Поиск по Codeberg (Forgejo/Gitea API v1): репозитории по `beamng` и их
/// latest release с zip-ассетом. Анонимные запросы без токена; rate limit
/// мягкий, но N+1 по релизам ограничен сверху (как в gitlab).
const API: &str = "https://codeberg.org/api/v1";
const PER_PAGE: u32 = 20;
const ENRICH_LIMIT: usize = 15;
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(30 * 60);

static RELEASE_CACHE: LazyLock<Mutex<HashMap<String, (ReleaseAsset, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const CATEGORIES: &[(&str, &str)] = &[
    ("all", ""),
    ("maps", "map"),
    ("vehicles", "car"),
    ("other", "beammp"),
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

async fn fetch(client: &reqwest::Client, url: &str) -> Result<String, SourceError> {
    crate::urlguard::validate_url(url)
        .map_err(|e| SourceError::Network(format!("SSRF-gate: {e}")))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| SourceError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| map_err(e, url))?;
    resp.text()
        .await
        .map_err(|e| SourceError::Network(e.to_string()))
}

fn map_err(e: reqwest::Error, url: &str) -> SourceError {
    if let Some(status) = e.status() {
        if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            return SourceError::Unavailable(format!(
                "Codeberg ограничил частоту запросов. URL: {url}"
            ));
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return SourceError::Parse(format!("не найдено на Codeberg: {url}"));
        }
    }
    SourceError::Network(e.to_string())
}

#[derive(Deserialize, Debug)]
struct SearchBody {
    data: Vec<Repo>,
    total_count: u64,
}

#[derive(Deserialize, Debug)]
struct Repo {
    id: u64,
    name: String,
    full_name: String,
    description: Option<String>,
    html_url: String,
    topics: Vec<String>,
    stars_count: u64,
    updated_at: String,
    owner: OptOwner,
}

#[derive(Deserialize, Debug, Default)]
struct OptOwner {
    #[serde(default)]
    login: String,
    #[serde(default)]
    avatar_url: Option<String>,
}

impl Repo {
    fn owner(&self) -> &str {
        if self.owner.login.is_empty() {
            self.full_name.split('/').next().unwrap_or("")
        } else {
            &self.owner.login
        }
    }
    fn published(&self) -> String {
        self.updated_at.split('T').next().unwrap_or("").to_string()
    }
}

#[derive(Deserialize, Debug, Clone)]
struct ReleaseAsset {
    zip_url: String,
    name: String,
    published: Option<String>,
}

fn pick_zip_asset(body: &[serde_json::Value]) -> Option<ReleaseAsset> {
    for rel in body {
        let published = rel
            .get("published_at")
            .or_else(|| rel.get("created_at"))
            .and_then(|v| v.as_str())
            .map(|s| s.split('T').next().unwrap_or("").to_string());
        if let Some(assets) = rel.get("assets").and_then(|a| a.as_array()) {
            for a in assets {
                let url = a.get("browser_download_url").and_then(|v| v.as_str());
                if let Some(u) = url {
                    let fname = u.rsplit('/').next().unwrap_or("");
                    if fname.to_ascii_lowercase().ends_with(".zip") {
                        return Some(ReleaseAsset {
                            zip_url: u.to_string(),
                            name: fname.to_string(),
                            published,
                        });
                    }
                }
            }
        }
    }
    None
}

fn item(r: &Repo) -> ModItem {
    ModItem {
        id: format!("codeberg:{}", r.full_name),
        source: "codeberg".to_string(),
        name: r.full_name.clone(),
        thumbnail: r.owner.avatar_url.clone(),
        description: r.description.clone(),
        key: r.html_url.clone(),
        category: r.topics.first().cloned(),
        author: Some(r.owner().to_string()),
        published: Some(r.published()),
        downloads: Some(r.stars_count.to_string()),
        size_bytes: None,
    }
}

fn search_extra(category: Option<&str>, query: Option<&str>) -> Option<String> {
    let word = CATEGORIES
        .iter()
        .find(|(id, _)| *id == category.unwrap_or("all"))
        .map(|(_, w)| *w)
        .unwrap_or("");
    let mut terms: Vec<String> = Vec::new();
    if !word.is_empty() {
        terms.push(word.to_string());
    }
    if let Some(q) = query {
        let clean: String = q
            .split_whitespace()
            .flat_map(|w| w.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-'))
            .collect::<String>();
        if !clean.is_empty() {
            terms.push(clean);
        }
    }
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

pub async fn search(
    client: &reqwest::Client,
    query: Option<&str>,
    category: Option<&str>,
    page: u32,
    _order: Option<&str>,
) -> Result<ModSearchResult, SourceError> {
    let extra = search_extra(category, query);
    // Forgejo умеет сортировать по обновлению при включённой теме.
    let url = match &extra {
        Some(w) => format!(
            "{API}/repos/search?q=beamng+{w}&limit={PER_PAGE}&page={}&topic=true&sort=updated",
            page.max(1)
        ),
        None => format!(
            "{API}/repos/search?q=beamng&limit={PER_PAGE}&page={}&topic=true&sort=updated",
            page.max(1)
        ),
    };
    let body = fetch(client, &url).await?;
    let resp: SearchBody = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("Codeberg ответил не JSON: {e}")))?;

    let mut items = Vec::with_capacity(resp.data.len().min(ENRICH_LIMIT));
    for r in resp.data.iter().take(ENRICH_LIMIT) {
        match latest_release(client, &r.full_name).await {
            Ok(_) => items.push(item(r)),
            Err(e) => log::debug!("codeberg: пропуск {} без релиза/zip: {e}", r.full_name),
        }
    }

    let total_pages = ((resp.total_count as u32).div_ceil(PER_PAGE)).max(1);
    Ok(ModSearchResult {
        items,
        total_pages,
        current_page: page.max(1),
    })
}

async fn latest_release(
    client: &reqwest::Client,
    full_name: &str,
) -> Result<ReleaseAsset, SourceError> {
    if let Some(cached) = cached_release(full_name) {
        return Ok(cached);
    }
    let url = format!(
        "{API}/repos/{full_name}/releases?limit=1&draft=false&prerelease=false&page=1"
    );
    let body = fetch(client, &url).await?;
    let raw: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("Codeberg ответил не JSON: {e}")))?;
    let releases = raw
        .as_array()
        .ok_or_else(|| SourceError::Parse("тело releases не массив".to_string()))?;
    let asset = pick_zip_asset(releases).ok_or_else(|| {
        SourceError::Unavailable(format!("у {full_name} нет zip-ассета в последнем релизе"))
    })?;
    cache_release(full_name, asset.clone());
    Ok(asset)
}

fn cached_release(full_name: &str) -> Option<ReleaseAsset> {
    let now = Instant::now();
    let mut cache = RELEASE_CACHE.lock().ok()?;
    cache.retain(|_, (_, at)| now.duration_since(*at) < CACHE_TTL);
    cache.get(full_name).map(|(a, _)| a.clone())
}

fn cache_release(full_name: &str, asset: ReleaseAsset) {
    if let Ok(mut cache) = RELEASE_CACHE.lock() {
        cache.insert(full_name.to_string(), (asset, Instant::now()));
    }
}

fn repo_from_key(key: &str) -> Result<String, SourceError> {
    let parts: Vec<&str> = key.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 4 || parts[1] != "codeberg.org" {
        return Err(SourceError::Parse(format!(
            "не похоже на ссылку Codeberg: {key}"
        )));
    }
    let owner = parts[2];
    let repo = parts[3];
    if owner.is_empty() || repo.is_empty() {
        return Err(SourceError::Parse(format!(
            "не похоже на ссылку Codeberg: {key}"
        )));
    }
    Ok(format!("{owner}/{repo}"))
}

pub async fn detail(
    client: &reqwest::Client,
    _mod_id: &str,
    key: &str,
) -> Result<ModDetail, SourceError> {
    let full = repo_from_key(key)?;
    let url = format!("{API}/repos/{full}");
    let body = fetch(client, &url).await?;
    let r: Repo = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("Codeberg ответил не JSON: {e}")))?;
    let full_description = r.description.clone();
    let item = item(&r);
    Ok(ModDetail {
        item,
        full_description,
        screenshots: Vec::new(),
    })
}

pub async fn resolve_download(
    client: &reqwest::Client,
    key: &str,
) -> Result<(String, String, Option<String>), SourceError> {
    let full = repo_from_key(key)?;
    let asset = latest_release(client, &full).await?;
    let zip_name = format!("{}.zip", full.replace('/', "-"));
    Ok((asset.zip_url, zip_name, asset.published))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_from_key_parses_codeberg_urls() {
        assert_eq!(
            repo_from_key("https://codeberg.org/o/r").unwrap(),
            "o/r"
        );
        assert_eq!(
            repo_from_key("https://codeberg.org/o/r/releases/tag/1.0").unwrap(),
            "o/r"
        );
        assert!(repo_from_key("https://example.com/x").is_err());
        assert!(repo_from_key("https://codeberg.org/o/").is_err());
    }

    #[test]
    fn pick_zip_prefers_zip_asset() {
        let json = r#"[
          {"tag_name":"v1.0","published_at":"2026-02-02T00:00:00Z",
           "assets":[
             {"name":"mod.tar.gz","browser_download_url":"https://codeberg.org/o/r/releases/download/v1.0/mod.tar.gz"},
             {"name":"mod.zip","browser_download_url":"https://codeberg.org/o/r/releases/download/v1.0/mod.zip"},
             {"name":"mod.zip.sha256","browser_download_url":"https://codeberg.org/o/r/releases/download/v1.0/mod.zip.sha256"}
           ]}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        let a = pick_zip_asset(&val).expect("zip asset");
        assert!(a.zip_url.ends_with("mod.zip"));
        assert_eq!(a.published.as_deref(), Some("2026-02-02"));
    }

    #[test]
    fn pick_zip_none_when_no_zip() {
        let json = r#"[{"assets":[{"browser_download_url":"https://codeberg.org/o/r/x.tar.gz"}]}]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(pick_zip_asset(&val).is_none());
    }

    #[test]
    fn build_item_fields() {
        let r: Repo = serde_json::from_str(
            r#"{"id":1,"name":"repo","full_name":"o/repo","description":"d",
                "html_url":"https://codeberg.org/o/repo","topics":["beamng"],
                "stars_count":7,"updated_at":"2026-04-01T00:00:00Z",
                "owner":{"login":"o","avatar_url":"https://a/b.png"}}"#,
        )
        .unwrap();
        let it = item(&r);
        assert_eq!(it.id, "codeberg:o/repo");
        assert_eq!(it.author.as_deref(), Some("o"));
        assert_eq!(it.published.as_deref(), Some("2026-04-01"));
        assert_eq!(it.downloads.as_deref(), Some("7"));
        assert_eq!(it.category.as_deref(), Some("beamng"));
    }
}
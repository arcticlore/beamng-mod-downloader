use crate::models::{ModDetail, ModItem, ModSearchResult, SourceCategory};
use crate::sources::SourceError;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

/// Поиск по GitLab: проекты с topic=beamng и их latest release с zip-ассетом.
/// Публичный API без токена (rate limit анонимов мягкий); поиск = 1 запрос,
/// потом по 1 запросу на проект за latest release — ограничиваем N+1 сверху.
const API: &str = "https://gitlab.com/api/v4";
const PER_PAGE: u32 = 20;
/// Сколько проектов из страницы пытаемся обогатить релизами (бюджет N+1).
const ENRICH_LIMIT: usize = 15;
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Кэш последних релизов по path, чтобы повторные установки не били по API.
static RELEASE_CACHE: LazyLock<Mutex<HashMap<String, (ReleaseAsset, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Кодирование пути `owner/repo` для `/projects/{path}` (слеши -> %2F).
const GITLAB_PATH_CHARS: &AsciiSet = &CONTROLS
    .add(b'/')
    .add(b'?')
    .add(b'#')
    .add(b'%')
    .add(b'+')
    .add(b' ')
    .add(b'&')
    .add(b'=');

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

fn encode_path(path: &str) -> String {
    utf8_percent_encode(path, GITLAB_PATH_CHARS).to_string()
}

async fn fetch_with_headers(
    client: &reqwest::Client,
    url: &str,
) -> Result<(String, Option<String>), SourceError> {
    crate::urlguard::validate_url(url)
        .map_err(|e| SourceError::Network(format!("SSRF-gate: {e}")))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| SourceError::Network(e.to_string()))?;
    let next = resp
        .headers()
        .get("x-next-page")
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);
    let resp = resp.error_for_status().map_err(|e| map_err(e, url))?;
    let body = resp
        .text()
        .await
        .map_err(|e| SourceError::Network(e.to_string()))?;
    Ok((body, next))
}

fn map_err(e: reqwest::Error, url: &str) -> SourceError {
    if let Some(status) = e.status() {
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return SourceError::Unavailable(format!(
                "GitLab ограничил частоту запросов. URL: {url}"
            ));
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return SourceError::Parse(format!("не найдено на GitLab: {url}"));
        }
    }
    SourceError::Network(e.to_string())
}

#[derive(Deserialize, Debug)]
struct Project {
    path_with_namespace: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
    web_url: String,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    star_count: u64,
    last_activity_at: String,
    namespace: OptNamespace,
}

#[derive(Deserialize, Debug, Default)]
struct OptNamespace {
    #[serde(default)]
    path: String,
}

impl Project {
    fn owner(&self) -> &str {
        if self.namespace.path.is_empty() {
            self.path_with_namespace.split('/').next().unwrap_or("")
        } else {
            self.namespace.path.as_str()
        }
    }
    fn published(&self) -> String {
        self.last_activity_at
            .split('T')
            .next()
            .unwrap_or("")
            .to_string()
    }
}

#[derive(Deserialize, Debug, Clone)]
struct ReleaseAsset {
    zip_url: String,
    published: Option<String>,
}

fn pick_zip_asset(releases: &[serde_json::Value]) -> Option<ReleaseAsset> {
    for rel in releases {
        let created = rel
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(|s| s.split('T').next().unwrap_or("").to_string());
        if let Some(link) = rel
            .get("assets")
            .and_then(|a| a.get("links"))
            .and_then(|l| l.as_array())
        {
            for l in link {
                // Фактический `url` — provenance target, который задаёт автор
                // release. GitLab перенаправляет `direct_asset_url` на него,
                // поэтому `url` обязан быть first-party GitLab, иначе внешний
                // target «просочится» через redirect-wrapper.
                let Some(url) = l.get("url").and_then(|v| v.as_str()) else {
                    // Fail closed: без фактического target безопасного выбора нет.
                    continue;
                };
                if crate::urlguard::validate_gitlab_url(url).is_err() {
                    continue;
                }
                let chosen = l
                    .get("direct_asset_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or(url);
                // Редирект-обёртка сама по себе тоже должна быть first-party.
                if crate::urlguard::validate_gitlab_url(chosen).is_err() {
                    continue;
                }
                let fname = chosen.rsplit('/').next().unwrap_or("");
                if fname.to_ascii_lowercase().ends_with(".zip") {
                    return Some(ReleaseAsset {
                        zip_url: chosen.to_string(),
                        published: created,
                    });
                }
            }
        }
    }
    None
}

fn item(p: &Project) -> ModItem {
    ModItem {
        id: format!("gitlab:{}", p.path_with_namespace),
        source: "gitlab".to_string(),
        name: p.path_with_namespace.clone(),
        thumbnail: p.avatar_url.clone(),
        description: p.description.clone(),
        key: p.web_url.clone(),
        category: p.topics.first().cloned(),
        author: Some(p.owner().to_string()),
        published: Some(p.published()),
        downloads: Some(p.star_count.to_string()),
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
    order: Option<&str>,
) -> Result<ModSearchResult, SourceError> {
    let extra = search_extra(category, query);
    let sort = match order {
        Some("popularity") => "star_count",
        Some("updated") => "last_activity_at",
        _ => "last_activity_at",
    };

    let url = match &extra {
        Some(w) => format!(
            "{API}/projects?topic=beamng&search={w}&order_by={sort}&sort=desc&per_page={PER_PAGE}&page={}",
            page.max(1)
        ),
        None => format!(
            "{API}/projects?topic=beamng&order_by={sort}&sort=desc&per_page={PER_PAGE}&page={}",
            page.max(1)
        ),
    };
    let (body, next) = fetch_with_headers(client, &url).await?;
    let projects: Vec<Project> = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("GitLab ответил не JSON: {e}")))?;

    let mut items = Vec::with_capacity(projects.len().min(ENRICH_LIMIT));
    for p in projects.iter().take(ENRICH_LIMIT) {
        match latest_release(client, &p.path_with_namespace).await {
            Ok(_) => items.push(item(p)),
            Err(e) => log::debug!(
                "gitlab: пропуск {} без релиза/zip: {e}",
                p.path_with_namespace
            ),
        }
    }

    let total_pages = if next.is_some() {
        page.max(1) + 1
    } else {
        page.max(1)
    };
    Ok(ModSearchResult {
        items,
        total_pages,
        current_page: page.max(1),
    })
}

async fn latest_release(client: &reqwest::Client, path: &str) -> Result<ReleaseAsset, SourceError> {
    if let Some(cached) = cached_release(path) {
        return Ok(cached);
    }
    let url = format!(
        "{API}/projects/{}/releases?per_page=1&order_by=created_at&sort=desc",
        encode_path(path)
    );
    let (body, _) = fetch_with_headers(client, &url).await?;
    let raw: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("GitLab ответил не JSON: {e}")))?;
    let releases = raw
        .as_array()
        .ok_or_else(|| SourceError::Parse("тело releases не массив".to_string()))?;
    let asset = pick_zip_asset(releases).ok_or_else(|| {
        SourceError::Unavailable(format!("у {path} нет zip-ассета в последнем релизе"))
    })?;
    cache_release(path, asset.clone());
    Ok(asset)
}

fn cached_release(path: &str) -> Option<ReleaseAsset> {
    let now = Instant::now();
    let mut cache = RELEASE_CACHE.lock().ok()?;
    cache.retain(|_, (_, at)| now.duration_since(*at) < CACHE_TTL);
    cache.get(path).map(|(a, _)| a.clone())
}

fn cache_release(path: &str, asset: ReleaseAsset) {
    if let Ok(mut cache) = RELEASE_CACHE.lock() {
        cache.insert(path.to_string(), (asset, Instant::now()));
    }
}

fn repo_from_key(key: &str) -> Result<String, SourceError> {
    let parts: Vec<&str> = key.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 4 || parts[1] != "gitlab.com" {
        return Err(SourceError::Parse(format!(
            "не похоже на ссылку GitLab: {key}"
        )));
    }
    let owner = parts[2];
    let repo = parts[3];
    if owner.is_empty() || repo.is_empty() {
        return Err(SourceError::Parse(format!(
            "не похоже на ссылку GitLab: {key}"
        )));
    }
    Ok(format!("{owner}/{repo}"))
}

pub async fn detail(
    client: &reqwest::Client,
    _mod_id: &str,
    key: &str,
) -> Result<ModDetail, SourceError> {
    let path = repo_from_key(key)?;
    let url = format!("{API}/projects/{}", encode_path(&path));
    let (body, _) = fetch_with_headers(client, &url).await?;
    let p: Project = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("GitLab ответил не JSON: {e}")))?;
    let full_description = p.description.clone();
    let item = item(&p);
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
    let path = repo_from_key(key)?;
    let asset = latest_release(client, &path).await?;
    crate::urlguard::validate_gitlab_url(&asset.zip_url)
        .map_err(|e| SourceError::Network(format!("SSRF-gate: {e}")))?;
    let zip_name = format!("{}.zip", path.replace('/', "-"));
    Ok((asset.zip_url, zip_name, asset.published))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_from_key_parses_gitlab_urls() {
        assert_eq!(repo_from_key("https://gitlab.com/o/r").unwrap(), "o/r");
        assert_eq!(
            repo_from_key("https://gitlab.com/o/r/-/releases/tag/1.0").unwrap(),
            "o/r"
        );
        assert!(repo_from_key("https://example.com/x").is_err());
        assert!(repo_from_key("https://gitlab.com/o/").is_err());
    }

    #[test]
    fn encode_path_escapes_slashes() {
        assert_eq!(encode_path("o/r"), "o%2Fr");
        assert_eq!(encode_path("o/r?x&y=1"), "o%2Fr%3Fx%26y%3D1");
    }

    #[test]
    fn pick_zip_prefers_direct_asset() {
        let json = r#"[
          {"created_at":"2026-03-01T10:00:00Z",
           "assets":{"links":[
             {"name":"mod","url":"https://gitlab.com/o/r/-/jobs/123/artifacts/raw/mod.tar.gz","direct_asset_url":"https://gitlab.com/o/r/-/releases/1.0/downloads/mod.tar.gz"},
             {"name":"zip","url":"https://gitlab.com/o/r/-/jobs/123/artifacts/raw/mod.zip","direct_asset_url":"https://gitlab.com/o/r/-/releases/1.0/downloads/mod.zip"},
             {"name":"sha","url":"https://gitlab.com/o/r/-/jobs/123/artifacts/raw/mod.zip.sha256","direct_asset_url":"https://gitlab.com/o/r/-/releases/1.0/downloads/mod.zip.sha256"}
           ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        let a = pick_zip_asset(&val).expect("zip asset");
        assert!(a.zip_url.ends_with("mod.zip"));
        assert_eq!(a.published.as_deref(), Some("2026-03-01"));
    }

    #[test]
    fn pick_zip_none_when_no_zip_link() {
        let json = r#"[
          {"assets":{"links":[
            {"url":"https://gitlab.com/o/r/-/jobs/1/artifacts/raw/mod.tar.gz",
             "direct_asset_url":"https://gitlab.com/o/r/-/releases/v1/downloads/mod.tar.gz"}
          ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(pick_zip_asset(&val).is_none());
    }

    #[test]
    fn pick_zip_rejects_external_underlying_url() {
        // Blocker reproduction: `direct_asset_url` first-party, но фактический
        // `url` (на который GitLab перенаправляет) внешний — даже github.com,
        // глобально разрешённый другому source, не должен стать GitLab
        // first-party через redirect-wrapper.
        let json = r#"[
          {"created_at":"2026-03-01T10:00:00Z",
           "assets":{"links":[
             {"name":"evil","url":"https://github.com/evil/mod.zip",
              "direct_asset_url":"https://gitlab.com/o/r/-/releases/v1/downloads/mod.zip"},
             {"name":"good","url":"https://gitlab.com/o/r/-/jobs/7/artifacts/raw/mod.zip",
              "direct_asset_url":"https://gitlab.com/o/r/-/releases/1.0/downloads/mod.zip"}
           ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        let a = pick_zip_asset(&val).expect("должен выбрать first-party zip");
        assert_eq!(a.zip_url, "https://gitlab.com/o/r/-/releases/1.0/downloads/mod.zip");
    }

    #[test]
    fn rejects_redirect_wrapper_to_evil_cdn() {
        let json = r#"[
          {"assets":{"links":[
            {"name":"evil","url":"https://evil-cdn.example/mod.zip",
             "direct_asset_url":"https://gitlab.com/o/r/-/releases/v1/downloads/mod.zip"}
          ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(pick_zip_asset(&val).is_none());
    }

    #[test]
    fn rejects_redirect_wrapper_to_lookalike_gitlab() {
        let json = r#"[
          {"assets":{"links":[
            {"name":"evil","url":"https://gitlab.com.evil.example/mod.zip",
             "direct_asset_url":"https://gitlab.com/o/r/-/releases/v1/downloads/mod.zip"}
          ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(pick_zip_asset(&val).is_none());
    }

    #[test]
    fn rejects_external_wrapper_even_with_first_party_url() {
        let json = r#"[
          {"assets":{"links":[
            {"name":"evil","url":"https://gitlab.com/o/r/-/jobs/1/artifacts/raw/mod.zip",
             "direct_asset_url":"https://evil-cdn.example/mod.zip"}
          ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(pick_zip_asset(&val).is_none());
    }

    #[test]
    fn rejects_link_missing_actual_url() {
        // Fail closed: официальный release link обязан содержать фактический
        // `url`; объект с одним `direct_asset_url` не выбирается.
        let json = r#"[
          {"assets":{"links":[
            {"name":"no-url","direct_asset_url":"https://gitlab.com/o/r/-/releases/v1/downloads/mod.zip"}
          ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(pick_zip_asset(&val).is_none());
    }

    #[test]
    fn accepts_first_party_url_and_direct_asset() {
        let json = r#"[
          {"assets":{"links":[
            {"name":"ok","url":"https://gitlab.com/o/r/-/jobs/123/artifacts/raw/mod.zip",
             "direct_asset_url":"https://gitlab.com/o/r/-/releases/v1/downloads/mod.zip"}
          ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        let a = pick_zip_asset(&val).expect("должен выбрать first-party zip");
        assert_eq!(a.zip_url, "https://gitlab.com/o/r/-/releases/v1/downloads/mod.zip");
    }

    #[test]
    fn accepts_first_party_url_without_direct_asset() {
        // Если `direct_asset_url` отсутствует, допустим проверенный `url`.
        let json = r#"[
          {"assets":{"links":[
            {"name":"ok","url":"https://gitlab.com/o/r/-/jobs/5/artifacts/raw/mod.zip"}
          ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        let a = pick_zip_asset(&val).expect("должен выбрать first-party zip");
        assert_eq!(a.zip_url, "https://gitlab.com/o/r/-/jobs/5/artifacts/raw/mod.zip");
    }

    #[test]
    fn pick_zip_prefers_zip_over_foreign_and_rejects_globally_allowed_target() {
        // Иначе глобально разрешённый другой source (github.com) как фактический
        // `url` — отклоняется наравне с неразрешённым внешним.
        let json = r#"[
          {"assets":{"links":[
            {"name":"evil","url":"https://github.com/o/r/releases/download/v1/mod.zip",
             "direct_asset_url":"https://gitlab.com/o/r/-/releases/v1/downloads/mod.zip"},
            {"name":"good","url":"https://gitlab.com/o/r/-/jobs/9/artifacts/raw/mod.zip",
             "direct_asset_url":"https://gitlab.com/o/r/-/releases/v2/downloads/mod.zip"}
          ]}}
        ]"#;
        let val: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        let a = pick_zip_asset(&val).expect("должен выбрать first-party zip");
        assert_eq!(a.zip_url, "https://gitlab.com/o/r/-/releases/v2/downloads/mod.zip");
    }

    #[test]
    fn build_item_fields() {
        let p: Project = serde_json::from_str(
            r#"{"id":1,"name":"Repo","path_with_namespace":"o/repo",
                "description":"d","avatar_url":null,"web_url":"https://gitlab.com/o/repo",
                "topics":["beamng"],"star_count":42,
                "last_activity_at":"2026-05-02T00:00:00Z","namespace":{"path":"o"}}"#,
        )
        .unwrap();
        let it = item(&p);
        assert_eq!(it.id, "gitlab:o/repo");
        assert_eq!(it.source, "gitlab");
        assert_eq!(it.author.as_deref(), Some("o"));
        assert_eq!(it.published.as_deref(), Some("2026-05-02"));
        assert_eq!(it.downloads.as_deref(), Some("42"));
    }
}

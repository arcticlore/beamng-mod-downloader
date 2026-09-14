use crate::models::{ModDetail, ModItem, ModSearchResult, SourceCategory};
use crate::sources::SourceError;
use anyhow::Result;
use reqwest::{Client, StatusCode};
use serde_json::Value;

pub const BASE: &str = "https://api.beamng.com";

pub fn categories() -> Vec<SourceCategory> {
    vec![
        SourceCategory {
            id: "veh".to_string(),
            label: "Авто".to_string(),
        },
        SourceCategory {
            id: "map".to_string(),
            label: "Карты".to_string(),
        },
        SourceCategory {
            id: "mis".to_string(),
            label: "Сценарии".to_string(),
        },
        SourceCategory {
            id: "mod".to_string(),
            label: "Прочее".to_string(),
        },
    ]
}

fn auth_header(token: &str) -> String {
    format!("Bearer {token}")
}

fn icon_url(tagid: &str, ver: &str) -> Option<String> {
    if tagid.is_empty() || ver.is_empty() {
        return None;
    }
    Some(format!("{BASE}/s1/v4/download/mods/{tagid}/{ver}/icon.jpg"))
}

fn download_url(tagid: &str, ver: &str, filename: &str) -> String {
    format!("{BASE}/s1/v4/download/mods/{tagid}/{ver}/{filename}")
}

fn item_from_mod_json(v: &Value) -> Option<ModItem> {
    let id = v
        .get("tagid")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| v.get("id").and_then(Value::as_u64).map(|n| n.to_string()))?;
    let ver = v
        .get("ver")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let filename = v
        .get("filename")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let name = v
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| v.get("title").and_then(Value::as_str))
        .map(|s| s.to_string())
        .unwrap_or_else(|| id.clone());
    let published = v
        .get("published")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| v.get("date").and_then(Value::as_str).map(ToOwned::to_owned));
    let description = v
        .get("message")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            v.get("description")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        });
    let size_bytes = v
        .get("filesize")
        .and_then(Value::as_u64)
        .or_else(|| v.get("size").and_then(Value::as_u64));
    let category = v
        .get("categories")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    Some(ModItem {
        id: format!("beamng:{id}"),
        source: "beamng".to_string(),
        name,
        thumbnail: icon_url(&id, &ver),
        description,
        key: format!("{id}/{ver}/{filename}"),
        category,
        author: None,
        published,
        downloads: None,
        size_bytes,
    })
}

pub async fn search(
    client: &Client,
    query: Option<&str>,
    _category: Option<&str>,
    page: u32,
    token: Option<&str>,
) -> Result<ModSearchResult, SourceError> {
    let token = token.ok_or_else(|| {
        SourceError::Auth("указан источник BeamNG, но не сохранён токен авторизации".into())
    })?;

    let mut body = serde_json::json!({
        "query": query.unwrap_or(""),
        "order_by": "created",
        "order": "desc",
        "page": page.saturating_sub(1),
        "categories": [],
    });

    if let Some(cat) = _category {
        if !cat.is_empty() {
            body["categories"] = serde_json::json!([cat]);
        }
    }

    let resp = client
        .post(format!("{BASE}/s1/v4/getMods"))
        .header("Authorization", auth_header(token))
        .json(&body)
        .send()
        .await
        .map_err(|e| SourceError::Network(e.to_string()))?;

    if resp.status() == StatusCode::UNAUTHORIZED {
        return Err(SourceError::Auth(
            "токен не принят репозиторием BeamNG (401). Проверьте токен в настройках".into(),
        ));
    }
    let status = resp.status();
    let json: Value = resp
        .json()
        .await
        .map_err(|e| SourceError::Parse(format!("нет JSON: {e} (HTTP {status})")))?;

    let data = json
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| SourceError::Parse("в ответе getMods нет массива data".into()))?;

    let items: Vec<ModItem> = data.iter().filter_map(item_from_mod_json).collect();
    let total_pages = json
        .get("totalPages")
        .or_else(|| json.get("pages"))
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .max(1) as u32;

    Ok(ModSearchResult {
        items,
        total_pages,
        current_page: page,
    })
}

pub async fn detail(
    client: &Client,
    mod_id: &str,
    key: &str,
    token: Option<&str>,
) -> Result<ModDetail, SourceError> {
    let token = token.ok_or_else(|| SourceError::Auth("нет токена авторизации BeamNG".into()))?;
    let id = key.split('/').next().unwrap_or(mod_id);

    let resp = client
        .get(format!("{BASE}/s1/v4/getMod/{id}"))
        .header("Authorization", auth_header(token))
        .send()
        .await
        .map_err(|e| SourceError::Network(e.to_string()))?;

    if resp.status() == StatusCode::UNAUTHORIZED {
        return Err(SourceError::Auth(
            "токен не принят репозиторием BeamNG (401)".into(),
        ));
    }
    let json: Value = resp
        .json()
        .await
        .map_err(|e| SourceError::Parse(format!("нет JSON: {e}")))?;
    let data = json
        .get("data")
        .ok_or_else(|| SourceError::Parse("нет data".into()))?;

    let item = item_from_mod_json(data)
        .ok_or_else(|| SourceError::Parse("не удалось преобразовать данные мода BeamNG".into()))?;

    let mut screenshots = Vec::new();
    if let Some(big) = data.get("screenshotBig").and_then(Value::as_str) {
        screenshots.push(big.to_string());
    }
    if let Some(icon) = data.get("icon").and_then(Value::as_str) {
        screenshots.push(icon.to_string());
    }

    Ok(ModDetail {
        full_description: item.description.clone(),
        screenshots,
        item,
    })
}

pub async fn resolve_download(
    _client: &Client,
    key: &str,
    _token: Option<&str>,
) -> Result<(String, String), SourceError> {
    let parts: Vec<&str> = key.split('/').collect();
    if parts.len() < 3 {
        return Err(SourceError::Parse(
            "некорректный ключ мода BeamNG: ожидалось id/ver/filename".into(),
        ));
    }
    let (id, ver, filename) = (parts[0], parts[1], parts[2]);
    if filename.trim().is_empty() {
        return Err(SourceError::Parse("у мода BeamNG не задан filename".into()));
    }
    Ok((download_url(id, ver, filename), filename.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn item_from_full_mod_json() {
        let v = json!({
            "tagid": "abc.123",
            "ver": "1.2.3",
            "filename": "my_mod.zip",
            "name": "My Mod",
            "message": "Полное описание",
            "filesize": 4096,
            "categories": ["veh"],
            "published": "2024-05-01T00:00:00Z"
        });
        let item = item_from_mod_json(&v).expect("valid mod json");
        assert_eq!(item.id, "beamng:abc.123");
        assert_eq!(item.name, "My Mod");
        assert_eq!(item.key, "abc.123/1.2.3/my_mod.zip");
        assert_eq!(item.description.as_deref(), Some("Полное описание"));
        assert_eq!(item.size_bytes, Some(4096));
        assert_eq!(item.category.as_deref(), Some("veh"));
        assert_eq!(
            item.thumbnail.as_deref(),
            Some("https://api.beamng.com/s1/v4/download/mods/abc.123/1.2.3/icon.jpg")
        );
    }

    #[test]
    fn item_falls_back_to_numeric_id_and_title() {
        let v = json!({ "id": 42, "title": "Extra Mod" });
        let item = item_from_mod_json(&v).expect("minimal mod json");
        assert_eq!(item.id, "beamng:42");
        assert_eq!(item.name, "Extra Mod");
        assert_eq!(item.thumbnail, None);
        assert_eq!(item.size_bytes, None);
    }

    #[test]
    fn icon_url_requires_tagid_and_ver() {
        assert_eq!(icon_url("", "1.0"), None);
        assert_eq!(icon_url("a.b", ""), None);
        assert_eq!(
            icon_url("a.b", "1.0").as_deref(),
            Some("https://api.beamng.com/s1/v4/download/mods/a.b/1.0/icon.jpg")
        );
    }

    #[tokio::test]
    async fn resolve_download_builds_url_and_filename() {
        let key = "abc.123/1.2.3/pack.zip";
        let (url, filename) = super::resolve_download(&reqwest::Client::new(), key, None)
            .await
            .expect("valid key");
        assert_eq!(
            url,
            "https://api.beamng.com/s1/v4/download/mods/abc.123/1.2.3/pack.zip"
        );
        assert_eq!(filename, "pack.zip");
    }

    #[tokio::test]
    async fn resolve_download_rejects_bad_keys() {
        let client = reqwest::Client::new();
        assert!(super::resolve_download(&client, "short", None)
            .await
            .is_err());
        assert!(super::resolve_download(&client, "a/1/", None)
            .await
            .is_err());
    }
}

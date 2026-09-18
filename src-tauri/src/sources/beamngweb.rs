use crate::http;
use crate::models::{ModDetail, ModItem, ModSearchResult, SourceCategory};
use crate::sources::SourceError;
use anyhow::Result;
use scraper::{Html, Selector};

pub const BASE: &str = "https://www.beamng.com";

const CATEGORIES: &[(&str, &str)] = &[
    ("all", ""),
    ("vehicles", "vehicles.2"),
    ("maps", "terrains-levels-maps.9"),
    ("scenarios", "scenarios.8"),
    ("automation", "automation.16"),
    ("land", "land.3"),
    ("skins", "skins.12"),
    ("sounds", "sounds.13"),
    ("ui", "user-interface-apps.10"),
    ("track", "track-builder.17"),
    ("license-plates", "license-plates.15"),
    ("mods-of-mods", "mods-of-mods.7"),
];

pub fn categories() -> Vec<SourceCategory> {
    CATEGORIES
        .iter()
        .map(|(id, _)| SourceCategory {
            id: id.to_string(),
            label: match *id {
                "all" => "Все моды".to_string(),
                "vehicles" => "Авто".to_string(),
                "maps" => "Карты и террейны".to_string(),
                "scenarios" => "Сценарии".to_string(),
                "automation" => "Автоматизация".to_string(),
                "land" => "Ландшафт".to_string(),
                "skins" => "Скины".to_string(),
                "sounds" => "Звуки".to_string(),
                "ui" => "UI и приложения".to_string(),
                "track" => "Track Builder".to_string(),
                "license-plates" => "Номерные знаки".to_string(),
                "mods-of-mods" => "Моды модов".to_string(),
                other => other.to_string(),
            },
        })
        .collect()
}

fn category_path(category: &str) -> &str {
    CATEGORIES
        .iter()
        .find(|(id, _)| *id == category)
        .map(|(_, p)| *p)
        .unwrap_or("")
}

fn listing_url(category: &str, page: u32, order: &str) -> String {
    let path = category_path(category);
    let page = page.max(1);
    let mut url = format!("{BASE}/resources/");
    let mut params: Vec<String> = Vec::new();
    if !path.is_empty() || page > 1 {
        params.push(format!("page={page}"));
    }
    if !path.is_empty() {
        params.insert(0, format!("categories/{path}"));
    }
    if let Some(o) = order_param(order) {
        params.push(format!("order={o}"));
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }
    url
}

/// Маппинг сортировок пользователя на параметр `order` XenForo.
fn order_param(order: &str) -> Option<&'static str> {
    match order {
        "updated" => Some("update_date"),
        "name" => Some("title"),
        "popularity" => Some("download_count"),
        _ => None,
    }
}

fn absolute(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with("//") {
        format!("https:{href}")
    } else {
        let base = base.trim_end_matches('/');
        format!("{base}/{href}")
    }
}

fn total_pages(doc: &Html) -> u32 {
    Selector::parse(".pageNavHeader")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
        .and_then(|n| {
            n.text()
                .collect::<Vec<_>>()
                .concat()
                .split_whitespace()
                .filter_map(|w| w.parse::<u32>().ok())
                .next_back()
        })
        .unwrap_or(1)
        .max(1)
}

/// Парсит листинг ресурсов (`li.resourceListItem`) в список модов.
fn parse_listing(html: &str, page: u32) -> Result<ModSearchResult, SourceError> {
    let doc = Html::parse_document(html);
    let sel_item =
        Selector::parse("li.resourceListItem").map_err(|e| SourceError::Parse(e.to_string()))?;
    let sel_icon = Selector::parse(".resourceIcon img").unwrap();
    let sel_title = Selector::parse("h3.title a").unwrap();
    let sel_author = Selector::parse(".resourceDetails a[href*=\"/authors/\"]").unwrap();
    let sel_cat = Selector::parse(".resourceDetails a[href*=\"/categories/\"]").unwrap();
    let sel_datetime = Selector::parse(".resourceDetails .DateTime").unwrap();
    let sel_tagline = Selector::parse(".tagLine").unwrap();
    let sel_downloads = Selector::parse(".resourceDownloads dd").unwrap();

    let mut items = Vec::new();
    for li in doc.select(&sel_item) {
        let Some(href) = li
            .select(&sel_title)
            .next()
            .and_then(|a| a.value().attr("href"))
        else {
            continue;
        };
        let id = li
            .value()
            .attr("id")
            .and_then(|v| v.strip_prefix("resource-"))
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        let title = li
            .select(&sel_title)
            .next()
            .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string())
            .filter(|t| !t.is_empty());
        let name = title.unwrap_or_else(|| format!("id {id}"));
        let icon = li
            .select(&sel_icon)
            .next()
            .and_then(|img| img.value().attr("src"))
            .map(|src| absolute(BASE, src));
        let author = li
            .select(&sel_author)
            .next()
            .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string());
        let cat_name = li
            .select(&sel_cat)
            .next()
            .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string());
        let published = li
            .select(&sel_datetime)
            .next()
            .and_then(|s| s.value().attr("title"))
            .map(ToOwned::to_owned);
        let short = li
            .select(&sel_tagline)
            .next()
            .map(|n| {
                n.text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|t| !t.is_empty());

        let downloads = li
            .select(&sel_downloads)
            .next()
            .map(|n| {
                n.text()
                    .collect::<Vec<_>>()
                    .join("")
                    .split(|c: char| !c.is_ascii_digit())
                    .collect::<Vec<_>>()
                    .concat()
            })
            .filter(|s| !s.is_empty());

        items.push(ModItem {
            id: format!("beamngweb:{id}"),
            source: "beamngweb".to_string(),
            name,
            thumbnail: icon,
            description: short,
            key: absolute(BASE, href),
            category: cat_name,
            author,
            published,
            downloads,
            size_bytes: None,
        });
    }

    Ok(ModSearchResult {
        items,
        total_pages: total_pages(&doc),
        current_page: page,
    })
}

pub async fn search(
    client: &reqwest::Client,
    _query: Option<&str>,
    category: Option<&str>,
    page: u32,
    order: Option<&str>,
) -> Result<ModSearchResult, SourceError> {
    let url = listing_url(category.unwrap_or("all"), page, order.unwrap_or(""));
    let html = http::fetch_string(client, &url, None).await?;
    let mut result = parse_listing(&html, page)?;
    // pageNavHeader показывает *глобальное* число страниц ресурсов, а не
    // категории — для категорий страниц может быть тысячи и они врут.
    if category != Some("all") {
        // неполная страница = категория закончилась
        let is_last = result.items.len() < 100;
        result.total_pages = if is_last { page.max(1) } else { page + 1 };
    }
    if result.items.is_empty() && page > 1 {
        // категория могла закончиться раньше
        return Ok(ModSearchResult {
            items: Vec::new(),
            total_pages: page.saturating_sub(1).max(1),
            current_page: page,
        });
    }
    Ok(result)
}

/// Размер из подписи кнопки скачивания, например "191.6 MB .zip".
fn parse_size(text: &str) -> Option<u64> {
    let upper = text.to_uppercase();
    let (num, mult) = if upper.contains("GB") {
        (upper.replace("GB", ""), 1024u64 * 1024 * 1024)
    } else if upper.contains("MB") {
        (upper.replace("MB", ""), 1024u64 * 1024)
    } else if upper.contains("KB") {
        (upper.replace("KB", ""), 1024u64)
    } else {
        return None;
    };
    let value: f64 = num
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())?;
    Some((value * mult as f64).round() as u64)
}

/// Скриншоты и полное описание со страницы ресурса.
fn details_from_doc(doc: &Html) -> (Option<String>, Vec<String>) {
    let body_node = Selector::parse("blockquote.messageText")
        .ok()
        .and_then(|sel| doc.select(&sel).next());

    let full_description = body_node
        .as_ref()
        .map(|node| {
            node.text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|t| !t.is_empty());

    let mut screenshots = Vec::new();
    if let Some(body) = body_node {
        let sel_img = Selector::parse("img").unwrap();
        for img in body.select(&sel_img) {
            if let Some(src) = img.value().attr("src") {
                if src.contains("/attachments/") {
                    let url = absolute(BASE, src);
                    if !screenshots.contains(&url) {
                        screenshots.push(url);
                    }
                }
            }
        }
    }
    (full_description, screenshots)
}

/// Поля, извлекаемые со страницы ресурса (общие для detail и resolve_download).
struct ResourcePage {
    name: Option<String>,
    author: Option<String>,
    full_description: Option<String>,
    screenshots: Vec<String>,
    size_bytes: Option<u64>,
    download_url: Option<String>,
}

fn page_payload(html: &str) -> ResourcePage {
    let doc = Html::parse_document(html);

    let name = Selector::parse("meta[property='og:title']")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
        .and_then(|n| n.value().attr("content"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let author = Selector::parse("a[href*=\"/authors/\"]")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
        .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string())
        .filter(|a| !a.is_empty());

    let (full_description, screenshots) = details_from_doc(&doc);

    let size_bytes = Selector::parse("a[href*=\"download?version=\"] small")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
        .map(|s| s.text().collect::<Vec<_>>().concat())
        .and_then(|t| parse_size(&t));

    let download_url = Selector::parse("a[href*=\"download?version=\"]")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
        .and_then(|a| a.value().attr("href"))
        .map(|href| absolute(BASE, href));

    ResourcePage {
        name,
        author,
        full_description,
        screenshots,
        size_bytes,
        download_url,
    }
}

/// Дату последнего обновления берём из секции «Recent Updates» той же страницы,
/// на которую позже (при проверке обновлений) смотрит `detail`.
fn published_from_page(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    Selector::parse(".section.updates .DateTime")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
        .and_then(|n| {
            n.value()
                .attr("data-datestring")
                .or_else(|| n.value().attr("title"))
                .map(ToOwned::to_owned)
        })
}

pub async fn detail(
    client: &reqwest::Client,
    mod_id: &str,
    key: &str,
) -> Result<ModDetail, SourceError> {
    let html = http::fetch_string(client, key, None).await?;
    let p = page_payload(&html);
    let published = published_from_page(&html);

    Ok(ModDetail {
        item: ModItem {
            id: format!("beamngweb:{mod_id}"),
            source: "beamngweb".to_string(),
            name: p.name.unwrap_or_else(|| mod_id.to_string()),
            thumbnail: p.screenshots.first().cloned(),
            description: p.full_description.clone(),
            key: key.to_string(),
            category: None,
            author: p.author,
            published,
            downloads: None,
            size_bytes: p.size_bytes,
        },
        full_description: p.full_description,
        screenshots: p.screenshots,
    })
}

/// Возвращает прямую ссылку на архив (следующую за 302) и имя локального файла
/// вида `slug.zip` — при скачивании reqwest сам пройдёт редирект на R2.
pub async fn resolve_download(
    client: &reqwest::Client,
    key: &str,
) -> Result<(String, String, Option<String>), SourceError> {
    let mut html = http::fetch_string(client, key, None).await?;
    let mut url = page_payload(&html).download_url;
    if url.is_none() {
        // сайт мог отдать пустой/обрезанный ответ — пробуем ещё раз
        html = http::fetch_string(client, key, None).await?;
        url = page_payload(&html).download_url;
    }
    let url = url.ok_or_else(|| {
        SourceError::Parse("не найдена кнопка скачивания на странице мода".into())
    })?;

    let published = published_from_page(&html);

    let slug = key
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("mod");
    let filename = http::sanitize_filename(slug) + ".zip";

    Ok((url, filename, published))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_listing() -> &'static str {
        r#"<html><body>
          <div class="pageNavHeader">Page 3 of 1572</div>
          <ol class="resourceList">
            <li class="resourceListItem visible" id="resource-28903">
              <div class="listBlock resourceImage">
                <a href="resources/1982-hirochi-rush-demo.28903/" class="resourceIcon"><img src="data/resource_icons/28/28903.jpg?t" alt="" /></a>
              </div>
              <div class="listBlock main">
                <h3 class="title"><a href="resources/1982-hirochi-rush-demo.28903/">1982 Hirochi RUSH Demo</a> <span class="version">1.4</span></h3>
                <div class="resourceDetails muted">
                  <a href="resources/authors/lj74.330398/">LJ74</a>,
                  <span class="DateTime" title="Nov 7, 2023 at 10:07 AM">Nov 7, 2023</span>,
                  <a href="resources/categories/land.3/">Land</a>
                </div>
                <div class="tagLine">Demo version of the Hirochi RUSH</div>
                <dl class="resourceDownloads"><dt>Downloads:</dt> <dd>10,170</dd></dl>
              </div>
            </li>
          </ol>
        </body></html>"#
    }

    fn sample_detail() -> &'static str {
        r#"<html><head>
          <meta property="og:title" content="1982 Hirochi RUSH Demo" />
        </head><body>
          <div class="resourceDetails">
            <a href="resources/authors/lj74.330398/">LJ74</a>
          </div>
          <div class="section updates">
            <h3 class="textHeading">Recent Updates</h3>
            <ol>
              <li><a href="resources/1982-hirochi-rush-demo.28903/update?update=72681">0.39 Refresh</a>
                  <span class="postDate"><abbr class="DateTime" data-time="0" data-datestring="Sep 14, 2026">Sep 14, 2026</abbr></span></li>
              <li><a href="resources/1982-hirochi-rush-demo.28903/update?update=1">0.38</a>
                  <span class="postDate"><abbr class="DateTime" data-datestring="Nov 7, 2023">Nov 7, 2023</abbr></span></li>
            </ol>
          </div>
          <blockquote class="messageText">
            Great <b>car</b> demo.
            <a href="https://www.beamng.com/attachments/x.1076407/"><img src="data/attachments/1058/1058038.jpg" alt="" /></a>
          </blockquote>
          <label class="downloadButton">
            <a href="resources/1982-hirochi-rush-demo.28903/download?version=72681" class="inner">
              Download Now
              <small class="minorText">191.6 MB .zip</small>
            </a>
          </label>
        </body></html>"#
    }

    #[test]
    fn listing_url_formats() {
        assert_eq!(listing_url("all", 1, ""), "https://www.beamng.com/resources/");
        assert_eq!(
            listing_url("all", 2, ""),
            "https://www.beamng.com/resources/?page=2"
        );
        assert_eq!(
            listing_url("vehicles", 1, ""),
            "https://www.beamng.com/resources/?categories/vehicles.2&page=1"
        );
        assert_eq!(
            listing_url("vehicles", 3, ""),
            "https://www.beamng.com/resources/?categories/vehicles.2&page=3"
        );
        assert_eq!(
            listing_url("vehicles", 1, "popularity"),
            "https://www.beamng.com/resources/?categories/vehicles.2&page=1&order=download_count"
        );
        assert_eq!(
            listing_url("all", 1, "updated"),
            "https://www.beamng.com/resources/?order=update_date"
        );
    }

    #[test]
    fn parse_listing_extracts_items() {
        let res = parse_listing(sample_listing(), 3).expect("parse ok");
        assert_eq!(res.items.len(), 1);
        assert_eq!(res.total_pages, 1572);
        assert_eq!(res.current_page, 3);
        let it = &res.items[0];
        assert_eq!(it.id, "beamngweb:28903");
        assert_eq!(it.name, "1982 Hirochi RUSH Demo");
        assert_eq!(it.author.as_deref(), Some("LJ74"));
        assert_eq!(it.published.as_deref(), Some("Nov 7, 2023 at 10:07 AM"));
        assert_eq!(it.category.as_deref(), Some("Land"));
        assert_eq!(it.downloads.as_deref(), Some("10170"));
        assert_eq!(
            it.thumbnail.as_deref(),
            Some("https://www.beamng.com/data/resource_icons/28/28903.jpg?t")
        );
        assert_eq!(
            it.description.as_deref(),
            Some("Demo version of the Hirochi RUSH")
        );
        assert_eq!(
            it.key,
            "https://www.beamng.com/resources/1982-hirochi-rush-demo.28903/"
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn parse_size_units() {
        assert_eq!(parse_size("191.6 MB .zip"), Some(200_907_162));
        assert_eq!(parse_size("1.2 GB .zip"), Some(1_288_490_189));
        assert_eq!(parse_size("512 KB .zip"), Some(524_288));
        assert_eq!(parse_size("nope"), None);
    }

    #[test]
    fn detail_page_parses() {
        let p = page_payload(sample_detail());
        assert_eq!(p.name.as_deref(), Some("1982 Hirochi RUSH Demo"));
        assert_eq!(p.author.as_deref(), Some("LJ74"));
        assert_eq!(p.full_description.as_deref(), Some("Great car demo."));
        assert_eq!(
            p.screenshots,
            vec!["https://www.beamng.com/data/attachments/1058/1058038.jpg"]
        );
        assert_eq!(p.size_bytes, Some(200_907_162));
        assert_eq!(
            p.download_url.as_deref(),
            Some("https://www.beamng.com/resources/1982-hirochi-rush-demo.28903/download?version=72681")
        );
    }

    #[test]
    fn published_from_page_takes_latest_update_date() {
        assert_eq!(
            published_from_page(sample_detail()).as_deref(),
            Some("Sep 14, 2026")
        );
        assert_eq!(published_from_page("<html></html>"), None);
    }

    #[test]
    fn absolute_handles_all_forms() {
        assert_eq!(
            absolute(BASE, "resources/x.1/"),
            "https://www.beamng.com/resources/x.1/"
        );
        assert_eq!(
            absolute(BASE, "//cdn.example/x.png"),
            "https://cdn.example/x.png"
        );
        assert_eq!(absolute(BASE, "https://x.y/z"), "https://x.y/z");
    }

    #[tokio::test]
    #[ignore]
    async fn network_listing_detail_and_resolve() {
        let client = crate::http::build_client().expect("http client");
        let list = search(&client, None, Some("vehicles"), 1, None)
            .await
            .expect("listing");
        assert!(!list.items.is_empty(), "официальный листинг пуст");
        assert!(list.total_pages >= 1);

        // сайт может отдавать пустые страницы — достаточно, чтобы хоть один
        // элемент из первых шести довёл цепочку до прямой ссылки.
        let mut resolved = 0usize;
        for item in list.items.iter().take(6) {
            assert!(item.id.starts_with("beamngweb:"), "id: {}", item.id);
            match detail(&client, &item.id, &item.key).await {
                Ok(d) => assert!(!d.item.name.is_empty()),
                Err(_) => continue,
            }
            if let Ok((url, filename, _)) = resolve_download(&client, &item.key).await {
                assert!(url.contains("download?version="), "url: {url}");
                assert!(filename.ends_with(".zip"), "filename: {filename}");
                resolved += 1;
            }
        }
        assert!(resolved >= 1, "ни один мод не довёл до прямой ссылки");
    }
}

use crate::http;
use crate::models::{ModDetail, ModItem, ModSearchResult, SourceCategory};
use crate::sources::SourceError;
use anyhow::Result;
use scraper::{Html, Selector};

pub const BASE: &str = "https://www.worldofmods.com";

const CATEGORY_PATHS: &[(&str, &str)] = &[
    ("all", "mods"),
    ("cars", "cars"),
    ("maps", "maps"),
    ("bikes", "bikes"),
    ("planes", "planes"),
];

pub fn categories() -> Vec<SourceCategory> {
    CATEGORY_PATHS
        .iter()
        .map(|(id, label)| {
            let label = match *label {
                "mods" => "Все моды".to_string(),
                "cars" => "Авто".to_string(),
                "maps" => "Карты".to_string(),
                "bikes" => "Мото".to_string(),
                "planes" => "Авиа".to_string(),
                other => other.to_string(),
            };
            SourceCategory {
                id: id.to_string(),
                label,
            }
        })
        .collect()
}

fn category_path(category: &str) -> &str {
    CATEGORY_PATHS
        .iter()
        .find(|(id, _)| *id == category)
        .map(|(_, p)| *p)
        .unwrap_or("mods")
}

fn listing_url(category: &str, page: u32) -> String {
    let path = category_path(category);
    if page <= 1 {
        format!("{BASE}/beamng/{path}/")
    } else {
        format!("{BASE}/beamng/{path}/page:{page}/")
    }
}

fn mod_id_from_url(href: &str) -> Option<String> {
    let stem = href.rsplit('/').next()?;
    let id = stem
        .split('-')
        .next()
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))?
        .to_string();
    Some(id)
}

fn h3_text(article: &scraper::element_ref::ElementRef) -> Option<String> {
    let sel_a = Selector::parse("header h3 a").ok()?;
    let text = article
        .select(&sel_a)
        .next()?
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    (text.len() >= 2).then_some(text)
}

fn first_img_src(article: &scraper::element_ref::ElementRef) -> Option<String> {
    let sel_img = Selector::parse("img").ok()?;
    article
        .select(&sel_img)
        .filter_map(|img| img.value().attr("src"))
        .find(|src| !src.contains("loading-wide"))
        .map(ToOwned::to_owned)
}

fn counters(article: &scraper::element_ref::ElementRef) -> Vec<String> {
    let sel_span = Selector::parse(".counter span").unwrap();
    article
        .select(&sel_span)
        .map(|s| s.text().collect::<Vec<_>>().join("").trim().to_string())
        .collect()
}

/// Парсит страницу категории в список модов.
fn parse_listing(
    html: &str,
    source: &str,
    category: Option<&str>,
    page: u32,
) -> Result<ModSearchResult, SourceError> {
    let doc = Html::parse_document(html);
    let sel_article = Selector::parse("article.catalog-content")
        .map_err(|e| SourceError::Parse(e.to_string()))?;
    let sel_a = Selector::parse("header h3 a").unwrap();

    let mut items = Vec::new();
    for article in doc.select(&sel_article) {
        let Some(href) = article
            .select(&sel_a)
            .next()
            .and_then(|a| a.value().attr("href"))
        else {
            continue;
        };
        let Some(id) = mod_id_from_url(href) else {
            continue;
        };
        let Some(name) = h3_text(&article) else {
            continue;
        };
        let full_url = if href.starts_with('/') {
            format!("{BASE}{href}")
        } else {
            href.to_string()
        };
        let counters = counters(&article);
        let downloads = counters.get(1).cloned();
        let author = article
            .select(&Selector::parse(".author a").unwrap())
            .next()
            .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string());
        let published = article
            .select(&Selector::parse("time").unwrap())
            .next()
            .and_then(|t| t.value().attr("datetime"))
            .map(ToOwned::to_owned);

        items.push(ModItem {
            id: format!("{source}:{id}"),
            source: source.to_string(),
            name,
            thumbnail: first_img_src(&article),
            description: None,
            key: full_url,
            category: Some(category_path(category.unwrap_or("all")).to_string()),
            author,
            published,
            downloads,
            size_bytes: None,
        });
    }

    // Пагинация из data-total на .paginator
    let total_pages = Selector::parse(".paginator")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
        .and_then(|n| n.value().attr("data-total"))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);

    Ok(ModSearchResult {
        items,
        total_pages,
        current_page: page,
    })
}

pub async fn search(
    client: &reqwest::Client,
    _query: Option<&str>,
    category: Option<&str>,
    page: u32,
    _order: Option<&str>,
) -> Result<ModSearchResult, SourceError> {
    let url = listing_url(category.unwrap_or("all"), page);
    let html = http::fetch_string(client, &url, Some(&format!("{BASE}/"))).await?;
    let result = parse_listing(&html, "worldofmods", category, page)?;
    if result.items.is_empty() && page > 1 {
        // страница могла перескочить в одну из зеркальных зон — пробуем базовую
        let url = listing_url(category.unwrap_or("all"), 1);
        let html = http::fetch_string(client, &url, None).await?;
        return parse_listing(&html, "worldofmods", category, 1);
    }
    Ok(result)
}

fn og_content(doc: &Html, property: &str) -> Option<String> {
    let sel = Selector::parse(&format!("meta[property='{property}']")).ok()?;
    doc.select(&sel)
        .next()
        .and_then(|m| m.value().attr("content"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Дата публикации мода на WorldOfMods (`<time datetime=...>` на странице мода).
/// Сайт не выставляет отдельную дату последнего обновления, поэтому эта дата
/// стабильна по времени — обновления на этом источнике не детектируются.
fn published_from_page(doc: &Html) -> Option<String> {
    Selector::parse("time[datetime]")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
        .and_then(|t| t.value().attr("datetime"))
        .map(ToOwned::to_owned)
}

pub async fn detail(
    client: &reqwest::Client,
    mod_id: &str,
    key: &str,
) -> Result<ModDetail, SourceError> {
    let html = http::fetch_string(client, key, Some(&format!("{BASE}/"))).await?;
    let doc = Html::parse_document(&html);

    let name = og_content(&doc, "og:title").unwrap_or_else(|| mod_id.to_string());
    let thumbnail = og_content(&doc, "og:image");
    let short = og_content(&doc, "og:description");
    let published = published_from_page(&doc);

    let full_description = Selector::parse(".article-body")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
        .map(|node| {
            node.text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|t| !t.is_empty());

    // Скриншоты: картинки из body
    let mut screenshots = Vec::new();
    if let Some(body) = Selector::parse(".article-body")
        .ok()
        .and_then(|sel| doc.select(&sel).next())
    {
        let sel_img = Selector::parse("img").unwrap();
        for img in body.select(&sel_img) {
            if let Some(src) = img.value().attr("src") {
                if src.contains("screenshots") || src.contains("cs") {
                    let url = if src.starts_with("//") {
                        format!("https:{src}")
                    } else {
                        src.to_string()
                    };
                    if !screenshots.contains(&url) {
                        screenshots.push(url);
                    }
                }
            }
        }
    }

    Ok(ModDetail {
        item: ModItem {
            id: format!("worldofmods:{mod_id}"),
            source: "worldofmods".to_string(),
            name,
            thumbnail,
            description: short,
            key: key.to_string(),
            category: None,
            author: None,
            published,
            downloads: None,
            size_bytes: None,
        },
        full_description,
        screenshots,
    })
}

/// Из страницы мода достаём ссылку `/get-manual/...` и через `?ajax=true`
/// получаем настоящий URL прямого скачивания. Возвращаем (url, filename).
pub async fn resolve_download(
    client: &reqwest::Client,
    page_url: &str,
) -> Result<(String, String, Option<String>), SourceError> {
    let html = http::fetch_string(client, page_url, Some(&format!("{BASE}/"))).await?;
    let published = {
        let doc = Html::parse_document(&html);
        published_from_page(&doc)
    };
    let manual_href = manual_download_href(&html).ok_or_else(|| {
        SourceError::Parse("не найдена ссылка на скачивание с WorldOfMods".into())
    })?;

    // Запрос, который возвращает актуальный файл
    let manual_href = if manual_href.contains('?') {
        format!("{manual_href}&ajax=true")
    } else {
        format!("{manual_href}?ajax=true")
    };
    let ajax_url = if manual_href.starts_with('/') {
        format!("{BASE}{manual_href}")
    } else {
        manual_href
    };

    let ajax = http::fetch_string(client, &ajax_url, Some(page_url)).await?;
    let sel = Selector::parse("#download-button").map_err(|e| SourceError::Parse(e.to_string()))?;
    let doc = Html::parse_document(&ajax);
    let Some(final_url) = doc.select(&sel).next().and_then(|a| a.value().attr("href")) else {
        return Err(SourceError::Parse(
            "WorldOfMods не отдал прямую ссылку (возможно, нужен аккаунт)".into(),
        ));
    };

    let final_url = if final_url.starts_with("//") {
        format!("https:{final_url}")
    } else {
        final_url.to_string()
    };

    let filename = page_url
        .rsplit('/')
        .next()
        .unwrap_or("mod")
        .trim_end_matches(".html")
        .to_string();
    let filename = http::sanitize_filename(&filename) + ".zip";

    Ok((final_url, filename, published))
}

fn manual_download_href(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a").ok()?;
    doc.select(&sel)
        .filter_map(|a| a.value().attr("href"))
        .find(|href| href.contains("/get-manual/"))
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_listing_html() -> &'static str {
        r#"<html><body>
        <article class="catalog-content">
          <header><h3><a href="/beamng/mods/14735-hirochi-super-race-v105.html">Hirochi Super Race v1.05</a></h3></header>
          <img src="https://cdn.example.com/img1.jpg">
          <div class="counter"><span>1 234</span></div>
          <div class="counter"><span>56 789</span></div>
          <div class="author"><a href="/user/foo">SomeAuthor</a></div>
          <time datetime="2024-01-02 10:00"></time>
        </article>
        <article class="catalog-content">
          <header><h3><a href="/beamng/mods/23567-balances-suel.html">Balances Suel</a></h3></header>
          <img src="/assets/loading-wide.gif">
          <img src="https://cdn.example.com/img2.jpg">
        </article>
        <div class="paginator" data-total="7"></div>
        </body></html>"#
    }

    #[test]
    fn parse_listing_extracts_items_and_pagination() {
        let res =
            parse_listing(sample_listing_html(), "worldofmods", Some("all"), 1).expect("parse ok");
        assert_eq!(res.items.len(), 2);
        assert_eq!(res.total_pages, 7);
        assert_eq!(res.current_page, 1);

        let first = &res.items[0];
        assert_eq!(first.id, "worldofmods:14735");
        assert_eq!(first.name, "Hirochi Super Race v1.05");
        assert!(first
            .key
            .ends_with("/beamng/mods/14735-hirochi-super-race-v105.html"));
        assert_eq!(
            first.thumbnail.as_deref(),
            Some("https://cdn.example.com/img1.jpg")
        );
        assert_eq!(first.category.as_deref(), Some("mods"));
        assert_eq!(first.author.as_deref(), Some("SomeAuthor"));
        assert_eq!(first.published.as_deref(), Some("2024-01-02 10:00"));
        assert_eq!(first.downloads.as_deref(), Some("56 789"));

        let second = &res.items[1];
        assert_eq!(second.id, "worldofmods:23567");
        assert_eq!(
            second.thumbnail.as_deref(),
            Some("https://cdn.example.com/img2.jpg")
        );
        assert_eq!(second.downloads, None);
    }

    #[test]
    fn mod_id_from_url_extracts_leading_digids() {
        assert_eq!(
            mod_id_from_url("/beamng/mods/14735-hirochi-super-race-v105.html"),
            Some("14735".to_string())
        );
        assert_eq!(mod_id_from_url("/beamng/mods/"), None);
        assert_eq!(mod_id_from_url("no-digits-here.html"), None);
    }

    #[test]
    fn listing_url_handles_first_and_other_pages() {
        assert_eq!(
            listing_url("mods", 1),
            "https://www.worldofmods.com/beamng/mods/"
        );
        assert_eq!(
            listing_url("cars", 2),
            "https://www.worldofmods.com/beamng/cars/page:2/"
        );
        assert_eq!(
            listing_url("planes", 0),
            "https://www.worldofmods.com/beamng/planes/"
        );
    }

    #[test]
    fn unknown_category_falls_back_to_mods() {
        assert_eq!(category_path("nope"), "mods");
        assert_eq!(category_path("maps"), "maps");
    }

    #[test]
    fn categories_list_is_stable() {
        let cats = categories();
        assert_eq!(cats.len(), 5);
        assert_eq!(cats[0].id, "all");
        assert_eq!(cats[0].label, "Все моды");
        assert!(cats.iter().any(|c| c.id == "planes"));
    }

    /// Сквозная проверка живой сети: листинг -> деталь -> прямая ссылка.
    #[tokio::test]
    #[ignore]
    async fn network_listing_detail_and_resolve() {
        let client = crate::http::build_client().expect("http client");
        let list = search(&client, None, Some("cars"), 1, None)
            .await
            .expect("listing");
        assert!(!list.items.is_empty(), "листинг WorldOfMods пуст");
        assert!(list.total_pages >= 1);

        let item = &list.items[0];
        assert!(item.id.starts_with("worldofmods:"), "id: {}", item.id);

        let d = detail(&client, &item.id, &item.key).await.expect("detail");
        assert!(!d.item.name.is_empty());

        let (url, filename, _) = resolve_download(&client, &item.key).await.expect("resolve");
        assert!(url.starts_with("https://"));
        assert!(filename.ends_with(".zip"));
    }

    #[test]
    fn published_from_page_reads_time_datetime() {
        let doc = Html::parse_document(
            r#"<div class="col-sm-6 text-right">
                 <i class="glyphicon glyphicon-time" title="Published"></i>&nbsp;
                 <time datetime="2016-04-06T04:08:50-04:00" title="06.04.2016 04:08:50">06.04.2016 04:08:50</time>
               </div>"#,
        );
        assert_eq!(
            published_from_page(&doc).as_deref(),
            Some("2016-04-06T04:08:50-04:00")
        );
        assert_eq!(
            published_from_page(&Html::parse_document("<html></html>")),
            None
        );
    }
}

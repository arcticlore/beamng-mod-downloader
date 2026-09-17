use anyhow::Result;
use std::time::Duration;

pub const DEFAULT_UA: &str = concat!(
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/126.0 Safari/537.36 BeamNGModDownloader/",
    env!("CARGO_PKG_VERSION")
);

pub fn build_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(DEFAULT_UA)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()?)
}

pub async fn fetch_bytes(
    client: &reqwest::Client,
    url: &str,
    referer: Option<&str>,
) -> Result<Vec<u8>> {
    let mut req = client.get(url);
    if let Some(r) = referer {
        req = req.header(reqwest::header::REFERER, r);
    }
    let response = req.send().await?.error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}

pub async fn fetch_string(
    client: &reqwest::Client,
    url: &str,
    referer: Option<&str>,
) -> Result<String> {
    let bytes = fetch_bytes(client, url, referer).await?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Санитизирует имя файла: убирает `..`, разделители и пробелы превращает в `_`.
pub fn sanitize_filename(raw: &str) -> String {
    let mut cleaned: String = raw
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            ' ' => '_',
            c => c,
        })
        .collect::<String>()
        .trim_end_matches('.')
        .to_string();
    if cleaned.is_empty() {
        cleaned = "mod".to_string();
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::sanitize_filename;

    #[test]
    fn sanitize_replaces_path_chars_and_spaces() {
        assert_eq!(sanitize_filename("my mod v1.0"), "my_mod_v1.0");
        assert_eq!(
            sanitize_filename("a/b\\c:d*e?f\"g<h>i|j"),
            "a_b_c_d_e_f_g_h_i_j"
        );
        assert_eq!(sanitize_filename(".."), "mod");
        assert_eq!(sanitize_filename("   "), "mod");
        assert_eq!(sanitize_filename("trailing.dot."), "trailing.dot");
        assert_eq!(sanitize_filename("normal.zip"), "normal.zip");
    }
}

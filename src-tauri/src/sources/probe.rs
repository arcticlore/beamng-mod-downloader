//! Лёгкий HTTP-probe архива: узнаёт имя файла и дату Last-Modified по
//! заголовкам (HEAD, с фолбэком на GET + Range), не скачивая тело.
//!
//! Используется вложениями форума (`beamngforum`) и источником «Прямая
//! ссылка» (`directurl`). Информация из заголовков достаточно, чтобы понять
//! итоговое имя файла и «версию» (дату) для ledger до фактической загрузки.

use anyhow::{anyhow, Result};
use reqwest::StatusCode;

/// Данные, читаемые из заголовков ответа без затрагивания тела.
#[derive(Debug, Clone, Default)]
pub struct ProbeResult {
    /// Имя файла из `Content-Disposition` (уже санитизированное), если есть.
    pub filename: Option<String>,
    /// `Last-Modified` (HTTP-date как есть) — для записи в ledger/PD «версии».
    pub published: Option<String>,
    /// `Content-Length`, если сервер его отдал.
    pub size: Option<u64>,
}

/// HEAD-запрос; для серверов без HEAD (405/501) — GET с `Range: bytes=0-0`.
/// Тело не читается: если сервер проигнорировал Range, соединение просто
/// закрывается (это одноразовый probe, а не скачивание).
pub async fn probe(client: &reqwest::Client, url: &str) -> Result<ProbeResult> {
    let head = client.get(url).send().await?;
    let status = head.status();
    if status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT {
        return Ok(from_response(&head));
    }
    drop(head);

    let ranged = client
        .get(url)
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()
        .await?;
    let status = ranged.status();
    if status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT {
        return Ok(from_response(&ranged));
    }
    Err(anyhow!(crate::i18n::tf(
        "сервер ответил HTTP {0} на запрос заголовков",
        "the server answered HTTP {0} to the headers request",
        &[&status.to_string()],
    )))
}

fn from_response(resp: &reqwest::Response) -> ProbeResult {
    let filename = resp
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_disposition_filename)
        .map(|raw| crate::http::sanitize_filename(&raw));
    let published = resp
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);
    ProbeResult {
        filename,
        published,
        size: resp.content_length(),
    }
}

/// Имя файла из `Content-Disposition`: поддержаны обычный `filename="..."` и
/// RFC 5987 `filename*=UTF-8''...` (percent-decoded). По RFC 6266 при
/// наличии обоих приоритет у `filename*`.
pub fn parse_content_disposition_filename(value: &str) -> Option<String> {
    if let Some(pos) = value.find("filename*=") {
        let tail = &value[pos + "filename*=".len()..];
        let val = tail.split(';').next().unwrap_or("").trim();
        let encoded = val
            .strip_prefix("UTF-8''")
            .or_else(|| val.strip_prefix("utf-8''"));
        if let Some(enc) = encoded {
            let decoded = percent_decode(enc);
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }
    }
    let pos = value.find("filename=")?;
    let tail = &value[pos + "filename=".len()..];
    let val = tail
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"');
    let val = val.trim();
    if val.is_empty() {
        None
    } else {
        Some(val.to_string())
    }
}

pub(crate) fn percent_decode(s: &str) -> String {
    fn hexd(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hexd(bytes[i + 1]), hexd(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_extended_filenames() {
        assert_eq!(
            parse_content_disposition_filename("attachment; filename=\"mod.zip\""),
            Some("mod.zip".to_string())
        );
        assert_eq!(
            parse_content_disposition_filename("attachment; filename=mod.zip"),
            Some("mod.zip".to_string())
        );
        assert_eq!(parse_content_disposition_filename("inline"), None);
        assert_eq!(
            parse_content_disposition_filename(
                "attachment; filename*=UTF-8''%D0%9C%D0%BE%D0%B4%201.zip"
            ),
            Some("Мод 1.zip".to_string())
        );
        // RFC 6266: filename* имеет приоритет над filename.
        assert_eq!(
            parse_content_disposition_filename(
                "attachment; filename=\"old.zip\"; filename*=UTF-8''new.zip"
            ),
            Some("new.zip".to_string())
        );
    }

    #[test]
    fn percent_decode_handles_utf8_and_plain() {
        assert_eq!(percent_decode("hello"), "hello");
        assert_eq!(percent_decode("a%20b"), "a b");
        // Кириллица "Мод" в URL-форме.
        assert_eq!(percent_decode("%D0%9C%D0%BE%D0%B4"), "Мод");
        // Невалидный проц-эскейп оставляем как есть.
        assert_eq!(percent_decode("%zz"), "%zz");
    }
}

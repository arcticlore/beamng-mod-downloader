//! SSRF-защита исходящих HTTP-запросов (PR runtime-security).
//!
//! Политика:
//! - допустимы только схемы http/https;
//! - явно указанный порт — только 80/443;
//! - host не может быть IP-литералом (никаких 127.0.0.1, ::1, RFC1918, link-local,
//!   metadata-адресов) — разрешены только доменные имена;
//! - userinfo (user:pass@) запрещён;
//! - домен обязан входить в allowlist по registrable-domain соответствию
//!   (запись `beamng.com` покрывает и `beamng.com`, и `www.beamng.com`).
//!
//! Гейт применяется в трёх местах:
//! - `http::fetch_bytes` / `http::fetch_string` — страницы и API источников;
//! - `sources::github::gh_get` — прямой вызов GitHub API;
//! - `download::run_download` — непосредственное скачивание архива.
//!
//! Плюс custom redirect-политика в `http::build_client`: каждый редирект-хоп
//! повторно валидируется (beamng.com и github.com отдают контент через
//! редиректы с других доменов — поэтому РАЗРЕШЕНЫ r2.dev,
//! r2.cloudflarestorage.com и githubusercontent.com).
//!
//! Ограничение: против DNS-ребinding на уровне hostname мы не делаем собственный
//! DNS-резолв (это async); literal-IP заблокированы напрямую, что покрывает
//! классические SSRF-векторы без потери функциональности.

use reqwest::Url;

/// Registrable-домены, с которых приложение может получать данные.
const ALLOWED_HOST_SUFFIXES: &[&str] = &[
    "beamng.com",
    "github.com",
    "githubusercontent.com", // avatars, release-assets, objects
    "worldofmods.com",
    "r2.dev",                   // Cloudflare R2 (beamng.com редиректы)
    "r2.cloudflarestorage.com", // Cloudflare R2 storage endpoint
];

/// Максимальное число редирект-хопов.
pub const MAX_REDIRECTS: usize = 8;

/// Возвращает ошибку, если URL нарушает транспортную политику.
pub fn validate_url(url: &str) -> Result<(), String> {
    let parsed = Url::parse(url).map_err(|e| format!("невалидный URL `{url}`: {e}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(format!("схема `{other}://` запрещена (только http/https)")),
    }

    if parsed.username() != "" || parsed.password().is_some() {
        return Err("userinfo в URL запрещён".to_string());
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| format!("URL без host: `{url}`"))?;

    if host.parse::<std::net::IpAddr>().is_ok() {
        return Err(format!("host `{host}` — IP-литерал, запрещено"));
    }

    if host.parse::<u32>().is_ok() {
        return Err(format!("host `{host}` похож на IP-индекс, запрещено"));
    }

    if !host.contains('.') {
        return Err(format!("host `{host}` — не доменное имя"));
    }

    if let Some(port) = parsed.port() {
        if port != 80 && port != 443 {
            return Err(format!("порт {port} запрещён (только 80/443)"));
        }
    }

    if !ALLOWED_HOST_SUFFIXES
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
    {
        return Err(format!(
            "host `{host}` не входит в разрешённые домены ({})",
            ALLOWED_HOST_SUFFIXES.join(", ")
        ));
    }

    Ok(())
}

/// Custom redirect-политика reqwest: валидирует каждый хоп.
pub fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        let hop = 1 + attempt.previous().len();
        if hop > MAX_REDIRECTS {
            return attempt.stop();
        }
        if validate_url(attempt.url().as_str()).is_err() {
            return attempt.stop();
        }
        attempt.follow()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(url: &str) {
        assert!(validate_url(url).is_ok(), "ожидался разрешённый URL: {url}");
    }
    fn err(url: &str) {
        assert!(
            validate_url(url).is_err(),
            "ожидался блокируемый URL: {url}"
        );
    }

    #[test]
    fn allows_expected_hosts() {
        ok("https://www.beamng.com/download/123");
        ok("https://beamng.com/repo/abc.zip");
        ok("https://api.github.com/repos/arcticlore/beamng-mod-downloader");
        ok("https://github.com/arcticlore/beamng-mod-downloader/releases/download/v1/x.zip");
        ok("https://codeload.github.com/arcticlore/beamng-mod-downloader/zip/refs/heads/main");
        ok("https://www.worldofmods.com/mods/1/");
        ok("http://www.beamng.com/legacy-link");
    }

    #[test]
    fn blocks_schemes() {
        err("file:///etc/passwd");
        err("file://localhost/etc/passwd");
        err("gopher://127.0.0.1:70/");
        err("tcp://127.0.0.1:22/");
        err("ftp://www.beamng.com/x");
        err("javascript:alert(1)");
        err("data:text/plain;base64,SGVsbG8=");
    }

    #[test]
    fn blocks_ip_literals_and_localhost() {
        err("http://127.0.0.1/");
        err("http://localhost/");
        err("http://0.0.0.0/");
        err("http://169.254.169.254/latest/meta-data/");
        err("http://10.0.0.1/");
        err("http://192.168.1.1/");
        err("http://[::1]/");
        err("http://[::ffff:127.0.0.1]/");
    }

    #[test]
    fn blocks_foreign_hosts_and_ports() {
        err("https://1.1.1.1/cloudflare");
        err("https://evil.com/");
        err("https://beamng.com.evil.com/");
        err("https://www.beamng.com:4443/x");
        err("https://github.com:8443/x");
        err("https://user:pass@github.com/x");
        err("https://ユーザ@beamng.com/"); // userinfo
    }

    #[test]
    fn redirect_policy_is_bounded() {
        assert_eq!(MAX_REDIRECTS, 8);
        let _ = redirect_policy();
        // validate_url остаётся единым гейтом и для custom redirect-политики:
        assert!(validate_url("https://www.beamng.com/x").is_ok());
        assert!(validate_url("https://evil.com/").is_err());
    }

    #[test]
    fn allows_cloudflare_r2_redirect_targets() {
        ok("https://pub-1234567890abcdef1234.r2.dev/beamng/abc.zip");
        ok("https://release-assets.githubusercontent.com/arcticlore/x");
        ok("https://objects.githubusercontent.com/arcticlore/archive");
    }
}

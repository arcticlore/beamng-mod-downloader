//! SSRF-защита исходящих HTTP-запросов (PR runtime-security).
//!
//! Политика:
//! - допустимы только схемы http/https;
//! - явно указанный порт — только 80/443;
//! - host не может быть IP-литералом (никаких 127.0.0.1, ::1, RFC1918, link-local,
//!   metadata-адресов) — разрешены только доменные имена;
//! - userinfo (user:pass@) запрещён;
//! - домен обязан входить в allowlist: исторические источники — по
//!   registrable-domain соответствию (запись `beamng.com` покрывает и
//!   `beamng.com`, и `www.beamng.com`); GitLab/Codeberg — строго exact-host
//!   (только `gitlab.com` / `codeberg.org`, поддомены не разрешены).
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
//! Отдельные source-scoped гейты (`validate_gitlab_url`/`validate_codeberg_url`)
//! применяются адаптерами источников к release-ассетам ДО скачивания: внешний
//! URL из attacker-controlled release-метаданных не пройдёт, даже если хост
//! разрешён историческим суффиксом (например `github.com`).
//!
//! Ограничение: против DNS-ребinding на уровне hostname мы не делаем собственный
//! DNS-резолв (это async); literal-IP заблокированы напрямую, что покрывает
//! классические SSRF-векторы без потери функциональности.

use reqwest::Url;

/// Registrable-домены, с которых приложение может получать данные.
/// Суффиксное соответствие покрывает и сам домен, и его поддомены.
/// GitLab/Codeberg в этот список не входят — для них строгий exact-host
/// allowlist (см. `ALLOWED_EXACT_HOSTS`), чтобы не открывать произвольные
/// subdomain'ы вида `*.gitlab.com` / `*.codeberg.org`.
const ALLOWED_HOST_SUFFIXES: &[&str] = &[
    "beamng.com",
    "github.com",
    "githubusercontent.com", // avatars, release-assets, objects
    "worldofmods.com",
    "r2.dev",                   // Cloudflare R2 (beamng.com редиректы)
    "r2.cloudflarestorage.com", // Cloudflare R2 storage endpoint
];

/// Точные хосты форджей, разрешённые ГЛОБАЛЬНО (API, веб, релизы и
/// редирект-хопы). Поддомены сюда не входят: только сам exact host.
const ALLOWED_EXACT_HOSTS: &[&str] = &["gitlab.com", "codeberg.org"];

/// Exact-host набор для first-party релизов GitLab.
const GITLAB_EXACT_HOSTS: &[&str] = &["gitlab.com"];

/// Exact-host набор для first-party релизов Codeberg.
const CODEBERG_EXACT_HOSTS: &[&str] = &["codeberg.org"];

/// Максимальное число редирект-хопов.
pub const MAX_REDIRECTS: usize = 8;

/// Возвращает ошибку, если URL нарушает транспортную политику allowlist-источников.
pub fn validate_url(url: &str) -> Result<(), String> {
    let parsed = Url::parse(url).map_err(|e| {
        crate::i18n::tf(
            "невалидный URL `{0}`: {1}",
            "invalid URL `{0}`: {1}",
            &[url, &e.to_string()],
        )
    })?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(crate::i18n::tf(
                "схема `{0}://` запрещена (только http/https)",
                "scheme `{0}://` is forbidden (http/https only)",
                &[other],
            ))
        }
    }

    if parsed.username() != "" || parsed.password().is_some() {
        return Err(
            crate::i18n::t("userinfo в URL запрещён", "userinfo in a URL is forbidden").to_string(),
        );
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| crate::i18n::tf("URL без host: `{0}`", "URL has no host: `{0}`", &[url]))?;

    if host.parse::<std::net::IpAddr>().is_ok() {
        return Err(crate::i18n::tf(
            "host `{0}` — IP-литерал, запрещено",
            "host `{0}` is an IP literal, forbidden",
            &[host],
        ));
    }

    if host.parse::<u32>().is_ok() {
        return Err(crate::i18n::tf(
            "host `{0}` похож на IP-индекс, запрещено",
            "host `{0}` looks like an IP index, forbidden",
            &[host],
        ));
    }

    if !host.contains('.') {
        return Err(crate::i18n::tf(
            "host `{0}` — не доменное имя",
            "host `{0}` is not a domain name",
            &[host],
        ));
    }

    if let Some(port) = parsed.port() {
        if port != 80 && port != 443 {
            return Err(crate::i18n::tf(
                "порт {0} запрещён (только 80/443)",
                "port {0} is forbidden (80/443 only)",
                &[&port.to_string()],
            ));
        }
    }

    let suffix_ok = ALLOWED_HOST_SUFFIXES
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")));
    let exact_ok = ALLOWED_EXACT_HOSTS
        .iter()
        .any(|exact| host.eq_ignore_ascii_case(exact));
    if !suffix_ok && !exact_ok {
        return Err(crate::i18n::tf(
            "host `{0}` не входит в разрешённые домены ({1}; exact: {2})",
            "host `{0}` is not in the allowed domains ({1}; exact: {2})",
            &[
                host,
                &ALLOWED_HOST_SUFFIXES.join(", "),
                &ALLOWED_EXACT_HOSTS.join(", "),
            ],
        ));
    }
    Ok(())
}

/// Source-scoped гейт для release-ассетов источников: транспортная политика
/// (`validate_url`-уровень) плюс host обязан точно совпасть с одним из
/// заданных exact-host'ов источника. Используется адаптерами источников ДО
/// скачивания архива, чтобы attacker-controlled внешний URL из метаданных
/// релиза не прошёл (даже если его хост разрешён историческим суффиксом).
pub fn validate_source_url(url: &str, allowed_exact_hosts: &[&str]) -> Result<(), String> {
    validate_url(url)?;
    let parsed = Url::parse(url).map_err(|e| {
        crate::i18n::tf(
            "невалидный URL `{0}`: {1}",
            "invalid URL `{0}`: {1}",
            &[url, &e.to_string()],
        )
    })?;
    let host = parsed
        .host_str()
        .ok_or_else(|| crate::i18n::tf("URL без host: `{0}`", "URL has no host: `{0}`", &[url]))?;
    if !allowed_exact_hosts
        .iter()
        .any(|exact| host.eq_ignore_ascii_case(exact))
    {
        return Err(crate::i18n::tf(
            "host `{0}` не входит в exact-host allowlist источника ({1})",
            "host `{0}` is not in the source exact-host allowlist ({1})",
            &[host, &allowed_exact_hosts.join(", ")],
        ));
    }
    Ok(())
}

/// Source-scoped гейт для first-party релизов GitLab.
pub fn validate_gitlab_url(url: &str) -> Result<(), String> {
    validate_source_url(url, GITLAB_EXACT_HOSTS)
}

/// Source-scoped гейт для first-party релизов Codeberg.
pub fn validate_codeberg_url(url: &str) -> Result<(), String> {
    validate_source_url(url, CODEBERG_EXACT_HOSTS)
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
    fn gok(url: &str) {
        assert!(
            validate_gitlab_url(url).is_ok(),
            "ожидался разрешённый GitLab URL: {url}"
        );
    }
    fn gerr(url: &str) {
        assert!(
            validate_gitlab_url(url).is_err(),
            "ожидался блокируемый GitLab URL: {url}"
        );
    }
    fn cok(url: &str) {
        assert!(
            validate_codeberg_url(url).is_ok(),
            "ожидался разрешённый Codeberg URL: {url}"
        );
    }
    fn cerr(url: &str) {
        assert!(
            validate_codeberg_url(url).is_err(),
            "ожидался блокируемый Codeberg URL: {url}"
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
    fn allows_gitlab_and_codeberg_exact_hosts() {
        ok("https://gitlab.com/api/v4/projects?topic=beamng");
        ok("https://gitlab.com/owner/repo/-/releases/1.0/mod.zip");
        ok("https://codeberg.org/api/v1/repos/search?q=beamng");
        ok("https://codeberg.org/owner/repo/releases/download/1.0/mod.zip");
    }

    #[test]
    fn blocks_gitlab_and_codeberg_lookalike_hosts() {
        err("https://gitlab.com.evil.example/api/v4/projects");
        err("https://codeberg.org.evil.example/api/v1/repos/search");
        err("https://evilgitlab.com/api/v4/projects");
        err("https://evilcodeberg.org/api/v1/repos/search");
        err("https://gitlab.com@evil.example/x");
        err("https://codeberg.org@evil.example/x");
        // exact-host: поддомены форджей не разрешены.
        err("https://sub.gitlab.com/x");
        err("https://sub.codeberg.org/x");
        err("https://gitlab.io/x/");
        err("https://pages.gitlab.io/x/y/");
        err("https://codeberg.page/x/");
    }

    #[test]
    fn source_scoped_gates_only_exact_hosts() {
        gok("https://gitlab.com/owner/repo/-/releases/1.0/mod.zip");
        gerr("https://codeberg.org/o/r/releases/download/v1/mod.zip");
        gerr("https://github.com/o/r/releases/download/v1/mod.zip");
        gerr("https://evilgitlab.com/mod.zip");
        gerr("https://sub.gitlab.com/x");
        cok("https://codeberg.org/owner/repo/releases/download/1.0/mod.zip");
        cerr("https://gitlab.com/o/r/-/releases/1.0/mod.zip");
        cerr("https://evilcodeberg.org/mod.zip");
        cerr("https://sub.codeberg.org/x");
        // Нарушения транспортной политики пробрасываются через source-scoped гейт.
        gerr("https://gitlab.com:8443/x");
        gerr("https://user:pass@gitlab.com/x");
        cerr("https://codeberg.org:4443/x");
        cerr("https://user:pass@codeberg.org/x");
    }

    #[test]
    fn blocks_forbidden_schemes_and_host_shapes_in_source_gate() {
        gerr("file:///etc/passwd");
        gerr("gopher://gitlab.com/");
        gerr("http://127.0.0.1/");
        cerr("https://169.254.169.254/");
        cerr("https://localhost/");
        gerr("https://2130706433/");
        gerr("https://0x7f000001/");
        cerr("https://[::1]/");
        gerr("https://gitlab.com.evil.example/api/v4/projects");
        cerr("https://codeberg.org.evil.example/api/v1/repos/search");
    }

    #[test]
    fn blocks_redirect_to_unapproved_hosts() {
        // Редирект-хоп наружу за пределы exact-host/allowlist блокируется.
        err("https://evil-cdn.example/mod.zip");
        gerr("https://evil-cdn.example/mod.zip");
        cerr("https://evil-cdn.example/mod.zip");
        // Легитимный first-party хоп внутри exact-host проходит.
        gok("https://gitlab.com/o/r/-/releases/1.0/mod.zip");
        cok("https://codeberg.org/o/r/releases/download/1.0/mod.zip");
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

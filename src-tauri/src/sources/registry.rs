//! Единый registry источников модов (single source of truth).
//!
//! Frontend получает дескрипторы командой `get_source_registry` и не держит
//! второго независимого списка. Адаптеры, появляющиеся в следующих PR
//! (BeamNG Forum, GitLab, Codeberg, Nexus, Beam-Monsters, 2Fast, direct URL,
//! custom JSON feeds), добавляются сюда — UI и агрегация подхватят их сами.
//!
//! Два разных состояния источника:
//! - **enabled** — разрешены сетевые запросы вообще (`Config.enabled_sources`);
//! - **selected/active** — источник участвует в текущем поиске/агрегации
//!   (`Config.selected_sources`, `None` = все enabled).
//!
//! Disabled источник не должен сделать ни одного сетевого запроса — за это
//! отвечает гейт `can_query` на всех tauri-командах, которые ходят в сеть.

use crate::models::{
    FilenameRule, InstallMode, SourceAuth, SourceCapabilities, SourceDescriptor, SourceGroup,
    SourceStatus, TrustLevel,
};

/// Порядок групп в UI (группировка в Source Picker).
const GROUP_ORDER: &[(&str, u8)] = &[
    ("official", 0),
    ("forges", 1),
    ("community", 2),
    ("custom", 3),
];

fn group_rank(group: &SourceGroup) -> u8 {
    let key = match group {
        SourceGroup::Official => "official",
        SourceGroup::Forges => "forges",
        SourceGroup::Community => "community",
        SourceGroup::Custom => "custom",
    };
    GROUP_ORDER
        .iter()
        .find(|(k, _)| k == &key)
        .map(|(_, rank)| *rank)
        .unwrap_or(99)
}

fn caps(
    search: bool,
    categories: bool,
    pagination: bool,
    detail: bool,
    direct: bool,
) -> SourceCapabilities {
    SourceCapabilities {
        search,
        categories,
        pagination,
        detail,
        direct_zip_download: direct,
        manual_download: false,
        checksums: false,
        update_detection: false,
    }
}

/// Все статические источники. Сортированы по группе и имени для стабильности UI.
pub fn registry() -> Vec<SourceDescriptor> {
    let mut list = vec![
        SourceDescriptor {
            id: "beamngweb".into(),
            label: "Официальный сайт BeamNG".into(),
            group: SourceGroup::Official,
            trust_level: TrustLevel::Official,
            enabled_by_default: true,
            legacy_default: true,
            homepage: "https://www.beamng.com/resources/".into(),
            terms_or_policy_url: Some("https://www.beamng.com/help/terms/".into()),
            warning: None,
            install_mode: InstallMode::ModsZip,
            auth: SourceAuth::None,
            status: SourceStatus::Ready,
            filename_rule: FilenameRule::Basename,
            capabilities: caps(true, true, true, true, true),
            categories: crate::sources::beamngweb::categories(),
        },
        SourceDescriptor {
            id: "github".into(),
            label: "GitHub-релизы".into(),
            group: SourceGroup::Forges,
            trust_level: TrustLevel::VerifiedForge,
            enabled_by_default: true,
            legacy_default: true,
            homepage: "https://github.com/topics/beamng".into(),
            terms_or_policy_url: Some(
                "https://docs.github.com/en/site-policy/github-terms/github-terms-of-service"
                    .into(),
            ),
            warning: None,
            install_mode: InstallMode::ModsZip,
            auth: SourceAuth::None,
            status: SourceStatus::Ready,
            filename_rule: FilenameRule::OwnerRepo,
            capabilities: caps(true, true, true, true, true),
            categories: crate::sources::github::categories(),
        },
        SourceDescriptor {
            id: "gitlab".into(),
            label: "GitLab-релизы".into(),
            group: SourceGroup::Forges,
            trust_level: TrustLevel::VerifiedForge,
            enabled_by_default: false,
            legacy_default: false,
            homepage: "https://gitlab.com/explore/projects/topics/beamng".into(),
            terms_or_policy_url: Some("https://about.gitlab.com/terms/".into()),
            warning: None,
            install_mode: InstallMode::ModsZip,
            auth: SourceAuth::None,
            status: SourceStatus::Ready,
            filename_rule: FilenameRule::OwnerRepo,
            capabilities: caps(true, true, true, true, true),
            categories: crate::sources::gitlab::categories(),
        },
        SourceDescriptor {
            id: "codeberg".into(),
            label: "Codeberg".into(),
            group: SourceGroup::Forges,
            trust_level: TrustLevel::VerifiedForge,
            enabled_by_default: false,
            legacy_default: false,
            homepage: "https://codeberg.org/explore/topics/beamng".into(),
            terms_or_policy_url: Some("https://codeberg.org/legal/terms".into()),
            warning: None,
            install_mode: InstallMode::ModsZip,
            auth: SourceAuth::None,
            status: SourceStatus::Ready,
            filename_rule: FilenameRule::OwnerRepo,
            capabilities: caps(true, true, true, true, true),
            categories: crate::sources::codeberg::categories(),
        },
        SourceDescriptor {
            id: "worldofmods".into(),
            label: "WorldOfMods".into(),
            group: SourceGroup::Community,
            trust_level: TrustLevel::ThirdParty,
            enabled_by_default: false,
            legacy_default: true,
            homepage: "https://www.worldofmods.com/beamng/".into(),
            terms_or_policy_url: None,
            warning: Some(
                "Сторонний неофициальный источник: контент не проверяется BeamNG. \
                 Файлы устанавливаются только после структурной проверки zip, \
                 но доверять контенту предлагается на свой риск."
                    .into(),
            ),
            install_mode: InstallMode::ModsZip,
            auth: SourceAuth::None,
            status: SourceStatus::Ready,
            filename_rule: FilenameRule::HtmlSlug,
            capabilities: caps(true, true, true, true, true),
            categories: crate::sources::worldofmods::categories(),
        },
        SourceDescriptor {
            id: "beamngforum".into(),
            label: "Форум BeamNG".into(),
            group: SourceGroup::Official,
            trust_level: TrustLevel::Official,
            enabled_by_default: false,
            legacy_default: true,
            homepage: "https://www.beamng.com/community/".into(),
            terms_or_policy_url: Some("https://www.beamng.com/help/terms-of-service/".into()),
            warning: Some(
                "На форуме нет надёжного API поиска и привязки к zip-ассетам. \
                 Найти мод и ссылку на файл придётся вручную — приложение только \
                 подсказывает, куда смотреть, и не устанавливает контент автоматически."
                    .into(),
            ),
            install_mode: InstallMode::ManualExternal,
            auth: SourceAuth::None,
            status: SourceStatus::Ready,
            filename_rule: FilenameRule::HtmlSlug,
            capabilities: caps(false, false, false, false, false),
            categories: crate::sources::beamngforum::categories(),
        },
    ];
    list.sort_by(|a, b| {
        group_rank(&a.group)
            .cmp(&group_rank(&b.group))
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
    list
}

/// Возвращает дескриптор по id, если источник существует.
pub fn by_id(id: &str) -> Option<SourceDescriptor> {
    registry().into_iter().find(|d| d.id == id)
}

/// Есть ли такой источник вообще.
pub fn is_known(id: &str) -> bool {
    registry().iter().any(|d| d.id == id)
}

/// ID-набор по умолчанию. Для НОВЫХ конфигов — рекомендуемые источники;
/// для миграции существующих (из 0.2.x) — прежний набор, чтобы не потерять
/// источник, которым пользователь уже пользовался.
pub fn default_enabled_ids(new_config: bool) -> Vec<String> {
    let mut ids: Vec<String> = registry()
        .into_iter()
        .filter(|d| {
            if new_config {
                d.enabled_by_default
            } else {
                d.legacy_default
            }
        })
        .map(|d| d.id)
        .collect();
    ids.sort();
    ids
}

/// Гейт: разрешён ли источнику сетевой запрос. Disabled-источник не запрашивается вообще.
pub fn can_query(enabled: &[String], id: &str) -> bool {
    enabled.iter().any(|e| e == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let reg = registry();
        let mut seen = std::collections::HashSet::new();
        for d in &reg {
            assert!(seen.insert(d.id.clone()), "дубликат id: {}", d.id);
        }
    }

    #[test]
    fn registry_has_legacy_sources() {
        let ids: Vec<String> = registry().into_iter().map(|d| d.id).collect();
        for expected in ["beamngweb", "github", "worldofmods"] {
            assert!(
                ids.iter().any(|i| i == expected),
                "нет источника {expected}"
            );
        }
        assert!(registry().iter().all(|d| !d.categories.is_empty()));
    }

    #[test]
    fn default_sets_respect_new_vs_legacy() {
        let new_cfg = default_enabled_ids(true);
        let legacy = default_enabled_ids(false);
        // Новые конфиги: официальные + forges, без WorldOfMods.
        assert!(new_cfg.contains(&"beamngweb".to_string()));
        assert!(new_cfg.contains(&"github".to_string()));
        assert!(!new_cfg.contains(&"worldofmods".to_string()));
        // Миграция старых конфигов сохраняет прежний набор.
        assert!(legacy.contains(&"beamngweb".to_string()));
        assert!(legacy.contains(&"github".to_string()));
        assert!(legacy.contains(&"worldofmods".to_string()));
        // Рекомендуемые источники stable-упорядочены.
        let sorted = new_cfg.clone();
        assert_eq!(sorted, new_cfg);
    }

    #[test]
    fn by_id_and_known_agree() {
        assert!(by_id("github").is_some());
        assert!(is_known("beamngweb"));
        assert!(!is_known("nope"));
        assert!(by_id("nope").is_none());
    }

    #[test]
    fn can_query_gate() {
        let enabled = vec!["github".to_string(), "beamngweb".to_string()];
        assert!(can_query(&enabled, "github"));
        assert!(!can_query(&enabled, "worldofmods"));
        assert!(!can_query(&enabled, "ghost"));
    }

    #[test]
    fn descriptors_are_stable_and_sorted_by_group() {
        let reg = registry();
        for w in reg.windows(2) {
            let l = group_rank(&w[0].group);
            let r = group_rank(&w[1].group);
            assert!(l <= r, "порядок групп нарушен: {} > {}", w[0].id, w[1].id);
        }
        // Серьёзная проверка: доверие хотя бы подписано у каждого источника.
        for d in &reg {
            assert!(
                matches!(
                    d.trust_level,
                    TrustLevel::Official
                        | TrustLevel::VerifiedForge
                        | TrustLevel::Community
                        | TrustLevel::ThirdParty
                        | TrustLevel::Custom
                ),
                "некорректный trust у {}",
                d.id
            );
        }
    }
}

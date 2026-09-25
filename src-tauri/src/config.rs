use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub mods_folder: Option<String>,
    pub theme: Option<String>,
    pub accent: Option<String>,
    pub installed_sort: Option<String>,
    pub installed_collapsed: Option<bool>,
    pub card_size: Option<String>,
    /// Стиль интерфейса: "material" | "classic". `None` у существующих конфигов
    /// v0.3.0 — материал становится default-стилем (Material 3).
    pub style: Option<String>,
    /// Язык интерфейса: "ru" | "en". `None` до первой миграции/установки
    /// (для уже существующих конфигов 0.3.0 остаётся русский).
    pub language: Option<String>,
    /// Источники, которым пользователь разрешил сетевые запросы.
    /// `None` до первой инициализации (мигрируется при `load`).
    pub enabled_sources: Option<Vec<String>>,
    /// Активные для поиска/агрегации. `None` = «все enabled».
    pub selected_sources: Option<Vec<String>>,
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context(crate::i18n::t(
                "не удалось определить каталог конфигурации пользователя",
                "failed to determine the user config directory",
            ))?
            .join("beamng-mod-downloader");
        std::fs::create_dir_all(&dir).with_context(|| {
            crate::i18n::tf(
                "не удалось создать {0}",
                "failed to create {0}",
                &[&dir.display().to_string()],
            )
        })?;
        Ok(dir.join("config.json"))
    }

    pub fn load() -> Self {
        let existed = Self::config_path().map(|p| p.exists()).unwrap_or(false);
        let mut cfg = match Self::config_path() {
            Ok(path) => match std::fs::read_to_string(&path) {
                Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
                Err(_) => Self::default(),
            },
            Err(_) => Self::default(),
        };
        if cfg.language.is_none() {
            cfg.language = Some(if existed {
                // Существующий конфиг v0.3.0 был русским — язык по умолчанию RU.
                "ru".to_string()
            } else {
                // Первый запуск: язык определяется локалью системы.
                crate::i18n::Lang::detect_env().as_str().to_string()
            });
        }
        cfg.init_and_sanitize_sources(existed);
        cfg
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw).with_context(|| {
            crate::i18n::tf(
                "не удалось записать {0}",
                "failed to write {0}",
                &[&path.display().to_string()],
            )
        })
    }

    /// Миграция и санитизация выбора источников.
    ///
    /// - Новый конфиг (файла ещё нет): `enabled_sources` = рекомендуемые
    ///   defaults, `selected_sources` = `None` (все enabled).
    /// - Старый конфиг 0.2.x без поля `enabled_sources`: включаем legacy-набор
    ///   (все источники, которые были доступны раньше), остальные настройки не трогаем.
    /// - Неизвестные/удалённые id выбрасываются из обоих списков — они не должны
    ///   ломать загрузку конфига или UI.
    fn init_and_sanitize_sources(&mut self, file_existed: bool) {
        if self.enabled_sources.is_none() {
            self.enabled_sources =
                Some(crate::sources::registry::default_enabled_ids(!file_existed));
        }
        if let Some(list) = self.enabled_sources.as_mut() {
            list.retain(|id| crate::sources::registry::is_known(id));
            list.sort();
            list.dedup();
        }
        if let Some(list) = self.selected_sources.as_mut() {
            list.retain(|id| crate::sources::registry::is_known(id));
            list.sort();
            list.dedup();
        }
    }

    pub fn enabled_sources(&self) -> Vec<String> {
        self.enabled_sources
            .clone()
            .unwrap_or_else(|| crate::sources::registry::default_enabled_ids(true))
    }

    /// Нормализованный язык интерфейса ("ru" | "en"); при пустом/неизвестном
    /// значении — русский (back-compat с v0.3.0).
    pub fn language_str(&self) -> String {
        match self.language.as_deref() {
            Some(v) if v == "en" || v == "en-US" || v == "en_US" => "en".to_string(),
            Some(v) if v == "ru" || v == "ru-RU" || v == "ru_RU" => "ru".to_string(),
            _ => "ru".to_string(),
        }
    }

    /// Нормализованный стиль интерфейса; неизвестное значение = "material"
    /// (Material 3 — default-стиль v0.4.x; старые конфиги без поля получают его).
    pub fn style_str(&self) -> String {
        match self.style.as_deref() {
            Some("classic") => "classic".to_string(),
            _ => "material".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    /// Тесты с переменными окружения не должны гоняться параллельно.
    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn config_dir_path() -> PathBuf {
        dirs::config_dir()
            .map(|d| d.join("beamng-mod-downloader"))
            .expect("config dir")
    }

    /// Защита тестов: пишем config только во временный XDG_CONFIG_HOME.
    fn with_out_dirs<T>(name: &str, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp =
            std::env::temp_dir().join(format!("bmd-config-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        // dirs::config_dir уважает XDG_CONFIG_HOME для Linux.
        std::env::set_var("XDG_CONFIG_HOME", &tmp);
        let r = f();
        std::env::remove_var("XDG_CONFIG_HOME");
        if let Some(old) = old_xdg {
            std::env::set_var("XDG_CONFIG_HOME", old);
        }
        let _ = std::fs::remove_dir_all(&tmp);
        r
    }

    #[test]
    fn new_config_gets_recommended_defaults() {
        with_out_dirs("new", || {
            let cfg = Config::load();
            let enabled = cfg.enabled_sources();
            assert!(enabled.contains(&"beamngweb".to_string()));
            assert!(enabled.contains(&"github".to_string()));
            assert!(!enabled.contains(&"worldofmods".to_string()));
            assert!(cfg.selected_sources.is_none());
        });
    }

    #[test]
    fn legacy_config_migrates_all_old_sources_and_keeps_settings() {
        with_out_dirs("legacy", || {
            let dir = config_dir_path();
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("config.json"),
                r##"{"modsFolder":"/x/mods","theme":"light","accent":"#ffffff"}"##,
            )
            .unwrap();
            let cfg = Config::load();
            assert_eq!(cfg.mods_folder.as_deref(), Some("/x/mods"));
            assert_eq!(cfg.theme.as_deref(), Some("light"));
            assert_eq!(cfg.accent.as_deref(), Some("#ffffff"));
            let enabled = cfg.enabled_sources();
            assert!(enabled.contains(&"worldofmods".to_string()));
            assert!(enabled.contains(&"beamngweb".to_string()));
            assert!(enabled.contains(&"github".to_string()));
        });
    }

    #[test]
    fn unknown_source_ids_are_dropped_but_known_kept() {
        with_out_dirs("unknown", || {
            let dir = config_dir_path();
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("config.json"),
                r#"{
                    "modsFolder": "/x/mods",
                    "enabledSources": ["beamngweb", "ghost", "github"],
                    "selectedSources": ["github", "deleted-source"]
                }"#,
            )
            .unwrap();
            let cfg = Config::load();
            let enabled = cfg.enabled_sources();
            assert_eq!(enabled, vec!["beamngweb".to_string(), "github".to_string()]);
            assert_eq!(
                cfg.selected_sources.clone().unwrap(),
                vec!["github".to_string()]
            );
            assert_eq!(cfg.mods_folder.as_deref(), Some("/x/mods"));
        });
    }

    #[test]
    fn explicit_selection_of_unknown_id_survives_save() {
        // Санитизация происходит на load; запись работает с уже чистыми списками.
        let mut cfg = Config {
            enabled_sources: Some(vec!["beamngweb".to_string()]),
            ..Config::default()
        };
        cfg.init_and_sanitize_sources(false);
        assert_eq!(cfg.enabled_sources(), vec!["beamngweb".to_string()]);
    }

    #[test]
    fn existing_config_keeps_russian_by_default() {
        with_out_dirs("lang-legacy", || {
            let dir = config_dir_path();
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("config.json"), r##"{"theme":"dark"}"##).unwrap();
            let cfg = Config::load();
            // Существующий конфиг без поля language — русский (back-compat).
            assert_eq!(cfg.language_str(), "ru");
        });
    }

    #[test]
    fn explicit_language_is_preserved() {
        with_out_dirs("lang-en", || {
            let dir = config_dir_path();
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("config.json"), r##"{"language":"en"}"##).unwrap();
            let cfg = Config::load();
            assert_eq!(cfg.language_str(), "en");
        });
    }

    #[test]
    fn new_config_language_is_valid() {
        with_out_dirs("lang-new", || {
            let cfg = Config::load();
            assert!(matches!(cfg.language_str().as_str(), "ru" | "en"));
        });
    }

    #[test]
    fn unknown_language_falls_back_to_russian() {
        let cfg = Config {
            language: Some("de".to_string()),
            ..Config::default()
        };
        assert_eq!(cfg.language_str(), "ru");
    }

    #[test]
    fn missing_style_defaults_to_material() {
        let cfg = Config::default();
        assert_eq!(cfg.style_str(), "material");
        let classic = Config {
            style: Some("classic".to_string()),
            ..Config::default()
        };
        assert_eq!(classic.style_str(), "classic");
    }

    #[test]
    fn unknown_style_falls_back_to_material() {
        let cfg = Config {
            style: Some("amoled".to_string()),
            ..Config::default()
        };
        assert_eq!(cfg.style_str(), "material");
    }
}

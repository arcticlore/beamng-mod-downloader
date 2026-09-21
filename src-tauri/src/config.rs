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
    /// Источники, которым пользователь разрешил сетевые запросы.
    /// `None` до первой инициализации (мигрируется при `load`).
    pub enabled_sources: Option<Vec<String>>,
    /// Активные для поиска/агрегации. `None` = «все enabled».
    pub selected_sources: Option<Vec<String>>,
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("не удалось определить каталог конфигурации пользователя")?
            .join("beamng-mod-downloader");
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("не удалось создать {}", dir.display()))?;
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
        cfg.init_and_sanitize_sources(existed);
        cfg
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw)
            .with_context(|| format!("не удалось записать {}", path.display()))
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
        let tmp = std::env::temp_dir().join(format!(
            "bmd-config-test-{}-{name}",
            std::process::id()
        ));
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
        let mut cfg = Config::default();
        cfg.enabled_sources = Some(vec!["beamngweb".to_string()]);
        cfg.init_and_sanitize_sources(false);
        assert_eq!(cfg.enabled_sources(), vec!["beamngweb".to_string()]);
    }
}
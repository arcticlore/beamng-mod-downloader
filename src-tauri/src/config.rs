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
        match Self::config_path() {
            Ok(path) => match std::fs::read_to_string(&path) {
                Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
                Err(_) => Self::default(),
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw)
            .with_context(|| format!("не удалось записать {}", path.display()))
    }
}

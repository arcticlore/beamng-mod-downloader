use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModItem {
    pub id: String,
    pub source: String,
    pub name: String,
    pub thumbnail: Option<String>,
    pub description: Option<String>,
    /// Опакованный ключ источника для деталей/установки
    /// (worldofmods: полный URL страницы мода; beamng: "id/ver/filename").
    pub key: String,
    pub category: Option<String>,
    pub author: Option<String>,
    pub published: Option<String>,
    pub downloads: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModSearchResult {
    pub items: Vec<ModItem>,
    pub total_pages: u32,
    pub current_page: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModDetail {
    pub item: ModItem,
    pub full_description: Option<String>,
    pub screenshots: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub source: String,
    pub mod_id: String,
    pub name: String,
    pub key: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DownloadState {
    pub key: String,
    pub name: String,
    pub filename: String,
    pub received: u64,
    pub total: Option<u64>,
    pub speed_bps: u64,
    pub state: String,
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified: u64,
    pub source: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModsFolderCandidate {
    pub path: String,
    pub exists: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SourceCategory {
    pub id: String,
    pub label: String,
}

/// Пользовательские настройки интерфейса (тема, акцент, панель модов).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// "dark" | "light".
    pub theme: Option<String>,
    /// HEX-цвет акцента, например "#4f8cff".
    pub accent: Option<String>,
    /// Сортировка установленных: "date" | "name" | "size".
    pub installed_sort: Option<String>,
    /// Сворачивать таблицу установленных до сводки.
    pub installed_collapsed: Option<bool>,
    /// Размер карточек в браузере: "compact" | "normal" | "large".
    pub card_size: Option<String>,
}

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
    pub published: Option<String>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified: u64,
    pub source: String,
    /// Запись о происхождении: какую версию (дату публикации) мы видели при установке.
    pub key: Option<String>,
    pub published: Option<String>,
}

/// Результат проверки целостности одного установленного архива.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub filename: String,
    pub size_bytes: u64,
    /// Прошёл ли архив структурную zip-проверку.
    pub zip_ok: bool,
    pub entries: usize,
    /// Текущий SHA-256 файла.
    pub sha256: Option<String>,
    /// SHA-256, сохранённый в ledger при установке.
    pub tracked_sha256: Option<String>,
    /// Some(true) — совпадает; Some(false) — файл изменён/заменён после установки.
    pub hash_ok: Option<bool>,
    pub error: Option<String>,
}

/// Результат проверки обновления одного установленного мода.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdate {
    pub filename: String,
    pub source: String,
    pub key: String,
    /// Версия, установленная лаунчером (когда мы её поставили).
    pub installed_published: Option<String>,
    /// Текущая версия на источнике.
    pub latest_published: Option<String>,
    pub has_update: bool,
    pub error: Option<String>,
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

// --- Единый registry источников (single source of truth для UI) ---

/// Группа источника для группировки в Source Picker.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceGroup {
    Official,
    Forges,
    Community,
    Custom,
}

/// Уровень доверия к источнику. Показывается в карточках и в Settings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Official,
    VerifiedForge,
    Community,
    ThirdParty,
    Custom,
}

/// Режим установки, который источник реально поддерживает.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
    /// Безопасный кандидат для копирования в папку модов.
    ModsZip,
    /// Открыть страницу/инструкцию, не устанавливать автоматически.
    ManualExternal,
    /// Показать причину.
    Unsupported,
}

/// Требования к аутентификации источника.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceAuth {
    None,
    /// Требуется API key (хранится в защищённом хранилище, не в config.json).
    ApiKey,
    /// Другая пользовательская настройка (например учётная запись).
    Custom,
}

/// Статус конфигурации источника на текущей машине.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Ready,
    NeedsApiKey,
    NotConfigured,
    Unavailable,
    RateLimited,
}

/// Правило, по которому frontend предсказывает имя локального zip-файла
/// до фактического resolve на backend'е (используется для сравнения с уже
/// установленными модами). Эталонный ответ всегда даёт backend.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilenameRule {
    /// Последний сегмент key, очищенный от спецсимволов.
    Basename,
    /// Последний сегмент key без расширения `.html`.
    HtmlSlug,
    /// `owner-repo.zip` из `https://…/owner/repo`.
    OwnerRepo,
    /// Ключ как есть (после санитизации backend'а).
    KeyStem,
}

/// Честно описывает, что источник умеет (не изображаем недоступное доступным).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceCapabilities {
    pub search: bool,
    pub categories: bool,
    pub pagination: bool,
    pub detail: bool,
    pub direct_zip_download: bool,
    pub manual_download: bool,
    pub checksums: bool,
    pub update_detection: bool,
}

/// Декларативное описание источника для UI. Единственный source of truth —
/// backend registry (`sources::registry::registry()`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceDescriptor {
    pub id: String,
    pub label: String,
    pub group: SourceGroup,
    pub trust_level: TrustLevel,
    /// Входит ли источник в рекомендуемый набор по умолчанию (для новых конфигов).
    pub enabled_by_default: bool,
    /// Был ли источник включён в legacy-версии (0.2.x) — для миграции старых конфигов.
    pub legacy_default: bool,
    pub homepage: String,
    pub terms_or_policy_url: Option<String>,
    /// Предупреждение при включении (например «сторонний источник»).
    pub warning: Option<String>,
    pub install_mode: InstallMode,
    pub auth: SourceAuth,
    pub status: SourceStatus,
    pub filename_rule: FilenameRule,
    pub capabilities: SourceCapabilities,
    pub categories: Vec<SourceCategory>,
}

/// Состояние выбора источников пользователем (persisted в config).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceSelection {
    /// Источники, которым пользователь разрешил сетевые запросы.
    pub enabled: Vec<String>,
    /// Активные для поиска/агрегации. `None` = «все enabled».
    pub selected: Option<Vec<String>>,
}

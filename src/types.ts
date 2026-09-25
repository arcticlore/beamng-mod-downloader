import type { MessageKey, Lang } from "./i18n";

export interface ModItem {
  id: string;
  source: string;
  name: string;
  thumbnail: string | null;
  description: string | null;
  key: string;
  category: string | null;
  author: string | null;
  published: string | null;
  downloads: string | null;
  sizeBytes: number | null;
}

export interface ModSearchResult {
  items: ModItem[];
  totalPages: number;
  currentPage: number;
}

export interface ModDetail {
  item: ModItem;
  fullDescription: string | null;
  screenshots: string[];
}

export interface InstallRequest {
  source: string;
  modId: string;
  name: string;
  key: string;
  published?: string | null;
}

export type DownloadState = {
  key: string;
  name: string;
  filename: string;
  received: number;
  total: number | null;
  speedBps: number;
  state: "downloading" | "done" | "error";
  error: string | null;
};

export interface InstalledMod {
  filename: string;
  path: string;
  sizeBytes: number;
  modified: number;
  source: string;
  key?: string | null;
  published?: string | null;
}

export interface ModUpdate {
  filename: string;
  source: string;
  key: string;
  installedPublished: string | null;
  latestPublished: string | null;
  hasUpdate: boolean;
  error: string | null;
}

export interface IntegrityReport {
  filename: string;
  sizeBytes: number;
  zipOk: boolean;
  entries: number;
  sha256: string | null;
  trackedSha256: string | null;
  /** null — неизвестно (нет записи в ledger); false — файл изменён после установки. */
  hashOk: boolean | null;
  error: string | null;
}

export interface SourceCategory {
  id: string;
  label: string;
}

export interface ModsFolderCandidate {
  path: string;
  exists: boolean;
}

export interface AppSettings {
  theme?: string | null;
  accent?: string | null;
  installedSort?: string | null;
  installedCollapsed?: boolean | null;
  cardSize?: string | null;
  style?: string | null;
  language?: string | null;
}

/** Опции интерфейса: label задаётся ключом словаря (RU/EN), значение — id для сохранения. */
export interface UiOption {
  id: string;
  labelKey: MessageKey;
}

export const THEMES: UiOption[] = [
  { id: "dark", labelKey: "theme_dark" },
  { id: "light", labelKey: "theme_light" },
];

/** Стиль интерфейса: "material" (default, Material 3) | "classic" (v0.3.0 CSS). */
export const STYLES: UiOption[] = [
  { id: "material", labelKey: "style_material" },
  { id: "classic", labelKey: "style_classic" },
];

export const CARD_SIZES: UiOption[] = [
  { id: "compact", labelKey: "cardsize_compact" },
  { id: "normal", labelKey: "cardsize_normal" },
  { id: "large", labelKey: "cardsize_large" },
];

export const INSTALLED_SORTS: UiOption[] = [
  { id: "date", labelKey: "sort_installed_date" },
  { id: "name", labelKey: "sort_installed_name" },
  { id: "size", labelKey: "sort_installed_size" },
];

export const BROWSER_SORTS: UiOption[] = [
  { id: "relevance", labelKey: "sort_browser_relevance" },
  { id: "updated", labelKey: "sort_browser_updated" },
  { id: "name", labelKey: "sort_browser_name" },
  { id: "popularity", labelKey: "sort_browser_popularity" },
  { id: "size", labelKey: "sort_browser_size" },
];

export type SourceGroup = "official" | "forges" | "community" | "custom";
export type TrustLevel =
  | "official"
  | "verified_forge"
  | "community"
  | "third_party"
  | "custom";
export type InstallMode = "mods_zip" | "manual_external" | "unsupported";
export type SourceAuth = "none" | "api_key" | "custom";
export type SourceStatus =
  | "ready"
  | "needs_api_key"
  | "not_configured"
  | "unavailable"
  | "rate_limited";
export type FilenameRule = "basename" | "html_slug" | "owner_repo" | "key_stem";

export interface SourceCapabilities {
  search: boolean;
  categories: boolean;
  pagination: boolean;
  detail: boolean;
  directZipDownload: boolean;
  manualDownload: boolean;
  checksums: boolean;
  updateDetection: boolean;
}

/** Описание источника из backend registry — единый source of truth. */
export interface SourceDescriptor {
  id: string;
  label: string;
  group: SourceGroup;
  trustLevel: TrustLevel;
  enabledByDefault: boolean;
  legacyDefault: boolean;
  homepage: string;
  termsOrPolicyUrl: string | null;
  warning: string | null;
  installMode: InstallMode;
  auth: SourceAuth;
  status: SourceStatus;
  filenameRule: FilenameRule;
  capabilities: SourceCapabilities;
  categories: SourceCategory[];
}

/** enabled — разрешены сетевые запросы; selected — активны в поиске (null = все enabled). */
export interface SourceSelection {
  enabled: string[];
  selected: string[] | null;
}

/** Санитизация имени файла — зеркалит `sanitize_filename` в src-tauri/src/http.rs. */
export function sanitizeFileName(raw: string): string {
  let cleaned = raw
    .trim()
    .replace(/[\/\\:*?"<>|\0]/g, "_")
    .replace(/\s/g, "_")
    .replace(/\.+$/g, "");
  if (!cleaned) cleaned = "mod";
  return cleaned;
}

export function formatBytes(
  n: number | null | undefined,
  lang: Lang = "ru",
): string {
  if (n == null) return "";
  const unit = (value: number): string => {
    if (value < 1024) return `${value} ${tUnit(lang, "bytes_byte")}`;
    if (value < 1024 * 1024)
      return `${(value / 1024).toFixed(1)} ${tUnit(lang, "bytes_kb")}`;
    if (value < 1024 * 1024 * 1024)
      return `${(value / (1024 * 1024)).toFixed(1)} ${tUnit(lang, "bytes_mb")}`;
    return `${(value / (1024 * 1024 * 1024)).toFixed(2)} ${tUnit(
      lang,
      "bytes_gb",
    )}`;
  };
  return unit(n);
}

export function formatSpeed(
  bps: number | null | undefined,
  lang: Lang = "ru",
): string {
  if (!bps) return "";
  const suffix = tUnit(lang, "bytes_per_sec_suffix");
  if (bps < 1024) return `${Math.round(bps)} ${tUnit(lang, "bytes_byte")}${suffix}`;
  if (bps < 1024 * 1024)
    return `${(bps / 1024).toFixed(1)} ${tUnit(lang, "bytes_kb")}${suffix}`;
  return `${(bps / (1024 * 1024)).toFixed(1)} ${tUnit(lang, "bytes_mb")}${suffix}`;
}

function tUnit(lang: Lang, key: MessageKey): string {
  return lang === "en" ? (enUnits[key] ?? "") : (ruUnits[key] ?? "");
}

const ruUnits: Record<string, string> = {
  bytes_byte: "Б",
  bytes_kb: "КБ",
  bytes_mb: "МБ",
  bytes_gb: "ГБ",
  bytes_per_sec_suffix: "/с",
};

const enUnits: Record<string, string> = {
  bytes_byte: "B",
  bytes_kb: "KB",
  bytes_mb: "MB",
  bytes_gb: "GB",
  bytes_per_sec_suffix: "/s",
};

const STOPWORDS = new Set(["mod", "mods", "beamng", "drive", "the", "and", "for", "with", "new"]);

export function tokenize(text: string): Set<string> {
  const out = new Set<string>();
  for (const raw of text.toLowerCase().split(/[^a-z0-9а-яё]+/)) {
    if (raw.length < 2) continue;
    if (raw.length >= 2 && raw.length <= 3 && /\d/.test(raw)) continue;
    if (STOPWORDS.has(raw)) continue;
    out.add(raw);
  }
  return out;
}

/** Схожесть двух наборов токенов (Jaccard): 1 = совпадают, 0 = нет общих. */
export function similar(a: Set<string>, b: Set<string>): number {
  if (a.size === 0 || b.size === 0) return 0;
  let inter = 0;
  for (const t of a) if (b.has(t)) inter++;
  return inter / (a.size + b.size - inter);
}

/**
 * Ищет среди установленных архивов файл, который, скорее всего, и есть этот мод.
 * Используется, чтобы не качать один и тот же мод дважды, когда он был
 * установлен не через наш лаунчер (другое имя файла).
 */
export function findSimilarInstalled(
  item: ModItem,
  installed: { filename: string; path: string }[],
): { filename: string; path: string } | null {
  const nameTokens = tokenize(item.name);
  if (nameTokens.size === 0) return null;
  let best: { filename: string; path: string; score: number } | null = null;
  for (const mod of installed) {
    const fileTokens = tokenize(mod.filename.replace(/\.zip$/i, ""));
    const score = similar(nameTokens, fileTokens);
    if (score < 0.4) continue;
    if (!best || score > best.score) best = { ...mod, score };
  }
  return best ? { filename: best.filename, path: best.path } : null;
}
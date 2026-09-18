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
}

export const THEMES = [
  { id: "dark", label: "Тёмная" },
  { id: "light", label: "Светлая" },
];

export const CARD_SIZES = [
  { id: "compact", label: "Компактные" },
  { id: "normal", label: "Обычные" },
  { id: "large", label: "Крупные" },
];

export const INSTALLED_SORTS = [
  { id: "date", label: "По дате изменения" },
  { id: "name", label: "По имени" },
  { id: "size", label: "По размеру" },
];

export const BROWSER_SORTS = [
  { id: "relevance", label: "Актуальность" },
  { id: "updated", label: "По новизне" },
  { id: "name", label: "По имени (А-Я)" },
  { id: "popularity", label: "По популярности" },
  { id: "size", label: "По размеру" },
];

export const SOURCES: Record<string, { id: string; label: string }> = {
  worldofmods: { id: "worldofmods", label: "WorldOfMods" },
  beamngweb: { id: "beamngweb", label: "Официальный сайт BeamNG" },
  github: { id: "github", label: "GitHub-релизы" },
};

/** Имя zip-файла, который будет создан при установке этого мода, по его ключу. */
export function installedFileName(item: ModItem): string {
  const parts = item.key.split("/").filter(Boolean);
  const last = parts.pop() ?? "";
  if (item.source === "worldofmods") return last.replace(/\.html$/, "") + ".zip";
  if (item.source === "beamngweb") return last + ".zip";
  if (item.source === "github") {
    const owner = parts.pop() ?? "";
    return `${owner}-${last}.zip`;
  }
  return last;
}

export function formatBytes(n: number | null | undefined): string {
  if (n == null) return "";
  if (n < 1024) return `${n} Б`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} КБ`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} МБ`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} ГБ`;
}

export function formatSpeed(bps: number | null | undefined): string {
  if (!bps) return "";
  if (bps < 1024) return `${Math.round(bps)} Б/с`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} КБ/с`;
  return `${(bps / (1024 * 1024)).toFixed(1)} МБ/с`;
}

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
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
}

export interface SourceCategory {
  id: string;
  label: string;
}

export interface ModsFolderCandidate {
  path: string;
  exists: boolean;
}

export const SOURCES: Record<string, { id: string; label: string }> = {
  worldofmods: { id: "worldofmods", label: "WorldOfMods" },
  beamngweb: { id: "beamngweb", label: "Официальный сайт BeamNG" },
  github: { id: "github", label: "GitHub-релизы (без токена)" },
  beamng: { id: "beamng", label: "Репозиторий BeamNG (токен)" },
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
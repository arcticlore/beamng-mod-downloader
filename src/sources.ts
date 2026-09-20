import type {
  FilenameRule,
  InstallMode,
  ModItem,
  SourceDescriptor,
  SourceGroup,
  SourceStatus,
  TrustLevel,
} from "./types";
import { sanitizeFileName } from "./types.ts";

export type SourcePreset =
  | "recommended"
  | "official_forges"
  | "all_configured"
  | "clear";

export const GROUP_ORDER: Record<SourceGroup, number> = {
  official: 0,
  forges: 1,
  community: 2,
  custom: 3,
};

export const TRUST_ORDER: Record<TrustLevel, number> = {
  official: 0,
  verified_forge: 1,
  community: 2,
  third_party: 3,
  custom: 4,
};

export function groupLabel(g: SourceGroup): string {
  switch (g) {
    case "official":
      return "Официальные";
    case "forges":
      return "Open-source forges";
    case "community":
      return "Community";
    case "custom":
      return "Пользовательские";
  }
}

export function trustLabel(t: TrustLevel): string {
  switch (t) {
    case "official":
      return "Официальный";
    case "verified_forge":
      return "Open-source forge";
    case "community":
      return "Community";
    case "third_party":
      return "Сторонний / не проверен";
    case "custom":
      return "Пользовательский / не проверен";
  }
}

export function statusLabel(s: SourceStatus): string {
  switch (s) {
    case "ready":
      return "готов";
    case "needs_api_key":
      return "требуется API key";
    case "not_configured":
      return "не настроен";
    case "unavailable":
      return "недоступен";
    case "rate_limited":
      return "rate limited";
  }
}

export function installModeLabel(m: InstallMode): string {
  switch (m) {
    case "mods_zip":
      return "Автоустановка ZIP";
    case "manual_external":
      return "Только вручную (внешняя страница)";
    case "unsupported":
      return "Не поддерживается";
  }
}

/** Источники, отсортированные по общему порядку групп, затем по label. */
export function sortByGroup(registry: SourceDescriptor[]): SourceDescriptor[] {
  return [...registry].sort(
    (a, b) =>
      GROUP_ORDER[a.group] - GROUP_ORDER[b.group] ||
      a.label.localeCompare(b.label, "ru"),
  );
}

/**
 * Активные для поиска источники: selected == null означает «все enabled».
 * Результат пересекается с enabled (disabled не запрашивается никогда) и
 * упорядочивается по registry.
 */
export function resolveActiveSources(
  registry: SourceDescriptor[],
  enabled: string[],
  selected: string[] | null,
): string[] {
  const allowed = new Set(enabled);
  const chosen = selected === null ? enabled : selected;
  return sortByGroup(registry)
    .map((d) => d.id)
    .filter((id) => allowed.has(id) && chosen.includes(id));
}

/** Применяет quick-preset к заданному набору enabled-источников. */
export function applyPreset(
  preset: SourcePreset,
  registry: SourceDescriptor[],
  enabled: string[],
): string[] {
  const allowed = new Set(enabled);
  const ordered = sortByGroup(registry);
  switch (preset) {
    case "recommended":
      return ordered
        .filter((d) => d.enabledByDefault && allowed.has(d.id))
        .map((d) => d.id);
    case "official_forges":
      return ordered
        .filter(
          (d) =>
            allowed.has(d.id) &&
            (d.group === "official" || d.group === "forges"),
        )
        .map((d) => d.id);
    case "all_configured":
      return ordered.filter((d) => allowed.has(d.id)).map((d) => d.id);
    case "clear":
      return [];
  }
}

/**
 * Имя zip-файла, который создаст backend для этого мода, по правилу источника.
 * Это лишь предикция для UI (сравнение с установленными, отображение имени);
 * эталонное имя всегда возвращает backend при resolve/download.
 */
export function filenameFor(item: ModItem, rule: FilenameRule): string {
  const parts = item.key.split("/").filter(Boolean);
  const last = parts.pop() ?? "";
  switch (rule) {
    case "basename":
      return sanitizeFileName(last) + ".zip";
    case "html_slug":
      return sanitizeFileName(last.replace(/\.html$/, "")) + ".zip";
    case "owner_repo": {
      const owner = parts.pop() ?? "";
      return `${owner}-${last}.zip`;
    }
    case "key_stem":
      return sanitizeFileName(last) + ".zip";
  }
}

/** Домен из key мода (для показа final domain), если это http-ссылка. */
export function urlHost(url: string | null | undefined): string | null {
  if (!url) return null;
  try {
    const u = new URL(url);
    if (u.protocol !== "http:" && u.protocol !== "https:") return null;
    return u.hostname;
  } catch {
    return null;
  }
}

/** Дедупликация листинга по каноническому идентификатору (`source:id`). */
export function dedupById(items: ModItem[]): ModItem[] {
  const seen = new Set<string>();
  const out: ModItem[] = [];
  for (const it of items) {
    if (seen.has(it.id)) continue;
    seen.add(it.id);
    out.push(it);
  }
  return out;
}
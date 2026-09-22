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
 * Результат пересекается с enabled (disabled не запрашивается никогда) и с
 * `capabilities.search` (manual/link-only источник не становится searchable),
 * затем упорядочивается по registry.
 */
export function resolveActiveSources(
  registry: SourceDescriptor[],
  enabled: string[],
  selected: string[] | null,
): string[] {
  const allowed = new Set(enabled);
  const chosen = selected === null ? enabled : selected;
  return sortByGroup(registry)
    .filter(
      (d) =>
        d.capabilities.search && allowed.has(d.id) && chosen.includes(d.id),
    )
    .map((d) => d.id);
}

/**
 * Все enabled и search-capable источники в порядке registry.
 * База выбора поиска: presets и toggle работают только с этим intersection.
 */
export function searchCapableEnabledIds(
  registry: SourceDescriptor[],
  enabled: string[],
): string[] {
  const allowed = new Set(enabled);
  return sortByGroup(registry)
    .filter((d) => d.capabilities.search && allowed.has(d.id))
    .map((d) => d.id);
}

/**
 * Канонизация набора выбранных id: если он равен всем допустимым
 * (search-capable enabled) — возвращает `null` («все»); иначе сортированный
 * дедуплицированный список (ноль = `[]`).
 */
export function canonicalizeSelection(
  searchCapable: string[],
  ids: string[],
): string[] | null {
  const canon = [...new Set(ids)].sort();
  const all = [...searchCapable].sort();
  if (canon.length === all.length && all.every((v, i) => v === canon[i])) {
    return null;
  }
  return canon;
}

/**
 * Toggle поиска по источнику с учётом `selected=null`. Non-search-capable
 * источники не трогаются. Возвращает `null`, когда результат равен всем
 * допустимым, иначе explicit подмножество.
 */
export function toggleSourceSelection(
  registry: SourceDescriptor[],
  enabled: string[],
  selected: string[] | null,
  id: string,
): string[] | null {
  const desc = registry.find((d) => d.id === id);
  if (!desc || !desc.capabilities.search) return selected;
  const all = searchCapableEnabledIds(registry, enabled);
  if (!all.includes(id)) return selected;
  const base = selected === null ? all : selected.filter((i) => all.includes(i));
  const next = new Set(base);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  return canonicalizeSelection(all, [...next]);
}

/** Поиск/фильтр источников по label или id (case-insensitive), сохраняя порядок registry. */
export function filterSources(
  registry: SourceDescriptor[],
  text: string,
): SourceDescriptor[] {
  const q = text.trim().toLowerCase();
  const ordered = sortByGroup(registry);
  if (!q) return ordered;
  return ordered.filter(
    (d) => d.label.toLowerCase().includes(q) || d.id.toLowerCase().includes(q),
  );
}

/** Применяет quick-preset к enabled ∩ search-capable источникам. */
export function applyPreset(
  preset: SourcePreset,
  registry: SourceDescriptor[],
  enabled: string[],
): string[] {
  const ordered = sortByGroup(registry).filter(
    (d) => d.capabilities.search && enabled.includes(d.id),
  );
  switch (preset) {
    case "recommended":
      return ordered.filter((d) => d.enabledByDefault).map((d) => d.id);
    case "official_forges":
      return ordered
        .filter((d) => d.group === "official" || d.group === "forges")
        .map((d) => d.id);
    case "all_configured":
      return ordered.map((d) => d.id);
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
    const key = `${it.source}:${it.id}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(it);
  }
  return out;
}
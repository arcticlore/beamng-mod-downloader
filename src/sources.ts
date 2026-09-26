import type {
  FilenameRule,
  InstallMode,
  ModItem,
  SourceDescriptor,
  SourceGroup,
  SourceStatus,
  TrustLevel,
} from "./types";
import type { MessageKey } from "./i18n";
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

export function groupLabel(g: SourceGroup): MessageKey {
  switch (g) {
    case "official":
      return "group_official";
    case "forges":
      return "group_forges";
    case "community":
      return "group_community";
    case "custom":
      return "group_custom";
  }
}

export function trustLabel(t: TrustLevel): MessageKey {
  switch (t) {
    case "official":
      return "trust_official";
    case "verified_forge":
      return "trust_verified_forge";
    case "community":
      return "trust_community";
    case "third_party":
      return "trust_third_party";
    case "custom":
      return "trust_custom";
  }
}

export function statusLabel(s: SourceStatus): MessageKey {
  switch (s) {
    case "ready":
      return "status_ready";
    case "needs_api_key":
      return "status_needs_api_key";
    case "not_configured":
      return "status_not_configured";
    case "unavailable":
      return "status_unavailable";
    case "rate_limited":
      return "status_rate_limited";
  }
}

export function installModeLabel(m: InstallMode): MessageKey {
  switch (m) {
    case "mods_zip":
      return "installmode_autozip";
    case "manual_external":
      return "installmode_manual";
    case "unsupported":
      return "installmode_unsupported";
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

/** Категория ссылки для панели «Добавить мод»: вложение форума или прямой .zip. */
export type LinkKind = "forum" | "direct";

/**
 * Классифицирует вставленную ссылку по форме (те же правила, что у backend
 * `beamngforum::canonical_key`): `attachment:<id>`, голые цифры и любые
 * beamng.com-ссылки с путём `/attachments/<…><digits>` — вложение форума;
 * всё остальное — прямой .zip (backend всё равно валидирует .zip + allowlist).
 * Пустая строка — `null`.
 */
export function classifyLink(url: string): LinkKind | null {
  const t = url.trim();
  if (!t) return null;
  if (/^attachment:\d+$/.test(t)) return "forum";
  if (/^\d+$/.test(t)) return "forum";
  let u: URL;
  try {
    u = new URL(t);
  } catch {
    return "direct";
  }
  if (u.protocol !== "http:" && u.protocol !== "https:") return "direct";
  const host = u.hostname.toLowerCase();
  if (host === "beamng.com" || host.endsWith(".beamng.com")) {
    const segs = u.pathname.split("/").filter(Boolean);
    if (
      segs.length === 2 &&
      segs[0] === "attachments" &&
      /\d+$/.test(segs[1])
    ) {
      return "forum";
    }
  }
  return "direct";
}

/** Канонический URL вложения: `…/attachments/<id>/` (для показа disabled-хинта). */
export function forumIdFromLink(url: string): string | null {
  const t = url.trim();
  const m = t.match(/attachment:(\d+)/) ?? t.match(/^(\d+)$/);
  if (m) return m[1];
  try {
    const u = new URL(t);
    const segs = u.pathname.split("/").filter(Boolean);
    if (segs.length === 2 && segs[0] === "attachments") {
      const digits = (segs[1].match(/(\d+)$/) ?? [])[1];
      return digits ?? null;
    }
  } catch {
    /* пустая */
  }
  return null;
}
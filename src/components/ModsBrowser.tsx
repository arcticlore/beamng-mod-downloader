import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCategories, searchMods } from "../api";
import { useSources } from "../SourcesContext";
import { dedupById, resolveActiveSources } from "../sources";
import {
  BROWSER_SORTS,
  findSimilarInstalled,
  type DownloadState,
  type ModItem,
  type SourceCategory,
} from "../types";
import { ModCard } from "./ModCard";
import { SourcePicker } from "./SourcePicker";

/** Политика глубины агрегации по нескольким источникам — не «список источников». */
const AGG_DEPTH_MAX = 3;

function dateOf(published: string | null): number {
  if (!published) return 0;
  const ms = Date.parse(published);
  return Number.isFinite(ms) ? ms : 0;
}

function popOf(m: ModItem): number {
  const n = Number((m.downloads ?? "").replace(/[^\d]/g, ""));
  return Number.isFinite(n) ? n : 0;
}

interface Props {
  downloads: Record<string, DownloadState>;
  installedNames: Set<string>;
  installedList: { filename: string; path: string }[];
  cardSize: string;
  onInstall: (item: ModItem) => void;
  onInfo: (item: ModItem) => void;
  onOpenSettings: () => void;
}

export function ModsBrowser({
  downloads,
  installedNames,
  installedList,
  cardSize,
  onInstall,
  onInfo,
  onOpenSettings,
}: Props) {
  const { registry, selection, labelOf, filenameFor } = useSources();
  const [pickerOpen, setPickerOpen] = useState(false);

  const activeSources = useMemo(() => {
    if (!selection) return [];
    return resolveActiveSources(registry, selection.enabled, selection.selected);
  }, [registry, selection]);

  const activeKey = activeSources.join(",");
  const categoriesDescriptor = useMemo(() => {
    return activeSources.length === 1
      ? registry.find((d) => d.id === activeSources[0])
      : undefined;
  }, [registry, activeSources]);

  const [items, setItems] = useState<ModItem[]>([]);
  const [totalPages, setTotalPages] = useState(1);
  const [page, setPage] = useState(1);
  const [category, setCategory] = useState<string>("all");
  const [sort, setSort] = useState<string>("relevance");
  const [categories, setCategories] = useState<SourceCategory[]>([]);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [partialErrors, setPartialErrors] = useState<string[]>([]);
  const [aggDepth, setAggDepth] = useState(1);
  const [aggMore, setAggMore] = useState(false);
  const [hiddenSources, setHiddenSources] = useState<Set<string>>(new Set());

  const seqRef = useRef(0);

  const single = activeSources.length === 1;
  const multi = activeSources.length > 1;

  useEffect(() => {
    if (!categoriesDescriptor) {
      setCategories([]);
      return;
    }
    if (categoriesDescriptor.capabilities.categories) {
      getCategories(categoriesDescriptor.id)
        .then(setCategories)
        .catch(() => setCategories([]));
    } else {
      setCategories([]);
    }
  }, [categoriesDescriptor]);

  useEffect(() => {
    const t = setTimeout(() => setDebouncedQuery(query.trim()), 350);
    return () => clearTimeout(t);
  }, [query]);

  const load = useCallback(
    async (
      srcs: string[],
      cat: string,
      pg: number,
      q: string,
      ord: string,
      depth: number,
    ) => {
      const seq = ++seqRef.current;
      setLoading(true);
      setError(null);
      setPartialErrors([]);
      try {
        if (srcs.length === 1) {
          const res = await searchMods(
            srcs[0],
            q || null,
            cat === "all" ? null : cat,
            pg,
            ord,
          );
          if (seq !== seqRef.current) return;
          setItems(res.items);
          setTotalPages(res.totalPages || 1);
        } else if (srcs.length > 1) {
          const merged: ModItem[] = [];
          const errors: string[] = [];
          const pages = Math.max(1, Math.min(depth, AGG_DEPTH_MAX));
          let maxTotal = 1;
          await Promise.all(
            srcs.map(async (s) => {
              let srcOk = false;
              try {
                for (let i = 1; i <= pages; i++) {
                  const r = await searchMods(s, q || null, null, i, ord);
                  srcOk = true;
                  if (r.totalPages > maxTotal) maxTotal = r.totalPages;
                  if (r.items) merged.push(...r.items);
                }
              } catch (e) {
                console.warn(`источник ${s} ${srcOk ? "частично" : ""} недоступен:`, e);
                errors.push(`${labelOf(s)}: ${String(e)}`);
              }
            }),
          );
          if (seq !== seqRef.current) return;
          if (merged.length === 0 && errors.length === srcs.length) {
            throw new Error("все источники сейчас недоступны");
          }
          setItems(dedupById(merged));
          setPartialErrors(errors);
          setAggMore(maxTotal > pages);
          setTotalPages(1);
        } else {
          setItems([]);
          setTotalPages(1);
        }
      } catch (e) {
        if (seq === seqRef.current) {
          setError(String(e));
          setItems([]);
        }
      } finally {
        if (seq === seqRef.current) setLoading(false);
      }
    },
    [labelOf],
  );

  useEffect(() => {
    setPage(1);
    setAggDepth(1);
    setHiddenSources(new Set());
    setAggMore(false);
    setCategory("all");
  }, [activeKey]);

  useEffect(() => {
    setPage(1);
    setAggDepth(1);
    setAggMore(false);
  }, [category, debouncedQuery, sort]);

  useEffect(() => {
    load(activeSources, category, page, debouncedQuery, sort, aggDepth);
  }, [activeSources, activeKey, category, page, debouncedQuery, sort, aggDepth, load]);

  const visible = useMemo(() => {
    let list = items.filter((m) => !hiddenSources.has(m.source));
    if (debouncedQuery) {
      const q = debouncedQuery.toLowerCase();
      list = list.filter((m) => m.name.toLowerCase().includes(q));
    }
    if (sort === "name") {
      list = [...list].sort((a, b) => a.name.localeCompare(b.name, "ru"));
    } else if (sort === "updated") {
      list = [...list].sort((a, b) => dateOf(b.published) - dateOf(a.published));
    } else if (sort === "popularity") {
      list = [...list].sort((a, b) => popOf(b) - popOf(a));
    } else if (sort === "size") {
      list = [...list].sort((a, b) => (b.sizeBytes ?? 0) - (a.sizeBytes ?? 0));
    }
    return list;
  }, [items, hiddenSources, debouncedQuery, sort]);

  const toggleHidden = useCallback((id: string) => {
    setHiddenSources((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  return (
    <div className="browser">
      <div className="browser-toolbar">
        <input
          className="search-input"
          placeholder="Поиск по названию…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="source-picker-anchor">
          <button
            className={`btn btn-sm source-picker-trigger ${
              activeSources.length === 0 ? "source-picker-trigger-zero" : ""
            }`}
            aria-haspopup="dialog"
            aria-expanded={pickerOpen}
            onClick={() => setPickerOpen((o) => !o)}
          >
            Источники: {activeSources.length}
          </button>
          <SourcePicker
            open={pickerOpen}
            onClose={() => setPickerOpen(false)}
            onOpenSettings={onOpenSettings}
          />
        </div>
        {single &&
          categories.length > 1 &&
          categoriesDescriptor?.capabilities.categories && (
            <select
              className="category-select"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
            >
              {categories.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.label}
                </option>
              ))}
            </select>
          )}
        {multi && (
          <span className="browser-count">
            {activeSources.length} источника, страниц: {aggDepth} / {AGG_DEPTH_MAX}
          </span>
        )}
        <select
          className="category-select"
          value={sort}
          onChange={(e) => setSort(e.target.value)}
          title="Сортировка"
        >
          {BROWSER_SORTS.map((s) => (
            <option key={s.id} value={s.id}>
              {s.label}
            </option>
          ))}
        </select>
        {!multi && <span className="browser-count">{visible.length} модов</span>}
      </div>

      {activeSources.length === 0 && (
        <div className="banner banner-hint">
          Не выбран ни один источник для поиска. Откройте «Источники» рядом с
          полем поиска или «Настройки → Источники» и включите хотя бы один.
        </div>
      )}

      {multi && (
        <div className="source-chips">
          {activeSources.map((id) => (
            <button
              key={id}
              className={`source-chip ${hiddenSources.has(id) ? "source-chip-off" : ""}`}
              onClick={() => toggleHidden(id)}
              title={
                hiddenSources.has(id)
                  ? "Показать этот источник"
                  : "Скрыть этот источник из результатов"
              }
            >
              {labelOf(id)}
              {hiddenSources.has(id) ? " (скрыт)" : ""}
            </button>
          ))}
        </div>
      )}

      {error && <div className="banner banner-error">{error}</div>}

      {partialErrors.length > 0 && (
        <div className="banner banner-warn">
          Некоторые источники вернули ошибки:
          <ul className="banner-list">
            {partialErrors.map((e, i) => (
              <li key={i}>{e}</li>
            ))}
          </ul>
        </div>
      )}

      {loading && <div className="browser-loading">Загрузка…</div>}

      {!loading &&
        activeSources.length > 0 &&
        visible.length === 0 &&
        !error &&
        partialErrors.length === 0 && <div className="browser-empty">Моды не найдены</div>}

      <div className="mod-grid" data-size={cardSize}>
        {visible.map((item) => (
          <ModCard
            key={item.id}
            item={item}
            installed={installedNames.has(filenameFor(item))}
            similar={
              installedNames.has(filenameFor(item))
                ? undefined
                : findSimilarInstalled(item, installedList)
            }
            dl={downloads[item.id]}
            onInstall={onInstall}
            onInfo={onInfo}
          />
        ))}
      </div>

      {multi && aggMore && (
        <div className="pagination">
          <button
            className="btn"
            disabled={aggDepth >= AGG_DEPTH_MAX}
            onClick={() => setAggDepth((d) => Math.min(AGG_DEPTH_MAX, d + 1))}
          >
            Показать ещё
          </button>
          <span>
            страниц в источнике: {aggDepth} / {AGG_DEPTH_MAX}
          </span>
        </div>
      )}

      {single && totalPages > 1 && (
        <div className="pagination">
          <button
            className="btn"
            disabled={page <= 1}
            onClick={() => setPage((p) => Math.max(1, p - 1))}
          >
            ← Назад
          </button>
          <span>
            {page} / {totalPages}
          </span>
          <button
            className="btn"
            disabled={page >= totalPages}
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
          >
            Вперёд →
          </button>
        </div>
      )}
    </div>
  );
}
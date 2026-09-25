import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCategories, searchMods } from "../api";
import { useSources } from "../SourcesContext";
import { dedupById, resolveActiveSources } from "../sources";
import type { ModItem, SourceCategory } from "../types";
import { useI18n } from "../i18n/LanguageContext";

/** Политика глубины агрегации по нескольким источникам — не «список источников». */
const AGG_DEPTH_MAX = 3;

export function dateOf(published: string | null): number {
  if (!published) return 0;
  const ms = Date.parse(published);
  return Number.isFinite(ms) ? ms : 0;
}

export function popOf(m: ModItem): number {
  const n = Number((m.downloads ?? "").replace(/[^\d]/g, ""));
  return Number.isFinite(n) ? n : 0;
}

/**
 * Общее состояние обзора модов: поиск по источникам (один или агрегация),
 * категории, сортировка, пагинация, скрытие источников. Используется и
 * классическим `ModsBrowser`, и Material-композицией Browse — рендер отдельный,
 * данные/действия общие.
 */
export function useModsBrowser() {
  const { registry, selection, labelOf } = useSources();
  const { t } = useI18n();
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
    const timer = setTimeout(() => setDebouncedQuery(query.trim()), 350);
    return () => clearTimeout(timer);
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
            throw new Error(t("browser_all_sources_down"));
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
    [labelOf, t],
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

  return {
    registry,
    selection,
    activeSources,
    activeKey,
    single: activeSources.length === 1,
    multi: activeSources.length > 1,
    categoriesDescriptor,
    pickerOpen,
    setPickerOpen,
    items,
    totalPages,
    page,
    setPage,
    category,
    setCategory,
    sort,
    setSort,
    categories,
    query,
    setQuery,
    debouncedQuery,
    loading,
    error,
    partialErrors,
    aggDepth,
    setAggDepth,
    aggMore,
    hiddenSources,
    toggleHidden,
    visible,
  };
}

export type ModsBrowserState = ReturnType<typeof useModsBrowser>;
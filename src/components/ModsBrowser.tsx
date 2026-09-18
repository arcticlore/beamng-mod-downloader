import { useCallback, useEffect, useMemo, useState } from "react";
import { getCategories, searchMods } from "../api";
import {
  BROWSER_SORTS,
  findSimilarInstalled,
  installedFileName,
  type DownloadState,
  type ModItem,
  type SourceCategory,
} from "../types";
import { ModCard } from "./ModCard";

const AGGREGATE_SOURCES = ["worldofmods", "beamngweb", "github"];
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
  source: string;
  downloads: Record<string, DownloadState>;
  installedNames: Set<string>;
  installedList: { filename: string; path: string }[];
  cardSize: string;
  onInstall: (item: ModItem) => void;
  onInfo: (item: ModItem) => void;
}

export function ModsBrowser({
  source,
  downloads,
  installedNames,
  installedList,
  cardSize,
  onInstall,
  onInfo,
}: Props) {
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
  const [aggDepth, setAggDepth] = useState(1);
  const [aggMore, setAggMore] = useState(false);

  useEffect(() => {
    getCategories(source).then(setCategories).catch(() => setCategories([]));
  }, [source]);

  useEffect(() => {
    const t = setTimeout(() => setDebouncedQuery(query.trim()), 350);
    return () => clearTimeout(t);
  }, [query]);

  const load = useCallback(
    async (
      src: string,
      cat: string,
      pg: number,
      q: string,
      ord: string,
      depth: number,
    ) => {
      setLoading(true);
      setError(null);
      try {
        if (src === "all") {
          const merged: ModItem[] = [];
          const seen = new Set<string>();
          let anyOk = false;
          let more = false;
          await Promise.all(
            AGGREGATE_SOURCES.map(async (s) => {
              try {
                const pages = Math.max(1, Math.min(depth, AGG_DEPTH_MAX));
                const res = await Promise.all(
                  Array.from({ length: pages }, async (_, i) => {
                    try {
                      return await searchMods(s, q || null, null, i + 1, ord);
                    } catch (e) {
                      console.warn(`источник ${s} (стр. ${i + 1}) недоступен:`, e);
                      return null;
                    }
                  }),
                );
                for (const r of res) {
                  if (!r) continue;
                  anyOk = true;
                  if (r.totalPages > 1) more = true;
                  for (const it of r.items) {
                    if (!seen.has(it.id)) {
                      seen.add(it.id);
                      merged.push(it);
                    }
                  }
                }
              } catch (e) {
                console.warn(`источник ${s} недоступен во вкладке «Все»:`, e);
              }
            }),
          );
          if (!anyOk) throw new Error("все источники сейчас недоступны");
          setItems(merged);
          setAggMore(more);
          setTotalPages(1);
        } else {
          const res = await searchMods(
            src,
            q || null,
            cat === "all" ? null : cat,
            pg,
            ord,
          );
          setItems(res.items);
          setTotalPages(res.totalPages || 1);
        }
      } catch (e) {
        setError(String(e));
        setItems([]);
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    setPage(1);
    setAggDepth(1);
  }, [source, category, debouncedQuery, sort]);

  useEffect(() => {
    load(source, category, page, debouncedQuery, sort, aggDepth);
  }, [source, category, page, debouncedQuery, sort, aggDepth, load]);

  const visible = useMemo(() => {
    let list = items;
    if (debouncedQuery) {
      const q = debouncedQuery.toLowerCase();
      list = list.filter((m) => m.name.toLowerCase().includes(q));
    }
    if (sort === "name") {
      list = [...list].sort((a, b) => a.name.localeCompare(b.name, "ru"));
    } else if (sort === "updated") {
      list = [...list].sort(
        (a, b) => dateOf(b.published) - dateOf(a.published),
      );
    } else if (sort === "popularity") {
      list = [...list].sort((a, b) => popOf(b) - popOf(a));
    } else if (sort === "size") {
      list = [...list].sort((a, b) => (b.sizeBytes ?? 0) - (a.sizeBytes ?? 0));
    }
    return list;
  }, [items, debouncedQuery, sort]);

  return (
    <div className="browser">
      <div className="browser-toolbar">
        <input
          className="search-input"
          placeholder="Поиск по названию…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        {categories.length > 1 && (
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
        <span className="browser-count">{visible.length} модов</span>
      </div>

      {error && <div className="banner banner-error">{error}</div>}

      {loading && <div className="browser-loading">Загрузка…</div>}

      {!loading && visible.length === 0 && !error && (
        <div className="browser-empty">Моды не найдены</div>
      )}

      <div className="mod-grid" data-size={cardSize}>
        {visible.map((item) => (
          <ModCard
            key={item.id}
            item={item}
            installed={installedNames.has(installedFileName(item))}
            similar={
              installedNames.has(installedFileName(item))
                ? undefined
                : findSimilarInstalled(item, installedList)
            }
            dl={downloads[item.id]}
            onInstall={onInstall}
            onInfo={onInfo}
          />
        ))}
      </div>

      {source === "all" && aggMore && (
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

      {source !== "all" && totalPages > 1 && (
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
import { useCallback, useEffect, useMemo, useState } from "react";
import { getCategories, searchMods } from "../api";
import { installedFileName, type DownloadState, type ModItem, type SourceCategory } from "../types";
import { ModCard } from "./ModCard";

interface Props {
  source: string;
  downloads: Record<string, DownloadState>;
  installedNames: Set<string>;
  tokenWarning: boolean;
  onInstall: (item: ModItem) => void;
  onInfo: (item: ModItem) => void;
  onTokenRequest: () => void;
}

export function ModsBrowser({
  source,
  downloads,
  installedNames,
  tokenWarning,
  onInstall,
  onInfo,
  onTokenRequest,
}: Props) {
  const [items, setItems] = useState<ModItem[]>([]);
  const [totalPages, setTotalPages] = useState(1);
  const [page, setPage] = useState(1);
  const [category, setCategory] = useState<string>("all");
  const [categories, setCategories] = useState<SourceCategory[]>([]);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getCategories(source).then(setCategories).catch(() => setCategories([]));
  }, [source]);

  useEffect(() => {
    const t = setTimeout(() => setDebouncedQuery(query.trim()), 350);
    return () => clearTimeout(t);
  }, [query]);

  const load = useCallback(
    async (src: string, cat: string, pg: number, q: string) => {
      setLoading(true);
      setError(null);
      try {
        const res = await searchMods(src, q || null, cat === "all" ? null : cat, pg);
        setItems(res.items);
        setTotalPages(res.totalPages || 1);
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
  }, [source, category, debouncedQuery]);

  useEffect(() => {
    load(source, category, page, debouncedQuery);
  }, [source, category, page, debouncedQuery, load]);

  const visible = useMemo(() => {
    if (!debouncedQuery) return items;
    const q = debouncedQuery.toLowerCase();
    return items.filter((m) => m.name.toLowerCase().includes(q));
  }, [items, debouncedQuery]);

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
        <span className="browser-count">{visible.length} модов</span>
      </div>

      {tokenWarning && source === "beamng" && (
        <div className="banner banner-warn">
          Для репозитория BeamNG нужен токен авторизации.
          <button className="btn btn-sm" onClick={onTokenRequest}>
            Задать токен в настройках
          </button>
        </div>
      )}

      {error && <div className="banner banner-error">{error}</div>}

      {loading && <div className="browser-loading">Загрузка…</div>}

      {!loading && visible.length === 0 && !error && (
        <div className="browser-empty">Моды не найдены</div>
      )}

      <div className="mod-grid">
        {visible.map((item) => (
          <ModCard
            key={item.id}
            item={item}
            installed={installedNames.has(installedFileName(item))}
            dl={downloads[item.id]}
            onInstall={onInstall}
            onInfo={onInfo}
          />
        ))}
      </div>

      {totalPages > 1 && (
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
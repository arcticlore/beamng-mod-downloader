import { useMemo } from "react";
import { useSources } from "../SourcesContext";
import { findSimilarInstalled } from "../types";
import type { DownloadState, ModItem } from "../types";
import { useModsBrowser } from "../hooks/useModsBrowser";
import { useI18n } from "../i18n/LanguageContext";
import { BROWSER_SORTS } from "../types";
import { ModCard } from "./ModCard";
import { SourcePicker } from "./SourcePicker";

interface Props {
  downloads: Record<string, DownloadState>;
  installedNames: Set<string>;
  installedList: { filename: string; path: string }[];
  cardSize: string;
  onInstall: (item: ModItem) => void;
  onInfo: (item: ModItem) => void;
  onOpenSettings: () => void;
  onImportLink: () => void;
}

export function ModsBrowser({
  downloads,
  installedNames,
  installedList,
  cardSize,
  onInstall,
  onInfo,
  onOpenSettings,
  onImportLink,
}: Props) {
  const { filenameFor, labelOf } = useSources();
  const { t, tp } = useI18n();
  const {
    pickerOpen,
    setPickerOpen,
    activeSources,
    single,
    multi,
    categoriesDescriptor,
    page,
    setPage,
    totalPages,
    category,
    setCategory,
    sort,
    setSort,
    categories,
    query,
    setQuery,
    loading,
    error,
    partialErrors,
    aggDepth,
    setAggDepth,
    aggMore,
    hiddenSources,
    toggleHidden,
    visible,
  } = useModsBrowser();

  const categorySelect = useMemo(() => {
    return (
      single &&
      categories.length > 1 &&
      categoriesDescriptor?.capabilities.categories
    );
  }, [single, categories.length, categoriesDescriptor]);

  return (
    <div className="browser">
      <div className="browser-toolbar">
        <input
          className="search-input"
          placeholder={t("search_placeholder")}
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
            {t("browser_sources_label", { n: activeSources.length })}
          </button>
          <SourcePicker
            open={pickerOpen}
            onClose={() => setPickerOpen(false)}
            onOpenSettings={onOpenSettings}
          />
        </div>
        <button
          className="btn btn-sm"
          onClick={onImportLink}
          title={t("link_import_title")}
        >
          + {t("link_import_open")}
        </button>
        {categorySelect && (
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
            {tp("count_sources", activeSources.length)}, {t("browser_pages", { depth: aggDepth, max: 3 })}
          </span>
        )}
        <select
          className="category-select"
          value={sort}
          onChange={(e) => setSort(e.target.value)}
          title={t("browser_sort_title")}
        >
          {BROWSER_SORTS.map((s) => (
            <option key={s.id} value={s.id}>
              {t(s.labelKey)}
            </option>
          ))}
        </select>
        {!multi && (
          <span className="browser-count">{tp("count_mods", visible.length)}</span>
        )}
      </div>

      {activeSources.length === 0 && (
        <div className="banner banner-hint">{t("browser_no_source_banner")}</div>
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
                  ? t("browser_chip_show")
                  : t("browser_chip_hide")
              }
            >
              {labelOf(id)}
              {hiddenSources.has(id) ? t("browser_chip_hidden") : ""}
            </button>
          ))}
        </div>
      )}

      {error && <div className="banner banner-error">{error}</div>}

      {partialErrors.length > 0 && (
        <div className="banner banner-warn">
          {t("browser_partial_errors")}
          <ul className="banner-list">
            {partialErrors.map((e, i) => (
              <li key={i}>{e}</li>
            ))}
          </ul>
        </div>
      )}

      {loading && <div className="browser-loading">{t("browser_loading")}</div>}

      {!loading &&
        activeSources.length > 0 &&
        visible.length === 0 &&
        !error &&
        partialErrors.length === 0 && (
          <div className="browser-empty">{t("browser_empty")}</div>
        )}

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
            disabled={aggDepth >= 3}
            onClick={() => setAggDepth((d) => Math.min(3, d + 1))}
          >
            {t("browser_show_more")}
          </button>
          <span>
            {t("browser_pages_in_source", { depth: aggDepth, max: 3 })}
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
            {t("browser_prev")}
          </button>
          <span>
            {page} / {totalPages}
          </span>
          <button
            className="btn"
            disabled={page >= totalPages}
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
          >
            {t("browser_next")}
          </button>
        </div>
      )}
    </div>
  );
}
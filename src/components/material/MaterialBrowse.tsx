import { useSources } from "../../SourcesContext";
import { useI18n } from "../../i18n/LanguageContext";
import { useModsBrowser } from "../../hooks/useModsBrowser";
import { BROWSER_SORTS } from "../../types";
import type { DownloadState, ModItem } from "../../types";
import { SourcePicker } from "../SourcePicker";
import { findSimilarInstalled } from "../../types";
import { MaterialModCard } from "./MaterialModCard";
import {
  IconChevronDown,
  IconChevronLeft,
  IconChevronRight,
  IconClose,
  IconSearch,
} from "./icons";

interface Props {
  downloads: Record<string, DownloadState>;
  installedNames: Set<string>;
  installedList: { filename: string; path: string }[];
  onInstall: (item: ModItem) => void;
  onInfo: (item: ModItem) => void;
  onOpenSettings: () => void;
}

export function MaterialBrowse({
  downloads,
  installedNames,
  installedList,
  onInstall,
  onInfo,
  onOpenSettings,
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

  const showCategory =
    single && categories.length > 1 && categoriesDescriptor?.capabilities.categories;

  return (
    <div className="m3-page m3-browse">
      <div className="m3-page-head">
        <div className="m3-page-title">
          <h1>{t("m3_browse_title")}</h1>
          <div className="m3-page-sub">
            {multi
              ? t("browser_pages", { depth: aggDepth, max: 3 })
              : tp("count_mods", visible.length)}
          </div>
        </div>
      </div>

      <div className="m3-search">
        <IconSearch size={20} className="m3-search-icon" />
        <input
          className="m3-search-input"
          placeholder={t("search_placeholder")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label={t("search_placeholder")}
        />
        {query && (
          <button
            className="m3-iconbtn m3-search-clear"
            onClick={() => setQuery("")}
            title={t("m3_clear_search")}
            aria-label={t("m3_clear_search")}
          >
            <IconClose size={18} />
          </button>
        )}
      </div>

      <div className="m3-filter-row">
        <div className="m3-source-picker-anchor">
          <button
            className={`m3-chip ${activeSources.length === 0 ? "m3-chip-alert" : ""}`}
            aria-haspopup="dialog"
            aria-expanded={pickerOpen}
            onClick={() => setPickerOpen((o) => !o)}
          >
            {t("browser_sources_label", { n: activeSources.length })}
            <IconChevronDown size={16} />
          </button>
          <SourcePicker
            open={pickerOpen}
            onClose={() => setPickerOpen(false)}
            onOpenSettings={onOpenSettings}
          />
        </div>

        {showCategory && (
          <label className="m3-select">
            <select
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              aria-label={t("filter_category")}
            >
              {categories.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.label}
                </option>
              ))}
            </select>
            <IconChevronDown size={16} />
          </label>
        )}

        <label className="m3-select">
          <select
            value={sort}
            onChange={(e) => setSort(e.target.value)}
            aria-label={t("browser_sort_title")}
          >
            {BROWSER_SORTS.map((s) => (
              <option key={s.id} value={s.id}>
                {t(s.labelKey)}
              </option>
            ))}
          </select>
          <IconChevronDown size={16} />
        </label>

        <span className="m3-filter-spacer" />
      </div>

      {multi && (
        <div className="m3-chips">
          {activeSources.map((id) => (
            <button
              key={id}
              className={`m3-chip m3-chip-filter ${
                hiddenSources.has(id) ? "m3-chip-off" : ""
              }`}
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

      {activeSources.length === 0 && (
        <div className="m3-banner m3-banner-hint">{t("browser_no_source_banner")}</div>
      )}

      {error && <div className="m3-banner m3-banner-error">{error}</div>}

      {partialErrors.length > 0 && (
        <div className="m3-banner m3-banner-warn">
          {t("browser_partial_errors")}
          <ul className="m3-banner-list">
            {partialErrors.map((e, i) => (
              <li key={i}>{e}</li>
            ))}
          </ul>
        </div>
      )}

      {loading && (
        <div className="m3-loading">
          <span className="m3-spinner" />
          {t("browser_loading")}
        </div>
      )}

      {!loading && activeSources.length > 0 && visible.length === 0 && !error && (
        <div className="m3-empty">{t("browser_empty")}</div>
      )}

      <div className="m3-mod-grid">
        {visible.map((item) => (
          <MaterialModCard
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
        <div className="m3-pagination">
          <button
            className="m3-btn m3-btn-text"
            disabled={aggDepth >= 3}
            onClick={() => setAggDepth((d) => Math.min(3, d + 1))}
          >
            {t("browser_show_more")}
          </button>
          <span className="m3-page-indicator">
            {t("browser_pages_in_source", { depth: aggDepth, max: 3 })}
          </span>
        </div>
      )}

      {single && totalPages > 1 && (
        <div className="m3-pagination">
          <button
            className="m3-iconbtn"
            disabled={page <= 1}
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            aria-label={t("browser_prev")}
          >
            <IconChevronLeft size={20} />
          </button>
          <span className="m3-page-indicator">
            {page} / {totalPages}
          </span>
          <button
            className="m3-iconbtn"
            disabled={page >= totalPages}
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            aria-label={t("browser_next")}
          >
            <IconChevronRight size={20} />
          </button>
        </div>
      )}
    </div>
  );
}
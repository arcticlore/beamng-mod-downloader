import { useMemo } from "react";
import type { AppSettings, IntegrityReport } from "../../types";
import { useI18n } from "../../i18n/LanguageContext";
import { formatBytes } from "../../types";
import { useInstalled } from "../../hooks/useInstalled";
import { IconInstalled, IconRefresh, IconShield, IconTrash, IconUpdate } from "./icons";

interface Props {
  settings: AppSettings | null;
  onOpenSettings: () => void;
}

function IntegrityPill({ report }: { report: IntegrityReport }) {
  const { t } = useI18n();
  const bad = !report.zipOk || report.hashOk === false;
  const title = bad
    ? report.hashOk === false
      ? t("integrity_bad_modified")
      : (report.error ?? t("integrity_bad_archive"))
    : t("integrity_ok_title");
  return (
    <span
      className={`m3-badge ${bad ? "m3-badge-error" : "m3-badge-ok"}`}
      title={title}
    >
      <IconShield size={13} />
      {bad ? t("integrity_bad_label") : t("integrity_ok_label")}
    </span>
  );
}

export function MaterialInstalled({ settings, onOpenSettings }: Props) {
  const { t, tp, lang } = useI18n();
  const {
    items,
    error,
    updates,
    checking,
    integrity,
    verifying,
    updating,
    refresh,
    runIntegrity,
    doUpdate,
    runUpdateCheck,
    remove,
    managedCount,
    updateCount,
  } = useInstalled(settings);

  let integritySummary: string | null = null;
  if (integrity) {
    const list = [...integrity.values()];
    const fine = list.filter(
      (r) => r.zipOk && (r.hashOk === true || r.hashOk == null),
    ).length;
    integritySummary = t("installed_integrity_summary", { fine, total: list.length });
  }

  const total = items ? items.reduce((acc, m) => acc + m.sizeBytes, 0) : 0;
  const sortKey = settings?.installedSort ?? "date";
  const sorted = useMemo(() => {
    if (!items) return [];
    return [...items].sort((a, b) => {
      if (sortKey === "name") return a.filename.localeCompare(b.filename);
      if (sortKey === "size") return b.sizeBytes - a.sizeBytes;
      return b.modified - a.modified;
    });
  }, [items, sortKey]);

  if (error && items?.length === 0) {
    return (
      <div className="m3-page">
        <div className="m3-empty">
          <div className="m3-empty-icon">
            <IconInstalled size={40} />
          </div>
          <p>{error}</p>
          <button className="m3-btn m3-btn-primary" onClick={onOpenSettings}>
            {t("installed_select_folder")}
          </button>
        </div>
      </div>
    );
  }

  if (items === null) {
    return (
      <div className="m3-page">
        <div className="m3-loading">
          <span className="m3-spinner" />
          {t("installed_loading")}
        </div>
      </div>
    );
  }

  return (
    <div className="m3-page m3-installed">
      <div className="m3-page-head">
        <div className="m3-page-title">
          <h1>{t("m3_installed_title")}</h1>
          <div className="m3-page-sub">
            {tp("archives_count", items.length)}
            <span className="m3-meta-sep">·</span>
            {formatBytes(total, lang)}
            <span className="m3-meta-sep">·</span>
            {t("installed_manual_count", { n: items.length - managedCount })}
            {integritySummary && (
              <>
                <span className="m3-meta-sep">·</span>
                {integritySummary}
              </>
            )}
          </div>
        </div>
        <div className="m3-page-actions">
          <button
            className="m3-btn m3-btn-text"
            disabled={checking || items.length === 0}
            onClick={() => runUpdateCheck(items)}
            title={t("updates_check_title")}
          >
            <IconUpdate size={18} />
            {checking
              ? t("updates_checking")
              : updates.size
                ? t("updates_count", { n: updateCount })
                : t("updates_check")}
          </button>
          <button
            className="m3-btn m3-btn-text"
            disabled={verifying || items.length === 0}
            onClick={runIntegrity}
            title={t("integrity_check_title")}
          >
            {verifying ? t("integrity_checking") : t("integrity_check")}
          </button>
          <button className="m3-iconbtn" onClick={refresh} title={t("installed_refresh")} aria-label={t("installed_refresh")}>
            <IconRefresh size={20} />
          </button>
        </div>
      </div>

      {items.length === 0 ? (
        <div className="m3-empty">
          <div className="m3-empty-icon">
            <IconInstalled size={40} />
          </div>
          <p>{t("installed_empty")}</p>
          <button className="m3-btn m3-btn-primary" onClick={onOpenSettings}>
            {t("installed_change_folder")}
          </button>
        </div>
      ) : (
        <div className="m3-installed-list">
          {sorted.map((m) => {
            const up = updates.get(m.filename);
            const dateStr = new Date(m.modified * 1000).toLocaleDateString(
              lang === "en" ? "en-US" : "ru-RU",
            );
            const sourceLabel = m.source === "repo"
              ? t("src_repo")
              : m.key
                ? t("src_launcher")
                : t("src_manual");
            return (
              <div className="m3-row" key={m.path}>
                <div className="m3-row-leading">
                  <IconInstalled size={24} />
                </div>
                <div className="m3-row-body">
                  <div className="m3-row-title" title={m.path}>
                    {m.filename}
                  </div>
                  <div className="m3-row-sub">
                    <span className={`m3-badge ${m.source === "repo" ? "m3-badge-repo" : "m3-badge-outline"}`}>
                      {sourceLabel}
                    </span>
                    <span>{formatBytes(m.sizeBytes, lang)}</span>
                    <span className="m3-meta-sep">·</span>
                    <span>{t("m3_installed_updated_on", { date: dateStr })}</span>
                    {integrity?.get(m.filename) && (
                      <IntegrityPill report={integrity.get(m.filename)!} />
                    )}
                  </div>
                </div>
                <div className="m3-row-version">
                  {m.key ? (
                    up?.hasUpdate ? (
                      <span
                        className="m3-badge m3-badge-warn"
                        title={t("update_title_newer", { date: up.latestPublished ?? "?" })}
                      >
                        {t("has_update")}
                      </span>
                    ) : (
                      <span className="m3-muted">
                        {(up?.latestPublished ?? m.published)?.slice(0, 10) ?? "—"}
                      </span>
                    )
                  ) : (
                    <span className="m3-muted">—</span>
                  )}
                </div>
                <div className="m3-row-actions">
                  {m.key && up?.hasUpdate && (
                    <button
                      className="m3-btn m3-btn-primary"
                      disabled={updating.has(m.filename)}
                      onClick={() => doUpdate(m)}
                    >
                      <IconUpdate size={16} />
                      {updating.has(m.filename) ? t("updating") : t("update_btn")}
                    </button>
                  )}
                  <button
                    className="m3-iconbtn m3-iconbtn-danger"
                    onClick={() => remove(m)}
                    title={t("delete_btn")}
                    aria-label={t("delete_btn")}
                  >
                    <IconTrash size={18} />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
import { useMemo } from "react";
import type { AppSettings, IntegrityReport } from "../types";
import { formatBytes } from "../types";
import { useI18n } from "../i18n/LanguageContext";
import { useInstalled } from "../hooks/useInstalled";

function IntegrityBadge({ report }: { report: IntegrityReport }) {
  const { t } = useI18n();
  const bad = !report.zipOk || report.hashOk === false;
  const title = bad
    ? report.hashOk === false
      ? t("integrity_bad_modified")
      : (report.error ?? t("integrity_bad_archive"))
    : t("integrity_ok_title");
  return (
    <span className={`badge ${bad ? "badge-error" : "badge-ok"}`} title={title}>
      {bad ? t("integrity_bad_label") : t("integrity_ok_label")}
    </span>
  );
}

export function InstalledPanel({
  onOpenSettings,
  settings,
}: {
  onOpenSettings: () => void;
  settings: AppSettings | null;
}) {
  const { t, tp, lang } = useI18n();
  const {
    items,
    error,
    collapsed,
    setCollapsed,
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
    integritySummary = t("installed_integrity_summary", {
      fine,
      total: list.length,
    });
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
      <div className="panel-empty">
        <p>{error}</p>
        <button className="btn btn-primary" onClick={onOpenSettings}>
          {t("installed_select_folder")}
        </button>
      </div>
    );
  }

  if (items === null)
    return <div className="browser-loading">{t("installed_loading")}</div>;

  if (items.length === 0) {
    return (
      <div className="panel-empty">
        <p>{t("installed_empty")}</p>
        <button className="btn btn-primary" onClick={onOpenSettings}>
          {t("installed_change_folder")}
        </button>
      </div>
    );
  }

  return (
    <div className="installed">
      <div className="installed-summary">
        {tp("archives_count", items.length)} · {formatBytes(total, lang)} ·{" "}
        {t("installed_manual_count", { n: items.length - managedCount })}
        {integritySummary && <span className="text-muted"> · {integritySummary}</span>}
        <button className="btn btn-sm" onClick={() => setCollapsed((v) => !v)}>
          {collapsed ? t("installed_expand") : t("installed_collapse")}
        </button>
        {!collapsed && (
          <button
            className="btn btn-sm"
            disabled={checking}
            onClick={() => runUpdateCheck(items)}
            title={t("updates_check_title")}
          >
            {checking
              ? t("updates_checking")
              : updates.size
                ? t("updates_count", { n: updateCount })
                : t("updates_check")}
          </button>
        )}
        {!collapsed && (
          <button
            className="btn btn-sm"
            disabled={verifying}
            onClick={runIntegrity}
            title={t("integrity_check_title")}
          >
            {verifying ? t("integrity_checking") : t("integrity_check")}
          </button>
        )}
        {!collapsed && (
          <button className="btn btn-sm" onClick={refresh}>
            {t("installed_refresh")}
          </button>
        )}
      </div>
      {!collapsed && (
        <table className="installed-table">
          <thead>
            <tr>
              <th>{t("col_file")}</th>
              <th>{t("col_source")}</th>
              <th>{t("col_size")}</th>
              <th>{t("col_modified")}</th>
              <th>{t("col_version")}</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((m) => {
              const up = updates.get(m.filename);
              return (
                <tr key={m.path}>
                  <td title={m.path}>{m.filename}</td>
                  <td>
                    <span className={`badge ${m.source === "repo" ? "badge-repo" : "badge-local"}`}>
                      {m.source === "repo"
                        ? t("src_repo")
                        : m.key
                          ? t("src_launcher")
                          : t("src_manual")}
                    </span>
                  </td>
                  <td>{formatBytes(m.sizeBytes, lang)}</td>
                  <td>{new Date(m.modified * 1000).toLocaleDateString(lang === "en" ? "en-US" : "ru-RU")}</td>
                  <td>
                    {m.key ? (
                      up?.hasUpdate ? (
                        <span
                          className="badge badge-ok"
                          title={t("update_title_newer", {
                            date: up.latestPublished ?? "?",
                          })}
                        >
                          {t("has_update")}
                        </span>
                      ) : (
                        <span className="text-muted">
                          {(up?.latestPublished ?? m.published)?.slice(0, 10) ?? "—"}
                        </span>
                      )
                    ) : (
                      <span className="text-muted">—</span>
                    )}
                    {integrity?.get(m.filename) && (
                      <IntegrityBadge report={integrity.get(m.filename)!} />
                    )}
                  </td>
                  <td>
                    {m.key && up?.hasUpdate && (
                      <button
                        className="btn btn-primary btn-sm"
                        disabled={updating.has(m.filename)}
                        onClick={() => doUpdate(m)}
                      >
                        {updating.has(m.filename) ? t("updating") : t("update_btn")}
                      </button>
                    )}
                    <button className="btn btn-danger btn-sm" onClick={() => remove(m)}>
                      {t("delete_btn")}
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}
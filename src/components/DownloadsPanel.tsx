import { useMemo } from "react";
import type { DownloadState } from "../types";
import { formatBytes, formatSpeed } from "../types";
import type { MessageKey } from "../i18n";
import { useI18n } from "../i18n/LanguageContext";
import { ProgressBar } from "./ProgressBar";

interface Props {
  downloads: Record<string, DownloadState>;
  onClearFinished: () => void;
  onCancel: (key: string) => void;
}

function formatEta(
  remaining: number,
  speedBps: number,
  units: { h: string; m: string; s: string },
): string | null {
  if (speedBps <= 0 || remaining <= 0) return null;
  const secs = Math.ceil(remaining / speedBps);
  if (secs >= 3600)
    return `${Math.floor(secs / 3600)} ${units.h} ${Math.floor((secs % 3600) / 60)} ${units.m}`;
  if (secs >= 60)
    return `${Math.floor(secs / 60)} ${units.m} ${secs % 60} ${units.s}`;
  return `${secs} ${units.s}`;
}

const ETA_KEYS: { h: MessageKey; m: MessageKey; s: MessageKey } = {
  h: "eta_hours",
  m: "eta_minutes",
  s: "eta_seconds",
};

const ACTIVE_ONLY = ["downloading", "error"];

export function DownloadsPanel({ downloads, onClearFinished, onCancel }: Props) {
  const { t, lang } = useI18n();
  const list = useMemo(() => {
    return Object.values(downloads).sort(
      (a, b) => ACTIVE_ONLY.indexOf(a.state) - ACTIVE_ONLY.indexOf(b.state),
    );
  }, [downloads]);

  const activeCount = list.filter((d) => d.state === "downloading").length;
  const doneCount = list.filter((d) => d.state === "done").length;
  const errCount = list.filter((d) => d.state === "error").length;

  if (list.length === 0) {
    return (
      <div className="panel-empty">
        <p>{t("downloads_empty")}</p>
      </div>
    );
  }

  const totalBytes = list.reduce(
    (acc, d) => acc + (d.state === "downloading" ? d.received : d.total ?? 0),
    0,
  );

  const units = {
    h: t(ETA_KEYS.h),
    m: t(ETA_KEYS.m),
    s: t(ETA_KEYS.s),
  };

  return (
    <div className="downloads">
      <div className="downloads-summary">
        <span>
          {t("nav_downloads")} · {t("downloads_active", { n: activeCount })} ·{" "}
          {t("downloads_done", { n: doneCount })}
          {errCount > 0 && ` · ${t("downloads_errors", { n: errCount })}`}
        </span>
        <span className="downloads-total">{formatBytes(totalBytes, lang)}</span>
        <button className="btn btn-sm" onClick={onClearFinished}>
          {t("downloads_clear_finished")}
        </button>
      </div>
      <ul className="downloads-list">
        {list.map((d) => (
          <li
            key={d.key}
            className={`download-row ${d.state === "downloading" ? "download-row-active" : ""}`}
          >
            <div className="download-main">
              <span className="download-name" title={d.filename}>
                {d.name || d.filename}
              </span>
              {d.state === "downloading" && d.total ? (
                <span className="download-eta">
                  {t("downloads_eta_remaining", {
                    eta:
                      formatEta(d.total - d.received, d.speedBps, units) ?? "…",
                  })}{" "}
                  · {formatBytes(d.received, lang)} / {formatBytes(d.total, lang)}
                </span>
              ) : null}
              {d.state !== "downloading" && (
                <span className="download-size">
                  {formatBytes(d.total ?? d.received, lang)}
                </span>
              )}
            </div>
            <div className="download-progress">
              {d.state === "downloading" && (
                <ProgressBar received={d.received} total={d.total} />
              )}
              {d.state === "downloading" && (
                <span className="download-speed">
                  {formatSpeed(d.speedBps, lang)}
                </span>
              )}
              {d.state === "downloading" && (
                <button className="btn btn-sm btn-danger" onClick={() => onCancel(d.key)}>
                  {t("dl_cancel")}
                </button>
              )}
              {d.state === "done" && (
                <span className="badge badge-ok">{t("dl_badge_done")}</span>
              )}
              {d.state === "error" && (
                <span className="badge badge-error" title={d.error ?? ""}>
                  {t("dl_badge_error")}
                </span>
              )}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
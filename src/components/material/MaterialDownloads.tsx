import { useMemo } from "react";
import type { DownloadState } from "../../types";
import type { MessageKey } from "../../i18n";
import { useI18n } from "../../i18n/LanguageContext";
import { formatBytes, formatSpeed } from "../../types";
import { formatEta } from "../../format";
import { IconCancel, IconDone, IconDownload, IconError } from "./icons";

interface Props {
  downloads: Record<string, DownloadState>;
  onClearFinished: () => void;
  onCancel: (key: string) => void;
}

const ACTIVE_ONLY = ["downloading", "error"];

const ETA_KEYS: { h: MessageKey; m: MessageKey; s: MessageKey } = {
  h: "eta_hours",
  m: "eta_minutes",
  s: "eta_seconds",
};

function M3Progress({
  received,
  total,
}: {
  received: number;
  total: number | null;
}) {
  const pct = total && total > 0 ? Math.min(100, (received / total) * 100) : 0;
  return (
    <div className="m3-progress" role="progressbar" aria-valuenow={Math.round(pct)}>
      <div className="m3-progress-fill" style={{ width: `${pct.toFixed(1)}%` }} />
    </div>
  );
}

export function MaterialDownloads({ downloads, onClearFinished, onCancel }: Props) {
  const { t, lang } = useI18n();
  const list = useMemo(() => {
    return Object.values(downloads).sort(
      (a, b) => ACTIVE_ONLY.indexOf(a.state) - ACTIVE_ONLY.indexOf(b.state),
    );
  }, [downloads]);

  const activeCount = list.filter((d) => d.state === "downloading").length;
  const doneCount = list.filter((d) => d.state === "done").length;
  const errCount = list.filter((d) => d.state === "error").length;
  const hasFinished = doneCount + errCount > 0;

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
    <div className="m3-page m3-downloads">
      <div className="m3-page-head">
        <div className="m3-page-title">
          <h1>{t("m3_downloads_title")}</h1>
          <div className="m3-page-sub">
            {t("downloads_active", { n: activeCount })}
            <span className="m3-meta-sep">·</span>
            {t("downloads_done", { n: doneCount })}
            {errCount > 0 && (
              <>
                <span className="m3-meta-sep">·</span>
                <span className="m3-text-error">{t("downloads_errors", { n: errCount })}</span>
              </>
            )}
            <span className="m3-meta-sep">·</span>
            <span>{formatBytes(totalBytes, lang)}</span>
          </div>
        </div>
        {hasFinished && (
          <button className="m3-btn m3-btn-text" onClick={onClearFinished}>
            {t("downloads_clear_finished")}
          </button>
        )}
      </div>

      {list.length === 0 ? (
        <div className="m3-empty">
          <div className="m3-empty-icon">
            <IconDownload size={40} />
          </div>
          <p>{t("downloads_empty")}</p>
        </div>
      ) : (
        <div className="m3-dl-list">
          {list.map((d) => {
            const downloading = d.state === "downloading";
            return (
              <div
                key={d.key}
                className={`m3-dl-card ${downloading ? "m3-dl-card-active" : ""}`}
              >
                <div className="m3-dl-leading">
                  {d.state === "downloading" && <IconDownload size={24} className="m3-dl-pulse" />}
                  {d.state === "done" && <IconDone size={24} />}
                  {d.state === "error" && <IconError size={24} className="m3-text-error" />}
                </div>
                <div className="m3-dl-body">
                  <div className="m3-dl-title-row">
                    <span className="m3-dl-name" title={d.filename}>
                      {d.name || d.filename}
                    </span>
                    {d.state === "downloading" ? (
                      <span className="m3-dl-pct">
                        {d.total ? `${Math.min(100, Math.round((d.received / d.total) * 100))}%` : "…"}
                      </span>
                    ) : null}
                  </div>
                  <div className="m3-dl-file">{d.filename}</div>
                  {downloading && (
                    <div className="m3-dl-progress-row">
                      <M3Progress received={d.received} total={d.total} />
                    </div>
                  )}
                  <div className="m3-dl-meta">
                    {downloading && (
                      <>
                        <span>
                          {formatBytes(d.received, lang)}
                          {d.total ? ` / ${formatBytes(d.total, lang)}` : ""}
                        </span>
                        <span className="m3-meta-sep">·</span>
                        <span>{formatSpeed(d.speedBps, lang)}</span>
                        {d.total && (
                          <>
                            <span className="m3-meta-sep">·</span>
                            <span>
                              {t("downloads_eta_remaining", {
                                eta:
                                  formatEta(d.total - d.received, d.speedBps, units) ?? "…",
                              })}
                            </span>
                          </>
                        )}
                      </>
                    )}
                    {d.state === "done" && (
                      <span className="m3-badge m3-badge-ok">{t("dl_badge_done")}</span>
                    )}
                    {d.state === "error" && (
                      <span className="m3-badge m3-badge-error" title={d.error ?? ""}>
                        {t("dl_badge_error")}
                      </span>
                    )}
                  </div>
                </div>
                <div className="m3-dl-actions">
                  {downloading ? (
                    <button
                      className="m3-iconbtn m3-iconbtn-danger"
                      onClick={() => onCancel(d.key)}
                      title={t("dl_cancel")}
                      aria-label={t("dl_cancel")}
                    >
                      <IconCancel size={20} />
                    </button>
                  ) : null}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
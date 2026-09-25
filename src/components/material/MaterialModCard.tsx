import { useMemo } from "react";
import { useSources } from "../../SourcesContext";
import { installModeLabel, trustLabel } from "../../sources";
import { useI18n } from "../../i18n/LanguageContext";
import type { DownloadState, ModItem } from "../../types";
import { formatBytes, formatSpeed } from "../../types";
import { IconDownload, IconInfo, IconModPlaceholder, IconShield } from "./icons";

interface Props {
  item: ModItem;
  installed: boolean;
  similar?: { filename: string; path: string } | null;
  dl: DownloadState | undefined;
  onInstall: (item: ModItem) => void;
  onInfo: (item: ModItem) => void;
}

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

export function MaterialModCard({ item, installed, similar, dl, onInstall, onInfo }: Props) {
  const { labelOf, descriptorOf } = useSources();
  const { t, lang } = useI18n();
  const descriptor = descriptorOf(item.source);
  const installMode = descriptor?.installMode ?? "mods_zip";
  const autoInstall = installMode === "mods_zip";
  const active = dl?.state === "downloading" || dl?.state === "error" ? dl : undefined;
  const disabled = !!active || installed || !autoInstall;
  const trustKey = descriptor ? trustLabel(descriptor.trustLevel) : null;

  const meta = useMemo(() => {
    const parts: string[] = [];
    if (item.author) parts.push(item.author);
    if (item.downloads) parts.push(`↓ ${item.downloads}`);
    if (item.author || item.downloads) parts.push("·");
    if (item.category) parts.push(item.category);
    if (item.sizeBytes) parts.push(`· ${formatBytes(item.sizeBytes, lang)}`);
    return parts.join(" ");
  }, [item, lang]);

  const status = similar
    ? { cls: "m3-status-warn", text: t("modcard_similar_warning", { file: similar.filename }) }
    : installed
      ? { cls: "m3-status-ok", text: t("modcard_installed") }
      : dl?.state === "error"
        ? { cls: "m3-status-error", text: dl.error ?? t("modcard_error_btn") }
        : null;

  const installLabel = !autoInstall
    ? t("modcard_manual_install")
    : installed
      ? t("modcard_installed_btn")
      : active
        ? t("modcard_error_btn")
        : similar
          ? t("modcard_similar_btn")
          : t("modcard_install_btn");

  return (
    <article className={`m3-card ${installed ? "m3-card-installed" : ""}`}>
      <div className="m3-card-top">
        <div className="m3-card-thumb">
          {item.thumbnail ? (
            <img src={item.thumbnail} alt={item.name} loading="lazy" />
          ) : (
            <IconModPlaceholder size={26} className="m3-card-thumb-ph" />
          )}
        </div>
        <div className="m3-card-meta">
          <h3 className="m3-card-title" title={item.name}>
            {item.name}
          </h3>
          <div className="m3-card-sub meta">
            {labelOf(item.source)}
            {meta ? <span className="m3-meta-sep">· {meta}</span> : null}
          </div>
        </div>
        {trustKey && (
          <span
            className={`m3-badge m3-badge-outline trust-${descriptor!.trustLevel}`}
            title={t("trust_badge_title", { level: t(trustKey) })}
          >
            <IconShield size={13} />
            {t(trustKey)}
          </span>
        )}
        <button
          className="m3-iconbtn"
          onClick={() => onInfo(item)}
          title={t("modcard_details")}
          aria-label={t("modcard_details")}
        >
          <IconInfo size={20} />
        </button>
      </div>

      {item.description && (
        <p className="m3-card-desc">{item.description.slice(0, 190)}</p>
      )}

      {status && <div className={`m3-status ${status.cls}`}>{status.text}</div>}

      <div className="m3-card-actions">
        {active?.state === "downloading" ? (
          <div className="m3-dl">
            <M3Progress received={active.received} total={active.total} />
            <div className="m3-dl-label">
              {formatBytes(active.received, lang)}
              {active.total ? ` / ${formatBytes(active.total, lang)}` : ""}
              <span className="m3-meta-sep">·</span>
              {formatSpeed(active.speedBps, lang)}
            </div>
          </div>
        ) : (
          <button
            className="m3-btn m3-btn-primary"
            disabled={disabled}
            onClick={() => onInstall(item)}
            title={
              similar
                ? t("modcard_similar_install_title")
                : !autoInstall
                  ? t(installModeLabel(installMode))
                  : undefined
            }
          >
            <IconDownload size={18} />
            {installLabel}
          </button>
        )}
      </div>
    </article>
  );
}
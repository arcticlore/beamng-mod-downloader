import { useMemo } from "react";
import { useSources } from "../SourcesContext";
import { installModeLabel, trustLabel } from "../sources";
import { useI18n } from "../i18n/LanguageContext";
import type { DownloadState, ModItem } from "../types";
import { formatBytes, formatSpeed } from "../types";
import { ProgressBar } from "./ProgressBar";

interface Props {
  item: ModItem;
  installed: boolean;
  similar?: { filename: string; path: string } | null;
  dl: DownloadState | undefined;
  onInstall: (item: ModItem) => void;
  onInfo: (item: ModItem) => void;
}

function installedTail(dl: DownloadState | undefined) {
  if (dl?.state === "downloading") return dl;
  if (dl?.state === "error") return dl;
  return undefined;
}

export function ModCard({ item, installed, similar, dl, onInstall, onInfo }: Props) {
  const { labelOf, descriptorOf } = useSources();
  const { t, lang } = useI18n();
  const active = installedTail(dl);
  const descriptor = descriptorOf(item.source);
  const installMode = descriptor?.installMode ?? "mods_zip";
  const autoInstall = installMode === "mods_zip";
  const disabled = !!active || installed || !autoInstall;

  const meta = useMemo(() => {
    const parts: string[] = [];
    if (item.author) parts.push(item.author);
    if (item.downloads) parts.push(`↓ ${item.downloads}`);
    if (item.category) parts.push(item.category);
    if (item.sizeBytes) parts.push(formatBytes(item.sizeBytes, lang));
    return parts.join(" · ");
  }, [item, lang]);

  const warning = similar
    ? t("modcard_similar_warning", { file: similar.filename })
    : installed
      ? t("modcard_installed")
      : undefined;

  const trustKey = descriptor ? trustLabel(descriptor.trustLevel) : null;

  return (
    <div className={`mod-card ${installed ? "mod-card-installed" : ""}`}>
      <div className="mod-thumb">
        {item.thumbnail ? (
          <img src={item.thumbnail} alt={item.name} loading="lazy" />
        ) : (
          <div className="mod-thumb-placeholder">?</div>
        )}
        <span className="mod-source">
          {labelOf(item.source)}
        </span>
      </div>
      <div className="mod-body">
        <div className="mod-name-row">
          <h3 className="mod-name" title={item.name}>
            {item.name}
          </h3>
          {trustKey && (
            <span
              className={`trust-badge trust-${descriptor!.trustLevel}`}
              title={t("trust_badge_title", { level: t(trustKey) })}
            >
              {t(trustKey)}
            </span>
          )}
        </div>
        {warning && <div className="mod-warning">{warning}</div>}
        {meta && <div className="mod-meta">{meta}</div>}
        {item.description && (
          <p className="mod-desc">{item.description.slice(0, 190)}</p>
        )}
      </div>
      <div className="mod-actions">
        {active?.state === "downloading" ? (
          <div className="mod-dl">
            <ProgressBar
              received={active.received}
              total={active.total}
            />
            <div className="mod-dl-label">
              {formatBytes(active.received, lang)}
              {active.total ? ` / ${formatBytes(active.total, lang)}` : ""}
              {" · "}
              {formatSpeed(active.speedBps, lang)}
            </div>
          </div>
        ) : (
          <div className="mod-actions-btns">
            <button
              className="btn"
              onClick={() => onInfo(item)}
              title={t("modcard_details")}
            >
              {t("modcard_details")}
            </button>
            <button
              className="btn btn-primary"
              disabled={disabled}
              onClick={() => onInstall(item)}
              title={
                !autoInstall
                  ? t(installModeLabel(installMode))
                  : installed
                    ? t("modcard_installed")
                    : active?.state === "error"
                      ? (active.error ?? t("modcard_error_title"))
                      : similar
                        ? t("modcard_similar_install_title")
                        : t("modcard_install_title")
              }
            >
              {!autoInstall
                ? t("modcard_manual_install")
                : installed
                  ? t("modcard_installed_btn")
                  : active
                    ? t("modcard_error_btn")
                    : similar
                      ? t("modcard_similar_btn")
                      : t("modcard_install_btn")}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
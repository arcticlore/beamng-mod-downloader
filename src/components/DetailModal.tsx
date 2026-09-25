import { useEffect, useState } from "react";
import { getModDetail } from "../api";
import { useSources } from "../SourcesContext";
import { installModeLabel, trustLabel, urlHost } from "../sources";
import { useI18n } from "../i18n/LanguageContext";
import type { DownloadState, ModDetail, ModItem } from "../types";
import { formatBytes } from "../types";

interface Props {
  item: ModItem;
  dl: DownloadState | undefined;
  installed: boolean;
  similar?: { filename: string; path: string } | null;
  onInstall: (item: ModItem) => void;
  onClose: () => void;
}

export function DetailModal({ item, dl, installed, similar, onInstall, onClose }: Props) {
  const { labelOf, descriptorOf, filenameFor } = useSources();
  const { t, lang } = useI18n();
  const [detail, setDetail] = useState<ModDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [imgIndex, setImgIndex] = useState(0);

  const descriptor = descriptorOf(item.source);
  const autoInstall = descriptor?.installMode === "mods_zip";
  const modeLabel = descriptor ? installModeLabel(descriptor.installMode) : null;
  const host = urlHost(item.key);

  useEffect(() => {
    let cancelled = false;
    setDetail(null);
    setError(null);
    setImgIndex(0);
    getModDetail(item.source, item.id, item.key)
      .then((d) => {
        if (!cancelled) setDetail(d);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [item.id, item.key, item.source]);

  const images = detail?.screenshots && detail.screenshots.length > 0 ? detail.screenshots : [];
  const currentImage = images[imgIndex > 0 ? imgIndex : 0] ?? detail?.item.thumbnail ?? item.thumbnail;
  const downloading = dl?.state === "downloading";

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <button className="modal-close" onClick={onClose}>✕</button>
        <div className="modal-content">
          <div className="modal-gallery">
            {currentImage ? (
              <img src={currentImage} alt={item.name} />
            ) : (
              <div className="mod-thumb-placeholder big">?</div>
            )}
            {images.length > 1 && (
              <div className="gallery-thumbs">
                {images.map((src, i) => (
                  <img
                    key={src}
                    src={src}
                    className={i === imgIndex ? "gallery-thumb active" : "gallery-thumb"}
                    onClick={() => setImgIndex(i)}
                    alt=""
                  />
                ))}
              </div>
            )}
          </div>
<div className="modal-info">
              <div className="mod-source">
                {labelOf(item.source)}
                {descriptor && (
                  <span
                    className={`trust-badge trust-${descriptor.trustLevel}`}
                    title={t("trust_badge_title", { level: t(trustLabel(descriptor.trustLevel)) })}
                  >
                    {t(trustLabel(descriptor.trustLevel))}
                  </span>
                )}
                {descriptor && descriptor.installMode !== "mods_zip" && (
                  <span
                    className="install-mode-badge"
                    title={modeLabel ? t(modeLabel) : undefined}
                  >
                    {modeLabel ? t(modeLabel) : null}
                  </span>
                )}
              </div>
              <h2>{item.name}</h2>
              {!installed && similar && (
                <div className="mod-warning">
                  {t("detail_similar_warning", { file: similar.filename })}
                </div>
              )}
              {item.author && <div className="mod-meta">{t("detail_author", { author: item.author })}</div>}
            {item.downloads && <div className="mod-meta">{t("detail_downloads", { downloads: item.downloads })}</div>}
            {item.sizeBytes ? (
              <div className="mod-meta">{t("detail_size", { size: formatBytes(item.sizeBytes, lang) })}</div>
            ) : null}
            {item.published && <div className="mod-meta">{t("detail_date", { date: item.published })}</div>}
            {host && <div className="mod-meta">{t("detail_domain", { host })}</div>}
            <div className="mod-full-desc">
              {detail ? (
                detail.fullDescription || detail.item.description || t("detail_no_desc")
              ) : error ? (
                <span className="error-text">{t("detail_load_error", { err: error })}</span>
              ) : (
                t("detail_loading_desc")
              )}
            </div>
            {dl?.state === "error" && <div className="error-text">{dl.error}</div>}
            <div className="modal-actions">
              <button
                className="btn btn-primary btn-lg"
                disabled={installed || downloading || !autoInstall}
                onClick={() => onInstall(item)}
                title={modeLabel ? t(modeLabel) : undefined}
              >
                {!autoInstall
                  ? t("detail_manual_install")
                  : installed
                    ? t("detail_installed")
                    : downloading
                      ? t("detail_downloading")
                      : t("detail_install")}
              </button>
              <span className="hint-file">{filenameFor(item)}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
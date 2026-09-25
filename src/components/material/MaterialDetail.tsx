import { useEffect, useState } from "react";
import { getModDetail } from "../../api";
import { useSources } from "../../SourcesContext";
import { installModeLabel, trustLabel, urlHost } from "../../sources";
import { useI18n } from "../../i18n/LanguageContext";
import type { DownloadState, ModDetail, ModItem } from "../../types";
import { formatBytes } from "../../types";
import { IconClose, IconDownload, IconExternal, IconShield } from "./icons";

interface Props {
  item: ModItem;
  dl: DownloadState | undefined;
  installed: boolean;
  similar?: { filename: string; path: string } | null;
  onInstall: (item: ModItem) => void;
  onClose: () => void;
}

export function MaterialDetail({ item, dl, installed, similar, onInstall, onClose }: Props) {
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

  const images =
    detail?.screenshots && detail.screenshots.length > 0 ? detail.screenshots : [];
  const currentImage =
    images[imgIndex >= 0 && imgIndex < images.length ? imgIndex : 0] ??
    detail?.item.thumbnail ??
    item.thumbnail;
  const downloading = dl?.state === "downloading";
  const trustKey = descriptor ? trustLabel(descriptor.trustLevel) : null;

  return (
    <div className="m3-dialog-backdrop" onClick={onClose}>
      <div
        className="m3-dialog m3-detail-dialog"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="m3-detail-hero">
          {currentImage ? (
            <img className="m3-detail-hero-img" src={currentImage} alt={item.name} />
          ) : (
            <div className="m3-detail-hero-ph" />
          )}
          <button
            className="m3-iconbtn m3-dialog-close"
            onClick={onClose}
            aria-label={t("m3_detail_close")}
            title={t("close")}
          >
            <IconClose size={22} />
          </button>
          {images.length > 1 && (
            <div className="m3-gallery-thumbs">
              {images.map((src, i) => (
                <button
                  key={src}
                  type="button"
                  className={`m3-gallery-thumb ${i === imgIndex ? "m3-gallery-thumb-active" : ""}`}
                  onClick={() => setImgIndex(i)}
                >
                  <img src={src} alt="" />
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="m3-detail-body">
          <div className="m3-detail-badges">
            <span className="m3-badge m3-badge-outline">{labelOf(item.source)}</span>
            {trustKey && descriptor && (
              <span
                className={`m3-badge m3-badge-outline trust-${descriptor.trustLevel}`}
                title={t("trust_badge_title", { level: t(trustKey) })}
              >
                <IconShield size={13} />
                {t(trustKey)}
              </span>
            )}
            {descriptor && descriptor.installMode !== "mods_zip" && modeLabel && (
              <span className="m3-badge m3-badge-outline">{t(modeLabel)}</span>
            )}
            {host && (
              <span className="m3-badge m3-badge-outline">
                <IconExternal size={13} />
                {host}
              </span>
            )}
          </div>

          <h2 className="m3-detail-title">{item.name}</h2>

          <div className="m3-detail-meta">
            {item.author && <span>{t("detail_author", { author: item.author })}</span>}
            {item.downloads && (
              <span>{t("detail_downloads", { downloads: item.downloads })}</span>
            )}
            {item.sizeBytes ? (
              <span>{t("detail_size", { size: formatBytes(item.sizeBytes, lang) })}</span>
            ) : null}
            {item.published && (
              <span>{t("detail_date", { date: item.published })}</span>
            )}
          </div>

          {!installed && similar && (
            <div className="m3-banner m3-banner-warn">
              {t("detail_similar_warning", { file: similar.filename })}
            </div>
          )}

          <div className="m3-detail-desc">
            {detail ? (
              detail.fullDescription || detail.item.description || t("detail_no_desc")
            ) : error ? (
              <span className="m3-text-error">{t("detail_load_error", { err: error })}</span>
            ) : (
              t("detail_loading_desc")
            )}
          </div>

          {dl?.state === "error" && <p className="m3-text-error">{dl.error}</p>}

          <footer className="m3-detail-actions">
            <span className="m3-hint-file">{filenameFor(item)}</span>
            <button
              type="button"
              className="m3-btn m3-btn-primary m3-btn-lg"
              disabled={installed || downloading || !autoInstall}
              onClick={() => onInstall(item)}
              title={modeLabel ? t(modeLabel) : undefined}
            >
              <IconDownload size={20} />
              {!autoInstall
                ? t("detail_manual_install")
                : installed
                  ? t("detail_installed")
                  : downloading
                    ? t("detail_downloading")
                    : t("detail_install")}
            </button>
          </footer>
        </div>
      </div>
    </div>
  );
}
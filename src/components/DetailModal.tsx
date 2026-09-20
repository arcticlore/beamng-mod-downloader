import { useEffect, useState } from "react";
import { getModDetail } from "../api";
import { useSources } from "../SourcesContext";
import { installModeLabel, trustLabel, urlHost } from "../sources";
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
                    title={`Доверие: ${trustLabel(descriptor.trustLevel)}`}
                  >
                    {trustLabel(descriptor.trustLevel)}
                  </span>
                )}
                {descriptor && descriptor.installMode !== "mods_zip" && (
                  <span className="install-mode-badge" title={modeLabel ?? undefined}>
                    {modeLabel}
                  </span>
                )}
              </div>
              <h2>{item.name}</h2>
              {!installed && similar && (
                <div className="mod-warning">
                  Похожий мод уже установлен как «{similar.filename}». Если это не обновление —
                  установка создаст второй экземпляр мода.
                </div>
              )}
              {item.author && <div className="mod-meta">автор: {item.author}</div>}
            {item.downloads && <div className="mod-meta">скачиваний: {item.downloads}</div>}
            {item.sizeBytes ? <div className="mod-meta">размер: {formatBytes(item.sizeBytes)}</div> : null}
            {item.published && <div className="mod-meta">дата: {item.published}</div>}
            {host && <div className="mod-meta">домен: {host}</div>}
            <div className="mod-full-desc">
              {detail ? (
                detail.fullDescription || detail.item.description || "Описание отсутствует."
              ) : error ? (
                <span className="error-text">Не удалось загрузить описание: {error}</span>
              ) : (
                "Загрузка описания…"
              )}
            </div>
            {dl?.state === "error" && <div className="error-text">{dl.error}</div>}
            <div className="modal-actions">
              <button
                className="btn btn-primary btn-lg"
                disabled={installed || downloading || !autoInstall}
                onClick={() => onInstall(item)}
                title={modeLabel ?? undefined}
              >
                {!autoInstall
                  ? "Установка вручную"
                  : installed
                    ? "✓ Установлено"
                    : downloading
                      ? "Загрузка…"
                      : "Скачать и установить"}
              </button>
              <span className="hint-file">{filenameFor(item)}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
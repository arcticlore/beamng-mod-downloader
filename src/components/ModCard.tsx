import { useMemo } from "react";
import { useSources } from "../SourcesContext";
import { installModeLabel, trustLabel } from "../sources";
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
    if (item.sizeBytes) parts.push(formatBytes(item.sizeBytes));
    return parts.join(" · ");
  }, [item]);

  const warning = similar
    ? `Похожий мод уже установлен: ${similar.filename}`
    : installed
      ? "Уже в папке модов"
      : undefined;

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
          {descriptor && (
            <span
              className={`trust-badge trust-${descriptor.trustLevel}`}
              title={`Доверие: ${trustLabel(descriptor.trustLevel)}`}
            >
              {trustLabel(descriptor.trustLevel)}
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
              {formatBytes(active.received)}
              {active.total ? ` / ${formatBytes(active.total)}` : ""}
              {" · "}
              {formatSpeed(active.speedBps)}
            </div>
          </div>
        ) : (
          <div className="mod-actions-btns">
            <button
              className="btn"
              onClick={() => onInfo(item)}
              title="Подробнее"
            >
              Подробнее
            </button>
            <button
              className="btn btn-primary"
              disabled={disabled}
              onClick={() => onInstall(item)}
              title={
                !autoInstall
                  ? installModeLabel(installMode)
                  : installed
                    ? "Уже в папке модов"
                    : active?.state === "error"
                      ? active.error ?? "Ошибка загрузки"
                      : similar
                        ? "Похожий мод уже установлен — установка запросит подтверждение"
                        : "Скачать и установить"
              }
            >
              {!autoInstall
                ? "Установка вручную"
                : installed
                  ? "✓ Установлено"
                  : active
                    ? "Ошибка"
                    : similar
                      ? "Есть похожий"
                      : "Установить"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
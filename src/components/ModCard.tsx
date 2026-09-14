import { useMemo } from "react";
import type { DownloadState, ModItem } from "../types";
import { formatBytes, formatSpeed, SOURCES } from "../types";
import { ProgressBar } from "./ProgressBar";

interface Props {
  item: ModItem;
  installed: boolean;
  dl: DownloadState | undefined;
  onInstall: (item: ModItem) => void;
  onInfo: (item: ModItem) => void;
}

function installedTail(dl: DownloadState | undefined) {
  if (dl?.state === "downloading") return dl;
  if (dl?.state === "error") return dl;
  return undefined;
}

export function ModCard({ item, installed, dl, onInstall, onInfo }: Props) {
  const active = installedTail(dl);
  const disabled = !!active || installed;

  const meta = useMemo(() => {
    const parts: string[] = [];
    if (item.author) parts.push(item.author);
    if (item.downloads) parts.push(`↓ ${item.downloads}`);
    if (item.category) parts.push(item.category);
    if (item.sizeBytes) parts.push(formatBytes(item.sizeBytes));
    return parts.join(" · ");
  }, [item]);

  return (
    <div className={`mod-card ${installed ? "mod-card-installed" : ""}`}>
      <div className="mod-thumb">
        {item.thumbnail ? (
          <img src={item.thumbnail} alt={item.name} loading="lazy" />
        ) : (
          <div className="mod-thumb-placeholder">?</div>
        )}
        <span className="mod-source">
          {SOURCES[item.source]?.label ?? item.source}
        </span>
      </div>
      <div className="mod-body">
        <h3 className="mod-name" title={item.name}>
          {item.name}
        </h3>
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
                installed
                  ? "Уже в папке модов"
                  : active?.state === "error"
                    ? active.error ?? "Ошибка загрузки"
                    : "Скачать и установить"
              }
            >
              {installed ? "✓ Установлено" : active ? "Ошибка" : "Установить"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
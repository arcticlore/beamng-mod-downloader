import { useMemo } from "react";
import type { DownloadState } from "../types";
import { formatBytes, formatSpeed } from "../types";
import { ProgressBar } from "./ProgressBar";

interface Props {
  downloads: Record<string, DownloadState>;
  onClearFinished: () => void;
  onCancel: (key: string) => void;
}

function formatEta(remaining: number, speedBps: number): string | null {
  if (speedBps <= 0 || remaining <= 0) return null;
  const secs = Math.ceil(remaining / speedBps);
  if (secs >= 3600) return `${Math.floor(secs / 3600)} ч ${Math.floor((secs % 3600) / 60)} мин`;
  if (secs >= 60) return `${Math.floor(secs / 60)} мин ${secs % 60} с`;
  return `${secs} с`;
}

const ACTIVE_ONLY = ["downloading", "error"];

export function DownloadsPanel({ downloads, onClearFinished, onCancel }: Props) {
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
        <p>
          Загрузок пока нет. Начните скачивание, нажав «Установить» на карточке мода.
        </p>
      </div>
    );
  }

  const totalBytes = list.reduce(
    (acc, d) => acc + (d.state === "downloading" ? d.received : d.total ?? 0),
    0,
  );

  return (
    <div className="downloads">
      <div className="downloads-summary">
        <span>
          Загрузки · активно: {activeCount} · готово: {doneCount}
          {errCount > 0 && ` · ошибки: ${errCount}`}
        </span>
        <span className="downloads-total">{formatBytes(totalBytes)}</span>
        <button className="btn btn-sm" onClick={onClearFinished}>
          Очистить завершённые
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
                  осталось {formatEta(d.total - d.received, d.speedBps) ?? "…"} ·{" "}
                  {formatBytes(d.received)} / {formatBytes(d.total)}
                </span>
              ) : null}
              {d.state !== "downloading" && (
                <span className="download-size">{formatBytes(d.total ?? d.received)}</span>
              )}
            </div>
            <div className="download-progress">
              {d.state === "downloading" && (
                <ProgressBar received={d.received} total={d.total} />
              )}
              {d.state === "downloading" && (
                <span className="download-speed">{formatSpeed(d.speedBps)}</span>
              )}
              {d.state === "downloading" && (
                <button className="btn btn-sm btn-danger" onClick={() => onCancel(d.key)}>
                  Отмена
                </button>
              )}
              {d.state === "done" && (
                <span className="badge badge-ok">Готово</span>
              )}
              {d.state === "error" && (
                <span className="badge badge-error" title={d.error ?? ""}>
                  Ошибка
                </span>
              )}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
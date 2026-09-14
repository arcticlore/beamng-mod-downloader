import { useEffect, useState } from "react";
import { listInstalled, removeInstalled } from "../api";
import type { InstalledMod } from "../types";
import { formatBytes } from "../types";

export function InstalledPanel({ onOpenSettings }: { onOpenSettings: () => void }) {
  const [items, setItems] = useState<InstalledMod[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    listInstalled()
      .then(setItems)
      .catch((e) => {
        setError(String(e));
        setItems([]);
      });
  };

  useEffect(() => {
    refresh();
  }, []);

  if (error && items?.length === 0) {
    return (
      <div className="panel-empty">
        <p>{error}</p>
        <button className="btn btn-primary" onClick={onOpenSettings}>
          Выбрать папку с модами
        </button>
      </div>
    );
  }

  if (items === null) return <div className="browser-loading">Загрузка…</div>;

  if (items.length === 0) {
    return (
      <div className="panel-empty">
        <p>В папке модов пока ничего нет. Выберите моды в вкладках выше и установите.</p>
        <button className="btn btn-primary" onClick={onOpenSettings}>
          Изменить папку модов
        </button>
      </div>
    );
  }

  const total = items.reduce((acc, m) => acc + m.sizeBytes, 0);
  const sorted = [...items].sort((a, b) => b.modified - a.modified);

  return (
    <div className="installed">
      <div className="installed-summary">
        Установлено архивов: {items.length} · всего {formatBytes(total)}
        <button className="btn btn-sm" onClick={refresh}>
          Обновить
        </button>
      </div>
      <table className="installed-table">
        <thead>
          <tr>
            <th>Файл</th>
            <th>Источник</th>
            <th>Размер</th>
            <th>Изменён</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((m) => (
            <tr key={m.path}>
              <td title={m.path}>{m.filename}</td>
              <td>
                <span className={`badge ${m.source === "repo" ? "badge-repo" : "badge-local"}`}>
                  {m.source === "repo" ? "репо" : "локально"}
                </span>
              </td>
              <td>{formatBytes(m.sizeBytes)}</td>
              <td>{new Date(m.modified * 1000).toLocaleDateString("ru-RU")}</td>
              <td>
                <button
                  className="btn btn-danger btn-sm"
                  onClick={async () => {
                    try {
                      await removeInstalled(m.filename);
                      refresh();
                    } catch (e) {
                      alert(String(e));
                    }
                  }}
                >
                  Удалить
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
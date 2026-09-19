import { useEffect, useMemo, useState } from "react";
import {
  checkUpdates,
  listInstalled,
  removeInstalled,
  updateMod,
  verifyInstalled,
} from "../api";
import type { AppSettings, InstalledMod, IntegrityReport, ModUpdate } from "../types";
import { formatBytes } from "../types";

function IntegrityBadge({ report }: { report: IntegrityReport }) {
  const bad = !report.zipOk || report.hashOk === false;
  const title = bad
    ? report.hashOk === false
      ? "Файл изменён или заменён после установки лаунчером"
      : report.error ?? "Архив повреждён или усечён"
    : "Целостность подтверждена";
  return (
    <span className={`badge ${bad ? "badge-error" : "badge-ok"}`} title={title}>
      {bad ? "Целостность нарушена" : "OK"}
    </span>
  );
}

export function InstalledPanel({
  onOpenSettings,
  settings,
}: {
  onOpenSettings: () => void;
  settings: AppSettings | null;
}) {
  const [items, setItems] = useState<InstalledMod[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState(
    () => settings?.installedCollapsed ?? false,
  );
  const [updates, setUpdates] = useState<Map<string, ModUpdate>>(new Map());
  const [checking, setChecking] = useState(false);
  const [integrity, setIntegrity] = useState<Map<string, IntegrityReport> | null>(null);
  const [verifying, setVerifying] = useState(false);
  const [updating, setUpdating] = useState<Set<string>>(new Set());

  const refresh = () => {
    listInstalled()
      .then((list) => {
        setItems(list);
        setError(null);
      })
      .catch((e) => {
        setError(String(e));
        setItems([]);
      });
  };

  const runIntegrity = async () => {
    setVerifying(true);
    try {
      const res = await verifyInstalled();
      setIntegrity(new Map(res.map((r) => [r.filename, r])));
    } catch (e) {
      setError(String(e));
    } finally {
      setVerifying(false);
    }
  };

  const doUpdate = async (m: InstalledMod) => {
    setUpdating((prev) => new Set(prev).add(m.filename));
    try {
      await updateMod(m.filename);
    } catch (e) {
      alert(String(e));
    } finally {
      setUpdating((prev) => {
        const next = new Set(prev);
        next.delete(m.filename);
        return next;
      });
    }
    // Событие download::finished приходит асинхронно — даём файлу замениться.
    window.setTimeout(refresh, 3000);
  };

  const runUpdateCheck = async (list: InstalledMod[]) => {
    const managed = list.filter((m) => m.key);
    if (managed.length === 0) return;
    setChecking(true);
    try {
      const res = await checkUpdates(managed);
      setUpdates(new Map(res.map((u) => [u.filename, u])));
    } catch (e) {
      setError(String(e));
    } finally {
      setChecking(false);
    }
  };

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (items && items.length > 0) runUpdateCheck(items);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items]);

  const managedCount = useMemo(
    () => items?.filter((m) => m.key).length ?? 0,
    [items],
  );
  const updateCount = useMemo(
    () => [...updates.values()].filter((u) => u.hasUpdate).length,
    [updates],
  );
  let integritySummary: string | null = null;
  if (integrity) {
    const list = [...integrity.values()];
    const fine = list.filter(
      (r) => r.zipOk && (r.hashOk === true || r.hashOk == null),
    ).length;
    integritySummary = `Целостность: ${fine}/${list.length}`;
  }

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
  const sortKey = settings?.installedSort ?? "date";
  const sorted = [...items].sort((a, b) => {
    if (sortKey === "name") return a.filename.localeCompare(b.filename);
    if (sortKey === "size") return b.sizeBytes - a.sizeBytes;
    return b.modified - a.modified;
  });

  return (
    <div className="installed">
      <div className="installed-summary">
        Архивов: {items.length} · {formatBytes(total)} · вручную:
        {items.length - managedCount}
        {integritySummary && <span className="text-muted"> · {integritySummary}</span>}
        <button className="btn btn-sm" onClick={() => setCollapsed((v) => !v)}>
          {collapsed ? "Развернуть" : "Свернуть"}
        </button>
        {!collapsed && (
          <button
            className="btn btn-sm"
            disabled={checking}
            onClick={() => runUpdateCheck(items ?? [])}
            title="Сравнить версии установленных модов с источниками"
          >
            {checking ? "Проверяю…" : updates.size ? `Обновления: ${updateCount}` : "Проверить обновления"}
          </button>
        )}
        {!collapsed && (
          <button
            className="btn btn-sm"
            disabled={verifying}
            onClick={runIntegrity}
            title="Проверить zip-структуру и SHA-256 установленных архивов"
          >
            {verifying ? "Проверяю…" : "Проверить целостность"}
          </button>
        )}
        {!collapsed && (
          <button className="btn btn-sm" onClick={refresh}>
            Обновить
          </button>
        )}
      </div>
      {!collapsed && (
        <table className="installed-table">
          <thead>
            <tr>
              <th>Файл</th>
              <th>Источник</th>
              <th>Размер</th>
              <th>Изменён</th>
              <th>Версия</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((m) => {
              const up = updates.get(m.filename);
              return (
                <tr key={m.path}>
                  <td title={m.path}>{m.filename}</td>
                  <td>
                    <span className={`badge ${m.source === "repo" ? "badge-repo" : "badge-local"}`}>
                      {m.source === "repo" ? "репо" : m.key ? "лаунчер" : "вручную"}
                    </span>
                  </td>
                  <td>{formatBytes(m.sizeBytes)}</td>
                  <td>{new Date(m.modified * 1000).toLocaleDateString("ru-RU")}</td>
                  <td>
                    {m.key ? (
                      up?.hasUpdate ? (
                        <span className="badge badge-ok" title={`На сайте новее: ${up.latestPublished ?? "?"}`}>
                          Есть обновление ↓
                        </span>
                      ) : (
                        <span className="text-muted">
                          {(up?.latestPublished ?? m.published)?.slice(0, 10) ?? "—"}
                        </span>
                      )
                    ) : (
                      <span className="text-muted">—</span>
                    )}
                    {integrity?.get(m.filename) && (
                      <IntegrityBadge report={integrity.get(m.filename)!} />
                    )}
                  </td>
                  <td>
                    {m.key && up?.hasUpdate && (
                      <button
                        className="btn btn-primary btn-sm"
                        disabled={updating.has(m.filename)}
                        onClick={() => doUpdate(m)}
                      >
                        {updating.has(m.filename) ? "Обновляю…" : "Обновить"}
                      </button>
                    )}
                    <button
                      className="btn btn-danger btn-sm"
                      onClick={async () => {
                        try {
                          await removeInstalled(m.filename);
                          setUpdates((prev) => {
                            const next = new Map(prev);
                            next.delete(m.filename);
                            return next;
                          });
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
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}
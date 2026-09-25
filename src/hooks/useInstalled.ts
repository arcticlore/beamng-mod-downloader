import { useCallback, useEffect, useMemo, useState } from "react";
import {
  checkUpdates,
  listInstalled,
  removeInstalled,
  updateMod,
  verifyInstalled,
} from "../api";
import type { AppSettings, InstalledMod, IntegrityReport, ModUpdate } from "../types";

/**
 * Общее состояние экрана «Установленные»: список, проверка обновлений,
 * целостность, удаление. Классический `InstalledPanel` и Material-композиция
 * используют одни и те же данные/действия; рендер раздельный.
 */
export function useInstalled(settings: AppSettings | null) {
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

  const refresh = useCallback(() => {
    listInstalled()
      .then((list) => {
        setItems(list);
        setError(null);
      })
      .catch((e) => {
        setError(String(e));
        setItems([]);
      });
  }, []);

  const runIntegrity = useCallback(async () => {
    setVerifying(true);
    try {
      const res = await verifyInstalled();
      setIntegrity(new Map(res.map((r) => [r.filename, r])));
    } catch (e) {
      setError(String(e));
    } finally {
      setVerifying(false);
    }
  }, []);

  const doUpdate = useCallback(
    async (m: InstalledMod) => {
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
    },
    [refresh],
  );

  const runUpdateCheck = useCallback(
    async (list: InstalledMod[]) => {
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
    },
    [],
  );

  const remove = useCallback(
    async (m: InstalledMod) => {
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
    },
    [refresh],
  );

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    if (items && items.length > 0) runUpdateCheck(items);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items, runUpdateCheck]);

  const managedCount = useMemo(
    () => items?.filter((m) => m.key).length ?? 0,
    [items],
  );
  const updateCount = useMemo(
    () => [...updates.values()].filter((u) => u.hasUpdate).length,
    [updates],
  );

  return {
    items,
    error,
    collapsed,
    setCollapsed,
    updates,
    checking,
    integrity,
    verifying,
    updating,
    refresh,
    runIntegrity,
    doUpdate,
    runUpdateCheck,
    remove,
    managedCount,
    updateCount,
  };
}
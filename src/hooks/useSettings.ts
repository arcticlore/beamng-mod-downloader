import { useEffect, useMemo, useState } from "react";
import {
  detectModsFolders,
  getModsFolder,
  getSettings,
  openLogDir,
  setModsFolder,
  setModsFolderForce,
  setSettings,
} from "../api";
import { applyAppearance } from "../theme";
import { useSources } from "../SourcesContext";
import {
  applyPreset,
  canonicalizeSelection,
  searchCapableEnabledIds,
  sortByGroup,
  toggleSourceSelection,
} from "../sources";
import { useI18n } from "../i18n/LanguageContext";
import type { AppSettings, SourceGroup } from "../types";

export const GROUPS: SourceGroup[] = ["official", "forges", "community", "custom"];

/**
 * Контроллер настроек: папка модов, внешний вид, источники, логи — общий для
 * классического `SettingsModal` и Material-диалога настроек. Рендер раздельный.
 */
export function useSettings(onChanged: () => void) {
  const { t } = useI18n();
  const [current, setCurrent] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<Array<{ path: string; exists: boolean }>>([]);
  const [manual, setManual] = useState("");
  const [settings, setSettingsState] = useState<AppSettings>({});
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const folder = await getModsFolder();
      if (cancelled) return;
      setCurrent(folder);
      setManual(folder ?? "");
      try {
        const cands = await detectModsFolders();
        if (!cancelled) setCandidates(cands);
      } catch {
        if (!cancelled) setCandidates([]);
      }
      const s = await getSettings();
      if (!cancelled) setSettingsState(s);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const showMsg = (ok: boolean, text: string) => setMsg({ ok, text });

  const saveSettings = async (next: AppSettings, quiet = false) => {
    setSettingsState(next);
    applyAppearance(next);
    try {
      await setSettings(next);
      onChanged();
      if (!quiet) showMsg(true, t("saved_settings"));
    } catch (e) {
      showMsg(false, String(e));
    }
  };

  const applyFolder = async (path: string, force: boolean) => {
    setBusy(true);
    setMsg(null);
    try {
      if (force) {
        await setModsFolderForce(path);
      } else {
        await setModsFolder(path);
      }
      setCurrent(path);
      setManual(path);
      showMsg(true, t("saved_folder"));
      onChanged();
    } catch (e) {
      showMsg(false, String(e));
    } finally {
      setBusy(false);
    }
  };

  const openLogs = async () => {
    setMsg(null);
    try {
      await openLogDir();
    } catch (e) {
      showMsg(false, String(e));
    }
  };

  const {
    registry,
    selection,
    loading: sourcesLoading,
    error: sourcesError,
    setEnabled,
    setSelected,
    resetDefaults,
  } = useSources();
  const [sourcesBusy, setSourcesBusy] = useState(false);

  const enabledSet = useMemo(
    () => new Set(selection?.enabled ?? []),
    [selection],
  );
  const selectedIsAll = selection?.selected === null;
  const searchCapable = useMemo(
    () => searchCapableEnabledIds(registry, selection?.enabled ?? []),
    [registry, selection],
  );
  const selectedSet = useMemo(() => {
    if (!selection) return new Set<string>();
    if (selection.selected === null) return new Set(searchCapable);
    return new Set(selection.selected);
  }, [selection, searchCapable]);
  const anyEnabled = enabledSet.size > 0;

  const runSource = async (fn: () => Promise<void>, okText: string) => {
    setSourcesBusy(true);
    try {
      await fn();
      showMsg(true, okText);
    } catch (e) {
      showMsg(false, String(e));
    } finally {
      setSourcesBusy(false);
    }
  };

  const toggleEnabled = (id: string) =>
    runSource(() => setEnabled(id, !enabledSet.has(id)), t("msg_sources_saved"));

  const toggleSelected = (id: string) =>
    runSource(
      () =>
        setSelected(
          toggleSourceSelection(
            registry,
            selection?.enabled ?? [],
            selection?.selected ?? null,
            id,
          ),
        ),
      t("msg_search_updated"),
    );

  const applyPresetNow = (
    preset: "recommended" | "official_forges" | "all_configured" | "clear",
  ) => {
    const list = applyPreset(preset, registry, [...enabledSet]);
    const next = canonicalizeSelection(searchCapable, list);
    return runSource(() => setSelected(next), t("msg_preset_applied"));
  };

  const resetDefaultsNow = () =>
    runSource(resetDefaults, t("msg_sources_reset"));

  const groups = useMemo(() => {
    const ordered = sortByGroup(registry);
    const m = new Map<SourceGroup, typeof ordered>();
    for (const d of ordered) {
      const arr = m.get(d.group) ?? [];
      arr.push(d);
      m.set(d.group, arr);
    }
    return m;
  }, [registry]);

  return {
    folder: {
      current,
      candidates,
      manual,
      setManual,
      busy,
      applyFolder,
    },
    appearance: {
      settings,
      saveSettings,
    },
    logs: {
      openLogs,
    },
    sources: {
      sourcesLoading,
      sourcesError,
      sourcesBusy,
      enabledSet,
      selectedSet,
      selectedIsAll,
      anyEnabled,
      searchCapable,
      toggleEnabled,
      toggleSelected,
      applyPresetNow,
      resetDefaultsNow,
      groups,
    },
    msg,
    showMsg,
  };
}

export type SettingsController = ReturnType<typeof useSettings>;
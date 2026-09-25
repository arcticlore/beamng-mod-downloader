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
import { applyAppearance, ACCENT_PRESETS } from "../theme";
import { useSources } from "../SourcesContext";
import {
  applyPreset,
  canonicalizeSelection,
  groupLabel,
  searchCapableEnabledIds,
  sortByGroup,
  statusLabel,
  toggleSourceSelection,
  trustLabel,
} from "../sources";
import { useI18n } from "../i18n/LanguageContext";
import type { AppSettings, ModsFolderCandidate, SourceGroup } from "../types";
import { CARD_SIZES, INSTALLED_SORTS, STYLES, THEMES } from "../types";

const GROUPS: SourceGroup[] = ["official", "forges", "community", "custom"];

const LANGUAGES: { id: "ru" | "en"; labelKey: "language_ru" | "language_en" }[] = [
  { id: "ru", labelKey: "language_ru" },
  { id: "en", labelKey: "language_en" },
];

interface Props {
  onClose: () => void;
  onChanged: () => void;
}

export function SettingsModal({ onClose, onChanged }: Props) {
  const { t } = useI18n();
  const [current, setCurrent] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<ModsFolderCandidate[]>([]);
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

  const groups = useMemo(() => {
    const ordered = sortByGroup(registry);
    const m = new Map<string, typeof ordered>();
    for (const d of ordered) {
      const arr = m.get(d.group) ?? [];
      arr.push(d);
      m.set(d.group, arr);
    }
    return m;
  }, [registry]);

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal modal-settings" onClick={(e) => e.stopPropagation()}>
        <button className="modal-close" onClick={onClose}>✕</button>
        <h2>{t("settings_title")}</h2>

        <section>
          <h3>{t("folder_section_title")}</h3>
          {current && (
            <div className="current-path">
              {t("folder_current")} <code>{current}</code>
            </div>
          )}
          <p className="hint">{t("folder_hint")}</p>
          {candidates.length > 0 ? (
            <div className="candidate-list">
              {candidates.map((c) => (
                <button
                  key={c.path}
                  className={`candidate ${current === c.path ? "candidate-active" : ""}`}
                  onClick={() => {
                    setManual(c.path);
                    applyFolder(c.path, false);
                  }}
                >
                  {c.path}
                </button>
              ))}
            </div>
          ) : (
            <div className="hint">{t("folder_none_found")}</div>
          )}
          <div className="manual-row">
            <input
              className="search-input"
              placeholder="/путь/к/.../mods"
              value={manual}
              onChange={(e) => setManual(e.target.value)}
            />
            <button
              className="btn btn-primary"
              disabled={busy || !manual.trim()}
              onClick={() => applyFolder(manual.trim(), false)}
            >
              {t("folder_apply")}
            </button>
            <button
              className="btn"
              disabled={busy || !manual.trim()}
              title={t("folder_force_title")}
              onClick={() => applyFolder(manual.trim(), true)}
            >
              {t("folder_force")}
            </button>
          </div>
        </section>

        <section>
          <h3>{t("sources_section_title")}</h3>
          <p className="hint" dangerouslySetInnerHTML={{ __html: t("sources_hint") }} />

          {sourcesLoading && <div className="hint">{t("sources_loading")}</div>}
          {!sourcesLoading && sourcesError && (
            <div className="banner banner-error">{sourcesError}</div>
          )}

          {!sourcesLoading && !sourcesError && (
            <>
              <div className="source-presets">
                <button
                  className="btn btn-sm"
                  disabled={!anyEnabled || sourcesBusy}
                  title={t("preset_recommended_title")}
                  onClick={() => applyPresetNow("recommended")}
                >
                  {t("preset_recommended")}
                </button>
                <button
                  className="btn btn-sm"
                  disabled={!anyEnabled || sourcesBusy}
                  onClick={() => applyPresetNow("official_forges")}
                >
                  {t("preset_official_forges")}
                </button>
                <button
                  className="btn btn-sm"
                  disabled={!anyEnabled || sourcesBusy}
                  onClick={() => applyPresetNow("all_configured")}
                >
                  {t("preset_all_configured")}
                </button>
                <button
                  className="btn btn-sm"
                  disabled={!anyEnabled || sourcesBusy}
                  onClick={() => applyPresetNow("clear")}
                >
                  {t("preset_clear")}
                </button>
                <button
                  className="btn btn-sm"
                  disabled={sourcesBusy}
                  title={t("preset_reset_defaults_title")}
                  onClick={() =>
                    runSource(resetDefaults, t("msg_sources_reset"))
                  }
                >
                  {t("preset_reset_defaults")}
                </button>
              </div>

              {GROUPS.map(
                (g) =>
                  groups.has(g) && (
                    <div key={g} className="source-group">
                      <div className="source-group-label">{t(groupLabel(g))}</div>
                      {groups.get(g)!.map((d) => {
                        const isEnabled = enabledSet.has(d.id);
                        const isSelected = selectedSet.has(d.id);
                        const trustKey = trustLabel(d.trustLevel);
                        return (
                          <div
                            key={d.id}
                            className={`source-item ${isEnabled ? "" : "source-item-off"}`}
                          >
                            <div className="source-item-meta">
                              <span className="source-item-name">{d.label}</span>
                              <span
                                className={`trust-badge trust-${d.trustLevel}`}
                                title={t("trust_badge_title", { level: t(trustKey) })}
                              >
                                {t(trustKey)}
                              </span>
                              {d.status !== "ready" && (
                                <span className="source-status-badge">
                                  {t(statusLabel(d.status))}
                                </span>
                              )}
                            </div>
                            <div className="source-item-controls">
                              <label
                                className="switch-label"
                                title={
                                  isEnabled
                                    ? t("toggle_enable_on_title")
                                    : t("toggle_enable_off_title")
                                }
                              >
                                <input
                                  type="checkbox"
                                  checked={isEnabled}
                                  className="switch-input"
                                  disabled={sourcesBusy}
                                  onChange={() => toggleEnabled(d.id)}
                                />
                                <span className="switch-box" />
                                <span>{t("source_enabled")}</span>
                              </label>
                              <label
                                className="switch-label switch-label-secondary"
                                title={
                                  !d.capabilities.search
                                    ? t("toggle_search_unsupported_title")
                                    : isEnabled
                                      ? t("toggle_search_on_title")
                                      : t("toggle_search_disabled_title")
                                }
                              >
                                <input
                                  type="checkbox"
                                  checked={isEnabled && d.capabilities.search && isSelected}
                                  className="switch-input"
                                  disabled={
                                    !isEnabled || !d.capabilities.search || sourcesBusy
                                  }
                                  onChange={() => toggleSelected(d.id)}
                                />
                                <span className="switch-box" />
                                <span>
                                  {d.capabilities.search
                                    ? t("source_search")
                                    : t("source_search_unsupported")}
                                </span>
                              </label>
                            </div>
                            {!isEnabled && d.warning && (
                              <div className="source-warning">{d.warning}</div>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  ),
              )}
              {selectedIsAll && (
                <div className="hint">{t("sources_search_all_hint")}</div>
              )}
            </>
          )}
        </section>

        <section>
          <h3>{t("appearance_section_title")}</h3>
          <div className="hint">{t("appearance_style")}</div>
          <div className="theme-toggle">
            {STYLES.map((opt) => (
              <button
                key={opt.id}
                className={`btn ${(settings.style ?? "material") === opt.id ? "btn-active" : ""}`}
                onClick={() => saveSettings({ ...settings, style: opt.id })}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
          <div className="hint">{t("language_label")}</div>
          <div className="theme-toggle">
            {LANGUAGES.map((l) => (
              <button
                key={l.id}
                className={`btn ${(settings.language ?? "ru") === l.id ? "btn-active" : ""}`}
                onClick={() => saveSettings({ ...settings, language: l.id })}
              >
                {t(l.labelKey)}
              </button>
            ))}
          </div>
          <div className="hint">{t("appearance_theme")}</div>
          <div className="theme-toggle">
            {THEMES.map((opt) => (
              <button
                key={opt.id}
                className={`btn ${(settings.theme ?? "dark") === opt.id ? "btn-active" : ""}`}
                onClick={() => saveSettings({ ...settings, theme: opt.id })}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
          <div className="hint">{t("appearance_accent")}</div>
          <div className="accent-row">
            {ACCENT_PRESETS.map((c) => (
              <button
                key={c}
                className={`swatch ${(settings.accent ?? "#4f8cff").toLowerCase() === c ? "swatch-active" : ""}`}
                style={{ background: c }}
                onClick={() => saveSettings({ ...settings, accent: c })}
                title={c}
              />
            ))}
            <input
              type="color"
              className="color-input"
              value={settings.accent ?? "#4f8cff"}
              onChange={(e) => saveSettings({ ...settings, accent: e.target.value })}
              title={t("appearance_custom_color")}
            />
          </div>
          <div className="hint">{t("appearance_cards")}</div>
          <div className="manual-row">
            <select
              className="category-select"
              value={settings.installedSort ?? "date"}
              onChange={(e) => saveSettings({ ...settings, installedSort: e.target.value })}
            >
              {INSTALLED_SORTS.map((s) => (
                <option key={s.id} value={s.id}>
                  {t(s.labelKey)}
                </option>
              ))}
            </select>
            <select
              className="category-select"
              value={settings.cardSize ?? "normal"}
              onChange={(e) => saveSettings({ ...settings, cardSize: e.target.value })}
            >
              {CARD_SIZES.map((s) => (
                <option key={s.id} value={s.id}>
                  {t(s.labelKey)}
                </option>
              ))}
            </select>
          </div>
        </section>

        <section>
          <h3>{t("logs_section_title")}</h3>
          <p className="hint">
            {t("logs_hint")}
            <code> beamng.log</code>
          </p>
          <button className="btn" onClick={openLogs}>
            {t("logs_open_folder")}
          </button>
        </section>

        {msg && (
          <div className={msg.ok ? "banner banner-ok" : "banner banner-error"}>{msg.text}</div>
        )}
      </div>
    </div>
  );
}
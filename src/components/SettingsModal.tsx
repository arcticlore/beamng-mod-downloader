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
import { applyPreset, groupLabel, sortByGroup, statusLabel, trustLabel } from "../sources";
import type { AppSettings, ModsFolderCandidate, SourceGroup } from "../types";
import { CARD_SIZES, INSTALLED_SORTS, THEMES } from "../types";

const GROUPS: SourceGroup[] = ["official", "forges", "community", "custom"];

interface Props {
  onClose: () => void;
  onChanged: () => void;
}

export function SettingsModal({ onClose, onChanged }: Props) {
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

  const saveSettings = async (next: AppSettings, quiet = false) => {
    setSettingsState(next);
    applyAppearance(next);
    try {
      await setSettings(next);
      onChanged();
      if (!quiet) setMsg({ ok: true, text: "Настройки интерфейса сохранены" });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
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
      setMsg({ ok: true, text: "Папка модов сохранена" });
      onChanged();
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  };

  const openLogs = async () => {
    setMsg(null);
    try {
      await openLogDir();
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
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
  const selectedSet = useMemo(() => {
    if (!selection) return new Set<string>();
    if (selection.selected === null) return new Set(selection.enabled);
    return new Set(selection.selected);
  }, [selection]);
  const anyEnabled = enabledSet.size > 0;

  const runSource = async (fn: () => Promise<void>, okText: string) => {
    setSourcesBusy(true);
    try {
      await fn();
      setMsg({ ok: true, text: okText });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setSourcesBusy(false);
    }
  };

  const toggleEnabled = (id: string) =>
    runSource(() => setEnabled(id, !enabledSet.has(id)), "Выбор источников сохранён");

  const toggleSelected = (id: string) => {
    const base = selectedIsAll ? [...enabledSet] : [...selectedSet];
    const next = new Set(base);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    const allEnabled =
      anyEnabled &&
      [...enabledSet].every((i) => next.has(i)) &&
      next.size === enabledSet.size;
    return runSource(
      () => setSelected(allEnabled ? null : [...next].sort()),
      "Поиск по источникам обновлён",
    );
  };

  const applyPresetNow = (preset: "recommended" | "official_forges" | "all_configured" | "clear") => {
    const list = applyPreset(preset, registry, [...enabledSet]);
    const allEnabled =
      anyEnabled &&
      list.length === enabledSet.size &&
      [...enabledSet].every((i) => list.includes(i));
    if (allEnabled) {
      return runSource(() => setSelected(null), "Поиск по всем включённым источникам");
    }
    return runSource(() => setSelected(list), "Пресет применён");
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
        <h2>Настройки</h2>

        <section>
          <h3>Папка с модами BeamNG.drive</h3>
          {current && (
            <div className="current-path">
              Текущая: <code>{current}</code>
            </div>
          )}
          <p className="hint">
            Приложение само ищет папки модов в стандартных местах (Linux, Windows, macOS).
            Выберите вариант ниже или укажите путь вручную.
          </p>
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
            <div className="hint">Автоматически ничего не найдено — укажите путь вручную.</div>
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
              Применить
            </button>
            <button
              className="btn"
              disabled={busy || !manual.trim()}
              title="Сохранить путь без проверки .zip"
              onClick={() => applyFolder(manual.trim(), true)}
            >
              Принудительно
            </button>
          </div>
        </section>

        <section>
          <h3>Источники</h3>
          <p className="hint">
            Источник должен быть <b>включён</b>, чтобы приложение обращалось к нему в сеть
            (поиск, описание, установка, обновления). Отключённый источник никогда не
            запрашивается. <b>Поиск</b> определяет, в каких включённых источниках искать.
            Если «поиск» включён у всех — поиск автоматически охватывает все включённые
            источники.
          </p>

          {sourcesLoading && <div className="hint">Загрузка источников…</div>}
          {!sourcesLoading && sourcesError && (
            <div className="banner banner-error">{sourcesError}</div>
          )}

          {!sourcesLoading && !sourcesError && (
            <>
              <div className="source-presets">
                <button
                  className="btn btn-sm"
                  disabled={!anyEnabled || sourcesBusy}
                  title="Официальный сайт BeamNG и open-source forges"
                  onClick={() => applyPresetNow("recommended")}
                >
                  Рекомендуемые
                </button>
                <button
                  className="btn btn-sm"
                  disabled={!anyEnabled || sourcesBusy}
                  onClick={() => applyPresetNow("official_forges")}
                >
                  Официальные + forges
                </button>
                <button
                  className="btn btn-sm"
                  disabled={!anyEnabled || sourcesBusy}
                  onClick={() => applyPresetNow("all_configured")}
                >
                  Все включённые
                </button>
                <button
                  className="btn btn-sm"
                  disabled={!anyEnabled || sourcesBusy}
                  onClick={() => applyPresetNow("clear")}
                >
                  Очистить
                </button>
                <button
                  className="btn btn-sm"
                  disabled={sourcesBusy}
                  title="Включить рекомендуемые источники и сбросить выбор поиска"
                  onClick={() =>
                    runSource(resetDefaults, "Выбор источников сброшен к рекомендуемым")
                  }
                >
                  Сброс к defaults
                </button>
              </div>

              {GROUPS.map(
                (g) =>
                  groups.has(g) && (
                    <div key={g} className="source-group">
                      <div className="source-group-label">{groupLabel(g)}</div>
                      {groups.get(g)!.map((d) => {
                        const isEnabled = enabledSet.has(d.id);
                        const isSelected = selectedSet.has(d.id);
                        return (
                          <div
                            key={d.id}
                            className={`source-item ${isEnabled ? "" : "source-item-off"}`}
                          >
                            <div className="source-item-meta">
                              <span className="source-item-name">{d.label}</span>
                              <span
                                className={`trust-badge trust-${d.trustLevel}`}
                                title={`Доверие: ${trustLabel(d.trustLevel)}`}
                              >
                                {trustLabel(d.trustLevel)}
                              </span>
                              {d.status !== "ready" && (
                                <span className="source-status-badge">
                                  {statusLabel(d.status)}
                                </span>
                              )}
                            </div>
                            <div className="source-item-controls">
                              <label
                                className="switch-label"
                                title={
                                  isEnabled
                                    ? "Отключить: приложение перестанет обращаться к источнику"
                                    : "Включить источник (разрешить сетевые запросы)"
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
                                <span>Включён</span>
                              </label>
                              <label
                                className="switch-label switch-label-secondary"
                                title={
                                  isEnabled
                                    ? "Искать моды в этом источнике"
                                    : "Сначала включите источник"
                                }
                              >
                                <input
                                  type="checkbox"
                                  checked={isEnabled && isSelected}
                                  className="switch-input"
                                  disabled={!isEnabled || sourcesBusy}
                                  onChange={() => toggleSelected(d.id)}
                                />
                                <span className="switch-box" />
                                <span>Поиск</span>
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
                <div className="hint">
                  Поиск активен во всех включённых источниках. Снимите «Поиск» у источника,
                  чтобы искать в подмножестве.
                </div>
              )}
            </>
          )}
        </section>

        <section>
          <h3>Внешний вид</h3>
          <div className="hint">Тема:</div>
          <div className="theme-toggle">
            {THEMES.map((t) => (
              <button
                key={t.id}
                className={`btn ${(settings.theme ?? "dark") === t.id ? "btn-active" : ""}`}
                onClick={() => saveSettings({ ...settings, theme: t.id })}
              >
                {t.label}
              </button>
            ))}
          </div>
          <div className="hint" style={{ marginTop: 12 }}>Акцентный цвет:</div>
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
              title="Свой цвет"
            />
          </div>
          <div className="hint" style={{ marginTop: 12 }}>Панель установленных и карточки:</div>
          <div className="manual-row">
            <select
              className="category-select"
              value={settings.installedSort ?? "date"}
              onChange={(e) => saveSettings({ ...settings, installedSort: e.target.value })}
            >
              {INSTALLED_SORTS.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.label}
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
                  {s.label}
                </option>
              ))}
            </select>
          </div>
        </section>

        <section>
          <h3>Логи</h3>
          <p className="hint">
            Приложение пишет подробный лог (загрузки, ошибки сети, действия) в файл
            <code> beamng.log</code> в каталоге данных приложения.
          </p>
          <button className="btn" onClick={openLogs}>
            Открыть папку логов
          </button>
        </section>

        {msg && (
          <div className={msg.ok ? "banner banner-ok" : "banner banner-error"}>{msg.text}</div>
        )}
      </div>
    </div>
  );
}
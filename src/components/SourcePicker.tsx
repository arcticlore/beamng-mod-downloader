import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { useSources } from "../SourcesContext";
import {
  applyPreset,
  canonicalizeSelection,
  filterSources,
  groupLabel,
  searchCapableEnabledIds,
  toggleSourceSelection,
} from "../sources";
import type { SourceGroup, SourceDescriptor } from "../types";

type Preset = "recommended" | "official_forges" | "all_configured" | "clear";

const PRESETS: { id: Preset; label: string; title: string }[] = [
  {
    id: "recommended",
    label: "Рекомендуемые",
    title: "Официальный сайт BeamNG и open-source forges",
  },
  {
    id: "official_forges",
    label: "Официальные + forges",
    title: "Официальные и open-source forges",
  },
  {
    id: "all_configured",
    label: "Все включённые",
    title: "Все включённые источники с поддержкой поиска",
  },
  { id: "clear", label: "Снять выбор", title: "Ничего не искать" },
];

const GROUPS: SourceGroup[] = ["official", "forges", "community", "custom"];

interface Props {
  open: boolean;
  onClose: () => void;
  onOpenSettings: () => void;
}

export function SourcePicker({ open, onClose, onOpenSettings }: Props) {
  const { registry, selection, setSelected } = useSources();
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

  const enabled = selection?.enabled ?? [];
  const selected = selection?.selected ?? null;

  const searchCapable = useMemo(
    () => searchCapableEnabledIds(registry, enabled),
    [registry, enabled],
  );
  const searchCapableSet = useMemo(() => new Set(searchCapable), [searchCapable]);
  const disabledCount = useMemo(
    () => registry.filter((d) => !enabled.includes(d.id)).length,
    [registry, enabled],
  );
  const anySearchCapable = searchCapable.length > 0;

  const selectedSet = useMemo(() => {
    if (!selection) return new Set<string>();
    if (selection.selected === null) return new Set(searchCapable);
    return new Set(
      selection.selected.filter((id) => searchCapableSet.has(id)),
    );
  }, [selection, searchCapable, searchCapableSet]);

  const filtered = useMemo(() => filterSources(registry, text), [registry, text]);

  const groups = useMemo(() => {
    const m = new Map<SourceGroup, SourceDescriptor[]>();
    for (const d of filtered) {
      const arr = m.get(d.group) ?? [];
      arr.push(d);
      m.set(d.group, arr);
    }
    return m;
  }, [filtered]);

  useEffect(() => {
    if (!open) return;
    setText("");
  }, [open]);

  useEffect(() => {
    if (!open) return;
    inputRef.current?.focus();
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    const onPointer = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onPointer);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onPointer);
    };
  }, [open, onClose]);

  const run = async (fn: () => Promise<unknown>, quiet?: boolean) => {
    setBusy(true);
    try {
      await fn();
      if (!quiet) onClose();
    } finally {
      setBusy(false);
    }
  };

  const toggle = (id: string) => {
    const next = toggleSourceSelection(registry, enabled, selected, id);
    return run(() => setSelected(next));
  };

  const apply = (preset: Preset) => {
    const list = applyPreset(preset, registry, enabled);
    const next = canonicalizeSelection(searchCapable, list);
    return run(() => setSelected(next));
  };

  let panel: ReactNode = null;
  if (open) {
    panel = (
      <div
        ref={panelRef}
        className="source-picker-panel"
        role="dialog"
        aria-label="Выбор источников поиска"
      >
        <div className="source-picker-head">
          <span className="source-picker-title">Источники поиска</span>
          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label="Закрыть"
          >
            ✕
          </button>
        </div>
        <input
          ref={inputRef}
          className="search-input source-picker-filter"
          type="search"
          placeholder="Фильтр по названию или id…"
          value={text}
          onChange={(e) => setText(e.target.value)}
          aria-label="Фильтр источников"
          disabled={busy}
        />
        <div className="source-presets">
          {PRESETS.map((p) => (
            <button
              key={p.id}
              type="button"
              className="btn btn-sm"
              title={p.title}
              disabled={busy || (!anySearchCapable && p.id !== "clear")}
              onClick={() => void apply(p.id)}
            >
              {p.label}
            </button>
          ))}
        </div>

        {disabledCount > 0 && (
          <div className="hint source-picker-hint">
            Отключённые и источники без поиска не участвуют в поиске.
          </div>
        )}

        {!selection && <div className="hint">Загрузка источников…</div>}
        {selection && filtered.length === 0 && (
          <div className="hint">Ничего не найдено</div>
        )}

        {selection &&
          GROUPS.map(
            (g) =>
              groups.has(g) && (
                <div key={g} className="source-group">
                  <div className="source-group-label">{groupLabel(g)}</div>
                  {groups.get(g)!.map((d) => {
                    const isEnabled = enabled.includes(d.id);
                    const isSearchable = d.capabilities.search;
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
                            title={`Доверие: ${d.trustLevel}`}
                          >
                            {d.trustLevel === "official"
                              ? "Официальный"
                              : d.trustLevel === "verified_forge"
                                ? "Open-source forge"
                                : d.trustLevel === "community"
                                  ? "Community"
                                  : d.trustLevel === "third_party"
                                    ? "Сторонний"
                                    : "Пользовательский"}
                          </span>
                        </div>
                        <div className="source-item-controls">
                          {isEnabled && isSearchable ? (
                            <label className="switch-label switch-label-secondary">
                              <input
                                type="checkbox"
                                className="switch-input"
                                checked={isSelected}
                                disabled={busy}
                                onChange={() => void toggle(d.id)}
                              />
                              <span className="switch-box" />
                              <span>Поиск</span>
                            </label>
                          ) : (
                            <span className="source-picker-why">
                              {!isEnabled
                                ? "Выключен"
                                : "Только вручную"}
                            </span>
                          )}
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

        <div className="source-picker-footer">
          <button
            type="button"
            className="btn btn-sm"
            onClick={() => {
              onClose();
              onOpenSettings();
            }}
          >
            Настроить источники…
          </button>
        </div>
      </div>
    );
  }

  return <>{panel}</>;
}
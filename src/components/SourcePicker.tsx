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
  trustLabel,
} from "../sources";
import { useI18n } from "../i18n/LanguageContext";
import type { SourceGroup, SourceDescriptor } from "../types";

type Preset = "recommended" | "official_forges" | "all_configured" | "clear";

const PRESETS: {
  id: Preset;
  labelKey: "preset_recommended" | "preset_official_forges" | "preset_all_configured" | "preset_clear";
  titleKey: "preset_recommended_title" | "preset_official_forges_title" | "preset_all_title" | "picker_preset_clear_title";
}[] = [
  {
    id: "recommended",
    labelKey: "preset_recommended",
    titleKey: "preset_recommended_title",
  },
  {
    id: "official_forges",
    labelKey: "preset_official_forges",
    titleKey: "preset_official_forges_title",
  },
  {
    id: "all_configured",
    labelKey: "preset_all_configured",
    titleKey: "preset_all_title",
  },
  { id: "clear", labelKey: "preset_clear", titleKey: "picker_preset_clear_title" },
];

const GROUPS: SourceGroup[] = ["official", "forges", "community", "custom"];

interface Props {
  open: boolean;
  onClose: () => void;
  onOpenSettings: () => void;
}

export function SourcePicker({ open, onClose, onOpenSettings }: Props) {
  const { registry, selection, setSelected } = useSources();
  const { t } = useI18n();
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
        aria-label={t("picker_aria_label")}
      >
        <div className="source-picker-head">
          <span className="source-picker-title">{t("picker_title")}</span>
          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label={t("picker_close_aria")}
          >
            ✕
          </button>
        </div>
        <input
          ref={inputRef}
          className="search-input source-picker-filter"
          type="search"
          placeholder={t("picker_filter_placeholder")}
          value={text}
          onChange={(e) => setText(e.target.value)}
          aria-label={t("picker_filter_aria")}
          disabled={busy}
        />
        <div className="source-presets">
          {PRESETS.map((p) => (
            <button
              key={p.id}
              type="button"
              className="btn btn-sm"
              title={t(p.titleKey)}
              disabled={busy || (!anySearchCapable && p.id !== "clear")}
              onClick={() => void apply(p.id)}
            >
              {t(p.labelKey)}
            </button>
          ))}
        </div>

        {disabledCount > 0 && (
          <div className="hint source-picker-hint">
            {t("picker_hint_disabled")}
          </div>
        )}

        {!selection && <div className="hint">{t("picker_loading")}</div>}
        {selection && filtered.length === 0 && (
          <div className="hint">{t("picker_empty")}</div>
        )}

        {selection &&
          GROUPS.map(
            (g) =>
              groups.has(g) && (
                <div key={g} className="source-group">
                  <div className="source-group-label">{t(groupLabel(g))}</div>
                  {groups.get(g)!.map((d) => {
                    const isEnabled = enabled.includes(d.id);
                    const isSearchable = d.capabilities.search;
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
                              <span>{t("picker_search")}</span>
                            </label>
                          ) : (
                            <span className="source-picker-why">
                              {!isEnabled
                                ? t("picker_off")
                                : t("picker_manual_only")}
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
            {t("picker_configure")}
          </button>
        </div>
      </div>
    );
  }

  return <>{panel}</>;
}
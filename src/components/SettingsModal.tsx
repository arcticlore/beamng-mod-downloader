import { ACCENT_PRESETS } from "../theme";
import { groupLabel, statusLabel, trustLabel } from "../sources";
import { useI18n } from "../i18n/LanguageContext";
import { CARD_SIZES, INSTALLED_SORTS, STYLES, THEMES } from "../types";
import { GROUPS, useSettings } from "../hooks/useSettings";

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
  const controller = useSettings(onChanged);
  const { folder, appearance, logs, sources, msg } = controller;
  const { settings, saveSettings } = appearance;

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal modal-settings" onClick={(e) => e.stopPropagation()}>
        <button className="modal-close" onClick={onClose}>✕</button>
        <h2>{t("settings_title")}</h2>

        <section>
          <h3>{t("folder_section_title")}</h3>
          {folder.current && (
            <div className="current-path">
              {t("folder_current")} <code>{folder.current}</code>
            </div>
          )}
          <p className="hint">{t("folder_hint")}</p>
          {folder.candidates.length > 0 ? (
            <div className="candidate-list">
              {folder.candidates.map((c) => (
                <button
                  key={c.path}
                  className={`candidate ${folder.current === c.path ? "candidate-active" : ""}`}
                  onClick={() => {
                    folder.setManual(c.path);
                    folder.applyFolder(c.path, false);
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
              value={folder.manual}
              onChange={(e) => folder.setManual(e.target.value)}
            />
            <button
              className="btn btn-primary"
              disabled={folder.busy || !folder.manual.trim()}
              onClick={() => folder.applyFolder(folder.manual.trim(), false)}
            >
              {t("folder_apply")}
            </button>
            <button
              className="btn"
              disabled={folder.busy || !folder.manual.trim()}
              title={t("folder_force_title")}
              onClick={() => folder.applyFolder(folder.manual.trim(), true)}
            >
              {t("folder_force")}
            </button>
          </div>
        </section>

        <section>
          <h3>{t("sources_section_title")}</h3>
          <p className="hint" dangerouslySetInnerHTML={{ __html: t("sources_hint") }} />

          {sources.sourcesLoading && <div className="hint">{t("sources_loading")}</div>}
          {!sources.sourcesLoading && sources.sourcesError && (
            <div className="banner banner-error">{sources.sourcesError}</div>
          )}

          {!sources.sourcesLoading && !sources.sourcesError && (
            <>
              <div className="source-presets">
                <button
                  className="btn btn-sm"
                  disabled={!sources.anyEnabled || sources.sourcesBusy}
                  title={t("preset_recommended_title")}
                  onClick={() => sources.applyPresetNow("recommended")}
                >
                  {t("preset_recommended")}
                </button>
                <button
                  className="btn btn-sm"
                  disabled={!sources.anyEnabled || sources.sourcesBusy}
                  onClick={() => sources.applyPresetNow("official_forges")}
                >
                  {t("preset_official_forges")}
                </button>
                <button
                  className="btn btn-sm"
                  disabled={!sources.anyEnabled || sources.sourcesBusy}
                  onClick={() => sources.applyPresetNow("all_configured")}
                >
                  {t("preset_all_configured")}
                </button>
                <button
                  className="btn btn-sm"
                  disabled={!sources.anyEnabled || sources.sourcesBusy}
                  onClick={() => sources.applyPresetNow("clear")}
                >
                  {t("preset_clear")}
                </button>
                <button
                  className="btn btn-sm"
                  disabled={sources.sourcesBusy}
                  title={t("preset_reset_defaults_title")}
                  onClick={sources.resetDefaultsNow}
                >
                  {t("preset_reset_defaults")}
                </button>
              </div>

              {GROUPS.map(
                (g) =>
                  sources.groups.has(g) && (
                    <div key={g} className="source-group">
                      <div className="source-group-label">{t(groupLabel(g))}</div>
                      {sources.groups.get(g)!.map((d) => {
                        const isEnabled = sources.enabledSet.has(d.id);
                        const isSelected = sources.selectedSet.has(d.id);
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
                                  disabled={sources.sourcesBusy}
                                  onChange={() => sources.toggleEnabled(d.id)}
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
                                    !isEnabled || !d.capabilities.search || sources.sourcesBusy
                                  }
                                  onChange={() => sources.toggleSelected(d.id)}
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
              {sources.selectedIsAll && (
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
          <button className="btn" onClick={logs.openLogs}>
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
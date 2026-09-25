import { ACCENT_PRESETS } from "../../theme";
import { groupLabel, statusLabel, trustLabel } from "../../sources";
import { useI18n } from "../../i18n/LanguageContext";
import { CARD_SIZES, INSTALLED_SORTS, STYLES, THEMES } from "../../types";
import type { MessageKey } from "../../i18n";
import { GROUPS, useSettings } from "../../hooks/useSettings";
import { IconClose, IconFolder, IconShield } from "./icons";

const LANGUAGES: { id: "ru" | "en"; labelKey: "language_ru" | "language_en" }[] = [
  { id: "ru", labelKey: "language_ru" },
  { id: "en", labelKey: "language_en" },
];

interface SegmentedOption {
  id: string;
  labelKey: MessageKey;
}

function Segmented({
  options,
  value,
  onChange,
  title,
}: {
  options: SegmentedOption[];
  value: string;
  onChange: (id: string) => void;
  title?: string;
}) {
  const { t } = useI18n();
  return (
    <div className="m3-segmented" role="radiogroup" aria-label={title ?? value}>
      {options.map((opt) => (
        <button
          key={opt.id}
          type="button"
          role="radio"
          aria-checked={value === opt.id}
          className={`m3-seg-seg ${value === opt.id ? "m3-seg-active" : ""}`}
          onClick={() => onChange(opt.id)}
        >
          {t(opt.labelKey)}
        </button>
      ))}
    </div>
  );
}

interface Props {
  onClose: () => void;
  onChanged: () => void;
}

export function MaterialSettings({ onClose, onChanged }: Props) {
  const { t } = useI18n();
  const controller = useSettings(onChanged);
  const { folder, appearance, logs, sources, msg } = controller;
  const { settings, saveSettings } = appearance;

  return (
    <div className="m3-dialog-backdrop" onClick={onClose}>
      <div
        className="m3-dialog m3-settings-dialog"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="m3-dialog-head">
          <h2>{t("settings_title")}</h2>
          <button
            className="m3-iconbtn"
            onClick={onClose}
            aria-label={t("close")}
            title={t("close")}
          >
            <IconClose size={22} />
          </button>
        </header>

        <div className="m3-settings-body">
          <section className="m3-section">
            <h3 className="m3-section-title">{t("folder_section_title")}</h3>
            {folder.current && (
              <div className="m3-current-path">
                <IconFolder size={16} />
                <code>{folder.current}</code>
              </div>
            )}
            <p className="m3-hint">{t("folder_hint")}</p>
            <div className="m3-candidates">
              {folder.candidates.length > 0 ? (
                folder.candidates.map((c) => (
                  <button
                    type="button"
                    key={c.path}
                    className={`m3-candidate ${
                      folder.current === c.path ? "m3-candidate-active" : ""
                    }`}
                    onClick={() => {
                      folder.setManual(c.path);
                      folder.applyFolder(c.path, false);
                    }}
                  >
                    <span className="m3-candidate-path">{c.path}</span>
                    {folder.current === c.path && <span className="m3-candidate-check">✓</span>}
                  </button>
                ))
              ) : (
                <p className="m3-hint">{t("folder_none_found")}</p>
              )}
            </div>
            <div className="m3-row-input">
              <input
                className="m3-textfield"
                placeholder="/путь/к/.../mods"
                value={folder.manual}
                onChange={(e) => folder.setManual(e.target.value)}
                aria-label={t("folder_apply")}
              />
              <button
                type="button"
                className="m3-btn m3-btn-primary"
                disabled={folder.busy || !folder.manual.trim()}
                onClick={() => folder.applyFolder(folder.manual.trim(), false)}
              >
                {t("folder_apply")}
              </button>
              <button
                type="button"
                className="m3-btn m3-btn-outline"
                disabled={folder.busy || !folder.manual.trim()}
                title={t("folder_force_title")}
                onClick={() => folder.applyFolder(folder.manual.trim(), true)}
              >
                {t("folder_force")}
              </button>
            </div>
          </section>

          <section className="m3-section">
            <h3 className="m3-section-title">{t("appearance_section_title")}</h3>

            <div className="m3-setting-row">
              <span className="m3-setting-label">{t("appearance_style")}</span>
              <Segmented
                options={STYLES as SegmentedOption[]}
                value={settings.style ?? "material"}
                onChange={(id) => saveSettings({ ...settings, style: id })}
                title={t("appearance_style")}
              />
            </div>

            <div className="m3-setting-row">
              <span className="m3-setting-label">{t("language_label")}</span>
              <Segmented
                options={LANGUAGES as SegmentedOption[]}
                value={settings.language ?? "ru"}
                onChange={(id) => saveSettings({ ...settings, language: id })}
                title={t("language_label")}
              />
            </div>

            <div className="m3-setting-row">
              <span className="m3-setting-label">{t("appearance_theme")}</span>
              <Segmented
                options={THEMES as SegmentedOption[]}
                value={settings.theme ?? "dark"}
                onChange={(id) => saveSettings({ ...settings, theme: id })}
                title={t("appearance_theme")}
              />
            </div>

            <div className="m3-setting-row">
              <span className="m3-setting-label">{t("appearance_accent")}</span>
              <div className="m3-accent-row">
                {ACCENT_PRESETS.map((c) => (
                  <button
                    type="button"
                    key={c}
                    className={`m3-swatch ${
                      (settings.accent ?? "#4f8cff").toLowerCase() === c
                        ? "m3-swatch-active"
                        : ""
                    }`}
                    style={{ background: c }}
                    onClick={() => saveSettings({ ...settings, accent: c })}
                    title={c}
                    aria-label={c}
                  />
                ))}
                <input
                  type="color"
                  className="m3-color-input"
                  value={settings.accent ?? "#4f8cff"}
                  onChange={(e) => saveSettings({ ...settings, accent: e.target.value })}
                  title={t("appearance_custom_color")}
                />
              </div>
            </div>

            <div className="m3-setting-row">
              <span className="m3-setting-label">{t("appearance_cards")}</span>
              <Segmented
                options={CARD_SIZES as SegmentedOption[]}
                value={settings.cardSize ?? "normal"}
                onChange={(id) => saveSettings({ ...settings, cardSize: id })}
                title={t("appearance_cards")}
              />
            </div>

            <div className="m3-setting-row">
              <span className="m3-setting-label">{t("sort_installed_label")}</span>
              <Segmented
                options={INSTALLED_SORTS as SegmentedOption[]}
                value={settings.installedSort ?? "date"}
                onChange={(id) => saveSettings({ ...settings, installedSort: id })}
                title={t("sort_installed_label")}
              />
            </div>
          </section>

          <section className="m3-section">
            <h3 className="m3-section-title">{t("sources_section_title")}</h3>
            <p className="m3-hint" dangerouslySetInnerHTML={{ __html: t("sources_hint") }} />

            {sources.sourcesLoading && <p className="m3-hint">{t("sources_loading")}</p>}
            {!sources.sourcesLoading && sources.sourcesError && (
              <div className="m3-banner m3-banner-error">{sources.sourcesError}</div>
            )}

            {!sources.sourcesLoading && !sources.sourcesError && (
              <>
                <div className="m3-presets">
                  <button
                    type="button"
                    className="m3-btn m3-btn-outline"
                    disabled={!sources.anyEnabled || sources.sourcesBusy}
                    title={t("preset_recommended_title")}
                    onClick={() => sources.applyPresetNow("recommended")}
                  >
                    {t("preset_recommended")}
                  </button>
                  <button
                    type="button"
                    className="m3-btn m3-btn-outline"
                    disabled={!sources.anyEnabled || sources.sourcesBusy}
                    onClick={() => sources.applyPresetNow("official_forges")}
                  >
                    {t("preset_official_forges")}
                  </button>
                  <button
                    type="button"
                    className="m3-btn m3-btn-outline"
                    disabled={!sources.anyEnabled || sources.sourcesBusy}
                    onClick={() => sources.applyPresetNow("all_configured")}
                  >
                    {t("preset_all_configured")}
                  </button>
                  <button
                    type="button"
                    className="m3-btn m3-btn-text"
                    disabled={!sources.anyEnabled || sources.sourcesBusy}
                    onClick={() => sources.applyPresetNow("clear")}
                  >
                    {t("preset_clear")}
                  </button>
                  <button
                    type="button"
                    className="m3-btn m3-btn-text"
                    disabled={sources.sourcesBusy}
                    title={t("preset_reset_defaults_title")}
                    onClick={sources.resetDefaultsNow}
                  >
                    {t("preset_reset_defaults")}
                  </button>
                </div>

                {GROUPS.map((g) =>
                  sources.groups.has(g) ? (
                    <div key={g} className="m3-source-group">
                      <div className="m3-source-group-label">{t(groupLabel(g))}</div>
                      {sources.groups.get(g)!.map((d) => {
                        const isEnabled = sources.enabledSet.has(d.id);
                        const isSelected = sources.selectedSet.has(d.id);
                        const trustKey = trustLabel(d.trustLevel);
                        return (
                          <div
                            key={d.id}
                            className={`m3-source-row ${isEnabled ? "" : "m3-source-row-off"}`}
                          >
                            <div className="m3-source-info">
                              <span className="m3-source-name">{d.label}</span>
                              <div className="m3-source-badges">
                                <span
                                  className={`m3-badge m3-badge-outline trust-${d.trustLevel}`}
                                  title={t("trust_badge_title", { level: t(trustKey) })}
                                >
                                  <IconShield size={12} />
                                  {t(trustKey)}
                                </span>
                                {d.status !== "ready" && (
                                  <span className="m3-badge m3-badge-warn">
                                    {t(statusLabel(d.status))}
                                  </span>
                                )}
                              </div>
                            </div>
                            <div className="m3-source-switches">
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
                                <span className="m3-switch-text">{t("source_enabled")}</span>
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
                                <span className="m3-switch-text">
                                  {d.capabilities.search
                                    ? t("source_search")
                                    : t("source_search_unsupported")}
                                </span>
                              </label>
                            </div>
                            {!isEnabled && d.warning && (
                              <p className="m3-source-warning">{d.warning}</p>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  ) : null,
                )}
                {sources.selectedIsAll && (
                  <p className="m3-hint">{t("sources_search_all_hint")}</p>
                )}
              </>
            )}
          </section>

          <section className="m3-section">
            <h3 className="m3-section-title">{t("logs_section_title")}</h3>
            <p className="m3-hint">
              {t("logs_hint")}
              <code> beamng.log</code>
            </p>
            <button type="button" className="m3-btn m3-btn-outline" onClick={logs.openLogs}>
              {t("logs_open_folder")}
            </button>
          </section>

          {msg && (
            <div
              className={msg.ok ? "m3-banner m3-banner-ok" : "m3-banner m3-banner-error"}
            >
              {msg.text}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
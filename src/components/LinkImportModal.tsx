import { useState } from "react";
import type { ReactNode } from "react";
import { importLocalZip, installFromUrl, openExternal } from "../api";
import { useSources } from "../SourcesContext";
import { classifyLink } from "../sources";
import { useI18n } from "../i18n/LanguageContext";
import { pickZipFile } from "../picker";
import { IconClose } from "./material/icons";

interface Props {
  variant: "classic" | "material";
  onToast: (text: string) => void;
  onClose: () => void;
}

type Tab = "url" | "zip";

export function LinkImportModal({ variant, onToast, onClose }: Props) {
  const { selection } = useSources();
  const { t } = useI18n();
  const material = variant === "material";

  const [tab, setTab] = useState<Tab>("url");
  const [url, setUrl] = useState("");
  const [zipPath, setZipPath] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const enabled = selection?.enabled ?? [];
  const forumEnabled = enabled.includes("beamngforum");
  const directEnabled = enabled.includes("directurl");

  const kind = classifyLink(url);
  const forumDisabledWarn = kind === "forum" && !forumEnabled;
  const directDisabledWarn = kind === "direct" && !directEnabled;

  const tabBtn = (id: Tab, label: string) => (
    <button
      type="button"
      className={`li-tab ${tab === id ? "li-tab-active" : ""}`}
      onClick={() => setTab(id)}
      aria-selected={tab === id}
      role="tab"
    >
      {label}
    </button>
  );

  async function chooseZip() {
    setBusy(true);
    setError(null);
    try {
      const p = await pickZipFile();
      if (p) setZipPath(p);
    } catch (e) {
      onToast(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function submitUrl() {
    const trimmed = url.trim();
    if (!trimmed) {
      setError(t("link_import_url_bad"));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await installFromUrl(trimmed);
      onToast(t("link_import_started", { url: trimmed }));
      onClose();
    } catch (e) {
      setError(String(e));
      onToast(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function submitZip() {
    if (!zipPath) return;
    setBusy(true);
    setError(null);
    try {
      const filename = await importLocalZip(zipPath);
      onToast(t("link_import_zip_imported", { file: filename }));
      onClose();
    } catch (e) {
      setError(String(e));
      onToast(String(e));
    } finally {
      setBusy(false);
    }
  }

  const zipName = zipPath ? zipPath.split(/[/\\]/).pop() : null;

  const actions = (children: ReactNode[]) => (
    <div className="li-actions">
      <button type="button" className="btn btn-ghost" onClick={onClose}>
        {t("close")}
      </button>
      {children}
    </div>
  );

  const content = (
    <>
      {error && <div className="error-text">{error}</div>}
      <div className="li-tabs" role="tablist">
        {tabBtn("url", t("link_import_tab_url"))}
        {tabBtn("zip", t("link_import_tab_zip"))}
      </div>

      {tab === "url" ? (
        <div className="li-body">
          <label className="li-field">
            <span className="li-label">{t("link_import_url_label")}</span>
            <input
              className="li-input"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder={t("link_import_url_placeholder")}
              spellCheck={false}
              autoFocus
            />
          </label>

          {kind === "forum" && (
            <div className="li-hint">{t("link_import_kind_forum_hint")}</div>
          )}
          {kind === "direct" && (
            <div className="li-hint">{t("link_import_kind_direct_hint")}</div>
          )}
          {forumDisabledWarn && (
            <div className="li-warn">{t("link_import_forum_disabled")}</div>
          )}
          {directDisabledWarn && (
            <div className="li-warn">{t("link_import_direct_disabled")}</div>
          )}

          <div className="li-note">{t("link_import_domains_hint")}</div>

          {kind === "forum" && (
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => openExternal("https://www.beamng.com/community/")}
            >
              {t("link_import_open_forum")}
            </button>
          )}

          {actions([
            <button
              key="submit"
              type="button"
              className="btn btn-primary"
              disabled={busy || !url.trim()}
              onClick={submitUrl}
            >
              {busy ? t("browser_loading") : t("link_import_url_submit")}
            </button>,
          ])}
        </div>
      ) : (
        <div className="li-body">
          <div className="li-hint li-hint-title">{t("link_import_zip_title")}</div>
          <div className="li-warn">{t("link_import_zip_warning")}</div>
          <div className="li-row">
            <button
              type="button"
              className="btn btn-ghost"
              disabled={busy}
              onClick={chooseZip}
            >
              {t("link_import_zip_pick")}
            </button>
            <span className="li-file">
              {zipPath
                ? t("link_import_zip_chosen", { file: zipName ?? zipPath })
                : t("link_import_zip_none")}
            </span>
          </div>
          {actions([
            <button
              key="import"
              type="button"
              className="btn btn-primary"
              disabled={busy || !zipPath}
              onClick={submitZip}
            >
              {busy ? t("browser_loading") : t("link_import_zip_import")}
            </button>,
          ])}
        </div>
      )}
    </>
  );

  if (material) {
    return (
      <div className="m3-dialog-backdrop" onClick={onClose}>
        <div
          className="m3-dialog li-modal"
          role="dialog"
          aria-modal="true"
          aria-label={t("link_import_title")}
          onClick={(e) => e.stopPropagation()}
        >
          <div className="m3-dialog-head">
            <h2 className="m3-dialog-title">{t("link_import_title")}</h2>
            <button
              className="m3-iconbtn m3-dialog-close"
              onClick={onClose}
              aria-label={t("close")}
              title={t("close")}
            >
              <IconClose size={22} />
            </button>
          </div>
          {content}
        </div>
      </div>
    );
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="modal li-modal"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
      >
        <button className="modal-close" onClick={onClose} aria-label={t("close")}>
          ✕
        </button>
        <h2 className="li-title">{t("link_import_title")}</h2>
        {content}
      </div>
    </div>
  );
}
import { useI18n } from "../../i18n/LanguageContext";
import type { AppSettings, DownloadState, ModItem } from "../../types";
import type { MessageKey } from "../../i18n";
import { MaterialBrowse } from "./MaterialBrowse";
import { MaterialDownloads } from "./MaterialDownloads";
import { MaterialInstalled } from "./MaterialInstalled";
import { MaterialSettings } from "./MaterialSettings";
import { MaterialDetail } from "./MaterialDetail";
import {
  IconBrowse,
  IconDownload,
  IconFolder,
  IconInstalled,
  IconSettings,
} from "./icons";

export type View = "browse" | "downloads" | "installed";

const NAV: { id: View; labelKey: MessageKey; Icon: typeof IconBrowse }[] = [
  { id: "browse", labelKey: "nav_browse", Icon: IconBrowse },
  { id: "downloads", labelKey: "nav_downloads", Icon: IconDownload },
  { id: "installed", labelKey: "nav_installed", Icon: IconInstalled },
];

export interface MaterialAppProps {
  modsFolder: string | null;
  settings: AppSettings | null;
  view: View;
  setView: (v: View) => void;
  settingsOpen: boolean;
  setSettingsOpen: (v: boolean) => void;
  onSettingsChanged: () => void;
  detailItem: ModItem | null;
  detailInstalled: boolean;
  detailSimilar: { filename: string; path: string } | null;
  setDetailItem: (v: ModItem | null) => void;
  downloads: Record<string, DownloadState>;
  onCancel: (key: string) => void;
  onClearFinished: () => void;
  onInstall: (item: ModItem) => void;
  installedNames: Set<string>;
  installedList: { filename: string; path: string }[];
  toast: string | null;
  onImportLink: () => void;
}

export function MaterialApp({
  modsFolder,
  settings,
  view,
  setView,
  settingsOpen,
  setSettingsOpen,
  onSettingsChanged,
  detailItem,
  detailInstalled,
  detailSimilar,
  setDetailItem,
  downloads,
  onCancel,
  onClearFinished,
  onInstall,
  installedNames,
  installedList,
  toast,
  onImportLink,
}: MaterialAppProps) {
  const { t } = useI18n();
  const activeNav = NAV.find((n) => n.id === view) ?? NAV[0];

  return (
    <div className="m3-app">
      <aside className="m3-rail" aria-label={t("rail_label")}>
        <div className="m3-rail-logo" aria-label={t("app_name")} title={t("app_name")}>
          <span className="m3-logo-mark">B</span>
        </div>
        <nav className="m3-rail-nav">
          {NAV.map((n) => {
            const active = view === n.id;
            return (
              <button
                key={n.id}
                type="button"
                className={`m3-rail-item ${active ? "m3-rail-item-active" : ""}`}
                onClick={() => setView(n.id)}
                aria-current={active ? "page" : undefined}
                title={t(n.labelKey)}
              >
                <n.Icon size={24} />
                <span className="m3-rail-label">{t(n.labelKey)}</span>
              </button>
            );
          })}
        </nav>
        <div className="m3-rail-spacer" />
        <button
          type="button"
          className={`m3-rail-item ${settingsOpen ? "m3-rail-item-active" : ""}`}
          onClick={() => setSettingsOpen(true)}
          title={t("settings")}
        >
          <IconSettings size={24} />
          <span className="m3-rail-label">{t("settings")}</span>
        </button>
      </aside>

      <div className="m3-frame">
        <header className="m3-topbar">
          <div className="m3-topbar-title">{t(activeNav.labelKey)}</div>
          <button
            type="button"
            className={`m3-folder-chip ${modsFolder ? "" : "m3-folder-chip-none"}`}
            onClick={() => setSettingsOpen(true)}
            title={modsFolder ?? t("folder_title_none")}
          >
            <IconFolder size={18} />
            <span className="m3-folder-path">
              {modsFolder ? modsFolder.split(/[/\\]/).pop() : t("folder_choose")}
            </span>
          </button>
        </header>

        <main className="m3-content">
          {view === "downloads" ? (
            <MaterialDownloads
              downloads={downloads}
              onClearFinished={onClearFinished}
              onCancel={onCancel}
            />
          ) : view === "installed" ? (
            <MaterialInstalled
              settings={settings}
              onOpenSettings={() => setSettingsOpen(true)}
            />
          ) : (
            <MaterialBrowse
              downloads={downloads}
              installedNames={installedNames}
              installedList={installedList}
              onInstall={onInstall}
              onInfo={setDetailItem}
              onOpenSettings={() => setSettingsOpen(true)}
              onImportLink={onImportLink}
            />
          )}
        </main>

        {settingsOpen && (
          <MaterialSettings
            onClose={() => setSettingsOpen(false)}
            onChanged={onSettingsChanged}
          />
        )}

        {detailItem && (
          <MaterialDetail
            item={detailItem}
            dl={downloads[detailItem.id]}
            installed={detailInstalled}
            similar={detailSimilar}
            onInstall={onInstall}
            onClose={() => setDetailItem(null)}
          />
        )}

        {toast && <div className="m3-snackbar" role="status">{toast}</div>}
      </div>
    </div>
  );
}
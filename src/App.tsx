import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelDownload,
  getDownloads,
  getModsFolder,
  getSettings,
  installMod,
  listInstalled,
} from "./api";
import { applyAppearance } from "./theme";
import { DetailModal } from "./components/DetailModal";
import { DownloadsPanel } from "./components/DownloadsPanel";
import { InstalledPanel } from "./components/InstalledPanel";
import { ModsBrowser } from "./components/ModsBrowser";
import { SettingsModal } from "./components/SettingsModal";
import {
  findSimilarInstalled,
  installedFileName,
  type AppSettings,
  type DownloadState,
  type InstalledMod,
  type ModItem,
} from "./types";

type Tab = "all" | "worldofmods" | "beamngweb" | "github" | "downloads" | "installed";

const TABS: { id: Tab; label: string }[] = [
  { id: "all", label: "Все" },
  { id: "worldofmods", label: "WorldOfMods" },
  { id: "beamngweb", label: "Официальный сайт" },
  { id: "github", label: "GitHub-релизы" },
  { id: "downloads", label: "Загрузки" },
  { id: "installed", label: "Установленные" },
];

export default function App() {
  const [modsFolder, setModsFolder] = useState<string | null>(null);
  const [settings, setSettingsState] = useState<AppSettings | null>(null);
  const [tab, setTab] = useState<Tab>("worldofmods");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [detailItem, setDetailItem] = useState<ModItem | null>(null);
  const [installed, setInstalled] = useState<InstalledMod[]>([]);
  const [downloads, setDownloads] = useState<Record<string, DownloadState>>({});
  const [toast, setToast] = useState<string | null>(null);

  useEffect(() => {
    applyAppearance(settings);
  }, [settings]);

  const showToast = useCallback((text: string) => {
    setToast(text);
    window.setTimeout(() => setToast(null), 5000);
  }, []);

  const refreshInstalled = useCallback(async () => {
    try {
      const items = await listInstalled();
      setInstalled(items);
    } catch (e) {
      console.error(e);
      setInstalled([]);
    }
  }, []);

  useEffect(() => {
    getModsFolder().then(setModsFolder).catch(() => {});
    getSettings().then(setSettingsState).catch(() => {});
    refreshInstalled();
    setDownloadsBySnapshot();
    const un1 = listen<DownloadState>("download::progress", (e) => {
      setDownloads((prev) => ({ ...prev, [e.payload.key]: e.payload }));
    });
    const un2 = listen<DownloadState>("download::finished", (e) => {
      setDownloads((prev) => ({ ...prev, [e.payload.key]: e.payload }));
      if (e.payload.state === "done") {
        showToast(`Мод «${e.payload.name}» установлен: ${e.payload.filename}`);
        refreshInstalled();
      } else if (e.payload.state === "error") {
        showToast(`Ошибка: ${e.payload.error ?? "неизвестно"}`);
      }
    });
    return () => {
      un1.then((f) => f());
      un2.then((f) => f());
    };
  }, [refreshInstalled, showToast]);

  const setDownloadsBySnapshot = async () => {
    try {
      const snap = await getDownloads();
      const map: Record<string, DownloadState> = {};
      for (const d of snap) map[d.key] = d;
      setDownloads(map);
    } catch {
      /* ignore */
    }
  };

  const installedNames = useMemo(
    () => new Set(installed.map((m) => m.filename.split("/").pop() ?? m.filename)),
    [installed],
  );

  const installedList = useMemo(
    () =>
      installed.map((m) => ({
        filename: m.filename,
        path: m.path,
      })),
    [installed],
  );

  const onCancel = useCallback((key: string) => {
    cancelDownload(key).catch((e) => showToast(String(e)));
  }, [showToast]);

  const onInstall = useCallback(
    async (item: ModItem) => {
      const similar = findSimilarInstalled(item, installedList);
      if (similar && !installedNames.has(installedFileName(item))) {
        const ok = window.confirm(
          `Похоже, мод «${item.name}» уже установлен как файл «${similar.filename}». Скачать его ещё раз (возможно, это обновление или одноимённый мод)?`,
        );
        if (!ok) return;
      }
      try {
        const key = await installMod({
          source: item.source,
          modId: item.id,
          name: item.name,
          key: item.key,
          published: item.published,
        });
        setDownloads((prev) => ({
          ...prev,
          [key]: {
            key,
            name: item.name,
            filename: installedFileName(item),
            received: 0,
            total: null,
            speedBps: 0,
            state: "downloading",
            error: null,
          },
        }));
        setDetailItem(null);
      } catch (e) {
        showToast(String(e));
      }
    },
    [showToast, installedList, installedNames],
  );

  return (
    <div className="app">
      <header className="app-header">
        <div className="app-title">
          <span className="app-logo">🚗</span>
          <h1>BeamNG Mod Downloader</h1>
        </div>
        <div className="header-right">
          <span
            className={`folder-indicator ${modsFolder ? "" : "folder-none"}`}
            title={modsFolder ?? "Папка модов не выбрана"}
            onClick={() => setSettingsOpen(true)}
          >
            {modsFolder ? "📁 " + modsFolder : "📁 выбрать папку модов"}
          </span>
          <button className="btn btn-sm" onClick={() => setSettingsOpen(true)}>
            Настройки
          </button>
        </div>
      </header>

      <nav className="tabs">
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`tab ${tab === t.id ? "tab-active" : ""}`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <main className="app-main">
        {tab === "downloads" ? (
          <DownloadsPanel
            downloads={downloads}
            onClearFinished={() =>
              setDownloads((prev) => {
                const next: Record<string, DownloadState> = {};
                for (const d of Object.values(prev)) {
                  if (d.state === "downloading") next[d.key] = d;
                }
                return next;
              })
            }
            onCancel={onCancel}
          />
        ) : tab === "installed" ? (
          <InstalledPanel
            settings={settings}
            onOpenSettings={() => setSettingsOpen(true)}
          />
        ) : (
          <ModsBrowser
            source={tab}
            downloads={downloads}
            installedNames={installedNames}
            installedList={installedList}
            cardSize={settings?.cardSize ?? "normal"}
            onInstall={onInstall}
            onInfo={setDetailItem}
          />
        )}
      </main>

      {settingsOpen && (
        <SettingsModal
          onClose={() => setSettingsOpen(false)}
          onChanged={() => {
            getModsFolder().then(setModsFolder).catch(() => {});
            getSettings().then(setSettingsState).catch(() => {});
            refreshInstalled();
          }}
        />
      )}

      {detailItem && (
        <DetailModal
          item={detailItem}
          dl={downloads[detailItem.id]}
          installed={installedNames.has(installedFileName(detailItem))}
          similar={
            installedNames.has(installedFileName(detailItem))
              ? null
              : findSimilarInstalled(detailItem, installedList)
          }
          onInstall={onInstall}
          onClose={() => setDetailItem(null)}
        />
      )}

      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
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
import { SourcesProvider, useSources } from "./SourcesContext";
import { DetailModal } from "./components/DetailModal";
import { DownloadsPanel } from "./components/DownloadsPanel";
import { InstalledPanel } from "./components/InstalledPanel";
import { ModsBrowser } from "./components/ModsBrowser";
import { SettingsModal } from "./components/SettingsModal";
import {
  findSimilarInstalled,
  type AppSettings,
  type DownloadState,
  type InstalledMod,
  type ModItem,
} from "./types";

type View = "browse" | "downloads" | "installed";

const NAV: { id: View; label: string }[] = [
  { id: "browse", label: "Поиск модов" },
  { id: "downloads", label: "Загрузки" },
  { id: "installed", label: "Установленные" },
];

function AppInner() {
  const { filenameFor } = useSources();
  const [modsFolder, setModsFolder] = useState<string | null>(null);
  const [settings, setSettingsState] = useState<AppSettings | null>(null);
  const [view, setView] = useState<View>("browse");
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
    void setDownloadsBySnapshot();
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

  const onCancel = useCallback(
    (key: string) => {
      cancelDownload(key).catch((e) => showToast(String(e)));
    },
    [showToast],
  );

  const onInstall = useCallback(
    async (item: ModItem) => {
      const name = filenameFor(item);
      const similar = findSimilarInstalled(item, installedList);
      if (similar && !installedNames.has(name)) {
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
            filename: name,
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
    [showToast, installedList, installedNames, filenameFor],
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
        {NAV.map((t) => (
          <button
            key={t.id}
            className={`tab ${view === t.id ? "tab-active" : ""}`}
            onClick={() => setView(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <main className="app-main">
        {view === "downloads" ? (
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
        ) : view === "installed" ? (
          <InstalledPanel
            settings={settings}
            onOpenSettings={() => setSettingsOpen(true)}
          />
        ) : (
          <ModsBrowser
            downloads={downloads}
            installedNames={installedNames}
            installedList={installedList}
            cardSize={settings?.cardSize ?? "normal"}
            onInstall={onInstall}
            onInfo={setDetailItem}
            onOpenSettings={() => setSettingsOpen(true)}
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
          installed={installedNames.has(filenameFor(detailItem))}
          similar={
            installedNames.has(filenameFor(detailItem))
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

export default function App() {
  return (
    <SourcesProvider>
      <AppInner />
    </SourcesProvider>
  );
}
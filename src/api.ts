import { invoke } from "@tauri-apps/api/core";
import { debug, error as logError } from "@tauri-apps/plugin-log";
import type {
  AppSettings,
  DownloadState,
  InstallRequest,
  InstalledMod,
  IntegrityReport,
  ModDetail,
  ModSearchResult,
  ModsFolderCandidate,
  ModUpdate,
  SourceCategory,
} from "./types";

/** invoke с логированием команды и ошибок в файл лога приложения. */
async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  debug(`invoke ${cmd}`);
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    logError(`invoke ${cmd}: ${String(e)}`);
    throw e;
  }
}

export const getModsFolder = () => call<string | null>("get_mods_folder");

export const detectModsFolders = () => call<ModsFolderCandidate[]>("detect_mods_folders");

export const setModsFolder = (path: string, force = false) =>
  call<void>(force ? "set_mods_folder_force" : "set_mods_folder", { path });

export const setModsFolderForce = (path: string) => call<void>("set_mods_folder_force", { path });

export const openExternal = (url: string) => call<void>("open_url", { url });

export const searchMods = (
  source: string,
  query: string | null,
  category: string | null,
  page: number,
  order?: string | null,
) =>
  call<ModSearchResult>("search_mods", {
    source,
    query,
    category,
    page,
    order: order ?? null,
  });

export const getModDetail = (source: string, modId: string, key: string) =>
  call<ModDetail>("get_mod_detail", { source, modId, key });

export const getCategories = (source: string) => call<SourceCategory[]>("get_categories", { source });

export const installMod = (req: InstallRequest) => call<string>("install_mod", { req });

export const getDownloads = () => call<DownloadState[]>("get_downloads");

export const cancelDownload = (key: string) => call<void>("cancel_download", { key });

export const updateMod = (filename: string) => call<string>("update_mod", { filename });

export const verifyInstalled = () => call<IntegrityReport[]>("verify_installed");

export const listInstalled = () => call<InstalledMod[]>("list_installed");

export const removeInstalled = (path: string) => call<void>("remove_installed", { path });

export const checkUpdates = (items: InstalledMod[]) => call<ModUpdate[]>("check_updates", { items });

export const getSettings = () => call<AppSettings>("get_app_settings");

export const setSettings = (settings: AppSettings) =>
  call<void>("set_app_settings", { settings });

export const openLogDir = () => call<void>("open_log_dir");
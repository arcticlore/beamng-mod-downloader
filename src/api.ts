import { invoke } from "@tauri-apps/api/core";
import { debug, error as logError } from "@tauri-apps/plugin-log";
import type {
  AppSettings,
  CustomRepo,
  DownloadState,
  InstallRequest,
  InstalledMod,
  ModDetail,
  ModSearchResult,
  ModsFolderCandidate,
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

export const getRepoToken = () => call<string | null>("get_repo_token");

export const setRepoToken = (token: string) => call<void>("set_repo_token", { token });

export const openExternal = (url: string) => call<void>("open_url", { url });

export const searchMods = (
  source: string,
  query: string | null,
  category: string | null,
  page: number,
) =>
  call<ModSearchResult>("search_mods", {
    source,
    query,
    category,
    page,
  });

export const getModDetail = (source: string, modId: string, key: string) =>
  call<ModDetail>("get_mod_detail", { source, modId, key });

export const getCategories = (source: string) => call<SourceCategory[]>("get_categories", { source });

export const installMod = (req: InstallRequest) => call<string>("install_mod", { req });

export const getDownloads = () => call<DownloadState[]>("get_downloads");

export const listInstalled = () => call<InstalledMod[]>("list_installed");

export const removeInstalled = (path: string) => call<void>("remove_installed", { path });

export const getSettings = () => call<AppSettings>("get_app_settings");

export const setSettings = (settings: AppSettings) =>
  call<void>("set_app_settings", { settings });

export const getCustomRepos = () => call<CustomRepo[]>("get_custom_repos");

export const addCustomRepo = (repo: string) => call<CustomRepo[]>("add_custom_repo", { repo });

export const removeCustomRepo = (full: string) =>
  call<CustomRepo[]>("remove_custom_repo", { full });

export const openLogDir = () => call<void>("open_log_dir");
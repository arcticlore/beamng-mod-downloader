import { invoke } from "@tauri-apps/api/core";
import type {
  DownloadState,
  InstallRequest,
  InstalledMod,
  ModDetail,
  ModSearchResult,
  ModsFolderCandidate,
  SourceCategory,
} from "./types";

export const getModsFolder = () => invoke<string | null>("get_mods_folder");

export const detectModsFolders = () => invoke<ModsFolderCandidate[]>("detect_mods_folders");

export const setModsFolder = (path: string, force = false) =>
  invoke<void>(force ? "set_mods_folder_force" : "set_mods_folder", { path });

export const setModsFolderForce = (path: string) => invoke<void>("set_mods_folder_force", { path });

export const getRepoToken = () => invoke<string | null>("get_repo_token");

export const setRepoToken = (token: string) => invoke<void>("set_repo_token", { token });

export const openExternal = (url: string) => invoke<void>("open_url", { url });

export const searchMods = (
  source: string,
  query: string | null,
  category: string | null,
  page: number,
) =>
  invoke<ModSearchResult>("search_mods", {
    source,
    query,
    category,
    page,
  });

export const getModDetail = (source: string, modId: string, key: string) =>
  invoke<ModDetail>("get_mod_detail", { source, modId, key });

export const getCategories = (source: string) => invoke<SourceCategory[]>("get_categories", { source });

export const installMod = (req: InstallRequest) => invoke<string>("install_mod", { req });

export const getDownloads = () => invoke<DownloadState[]>("get_downloads");

export const listInstalled = () => invoke<InstalledMod[]>("list_installed");

export const removeInstalled = (path: string) => invoke<void>("remove_installed", { path });
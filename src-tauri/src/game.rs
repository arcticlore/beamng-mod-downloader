use crate::models::ModsFolderCandidate;
use std::path::{Path, PathBuf};

/// Все известные корневые каталоги пользовательских данных BeamNG.drive
/// для текущей платформы (Linux/Windows/macOS), плюс несколько
/// нестандартных мест, где активно используется настройка userFolder.
fn root_candidates() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir();
        if let Some(h) = &home {
            roots.push(h.join(".local/share/BeamNG/BeamNG.drive"));
            roots.push(h.join(".local/share/BeamNG.drive"));
            roots.push(h.join("BeamNG.drive"));
            roots.push(h.join(".var/app/com.beamng/BeamNG.drive"));
            roots.push(h.join(".steam/steam/steamapps/common/BeamNG.drive"));
            roots.push(h.join("Steam/steamapps/common/BeamNG.drive"));
            // Proton-префиксы (AppID 284160): игра под Windows в Wine-окружении,
            // каталог пользователя лежит глубоко в pfx.
            for steam_root in [".local/share/Steam", ".steam/steam"] {
                roots.push(h.join(
                    format!("{steam_root}/steamapps/compatdata/284160/pfx/drive_c/")
                        + "users/steamuser/AppData/Local/BeamNG.drive",
                ));
            }
        }
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                roots.push(PathBuf::from(&xdg).join("BeamNG/BeamNG.drive"));
                roots.push(PathBuf::from(xdg).join("BeamNG.drive"));
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            roots.push(PathBuf::from(local).join("BeamNG.drive"));
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let home = PathBuf::from(profile);
            roots.push(home.join("Documents/BeamNG.drive"));
            roots.push(home.join("AppData/Local/BeamNG.drive"));
            roots.push(home.join("Saved Games/BeamNG.drive"));
            roots.push(home.join("Steam/steamapps/common/BeamNG.drive"));
        }
        if let Ok(pf) = std::env::var("ProgramFiles(x86)") {
            roots.push(PathBuf::from(pf).join("Steam/steamapps/common/BeamNG.drive"));
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            roots.push(PathBuf::from(pf).join("Steam/steamapps/common/BeamNG.drive"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir();
        if let Some(h) = &home {
            roots.push(h.join("Library/Application Support/BeamNG.drive"));
            roots.push(h.join("Documents/BeamNG.drive"));
            roots.push(h.join("Library/Application Support/Steam/steamapps/common/BeamNG.drive"));
        }
    }

    roots
}

/// Возвращает `mods/<ver>` внутри интересующего нас пользовательского каталога:
/// папки вида `<root>/<ver>/mods`, а так же `<root>/current/mods`.
fn mods_dirs_under(root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if !root.is_dir() {
        return result;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy().to_string();
        let looks_like_version = name.starts_with(|c: char| c.is_ascii_digit())
            || name == "current"
            || name.starts_with("v");
        let mods = path.join("mods");
        if looks_like_version && mods.is_dir() {
            result.push(mods);
        }
    }
    result
}

/// Ищет существующие каталоги модов в известных местах.
pub fn detect_mods_folders() -> Vec<ModsFolderCandidate> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for root in root_candidates() {
        for dir in mods_dirs_under(&root) {
            if seen.insert(dir.clone()) {
                found.push(dir);
            }
        }
    }

    // Плюс каталог с модами как он обычно задаётся userFolder,
    // когда пользователь указал `mods` явно
    if let Some(steam_dir) = dirs::data_dir().map(|d| d.join("BeamNG/BeamNG.drive/current/mods")) {
        if steam_dir.is_dir() && seen.insert(steam_dir.clone()) {
            found.push(steam_dir);
        }
    }

    let mut candidates: Vec<ModsFolderCandidate> = found
        .into_iter()
        .map(|p| ModsFolderCandidate {
            path: p.display().to_string(),
            exists: true,
        })
        .collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    candidates
}

/// Предполагаем, что папка с модами — это каталог, куда BeamNG кладёт `.zip`
/// модов. Если пользователь имеет собственную папку, UI предлагает её вручную.
pub fn is_plausible_mods_folder(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    let has_zip = std::fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| e.path().extension().is_some_and(|ext| ext == "zip"))
        })
        .unwrap_or(false);
    let looks_named_mods = path.file_name().is_some_and(|n| n == "mods");
    has_zip || looks_named_mods
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("bimka-test-{}-{}", std::process::id(), n))
    }

    #[test]
    fn folder_with_zip_is_plausible() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("some.zip"), b"PK").unwrap();
        assert!(is_plausible_mods_folder(&dir));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn folder_named_mods_is_plausible_even_when_empty() {
        let dir = temp_dir().join("userdata").join("mods");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(is_plausible_mods_folder(&dir));
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    #[test]
    fn missing_or_empty_folder_is_not_plausible() {
        let dir = temp_dir();
        assert!(!is_plausible_mods_folder(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!is_plausible_mods_folder(&dir));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn mods_dirs_under_detects_version_folders() {
        let root = temp_dir();
        std::fs::create_dir_all(root.join("current/mods")).unwrap();
        std::fs::create_dir_all(root.join("0.33/mods")).unwrap();
        std::fs::create_dir_all(root.join("not_a_version/mods")).unwrap();
        std::fs::create_dir_all(root.join("v0.34/mods")).unwrap();
        let found = mods_dirs_under(&root);
        let names: Vec<String> = found
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert!(names.iter().any(|n| n == "mods"));
        assert!(found
            .iter()
            .any(|p| p.to_string_lossy().contains("current")));
        assert!(found.iter().any(|p| p.to_string_lossy().contains("0.33")));
        assert!(found.iter().any(|p| p.to_string_lossy().contains("v0.34")));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn root_candidates_include_home_and_proton() {
        let fake_home = temp_dir();
        std::fs::create_dir_all(&fake_home).ok();
        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_DATA_HOME").ok();
        std::env::set_var("HOME", &fake_home);
        std::env::remove_var("XDG_DATA_HOME");

        let roots = root_candidates();
        let roots: Vec<String> = roots.iter().map(|p| p.display().to_string()).collect();

        std::env::remove_var("HOME");
        if let Some(old) = old_home {
            std::env::set_var("HOME", old);
        }
        if let Some(old) = old_xdg {
            std::env::set_var("XDG_DATA_HOME", old);
        }

        let home = fake_home.display().to_string();
        assert!(
            roots.contains(&format!("{home}/.local/share/BeamNG/BeamNG.drive")),
            "roots: {roots:?}"
        );
        assert!(
            roots.contains(&format!("{home}/BeamNG.drive")),
            "roots: {roots:?}"
        );
        let proton = format!("{home}/.local/share/Steam/steamapps/compatdata/284160/pfx/")
            + "drive_c/users/steamuser/AppData/Local/BeamNG.drive";
        assert!(roots.contains(&proton), "roots: {roots:?}");
        let proton_alt = format!("{home}/.steam/steam/steamapps/compatdata/284160/pfx/")
            + "drive_c/users/steamuser/AppData/Local/BeamNG.drive";
        assert!(roots.contains(&proton_alt), "roots: {roots:?}");
        // никаких хардкод-путей чужой машины
        assert!(
            roots.iter().all(|p| !p.contains("/home/samsa")),
            "roots: {roots:?}"
        );
        std::fs::remove_dir_all(&fake_home).ok();
    }
}

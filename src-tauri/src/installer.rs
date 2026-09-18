use crate::models::InstalledMod;
use anyhow::{Context, Result};
use std::path::Path;

/// Список установленных модов: все архивы `.zip` в каталоге модов
/// (включая подкаталог `repo/`, куда BeamNG кладёт моды из официального репозитория).
pub fn list_installed(mods_folder: &Path) -> Result<Vec<InstalledMod>> {
    if !mods_folder.is_dir() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    collect_zips(mods_folder, mods_folder, &mut result)?;
    result.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(result)
}

fn collect_zips(root: &Path, dir: &Path, out: &mut Vec<InstalledMod>) -> Result<()> {
    for entry in std::fs::read_dir(dir).context("не удалось прочитать каталог модов")?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "repo") {
                collect_zips(root, &path, out)?;
            }
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "zip" {
            continue;
        }
        let meta = path.metadata()?;
        let filename = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");
        let source = if filename.starts_with("repo/") {
            "repo".to_string()
        } else {
            "local".to_string()
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out.push(InstalledMod {
            filename,
            path: path.display().to_string(),
            size_bytes: meta.len(),
            modified,
            source,
            key: None,
            published: None,
        });
    }
    Ok(())
}

pub fn remove_file(path: &str) -> Result<()> {
    std::fs::remove_file(path).with_context(|| format!("не удалось удалить {path}"))
}

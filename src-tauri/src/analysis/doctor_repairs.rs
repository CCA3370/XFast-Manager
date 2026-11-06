//! Reversible Health repairs. Only fixed, installation-relative targets may move.
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub static REPAIR_LOCK: Mutex<()> = Mutex::new(());
const BACKUP_DIR: &str = ".xfast-health-backups";
pub const INJECTOR_DLLS: &[&str] = &["dxgi.dll", "d3d11.dll", "d3d12.dll", "opengl32.dll"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairBackup {
    pub id: String,
    pub action: String,
    pub created_at: i64,
    pub paths: Vec<String>,
    pub restored: bool,
}

fn allowed_paths(action: &str) -> Result<Vec<String>, String> {
    Ok(match action {
        "disable_injectors" => INJECTOR_DLLS.iter().map(|name| name.to_string()).collect(),
        "rebuild_shader_cache" => vec!["Output/shadercache".into()],
        "reset_preferences" => vec!["Output/preferences".into()],
        _ => return Err("Unsupported Health repair".into()),
    })
}

fn checked_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err("Health repair path must stay inside the installation".into());
        }
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(format!("Health repair refuses linked paths: {relative}"))
            }
            Ok(_) => {
                // Also covers Windows junctions, which need not report as symlinks.
                let resolved = path.canonicalize().map_err(|e| e.to_string())?;
                let resolved_root = root.canonicalize().map_err(|e| e.to_string())?;
                if !resolved.starts_with(&resolved_root) {
                    return Err(format!(
                        "Health repair path resolves outside the installation: {relative}"
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{relative}: {error}")),
        }
    }
    Ok(path)
}

fn save_manifest(root: &Path, backup: &RepairBackup) -> Result<(), String> {
    let directory = checked_path(root, &format!("{BACKUP_DIR}/{}", backup.id))?;
    let mut file = tempfile::NamedTempFile::new_in(&directory).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(backup).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(directory.join("manifest.json"))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn read_manifest(root: &Path, id: &str) -> Result<RepairBackup, String> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err("Invalid Health backup id".into());
    }
    let path = checked_path(root, &format!("{BACKUP_DIR}/{id}/manifest.json"))?;
    let backup: RepairBackup = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let allowed = allowed_paths(&backup.action)?;
    let unique: std::collections::HashSet<_> = backup.paths.iter().collect();
    if backup.id != id
        || backup.paths.is_empty()
        || unique.len() != backup.paths.len()
        || backup.paths.iter().any(|path| !allowed.contains(path))
    {
        return Err("Invalid Health backup manifest".into());
    }
    Ok(backup)
}

fn stored_path(root: &Path, id: &str, path: &str) -> Result<PathBuf, String> {
    checked_path(root, &format!("{BACKUP_DIR}/{id}/files/{path}"))
}

pub fn list_backups(root: &Path) -> Result<Vec<RepairBackup>, String> {
    let directory = checked_path(root, BACKUP_DIR)?;
    if !directory.try_exists().map_err(|e| e.to_string())? {
        return Ok(vec![]);
    }
    let mut backups = Vec::new();
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let id = entry.file_name().to_string_lossy().to_string();
        let backup = read_manifest(root, &id)?;
        if !backup.restored
            && backup
                .paths
                .iter()
                .any(|path| stored_path(root, &id, path).is_ok_and(|p| p.exists()))
        {
            backups.push(backup);
        }
    }
    backups.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(backups)
}

fn backup_paths(root: &Path, action: &str, paths: Vec<String>) -> Result<RepairBackup, String> {
    if paths.is_empty() {
        return Err("There are no matching files to repair".into());
    }
    let backup = RepairBackup {
        id: uuid::Uuid::new_v4().to_string(),
        action: action.into(),
        created_at: chrono::Utc::now().timestamp_millis(),
        paths,
        restored: false,
    };
    // Validate every source before the first mutation.
    for path in &backup.paths {
        checked_path(root, path)?;
    }
    let directory = checked_path(root, &format!("{BACKUP_DIR}/{}", backup.id))?;
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    // The manifest is durable before moving anything, making interrupted work recoverable.
    save_manifest(root, &backup)?;
    let mut moved: Vec<String> = Vec::new();
    for path in &backup.paths {
        let operation = (|| {
            let source = checked_path(root, path)?;
            let destination = stored_path(root, &backup.id, path)?;
            fs::create_dir_all(destination.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::rename(source, destination).map_err(|e| format!("{path}: {e}"))
        })();
        if let Err(error) = operation {
            let mut errors = vec![error];
            for previous in moved.iter().rev() {
                let rollback = (|| {
                    let stored = stored_path(root, &backup.id, previous)?;
                    let original = checked_path(root, previous)?;
                    fs::rename(stored, original).map_err(|e| e.to_string())
                })();
                if let Err(error) = rollback {
                    errors.push(format!(
                        "Rollback {previous}: {error}; backup {} remains available",
                        backup.id
                    ));
                }
            }
            return Err(errors.join("\n"));
        }
        moved.push(path.clone());
    }
    Ok(backup)
}

pub fn apply_repair(root: &Path, action: &str) -> Result<RepairBackup, String> {
    let allowed = allowed_paths(action)?;
    if action == "disable_injectors"
        && !root.join("ReShade.ini").is_file()
        && !root.join("reshade-shaders").is_dir()
    {
        return Err("No ReShade configuration accompanies these DLLs".into());
    }
    let mut paths = Vec::new();
    for path in allowed {
        if checked_path(root, &path)?
            .try_exists()
            .map_err(|e| e.to_string())?
        {
            paths.push(path);
        }
    }
    backup_paths(root, action, paths)
}

pub fn restore_backup(root: &Path, id: &str) -> Result<(), String> {
    let mut backup = read_manifest(root, id)?;
    if backup.restored {
        return Err("This Health backup was already restored".into());
    }
    let mut paths = Vec::new();
    let mut current = Vec::new();
    for path in &backup.paths {
        if stored_path(root, id, path)?
            .try_exists()
            .map_err(|e| e.to_string())?
        {
            paths.push(path.clone());
            if checked_path(root, path)?
                .try_exists()
                .map_err(|e| e.to_string())?
            {
                current.push(path.clone());
            }
        }
    }
    if paths.is_empty() {
        return Err("No recoverable files remain in this Health backup".into());
    }
    // Preserve files generated since the repair as another restorable backup.
    let displaced = if current.is_empty() {
        None
    } else {
        Some(backup_paths(root, &backup.action, current)?)
    };
    let mut restored: Vec<String> = Vec::new();
    for path in &paths {
        let operation = (|| {
            let source = stored_path(root, id, path)?;
            let destination = checked_path(root, path)?;
            fs::create_dir_all(destination.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::rename(source, destination).map_err(|e| format!("{path}: {e}"))
        })();
        if let Err(error) = operation {
            let mut errors = vec![error];
            for previous in restored.iter().rev() {
                let result = (|| {
                    fs::rename(
                        checked_path(root, previous)?,
                        stored_path(root, id, previous)?,
                    )
                    .map_err(|e| e.to_string())
                })();
                if let Err(error) = result {
                    errors.push(format!("Restore rollback {previous}: {error}"));
                }
            }
            if let Some(displaced) = displaced {
                for previous in &displaced.paths {
                    let result = (|| {
                        let destination = checked_path(root, previous)?;
                        if destination.exists() {
                            return Err(
                                "Destination is occupied; preserved backup remains available"
                                    .into(),
                            );
                        }
                        fs::rename(stored_path(root, &displaced.id, previous)?, destination)
                            .map_err(|e| e.to_string())
                    })();
                    if let Err(error) = result {
                        errors.push(format!("Restore current files {previous}: {error}"));
                    }
                }
            }
            return Err(errors.join("\n"));
        }
        restored.push(path.clone());
    }
    backup.restored = true;
    save_manifest(root, &backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_can_be_reset_and_restored_without_losing_new_files() {
        let temp = tempfile::tempdir().unwrap();
        let prefs = temp.path().join("Output/preferences");
        fs::create_dir_all(&prefs).unwrap();
        fs::write(prefs.join("X-Plane.prf"), "original").unwrap();
        let backup = apply_repair(temp.path(), "reset_preferences").unwrap();
        assert!(!prefs.exists());
        fs::create_dir_all(&prefs).unwrap();
        fs::write(prefs.join("X-Plane.prf"), "regenerated").unwrap();
        restore_backup(temp.path(), &backup.id).unwrap();
        assert_eq!(
            fs::read_to_string(prefs.join("X-Plane.prf")).unwrap(),
            "original"
        );
        let current = list_backups(temp.path()).unwrap();
        assert_eq!(current.len(), 1);
        restore_backup(temp.path(), &current[0].id).unwrap();
        assert_eq!(
            fs::read_to_string(prefs.join("X-Plane.prf")).unwrap(),
            "regenerated"
        );
    }

    #[test]
    fn injector_repair_moves_only_accompanied_proxy_dlls() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("dxgi.dll"), "proxy").unwrap();
        fs::write(temp.path().join("other.dll"), "keep").unwrap();
        assert!(apply_repair(temp.path(), "disable_injectors").is_err());
        fs::write(temp.path().join("ReShade.ini"), "settings").unwrap();
        let backup = apply_repair(temp.path(), "disable_injectors").unwrap();
        assert!(temp.path().join("other.dll").exists());
        assert!(temp.path().join("ReShade.ini").exists());
        assert!(!temp.path().join("dxgi.dll").exists());
        restore_backup(temp.path(), &backup.id).unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("dxgi.dll")).unwrap(),
            "proxy"
        );
    }

    #[test]
    fn rejects_traversal_and_tampered_manifests() {
        let temp = tempfile::tempdir().unwrap();
        assert!(checked_path(temp.path(), "../outside").is_err());
        assert!(restore_backup(temp.path(), "../outside").is_err());
        fs::create_dir_all(temp.path().join("Output/shadercache")).unwrap();
        let mut backup = apply_repair(temp.path(), "rebuild_shader_cache").unwrap();
        backup.paths = vec!["../outside".into()];
        save_manifest(temp.path(), &backup).unwrap();
        assert!(restore_backup(temp.path(), &backup.id).is_err());
    }

    #[test]
    fn failed_multi_file_repair_rolls_back_earlier_moves() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("dxgi.dll"), "original").unwrap();
        assert!(backup_paths(
            temp.path(),
            "disable_injectors",
            vec!["dxgi.dll".into(), "d3d11.dll".into()]
        )
        .is_err());
        assert_eq!(
            fs::read_to_string(temp.path().join("dxgi.dll")).unwrap(),
            "original"
        );
        assert!(list_backups(temp.path()).unwrap().is_empty());
    }

    #[test]
    fn interrupted_repair_keeps_already_moved_files_recoverable() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("dxgi.dll"), "original").unwrap();
        let backup = apply_repair(temp.path(), "disable_injectors");
        assert!(backup.is_err()); // unaccompanied DLL cannot be disabled
        let backup = RepairBackup {
            id: uuid::Uuid::new_v4().to_string(),
            action: "disable_injectors".into(),
            created_at: 1,
            paths: vec!["dxgi.dll".into(), "d3d11.dll".into()],
            restored: false,
        };
        let stored = stored_path(temp.path(), &backup.id, "dxgi.dll").unwrap();
        fs::create_dir_all(stored.parent().unwrap()).unwrap();
        save_manifest(temp.path(), &backup).unwrap();
        fs::rename(temp.path().join("dxgi.dll"), stored).unwrap();
        assert_eq!(list_backups(temp.path()).unwrap().len(), 1);
        restore_backup(temp.path(), &backup.id).unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("dxgi.dll")).unwrap(),
            "original"
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_linked_repair_targets() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("Output")).unwrap();
        assert!(apply_repair(temp.path(), "reset_preferences").is_err());
    }
}

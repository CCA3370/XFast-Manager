use super::{folder_size, is_link, validate_target_path, OutputCleanupTarget};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const DIRECTORY: &str = ".xfast-output-disabled";
const MANIFEST: &str = "item.json";
const DATA: &str = "data";

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum OutputCleanupStateChange {
    Disable { target: OutputCleanupTarget },
    Restore { id: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DisabledOutputItem {
    pub(super) id: String,
    relative_path: String,
    pub(super) saved_path: String,
    saved_at: u64,
    size_bytes: u64,
    file_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    relative_path: String,
    saved_at: u64,
}

fn check_path(path: &Path, directory: bool) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link(&metadata)
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file() && !metadata.is_dir())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported file type at {}", path.display()),
        ));
    }
    Ok(())
}

fn vault(xplane: &Path, create: bool) -> io::Result<PathBuf> {
    let path = xplane.join(DIRECTORY);
    if create {
        match fs::create_dir(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    check_path(&path, true)?;
    Ok(path)
}

fn read_manifest(slot: &Path) -> io::Result<Manifest> {
    check_path(slot, true)?;
    check_path(&slot.join(DATA), false)?;
    let path = slot.join(MANIFEST);
    let metadata = fs::symlink_metadata(&path)?;
    if is_link(&metadata) || !metadata.is_file() || metadata.len() > 16_384 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid isolation record",
        ));
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn scan(xplane: &Path) -> Vec<DisabledOutputItem> {
    let Ok(entries) = vault(xplane, false).and_then(fs::read_dir) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for entry in entries.flatten() {
        let id = entry.file_name().to_string_lossy().into_owned();
        if !valid_id(&id) {
            continue;
        }
        let Ok(manifest) = read_manifest(&entry.path()) else {
            continue;
        };
        let (size_bytes, file_count) = folder_size(&entry.path().join(DATA));
        items.push(DisabledOutputItem {
            saved_path: format!("{DIRECTORY}/{id}/{DATA}"),
            id,
            relative_path: manifest.relative_path,
            saved_at: manifest.saved_at,
            size_bytes,
            file_count,
        });
    }
    items.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then_with(|| a.id.cmp(&b.id)));
    items
}

fn remove_empty_slot(slot: &Path) {
    // The payload has already been moved or was never created.
    let _ = fs::remove_file(slot.join(MANIFEST));
    let _ = fs::remove_dir(slot);
}

fn save_item(xplane: &Path, source: &Path, relative_path: &str) -> io::Result<PathBuf> {
    check_path(source, false)?;
    let slot = vault(xplane, true)?.join(Uuid::new_v4().simple().to_string());
    fs::create_dir(&slot)?;
    let result = (|| {
        let manifest = Manifest {
            relative_path: relative_path.to_string(),
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        let mut record = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(slot.join(MANIFEST))?;
        serde_json::to_writer(&mut record, &manifest)?;
        record.flush()?;
        record.sync_all()?;
        rename_without_overwrite(source, &slot.join(DATA))
    })();
    if let Err(error) = result {
        remove_empty_slot(&slot);
        return Err(error);
    }
    Ok(slot)
}

pub(super) fn disable(xplane: &Path, relative_path: &str) -> Result<(), String> {
    let source = validate_target_path(xplane, relative_path)?;
    save_item(xplane, &source, relative_path)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub(super) fn restore(xplane: &Path, id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("Invalid disabled Output item ID".into());
    }
    let slot = vault(xplane, false).map_err(|e| e.to_string())?.join(id);
    let manifest = read_manifest(&slot).map_err(|e| e.to_string())?;
    let target = validate_target_path(xplane, &manifest.relative_path)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let replacement = match fs::symlink_metadata(&target) {
        Ok(_) => {
            Some(save_item(xplane, &target, &manifest.relative_path).map_err(|e| e.to_string())?)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    if let Err(error) = rename_without_overwrite(&slot.join(DATA), &target) {
        if let Some(replacement) = replacement {
            if rename_without_overwrite(&replacement.join(DATA), &target).is_ok() {
                remove_empty_slot(&replacement);
            }
        }
        return Err(error.to_string());
    }
    remove_empty_slot(&slot);
    Ok(())
}

#[cfg(windows)]
fn rename_without_overwrite(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winbase::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    let source: Vec<u16> = source.as_os_str().encode_wide().chain([0]).collect();
    let destination: Vec<u16> = destination.as_os_str().encode_wide().chain([0]).collect();
    if unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rename_without_overwrite(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let source = CString::new(source.as_os_str().as_bytes())?;
    let destination = CString::new(destination.as_os_str().as_bytes())?;
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(target_os = "macos")]
    let result =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_preserves_a_destination_created_before_the_move() {
        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("original");
        let occupied = root.path().join("occupied");
        fs::create_dir(&original).unwrap();
        fs::create_dir(&occupied).unwrap();
        fs::write(original.join("settings"), "important").unwrap();
        assert!(rename_without_overwrite(&original, &occupied).is_err());
        assert!(occupied.is_dir());
        assert_eq!(
            fs::read_to_string(original.join("settings")).unwrap(),
            "important"
        );
    }
}

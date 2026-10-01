//! Manage user flight plans without overwriting existing routes or following links outside Output.
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

const MAX_PLAN_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightPlan {
    name: String,
    size: u64,
    modified: u64,
}

fn directory(root: &str, create: bool) -> Result<PathBuf, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    if !root.join("Resources").is_dir() {
        return Err("Select a valid X-Plane installation".into());
    }
    let mut dir = root.clone();
    for component in ["Output", "FMS plans"] {
        dir.push(component);
        if let Ok(meta) = fs::symlink_metadata(&dir) {
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err("Flight plan folder must be a regular directory".into());
            }
            if !fs::canonicalize(&dir)
                .map_err(|e| e.to_string())?
                .starts_with(&root)
            {
                return Err("Flight plan folder is outside X-Plane".into());
            }
        } else if create {
            fs::create_dir(&dir).map_err(|e| e.to_string())?;
        }
    }
    Ok(dir)
}

fn valid_name(name: &str) -> bool {
    let path = Path::new(name);
    !name.contains(['/', '\\', ':'])
        && matches!(path.components().next(), Some(Component::Normal(_)))
        && path.components().count() == 1
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("fms"))
}

pub fn list(root: &str) -> Result<Vec<FlightPlan>, String> {
    let dir = directory(root, false)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut plans = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        if valid_name(&name) && entry.file_type().map_err(|e| e.to_string())?.is_file() {
            plans.push(FlightPlan {
                name,
                size: meta.len(),
                modified: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs()),
            });
        }
    }
    plans.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(plans)
}

pub fn import(root: &str, source: &str) -> Result<String, String> {
    let source = Path::new(source);
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file name")?;
    if !valid_name(name) {
        return Err("Choose an .fms flight plan file".into());
    }
    let file = fs::File::open(source).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Choose a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_PLAN_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_PLAN_BYTES {
        return Err("Flight plan exceeds 8 MB".into());
    }
    let content = std::str::from_utf8(&bytes).map_err(|_| "Flight plan must be text")?;
    let mut lines = content.trim_start_matches('\u{feff}').lines();
    if !matches!(lines.next().map(str::trim), Some("I" | "A"))
        || !lines.next().is_some_and(|line| {
            matches!(line.trim(), "3 version" | "1100 Version" | "1100 version")
        })
    {
        return Err("Invalid or unsupported X-Plane FMS flight plan".into());
    }
    let dir = directory(root, true)?;
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid file name")?;
    // Stage on the destination filesystem, then publish without replacing an existing file.
    let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    temp.write_all(&bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    for index in 0..10000 {
        let candidate = if index == 0 {
            name.to_owned()
        } else {
            format!("{stem} ({index}).fms")
        };
        match temp.persist_noclobber(dir.join(&candidate)) {
            Ok(_) => return Ok(candidate),
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => temp = e.file,
            Err(e) => return Err(e.error.to_string()),
        }
    }
    Err("Too many flight plans with this name".into())
}

pub fn delete(root: &str, name: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err("Invalid flight plan name".into());
    }
    let target = directory(root, false)?.join(name);
    if !fs::symlink_metadata(&target)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_file()
    {
        return Err("Flight plan must be a regular file".into());
    }
    fs::remove_file(target).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_preserve_existing_files_and_reject_invalid_inputs() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("Resources")).unwrap();
        let source = tempfile::tempdir().unwrap();
        let plan = source.path().join("route.fms");
        fs::write(&plan, "I\n1100 Version\nCYCLE 2609\nNUMENR 0\n").unwrap();
        let root_str = root.path().to_str().unwrap();
        assert!(list(root_str).unwrap().is_empty());
        assert_eq!(
            import(root_str, plan.to_str().unwrap()).unwrap(),
            "route.fms"
        );
        assert_eq!(
            import(root_str, plan.to_str().unwrap()).unwrap(),
            "route (1).fms"
        );
        assert_eq!(list(root_str).unwrap().len(), 2);
        assert!(delete(root_str, "../route.fms").is_err());
        delete(root_str, "route.fms").unwrap();
        fs::write(&plan, "not a flight plan").unwrap();
        assert!(import(root_str, plan.to_str().unwrap()).is_err());
        assert_eq!(list(root_str).unwrap().len(), 1);
    }
    #[cfg(unix)]
    #[test]
    fn refuses_linked_output_and_files() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("Resources")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("Output")).unwrap();
        assert!(list(root.path().to_str().unwrap()).is_err());
    }
}

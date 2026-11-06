//! Manage user flight plans without overwriting existing routes or following links outside Output.
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

const MAX_PLAN_BYTES: u64 = 8 * 1024 * 1024;
static IMPORT_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictAction {
    Replace,
    Rename,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ImportResult {
    Imported {
        name: String,
        renamed: bool,
        replaced: bool,
    },
    Duplicate {
        name: String,
    },
    Conflict {
        name: String,
        #[serde(rename = "existingSize")]
        existing_size: u64,
        #[serde(rename = "incomingSize")]
        incoming_size: u64,
    },
}

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

pub fn import(
    root: &str,
    source: &str,
    conflict_action: Option<ConflictAction>,
) -> Result<ImportResult, String> {
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
    // Serialize check-and-publish so concurrent imports cannot create duplicates.
    let _guard = IMPORT_LOCK.lock().map_err(|error| error.to_string())?;
    let dir = directory(root, true)?;
    let normalized = normalized_content(content);
    let mut existing: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    existing.sort_by_key(|entry| entry.file_name());
    for entry in existing {
        let existing_name = entry.file_name().to_string_lossy().into_owned();
        if !valid_name(&existing_name) || !entry.file_type().map_err(|e| e.to_string())?.is_file() {
            continue;
        }
        let mut existing_bytes = Vec::new();
        fs::File::open(entry.path())
            .map_err(|e| format!("Cannot compare {existing_name}: {e}"))?
            .take(MAX_PLAN_BYTES + 1)
            .read_to_end(&mut existing_bytes)
            .map_err(|e| e.to_string())?;
        if existing_bytes.len() as u64 <= MAX_PLAN_BYTES
            && std::str::from_utf8(&existing_bytes)
                .is_ok_and(|text| normalized_content(text) == normalized)
        {
            return Ok(ImportResult::Duplicate {
                name: existing_name,
            });
        }
    }

    let destination = dir.join(name);
    let destination_meta = match fs::symlink_metadata(&destination) {
        Ok(meta) if meta.file_type().is_file() => Some(meta),
        Ok(_) => return Err("Flight plan destination must be a regular file".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    if let (Some(meta), None) = (&destination_meta, conflict_action) {
        return Ok(ImportResult::Conflict {
            name: name.into(),
            existing_size: meta.len(),
            incoming_size: bytes.len() as u64,
        });
    }

    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid file name")?;
    let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    temp.write_all(&bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    if matches!(conflict_action, Some(ConflictAction::Replace)) && destination_meta.is_some() {
        temp.persist(&destination)
            .map_err(|e| e.error.to_string())?;
        return Ok(ImportResult::Imported {
            name: name.into(),
            renamed: false,
            replaced: true,
        });
    }
    for index in 0..10000 {
        let candidate = if index == 0 {
            name.to_owned()
        } else {
            format!("{stem}.{index}.fms")
        };
        match temp.persist_noclobber(dir.join(&candidate)) {
            Ok(_) => {
                return Ok(ImportResult::Imported {
                    name: candidate,
                    renamed: index != 0,
                    replaced: false,
                })
            }
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temp = e.file;
                if !matches!(conflict_action, Some(ConflictAction::Rename)) {
                    // A file appeared after inspection. Ask again rather than silently overwrite it.
                    let meta = fs::symlink_metadata(&destination).map_err(|e| e.to_string())?;
                    if !meta.file_type().is_file() {
                        return Err("Flight plan destination must be a regular file".into());
                    }
                    return Ok(ImportResult::Conflict {
                        name: name.into(),
                        existing_size: meta.len(),
                        incoming_size: bytes.len() as u64,
                    });
                }
            }
            Err(e) => return Err(e.error.to_string()),
        }
    }
    Err("Too many flight plans with this name".into())
}

// Encoding markers and platform line endings do not make a different flight plan.
fn normalized_content(text: &str) -> String {
    text.trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .trim_end_matches('\n')
        .to_owned()
}

/// Read only a regular, bounded plan inside this installation's managed folder.
pub fn read(root: &str, name: &str) -> Result<String, String> {
    if !valid_name(name) {
        return Err("Invalid flight plan name".into());
    }
    let path = directory(root, false)?.join(name);
    if !fs::symlink_metadata(&path)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_file()
    {
        return Err("Flight plan must be a regular file".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_PLAN_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_PLAN_BYTES {
        return Err("Flight plan exceeds 8 MB".into());
    }
    String::from_utf8(bytes).map_err(|_| "Flight plan must be UTF-8 text".into())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavPoint {
    kind: u8,
    id: String,
    latitude: f64,
    longitude: f64,
    region: String,
    terminal: String,
    name: String,
    source: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavLookup {
    points: Vec<NavPoint>,
    sources: Vec<String>,
    warnings: Vec<String>,
}

/// Targeted streaming lookup: never load the entire navigation database into memory.
pub fn navigation(root: &str, identifiers: Vec<String>) -> Result<NavLookup, String> {
    use std::collections::{HashMap, HashSet};
    use std::io::{BufRead, BufReader};
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    if !root.join("Resources").is_dir() {
        return Err("Select a valid X-Plane installation".into());
    }
    if identifiers.len() > 10000 {
        return Err("Too many navigation identifiers".into());
    }
    let wanted: HashSet<_> = identifiers.into_iter().collect();
    let mut output = NavLookup {
        points: vec![],
        sources: vec![],
        warnings: vec![],
    };
    if root.join("Custom Data/earth_424.dat").exists() {
        output.warnings.push("arincOverride".into());
        return Ok(output);
    }
    let custom = root.join("Custom Data/earth_fix.dat").exists()
        || root.join("Custom Data/earth_nav.dat").exists();
    let base = if custom {
        "Custom Data"
    } else {
        "Resources/default data"
    };
    let mut points = HashMap::new();
    for (relative, fix) in [
        (format!("{base}/earth_fix.dat"), true),
        (format!("{base}/earth_nav.dat"), false),
        ("Custom Data/user_fix.dat".into(), true),
        ("Custom Data/user_nav.dat".into(), false),
    ] {
        let path = root.join(&relative);
        if !path.exists() {
            continue;
        }
        let result = (|| -> Result<(), String> {
            let file = fs::File::open(&path).map_err(|e| e.to_string())?;
            if file.metadata().map_err(|e| e.to_string())?.len() > 512 * 1024 * 1024 {
                return Err("Navigation file too large".into());
            }
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            loop {
                line.clear();
                let count = reader
                    .by_ref()
                    .take(16385)
                    .read_line(&mut line)
                    .map_err(|e| e.to_string())?;
                if count == 0 {
                    break;
                }
                if count > 16384 {
                    return Err("Navigation line too long".into());
                }
                let c: Vec<_> = line.split_whitespace().collect();
                let (kind, id, lat, lon, terminal, region, name) = if fix && c.len() >= 5 {
                    (11, c[2], c[0], c[1], c[3], c[4], String::new())
                } else if !fix && c.len() >= 10 && matches!(c[0], "2" | "3") {
                    (
                        c[0].parse().unwrap_or(0),
                        c[7],
                        c[1],
                        c[2],
                        c[8],
                        c[9],
                        c[10..].join(" "),
                    )
                } else {
                    continue;
                };
                if !wanted.contains(id) {
                    continue;
                }
                let (Ok(latitude), Ok(longitude)) = (lat.parse::<f64>(), lon.parse::<f64>()) else {
                    continue;
                };
                if !latitude.is_finite()
                    || !longitude.is_finite()
                    || latitude.abs() > 90.0
                    || longitude.abs() > 180.0
                {
                    continue;
                }
                if points.len() >= 100000 {
                    return Err("Too many navigation matches".into());
                }
                points.insert(
                    (
                        kind,
                        id.to_string(),
                        region.to_string(),
                        terminal.to_string(),
                    ),
                    NavPoint {
                        kind,
                        id: id.into(),
                        latitude,
                        longitude,
                        region: region.into(),
                        terminal: terminal.into(),
                        name,
                        source: relative.clone(),
                    },
                );
            }
            Ok(())
        })();
        if result.is_err() {
            output.warnings.push(format!("unreadable: {relative}"));
        } else {
            output.sources.push(relative);
        }
    }
    output.points = points.into_values().collect();
    output
        .points
        .sort_by(|a, b| (&a.id, &a.region, &a.terminal).cmp(&(&b.id, &b.region, &b.terminal)));
    if output.sources.is_empty() {
        output.warnings.push("noNavdata".into());
    }
    Ok(output)
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

    fn fixture() -> (tempfile::TempDir, tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("Resources")).unwrap();
        let source = tempfile::tempdir().unwrap();
        let plan = source.path().join("route.fms");
        fs::write(&plan, "I\n1100 Version\nCYCLE 2611\nNUMENR 0\n").unwrap();
        (root, source, plan)
    }
    fn run(root: &Path, plan: &Path, action: Option<ConflictAction>) -> ImportResult {
        import(root.to_str().unwrap(), plan.to_str().unwrap(), action).unwrap()
    }
    #[test]
    fn read_is_bounded_and_rejects_traversal_and_non_files() {
        let (root, _source, plan) = fixture();
        run(root.path(), &plan, None);
        let root_path = root.path().to_str().unwrap();
        assert_eq!(
            read(root_path, "route.fms").unwrap(),
            fs::read_to_string(plan).unwrap()
        );
        assert!(read(root_path, "../route.fms").is_err());
        fs::create_dir(root.path().join("Output/FMS plans/folder.fms")).unwrap();
        assert!(read(root_path, "folder.fms").is_err());
        fs::write(
            root.path().join("Output/FMS plans/large.fms"),
            vec![b'x'; MAX_PLAN_BYTES as usize + 1],
        )
        .unwrap();
        assert!(read(root_path, "large.fms").is_err());
    }

    #[test]
    fn nav_lookup_selects_custom_layers_and_user_overrides_without_guessing_regions() {
        let (root, _source, _) = fixture();
        fs::create_dir_all(root.path().join("Resources/default data")).unwrap();
        fs::create_dir_all(root.path().join("Custom Data")).unwrap();
        fs::write(
            root.path().join("Resources/default data/earth_fix.dat"),
            "1 2 TEST ENRT K1 0",
        )
        .unwrap();
        fs::write(
            root.path().join("Custom Data/earth_fix.dat"),
            "3 4 TEST ENRT K1 0\n5 6 TEST ENRT K2 0\n7 8 OTHER ENRT K1 0",
        )
        .unwrap();
        fs::write(
            root.path().join("Custom Data/user_fix.dat"),
            "9 10 TEST ENRT K1 0",
        )
        .unwrap();
        let output = navigation(root.path().to_str().unwrap(), vec!["TEST".into()]).unwrap();
        assert_eq!(output.points.len(), 2);
        assert!(output
            .points
            .iter()
            .any(|p| p.latitude == 9.0 && p.region == "K1"));
        assert!(output
            .points
            .iter()
            .any(|p| p.latitude == 5.0 && p.region == "K2"));
        fs::write(root.path().join("Custom Data/earth_424.dat"), "override").unwrap();
        let output = navigation(root.path().to_str().unwrap(), vec!["TEST".into()]).unwrap();
        assert!(output.points.is_empty());
        assert_eq!(output.warnings, vec!["arincOverride"]);
    }

    #[test]
    fn identical_content_is_skipped_even_under_another_name_or_line_endings() {
        let (root, _source, plan) = fixture();
        assert_eq!(
            run(root.path(), &plan, None),
            ImportResult::Imported {
                name: "route.fms".into(),
                renamed: false,
                replaced: false
            }
        );
        let another = plan.with_file_name("another.fms");
        fs::write(
            &another,
            "\u{feff}I\r\n1100 Version\r\nCYCLE 2611\r\nNUMENR 0\r\n",
        )
        .unwrap();
        for file in [&plan, &another] {
            assert_eq!(
                run(root.path(), file, Some(ConflictAction::Replace)),
                ImportResult::Duplicate {
                    name: "route.fms".into()
                }
            );
        }
        assert_eq!(list(root.path().to_str().unwrap()).unwrap().len(), 1);
    }
    #[test]
    fn conflict_without_a_decision_never_changes_existing_files() {
        let (root, _source, plan) = fixture();
        run(root.path(), &plan, None);
        let original = fs::read(&plan).unwrap();
        fs::write(&plan, "I\n1100 Version\nCYCLE 2612\nNUMENR 0\n").unwrap();
        assert!(matches!(
            run(root.path(), &plan, None),
            ImportResult::Conflict { .. }
        ));
        assert_eq!(
            fs::read(root.path().join("Output/FMS plans/route.fms")).unwrap(),
            original
        );
        assert_eq!(list(root.path().to_str().unwrap()).unwrap().len(), 1);
    }
    #[test]
    fn renaming_uses_numbered_suffixes_and_detects_duplicates_of_renamed_plans() {
        let (root, _source, plan) = fixture();
        run(root.path(), &plan, None);
        for index in 1..=2 {
            fs::write(
                &plan,
                format!("I\n1100 Version\nCYCLE {}\nNUMENR 0\n", 2611 + index),
            )
            .unwrap();
            assert_eq!(
                run(root.path(), &plan, Some(ConflictAction::Rename)),
                ImportResult::Imported {
                    name: format!("route.{index}.fms"),
                    renamed: true,
                    replaced: false
                }
            );
        }
        assert_eq!(
            run(root.path(), &plan, Some(ConflictAction::Rename)),
            ImportResult::Duplicate {
                name: "route.2.fms".into()
            }
        );
        assert_eq!(list(root.path().to_str().unwrap()).unwrap().len(), 3);
    }
    #[test]
    fn replace_updates_only_the_selected_file_and_preserves_other_plans() {
        let (root, _source, plan) = fixture();
        run(root.path(), &plan, None);
        let unrelated = root.path().join("Output/FMS plans/keep.fms");
        fs::write(&unrelated, "keep untouched").unwrap();
        fs::write(&plan, "I\n1100 Version\nCYCLE 2612\nNUMENR 0\n").unwrap();
        assert_eq!(
            run(root.path(), &plan, Some(ConflictAction::Replace)),
            ImportResult::Imported {
                name: "route.fms".into(),
                renamed: false,
                replaced: true
            }
        );
        assert_eq!(
            fs::read(root.path().join("Output/FMS plans/route.fms")).unwrap(),
            fs::read(&plan).unwrap()
        );
        assert_eq!(fs::read_to_string(unrelated).unwrap(), "keep untouched");
    }
    #[test]
    fn concurrent_identical_imports_create_only_one_plan() {
        let (root, _source, plan) = fixture();
        std::thread::scope(|scope| {
            let jobs: Vec<_> = (0..4)
                .map(|_| scope.spawn(|| run(root.path(), &plan, Some(ConflictAction::Rename))))
                .collect();
            let results: Vec<_> = jobs.into_iter().map(|job| job.join().unwrap()).collect();
            assert_eq!(
                results
                    .iter()
                    .filter(|result| matches!(result, ImportResult::Imported { .. }))
                    .count(),
                1
            );
            assert_eq!(
                results
                    .iter()
                    .filter(|result| matches!(result, ImportResult::Duplicate { .. }))
                    .count(),
                3
            );
        });
    }
    #[test]
    fn invalid_sources_and_directory_conflicts_are_rejected() {
        let (root, _source, plan) = fixture();
        fs::write(&plan, "invalid flight plan").unwrap();
        assert!(import(root.path().to_str().unwrap(), plan.to_str().unwrap(), None).is_err());
        fs::write(&plan, "I\n1100 Version\n").unwrap();
        fs::create_dir_all(root.path().join("Output/FMS plans/route.fms")).unwrap();
        assert!(import(
            root.path().to_str().unwrap(),
            plan.to_str().unwrap(),
            Some(ConflictAction::Replace)
        )
        .is_err());
        assert!(delete(root.path().to_str().unwrap(), "../route.fms").is_err());
    }
    #[test]
    fn import_result_serializes_the_frontend_contract() {
        let result = ImportResult::Conflict {
            name: "route.fms".into(),
            existing_size: 42,
            incoming_size: 43,
        };
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({"status":"conflict","name":"route.fms","existingSize":42,"incomingSize":43})
        );
    }
    #[test]
    fn deleting_a_plan_preserves_other_files() {
        let (root, _source, plan) = fixture();
        run(root.path(), &plan, None);
        fs::write(root.path().join("Output/FMS plans/keep.fms"), "keep").unwrap();
        delete(root.path().to_str().unwrap(), "route.fms").unwrap();
        let remaining = list(root.path().to_str().unwrap()).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "keep.fms");
    }
    #[cfg(unix)]
    #[test]
    fn refuses_linked_output_and_files() {
        let (root, outside, plan) = fixture();
        std::os::unix::fs::symlink(outside.path(), root.path().join("Output")).unwrap();
        assert!(list(root.path().to_str().unwrap()).is_err());
        assert!(import(
            root.path().to_str().unwrap(),
            plan.to_str().unwrap(),
            Some(ConflictAction::Replace)
        )
        .is_err());
    }
}

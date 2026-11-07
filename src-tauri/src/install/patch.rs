//! Aircraft patch planning, validated overlays, snapshots and restoration.
//! Matching compares every archive path against every installed aircraft tree.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::{AddonType, InstallTask, PatchFileWrite};
use crate::scanner::Scanner;

#[path = "patch_matching.rs"]
mod matching;
#[cfg(test)]
use matching::ProbeTarget;
use matching::{discover, rank_candidates};

const MIN_RECOMMEND_MATCH: usize = 3;
const MIN_RECOMMEND_RATIO: f64 = 0.25;

// --------------------------------------------------------------------------
// Serializable results (camelCase for the frontend)
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AircraftCandidate {
    pub folder_name: String,
    pub display_name: String,
    /// Number of complete patch-tree paths that already exist in this aircraft.
    pub matched_count: usize,
    /// Rarity-weighted identity evidence across the entire aircraft library.
    pub match_score: f64,
    pub file_count: usize,
    pub best_destination: String,
    pub ambiguous_mapping: bool,
    /// Archive prefix that aligned best ("" = archive root).
    pub best_offset: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDetection {
    pub candidates: Vec<AircraftCandidate>,
    pub recommended_folder: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchMapping {
    /// Subfolder inside the archive to take content from ("" = archive root).
    pub archive_subpath: String,
    /// Destination subpath inside the aircraft folder ("" = aircraft root).
    pub dest_subpath: String,
    pub file_count: usize,
    pub confidence: Confidence,
    /// Human-readable justification shown in the UI.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPlan {
    /// Directory paths inside the archive (for manual mapping edits).
    pub archive_tree: Vec<String>,
    pub suggested_mappings: Vec<PatchMapping>,
    /// Archive top-level entries not covered by any suggested mapping.
    pub unmapped: Vec<String>,
    /// Immediate sub-directories of the target aircraft (for the dest dropdown).
    pub aircraft_subdirs: Vec<String>,
}

/// Plain-language preview of what an install will do, for the confirmation UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchInstallSummary {
    /// Distinct files that will be written across all mappings.
    pub total_files: usize,
    /// How many of those already exist at their destination (and so are
    /// overwritten — the rest are newly added).
    pub overwrite_count: usize,
    pub plan_id: String,
}

// --------------------------------------------------------------------------
// Level 1 — detect the target aircraft
// --------------------------------------------------------------------------

/// Compare the complete patch file tree to every installed aircraft.
pub fn detect_target_aircraft(archive_path: &Path, xplane_path: &str) -> Result<TargetDetection> {
    let files = list_archive_files(archive_path)?;
    rank_candidates(&files, &discover(xplane_path)?)
}

// --------------------------------------------------------------------------
// Level 2 — infer the archive → aircraft subpath mappings
// --------------------------------------------------------------------------

/// For a chosen aircraft, infer how the archive maps into it: a root overlay
/// mapping (the subtree whose paths align with the aircraft) plus any liveries
/// mapping, with leftovers reported as `unmapped` for the user to resolve.
pub fn infer_mappings(
    archive_path: &Path,
    xplane_path: &str,
    aircraft_folder: &str,
) -> Result<PatchPlan> {
    let aircraft_dir = resolve_aircraft_dir_by_name(xplane_path, aircraft_folder)
        .ok_or_else(|| anyhow!("Aircraft folder not found: {}", aircraft_folder))?;

    let files = list_archive_files(archive_path)?;
    let dirs = derive_dirs(&files);
    let archive_tree: Vec<String> = dirs.iter().chain(&files).cloned().collect();
    let aircraft_subdirs = immediate_subdirs(&aircraft_dir);

    // Use exactly the same library-wide evidence as target detection, so the
    // recommended wrapper cannot change when the user opens the preview.
    let detection = rank_candidates(&files, &discover(xplane_path)?)?;
    let candidate = detection
        .candidates
        .iter()
        .find(|candidate| candidate.folder_name == aircraft_folder.replace('\\', "/"))
        .ok_or_else(|| anyhow!("Aircraft folder not found: {}", aircraft_folder))?;
    let (suggested_mappings, unmapped) =
        if candidate.ambiguous_mapping || candidate.matched_count == 0 {
            (Vec::new(), matching::uncovered(&files, &[]))
        } else {
            plan_at_offset(
                &files,
                &dirs,
                &candidate.best_offset,
                &candidate.best_destination,
                candidate.matched_count,
            )
        };

    Ok(PatchPlan {
        archive_tree,
        suggested_mappings,
        unmapped,
        aircraft_subdirs,
    })
}

#[cfg(test)]
fn plan_mappings(
    files: &[String],
    dirs: &BTreeSet<String>,
    aircraft_dir: &Path,
) -> (Vec<PatchMapping>, Vec<String>) {
    let detection = rank_candidates(
        files,
        &[ProbeTarget {
            folder_name: "test".into(),
            display_name: "test".into(),
            dir: aircraft_dir.to_path_buf(),
        }],
    )
    .unwrap();
    let candidate = &detection.candidates[0];
    plan_at_offset(
        files,
        dirs,
        &candidate.best_offset,
        &candidate.best_destination,
        candidate.matched_count,
    )
}

fn plan_at_offset(
    files: &[String],
    dirs: &BTreeSet<String>,
    offset: &str,
    destination: &str,
    matched: usize,
) -> (Vec<PatchMapping>, Vec<String>) {
    let count = count_under(files, offset);
    let mut mappings = vec![PatchMapping {
        archive_subpath: offset.to_string(),
        dest_subpath: destination.to_string(),
        file_count: count,
        confidence: confidence_for(matched, count),
        reason: if matched > 0 {
            format!("matchedExisting:{}", matched)
        } else {
            "noAnchorDefaultRoot".to_string()
        },
    }];
    // Recognize separate liveries at any wrapper depth. Never map an ancestor
    // of the selected root (that would duplicate already mapped files).
    for dir in dirs {
        if dir
            .rsplit('/')
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("liveries"))
            && !mappings.iter().any(|mapping| {
                strip_offset(dir, &mapping.archive_subpath).is_some()
                    || dir == &mapping.archive_subpath
                    || strip_offset(&mapping.archive_subpath, dir).is_some()
            })
        {
            mappings.push(PatchMapping {
                archive_subpath: dir.clone(),
                dest_subpath: "liveries".to_string(),
                file_count: count_under(files, dir),
                confidence: Confidence::Medium,
                reason: "liveryFolder".to_string(),
            });
        }
    }
    let livery_roots: BTreeSet<String> = files
        .iter()
        .filter_map(|file| {
            let (_, root) = crate::livery_patterns::check_livery_pattern(file)?;
            (!root.is_empty() && dirs.contains(&root)).then_some(root)
        })
        .collect();
    for root in livery_roots {
        if mappings.iter().any(|mapping| {
            root == mapping.archive_subpath
                || strip_offset(&root, &mapping.archive_subpath).is_some()
                || strip_offset(&mapping.archive_subpath, &root).is_some()
        }) {
            continue;
        }
        let Some(name) = root.rsplit('/').next() else {
            continue;
        };
        mappings.push(PatchMapping {
            archive_subpath: root.clone(),
            dest_subpath: format!("liveries/{name}"),
            file_count: count_under(files, &root),
            confidence: Confidence::Medium,
            reason: "liveryFolder".to_string(),
        });
    }
    let prefixes = mappings
        .iter()
        .map(|mapping| mapping.archive_subpath.clone())
        .collect::<Vec<_>>();
    let unmapped = matching::uncovered(files, &prefixes);
    (mappings, unmapped)
}

/// Preview exactly the same validated write set used by the installer.
pub fn summarize_install(
    archive_path: &Path,
    xplane_path: &str,
    aircraft_folder: &str,
    mappings: &[PatchMappingInput],
) -> Result<PatchInstallSummary> {
    let aircraft_dir = resolve_aircraft_dir_by_name(xplane_path, aircraft_folder)
        .ok_or_else(|| anyhow!("Aircraft folder not found: {}", aircraft_folder))?;
    let aircraft_dir = fs::canonicalize(aircraft_dir)?;
    let writes = plan_install_files(&list_archive_files(archive_path)?, &aircraft_dir, mappings)?;
    install_summary(archive_path, &aircraft_dir, &writes)
}

fn install_summary(
    archive: &Path,
    aircraft_dir: &Path,
    writes: &[PatchFileWrite],
) -> Result<PatchInstallSummary> {
    let metadata = fs::metadata(archive)?;
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(&(
        archive,
        aircraft_dir,
        metadata.len(),
        metadata.modified()?,
        writes,
    ))?);
    Ok(PatchInstallSummary {
        total_files: writes.len(),
        overwrite_count: writes.iter().filter(|file| file.overwrite).count(),
        plan_id: format!("{:x}", hash.finalize()),
    })
}

fn plan_install_files(
    files: &[String],
    root: &Path,
    mappings: &[PatchMappingInput],
) -> Result<Vec<PatchFileWrite>> {
    if mappings.is_empty() {
        return Err(anyhow!("No patch mappings provided"));
    }
    let mut writes = Vec::new();
    for mapping in mappings {
        let archive_sub = safe_relative_subpath(&mapping.archive_subpath)?;
        let dest_sub = safe_relative_subpath(&mapping.dest_subpath)?;
        let start = writes.len();
        for file in files {
            let Some(relative) = mapping_relative_file(file, &archive_sub) else {
                continue;
            };
            let mut destination = if dest_sub.is_empty() {
                relative.to_string()
            } else {
                format!("{dest_sub}/{relative}")
            };
            let mut path = root.join(safe_relative_subpath(&destination)?);
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("acf"))
                && !path.exists()
            {
                let disabled = path.with_extension("xfma");
                if disabled.is_file() {
                    destination = disabled
                        .strip_prefix(root)?
                        .to_string_lossy()
                        .replace('\\', "/");
                    path = disabled;
                }
            }
            validate_destination(root, &path)?;
            writes.push(PatchFileWrite {
                archive_path: file.clone(),
                destination,
                overwrite: path.is_file(),
            });
        }
        if writes.len() == start {
            return Err(anyhow!("Patch mapping contains no files: {}", archive_sub));
        }
    }
    writes.sort_by(|a, b| a.destination.cmp(&b.destination));
    validate_write_set(root, &writes)?;
    Ok(writes)
}

fn validate_write_set(root: &Path, writes: &[PatchFileWrite]) -> Result<()> {
    if writes.is_empty() {
        return Err(anyhow!("No patch files selected"));
    }
    let mut destinations = BTreeSet::new();
    for write in writes {
        let relative = safe_relative_subpath(&write.destination)?;
        if relative.is_empty() || !destinations.insert(matching::path_key(&relative)) {
            return Err(anyhow!(
                "Patch mappings write the same or an invalid destination: {}",
                write.destination
            ));
        }
    }
    for write in writes {
        let dest = root.join(&write.destination);
        validate_destination(root, &dest)?;
        if dest.exists() && !dest.is_file() {
            return Err(anyhow!(
                "Patch file conflicts with an existing path: {}",
                dest.display()
            ));
        }
        for parent in Path::new(&write.destination)
            .ancestors()
            .skip(1)
            .filter(|path| !path.as_os_str().is_empty())
        {
            if destinations.contains(&matching::path_key(&parent.to_string_lossy()))
                || root.join(parent).is_file()
            {
                return Err(anyhow!(
                    "Patch file conflicts with a parent file: {}",
                    dest.display()
                ));
            }
        }
    }
    Ok(())
}

/// Interpret an archive prefix as either a directory or one exact file. File
/// mappings preserve the basename, exactly as a directory overlay would.
fn mapping_relative_file<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    if !prefix.is_empty() && path == prefix {
        path.rsplit('/').next()
    } else {
        strip_offset(path, prefix)
    }
}

/// Validate paths with Windows semantics too, even on Unix. This prevents a
/// mapping validated on one platform from acquiring a drive/root on another.
fn safe_relative_subpath(value: &str) -> Result<String> {
    let normalized = value.replace('\\', "/");
    if normalized.starts_with('/')
        || normalized.contains(':')
        || normalized
            .chars()
            .any(|c| c.is_control() || "<>\"|?*".contains(c))
        || normalized
            .split('/')
            .any(|s| s == ".." || s == "." || s.ends_with('.') || s.ends_with(' '))
    {
        return Err(anyhow!(
            "Patch path must stay inside the selected aircraft: {}",
            value
        ));
    }
    let normalized = normalized.trim_end_matches('/').to_string();
    if !normalized.is_empty()
        && normalized.split('/').any(|segment| {
            let stem = segment.split('.').next().unwrap_or("").to_ascii_uppercase();
            segment.is_empty()
                || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || (stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        })
    {
        return Err(anyhow!("Invalid patch path: {}", value));
    }
    if normalized
        .split('/')
        .any(|s| s.eq_ignore_ascii_case("_xfast_patch_backups"))
    {
        return Err(anyhow!("Patch mappings cannot modify patch backups"));
    }
    Ok(normalized)
}

/// Check every existing path component, including file symlinks and dangling
/// links. Canonicalizing only the final target would miss a missing leaf below
/// a directory link that points outside the selected aircraft.
pub(crate) fn validate_destination(aircraft_root: &Path, target: &Path) -> Result<()> {
    validate_destination_inner(aircraft_root, target, false)
}

fn validate_destination_inner(
    aircraft_root: &Path,
    target: &Path,
    allow_backup: bool,
) -> Result<()> {
    let root = fs::canonicalize(aircraft_root)?;
    let relative = target
        .strip_prefix(aircraft_root)
        .map_err(|_| anyhow!("Patch destination is outside the selected aircraft"))?;
    let rel_string = relative.to_string_lossy();
    if allow_backup {
        // Backup paths are generated internally; still reject traversal/rooted
        // components and use the same symlink containment checks below.
        if relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(anyhow!("Invalid patch backup path"));
        }
    } else {
        safe_relative_subpath(&rel_string)?;
    }
    let mut probe = root.clone();
    for component in relative.components() {
        probe.push(component.as_os_str());
        match fs::symlink_metadata(&probe) {
            Ok(metadata) => {
                #[cfg(windows)]
                let is_link = {
                    use std::os::windows::fs::MetadataExt;
                    metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
                };
                #[cfg(not(windows))]
                let is_link = metadata.file_type().is_symlink();
                if is_link {
                    return Err(anyhow!(
                        "Patch path contains a filesystem link: {}",
                        probe.display()
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Task building, overwrite-backup and revert
// --------------------------------------------------------------------------

/// A single user-confirmed mapping (archive subfolder -> aircraft subpath).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchMappingInput {
    /// Subfolder inside the archive ("" = archive root).
    #[serde(default)]
    pub archive_subpath: String,
    /// Destination subpath inside the aircraft folder ("" = aircraft root).
    #[serde(default)]
    pub dest_subpath: String,
}

/// Build one task for the whole patch. Every mapping is staged before any
/// aircraft file is written, so one bad mapping cannot leave a half-applied patch.
pub fn build_install_tasks(
    archive_path: &str,
    password: Option<String>,
    xplane_path: &str,
    aircraft_folder: &str,
    mappings: Vec<PatchMappingInput>,
    backup_overwritten: bool,
    expected_plan_id: Option<&str>,
) -> Result<Vec<InstallTask>> {
    let aircraft_dir = resolve_aircraft_dir_by_name(xplane_path, aircraft_folder)
        .ok_or_else(|| anyhow!("Aircraft folder not found: {}", aircraft_folder))?;
    let aircraft_dir = fs::canonicalize(aircraft_dir)?;
    let archive = Path::new(archive_path);
    let files = plan_install_files(&list_archive_files(archive)?, &aircraft_dir, &mappings)?;
    let plan_id = install_summary(archive, &aircraft_dir, &files)?.plan_id;
    if expected_plan_id.is_some_and(|expected| expected != plan_id) {
        return Err(anyhow!("The patch or aircraft changed since preview. Review the updated preview before installing."));
    }
    let name = archive
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Patch");
    let id = Uuid::new_v4().to_string();
    let backup_dir = backup_overwritten.then(|| {
        aircraft_dir
            .join("_xfast_patch_backups")
            .join(format!("{}_{}", sanitize_component(name), id))
            .to_string_lossy()
            .into_owned()
    });
    let aircraft_root = aircraft_dir.to_string_lossy().into_owned();
    Ok(vec![InstallTask {
        id,
        addon_type: AddonType::Patch,
        source_path: archive_path.to_string(),
        resolved_source_path: None,
        original_input_path: Some(archive_path.to_string()),
        target_path: aircraft_root.clone(),
        display_name: format!("{name} → {aircraft_folder}"),
        conflict_exists: Some(true),
        archive_internal_root: None,
        extraction_chain: None,
        should_overwrite: true,
        password,
        estimated_size: None,
        size_warning: None,
        size_confirmed: true,
        existing_navdata_info: None,
        new_navdata_info: None,
        existing_version_info: None,
        new_version_info: None,
        backup_liveries: false,
        backup_config_files: false,
        config_file_patterns: Vec::new(),
        backup_navdata: false,
        file_hashes: None,
        enable_verification: false,
        livery_aircraft_type: None,
        livery_aircraft_found: true,
        flywithlua_installed: true,
        companion_paths: Vec::new(),
        patch_files: files,
        patch_plan_id: Some(plan_id),
        patch_backup: backup_overwritten,
        patch_backup_dir: backup_dir,
        patch_aircraft_root: Some(aircraft_root),
    }])
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupEntry {
    /// Absolute path of the backed-up copy.
    backup: String,
    /// Absolute path the file was overwritten at (restore destination).
    original: String,
    #[serde(default)]
    checksum: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PatchBackupManifest {
    #[serde(default)]
    patch: String,
    #[serde(default)]
    entries: Vec<BackupEntry>,
}

// Install and restore must not interleave their preflight/snapshot/write steps.
static PATCH_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Validate every extracted source and destination before committing the overlay.
pub(crate) fn apply_staged_patch(
    staged: &Path,
    root: &Path,
    task: &InstallTask,
    control: &crate::task_control::TaskControl,
) -> Result<()> {
    let _guard = PATCH_WRITE_LOCK
        .lock()
        .map_err(|_| anyhow!("Patch write lock poisoned"))?;
    let check_cancelled = || {
        if control.is_cancelled() || control.is_skip_requested() {
            Err(anyhow!("Patch installation cancelled"))
        } else {
            Ok(())
        }
    };
    check_cancelled()?;
    let summary = install_summary(Path::new(&task.source_path), root, &task.patch_files)?;
    if task.patch_plan_id.as_deref() != Some(&summary.plan_id) {
        return Err(anyhow!("The patch archive changed since it was previewed"));
    }
    validate_write_set(root, &task.patch_files)?;
    let mut writes = Vec::with_capacity(task.patch_files.len());
    for file in &task.patch_files {
        let source = staged.join(safe_relative_subpath(&file.archive_path)?);
        validate_destination(staged, &source)?;
        if !source.is_file() {
            return Err(anyhow!(
                "Patch did not extract its selected file: {}",
                file.archive_path
            ));
        }
        let destination = root.join(&file.destination);
        if destination.is_file() != file.overwrite {
            return Err(anyhow!(
                "Aircraft files changed since the patch preview: {}",
                destination.display()
            ));
        }
        writes.push((source, destination));
    }
    let backup = if task.patch_backup {
        let dir = task
            .patch_backup_dir
            .as_deref()
            .ok_or_else(|| anyhow!("Patch backup directory is missing"))?;
        Some(Path::new(dir))
    } else {
        None
    };
    write_patch_files(root, &writes, backup, |source, dest| {
        check_cancelled()?;
        replace_file(source, dest)
    })?;
    Ok(())
}

/// Replace one file without unlinking the old file first. The temporary file
/// lives alongside the destination so the rename works across staging volumes.
fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("Invalid patch destination"))?;
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    fs::copy(source, temp.path())?;
    temp.as_file().sync_all()?;
    temp.persist(destination)?;
    Ok(())
}

fn write_patch_files(
    root: &Path,
    writes: &[(PathBuf, PathBuf)],
    backup: Option<&Path>,
    replace: impl Fn(&Path, &Path) -> Result<()>,
) -> Result<usize> {
    // A temporary snapshot is required for rollback even when the user opts
    // out of a retained backup. Do not write anything before every copy exists.
    let journal = tempfile::Builder::new()
        .prefix(".xfast-patch-rollback-")
        .tempdir_in(root)?;
    let mut originals = Vec::with_capacity(writes.len());
    for (_, destination) in writes {
        validate_destination(root, destination)?;
        let snapshot = if destination.is_file() {
            let path = journal.path().join(destination.strip_prefix(root)?);
            fs::create_dir_all(path.parent().unwrap())?;
            fs::copy(destination, &path)?;
            Some(path)
        } else if destination.exists() {
            return Err(anyhow!(
                "Patch destination is not a file: {}",
                destination.display()
            ));
        } else {
            None
        };
        originals.push(snapshot);
    }
    if let Some(backup) = backup {
        save_backup(root, backup, writes, &originals)?;
    }
    let mut created_dirs = Vec::new();
    for (index, (source, destination)) in writes.iter().enumerate() {
        let result = (|| {
            validate_destination(root, destination)?;
            let parent = destination
                .parent()
                .ok_or_else(|| anyhow!("Invalid patch destination"))?;
            let missing: Vec<_> = parent.ancestors().take_while(|p| !p.exists()).collect();
            for dir in missing.into_iter().rev() {
                fs::create_dir(dir)?;
                created_dirs.push(dir.to_path_buf());
            }
            replace(source, destination)
        })();
        if let Err(error) = result {
            let mut rollback_errors = Vec::new();
            for ((_, dest), original) in writes[..index].iter().zip(&originals[..index]).rev() {
                let restored = validate_destination(root, dest).and_then(|()| match original {
                    Some(original) => replace_file(original, dest),
                    None => fs::remove_file(dest).map_err(Into::into),
                });
                if let Err(error) = restored {
                    rollback_errors.push(format!("{}: {}", dest.display(), error));
                }
            }
            for dir in created_dirs.iter().rev() {
                let _ = fs::remove_dir(dir);
            }
            if !rollback_errors.is_empty() {
                let recovery = journal.keep();
                return Err(anyhow!(
                    "Patch write failed at {}: {error}; rollback failed: {}. Recovery files: {}",
                    destination.display(),
                    rollback_errors.join("; "),
                    recovery.display()
                ));
            }
            return Err(error.context(format!(
                "Patch write failed at {}; changes were rolled back",
                destination.display()
            )));
        }
    }
    Ok(writes.len())
}

fn file_checksum(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    std::io::copy(&mut fs::File::open(path)?, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn save_backup(
    root: &Path,
    backup: &Path,
    writes: &[(PathBuf, PathBuf)],
    originals: &[Option<PathBuf>],
) -> Result<()> {
    if !originals.iter().any(Option::is_some) {
        return Ok(());
    }
    if backup.parent() != Some(root.join("_xfast_patch_backups").as_path()) {
        return Err(anyhow!("Invalid patch backup directory"));
    }
    validate_destination_inner(root, backup, true)?;
    let backup_root = backup.parent().unwrap();
    fs::create_dir_all(backup_root)?;
    // Publish only complete sessions, including all snapshots and their manifest.
    let staging = tempfile::tempdir_in(backup_root)?;
    let mut manifest = PatchBackupManifest::default();
    for ((_, destination), original) in writes.iter().zip(originals) {
        let Some(original) = original else {
            continue;
        };
        let relative = destination.strip_prefix(root)?;
        let staged = staging.path().join("files").join(relative);
        fs::create_dir_all(staged.parent().unwrap())?;
        fs::copy(original, &staged)?;
        manifest.entries.push(BackupEntry {
            backup: backup
                .join("files")
                .join(relative)
                .to_string_lossy()
                .into_owned(),
            original: destination.to_string_lossy().into_owned(),
            checksum: Some(file_checksum(&staged)?),
        });
    }
    fs::write(
        staging.path().join("manifest_patch.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::rename(staging.path(), backup).context("Could not publish the patch backup")?;
    Ok(())
}

/// Restore all overwritten files from one backup session. Validate every record
/// before making any changes and roll back the restore if a write fails.
pub fn restore_aircraft_backup(
    xplane_path: &str,
    aircraft_folder: &str,
    session: &Path,
) -> Result<usize> {
    let aircraft = resolve_aircraft_dir_by_name(xplane_path, aircraft_folder)
        .ok_or_else(|| anyhow!("Aircraft folder not found: {}", aircraft_folder))?;
    let aircraft = fs::canonicalize(aircraft)?;
    let session_name = session
        .file_name()
        .ok_or_else(|| anyhow!("Invalid patch backup session"))?;
    let expected = aircraft.join("_xfast_patch_backups").join(session_name);
    validate_destination_inner(&aircraft, &expected, true)?;
    let session = fs::canonicalize(session).context("Patch backup session not found")?;
    if session != expected {
        return Err(anyhow!(
            "Select a backup session belonging to this aircraft"
        ));
    }
    revert_patch(&session)
}

fn revert_patch(dir: &Path) -> Result<usize> {
    let _guard = PATCH_WRITE_LOCK
        .lock()
        .map_err(|_| anyhow!("Patch write lock poisoned"))?;
    let backup_root = dir
        .parent()
        .filter(|p| p.file_name() == Some(std::ffi::OsStr::new("_xfast_patch_backups")))
        .ok_or_else(|| anyhow!("Invalid patch backup session"))?;
    let aircraft_root = backup_root
        .parent()
        .ok_or_else(|| anyhow!("Invalid patch backup session"))?;
    validate_destination_inner(aircraft_root, dir, true)?;
    let mut writes = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("manifest_") && name.ends_with(".json")) {
            continue;
        }
        validate_destination_inner(aircraft_root, &entry.path(), true)?;
        let manifest: PatchBackupManifest = serde_json::from_slice(&fs::read(entry.path())?)
            .with_context(|| format!("Invalid patch backup manifest {name}"))?;
        for file in manifest.entries {
            let source = PathBuf::from(file.backup);
            let destination = PathBuf::from(file.original);
            validate_destination(aircraft_root, &destination)?;
            let relative = destination.strip_prefix(aircraft_root)?;
            if source != dir.join("files").join(relative) {
                return Err(anyhow!(
                    "Patch backup entry does not match its original location"
                ));
            }
            validate_destination_inner(aircraft_root, &source, true)?;
            if !source.is_file() {
                return Err(anyhow!(
                    "Patch backup file is missing: {}",
                    source.display()
                ));
            }
            if let Some(checksum) = file.checksum {
                if file_checksum(&source)? != checksum {
                    return Err(anyhow!(
                        "Patch backup file failed integrity checking: {}",
                        source.display()
                    ));
                }
            }
            if seen.insert(matching::path_key(&destination.to_string_lossy())) {
                writes.push((source, destination));
            }
        }
    }
    if writes.is_empty() {
        return Err(anyhow!("No files to restore in this patch backup"));
    }
    write_patch_files(aircraft_root, &writes, None, replace_file)
}

/// Sanitize a string for use as a single path component.
fn sanitize_component(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "patch".to_string()
    } else {
        trimmed.chars().take(60).collect()
    }
}

// --------------------------------------------------------------------------
// Archive listing / structure helpers
// --------------------------------------------------------------------------

/// List the archive's file entries, normalized to forward slashes (no dirs).
pub(crate) fn list_archive_files(archive_path: &Path) -> Result<Vec<String>> {
    let scanner = Scanner::new();
    let entries = scanner.list_archive_entries(archive_path)?;
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in entries {
        let entry = entry.replace('\\', "/");
        let entry = entry.strip_prefix("./").unwrap_or(&entry);
        if entry.is_empty()
            || entry.ends_with('/')
            || crate::package_artifacts::is_ignored_package_artifact_archive_path(entry)
        {
            continue;
        }
        let normalized = safe_relative_subpath(entry)?;
        if normalized != entry || normalized.split('/').any(str::is_empty) {
            return Err(anyhow!("Ambiguous archive path: {}", entry));
        }
        if !seen.insert(matching::path_key(&normalized)) {
            return Err(anyhow!("Duplicate archive file: {}", entry));
        }
        files.push(normalized);
    }
    if files.is_empty() {
        return Err(anyhow!("Patch archive contains no files"));
    }
    files.sort();
    Ok(files)
}

/// Every directory path implied by the file list.
fn derive_dirs(files: &[String]) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();
    for f in files {
        let mut acc = String::new();
        let segments: Vec<&str> = f.split('/').collect();
        // all but the last segment (the file name) are directories
        for seg in &segments[..segments.len().saturating_sub(1)] {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(seg);
            dirs.insert(acc.clone());
        }
    }
    dirs
}

fn strip_offset<'a>(path: &'a str, offset: &str) -> Option<&'a str> {
    if offset.is_empty() {
        Some(path)
    } else {
        path.strip_prefix(&format!("{}/", offset))
    }
}

fn count_under(files: &[String], prefix: &str) -> usize {
    files
        .iter()
        .filter(|f| strip_offset(f, prefix).is_some())
        .count()
}

fn confidence_for(matched: usize, sample: usize) -> Confidence {
    if sample == 0 {
        return Confidence::Low;
    }
    let ratio = matched as f64 / sample as f64;
    if matched >= 5 && ratio >= 0.6 {
        Confidence::High
    } else if matched >= MIN_RECOMMEND_MATCH && ratio >= MIN_RECOMMEND_RATIO {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

// --------------------------------------------------------------------------
// Aircraft path resolution
// --------------------------------------------------------------------------

/// Immediate sub-directory names of `dir` (for the destination dropdown).
fn immediate_subdirs(dir: &Path) -> Vec<String> {
    let mut subs = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Ok(name) = entry.file_name().into_string() {
                    if !name.starts_with('.') {
                        subs.push(name);
                    }
                }
            }
        }
    }
    subs.sort_by_key(|s| s.to_lowercase());
    subs
}

/// Targets are exact library-relative identifiers, never a first basename match.
fn resolve_aircraft_dir_by_name(xplane_path: &str, folder_name: &str) -> Option<PathBuf> {
    let relative = safe_relative_subpath(folder_name).ok()?;
    if relative.is_empty() {
        return None;
    }
    let direct = Path::new(xplane_path).join("Aircraft").join(relative);
    if !direct.is_dir() {
        return None;
    }
    let has_aircraft = fs::read_dir(&direct).ok()?.any(|entry| {
        entry.ok().is_some_and(|entry| {
            entry.path().extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("acf") || ext.eq_ignore_ascii_case("xfma")
            }) && entry.path().is_file()
        })
    });
    has_aircraft.then_some(direct)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn dirs_of(files: &[&str]) -> (Vec<String>, BTreeSet<String>) {
        let files: Vec<String> = files.iter().map(|s| s.to_string()).collect();
        let dirs = derive_dirs(&files);
        (files, dirs)
    }

    #[test]
    fn derive_dirs_builds_full_chain() {
        let (_f, dirs) = dirs_of(&["A/B/c.txt", "A/d.txt"]);
        assert!(dirs.contains("A"));
        assert!(dirs.contains("A/B"));
        assert_eq!(dirs.len(), 2);
    }

    /// Build a minimal on-disk aircraft folder with the given relative files.
    fn make_aircraft(rels: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for rel in rels {
            let p = dir.path().join(rel);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&p, b"x").unwrap();
        }
        dir
    }

    fn to_owned(files: &[&str]) -> Vec<String> {
        files.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn plan_pw_mod_yields_base_and_liveries() {
        // Installed aircraft already has these files.
        let ac = make_aircraft(&["A321.acf", "objects/fuselage.obj", "systems/hydraulics.txt"]);
        // PW-MOD archive: base content under a two-level wrapper + separate liveries.
        let files = to_owned(&[
            "Toliss321 Base Folder/Base Folder/A321.acf",
            "Toliss321 Base Folder/Base Folder/objects/fuselage.obj",
            "Toliss321 Base Folder/Base Folder/systems/hydraulics.txt",
            "Liveries/PW Repaint/objects/livery.obj",
        ]);
        let dirs = derive_dirs(&files);

        let (mappings, unmapped) = plan_mappings(&files, &dirs, ac.path());

        // Root overlay maps the aligned base folder to the aircraft root.
        let root = mappings.iter().find(|m| m.dest_subpath.is_empty()).unwrap();
        assert_eq!(root.archive_subpath, "Toliss321 Base Folder/Base Folder");
        // Liveries map into <aircraft>/liveries.
        let liveries = mappings
            .iter()
            .find(|m| m.dest_subpath == "liveries")
            .unwrap();
        assert_eq!(liveries.archive_subpath, "Liveries");
        // The wrapper folder is fully covered by the root overlay, not "unmapped".
        assert!(unmapped.is_empty(), "unexpected unmapped: {:?}", unmapped);
        assert_eq!(mappings.len(), 2);
    }

    #[test]
    fn plan_fmod_yields_single_root_overlay() {
        let ac = make_aircraft(&["sounds/engine.wav", "fmod/a319.bank", "A319.acf"]);
        // FMOD sound mod: content at archive root, no wrapper.
        let files = to_owned(&["sounds/engine.wav", "fmod/a319.bank"]);
        let dirs = derive_dirs(&files);

        let (mappings, unmapped) = plan_mappings(&files, &dirs, ac.path());

        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].archive_subpath, "");
        assert_eq!(mappings[0].dest_subpath, "");
        assert!(unmapped.is_empty());
    }

    /// Regression: an FMOD sound patch must align to the aircraft that owns its
    /// *model-specific* sound files, not to one that merely shares the universal
    /// `plugins/xlua` runtime and generic FMOD boilerplate. (Real case: the
    /// SBStudio A321 pack was matched to a LevelUp 737 because the 737 had 7
    /// xlua hits vs the A321's 5 real fmod hits.)
    #[test]
    fn rank_prefers_model_match_over_shared_runtime() {
        // Correct target: ToLiss A321 — owns the model-specific sound files.
        let a321 = make_aircraft(&[
            "fmod/a321.snd",
            "fmod/a321_StdDef.snd",
            "fmod/GUIDs.txt",
            "fmod/Master Bank.bank",
            "fmod/Master Bank.strings.bank",
        ]);
        // Wrong target: LevelUp 737 — only the universal xlua runtime + generic
        // FMOD names overlap (including a legacy xlua/64 copy unique to it).
        let b737 = make_aircraft(&[
            "fmod/GUIDs.txt",
            "fmod/Master Bank.bank",
            "plugins/xlua/64/win.xpl",
            "plugins/xlua/64/lin.xpl",
            "plugins/xlua/64/mac.xpl",
            "plugins/xlua/init.lua",
            "plugins/xlua/win_x64/xlua.xpl",
        ]);
        // A couple more aircraft so the universal FMOD names have realistic df.
        let g1 = make_aircraft(&[
            "fmod/GUIDs.txt",
            "fmod/Master Bank.bank",
            "plugins/xlua/init.lua",
        ]);
        let g2 = make_aircraft(&["fmod/GUIDs.txt", "fmod/Master Bank.bank"]);

        // The SBStudio A321 FMOD patch, wrapped in a versioned top folder.
        let files = to_owned(&[
            "A321_FMOD_SBStudio_v1.4.1/fmod/a321.snd",
            "A321_FMOD_SBStudio_v1.4.1/fmod/a321_StdDef.snd",
            "A321_FMOD_SBStudio_v1.4.1/fmod/CFM.bank",
            "A321_FMOD_SBStudio_v1.4.1/fmod/GUIDs.txt",
            "A321_FMOD_SBStudio_v1.4.1/fmod/Master Bank.bank",
            "A321_FMOD_SBStudio_v1.4.1/fmod/Master Bank.strings.bank",
            "A321_FMOD_SBStudio_v1.4.1/plugins/xlua/64/win.xpl",
            "A321_FMOD_SBStudio_v1.4.1/plugins/xlua/64/lin.xpl",
            "A321_FMOD_SBStudio_v1.4.1/plugins/xlua/64/mac.xpl",
            "A321_FMOD_SBStudio_v1.4.1/plugins/xlua/init.lua",
            "A321_FMOD_SBStudio_v1.4.1/plugins/xlua/win_x64/xlua.xpl",
            "A321_FMOD_SBStudio_v1.4.1/README!!!.txt",
        ]);

        let mk = |folder: &str, dir: &Path| ProbeTarget {
            folder_name: folder.to_string(),
            display_name: folder.to_string(),
            dir: dir.to_path_buf(),
        };
        let targets = vec![
            // 737 listed first so an unstable sort can't accidentally favour the A321.
            mk("737NG_Series_V2", b737.path()),
            mk("ToLissA321_V1p8", a321.path()),
            mk("GenericA", g1.path()),
            mk("GenericB", g2.path()),
        ];

        let det = rank_candidates(&files, &targets).unwrap();

        // The A321 — fewer raw matches (5) but the only *discriminative* ones —
        // must win and be recommended; the 737 (9 raw matches, all noise) must not.
        assert_eq!(
            det.candidates.first().map(|c| c.folder_name.as_str()),
            Some("ToLissA321_V1p8"),
            "expected the A321 to rank first, got: {:?}",
            det.candidates
                .iter()
                .map(|c| &c.folder_name)
                .collect::<Vec<_>>()
        );
        assert_eq!(det.recommended_folder.as_deref(), Some("ToLissA321_V1p8"));
    }

    #[test]
    fn rank_no_recommendation_when_only_universal_files_match() {
        // Two aircraft, both with the same universal FMOD boilerplate only.
        let a = make_aircraft(&["fmod/GUIDs.txt", "fmod/Master Bank.bank"]);
        let b = make_aircraft(&["fmod/GUIDs.txt", "fmod/Master Bank.bank"]);
        // Patch whose only on-disk overlap is those universal names.
        let files = to_owned(&["pack/fmod/GUIDs.txt", "pack/fmod/Master Bank.bank"]);
        let mk = |folder: &str, dir: &Path| ProbeTarget {
            folder_name: folder.to_string(),
            display_name: folder.to_string(),
            dir: dir.to_path_buf(),
        };
        let det = rank_candidates(&files, &[mk("A", a.path()), mk("B", b.path())]).unwrap();
        // Universal files (df == n) score zero, so we must decline to guess.
        assert_eq!(det.recommended_folder, None);
    }

    fn target(name: &str, dir: &Path) -> ProbeTarget {
        ProbeTarget {
            folder_name: name.into(),
            display_name: name.into(),
            dir: dir.to_path_buf(),
        }
    }

    #[test]
    fn full_tree_uses_evidence_beyond_first_200_files() {
        let names: Vec<String> = (0..350)
            .map(|index| format!("objects/{index:03}.obj"))
            .collect();
        let mut patch = names.clone();
        patch.push("systems/z-identity.lua".into());
        let generic = make_aircraft(&names.iter().map(String::as_str).collect::<Vec<_>>());
        let correct = make_aircraft(&patch.iter().map(String::as_str).collect::<Vec<_>>());
        let detected = rank_candidates(
            &patch,
            &[target("A", generic.path()), target("B", correct.path())],
        )
        .unwrap();
        assert_eq!(detected.recommended_folder.as_deref(), Some("B"));
        assert_eq!(detected.candidates[0].matched_count, 351);
        assert_eq!(detected.candidates[0].file_count, 351);
    }

    #[test]
    fn deep_non_landmark_wrappers_and_plugin_scripts_match() {
        let a = make_aircraft(&["plugins/custom/data/model321.lua"]);
        let b = make_aircraft(&["plugins/custom/data/model737.lua"]);
        let files = to_owned(&["one/two/three/four/five/plugins/custom/data/model321.lua"]);
        let detected =
            rank_candidates(&files, &[target("A", a.path()), target("B", b.path())]).unwrap();
        assert_eq!(detected.recommended_folder.as_deref(), Some("A"));
        assert_eq!(
            detected.candidates[0].best_offset,
            "one/two/three/four/five"
        );
    }

    #[test]
    fn single_aircraft_is_recommended_but_zero_overlap_and_ties_are_not() {
        let a = make_aircraft(&["custom/identity.dat"]);
        let files = to_owned(&["wrapper/custom/identity.dat"]);
        assert_eq!(
            rank_candidates(&files, &[target("A", a.path())])
                .unwrap()
                .recommended_folder
                .as_deref(),
            Some("A")
        );
        assert!(
            rank_candidates(&files, &[target("A", a.path()), target("B", a.path())])
                .unwrap()
                .recommended_folder
                .is_none()
        );
        assert!(
            rank_candidates(&to_owned(&["unrelated.dat"]), &[target("A", a.path())])
                .unwrap()
                .recommended_folder
                .is_none()
        );
    }

    #[test]
    fn directories_are_not_file_matches_and_backups_supply_no_evidence() {
        let a = make_aircraft(&["_xfast_patch_backups/session/files/only.dat"]);
        fs::create_dir(a.path().join("fake.acf")).unwrap();
        let files = to_owned(&["fake.acf", "only.dat"]);
        let detected = rank_candidates(&files, &[target("A", a.path())]).unwrap();
        assert_eq!(detected.candidates[0].matched_count, 0);
    }

    #[test]
    fn a_flat_sound_pack_maps_into_the_existing_sound_directory() {
        let correct = make_aircraft(&["fmod/A321.snd", "fmod/A321_HD.snd"]);
        let other = make_aircraft(&["other/A321.snd"]);
        let files = to_owned(&["A321.snd", "A321_HD.snd", "new.bank"]);
        let detected = rank_candidates(
            &files,
            &[target("B", other.path()), target("A", correct.path())],
        )
        .unwrap();
        assert_eq!(detected.recommended_folder.as_deref(), Some("A"));
        assert_eq!(detected.candidates[0].best_destination, "fmod");
        assert_eq!(detected.candidates[0].matched_count, 2);
        let (mappings, _) = plan_mappings(&files, &derive_dirs(&files), correct.path());
        assert_eq!(mappings[0].dest_subpath, "fmod");
        assert_eq!(mappings[0].file_count, 3);
    }

    #[test]
    fn discovery_preserves_nested_identity_and_has_no_depth_limit() {
        let sim = tempfile::tempdir().unwrap();
        for relative in ["VendorA/Demo", "one/two/three/four/VendorB/Demo"] {
            let dir = sim.path().join("Aircraft").join(relative);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("disabled.xfma"), b"x").unwrap();
        }
        let targets = discover(sim.path().to_str().unwrap()).unwrap();
        assert_eq!(targets.len(), 2);
        assert_ne!(targets[0].folder_name, targets[1].folder_name);
        assert!(resolve_aircraft_dir_by_name(sim.path().to_str().unwrap(), "Demo").is_none());
        assert!(resolve_aircraft_dir_by_name(sim.path().to_str().unwrap(), "VendorA").is_none());
    }

    #[test]
    fn nested_liveries_and_uncovered_siblings_remain_visible() {
        let files = to_owned(&[
            "wrapper/base/plane.acf",
            "wrapper/Liveries/Paint/objects/body.obj",
            "wrapper/manual.pdf",
            "readme.txt",
        ]);
        let (mappings, unmapped) =
            plan_at_offset(&files, &derive_dirs(&files), "wrapper/base", "", 1);
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[1].archive_subpath, "wrapper/Liveries");
        assert_eq!(unmapped, to_owned(&["readme.txt", "wrapper/manual.pdf"]));
    }

    #[test]
    fn summarize_counts_total_and_overwrites() {
        // Aircraft already has two of the patch's files.
        let ac = make_aircraft(&["fmod/a321.snd", "fmod/Master Bank.bank"]);
        let files = to_owned(&[
            "pack/fmod/a321.snd",         // exists -> overwrite
            "pack/fmod/Master Bank.bank", // exists -> overwrite
            "pack/fmod/CFM.bank",         // new
            "pack/README.txt",            // new
        ]);
        let mappings = vec![PatchMappingInput {
            archive_subpath: "pack".to_string(),
            dest_subpath: String::new(),
        }];
        let s = plan_install_files(&files, ac.path(), &mappings).unwrap();
        assert_eq!(s.len(), 4);
        assert_eq!(s.iter().filter(|file| file.overwrite).count(), 2);
    }

    #[test]
    fn destinations_reject_traversal_and_windows_roots() {
        let ac = make_aircraft(&["plane.acf"]);
        for invalid in [
            "../OtherPlane",
            "objects/../../OtherPlane",
            "/tmp/plane",
            "C:\\Aircraft",
            "C:plane",
            "\\\\server\\share",
            "objects/../plane",
            ".. /OtherPlane",
            "_XFAST_PATCH_BACKUPS/session",
            "_xfast_patch_backups/session",
        ] {
            let mappings = vec![PatchMappingInput {
                archive_subpath: String::new(),
                dest_subpath: invalid.to_string(),
            }];
            assert!(
                plan_install_files(&to_owned(&["plane.acf"]), ac.path(), &mappings).is_err(),
                "accepted {}",
                invalid
            );
        }
    }

    #[test]
    fn exact_file_mapping_counts_only_selected_file_and_rejects_empty_mapping() {
        let ac = make_aircraft(&["plane.acf"]);
        let files = to_owned(&["plane.acf", "objects/body.obj"]);
        let mapping = PatchMappingInput {
            archive_subpath: "plane.acf".to_string(),
            dest_subpath: String::new(),
        };
        let summary = plan_install_files(&files, ac.path(), &[mapping]).unwrap();
        assert_eq!(summary.len(), 1);
        assert!(summary[0].overwrite);
        let missing = PatchMappingInput {
            archive_subpath: "missing".to_string(),
            dest_subpath: String::new(),
        };
        assert!(plan_install_files(&files, ac.path(), &[missing]).is_err());
    }

    #[test]
    fn late_write_failure_restores_overwrites_and_removes_added_files() {
        let aircraft = make_aircraft(&["original.acf"]);
        let staged = make_aircraft(&["new.acf", "extra.obj", "locked.obj"]);
        let root = aircraft.path();
        let writes = [
            ("new.acf", "original.acf"),
            ("extra.obj", "new/extra.obj"),
            ("locked.obj", "locked.obj"),
        ]
        .map(|(source, destination)| (staged.path().join(source), root.join(destination)));
        fs::write(&writes[0].0, b"changed").unwrap();
        let error = write_patch_files(root, &writes, None, |source, dest| {
            if dest.ends_with("locked.obj") {
                return Err(anyhow!("simulated sharing violation"));
            }
            replace_file(source, dest)
        })
        .unwrap_err();
        assert!(error.to_string().contains("changes were rolled back"));
        assert_eq!(fs::read(root.join("original.acf")).unwrap(), b"x");
        assert!(!root.join("new").exists());
        assert!(!root.join("locked.obj").exists());
    }

    #[test]
    fn backup_restore_preflights_the_entire_session_and_checks_integrity() {
        let ac = make_aircraft(&["one.acf", "two.acf"]);
        let stage = make_aircraft(&["one.acf", "two.acf", "new.obj"]);
        let root = fs::canonicalize(ac.path()).unwrap();
        let writes = ["one.acf", "two.acf", "new.obj"].map(|name| {
            let source = stage.path().join(name);
            fs::write(&source, b"patched").unwrap();
            (source, root.join(name))
        });
        let backup = root.join("_xfast_patch_backups/session");
        assert_eq!(
            write_patch_files(&root, &writes, Some(&backup), replace_file).unwrap(),
            3
        );
        assert_eq!(fs::read(root.join("one.acf")).unwrap(), b"patched");
        fs::write(backup.join("files/two.acf"), b"tampered").unwrap();
        assert!(revert_patch(&backup).is_err());
        assert_eq!(fs::read(root.join("one.acf")).unwrap(), b"patched");
        fs::write(backup.join("files/two.acf"), b"x").unwrap();
        assert_eq!(revert_patch(&backup).unwrap(), 2);
        assert_eq!(fs::read(root.join("one.acf")).unwrap(), b"x");
        assert_eq!(fs::read(root.join("two.acf")).unwrap(), b"x");
        assert_eq!(fs::read(root.join("new.obj")).unwrap(), b"patched");
    }

    #[cfg(unix)]
    #[test]
    fn destinations_reject_directory_and_file_symlink_escapes() {
        use std::os::unix::fs::symlink;
        let ac = make_aircraft(&["plane.acf"]);
        let outside = make_aircraft(&["plane.acf"]);
        symlink(outside.path(), ac.path().join("linked")).unwrap();
        symlink(
            outside.path().join("plane.acf"),
            ac.path().join("external.acf"),
        )
        .unwrap();
        assert!(validate_destination(ac.path(), &ac.path().join("linked/new.obj")).is_err());
        assert!(validate_destination(ac.path(), &ac.path().join("external.acf")).is_err());
        symlink(ac.path().join("missing"), ac.path().join("dangling")).unwrap();
        assert!(validate_destination(ac.path(), &ac.path().join("dangling/new.obj")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn backup_symlink_escape_is_rejected_before_writing() {
        use std::os::unix::fs::symlink;
        let ac = make_aircraft(&["plane.acf"]);
        let staged = make_aircraft(&["plane.acf"]);
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), ac.path().join("_xfast_patch_backups")).unwrap();
        let backup = ac.path().join("_xfast_patch_backups/session");
        assert!(write_patch_files(
            ac.path(),
            &[(staged.path().join("plane.acf"), ac.path().join("plane.acf"))],
            Some(&backup),
            replace_file
        )
        .is_err());
        assert!(!outside.path().join("session").exists());
    }

    #[test]
    fn task_builder_validates_real_archive_and_retains_aircraft_boundary() {
        use std::io::Write;
        let xplane = tempfile::tempdir().unwrap();
        let aircraft = xplane.path().join("Aircraft/Demo");
        fs::create_dir_all(&aircraft).unwrap();
        fs::write(aircraft.join("plane.acf"), b"original").unwrap();
        let archive = xplane.path().join("patch.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        zip.start_file("plane.acf", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"patch").unwrap();
        zip.finish().unwrap();
        let mapping = |dest: &str, source: &str| {
            vec![PatchMappingInput {
                archive_subpath: source.to_string(),
                dest_subpath: dest.to_string(),
            }]
        };
        let build = |dest: &str, source: &str| {
            build_install_tasks(
                archive.to_str().unwrap(),
                None,
                xplane.path().to_str().unwrap(),
                "Demo",
                mapping(dest, source),
                false,
                None,
            )
        };
        let tasks = build("", "plane.acf").unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].patch_files[0].archive_path, "plane.acf");
        let real_aircraft = fs::canonicalize(&aircraft).unwrap();
        assert_eq!(
            Path::new(tasks[0].patch_aircraft_root.as_ref().unwrap()),
            real_aircraft
        );
        assert!(build("../OtherPlane", "plane.acf").is_err());
        assert!(build("", "missing").is_err());
        assert_eq!(fs::read(aircraft.join("plane.acf")).unwrap(), b"original");
    }

    #[test]
    fn conflicting_mappings_are_rejected_before_writes() {
        let aircraft = make_aircraft(&["plane.acf", "occupied/file.obj"]);
        let files = to_owned(&["base/body.obj", "option/body.obj", "base/occupied"]);
        let mapping = |archive: &str, destination: &str| PatchMappingInput {
            archive_subpath: archive.into(),
            dest_subpath: destination.into(),
        };
        assert!(plan_install_files(
            &files,
            aircraft.path(),
            &[mapping("base/body.obj", ""), mapping("option", "")]
        )
        .is_err());
        assert!(plan_install_files(&files, aircraft.path(), &[mapping("base", "")]).is_err());
        assert!(plan_install_files(
            &files,
            aircraft.path(),
            &[mapping("base/body.obj", "plane.acf")]
        )
        .is_err());
    }

    #[test]
    fn acf_patch_matches_a_disabled_variant_and_preserves_its_state() {
        let aircraft = make_aircraft(&["plane.xfma"]);
        let files = to_owned(&["pack/plane.acf"]);
        let detection = rank_candidates(&files, &[target("A", aircraft.path())]).unwrap();
        assert_eq!(detection.recommended_folder.as_deref(), Some("A"));
        let writes = plan_install_files(
            &files,
            aircraft.path(),
            &[PatchMappingInput {
                archive_subpath: "pack".into(),
                dest_subpath: String::new(),
            }],
        )
        .unwrap();
        assert_eq!(writes[0].destination, "plane.xfma");
        assert!(writes[0].overwrite);
    }

    #[test]
    fn zip_preview_and_one_task_apply_multiple_mappings_and_restore() {
        use std::io::Write;
        let sim = tempfile::tempdir().unwrap();
        let aircraft = sim.path().join("Aircraft/Vendor/Demo");
        fs::create_dir_all(&aircraft).unwrap();
        fs::write(aircraft.join("plane.acf"), b"original").unwrap();
        fs::write(aircraft.join("unrelated.obj"), b"untouched").unwrap();
        let archive = sim.path().join("patch.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        for file in [
            "bundle/base/plane.acf",
            "bundle/Liveries/New/objects/body.obj",
            "bundle/README.txt",
            "__MACOSX/._plane.acf",
        ] {
            zip.start_file(file, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"payload").unwrap();
        }
        zip.finish().unwrap();
        let sim = sim.path().to_str().unwrap();
        let folder = "Vendor/Demo";
        let detected = detect_target_aircraft(&archive, sim).unwrap();
        assert_eq!(detected.recommended_folder.as_deref(), Some(folder));
        let inferred = infer_mappings(&archive, sim, folder).unwrap();
        assert_eq!(inferred.unmapped, to_owned(&["bundle/README.txt"]));
        let mappings: Vec<_> = inferred
            .suggested_mappings
            .into_iter()
            .map(|mapping| PatchMappingInput {
                archive_subpath: mapping.archive_subpath,
                dest_subpath: mapping.dest_subpath,
            })
            .collect();
        let preview = summarize_install(&archive, sim, folder, &mappings).unwrap();
        assert_eq!((preview.total_files, preview.overwrite_count), (2, 1));
        let prepare = || {
            build_install_tasks(
                archive.to_str().unwrap(),
                None,
                sim,
                folder,
                mappings.clone(),
                true,
                Some(&preview.plan_id),
            )
        };
        let livery = aircraft.join("liveries/New/objects/body.obj");
        fs::create_dir_all(livery.parent().unwrap()).unwrap();
        fs::write(&livery, b"external change").unwrap();
        assert!(prepare().is_err());
        fs::remove_file(&livery).unwrap();
        let tasks = prepare().unwrap();
        assert_eq!(tasks.len(), 1);
        let task = &tasks[0];
        let staged = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(&aircraft).unwrap();
        let control = crate::task_control::TaskControl::new();
        assert!(apply_staged_patch(staged.path(), &root, task, &control).is_err());
        assert_eq!(fs::read(root.join("plane.acf")).unwrap(), b"original");
        zip::ZipArchive::new(fs::File::open(&archive).unwrap())
            .unwrap()
            .extract(staged.path())
            .unwrap();
        control.request_skip_current();
        assert!(apply_staged_patch(staged.path(), &root, task, &control).is_err());
        assert_eq!(fs::read(root.join("plane.acf")).unwrap(), b"original");
        control.reset_skip();
        apply_staged_patch(staged.path(), &root, task, &control).unwrap();
        assert_eq!(fs::read(&livery).unwrap(), b"payload");
        assert_eq!(fs::read(root.join("plane.acf")).unwrap(), b"payload");
        assert_eq!(fs::read(root.join("unrelated.obj")).unwrap(), b"untouched");
        assert!(!root.join("README.txt").exists());
        let backup = Path::new(task.patch_backup_dir.as_ref().unwrap());
        assert_eq!(restore_aircraft_backup(sim, folder, backup).unwrap(), 1);
        assert_eq!(fs::read(root.join("plane.acf")).unwrap(), b"original");
        assert_eq!(fs::read(&livery).unwrap(), b"payload");
    }

    #[test]
    fn restores_a_legacy_backup_without_a_checksum() {
        let aircraft =
            make_aircraft(&["plane.acf", "_xfast_patch_backups/session/files/plane.acf"]);
        let root = fs::canonicalize(aircraft.path()).unwrap();
        let session = root.join("_xfast_patch_backups/session");
        fs::write(root.join("plane.acf"), b"patched").unwrap();
        let manifest = serde_json::json!({
            "patch": "old",
            "entries": [{ "backup": session.join("files/plane.acf"), "original": root.join("plane.acf") }],
        });
        fs::write(
            session.join("manifest_legacy.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(revert_patch(&session).unwrap(), 1);
        assert_eq!(fs::read(root.join("plane.acf")).unwrap(), b"x");
    }
}

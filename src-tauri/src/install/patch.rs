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
    /// Existing destination directories, relative to the aircraft root.
    pub aircraft_directories: Vec<String>,
    /// Local memory key for this aircraft and archive layout.
    pub mapping_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchMappingPreview {
    pub file_count: usize,
    pub overwrite_count: usize,
    pub destination_exists: bool,
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
    pub mapping_previews: Vec<PatchMappingPreview>,
    pub unmapped: Vec<String>,
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
        build_mapping_plan(&files, &dirs, candidate, &aircraft_subdirs);
    let aircraft_root = fs::canonicalize(aircraft_dir)?;
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(&(
        fs::canonicalize(xplane_path)?,
        &aircraft_root,
        &candidate.folder_name,
        &files,
    ))?);

    Ok(PatchPlan {
        archive_tree,
        suggested_mappings,
        unmapped,
        aircraft_directories: matching::directory_tree(&aircraft_root)?,
        mapping_key: format!("{:x}", hash.finalize()),
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
    build_mapping_plan(
        files,
        dirs,
        &detection.candidates[0],
        &immediate_subdirs(aircraft_dir),
    )
}

fn build_mapping_plan(
    files: &[String],
    dirs: &BTreeSet<String>,
    candidate: &AircraftCandidate,
    aircraft_subdirs: &[String],
) -> (Vec<PatchMapping>, Vec<String>) {
    let mut mappings = if candidate.ambiguous_mapping || candidate.matched_count == 0 {
        Vec::new()
    } else {
        plan_at_offset(
            files,
            dirs,
            &candidate.best_offset,
            &candidate.best_destination,
            candidate.matched_count,
        )
        .0
    };
    for (source, destination) in matching::layout_mappings(files, dirs, aircraft_subdirs) {
        let livery = matching::is_livery_wrapper(&source, files);
        if mappings.iter().any(|mapping| {
            // A broader existing alignment that already places the anchor
            // correctly can include other siblings too. Keep that alignment.
            projected_dir(&source, &mapping.archive_subpath, &mapping.dest_subpath)
                .is_some_and(|dir| matching::path_key(&dir) == matching::path_key(&destination))
                // A narrower exact match into a different aircraft subtree
                // is stronger than a base directory name alone.
                || (!livery && projected_dir(&mapping.archive_subpath, &source, &destination)
                    .is_some_and(|dir| matching::path_key(&dir) != matching::path_key(&mapping.dest_subpath)))
        }) { continue; }
        // A conflicting broad alignment would preserve an extra wrapper, or
        // put a paint's files in the aircraft's shared objects directory.
        mappings.retain(|mapping| {
            projected_dir(&source, &mapping.archive_subpath, &mapping.dest_subpath).is_none()
                && projected_dir(&mapping.archive_subpath, &source, &destination).is_none()
        });
        mappings.push(PatchMapping {
            file_count: count_under(files, &source),
            archive_subpath: source,
            dest_subpath: destination,
            confidence: Confidence::Medium,
            reason: if livery {
                "liveryFolder"
            } else {
                "matchedLayout"
            }
            .to_string(),
        });
    }
    let prefixes = mappings
        .iter()
        .map(|mapping| mapping.archive_subpath.clone())
        .collect::<Vec<_>>();
    let unmapped = matching::uncovered(files, &prefixes);
    (mappings, unmapped)
}

/// Where an archive directory lands under a mapping, if that mapping owns it.
fn projected_dir(dir: &str, source: &str, destination: &str) -> Option<String> {
    if dir == source {
        return Some(destination.to_string());
    }
    let relative = strip_offset(dir, source)?;
    Some(if destination.is_empty() {
        relative.to_string()
    } else {
        format!("{destination}/{relative}")
    })
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
        if matching::is_livery_wrapper(dir, files)
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
    let files = list_archive_files(archive_path)?;
    let planned = plan_patch(&files, &aircraft_dir, mappings)?;
    let mut summary = install_summary(archive_path, &aircraft_dir, &planned.writes)?;
    summary.mapping_previews = planned.previews;
    summary.unmapped = matching::uncovered(&files, &planned.archive_prefixes);
    Ok(summary)
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
        mapping_previews: Vec::new(),
        unmapped: Vec::new(),
    })
}

fn plan_install_files(
    files: &[String],
    root: &Path,
    mappings: &[PatchMappingInput],
) -> Result<Vec<PatchFileWrite>> {
    Ok(plan_patch(files, root, mappings)?.writes)
}

struct PlannedPatch {
    writes: Vec<PatchFileWrite>,
    previews: Vec<PatchMappingPreview>,
    archive_prefixes: Vec<String>,
}

fn plan_patch(
    files: &[String],
    root: &Path,
    mappings: &[PatchMappingInput],
) -> Result<PlannedPatch> {
    if mappings.is_empty() {
        return Err(anyhow!("No patch mappings provided"));
    }
    let mut writes = Vec::new();
    let mut previews = Vec::with_capacity(mappings.len());
    let mut archive_prefixes = Vec::with_capacity(mappings.len());
    for mapping in mappings {
        let archive_sub = mapping_subpath(&mapping.archive_subpath)?;
        let dest_sub = mapping_subpath(&mapping.dest_subpath)?;
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
        previews.push(PatchMappingPreview {
            file_count: writes.len() - start,
            overwrite_count: writes[start..]
                .iter()
                .filter(|write| write.overwrite)
                .count(),
            destination_exists: root.join(dest_sub).is_dir(),
        });
        archive_prefixes.push(archive_sub);
    }
    writes.sort_by(|a, b| a.destination.cmp(&b.destination));
    validate_write_set(root, &writes)?;
    Ok(PlannedPatch {
        writes,
        previews,
        archive_prefixes,
    })
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

/// A single leading slash in a mapping starts at its archive or aircraft root.
fn mapping_subpath(value: &str) -> Result<String> {
    safe_relative_subpath(value.strip_prefix('/').unwrap_or(value))
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
                if is_filesystem_link(&metadata) {
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

fn is_filesystem_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
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
        zip_source: None,
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
        is_xlua_runtime: false,
        companion_paths: Vec::new(),
        scenery_options: Vec::new(),
        selected_scenery_option: None,
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

#[derive(Debug, Serialize, Deserialize)]
struct AddedFile {
    /// Aircraft-relative destination and the exact bytes installed by this patch.
    path: String,
    checksum: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PatchBackupManifest {
    #[serde(default)]
    patch: String,
    #[serde(default)]
    entries: Vec<BackupEntry>,
    /// None identifies older backups without an inventory of added files.
    #[serde(default)]
    added_files: Option<Vec<AddedFile>>,
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
    transact_patch_files(root, writes, &[], backup, replace, || Ok(()))?;
    Ok(writes.len())
}

enum PatchFileAction<'a> {
    Replace(&'a Path),
    Remove(&'a str),
}

fn transact_patch_files(
    root: &Path,
    writes: &[(PathBuf, PathBuf)],
    removals: &[(PathBuf, String)],
    backup: Option<&Path>,
    replace: impl Fn(&Path, &Path) -> Result<()>,
    verify: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let edits: Vec<_> = writes
        .iter()
        .map(|(source, destination)| (PatchFileAction::Replace(source), destination))
        .chain(
            removals
                .iter()
                .map(|(destination, checksum)| (PatchFileAction::Remove(checksum), destination)),
        )
        .collect();
    if edits.is_empty() {
        return verify();
    }
    // A temporary snapshot is required for rollback even when the user opts
    // out of a retained backup. Do not write anything before every copy exists.
    let journal = tempfile::Builder::new()
        .prefix(".xfast-patch-rollback-")
        .tempdir_in(root)?;
    let mut originals = Vec::with_capacity(edits.len());
    for (_, destination) in &edits {
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
    let mut applied = 0;
    let mut failure = None;
    for (action, destination) in &edits {
        let result = (|| {
            validate_destination(root, destination)?;
            match action {
                PatchFileAction::Remove(checksum) => {
                    // Recheck immediately before unlinking, after snapshots and
                    // any restores that may have taken time to finish.
                    if !file_checksum(destination)?.eq_ignore_ascii_case(checksum) {
                        return Err(anyhow!(
                            "Patch file changed during restore: {}",
                            destination.display()
                        ));
                    }
                    fs::remove_file(destination)?;
                    Ok(())
                }
                PatchFileAction::Replace(source) => {
                    let parent = destination
                        .parent()
                        .ok_or_else(|| anyhow!("Invalid patch destination"))?;
                    let missing: Vec<_> = parent.ancestors().take_while(|p| !p.exists()).collect();
                    for dir in missing.into_iter().rev() {
                        fs::create_dir(dir)?;
                        created_dirs.push(dir.to_path_buf());
                    }
                    replace(source, destination)
                }
            }
        })();
        if let Err(error) = result {
            failure =
                Some(error.context(format!("Patch write failed at {}", destination.display())));
            break;
        }
        applied += 1;
    }
    // Verification runs while the rollback journal is still intact.
    if let Some(error) = failure.or_else(|| verify().err()) {
        let mut rollback_errors = Vec::new();
        for ((_, dest), original) in edits[..applied].iter().zip(&originals[..applied]).rev() {
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
                "{error:#}; rollback failed: {}. Recovery files: {}",
                rollback_errors.join("; "),
                recovery.display()
            ));
        }
        return Err(error.context("Patch changes were rolled back"));
    }
    Ok(())
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
    if backup.parent() != Some(root.join("_xfast_patch_backups").as_path()) {
        return Err(anyhow!("Invalid patch backup directory"));
    }
    validate_destination_inner(root, backup, true)?;
    let backup_root = backup.parent().unwrap();
    fs::create_dir_all(backup_root)?;
    // Publish only complete sessions, including all snapshots and their manifest.
    let staging = tempfile::tempdir_in(backup_root)?;
    let mut manifest = PatchBackupManifest::default();
    let mut added_files = Vec::new();
    for ((source, destination), original) in writes.iter().zip(originals) {
        let relative = destination.strip_prefix(root)?;
        let Some(original) = original else {
            added_files.push(AddedFile {
                path: relative.to_string_lossy().replace('\\', "/"),
                checksum: file_checksum(source)?,
            });
            continue;
        };
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
    manifest.added_files = Some(added_files);
    fs::write(
        staging.path().join("manifest_patch.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::rename(staging.path(), backup).context("Could not publish the patch backup")?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchBackupFile {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchBackupSession {
    pub id: String,
    pub name: String,
    pub timestamp: Option<u64>,
    pub files: Vec<PatchBackupFile>,
    pub added_files: Option<Vec<String>>,
    pub error: Option<String>,
}

struct RestoreFile {
    source: PathBuf,
    destination: PathBuf,
    info: PatchBackupFile,
    verified_checksum: Option<String>,
}

struct BackupContents {
    files: Vec<RestoreFile>,
    added_files: Option<Vec<AddedFile>>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchRestoreResult {
    pub restored_count: usize,
    pub removed_count: usize,
    pub skipped_count: usize,
    pub backup_cleanup_error: Option<String>,
}

fn backup_aircraft_root(xplane_path: &str, aircraft_folder: &str) -> Result<PathBuf> {
    let aircraft = resolve_aircraft_dir_by_name(xplane_path, aircraft_folder)
        .ok_or_else(|| anyhow!("Aircraft folder not found: {}", aircraft_folder))?;
    Ok(fs::canonicalize(aircraft)?)
}

/// Discover the selected aircraft's backup sessions. A damaged session is
/// reported individually and cannot prevent browsing the other backups.
pub fn list_aircraft_backups(
    xplane_path: &str,
    aircraft_folder: &str,
) -> Result<Vec<PatchBackupSession>> {
    let root = backup_aircraft_root(xplane_path, aircraft_folder)?;
    let backup_root = root.join("_xfast_patch_backups");
    validate_destination_inner(&root, &backup_root, true)?;
    let entries = match fs::read_dir(&backup_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut backups = Vec::new();
    for entry in entries {
        let entry = entry?;
        let Ok(id) = entry.file_name().into_string() else {
            continue;
        };
        let name = backup_name(&id).to_string();
        // tempfile creates unpublished staging directories with a .tmp prefix.
        if !entry.file_type()?.is_dir() || (id.starts_with(".tmp") && id == name) {
            continue;
        }
        let mut timestamp = None;
        let result = read_backup_session(&root, &entry.path(), false, &mut timestamp);
        let (files, added_files, error) = match result {
            Ok(contents) => (
                contents.files.into_iter().map(|file| file.info).collect(),
                contents
                    .added_files
                    .map(|files| files.into_iter().map(|file| file.path).collect()),
                None,
            ),
            Err(error) => (Vec::new(), None, Some(format!("{error:#}"))),
        };
        backups.push(PatchBackupSession {
            id,
            name,
            timestamp,
            files,
            added_files,
            error,
        });
    }
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then_with(|| a.id.cmp(&b.id)));
    Ok(backups)
}

fn backup_name(id: &str) -> &str {
    id.rsplit_once('_')
        .filter(|(_, suffix)| {
            Uuid::parse_str(suffix).is_ok()
                || (suffix.len() == 8 && suffix.bytes().all(|b| b.is_ascii_hexdigit()))
        })
        .map_or(id, |(name, _)| name)
}

/// Resolve the selected session within this aircraft. The frontend never needs
/// a filesystem path or a folder picker to restore a backup.
pub fn restore_aircraft_backup(
    xplane_path: &str,
    aircraft_folder: &str,
    session_id: &str,
    remove_added_files: bool,
) -> Result<PatchRestoreResult> {
    if session_id.is_empty()
        || session_id.contains('/')
        || safe_relative_subpath(session_id)? != session_id
    {
        return Err(anyhow!("Invalid patch backup session"));
    }
    let root = backup_aircraft_root(xplane_path, aircraft_folder)?;
    revert_patch(
        &root.join("_xfast_patch_backups").join(session_id),
        remove_added_files,
        replace_file,
    )
}

fn revert_patch(
    dir: &Path,
    remove_added_files: bool,
    replace: impl Fn(&Path, &Path) -> Result<()>,
) -> Result<PatchRestoreResult> {
    let _guard = PATCH_WRITE_LOCK
        .lock()
        .map_err(|_| anyhow!("Patch write lock poisoned"))?;
    let root = dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| anyhow!("Invalid patch backup session"))?;
    let contents = read_backup_session(root, dir, true, &mut None)?;
    let writes: Vec<_> = contents
        .files
        .iter()
        .map(|file| (file.source.clone(), file.destination.clone()))
        .collect();
    if writes.is_empty() && !remove_added_files {
        return Err(anyhow!("Select removal of added files to undo this patch"));
    }
    let mut result = PatchRestoreResult {
        restored_count: writes.len(),
        ..Default::default()
    };
    let mut removals = Vec::new();
    let mut already_absent = Vec::new();
    if remove_added_files {
        let files = contents
            .added_files
            .ok_or_else(|| anyhow!("This backup does not track files added by the patch"))?;
        for file in files {
            let destination = root.join(&file.path);
            // Changed files, links and replacements with directories belong to
            // the current aircraft state and are left alone.
            if validate_destination(root, &destination).is_err() {
                result.skipped_count += 1;
                continue;
            }
            match fs::symlink_metadata(&destination) {
                Ok(metadata)
                    if metadata.is_file()
                        && file_checksum(&destination)?.eq_ignore_ascii_case(&file.checksum) =>
                {
                    removals.push((destination, file.checksum));
                }
                Ok(_) => result.skipped_count += 1,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    already_absent.push(destination)
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
    transact_patch_files(root, &writes, &removals, None, replace, || {
        for file in &contents.files {
            validate_destination(root, &file.destination)?;
            if file.verified_checksum.as_deref() != Some(file_checksum(&file.destination)?.as_str())
            {
                return Err(anyhow!(
                    "Restored file failed verification: {}",
                    file.info.path
                ));
            }
        }
        for destination in removals.iter().map(|(path, _)| path).chain(&already_absent) {
            validate_destination(root, destination)?;
            match fs::symlink_metadata(destination) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
                Ok(_) => {
                    return Err(anyhow!(
                        "Removed file is still present: {}",
                        destination.display()
                    ))
                }
            }
        }
        Ok(())
    })?;
    result.removed_count = removals.len();
    result.backup_cleanup_error = retire_verified_backup(root, dir)
        .err()
        .map(|error| format!("{error:#}"));
    Ok(result)
}

fn retire_verified_backup(root: &Path, dir: &Path) -> Result<()> {
    let backup_root = root.join("_xfast_patch_backups");
    validate_destination_inner(root, dir, true)?;
    if dir.parent() != Some(backup_root.as_path()) || fs::canonicalize(dir)? != dir {
        return Err(anyhow!("Invalid patch backup session"));
    }
    // Move the complete session out of the browser before deleting it. If
    // cleanup is interrupted, it cannot leave a half-deleted session listed.
    let cleanup = tempfile::tempdir_in(&backup_root)?;
    validate_destination_inner(root, cleanup.path(), true)?;
    fs::rename(dir, cleanup.path().join("session"))?;
    let cleanup_path = cleanup.path().to_path_buf();
    cleanup
        .close()
        .with_context(|| format!("Could not delete {}", cleanup_path.display()))
}

/// Use the same manifest and path validation for display and restore. The
/// browser reads metadata only; restoration also checks each recorded hash.
fn read_backup_session(
    root: &Path,
    dir: &Path,
    verify_checksums: bool,
    timestamp: &mut Option<u64>,
) -> Result<BackupContents> {
    if dir.parent() != Some(root.join("_xfast_patch_backups").as_path()) {
        return Err(anyhow!("Invalid patch backup session"));
    }
    validate_destination_inner(root, dir, true)?;
    let mut files = Vec::new();
    let mut added_files = Some(Vec::new());
    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("manifest_") && name.ends_with(".json")) {
            continue;
        }
        validate_destination_inner(root, &entry.path(), true)?;
        if !entry.file_type()?.is_file() {
            return Err(anyhow!("Invalid patch backup manifest {name}"));
        }
        let modified = entry
            .metadata()?
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
        *timestamp = (*timestamp).max(modified);
        let manifest: PatchBackupManifest = serde_json::from_slice(&fs::read(entry.path())?)
            .with_context(|| format!("Invalid patch backup manifest {name}"))?;
        match (&mut added_files, manifest.added_files) {
            (Some(all), Some(files)) => all.extend(files),
            (_, None) => added_files = None,
            (None, Some(_)) => {}
        }
        for file in manifest.entries {
            let source = PathBuf::from(file.backup);
            let destination = PathBuf::from(file.original);
            validate_destination(root, &destination)?;
            let relative = destination.strip_prefix(root)?;
            if source != dir.join("files").join(relative) {
                return Err(anyhow!(
                    "Patch backup entry does not match its original location"
                ));
            }
            validate_destination_inner(root, &source, true)?;
            let path = relative.to_string_lossy().replace('\\', "/");
            let metadata = fs::metadata(&source)
                .with_context(|| format!("Cannot read patch backup file: {path}"))?;
            if !metadata.is_file() {
                return Err(anyhow!("Invalid patch backup file: {path}"));
            }
            let verified_checksum = if verify_checksums {
                let actual = file_checksum(&source)?;
                if file
                    .checksum
                    .as_ref()
                    .is_some_and(|checksum| *checksum != actual)
                {
                    return Err(anyhow!(
                        "Patch backup file failed integrity checking: {path}"
                    ));
                }
                Some(actual)
            } else {
                None
            };
            if seen.insert(matching::path_key(&path)) {
                files.push(RestoreFile {
                    source,
                    destination,
                    verified_checksum,
                    info: PatchBackupFile {
                        path,
                        size_bytes: metadata.len(),
                    },
                });
            }
        }
    }
    if let Some(added) = &mut added_files {
        for file in &*added {
            if file.path.is_empty()
                || safe_relative_subpath(&file.path)? != file.path
                || file.checksum.len() != 64
                || !file.checksum.bytes().all(|byte| byte.is_ascii_hexdigit())
                || !seen.insert(matching::path_key(&file.path))
            {
                return Err(anyhow!("Invalid added file in patch backup: {}", file.path));
            }
        }
        added.sort_by(|a, b| a.path.cmp(&b.path));
    }
    if files.is_empty() && added_files.as_ref().is_none_or(Vec::is_empty) {
        return Err(anyhow!("No files to restore in this patch backup"));
    }
    files.sort_by(|a, b| a.info.path.cmp(&b.info.path));
    Ok(BackupContents { files, added_files })
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

/// Immediate sub-directory names used to infer the automatic mappings.
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
            "//tmp/plane",
            "/../OtherPlane",
            "/C:/Aircraft",
            "/\\server\\share",
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
    fn mapping_slash_refers_to_the_archive_or_aircraft_root() {
        let ac = make_aircraft(&["fmod/panel.obj"]);
        let files = to_owned(&["pack/panel.obj"]);
        let plan = |source: &str, destination: &str| {
            plan_install_files(
                &files,
                ac.path(),
                &[PatchMappingInput {
                    archive_subpath: source.into(),
                    dest_subpath: destination.into(),
                }],
            )
            .unwrap()
        };
        let rooted = plan("/pack", "/fmod");
        assert_eq!(rooted[0].destination, "fmod/panel.obj");
        assert!(rooted[0].overwrite);
        assert_eq!(plan("/pack", "/")[0].destination, "panel.obj");
        assert_eq!(plan("/", "/")[0].destination, "pack/panel.obj");
        assert!(safe_relative_subpath("/fmod").is_err());
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
        assert!(revert_patch(&backup, false, replace_file).is_err());
        assert!(backup.join("manifest_patch.json").is_file());
        assert_eq!(fs::read(root.join("one.acf")).unwrap(), b"patched");
        fs::write(backup.join("files/two.acf"), b"x").unwrap();
        let sibling = backup.with_file_name("another_session");
        fs::create_dir(&sibling).unwrap();
        fs::write(sibling.join("keep.txt"), b"other backup").unwrap();
        let restored = revert_patch(&backup, false, replace_file).unwrap();
        assert_eq!(restored.restored_count, 2);
        assert!(restored.backup_cleanup_error.is_none());
        assert!(!backup.exists());
        assert_eq!(fs::read(sibling.join("keep.txt")).unwrap(), b"other backup");
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
    fn preview_reflects_the_selected_destination_for_each_mapping() {
        let ac = make_aircraft(&["objects/deep/v2/panel.obj", "objects/deep/v2/old.obj"]);
        let files = to_owned(&["base/panel.obj", "base/new.obj", "extras/readme.txt"]);
        let planned = plan_patch(
            &files,
            ac.path(),
            &[
                PatchMappingInput {
                    archive_subpath: "base".into(),
                    dest_subpath: "objects/deep/v2".into(),
                },
                PatchMappingInput {
                    archive_subpath: "extras/readme.txt".into(),
                    dest_subpath: "custom/another/deep/folder".into(),
                },
            ],
        )
        .unwrap();
        assert_eq!(planned.writes.len(), 3);
        assert_eq!(planned.previews[0].file_count, 2);
        assert_eq!(planned.previews[0].overwrite_count, 1);
        assert!(planned.previews[0].destination_exists);
        assert_eq!(planned.previews[1].file_count, 1);
        assert_eq!(planned.previews[1].overwrite_count, 0);
        assert!(!planned.previews[1].destination_exists);
        assert!(matching::uncovered(&files, &planned.archive_prefixes).is_empty());
    }

    #[test]
    fn mapping_memory_keys_follow_archive_layout_and_aircraft() {
        use std::io::Write;

        let fixture = tempfile::tempdir().unwrap();
        let xplane = fixture.path().to_str().unwrap();
        for folder in ["A321", "A321-copy"] {
            let aircraft = fixture.path().join("Aircraft").join(folder);
            for relative in ["objects/nested/two/three", "_xfast_patch_backups/session"] {
                fs::create_dir_all(aircraft.join(relative)).unwrap();
            }
            fs::write(aircraft.join("A321.acf"), b"acf").unwrap();
        }
        let archive = |name: &str, entry: &str| {
            let path = fixture.path().join(name);
            let mut writer = zip::ZipWriter::new(fs::File::create(&path).unwrap());
            writer
                .start_file(entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(name.as_bytes()).unwrap();
            writer.finish().unwrap();
            path
        };
        let first = archive("first.zip", "base/patch.obj");
        let copy = archive("copy.zip", "base/patch.obj");
        let changed = archive("changed.zip", "base/other.obj");
        let plan = infer_mappings(&first, xplane, "A321").unwrap();
        assert!(plan
            .aircraft_directories
            .contains(&"objects/nested/two/three".to_string()));
        assert!(plan
            .aircraft_directories
            .iter()
            .all(|dir| !dir.starts_with("_xfast_patch_backups")));
        assert_eq!(
            plan.mapping_key,
            infer_mappings(&copy, xplane, "A321").unwrap().mapping_key
        );
        assert_ne!(
            plan.mapping_key,
            infer_mappings(&changed, xplane, "A321")
                .unwrap()
                .mapping_key
        );
        assert_ne!(
            plan.mapping_key,
            infer_mappings(&first, xplane, "A321-copy")
                .unwrap()
                .mapping_key
        );
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
        assert_eq!(
            restore_aircraft_backup(
                sim,
                folder,
                backup.file_name().unwrap().to_str().unwrap(),
                false
            )
            .unwrap()
            .restored_count,
            1
        );
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
        assert!(revert_patch(&session, true, replace_file).is_err());
        assert_eq!(fs::read(root.join("plane.acf")).unwrap(), b"patched");
        assert_eq!(
            revert_patch(&session, false, replace_file)
                .unwrap()
                .restored_count,
            1
        );
        assert_eq!(fs::read(root.join("plane.acf")).unwrap(), b"x");
    }

    #[test]
    fn model_and_layout_align_an_engine_pack_with_no_existing_assets() {
        let a321 = make_aircraft(&["a321.acf", "objects/engines.obj"]);
        let a346 = make_aircraft(&["a346.acf", "objects/engines.obj"]);
        let other = make_aircraft(&["b777.acf", "docs/MANUAL.pdf"]);
        let files = to_owned(&[
            "MANUAL.pdf",
            "Toliss321 Base Folder/objects/CFM56/cfm56_l_engine.obj",
            "Toliss321 Base Folder/objects/CFM56/particles/CFM-Particles.pss",
            "TolissA321 Liveries/Delta/objects/CFM56/cfm_l_engine.dds",
            "TolissA321 Liveries/Lufthansa Grey/objects/CFM56/cfm_l_engine.dds",
        ]);
        let targets = [
            target("777", other.path()),
            target("A346", a346.path()),
            target("A321", a321.path()),
        ];
        let detection = rank_candidates(&files, &targets).unwrap();
        assert_eq!(detection.recommended_folder.as_deref(), Some("A321"));
        assert_eq!(detection.candidates[0].matched_count, 0);
        let (mappings, unmapped) = build_mapping_plan(
            &files,
            &derive_dirs(&files),
            &detection.candidates[0],
            &immediate_subdirs(a321.path()),
        );
        assert_eq!(unmapped, to_owned(&["MANUAL.pdf"]));
        assert_eq!(mappings.len(), 2);
        let inputs: Vec<_> = mappings
            .into_iter()
            .map(|mapping| PatchMappingInput {
                archive_subpath: mapping.archive_subpath,
                dest_subpath: mapping.dest_subpath,
            })
            .collect();
        let writes = plan_install_files(&files, a321.path(), &inputs).unwrap();
        assert_eq!(
            writes
                .iter()
                .map(|write| write.destination.as_str())
                .collect::<Vec<_>>(),
            [
                "liveries/Delta/objects/CFM56/cfm_l_engine.dds",
                "liveries/Lufthansa Grey/objects/CFM56/cfm_l_engine.dds",
                "objects/CFM56/cfm56_l_engine.obj",
                "objects/CFM56/particles/CFM-Particles.pss",
            ]
        );
        assert!(writes.iter().all(|write| !write.overwrite));
        let mut duplicate = targets.into_iter().collect::<Vec<_>>();
        duplicate.push(target("A321 copy", a321.path()));
        assert!(rank_candidates(&files, &duplicate)
            .unwrap()
            .recommended_folder
            .is_none());
    }

    #[test]
    fn aligned_overwrite_still_installs_independent_new_branches() {
        let a321 = make_aircraft(&[
            "a321.acf",
            "objects/engines.obj",
            "liveries/Delta/objects/CFM56/cfm_l_engine.dds",
        ]);
        let files = to_owned(&[
            "Release/Toliss321 Base Folder/objects/CFM56/cfm56_l_engine.obj",
            "Release/TolissA321 Liveries/Delta/objects/CFM56/cfm_l_engine.dds",
            "Release/TolissA321 Liveries/Delta/objects/CFM56/cfm_r_engine.dds",
        ]);
        let (mappings, unmapped) = plan_mappings(&files, &derive_dirs(&files), a321.path());
        assert!(unmapped.is_empty());
        let inputs: Vec<_> = mappings
            .into_iter()
            .map(|mapping| PatchMappingInput {
                archive_subpath: mapping.archive_subpath,
                dest_subpath: mapping.dest_subpath,
            })
            .collect();
        let writes = plan_install_files(&files, a321.path(), &inputs).unwrap();
        assert_eq!(writes.len(), 3);
        let overwritten: Vec<_> = writes
            .iter()
            .filter(|write| write.overwrite)
            .map(|write| write.destination.as_str())
            .collect();
        assert_eq!(
            overwritten,
            ["liveries/Delta/objects/CFM56/cfm_l_engine.dds"]
        );
        assert!(writes
            .iter()
            .any(|write| write.destination == "objects/CFM56/cfm56_l_engine.obj"));
    }

    #[test]
    fn a_manual_or_generic_layout_does_not_identify_an_aircraft() {
        let aircraft = make_aircraft(&["a321.acf", "objects/body.obj", "docs/MANUAL.pdf"]);
        let files = to_owned(&["MANUAL.pdf", "Generic Pack/objects/extra.obj"]);
        let result = rank_candidates(&files, &[target("A321", aircraft.path())]).unwrap();
        assert!(result.recommended_folder.is_none());
        assert_eq!(result.candidates[0].matched_count, 0);
    }

    #[test]
    fn lists_backup_contents_by_aircraft_and_restores_the_selected_session() {
        let sim = tempfile::tempdir().unwrap();
        let xplane = sim.path().to_str().unwrap();
        let root = sim.path().join("Aircraft/Vendor/Demo");
        let other = sim.path().join("Aircraft/Other/Demo");
        for aircraft in [&root, &other] {
            fs::create_dir_all(aircraft).unwrap();
            fs::write(aircraft.join("plane.acf"), b"aircraft").unwrap();
        }
        let root = fs::canonicalize(root).unwrap();
        assert!(list_aircraft_backups(xplane, "Vendor/Demo")
            .unwrap()
            .is_empty());
        let staged = make_aircraft(&["one.obj", "two.obj"]);
        let dest = root.join("objects/body.obj");
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, b"stock").unwrap();
        fs::write(staged.path().join("one.obj"), b"first patch").unwrap();
        let backup_root = root.join("_xfast_patch_backups");
        let old_id = "old_patch_a1b2c3d4";
        let new_id = "new_patch_39d05131-85f7-4c6c-9d94-3aa25b981ad2";
        for (id, source, seconds) in [
            (old_id, "one.obj", 1_700_000_000),
            (new_id, "two.obj", 1_700_000_100),
        ] {
            let session = backup_root.join(id);
            write_patch_files(
                &root,
                &[(staged.path().join(source), dest.clone())],
                Some(&session),
                replace_file,
            )
            .unwrap();
            fs::OpenOptions::new()
                .write(true)
                .open(session.join("manifest_patch.json"))
                .unwrap()
                .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds))
                .unwrap();
        }
        fs::create_dir(backup_root.join(".tmpunpublished")).unwrap();
        let broken = backup_root.join("broken");
        fs::create_dir(&broken).unwrap();
        fs::write(broken.join("manifest_patch.json"), b"invalid json").unwrap();
        let backups = list_aircraft_backups(xplane, "Vendor/Demo").unwrap();
        assert_eq!(backups.len(), 3);
        assert_eq!(backups[0].id, "broken");
        assert!(backups[0].error.is_some());
        assert_eq!(backups[1].id, new_id);
        assert_eq!(backups[1].timestamp, Some(1_700_000_100_000));
        assert_eq!(backups[1].name, "new_patch");
        assert_eq!(backups[1].files[0].path, "objects/body.obj");
        assert_eq!(backups[1].files[0].size_bytes, 11);
        assert_eq!(backups[2].id, old_id);
        assert_eq!(backups[2].name, "old_patch");
        assert!(list_aircraft_backups(xplane, "Other/Demo")
            .unwrap()
            .is_empty());
        assert!(restore_aircraft_backup(xplane, "Other/Demo", new_id, false).is_err());
        assert!(restore_aircraft_backup(xplane, "Vendor/Demo", "../broken", false).is_err());
        assert!(restore_aircraft_backup(xplane, "Vendor/Demo", "broken", false).is_err());
        assert_eq!(fs::read(&dest).unwrap(), b"x");
        assert_eq!(
            restore_aircraft_backup(xplane, "Vendor/Demo", new_id, false)
                .unwrap()
                .restored_count,
            1
        );
        assert_eq!(fs::read(&dest).unwrap(), b"first patch");
        assert_eq!(
            restore_aircraft_backup(xplane, "Vendor/Demo", old_id, false)
                .unwrap()
                .restored_count,
            1
        );
        assert_eq!(fs::read(&dest).unwrap(), b"stock");
    }

    #[test]
    fn restore_can_remove_only_unchanged_added_files() {
        let aircraft = make_aircraft(&["plane.acf", "untouched.obj"]);
        let root = fs::canonicalize(aircraft.path()).unwrap();
        let staged = make_aircraft(&[
            "plane.acf",
            "extras/added.obj",
            "extras/changed.obj",
            "extras/absent.obj",
        ]);
        fs::write(staged.path().join("plane.acf"), b"patched plane").unwrap();
        let files = [
            "plane.acf",
            "extras/added.obj",
            "extras/changed.obj",
            "extras/absent.obj",
        ];
        let writes = files.map(|file| (staged.path().join(file), root.join(file)));
        let backup = root.join("_xfast_patch_backups/session");
        write_patch_files(&root, &writes, Some(&backup), replace_file).unwrap();
        fs::write(
            root.join("extras/changed.obj"),
            b"modified outside the patch",
        )
        .unwrap();
        fs::remove_file(root.join("extras/absent.obj")).unwrap();
        let remove = revert_patch(&backup, true, replace_file).unwrap();
        assert_eq!(
            (
                remove.restored_count,
                remove.removed_count,
                remove.skipped_count
            ),
            (1, 1, 1)
        );
        assert!(remove.backup_cleanup_error.is_none());
        assert!(!backup.exists());
        assert!(!root.join("extras/added.obj").exists());
        assert_eq!(
            fs::read(root.join("extras/changed.obj")).unwrap(),
            b"modified outside the patch"
        );
        assert_eq!(fs::read(root.join("untouched.obj")).unwrap(), b"x");
    }

    #[test]
    fn an_add_only_patch_has_a_listed_backup_and_can_be_undone() {
        let sim = tempfile::tempdir().unwrap();
        let root = sim.path().join("Aircraft/Test");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("plane.acf"), b"original").unwrap();
        let root = fs::canonicalize(root).unwrap();
        let staged = make_aircraft(&["new.obj"]);
        let writes = [(staged.path().join("new.obj"), root.join("new.obj"))];
        let session = root.join("_xfast_patch_backups/session");
        write_patch_files(&root, &writes, Some(&session), replace_file).unwrap();
        let xplane = sim.path().to_str().unwrap();
        let backups = list_aircraft_backups(xplane, "Test").unwrap();
        assert_eq!(backups.len(), 1);
        assert!(backups[0].files.is_empty());
        assert_eq!(backups[0].added_files.as_deref().unwrap(), ["new.obj"]);
        assert!(backups[0].error.is_none());
        assert!(restore_aircraft_backup(xplane, "Test", "session", false).is_err());
        assert!(session.is_dir());
        let removed = restore_aircraft_backup(xplane, "Test", "session", true).unwrap();
        assert_eq!((removed.restored_count, removed.removed_count), (0, 1));
        assert!(removed.backup_cleanup_error.is_none());
        assert!(!session.exists());
        assert!(!root.join("new.obj").exists());
    }

    #[test]
    fn failed_cleanup_rolls_back_prior_removals_and_restores() {
        let aircraft = make_aircraft(&["plane.acf", "first.obj", "second.obj"]);
        let root = aircraft.path();
        let staged = make_aircraft(&["plane.acf"]);
        fs::write(staged.path().join("plane.acf"), b"restored version").unwrap();
        let writes = [(staged.path().join("plane.acf"), root.join("plane.acf"))];
        let first = root.join("first.obj");
        let removals = [
            (first.clone(), file_checksum(&first).unwrap()),
            (root.join("second.obj"), "invalidated checksum".into()),
        ];
        assert!(
            transact_patch_files(root, &writes, &removals, None, replace_file, || Ok(())).is_err()
        );
        for file in ["plane.acf", "first.obj", "second.obj"] {
            assert_eq!(fs::read(root.join(file)).unwrap(), b"x");
        }
    }

    #[test]
    fn failed_post_restore_verification_rolls_back_and_keeps_the_backup() {
        let aircraft = make_aircraft(&["plane.acf"]);
        let root = fs::canonicalize(aircraft.path()).unwrap();
        let staged = make_aircraft(&["plane.acf", "added.obj"]);
        fs::write(staged.path().join("plane.acf"), b"patched").unwrap();
        let backup = root.join("_xfast_patch_backups/session");
        let writes =
            ["plane.acf", "added.obj"].map(|name| (staged.path().join(name), root.join(name)));
        write_patch_files(&root, &writes, Some(&backup), replace_file).unwrap();

        let error = revert_patch(&backup, true, |_, destination| {
            // Simulate a successful write that produced different bytes.
            fs::write(destination, b"different bytes")?;
            Ok(())
        })
        .unwrap_err();
        assert!(format!("{error:#}").contains("Restored file failed verification"));
        assert_eq!(fs::read(root.join("plane.acf")).unwrap(), b"patched");
        assert_eq!(fs::read(root.join("added.obj")).unwrap(), b"x");
        assert_eq!(fs::read(backup.join("files/plane.acf")).unwrap(), b"x");
        assert!(backup.join("manifest_patch.json").is_file());

        let result = revert_patch(&backup, true, replace_file).unwrap();
        assert!(result.backup_cleanup_error.is_none());
        assert_eq!(fs::read(root.join("plane.acf")).unwrap(), b"x");
        assert!(!root.join("added.obj").exists());
        assert!(!backup.exists());
    }
}

//! Aircraft model "patch" support.
//!
//! A patch is an archive whose contents are overlaid (merged, overwriting
//! existing files) into an already-installed aircraft folder. Patches are not
//! signature-detectable (they vary wildly in internal structure and often
//! contain `.xpl`/`.lua`/`.acf`-adjacent files that the scanner would
//! mis-classify), so they are entirely user-directed.
//!
//! Because the user picks (or we auto-detect) a target aircraft that already
//! exists on disk, we can align the patch against the *real* aircraft file tree
//! — far more reliable than guessing from folder names. This module provides:
//!
//! * Level 1 — [`detect_target_aircraft`]: align the patch against every
//!   installed aircraft and recommend the best match (most aligned files).
//! * Level 2 — [`infer_mappings`]: for the chosen aircraft, infer how each part
//!   of the archive maps into the aircraft folder (root overlay, liveries, …).
//!
//! Both levels score using cheap existence-probes (the patch's own paths used
//! as queries against the aircraft via `stat`) rather than walking the
//! aircraft's (potentially huge) file tree.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::models::{AddonType, AircraftInfo, InstallTask};
use crate::scanner::Scanner;

/// Directory names that, when found directly inside a folder, mark that folder
/// as an X-Plane aircraft "root" (i.e. a candidate overlay target). Deliberately
/// excludes `liveries` — liveries are handled as a separate, additive mapping
/// and a lone `liveries/` child must not make a wrapper folder look like a root.
const STRONG_LANDMARK_DIRS: &[&str] = &[
    "objects",
    "cockpit",
    "cockpit_3d",
    "sounds",
    "fmod",
    "plugins",
    "airfoils",
    "systems",
];

const MAX_SAMPLE: usize = 200;
const MAX_OFFSET_DEPTH: usize = 3;
const MAX_TREE: usize = 400;
/// Minimum aligned files (and ratio) before we *recommend* an aircraft outright.
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
    /// Number of sampled patch paths that already exist in this aircraft.
    pub matched_count: usize,
    pub sample_size: usize,
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

// --------------------------------------------------------------------------
// Level 1 — detect the target aircraft
// --------------------------------------------------------------------------

/// Align the patch against every installed aircraft and rank them by how many
/// of the patch's files already exist in each. Cheap: per-aircraft existence
/// probes of a bounded sample, run in parallel.
pub fn detect_target_aircraft(
    archive_path: &Path,
    xplane_path: &str,
) -> Result<TargetDetection> {
    let files = list_archive_files(archive_path).unwrap_or_default();
    if files.is_empty() {
        // Could not list (e.g. encrypted headers): no automatic recommendation.
        return Ok(TargetDetection {
            candidates: Vec::new(),
            recommended_folder: None,
        });
    }

    let dirs = derive_dirs(&files);
    let offsets = candidate_offsets(&files, &dirs);
    // One sample per offset, identical across all aircraft so scores compare fairly.
    let samples: Vec<(String, Vec<String>)> = offsets
        .iter()
        .map(|off| (off.clone(), build_sample(&files, off)))
        .filter(|(_, sample)| !sample.is_empty())
        .collect();

    let aircraft = crate::management_index::scan_aircraft(Path::new(xplane_path))?.entries;

    let mut candidates: Vec<AircraftCandidate> = aircraft
        .par_iter()
        .filter_map(|ac| {
            let dir = resolve_aircraft_dir(xplane_path, ac)?;
            let mut best_offset = String::new();
            let mut best_matched = 0usize;
            let mut best_sample = samples.first().map(|(_, s)| s.len()).unwrap_or(0);
            for (off, sample) in &samples {
                let matched = sample.iter().filter(|rel| dir.join(rel).exists()).count();
                if matched > best_matched {
                    best_matched = matched;
                    best_offset = off.clone();
                    best_sample = sample.len();
                }
            }
            Some(AircraftCandidate {
                folder_name: ac.folder_name.clone(),
                display_name: ac.display_name.clone(),
                matched_count: best_matched,
                sample_size: best_sample,
                best_offset,
                confidence: confidence_for(best_matched, best_sample),
            })
        })
        .collect();

    // Most-aligned first; stable tie-break by name.
    candidates.sort_by(|a, b| {
        b.matched_count
            .cmp(&a.matched_count)
            .then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
    });

    let recommended_folder = candidates.first().and_then(|c| {
        let ratio = c.matched_count as f64 / c.sample_size.max(1) as f64;
        if c.matched_count >= MIN_RECOMMEND_MATCH && ratio >= MIN_RECOMMEND_RATIO {
            Some(c.folder_name.clone())
        } else {
            None
        }
    });

    Ok(TargetDetection {
        candidates,
        recommended_folder,
    })
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

    let files = list_archive_files(archive_path).unwrap_or_default();
    let dirs = derive_dirs(&files);
    let archive_tree: Vec<String> = dirs.iter().take(MAX_TREE).cloned().collect();
    let aircraft_subdirs = immediate_subdirs(&aircraft_dir);

    if files.is_empty() {
        // No listing available — fall back to a single low-confidence root overlay.
        return Ok(PatchPlan {
            archive_tree,
            suggested_mappings: vec![PatchMapping {
                archive_subpath: String::new(),
                dest_subpath: String::new(),
                file_count: 0,
                confidence: Confidence::Low,
                reason: "couldNotInspectArchive".to_string(),
            }],
            unmapped: Vec::new(),
            aircraft_subdirs,
        });
    }

    let (suggested_mappings, unmapped) = plan_mappings(&files, &dirs, &aircraft_dir);

    Ok(PatchPlan {
        archive_tree,
        suggested_mappings,
        unmapped,
        aircraft_subdirs,
    })
}

/// Pure mapping core (no archive I/O): given the archive's file list and the
/// real aircraft directory, decide the root-overlay mapping plus any liveries
/// mapping, and report leftovers. Split out so it can be unit-tested directly.
fn plan_mappings(
    files: &[String],
    dirs: &BTreeSet<String>,
    aircraft_dir: &Path,
) -> (Vec<PatchMapping>, Vec<String>) {
    // 1. Choose the best root-overlay offset by probe score against this aircraft.
    let offsets = candidate_offsets(files, dirs);
    let mut root_offset = String::new();
    let mut root_matched = 0usize;
    let mut root_sample = 0usize;
    for off in &offsets {
        let sample = build_sample(files, off);
        if sample.is_empty() {
            continue;
        }
        let matched = sample.iter().filter(|rel| aircraft_dir.join(rel).exists()).count();
        if matched > root_matched {
            root_matched = matched;
            root_offset = off.clone();
            root_sample = sample.len();
        }
    }

    let mut mappings: Vec<PatchMapping> = Vec::new();

    // Always emit a root mapping: either an aligned subtree or, failing that,
    // the archive root (so plain overlays still install).
    let (root_confidence, root_reason) = if root_matched > 0 {
        (
            confidence_for(root_matched, root_sample.max(1)),
            format!("matchedExisting:{}", root_matched),
        )
    } else {
        (Confidence::Low, "noAnchorDefaultRoot".to_string())
    };
    mappings.push(PatchMapping {
        archive_subpath: root_offset.clone(),
        dest_subpath: String::new(),
        file_count: count_under(files, &root_offset),
        confidence: root_confidence,
        reason: root_reason,
    });
    let covered_prefix = root_offset;

    // 2. Anything not covered by the root overlay: liveries folders map into
    //    `<aircraft>/liveries`; everything else is reported for the user. A file
    //    is "covered" when it sits under the chosen root-overlay prefix, so an
    //    ancestor wrapper folder (fully covered) is correctly skipped.
    let mut unmapped: Vec<String> = Vec::new();
    for group in top_level_dirs(dirs) {
        let group_prefix = format!("{}/", group);
        let uncovered = files
            .iter()
            .filter(|f| f.starts_with(&group_prefix))
            .filter(|f| strip_offset(f, &covered_prefix).is_none())
            .count();
        if uncovered == 0 {
            continue; // fully covered by the root overlay (e.g. a wrapper folder)
        }
        if is_livery_group(&group, files) {
            mappings.push(PatchMapping {
                archive_subpath: group.clone(),
                dest_subpath: "liveries".to_string(),
                file_count: uncovered,
                confidence: Confidence::Medium,
                reason: "liveryFolder".to_string(),
            });
        } else {
            unmapped.push(group);
        }
    }

    // Loose files at the archive root not covered by a non-root overlay.
    if !covered_prefix.is_empty() {
        for f in files {
            if !f.contains('/') {
                unmapped.push(f.clone());
            }
        }
    }

    (mappings, unmapped)
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

/// Turn user-confirmed mappings into overlay `InstallTask`s targeting the
/// chosen aircraft. Each task merges (`should_overwrite = true`) its archive
/// subtree into `<aircraft>/<dest_subpath>`; the existing install engine then
/// handles extraction, progress, atomicity and activity logging.
pub fn build_install_tasks(
    archive_path: &str,
    password: Option<String>,
    xplane_path: &str,
    aircraft_folder: &str,
    mappings: Vec<PatchMappingInput>,
    backup_overwritten: bool,
) -> Result<Vec<InstallTask>> {
    let aircraft_dir = resolve_aircraft_dir_by_name(xplane_path, aircraft_folder)
        .ok_or_else(|| anyhow!("Aircraft folder not found: {}", aircraft_folder))?;

    if mappings.is_empty() {
        return Err(anyhow!("No patch mappings provided"));
    }

    let patch_name = Path::new(archive_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Patch")
        .to_string();

    // One shared backup session directory for every task of this patch install,
    // so a single revert restores all overwritten files at once. Created lazily
    // by the installer only if files are actually overwritten.
    let backup_dir = if backup_overwritten {
        let session = format!("{}_{}", sanitize_component(&patch_name), &Uuid::new_v4().to_string()[..8]);
        Some(
            aircraft_dir
                .join("_xfast_patch_backups")
                .join(session)
                .to_string_lossy()
                .to_string(),
        )
    } else {
        None
    };

    let tasks = mappings
        .into_iter()
        .map(|m| {
            let dest = m.dest_subpath.trim_matches('/').to_string();
            let target = if dest.is_empty() {
                aircraft_dir.clone()
            } else {
                aircraft_dir.join(&dest)
            };
            let dest_label = if dest.is_empty() {
                aircraft_folder.to_string()
            } else {
                format!("{}/{}", aircraft_folder, dest)
            };
            let archive_internal_root = {
                let trimmed = m.archive_subpath.trim_matches('/');
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            };

            InstallTask {
                id: Uuid::new_v4().to_string(),
                addon_type: AddonType::Patch,
                source_path: archive_path.to_string(),
                resolved_source_path: None,
                original_input_path: Some(archive_path.to_string()),
                target_path: target.to_string_lossy().to_string(),
                display_name: format!("{} → {}", patch_name, dest_label),
                conflict_exists: Some(true),
                archive_internal_root,
                extraction_chain: None,
                should_overwrite: true, // overlay/merge into the existing aircraft
                password: password.clone(),
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
                patch_backup: backup_overwritten,
                patch_backup_dir: backup_dir.clone(),
            }
        })
        .collect();

    Ok(tasks)
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupEntry {
    /// Absolute path of the backed-up copy.
    backup: String,
    /// Absolute path the file was overwritten at (restore destination).
    original: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PatchBackupManifest {
    #[serde(default)]
    patch: String,
    #[serde(default)]
    entries: Vec<BackupEntry>,
}

/// Before a patch task merges its staged content into `target`, copy every
/// existing target file that is about to be overwritten into the shared backup
/// session directory and record it in a per-task manifest. Returns how many
/// files were backed up.
pub(crate) fn backup_overwritten_files(
    staged: &Path,
    target: &Path,
    backup_dir: &Path,
    task_id: &str,
) -> Result<usize> {
    // Mirror backups under the aircraft-relative path when possible so that
    // root- and liveries-mappings of the same patch never collide.
    let aircraft_root = backup_dir.parent().and_then(|p| p.parent());
    let files_root = backup_dir.join("files");

    let mut entries: Vec<BackupEntry> = Vec::new();
    for entry in WalkDir::new(staged).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let staged_rel = match entry.path().strip_prefix(staged) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let original = target.join(staged_rel);
        if !original.is_file() {
            continue; // nothing to overwrite -> nothing to back up
        }

        let mirror_rel = aircraft_root
            .and_then(|ar| original.strip_prefix(ar).ok())
            .map(|r| r.to_path_buf())
            .unwrap_or_else(|| staged_rel.to_path_buf());
        let backup_file = files_root.join(&mirror_rel);
        if let Some(parent) = backup_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&original, &backup_file)?;
        entries.push(BackupEntry {
            backup: backup_file.to_string_lossy().to_string(),
            original: original.to_string_lossy().to_string(),
        });
    }

    if entries.is_empty() {
        return Ok(0);
    }

    let count = entries.len();
    let manifest = PatchBackupManifest {
        patch: backup_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string(),
        entries,
    };
    // One manifest per task avoids races when a patch installs in parallel.
    let manifest_path = backup_dir.join(format!("manifest_{}.json", task_id));
    fs::create_dir_all(backup_dir)?;
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    crate::logger::log_info(
        &format!("Patch backup: saved {} overwritten file(s) to {:?}", count, backup_dir),
        Some("patch"),
    );
    Ok(count)
}

/// Restore a patch backup session: copy every backed-up file back to its
/// original location. Reads all per-task manifests in the session directory.
pub fn revert_patch(backup_session_dir: &str) -> Result<usize> {
    let dir = Path::new(backup_session_dir);
    if !dir.is_dir() {
        return Err(anyhow!("Backup session not found: {}", backup_session_dir));
    }

    let mut restored = 0usize;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("manifest_") && name.ends_with(".json")) {
            continue;
        }
        let content = fs::read_to_string(entry.path())?;
        let manifest: PatchBackupManifest = serde_json::from_str(&content)
            .map_err(|e| anyhow!("Invalid patch backup manifest {}: {}", name, e))?;
        for be in manifest.entries {
            let backup = Path::new(&be.backup);
            let original = Path::new(&be.original);
            if !backup.is_file() {
                continue;
            }
            if let Some(parent) = original.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(backup, original)?;
            restored += 1;
        }
    }

    crate::logger::log_info(
        &format!("Patch revert: restored {} file(s) from {}", restored, backup_session_dir),
        Some("patch"),
    );
    Ok(restored)
}

/// Sanitize a string for use as a single path component.
fn sanitize_component(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
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
    Ok(entries
        .into_iter()
        .map(|e| e.replace('\\', "/"))
        .map(|e| e.trim_start_matches("./").to_string())
        .filter(|e| !e.is_empty() && !e.ends_with('/'))
        .collect())
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

fn depth(path: &str) -> usize {
    if path.is_empty() {
        0
    } else {
        path.matches('/').count() + 1
    }
}

/// Candidate overlay roots: the archive root plus any directory (up to a depth
/// cap) that directly contains an aircraft landmark (a `.acf`/`.xfma` file or a
/// strong landmark sub-directory). Scoring later decides which actually aligns.
fn candidate_offsets(files: &[String], dirs: &BTreeSet<String>) -> Vec<String> {
    let mut offsets = vec![String::new()];
    if is_content_root(files, dirs, "") {
        // "" already in offsets
    }
    for d in dirs {
        if depth(d) > MAX_OFFSET_DEPTH {
            continue;
        }
        if is_content_root(files, dirs, d) {
            offsets.push(d.clone());
        }
    }
    offsets
}

/// True if `dir` looks like an aircraft root (overlay target).
fn is_content_root(files: &[String], dirs: &BTreeSet<String>, dir: &str) -> bool {
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{}/", dir)
    };

    // A direct .acf/.xfma child is the strongest possible signal.
    let has_acf = files.iter().any(|f| {
        f.strip_prefix(&prefix).is_some_and(|rest| {
            !rest.contains('/') && {
                let l = rest.to_lowercase();
                l.ends_with(".acf") || l.ends_with(".xfma")
            }
        })
    });
    if has_acf {
        return true;
    }

    // Otherwise: a direct child directory whose name is a strong landmark.
    for child in dirs {
        if let Some(rest) = child.strip_prefix(&prefix) {
            if !rest.is_empty() && !rest.contains('/') {
                let l = rest.to_lowercase();
                if STRONG_LANDMARK_DIRS.contains(&l.as_str()) {
                    return true;
                }
            }
        }
    }
    false
}

/// Build a bounded, distinctive, deterministic sample of paths under `offset`
/// (prefix stripped). Distinctive paths (`.acf`, landmark subtrees) sort first.
fn build_sample(files: &[String], offset: &str) -> Vec<String> {
    let mut rels: Vec<String> = files
        .iter()
        .filter_map(|f| strip_offset(f, offset))
        .map(|s| s.to_string())
        .collect();
    rels.sort_by(|a, b| {
        is_distinctive(b)
            .cmp(&is_distinctive(a))
            .then_with(|| a.cmp(b))
    });
    rels.truncate(MAX_SAMPLE);
    rels
}

fn strip_offset<'a>(path: &'a str, offset: &str) -> Option<&'a str> {
    if offset.is_empty() {
        Some(path)
    } else {
        path.strip_prefix(&format!("{}/", offset))
    }
}

fn is_distinctive(rel: &str) -> bool {
    let l = rel.to_lowercase();
    if l.ends_with(".acf") || l.ends_with(".xfma") {
        return true;
    }
    STRONG_LANDMARK_DIRS
        .iter()
        .any(|d| l.starts_with(&format!("{}/", d)))
}

fn count_under(files: &[String], prefix: &str) -> usize {
    files
        .iter()
        .filter(|f| strip_offset(f, prefix).is_some())
        .count()
}

/// The distinct first path-segments of every directory (archive top level).
fn top_level_dirs(dirs: &BTreeSet<String>) -> Vec<String> {
    let mut tops: BTreeSet<String> = BTreeSet::new();
    for d in dirs {
        let top = d.split('/').next().unwrap_or(d);
        tops.insert(top.to_string());
    }
    tops.into_iter().collect()
}

/// A group is a liveries folder if its leaf name is "liveries", or any file
/// under it matches a known livery pattern.
fn is_livery_group(group: &str, files: &[String]) -> bool {
    let leaf = group.rsplit('/').next().unwrap_or(group).to_lowercase();
    if leaf == "liveries" {
        return true;
    }
    let prefix = format!("{}/", group);
    files
        .iter()
        .filter(|f| f.starts_with(&prefix))
        .any(|f| crate::livery_patterns::check_livery_pattern(f).is_some())
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

/// Resolve an aircraft entry to its absolute folder. `folder_name` is only the
/// immediate folder name, so for nested aircraft we fall back to a bounded
/// recursive search matching name (and `.acf` file when available).
fn resolve_aircraft_dir(xplane_path: &str, ac: &AircraftInfo) -> Option<PathBuf> {
    let base = Path::new(xplane_path).join("Aircraft");
    let direct = base.join(&ac.folder_name);
    if direct.is_dir() {
        return Some(direct);
    }
    find_named_dir(&base, &ac.folder_name, 0, 4)
}

fn resolve_aircraft_dir_by_name(xplane_path: &str, folder_name: &str) -> Option<PathBuf> {
    let base = Path::new(xplane_path).join("Aircraft");
    let direct = base.join(folder_name);
    if direct.is_dir() {
        return Some(direct);
    }
    find_named_dir(&base, folder_name, 0, 4)
}

fn find_named_dir(dir: &Path, name: &str, depth: usize, max_depth: usize) -> Option<PathBuf> {
    if depth > max_depth {
        return None;
    }
    let rd = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in rd.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let path = entry.path();
            if path.file_name().and_then(|s| s.to_str()) == Some(name) {
                return Some(path);
            }
            subdirs.push(path);
        }
    }
    for sub in subdirs {
        if let Some(found) = find_named_dir(&sub, name, depth + 1, max_depth) {
            return Some(found);
        }
    }
    None
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

    #[test]
    fn content_root_detects_acf_and_landmarks() {
        // PW-MOD-style: content lives under a two-level wrapper.
        let (files, dirs) = dirs_of(&[
            "Toliss321 Base Folder/Base Folder/A321.acf",
            "Toliss321 Base Folder/Base Folder/objects/a.obj",
            "Toliss321 Base Folder/Base Folder/systems/s.txt",
            "Liveries/Cool Livery/objects/x.obj",
        ]);
        // The base folder is a content root (has .acf); the wrapper and root are not.
        assert!(is_content_root(&files, &dirs, "Toliss321 Base Folder/Base Folder"));
        assert!(!is_content_root(&files, &dirs, "Toliss321 Base Folder"));
        assert!(!is_content_root(&files, &dirs, ""));
        // Offsets include the base folder so scoring can align it.
        let offsets = candidate_offsets(&files, &dirs);
        assert!(offsets.contains(&"Toliss321 Base Folder/Base Folder".to_string()));
    }

    #[test]
    fn content_root_detects_fmod_only() {
        // FMOD-style: content at root with sound landmark dirs.
        let (files, dirs) = dirs_of(&["sounds/x.wav", "fmod/y.bank"]);
        assert!(is_content_root(&files, &dirs, ""));
    }

    #[test]
    fn livery_group_by_name() {
        let (files, _dirs) = dirs_of(&["Liveries/My Livery/object.obj"]);
        assert!(is_livery_group("Liveries", &files));
        assert!(!is_livery_group("Base Folder", &files));
    }

    #[test]
    fn sample_prefers_distinctive_paths() {
        let files: Vec<String> = vec![
            "readme.txt".to_string(),
            "objects/a.obj".to_string(),
            "A321.acf".to_string(),
        ];
        let sample = build_sample(&files, "");
        // .acf / objects sort ahead of readme.txt
        assert_eq!(sample[0], "A321.acf");
        assert!(sample.iter().position(|s| s == "objects/a.obj").unwrap() < sample.iter().position(|s| s == "readme.txt").unwrap());
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
        let ac = make_aircraft(&[
            "A321.acf",
            "objects/fuselage.obj",
            "systems/hydraulics.txt",
        ]);
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
        let liveries = mappings.iter().find(|m| m.dest_subpath == "liveries").unwrap();
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
}

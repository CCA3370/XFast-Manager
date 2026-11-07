//! Complete file-tree alignment. Paths, never package/folder names, supply evidence.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rayon::prelude::*;
use walkdir::WalkDir;

use super::{AircraftCandidate, Confidence, TargetDetection};

pub(super) struct ProbeTarget {
    pub folder_name: String,
    pub display_name: String,
    pub dir: PathBuf,
}

pub(super) fn path_key(path: &str) -> String {
    let path = path.replace('\\', "/");
    if cfg!(windows) {
        path.to_lowercase()
    } else {
        path
    }
}

fn include_entry(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    !name.eq_ignore_ascii_case("_xfast_patch_backups")
        && !name.eq_ignore_ascii_case("__MACOSX")
        && !name.eq_ignore_ascii_case(".git")
        && !name.eq_ignore_ascii_case(".DS_Store")
}

/// Discover every aircraft, including disabled aircraft and arbitrarily nested
/// collections. Stop descending when an aircraft root is found. Linked aircraft
/// are supported; traversal errors/cycles fail visibly instead of hiding targets.
pub(super) fn discover(xplane_path: &str) -> Result<Vec<ProbeTarget>> {
    let base = Path::new(xplane_path).join("Aircraft");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut walk = WalkDir::new(&base).follow_links(true).into_iter();
    let mut targets = Vec::new();
    while let Some(entry) = walk.next() {
        let entry = entry.context("Cannot inspect the aircraft library")?;
        if !entry.file_type().is_dir() {
            continue;
        }
        if entry.depth() > 0 && !include_entry(&entry) {
            walk.skip_current_dir();
            continue;
        }
        let mut is_aircraft = false;
        for child in fs::read_dir(entry.path())? {
            let child = child?;
            if child.file_type()?.is_file()
                && child.path().extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("acf") || ext.eq_ignore_ascii_case("xfma")
                })
            {
                is_aircraft = true;
            }
        }
        if is_aircraft && entry.depth() > 0 {
            let folder_name = entry
                .path()
                .strip_prefix(&base)?
                .to_string_lossy()
                .replace('\\', "/");
            targets.push(ProbeTarget {
                display_name: folder_name.clone(),
                folder_name,
                dir: entry.path().to_path_buf(),
            });
            walk.skip_current_dir();
        }
    }
    targets.sort_by_key(|target| path_key(&target.folder_name));
    Ok(targets)
}

pub(super) fn file_tree(dir: &Path) -> Result<HashSet<String>> {
    let mut tree = HashSet::new();
    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_entry(include_entry)
    {
        let entry = entry.with_context(|| format!("Cannot inspect aircraft {}", dir.display()))?;
        if entry.file_type().is_file() {
            tree.insert(path_key(&entry.path().strip_prefix(dir)?.to_string_lossy()));
        }
    }
    Ok(tree)
}

/// Every possible wrapper is evaluated, without truncating the archive or
/// imposing a depth/landmark limit. Building the catalogue visits each file once
/// per ancestor rather than scanning all files for each directory.
fn alignments(files: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut result: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        result.entry(String::new()).or_default().push(file.clone());
        for (index, _) in file.match_indices('/') {
            result
                .entry(file[..index].to_string())
                .or_default()
                .push(file[index + 1..].to_string());
        }
    }
    result
}

/// Generic runtime binaries and documentation cannot identify an aircraft.
/// Aircraft-specific plugin scripts/resources DO participate in matching.
fn identity_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    let leaf = lower.rsplit('/').next().unwrap_or(&lower);
    !matches!(
        leaf,
        "win.xpl"
            | "lin.xpl"
            | "mac.xpl"
            | "xlua.xpl"
            | "sasl.xpl"
            | "init.lua"
            | "main.lua"
            | "manifest.json"
            | "thumbs.db"
            | "desktop.ini"
            | "guids.txt"
            | "master bank.bank"
            | "master bank.strings.bank"
    ) && !leaf.starts_with('.')
        && !leaf.starts_with("readme")
        && !leaf.starts_with("license")
}

#[derive(Default)]
struct AlignmentScore {
    identity: f64,
    matched: usize,
}

pub(super) fn rank_candidates(
    files: &[String],
    targets: &[ProbeTarget],
) -> Result<TargetDetection> {
    let trees = targets
        .par_iter()
        .map(|target| file_tree(&target.dir))
        .collect::<Result<Vec<_>>>()?;
    let catalogue = alignments(files);
    let queries: HashSet<String> = catalogue
        .values()
        .flatten()
        .map(|path| path_key(path))
        .collect();
    // Index installed files by the relative suffixes present in the patch. This
    // also finds a patch whose root is itself an aircraft subdirectory, without
    // trying the Cartesian product of every archive and aircraft directory.
    let indexed: Vec<_> = trees
        .par_iter()
        .map(|tree| {
            let mut index: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
            for path in tree {
                for offset in
                    std::iter::once(0).chain(path.match_indices('/').map(|(index, _)| index + 1))
                {
                    let relative = &path[offset..];
                    if queries.contains(relative) {
                        index
                            .entry(relative)
                            .or_default()
                            .push((path[..offset].trim_end_matches('/'), path));
                    }
                }
            }
            index
        })
        .collect();
    let mut frequencies = HashMap::new();
    for tree in &trees {
        for path in tree {
            *frequencies.entry(path.as_str()).or_insert(0usize) += 1;
        }
    }
    let mut scored = Vec::new();
    for (target, index) in targets.iter().zip(&indexed) {
        let mut candidate = AircraftCandidate {
            folder_name: target.folder_name.clone(),
            display_name: target.display_name.clone(),
            matched_count: 0,
            file_count: files.len(),
            best_offset: String::new(),
            best_destination: String::new(),
            ambiguous_mapping: false,
            confidence: Confidence::Low,
            match_score: 0.0,
        };
        let mut best_coverage = 0.0;
        for (offset, paths) in &catalogue {
            let mut destinations: BTreeMap<&str, AlignmentScore> = BTreeMap::new();
            for path in paths {
                let key = path_key(path);
                let Some(matches) = index.get(key.as_str()) else {
                    continue;
                };
                for &(destination, installed_path) in matches {
                    let score = destinations.entry(destination).or_default();
                    score.matched += 1;
                    if identity_path(path) {
                        let weight = if trees.len() == 1 {
                            1.0
                        } else {
                            (trees.len() as f64 / frequencies[installed_path] as f64).ln()
                        };
                        // Prefer preserving relative hierarchy. A flat file can
                        // still identify a unique destination inside a model.
                        score.identity += weight
                            * if destination.is_empty() {
                                1.0
                            } else if path.contains('/') {
                                0.85
                            } else {
                                0.6
                            };
                    }
                }
            }
            for (destination, score) in destinations {
                let coverage = score.matched as f64 / paths.len() as f64;
                let ordering = score
                    .identity
                    .total_cmp(&candidate.match_score)
                    .then_with(|| score.matched.cmp(&candidate.matched_count))
                    .then_with(|| coverage.total_cmp(&best_coverage))
                    .then_with(|| {
                        candidate
                            .best_destination
                            .is_empty()
                            .cmp(&destination.is_empty())
                            .reverse()
                    });
                if ordering.is_gt() {
                    candidate.match_score = score.identity;
                    candidate.matched_count = score.matched;
                    candidate.file_count = paths.len();
                    candidate.best_offset = offset.clone();
                    candidate.best_destination = destination.to_string();
                    candidate.ambiguous_mapping = false;
                    best_coverage = coverage;
                } else if ordering.is_eq() && candidate.matched_count > 0 {
                    candidate.ambiguous_mapping = true;
                }
            }
        }
        let unit = (trees.len() as f64).ln().max(1.0);
        candidate.confidence = if candidate.match_score >= unit * 1.5 {
            Confidence::High
        } else if candidate.match_score > 0.0 {
            Confidence::Medium
        } else {
            Confidence::Low
        };
        scored.push(candidate);
    }
    scored.sort_by(|a, b| {
        b.match_score
            .total_cmp(&a.match_score)
            .then_with(|| b.matched_count.cmp(&a.matched_count))
            .then_with(|| path_key(&a.folder_name).cmp(&path_key(&b.folder_name)))
    });
    let recommended_folder = scored.first().and_then(|top| {
        let runner = scored.get(1).map(|other| other.match_score).unwrap_or(0.0);
        (top.match_score > 0.0 && (runner == 0.0 || top.match_score >= runner * 1.5))
            .then(|| top.folder_name.clone())
    });
    Ok(TargetDetection {
        candidates: scored,
        recommended_folder,
    })
}

/// Report uncovered files at the highest wholly uncovered subtree. This keeps
/// siblings under a partly covered wrapper visible and selectable in the UI.
pub(super) fn uncovered(files: &[String], prefixes: &[String]) -> Vec<String> {
    let (covered, remaining): (Vec<_>, Vec<_>) = files.iter().partition(|file| {
        prefixes
            .iter()
            .any(|prefix| super::mapping_relative_file(file, prefix).is_some())
    });
    let covered_ancestors: HashSet<&str> = covered
        .iter()
        .flat_map(|file| file.match_indices('/').map(|(index, _)| &file[..index]))
        .collect();
    let mut result = BTreeSet::new();
    for file in remaining {
        let entry = file
            .match_indices('/')
            .map(|(index, _)| &file[..index])
            .find(|prefix| !covered_ancestors.contains(prefix))
            .unwrap_or(file);
        result.insert(entry.to_string());
    }
    result.into_iter().collect()
}

//! Aircraft file-tree alignment, with a model and layout fallback for new assets.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rayon::prelude::*;
use walkdir::WalkDir;

use super::{
    derive_dirs, immediate_subdirs, strip_offset, AircraftCandidate, Confidence, TargetDetection,
};

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

// XFast disables an ACF by renaming it to XFMA. Match the variant by name,
// while the install planner preserves the actual disabled destination.
fn aircraft_file_key(path: &str) -> String {
    let mut key = path_key(path);
    if key.to_ascii_lowercase().ends_with(".xfma") {
        key.truncate(key.len() - ".xfma".len());
        key.push_str(".acf");
    }
    key
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
        && !name.starts_with(".xfast-patch-rollback-")
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
    for entry in WalkDir::new(dir).into_iter().filter_entry(include_entry) {
        let entry = entry.with_context(|| format!("Cannot inspect aircraft {}", dir.display()))?;
        if entry.file_type().is_file() {
            tree.insert(aircraft_file_key(
                &entry.path().strip_prefix(dir)?.to_string_lossy(),
            ));
        }
    }
    Ok(tree)
}

/// Every possible wrapper is evaluated, without truncating the archive or
/// imposing a depth/landmark limit. Building the catalogue visits each file once
/// per ancestor rather than scanning all files for each directory.
fn alignments(files: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut result: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files.iter().filter(|file| !is_documentation(file)) {
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

fn is_documentation(path: &str) -> bool {
    let leaf = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    leaf.ends_with(".pdf")
        || leaf.starts_with("readme")
        || leaf.starts_with("license")
        || leaf.starts_with("changelog")
        || leaf.starts_with("manual.")
}

/// A model-labelled wrapper such as `A321 Liveries` is a collection only if
/// it contains a paint folder with an objects subtree. Preserve that paint
/// name and everything below it when removing the wrapper.
pub(super) fn is_livery_wrapper(dir: &str, files: &[String]) -> bool {
    let leaf = dir.rsplit('/').next().unwrap_or(dir);
    if leaf.eq_ignore_ascii_case("liveries") {
        return true;
    }
    leaf.rsplit(|c: char| !c.is_ascii_alphanumeric())
        .find(|word| !word.is_empty())
        .is_some_and(|word| word.eq_ignore_ascii_case("liveries"))
        && files
            .iter()
            .filter_map(|file| strip_offset(file, dir))
            .any(|path| {
                let mut parts = path.split('/');
                parts.next().is_some()
                    && parts
                        .next()
                        .is_some_and(|part| part.eq_ignore_ascii_case("objects"))
                    && parts.next().is_some()
            })
}

/// Align new files by a known aircraft directory, preserving all descendants
/// of that anchor. Repeated anchors may be alternative versions of a patch;
/// let the user choose between them instead of merging them automatically.
pub(super) fn layout_mappings(
    files: &[String],
    dirs: &BTreeSet<String>,
    aircraft_dirs: &[String],
) -> Vec<(String, String)> {
    const ROOTS: &[&str] = &[
        "objects",
        "fmod",
        "airfoils",
        "sounds",
        "cockpit",
        "cockpit_3d",
        "plugins",
        "instruments",
    ];
    let mut liveries = Vec::<&str>::new();
    for dir in dirs {
        if is_livery_wrapper(dir, files)
            && !liveries
                .iter()
                .any(|root| strip_offset(dir, root).is_some())
        {
            liveries.push(dir);
        }
    }
    let mut groups = BTreeMap::<String, Vec<String>>::new();
    let livery_dest = aircraft_dirs
        .iter()
        .find(|name| name.eq_ignore_ascii_case("liveries"))
        .map_or("liveries", String::as_str);
    if !liveries.is_empty() {
        groups.insert(
            livery_dest.to_string(),
            liveries.iter().map(|dir| dir.to_string()).collect(),
        );
    }
    let mut anchors = Vec::<&str>::new();
    for dir in dirs {
        if liveries
            .iter()
            .any(|root| *root == dir || strip_offset(dir, root).is_some())
            || anchors.iter().any(|root| strip_offset(dir, root).is_some())
        {
            continue;
        }
        let leaf = dir.rsplit('/').next().unwrap_or(dir);
        if !ROOTS.iter().any(|root| leaf.eq_ignore_ascii_case(root)) {
            continue;
        }
        let Some(dest) = aircraft_dirs
            .iter()
            .find(|name| leaf.eq_ignore_ascii_case(name))
        else {
            continue;
        };
        // A texture-only objects directory can be a standalone repaint. A
        // model/particle file or an explicit base wrapper establishes its role.
        if leaf.eq_ignore_ascii_case("objects") {
            let base_wrapper = dir.rsplit_once('/').is_some_and(|(parent, _)| {
                parent
                    .rsplit('/')
                    .next()
                    .unwrap_or(parent)
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .any(|word| word.eq_ignore_ascii_case("base"))
            });
            let contains_model =
                files
                    .iter()
                    .filter_map(|file| strip_offset(file, dir))
                    .any(|file| {
                        let lower = file.to_ascii_lowercase();
                        lower.ends_with(".obj") || lower.ends_with(".pss")
                    });
            if !base_wrapper && !contains_model {
                continue;
            }
        }
        anchors.push(dir);
        groups.entry(dest.clone()).or_default().push(dir.clone());
    }
    groups
        .into_iter()
        .filter_map(|(destination, mut sources)| {
            (sources.len() == 1).then(|| (sources.pop().unwrap(), destination))
        })
        .collect()
}

fn names_aircraft_model(tree: &HashSet<String>, sources: &[(String, String)]) -> bool {
    tree.iter()
        .filter_map(|file| {
            if file.contains('/') {
                return None;
            }
            let (name, extension) = file.rsplit_once('.')?;
            extension
                .eq_ignore_ascii_case("acf")
                .then(|| name.split(['_', ' ']).next().unwrap_or(name))
        })
        .any(|identifier| {
            let identifier = identifier.to_ascii_lowercase();
            if identifier.len() < 3
                || !identifier.bytes().any(|c| c.is_ascii_digit())
                || !identifier.bytes().any(|c| c.is_ascii_alphabetic())
            {
                return false;
            }
            sources.iter().any(|(source, _)| {
                source.split('/').any(|part| {
                    let part = part.to_ascii_lowercase();
                    part.match_indices(&identifier).any(|(index, _)| {
                        let after = part.as_bytes().get(index + identifier.len());
                        let before = index.checked_sub(1).and_then(|i| part.as_bytes().get(i));
                        !before.is_some_and(u8::is_ascii_digit)
                            && !after.is_some_and(u8::is_ascii_digit)
                    })
                })
            })
        })
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
        .map(|path| aircraft_file_key(path))
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
                let key = aircraft_file_key(path);
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
    // All-new assets cannot produce an overlap score. Require a model name
    // from a usable archive branch AND an alignable aircraft directory. Equal
    // model copies receive equal scores and still require explicit selection.
    if scored.iter().all(|candidate| candidate.match_score == 0.0) {
        let dirs = derive_dirs(files);
        for ((candidate, target), tree) in scored.iter_mut().zip(targets).zip(&trees) {
            let layout = layout_mappings(files, &dirs, &immediate_subdirs(&target.dir));
            if names_aircraft_model(tree, &layout) {
                candidate.match_score = 1.0;
                candidate.confidence = Confidence::Medium;
            }
        }
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

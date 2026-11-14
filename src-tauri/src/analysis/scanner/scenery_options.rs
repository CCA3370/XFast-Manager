use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::models::{AddonType, DetectedItem, SceneryInstallOption, SceneryOptionFile};
use crate::package_artifacts::{
    is_ignored_package_artifact_archive_path, is_ignored_package_artifact_path,
    normalize_safe_relative_path,
};

/// Reading a whole scenery for a byte comparison would be wasteful, but the
/// navigation files that options replace are small. Anything larger is treated
/// as "unknown" and can never be mistaken for an identical copy.
const MAX_FINGERPRINT_BYTES: u64 = 64 * 1024 * 1024;

const SELECTABLE_GROUP_KEYWORDS: &[&str] = &[
    "option",
    "optional",
    "selectable",
    "selection",
    "variant",
    "alternative",
    "preset",
    "config",
    "replacement",
    "choice",
    "extras",
    "可选",
    "选择",
    "配置",
    "替换",
    "备选",
    "备用",
    "预设",
    "版本",
];

const DEFAULT_BACKUP_KEYWORDS: &[&str] = &[
    "original", "default", "backup", "stock", "vanilla", "restore", "baseline", "原版", "原始",
    "默认", "备份", "还原", "标准",
];

/// Identity of an archive member. CRC32 comes from the archive directory, so
/// comparing two members costs nothing; directory packages compute it on the
/// small subset of navigation files instead.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct FileFingerprint {
    pub size: u64,
    pub crc32: Option<u32>,
}

#[derive(Debug)]
pub(super) struct PackageFile {
    pub path: String,
    pub fingerprint: FileFingerprint,
}

impl PackageFile {
    pub(super) fn new(path: impl Into<String>, size: u64, crc32: Option<u32>) -> Self {
        Self {
            path: path.into(),
            fingerprint: FileFingerprint { size, crc32 },
        }
    }
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_selectable_group(name: &str) -> bool {
    let normalized = normalize_name(name);
    SELECTABLE_GROUP_KEYWORDS
        .iter()
        .any(|keyword| normalized.contains(&normalize_name(keyword)))
}

/// Some packages ship the shipped state as a "restore" copy next to the real
/// alternatives. It is a backup, not a choice the user needs to make.
fn is_default_backup_label(name: &str) -> bool {
    let normalized = normalize_name(name);
    DEFAULT_BACKUP_KEYWORDS.iter().any(|keyword| {
        let keyword = normalize_name(keyword);
        normalized == keyword
            || (normalized.starts_with(&keyword) && normalized.len() <= keyword.len() + 6)
    })
}

fn path_component_name(path: &Path, name: &str) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(name))
}

fn is_nav_file(path: &Path) -> bool {
    path_component_name(path, "apt.dat")
        || path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dsf"))
}

fn is_documentation(path: &Path) -> bool {
    if path_component_name(path, "library.txt") {
        return false;
    }
    let by_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            [
                "txt", "md", "pdf", "rtf", "html", "htm", "doc", "docx", "nfo", "rst",
            ]
            .iter()
            .any(|doc| ext.eq_ignore_ascii_case(doc))
        });
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let by_name = [
        "readme",
        "read me",
        "license",
        "eula",
        "install",
        "changelog",
        "what this is",
        "说明",
        "安装说明",
        "使用说明",
        "自述",
        "备注",
    ]
    .iter()
    .any(|word| stem.contains(word));
    by_extension || by_name
}

fn fingerprints_match(left: FileFingerprint, right: FileFingerprint) -> bool {
    match (left.crc32, right.crc32) {
        (Some(a), Some(b)) => left.size == right.size && a == b,
        _ => false,
    }
}

#[derive(Default)]
struct PackageLayout {
    files: BTreeSet<PathBuf>,
    fingerprints: HashMap<PathBuf, FileFingerprint>,
    directories: BTreeSet<PathBuf>,
    /// Directories that contain an apt.dat or a DSF somewhere below them.
    nav_directories: BTreeSet<PathBuf>,
}

impl PackageLayout {
    fn from_archive(files: impl IntoIterator<Item = PackageFile>) -> Self {
        let mut layout = Self::default();
        for file in files {
            let path = file.path;
            if path.ends_with(['/', '\\']) || is_ignored_package_artifact_archive_path(&path) {
                continue;
            }
            let Some(relative) = normalize_safe_relative_path(&path) else {
                continue;
            };
            layout.insert(relative, file.fingerprint);
        }
        layout
    }

    fn from_directory(dir: &Path) -> Self {
        let mut layout = Self::default();
        for entry in walkdir::WalkDir::new(dir)
            .min_depth(1)
            .max_depth(12)
            .into_iter()
            .filter_entry(|entry| !is_ignored_package_artifact_path(entry.path()))
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let Ok(relative) = entry.path().strip_prefix(dir) else {
                continue;
            };
            let Some(relative) = normalize_safe_relative_path(&relative.to_string_lossy()) else {
                continue;
            };
            let fingerprint = fingerprint_for_local_file(relative.as_path(), entry.path());
            layout.insert(relative, fingerprint);
        }
        layout
    }

    fn insert(&mut self, path: PathBuf, fingerprint: FileFingerprint) {
        if !self.files.insert(path.clone()) {
            return;
        }
        self.fingerprints.insert(path.clone(), fingerprint);
        for ancestor in path.ancestors().skip(1) {
            if ancestor.as_os_str().is_empty() {
                continue;
            }
            self.directories.insert(ancestor.to_path_buf());
            if is_nav_file(&path) {
                self.nav_directories.insert(ancestor.to_path_buf());
            }
        }
    }

    /// Directories that could hold a navigation-data replacement. Real scenery
    /// roots, their wrappers and their own Earth nav data folders are excluded.
    /// Detected roots that only contain navigation data (no models/textures)
    /// are deliberately kept: they are usually option folders, not sceneries.
    fn candidate_roots(
        &self,
        base_roots: &BTreeSet<PathBuf>,
        detected_roots: &BTreeSet<PathBuf>,
    ) -> Vec<PathBuf> {
        self.directories
            .iter()
            .filter(|dir| self.nav_directories.contains(*dir))
            .filter(|dir| !base_roots.contains(*dir))
            .filter(|dir| !base_roots.iter().any(|base| base.starts_with(dir)))
            .filter(|dir| {
                !detected_roots
                    .iter()
                    .any(|root| dir.starts_with(root.join("Earth nav data")))
            })
            .cloned()
            .collect()
    }

    fn fingerprint(&self, root: &Path, relative: &Path) -> Option<FileFingerprint> {
        self.fingerprints.get(&root.join(relative)).copied()
    }
}

fn fingerprint_for_local_file(relative: &Path, full_path: &Path) -> FileFingerprint {
    let size = fs::metadata(full_path).map(|meta| meta.len()).unwrap_or(0);
    let crc32 = (is_nav_file(relative) && size <= MAX_FINGERPRINT_BYTES)
        .then(|| {
            let file = fs::File::open(full_path).ok()?;
            let mut reader = BufReader::new(file);
            let mut hasher = crc32fast::Hasher::new();
            let mut buffer = [0u8; 1024 * 1024];
            loop {
                let read = reader.read(&mut buffer).ok()?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Some(hasher.finalize())
        })
        .flatten();
    FileFingerprint { size, crc32 }
}

struct BaseProfile {
    detected_index: usize,
    root: PathBuf,
    nav_files: BTreeSet<PathBuf>,
    has_assets: bool,
}

struct AcceptedOption {
    root: PathBuf,
    group: PathBuf,
    name: String,
    detected_index: usize,
    mappings: Vec<(PathBuf, PathBuf)>,
    default_backup: bool,
}

fn build_bases(roots: &[Option<PathBuf>], layout: &PackageLayout) -> Vec<BaseProfile> {
    let mut bases = Vec::new();
    for (detected_index, root) in roots.iter().enumerate() {
        let Some(root) = root else { continue };
        let mut nav_files = BTreeSet::new();
        let mut has_assets = false;
        for file in &layout.files {
            let Ok(relative) = file.strip_prefix(root) else {
                continue;
            };
            if relative.starts_with("Earth nav data") && relative.components().count() > 1 {
                nav_files.insert(relative.to_path_buf());
            } else if !is_documentation(relative) {
                has_assets = true;
            }
        }
        if nav_files.is_empty() {
            continue;
        }
        bases.push(BaseProfile {
            detected_index,
            root: root.clone(),
            nav_files,
            has_assets,
        });
    }
    bases
}

struct OptionContents {
    /// (path relative to the option folder, path relative to the base scenery)
    mappings: Vec<(PathBuf, PathBuf)>,
    has_documentation: bool,
    has_dsf: bool,
}

/// Translate every file in a candidate folder into a path inside the base
/// scenery's Earth nav data folder. Both common layouts are supported:
/// `Option/Earth nav data/apt.dat` and a flat `Option/apt.dat` drop-in folder.
fn option_contents(candidate: &Path, layout: &PackageLayout) -> Option<OptionContents> {
    let mut mappings: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    let mut has_documentation = false;
    let mut has_dsf = false;

    for file in layout
        .files
        .iter()
        .filter(|file| file.starts_with(candidate))
    {
        let Ok(relative) = file.strip_prefix(candidate) else {
            continue;
        };
        has_documentation |= is_documentation(relative);
        has_dsf |= relative
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dsf"));

        let destination =
            if relative.starts_with("Earth nav data") && relative.components().count() > 1 {
                Some(relative.to_path_buf())
            } else if is_nav_file(relative) {
                Some(Path::new("Earth nav data").join(relative))
            } else {
                None
            };

        match destination {
            Some(destination) => {
                if mappings
                    .insert(destination, relative.to_path_buf())
                    .is_some()
                {
                    // Two different source files would overwrite the same target.
                    return None;
                }
            }
            None if is_documentation(relative) => {}
            None => return None,
        }
    }

    if mappings.is_empty() {
        return None;
    }
    Some(OptionContents {
        mappings: mappings
            .into_iter()
            .map(|(destination, source)| (source, destination))
            .collect(),
        has_documentation,
        has_dsf,
    })
}

fn group_ancestor(candidate: &Path) -> Option<&Path> {
    candidate.ancestors().find(|dir| {
        dir.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(is_selectable_group)
    })
}

/// A scenery stored at the archive root has an empty root path; its "parent"
/// is the archive root itself for sibling comparisons.
fn base_parent(root: &Path) -> Option<&Path> {
    if root.as_os_str().is_empty() {
        Some(Path::new(""))
    } else {
        root.parent()
    }
}

fn choose_base(
    candidate: &Path,
    bases: &[BaseProfile],
    layout: &PackageLayout,
) -> Option<(usize, Vec<(PathBuf, PathBuf)>)> {
    let contents = option_contents(candidate, layout)?;
    let mappings = contents.mappings;
    let destinations = mappings
        .iter()
        .map(|(_, destination)| destination.clone())
        .collect::<BTreeSet<_>>();
    let in_group = group_ancestor(candidate).is_some();

    let mut nested = Vec::new();
    let mut siblings = Vec::new();
    for (index, base) in bases.iter().enumerate() {
        if !destinations.is_subset(&base.nav_files) {
            continue;
        }
        let is_nested = candidate.starts_with(&base.root) && candidate != base.root;
        let parent = base_parent(&base.root);
        let is_sibling = candidate.parent() == parent;
        let group_is_sibling_of_base =
            in_group && group_ancestor(candidate).and_then(|group| group.parent()) == parent;
        let full_replacement = destinations == base.nav_files;
        let wrapper_name_matches = base.root.file_name().is_some_and(|name| {
            Some(name) == candidate.parent().and_then(|parent| parent.file_name())
        });
        let documented_sibling = is_sibling
            && contents.has_documentation
            && contents.has_dsf
            && base.has_assets
            && wrapper_name_matches;

        let evidence = if is_nested {
            // A folder inside the scenery is only an option when it is clearly
            // labelled as one, or when it is a complete navigation replacement.
            in_group || full_replacement
        } else {
            (is_sibling || group_is_sibling_of_base)
                && base.has_assets
                && (in_group || documented_sibling)
        };
        if !evidence {
            continue;
        }
        if is_nested {
            nested.push(index);
        } else {
            siblings.push(index);
        }
    }

    let chosen = if nested.len() == 1 {
        nested[0]
    } else if nested.is_empty() && siblings.len() == 1 {
        siblings[0]
    } else {
        return None;
    };
    Some((chosen, mappings))
}

pub(super) fn attach_archive_options(
    files: impl IntoIterator<Item = PackageFile>,
    detected: &mut Vec<DetectedItem>,
) {
    let roots = detected
        .iter()
        .map(|item| {
            (item.addon_type == AddonType::Scenery).then(|| {
                item.extraction_chain
                    .as_ref()
                    .and_then(|chain| chain.final_internal_root.as_deref())
                    .or(item.archive_internal_root.as_deref())
                    .and_then(normalize_safe_relative_path)
                    .unwrap_or_default()
            })
        })
        .collect::<Vec<_>>();
    let layout = PackageLayout::from_archive(files);
    attach_options(roots, &layout, detected, None);
}

pub(super) fn attach_directory_options(dir: &Path, detected: &mut Vec<DetectedItem>) {
    let roots = detected
        .iter()
        .map(|item| {
            if item.addon_type != AddonType::Scenery
                || item.archive_internal_root.is_some()
                || item.extraction_chain.is_some()
            {
                return None;
            }
            Path::new(&item.path).strip_prefix(dir).ok().map(|root| {
                normalize_safe_relative_path(&root.to_string_lossy()).unwrap_or_default()
            })
        })
        .collect::<Vec<_>>();
    if roots.iter().flatten().count() == 0 {
        return;
    }
    let layout = PackageLayout::from_directory(dir);
    attach_options(roots, &layout, detected, Some(dir));
}

fn attach_options(
    roots: Vec<Option<PathBuf>>,
    layout: &PackageLayout,
    detected: &mut Vec<DetectedItem>,
    package_dir: Option<&Path>,
) {
    let detected_roots = roots.iter().flatten().cloned().collect::<BTreeSet<_>>();
    let bases = build_bases(&roots, layout);
    if bases.is_empty() {
        return;
    }
    let mut base_roots = bases
        .iter()
        .filter(|base| base.has_assets)
        .map(|base| base.root.clone())
        .collect::<BTreeSet<_>>();
    if base_roots.is_empty() {
        // Gateway sceneries can consist of navigation data alone.
        base_roots = bases.iter().map(|base| base.root.clone()).collect();
    }

    let mut accepted = Vec::new();
    let mut discarded_roots = BTreeSet::new();
    for candidate in layout.candidate_roots(&base_roots, &detected_roots) {
        let Some((base_index, mappings)) = choose_base(&candidate, &bases, layout) else {
            continue;
        };
        let base = &bases[base_index];

        // An option that would not change a single file is the shipped state.
        let identical_to_base = mappings.iter().all(|(source, destination)| {
            let Some(option) = layout.fingerprint(&candidate, source) else {
                return false;
            };
            let Some(base_file) = layout.fingerprint(&base.root, destination) else {
                return false;
            };
            fingerprints_match(option, base_file)
        });
        if identical_to_base {
            // It is still a replacement folder, not an independent scenery.
            discarded_roots.insert(candidate);
            continue;
        }

        let destinations = mappings
            .iter()
            .map(|(_, destination)| destination.clone())
            .collect::<BTreeSet<_>>();
        let full_replacement = destinations == base.nav_files;
        let Some(name) = candidate
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
        else {
            continue;
        };
        accepted.push(AcceptedOption {
            group: candidate.parent().unwrap_or(Path::new("")).to_path_buf(),
            name: name.clone(),
            default_backup: is_default_backup_label(&name) && full_replacement,
            root: candidate,
            detected_index: base.detected_index,
            mappings,
        });
    }

    // "Original"/"Default" folders are backups of the state already shipped.
    // Only hide them when real alternatives were found in the same folder.
    let mut group_sizes: HashMap<PathBuf, usize> = HashMap::new();
    for option in &accepted {
        *group_sizes.entry(option.group.clone()).or_default() += 1;
    }
    accepted.retain(|option| {
        let hide =
            option.default_backup && group_sizes.get(&option.group).copied().unwrap_or(0) > 1;
        if hide {
            discarded_roots.insert(option.root.clone());
        }
        !hide
    });

    let indices = roots
        .iter()
        .enumerate()
        .filter_map(|(index, root)| Some((root.as_deref()?, index)))
        .collect::<HashMap<_, _>>();
    let mut option_indices = BTreeSet::new();
    for root in &discarded_roots {
        if let Some(&index) = indices.get(root.as_path()) {
            option_indices.insert(index);
        }
    }
    for mut option in accepted {
        option.mappings.sort_by(|a, b| a.1.cmp(&b.1));
        if let Some(&index) = indices.get(option.root.as_path()) {
            option_indices.insert(index);
        }
        let root = if let Some(dir) = package_dir {
            dir.join(&option.root).to_string_lossy().replace('\\', "/")
        } else {
            option.root.to_string_lossy().replace('\\', "/")
        };
        let files = option
            .mappings
            .into_iter()
            .map(|(source, destination)| SceneryOptionFile {
                source: source.to_string_lossy().replace('\\', "/"),
                destination: destination.to_string_lossy().replace('\\', "/"),
            })
            .collect();
        detected[option.detected_index]
            .scenery_options
            .push(SceneryInstallOption {
                root,
                name: option.name,
                files,
            });
    }

    for index in option_indices.into_iter().rev() {
        detected.remove(index);
    }
    for item in detected {
        item.scenery_options.sort_by(|a, b| a.root.cmp(&b.root));
    }
}

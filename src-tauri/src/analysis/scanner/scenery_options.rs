use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::models::{AddonType, DetectedItem, SceneryInstallOption};
use crate::package_artifacts::{
    is_ignored_package_artifact_archive_path, is_ignored_package_artifact_path,
    normalize_safe_relative_path,
};

#[derive(Default)]
struct SceneryContents {
    nav_files: BTreeSet<PathBuf>,
    has_dsf: bool,
    has_assets: bool,
    has_other_files: bool,
}

impl SceneryContents {
    fn record(&mut self, path: &Path) {
        if path.starts_with("Earth nav data") && path != Path::new("Earth nav data") {
            self.has_dsf |= path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dsf"));
            self.nav_files.insert(path.to_path_buf());
        } else if path.components().count() == 1
            && path.extension().is_some_and(|ext| {
                ["txt", "md", "pdf", "rtf", "html"]
                    .iter()
                    .any(|doc| ext.eq_ignore_ascii_case(doc))
            })
            && !path
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("library.txt"))
        {
            // Package instructions and licenses do not become scenery replacements.
        } else {
            self.has_other_files = true;
            self.has_assets |= path.components().count() > 1
                && path.iter().next().is_some_and(|dir| {
                    [
                        "objects",
                        "ortho",
                        "orthophotos",
                        "terrain",
                        "facades",
                        "polygons",
                    ]
                    .iter()
                    .any(|asset_dir| dir.eq_ignore_ascii_case(asset_dir))
                });
        }
    }
}

pub(super) fn attach_archive_options(
    files: impl IntoIterator<Item = impl AsRef<str>>,
    detected: &mut Vec<DetectedItem>,
) {
    let roots = detected
        .iter()
        .map(|item| {
            (item.addon_type == AddonType::Scenery)
                .then(|| normalize_safe_relative_path(item.archive_internal_root.as_deref()?))
                .flatten()
        })
        .collect::<Vec<_>>();
    attach_options(
        roots,
        files.into_iter().filter_map(|file| {
            let file = file.as_ref();
            if file.ends_with(['/', '\\']) || is_ignored_package_artifact_archive_path(file) {
                return None;
            }
            normalize_safe_relative_path(file)
        }),
        detected,
    );
}

pub(super) fn attach_directory_options(dir: &Path, detected: &mut Vec<DetectedItem>) {
    let roots = detected
        .iter()
        .map(|item| {
            (item.addon_type == AddonType::Scenery)
                .then(|| {
                    Path::new(&item.path)
                        .strip_prefix(dir)
                        .ok()
                        .map(Path::to_path_buf)
                })
                .flatten()
        })
        .collect::<Vec<_>>();
    if roots.iter().flatten().count() < 2 {
        return;
    }

    let files = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|entry| !is_ignored_package_artifact_path(entry.path()))
        .filter_map(Result::ok)
        .filter(|entry| !entry.file_type().is_dir())
        .filter_map(|entry| entry.path().strip_prefix(dir).ok().map(Path::to_path_buf));
    attach_options(roots, files, detected);
}

fn attach_options(
    roots: Vec<Option<PathBuf>>,
    files: impl IntoIterator<Item = PathBuf>,
    detected: &mut Vec<DetectedItem>,
) {
    let mut contents: HashMap<_, SceneryContents> = roots
        .iter()
        .flatten()
        .map(|root| (root.as_path(), SceneryContents::default()))
        .collect();
    if contents.len() < 2 {
        return;
    }

    for file in files {
        if let Some((root, scenery)) = file
            .ancestors()
            .skip(1)
            .find_map(|root| contents.get_key_value(root).map(|(key, _)| *key))
            .and_then(|root| contents.get_mut(root).map(|scenery| (root, scenery)))
        {
            scenery.record(file.strip_prefix(root).expect("ancestor is a prefix"));
        }
    }

    let indices: HashMap<_, _> = roots
        .iter()
        .enumerate()
        .filter_map(|(i, root)| Some((root.as_deref()?, i)))
        .collect();
    let mut option_indices = BTreeSet::new();
    for (&root, option) in &contents {
        if option.has_other_files || option.nav_files.is_empty() {
            continue;
        }
        let Some(parent) = root.parent() else {
            continue;
        };
        let group = parent.file_name().is_some_and(|name| {
            ["option", "options", "optional", "variants"]
                .iter()
                .any(|group| name.eq_ignore_ascii_case(group))
        });
        let base_parent = if group {
            parent.parent().unwrap_or(parent)
        } else {
            parent
        };

        // An apt.dat alone matches almost every airport. Require a matching DSF
        // or an explicitly grouped option, and never guess between possible bases.
        let mut bases = contents.iter().filter(|(base_root, base)| {
            base.has_assets
                && base_root.parent() == Some(base_parent)
                && (option.has_dsf || group)
                && option.nav_files.is_subset(&base.nav_files)
        });
        let Some((&base, _)) = bases.next() else {
            continue;
        };
        if bases.next().is_some() {
            continue;
        }
        let index = indices[root];
        let directory_source = detected[index].archive_internal_root.is_none();
        let option_root = if directory_source {
            detected[index].path.clone()
        } else {
            root.to_string_lossy().replace('\\', "/")
        };
        let Some(name) = root.file_name() else {
            continue;
        };
        detected[indices[base]]
            .scenery_options
            .push(SceneryInstallOption {
                root: option_root,
                name: name.to_string_lossy().into_owned(),
                files: option
                    .nav_files
                    .iter()
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
                    .collect(),
            });
        option_indices.insert(index);
    }

    for index in option_indices.into_iter().rev() {
        detected.remove(index);
    }
    for item in detected {
        item.scenery_options.sort_by(|a, b| a.root.cmp(&b.root));
    }
}

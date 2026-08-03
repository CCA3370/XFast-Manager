use std::path::{Component, Path, PathBuf};

const IGNORED_PACKAGE_ARTIFACT_NAMES: [&str; 4] =
    ["__MACOSX", ".DS_Store", "Thumbs.db", "desktop.ini"];

fn is_ignored_component(name: &str) -> bool {
    IGNORED_PACKAGE_ARTIFACT_NAMES
        .iter()
        .any(|ignored| name.eq_ignore_ascii_case(ignored))
}

/// Returns whether a filesystem path contains operating-system metadata that is
/// unrelated to the add-on being installed.
pub fn is_ignored_package_artifact_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(is_ignored_component)
    })
}

/// Returns whether a path stored in an archive points to operating-system
/// metadata. Archive paths can use either slash convention on every platform.
pub fn is_ignored_package_artifact_archive_path(path: &str) -> bool {
    path.split(['/', '\\'])
        .filter(|component| !component.is_empty())
        .any(is_ignored_component)
}

/// Normalizes an archive-style path while rejecting absolute paths and parent
/// traversal, so callers can safely join it to an installation root.
pub fn normalize_safe_relative_path(path: &str) -> Option<PathBuf> {
    let normalized = path.replace('\\', "/");
    let mut result = PathBuf::new();

    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(value) => result.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => return None,
        }
    }

    (!result.as_os_str().is_empty()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_metadata_at_any_depth_and_with_either_separator() {
        for path in [
            ".DS_Store",
            "Aircraft/__MACOSX/._plane.acf",
            "Aircraft\\Thumbs.db",
            "Scenery/Objects/desktop.ini",
            "scenery/objects/DESKTOP.INI",
        ] {
            assert!(is_ignored_package_artifact_archive_path(path), "{path}");
        }
    }

    #[test]
    fn keeps_similarly_named_addon_files() {
        for path in [
            "Aircraft/DS_Store.txt",
            "Aircraft/__MACOSX_readme.txt",
            "Scenery/Thumbs.db.backup",
            "Scenery/my-desktop.ini.txt",
        ] {
            assert!(!is_ignored_package_artifact_archive_path(path), "{path}");
        }
    }

    #[test]
    fn recognizes_filesystem_metadata_paths() {
        assert!(is_ignored_package_artifact_path(Path::new(
            "Aircraft/__MACOSX/plane.acf"
        )));
        assert!(!is_ignored_package_artifact_path(Path::new(
            "Aircraft/plane.acf"
        )));
    }

    #[test]
    fn normalizes_only_safe_relative_paths() {
        assert_eq!(
            normalize_safe_relative_path("Aircraft\\Test/plane.acf"),
            Some(PathBuf::from("Aircraft/Test/plane.acf"))
        );
        assert!(normalize_safe_relative_path("../outside.txt").is_none());
        assert!(normalize_safe_relative_path("/absolute.txt").is_none());
        assert!(normalize_safe_relative_path(".").is_none());
    }
}

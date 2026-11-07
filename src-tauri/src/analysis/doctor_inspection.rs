//! Read-only, bounded inspection. Findings describe observations, never guessed causes.
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "doctor_plugins.rs"]
mod plugins;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionReport {
    pub scenery_ini_exists: bool,
    pub missing_scenery_paths: Vec<String>,
    pub duplicate_scenery_paths: Vec<String>,
    pub broken_scenery_links: Vec<String>,
    pub empty_navdata_files: Vec<String>,
    pub scan_errors: Vec<String>,
    plugins: Option<plugins::PluginInspection>,
    pub shader_cache_present: bool,
    pub preferences_present: bool,
    pub log_age_days: Option<u64>,
}

type ShortcutResolver = fn(&Path) -> Option<std::path::PathBuf>;

fn scenery_target(
    root: &Path,
    path: &str,
    _resolve_shortcut: ShortcutResolver,
) -> std::path::PathBuf {
    let normalized = path.replace('\\', "/");
    let target = root.join(normalized.trim_end_matches('/'));
    #[cfg(windows)]
    if !target.exists() {
        let shortcut = target.with_file_name(format!(
            "{}.lnk",
            target.file_name().unwrap_or_default().to_string_lossy()
        ));
        if let Some(resolved) = _resolve_shortcut(&shortcut) {
            return resolved;
        }
    }
    target
}

fn inspect_scenery(
    root: &Path,
    report: &mut InspectionReport,
    _resolve_shortcut: ShortcutResolver,
) -> Option<Vec<PathBuf>> {
    let ini = root.join("Custom Scenery/scenery_packs.ini");
    let mut enabled_packs = None;
    match fs::metadata(&ini) {
        Ok(meta) if meta.is_file() => {
            report.scenery_ini_exists = true;
            match fs::read_to_string(&ini) {
                Ok(body) => {
                    let mut seen = HashMap::new();
                    let mut packs = Vec::new();
                    for (index, line) in body.lines().enumerate() {
                        let Some(path) = line.trim().strip_prefix("SCENERY_PACK ") else {
                            continue; // Disabled scenery is intentionally not loaded.
                        };
                        let path = path.trim();
                        if path.is_empty() || path == "*GLOBAL_AIRPORTS*" {
                            continue;
                        }
                        let target = scenery_target(root, path, _resolve_shortcut);
                        let mut key = target
                            .canonicalize()
                            .unwrap_or_else(|_| target.clone())
                            .to_string_lossy()
                            .replace('\\', "/");
                        if cfg!(windows) {
                            key.make_ascii_lowercase();
                        }
                        let duplicate = seen.insert(key, index + 1);
                        if let Some(first) = duplicate {
                            report
                                .duplicate_scenery_paths
                                .push(format!("lines {first}, {}: {path}", index + 1));
                        }
                        match fs::metadata(&target) {
                            Ok(meta) if meta.is_dir() => {
                                if duplicate.is_none() {
                                    packs.push(target);
                                }
                            }
                            Ok(_) => report
                                .missing_scenery_paths
                                .push(format!("line {}: {path} (not a directory)", index + 1)),
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => report
                                .missing_scenery_paths
                                .push(format!("line {}: {path}", index + 1)),
                            Err(error) => report.scan_errors.push(format!("{path}: {error}")),
                        }
                    }
                    enabled_packs = Some(packs);
                }
                Err(error) => report
                    .scan_errors
                    .push(format!("Custom Scenery/scenery_packs.ini: {error}")),
            }
        }
        Ok(_) => report
            .scan_errors
            .push("Custom Scenery/scenery_packs.ini: not a file".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => report
            .scan_errors
            .push(format!("Custom Scenery/scenery_packs.ini: {error}")),
    }
    match fs::read_dir(root.join("Custom Scenery")) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        let is_link = entry.file_type().is_ok_and(|kind| kind.is_symlink());
                        if is_link {
                            match fs::metadata(&path) {
                                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                                    report.broken_scenery_links.push(path.display().to_string())
                                }
                                Err(error) => report
                                    .scan_errors
                                    .push(format!("{}: {error}", path.display())),
                                _ => {}
                            }
                        }
                        #[cfg(windows)]
                        if path
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
                        {
                            match _resolve_shortcut(&path) {
                                Some(target) if target.is_dir() => {}
                                _ => report.broken_scenery_links.push(path.display().to_string()),
                            }
                        }
                    }
                    Err(error) => report.scan_errors.push(format!("Custom Scenery: {error}")),
                }
            }
        }
        Err(error) => report.scan_errors.push(format!("Custom Scenery: {error}")),
    }
    enabled_packs
}

pub fn inspect_installation(
    root: &Path,
    full: bool,
    resolve_shortcut: ShortcutResolver,
) -> InspectionReport {
    let mut report = InspectionReport::default();
    let scenery = inspect_scenery(root, &mut report, resolve_shortcut);
    for name in [
        "earth_nav.dat",
        "earth_fix.dat",
        "earth_awy.dat",
        "earth_hold.dat",
        "earth_mora.dat",
    ] {
        let path = root.join("Custom Data").join(name);
        match fs::metadata(&path) {
            Ok(meta) if meta.is_file() && meta.len() == 0 => report
                .empty_navdata_files
                .push(format!("Custom Data/{name}")),
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => report
                .scan_errors
                .push(format!("Custom Data/{name}: {error}")),
            _ => {}
        }
    }
    report.shader_cache_present = root.join("Output/shadercache").is_dir();
    report.preferences_present = root.join("Output/preferences").is_dir();
    report.log_age_days = fs::metadata(root.join("Log.txt"))
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.elapsed().ok())
        .map(|age| age.as_secs() / 86_400);
    if full {
        report.plugins = Some(plugins::inspect(root, scenery.as_deref()));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn dotted_scenery_folder_names_keep_the_full_shortcut_name() {
        let temp = tempfile::tempdir().unwrap();
        fn resolver(path: &Path) -> Option<std::path::PathBuf> {
            assert_eq!(path.file_name().unwrap(), "Airport.v2.lnk");
            Some(std::path::PathBuf::from("D:/External/Airport"))
        }
        assert_eq!(
            scenery_target(temp.path(), "Custom Scenery/Airport.v2/", resolver),
            std::path::PathBuf::from("D:/External/Airport")
        );
    }

    #[test]
    fn detects_duplicate_and_missing_enabled_scenery_but_ignores_disabled_entries() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("Custom Scenery/Present")).unwrap();
        fs::write(temp.path().join("Custom Scenery/scenery_packs.ini"),
            "I\n1000 Version\nSCENERY\nSCENERY_PACK Custom Scenery/Present/\nSCENERY_PACK Custom Scenery/Present/\nSCENERY_PACK Custom Scenery/Missing/\nSCENERY_PACK_DISABLED Custom Scenery/Disabled/\nSCENERY_PACK *GLOBAL_AIRPORTS*\n").unwrap();
        let report = inspect_installation(temp.path(), false, |_| None);
        assert_eq!(report.duplicate_scenery_paths.len(), 1);
        assert_eq!(
            report.missing_scenery_paths,
            vec!["line 6: Custom Scenery/Missing/"]
        );
    }

    #[test]
    fn finds_empty_navdata_and_corrupt_host_plugin_without_reading_other_platforms() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("Custom Data")).unwrap();
        fs::write(temp.path().join("Custom Data/earth_nav.dat"), "").unwrap();
        let plugin = temp.path().join("Resources/plugins/Test/64");
        fs::create_dir_all(&plugin).unwrap();
        for name in ["win.xpl", "mac.xpl", "lin.xpl"] {
            fs::write(plugin.join(name), "").unwrap();
        }
        let report = inspect_installation(temp.path(), true, |_| None);
        assert_eq!(report.empty_navdata_files.len(), 1);
        let plugins = report.plugins.unwrap();
        assert_eq!(plugins.checked, 1);
        assert_eq!(plugins.problems.len(), 1);
        assert!(inspect_installation(temp.path(), false, |_| None)
            .plugins
            .is_none());
    }

    #[cfg(windows)]
    #[test]
    fn skips_legacy_32_bit_variants_and_prefers_the_modern_entry() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        for relative in [
            "Resources/plugins/Modern/32/win.xpl",
            "Resources/plugins/Modern/64/win.xpl",
            "Resources/plugins/Modern/win_x64/Modern.xpl",
            "Resources/plugins/Legacy/32/win.xpl",
            "Resources/plugins/Legacy/64/win.xpl",
        ] {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut data = vec![0; 64 + 24 + 112];
            data[..2].copy_from_slice(b"MZ");
            data[60..64].copy_from_slice(&64u32.to_le_bytes());
            data[64..68].copy_from_slice(b"PE\0\0");
            data[68..70].copy_from_slice(&0x8664u16.to_le_bytes());
            data[84..86].copy_from_slice(&112u16.to_le_bytes());
            data[88..90].copy_from_slice(&0x20bu16.to_le_bytes());
            fs::write(
                path,
                if relative.contains("/32/") || relative.contains("Modern/64/") {
                    &[][..]
                } else {
                    &data
                },
            )
            .unwrap();
        }
        let report = inspect_installation(root, true, |_| None);
        let plugins = report.plugins.unwrap();
        assert_eq!(plugins.checked, 2);
        assert!(plugins.problems.is_empty(), "{:?}", plugins.problems);
    }

    #[test]
    fn checks_aircraft_and_registered_scenery_entries_only() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let (_, host) = plugins::test_layout();
        for path in [
            "Aircraft/Vendor/Active/plugins/Test/64",
            "Aircraft/Vendor/Disabled/plugins/Test/64",
            "Aircraft/Vendor/Active/objects/backup/plugins/Test/64",
            "Custom Scenery/Enabled/plugins/Test/64",
            "Custom Scenery/Disabled/plugins/Test/64",
            "Custom Scenery/Unregistered/plugins/Test/64",
        ] {
            let path = root.join(path);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join(host), "").unwrap();
        }
        fs::write(root.join("Aircraft/Vendor/Active/Active.acf"), "").unwrap();
        fs::write(root.join("Aircraft/Vendor/Disabled/Disabled.xfma"), "").unwrap();
        fs::write(
            root.join("Custom Scenery/scenery_packs.ini"),
            "I\n1000 Version\nSCENERY\nSCENERY_PACK Custom Scenery/Enabled/\nSCENERY_PACK_DISABLED Custom Scenery/Disabled/\n",
        )
        .unwrap();
        let plugins = inspect_installation(root, true, |_| None).plugins.unwrap();
        assert_eq!(plugins.checked, 2);
        assert_eq!(plugins.problems.len(), 2);
        let paths = plugins.problems.join("\n").replace('\\', "/");
        assert!(paths.contains("Aircraft/Vendor/Active/plugins/Test/64"));
        assert!(paths.contains("Custom Scenery/Enabled/plugins/Test/64"));
        assert!(plugins.errors.is_empty());
    }
}

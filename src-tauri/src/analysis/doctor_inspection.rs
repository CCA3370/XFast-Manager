//! Read-only, bounded inspection. Findings describe observations, never guessed causes.
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[path = "doctor_plugins.rs"]
mod plugins;

const SCENERY_INI: &str = "Custom Scenery/scenery_packs.ini";
const SCENERY_INI_BYTE_LIMIT: u64 = 4 * 1024 * 1024;
const SCENERY_LINK_ENTRY_LIMIT: usize = 60_000;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SceneryRegistration {
    #[default]
    Missing,
    Readable,
    Unavailable,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionReport {
    pub scenery_registration: SceneryRegistration,
    pub scenery_registration_error: Option<String>,
    pub scenery_path_errors: Vec<String>,
    pub scenery_link_errors: Vec<String>,
    pub invalid_scenery_entries: Vec<String>,
    pub missing_scenery_paths: Vec<String>,
    pub duplicate_scenery_paths: Vec<String>,
    pub broken_scenery_links: Vec<String>,
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

fn read_scenery_registration(root: &Path) -> io::Result<Option<String>> {
    let ini = root.join(SCENERY_INI);
    match fs::symlink_metadata(&ini) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    }
    if !fs::metadata(&ini)?.is_file() {
        return Err(io::Error::other("not a regular file"));
    }
    let mut body = String::new();
    fs::File::open(ini)?
        .take(SCENERY_INI_BYTE_LIMIT + 1)
        .read_to_string(&mut body)?;
    if body.len() as u64 > SCENERY_INI_BYTE_LIMIT {
        return Err(io::Error::other(
            "scenery registration exceeds the 4 MiB inspection limit",
        ));
    }
    Ok(Some(body))
}

fn inspect_scenery_registration(
    root: &Path,
    report: &mut InspectionReport,
    resolve_shortcut: ShortcutResolver,
) -> Option<Vec<PathBuf>> {
    let body = match read_scenery_registration(root) {
        Ok(Some(body)) => body,
        Ok(None) => return None,
        Err(error) => {
            report.scenery_registration = SceneryRegistration::Unavailable;
            report.scenery_registration_error = Some(format!("{SCENERY_INI}: {error}"));
            return None;
        }
    };
    report.scenery_registration = SceneryRegistration::Readable;
    let mut seen = HashMap::new();
    let mut packs = Vec::new();
    for (index, line) in body.trim_start_matches('\u{feff}').lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim();
        if line == "SCENERY_PACK" {
            report
                .invalid_scenery_entries
                .push(format!("line {line_number}: {line}"));
            continue;
        }
        let Some(path) = line
            .strip_prefix("SCENERY_PACK")
            .filter(|rest| rest.starts_with(char::is_whitespace))
        else {
            continue;
        };
        let path = path.trim();
        let target =
            (path != "*GLOBAL_AIRPORTS*").then(|| scenery_target(root, path, resolve_shortcut));
        let key = match &target {
            Some(target) => {
                let mut key = target
                    .canonicalize()
                    .unwrap_or_else(|_| target.clone())
                    .to_string_lossy()
                    .replace('\\', "/");
                if cfg!(windows) {
                    key.make_ascii_lowercase();
                }
                key
            }
            None => path.to_owned(),
        };
        let first = *seen.entry(key).or_insert(line_number);
        if first != line_number {
            report
                .duplicate_scenery_paths
                .push(format!("lines {first}, {line_number}: {path}"));
            continue;
        }
        let Some(target) = target else { continue };
        match fs::metadata(&target) {
            Ok(meta) if meta.is_dir() => packs.push(target),
            Ok(_) => report
                .missing_scenery_paths
                .push(format!("line {line_number}: {path} (not a directory)")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                report
                    .missing_scenery_paths
                    .push(format!("line {line_number}: {path}"));
            }
            Err(error) => report
                .scenery_path_errors
                .push(format!("line {line_number}: {path}: {error}")),
        }
    }
    Some(packs)
}

fn inspect_scenery_links(
    root: &Path,
    report: &mut InspectionReport,
    _resolve_shortcut: ShortcutResolver,
) {
    let entries = match fs::read_dir(root.join("Custom Scenery")) {
        Ok(entries) => entries,
        Err(error) => {
            report
                .scenery_link_errors
                .push(format!("Custom Scenery: {error}"));
            return;
        }
    };
    for (index, entry) in entries.enumerate() {
        if index == SCENERY_LINK_ENTRY_LIMIT {
            report.scenery_link_errors.push(format!(
                "Custom Scenery: reached the {SCENERY_LINK_ENTRY_LIMIT} entry inspection limit"
            ));
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report
                    .scenery_link_errors
                    .push(format!("Custom Scenery: {error}"));
                continue;
            }
        };
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_symlink() => match fs::metadata(&path) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    report.broken_scenery_links.push(path.display().to_string());
                }
                Err(error) => report
                    .scenery_link_errors
                    .push(format!("{}: {error}", path.display())),
                Ok(_) => {}
            },
            Ok(_) => {}
            Err(error) => report
                .scenery_link_errors
                .push(format!("{}: {error}", path.display())),
        }
        #[cfg(windows)]
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
        {
            match _resolve_shortcut(&path) {
                Some(target) => check_link_target(&path, &target, report),
                None => report.broken_scenery_links.push(path.display().to_string()),
            }
        }
    }
    report.broken_scenery_links.sort();
    report.broken_scenery_links.dedup();
}

#[cfg(windows)]
fn check_link_target(link: &Path, target: &Path, report: &mut InspectionReport) {
    match fs::metadata(target) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => report
            .broken_scenery_links
            .push(format!("{} (target is not a directory)", link.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            report.broken_scenery_links.push(link.display().to_string());
        }
        Err(error) => report
            .scenery_link_errors
            .push(format!("{}: {error}", link.display())),
    }
}

pub fn inspect_installation(
    root: &Path,
    full: bool,
    resolve_shortcut: ShortcutResolver,
) -> InspectionReport {
    let mut report = InspectionReport::default();
    let scenery = inspect_scenery_registration(root, &mut report, resolve_shortcut);
    inspect_scenery_links(root, &mut report, resolve_shortcut);
    report.shader_cache_present = root.join("Output/shadercache").is_dir();
    report.preferences_present = root.join("Output/preferences").is_dir();
    report.log_age_days = fs::metadata(root.join("Log.txt"))
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.elapsed().ok())
        .map(|age| age.as_secs() / 86_400);
    if full {
        let mut plugins = plugins::inspect(root, scenery.as_deref());
        plugins
            .errors
            .extend(report.scenery_path_errors.iter().cloned());
        report.plugins = Some(plugins);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_missing_registration_from_invalid_utf8_and_oversized_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("Custom Scenery")).unwrap();
        let report = inspect_installation(root, false, |_| None);
        assert!(matches!(
            report.scenery_registration,
            SceneryRegistration::Missing
        ));
        assert!(report.scenery_registration_error.is_none());
        assert!(report.scenery_link_errors.is_empty());

        let path = root.join(SCENERY_INI);
        fs::write(&path, [0xff]).unwrap();
        let report = inspect_installation(root, false, |_| None);
        assert!(matches!(
            report.scenery_registration,
            SceneryRegistration::Unavailable
        ));
        assert!(report
            .scenery_registration_error
            .unwrap()
            .contains(SCENERY_INI));

        let file = fs::File::create(path).unwrap();
        file.set_len(SCENERY_INI_BYTE_LIMIT + 1).unwrap();
        assert!(read_scenery_registration(root)
            .unwrap_err()
            .to_string()
            .contains("limit"));
    }

    #[test]
    fn checks_bom_and_tab_separated_registrations_with_original_line_numbers() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("Custom Scenery/Present")).unwrap();
        fs::write(
            root.join(SCENERY_INI),
            "\u{feff}SCENERY_PACK\tCustom Scenery/Present/\r\nSCENERY_PACK Custom Scenery/Present\nSCENERY_PACK Custom Scenery/Present\nSCENERY_PACK  \n",
        )
        .unwrap();
        let report = inspect_installation(root, false, |_| None);
        assert_eq!(
            report.duplicate_scenery_paths,
            [
                "lines 1, 2: Custom Scenery/Present",
                "lines 1, 3: Custom Scenery/Present"
            ]
        );
        assert_eq!(report.invalid_scenery_entries, ["line 4: SCENERY_PACK"]);
    }

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
    fn finds_corrupt_host_plugin_without_reading_other_platforms() {
        let temp = tempfile::tempdir().unwrap();
        let plugin = temp.path().join("Resources/plugins/Test/64");
        fs::create_dir_all(&plugin).unwrap();
        for name in ["win.xpl", "mac.xpl", "lin.xpl"] {
            fs::write(plugin.join(name), "").unwrap();
        }
        let report = inspect_installation(temp.path(), true, |_| None);
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

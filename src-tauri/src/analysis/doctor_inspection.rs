//! Read-only, bounded inspection. Findings describe observations, never guessed causes.
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
#[cfg(windows)]
use std::io::{Seek, SeekFrom};
use std::path::Path;
use walkdir::WalkDir;

const ENTRY_LIMIT: usize = 60_000;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionReport {
    pub scenery_ini_exists: bool,
    pub missing_scenery_paths: Vec<String>,
    pub duplicate_scenery_paths: Vec<String>,
    pub broken_scenery_links: Vec<String>,
    pub empty_navdata_files: Vec<String>,
    pub plugin_binary_problems: Vec<String>,
    pub scan_errors: Vec<String>,
    pub scan_capped: bool,
    pub plugin_binaries_checked: usize,
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
        let shortcut = target.with_extension("lnk");
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
) {
    let ini = root.join("Custom Scenery/scenery_packs.ini");
    report.scenery_ini_exists = ini.is_file();
    if report.scenery_ini_exists {
        match fs::read_to_string(&ini) {
            Ok(body) => {
                let mut seen = HashMap::new();
                for (index, line) in body.lines().enumerate() {
                    let Some(path) = line.trim().strip_prefix("SCENERY_PACK ") else {
                        continue; // Disabled scenery is intentionally not loaded.
                    };
                    let path = path.trim();
                    if path == "*GLOBAL_AIRPORTS*" {
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
                    if let Some(first) = seen.insert(key, index + 1) {
                        report
                            .duplicate_scenery_paths
                            .push(format!("lines {first}, {}: {path}", index + 1));
                    }
                    match fs::metadata(&target) {
                        Ok(meta) if meta.is_dir() => {}
                        Ok(_) => report
                            .missing_scenery_paths
                            .push(format!("line {}: {path} (not a directory)", index + 1)),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => report
                            .missing_scenery_paths
                            .push(format!("line {}: {path}", index + 1)),
                        Err(error) => report.scan_errors.push(format!("{path}: {error}")),
                    }
                }
            }
            Err(error) => report
                .scan_errors
                .push(format!("Custom Scenery/scenery_packs.ini: {error}")),
        }
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
}

/// Validate only binaries intended for this host, leaving other-platform files alone.
fn binary_problem(path: &Path) -> Result<Option<&'static str>, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut header = [0u8; 64];
    let length = file.read(&mut header)?;
    if length == 0 {
        return Ok(Some("empty plugin binary"));
    }
    #[cfg(windows)]
    {
        if length < 64 || &header[..2] != b"MZ" {
            return Ok(Some("invalid PE header"));
        }
        let offset = u32::from_le_bytes(header[60..64].try_into().unwrap());
        file.seek(SeekFrom::Start(offset as u64))?;
        let mut pe = [0u8; 6];
        if file.read_exact(&mut pe).is_err() || &pe[..4] != b"PE\0\0" {
            return Ok(Some("truncated or invalid PE header"));
        }
        let machine = u16::from_le_bytes([pe[4], pe[5]]);
        if machine == 0x014c {
            return Ok(Some("32-bit plugin; X-Plane requires 64-bit"));
        }
        #[cfg(target_arch = "x86_64")]
        if machine != 0x8664 {
            return Ok(Some("plugin architecture does not match x86-64 host"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        if length < 20 || &header[..4] != b"\x7fELF" {
            return Ok(Some("invalid ELF header"));
        }
        if header[4] != 2 {
            return Ok(Some("plugin is not 64-bit ELF"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if length < 4
            || !matches!(
                &header[..4],
                [0xcf, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xcf]
                    | [0xca, 0xfe, 0xba, 0xbe]
                    | [0xca, 0xfe, 0xba, 0xbf]
            )
        {
            return Ok(Some("invalid 64-bit or universal Mach-O header"));
        }
    }
    Ok(None)
}

fn inspect_plugins(root: &Path, report: &mut InspectionReport) {
    let host_name = if cfg!(windows) {
        "win.xpl"
    } else if cfg!(target_os = "macos") {
        "mac.xpl"
    } else {
        "lin.xpl"
    };
    let mut visited = 0;
    for directory in ["Resources/plugins", "Aircraft"] {
        let path = root.join(directory);
        if !path.is_dir() {
            continue;
        }
        for entry in WalkDir::new(path)
            .follow_links(false)
            .max_depth(12)
            .into_iter()
            .filter_entry(|entry| {
                !entry
                    .file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(".disabled")
            })
        {
            visited += 1;
            if visited > ENTRY_LIMIT {
                report.scan_capped = true;
                return;
            }
            match entry {
                Ok(entry)
                    if entry.file_type().is_file()
                        && entry
                            .file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(host_name) =>
                {
                    report.plugin_binaries_checked += 1;
                    let relative = entry
                        .path()
                        .strip_prefix(root)
                        .unwrap_or(entry.path())
                        .display()
                        .to_string();
                    match binary_problem(entry.path()) {
                        Ok(Some(reason)) => report
                            .plugin_binary_problems
                            .push(format!("{relative}: {reason}")),
                        Err(error) => report.scan_errors.push(format!("{relative}: {error}")),
                        _ => {}
                    }
                }
                Err(error) => report.scan_errors.push(error.to_string()),
                _ => {}
            }
        }
    }
}

pub fn inspect_installation(
    root: &Path,
    full: bool,
    resolve_shortcut: ShortcutResolver,
) -> InspectionReport {
    let mut report = InspectionReport::default();
    inspect_scenery(root, &mut report, resolve_shortcut);
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
        inspect_plugins(root, &mut report);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(report.plugin_binaries_checked, 1);
        assert_eq!(report.plugin_binary_problems.len(), 1);
        assert_eq!(
            inspect_installation(temp.path(), false, |_| None).plugin_binaries_checked,
            0
        );
    }
}

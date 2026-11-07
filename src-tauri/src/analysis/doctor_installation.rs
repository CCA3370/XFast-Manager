//! Startup assets and the stock aircraft needed by the supported X-Plane 12 installation.
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const STOCK_AIRCRAFT: &[&str] = &[
    "Cessna_172SP.acf",
    "Cessna_172SP_G1000.acf",
    "Cessna_172SP_seaplane.acf",
    "Cirrus SR22.acf",
    "L5_Sentinel.acf",
    "N844X.acf",
    "PA-18-150.acf",
    "S-76C.acf",
];

const STARTUP_FILES: &[&str] = &[
    "Resources/settings.txt",
    "Resources/rand.bin",
    "Resources/planet_ele.bin",
    "Resources/geoids/egm96-5.pgm",
    "Resources/timezones/timezone16.bin",
    "Resources/fonts/Roboto-Regular.ttf",
    "Resources/sounds/Master Bank.bank",
    "Resources/sounds/environment.bank",
    "Resources/default data/earth_nav.dat",
    "Resources/default data/earth_fix.dat",
    "Resources/default data/earth_awy.dat",
    "Resources/default data/earth_hold.dat",
    "Resources/default data/earth_mora.dat",
    "Resources/default data/earth_msa.dat",
    "Resources/default data/airspaces/airspace.txt",
    "Resources/default scenery/1200 atc data/Earth nav data/atc.dat",
    "Resources/default scenery/1000 autogen/library.txt",
    "Resources/default scenery/1000 roads/library.txt",
    "Resources/default scenery/1000 world terrain/library.txt",
    "Resources/default scenery/airport scenery/library.txt",
    "Resources/default scenery/sim objects/library.txt",
    "Resources/shaders/bin/blit_mapping.xsv",
    "Global Scenery/Global Airports/Earth nav data/apt.dat",
];

const AIRCRAFT_ENTRY_LIMIT: usize = 100_000;
const AIRFOIL_ENTRY_LIMIT: usize = 4096;
const ACF_BYTE_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Findings {
    pub problems: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationInspection {
    pub root_usable: bool,
    pub core: Findings,
    pub aircraft: Findings,
    pub airfoils: Findings,
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn open_regular(path: &Path) -> io::Result<File> {
    if !fs::metadata(path)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a regular file",
        ));
    }
    File::open(path)
}

fn check_file(
    root: &Path,
    path: &Path,
    findings: &mut Findings,
    inspect: impl FnOnce(File) -> io::Result<Option<&'static str>>,
) -> bool {
    let result = open_regular(path).and_then(inspect);
    match result {
        Ok(None) => true,
        Ok(Some(problem)) => {
            findings
                .problems
                .push(format!("{}: {problem}", relative(root, path)));
            false
        }
        Err(error) => {
            let name = relative(root, path);
            match error.kind() {
                io::ErrorKind::NotFound => findings.problems.push(format!("{name}: missing")),
                io::ErrorKind::InvalidData => findings.problems.push(format!("{name}: {error}")),
                _ => findings.errors.push(format!("{name}: {error}")),
            }
            false
        }
    }
}

fn empty_file(mut file: File) -> io::Result<Option<&'static str>> {
    Ok((file.read(&mut [0])? == 0).then_some("empty file"))
}

fn check_native_binary(
    root: &Path,
    path: &Path,
    findings: &mut Findings,
    inspect: fn(&Path) -> io::Result<Option<&'static str>>,
) {
    check_file(root, path, findings, |_| inspect(path));
}

fn check_core(
    root: &Path,
    launcher: Option<&Path>,
    findings: &mut Findings,
    inspect_binary: fn(&Path) -> io::Result<Option<&'static str>>,
) {
    if let Some(launcher) = launcher {
        let executable = if cfg!(target_os = "macos") {
            launcher.join("Contents/MacOS/X-Plane")
        } else {
            launcher.to_path_buf()
        };
        check_native_binary(root, &executable, findings, inspect_binary);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if fs::metadata(&executable).is_ok_and(|meta| meta.permissions().mode() & 0o111 == 0) {
                findings.problems.push(format!(
                    "{}: executable permission is missing",
                    relative(root, &executable)
                ));
            }
        }
    } else {
        findings.problems.push("X-Plane executable: missing".into());
    }

    let runtime_libraries: &[&str] = if cfg!(windows) {
        &[
            "Resources/plugins/XPLM_64.dll",
            "Resources/plugins/XPWidgets_64.dll",
            "Resources/dlls/64/fmod.dll",
            "Resources/dlls/64/fmodstudio.dll",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "Resources/plugins/XPLM.framework/XPLM",
            "Resources/plugins/XPWidgets.framework/XPWidgets",
        ]
    } else {
        &[
            "Resources/plugins/XPLM_64.so",
            "Resources/plugins/XPWidgets_64.so",
            "Resources/dlls/64/libfmod.so.13",
            "Resources/dlls/64/libfmodstudio.so.13",
        ]
    };
    for name in runtime_libraries {
        check_native_binary(root, &root.join(name), findings, inspect_binary);
    }
    for name in STARTUP_FILES {
        check_file(root, &root.join(name), findings, empty_file);
    }
    let renderer = if cfg!(target_os = "macos") {
        "msl"
    } else {
        "spv"
    };
    for name in ["blit.xsa", "atmosphere.xsa"] {
        let path = root.join("Resources/shaders/bin").join(renderer).join(name);
        check_file(root, &path, findings, |mut file| {
            let mut signature = [0; 4];
            match file.read_exact(&mut signature) {
                Ok(()) if signature == *b"PK\x03\x04" => Ok(None),
                Ok(()) => Ok(Some("invalid shader archive signature")),
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    Ok(Some("truncated shader archive"))
                }
                Err(error) => Err(error),
            }
        });
    }
}

fn find_stock_aircraft(root: &Path, findings: &mut Findings) -> BTreeMap<&'static str, PathBuf> {
    let mut found = BTreeMap::new();
    let mut skipped_link = false;
    let aircraft = root.join("Aircraft");
    for (index, entry) in WalkDir::new(&aircraft)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0 || !entry.file_name().to_string_lossy().starts_with('.')
        })
        .enumerate()
    {
        if index == AIRCRAFT_ENTRY_LIMIT {
            findings
                .errors
                .push("Aircraft: reached the inventory limit".into());
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error)
                if error.depth() == 0
                    && error
                        .io_error()
                        .is_some_and(|e| e.kind() == io::ErrorKind::NotFound) =>
            {
                findings.problems.push("Aircraft: missing directory".into());
                return found;
            }
            Err(error) => {
                findings.errors.push(error.to_string());
                continue;
            }
        };
        if entry.file_type().is_symlink()
            && fs::metadata(entry.path()).is_ok_and(|meta| meta.is_dir())
        {
            skipped_link = true;
        }
        let Some(name) = STOCK_AIRCRAFT
            .iter()
            .copied()
            .find(|name| entry.file_name().eq_ignore_ascii_case(name))
        else {
            continue;
        };
        match fs::metadata(entry.path()) {
            Ok(meta) if meta.is_file() => {
                found.entry(name).or_insert_with(|| entry.into_path());
            }
            Ok(_) => {}
            Err(error) => findings
                .errors
                .push(format!("{}: {error}", relative(root, entry.path()))),
        }
        if found.len() == STOCK_AIRCRAFT.len() {
            return found;
        }
    }
    if skipped_link {
        findings
            .errors
            .push("Aircraft: linked directories were not searched".into());
    }
    if findings.errors.is_empty() {
        findings.problems.extend(
            STOCK_AIRCRAFT
                .iter()
                .filter(|name| !found.contains_key(**name))
                .map(|name| format!("Aircraft/**/{name}: missing")),
        );
    }
    found
}

fn airfoil_header(mut file: File) -> io::Result<Option<&'static str>> {
    let mut head = Vec::new();
    file.by_ref().take(4096).read_to_end(&mut head)?;
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let marker = lines.next().map(str::trim);
    let mut version = lines.next().unwrap_or("").split_whitespace();
    let valid = matches!(marker, Some("I" | "A"))
        && version
            .next()
            .is_some_and(|token| token.parse::<u16>().is_ok())
        && version
            .next()
            .is_some_and(|token| token.eq_ignore_ascii_case("version"))
        && lines.any(|line| !line.trim().is_empty());
    Ok((!valid).then_some("empty or invalid airfoil header"))
}

fn airfoil_references(path: &Path) -> io::Result<BTreeSet<String>> {
    let mut bytes = Vec::new();
    open_regular(path)?
        .take(ACF_BYTE_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > ACF_BYTE_LIMIT {
        return Err(io::Error::other(
            "aircraft file exceeds the airfoil reference inspection limit",
        ));
    }
    Ok(String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("P ")?;
            let (key, value) = rest.split_once(char::is_whitespace)?;
            let value = value.trim();
            (key.contains("/_afl_file_")
                && value.to_ascii_lowercase().ends_with(".afl")
                && !value.contains(['/', '\\', ':']))
            .then(|| value.to_owned())
        })
        .collect())
}

fn inspect_airfoils(
    root: &Path,
    aircraft: &BTreeMap<&'static str, PathBuf>,
    findings: &mut Findings,
) {
    let global = root.join("Airfoils");
    let mut checked = BTreeSet::new();
    let mut usable = 0;
    match fs::read_dir(&global) {
        Ok(entries) => {
            for (index, entry) in entries.enumerate() {
                if index == AIRFOIL_ENTRY_LIMIT {
                    findings
                        .errors
                        .push("Airfoils: reached the inventory limit".into());
                    break;
                }
                let path = match entry {
                    Ok(entry) => entry.path(),
                    Err(error) => {
                        findings.errors.push(format!("Airfoils: {error}"));
                        continue;
                    }
                };
                if !path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("afl"))
                {
                    continue;
                }
                if check_file(root, &path, findings, airfoil_header) {
                    usable += 1;
                }
                checked.insert(path);
            }
            if usable == 0 && findings.errors.is_empty() {
                findings
                    .problems
                    .push("Airfoils: no usable .afl files".into());
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            findings.problems.push("Airfoils: missing directory".into())
        }
        Err(error) => findings.errors.push(format!("Airfoils: {error}")),
    }

    for (&name, path) in aircraft {
        let refs = match airfoil_references(path) {
            Ok(refs) => refs,
            Err(error) => {
                findings
                    .errors
                    .push(format!("{}: {error}", relative(root, path)));
                continue;
            }
        };
        let package = path.parent().expect("stock aircraft always has a parent");
        for reference in refs {
            let local = package.join("airfoils").join(&reference);
            let alternate = package.join("Airfoils").join(&reference);
            let global_path = global.join(&reference);
            let location = [local, alternate, global_path].into_iter().find(|candidate| {
                // An unreadable file may still be the simulator's selected file.
                !matches!(fs::metadata(candidate), Err(error) if error.kind() == io::ErrorKind::NotFound)
            });
            if let Some(location) = location {
                if checked.insert(location.clone()) {
                    check_file(root, &location, findings, airfoil_header);
                }
            } else {
                findings
                    .problems
                    .push(format!("{name}: missing airfoil {reference}"));
            }
        }
    }
    findings.problems.sort();
    findings.problems.dedup();
}

pub fn inspect(
    root: &Path,
    launcher: Option<&Path>,
    inspect_binary: fn(&Path) -> io::Result<Option<&'static str>>,
) -> InstallationInspection {
    let mut report = InstallationInspection::default();
    match fs::read_dir(root) {
        Ok(_) => report.root_usable = true,
        Err(error) => {
            let detail = format!("{}: {error}", root.display());
            if error.kind() == io::ErrorKind::NotFound
                || error.kind() == io::ErrorKind::NotADirectory
            {
                report.core.problems.push(detail);
            } else {
                report.core.errors.push(detail);
            }
            return report;
        }
    }
    check_core(root, launcher, &mut report.core, inspect_binary);
    let aircraft = find_stock_aircraft(root, &mut report.aircraft);
    inspect_airfoils(root, &aircraft, &mut report.airfoils);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn reports_missing_or_empty_runtime_files_inside_an_existing_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Resources/settings.txt", "");
        let result = inspect(root, None, |_| Ok(None));
        assert!(result.root_usable);
        assert!(result
            .core
            .problems
            .iter()
            .any(|item| item == "X-Plane executable: missing"));
        assert!(result
            .core
            .problems
            .iter()
            .any(|item| item == "Resources/settings.txt: empty file"));
        assert!(result
            .core
            .problems
            .iter()
            .any(|item| item == "Resources/default data/earth_nav.dat: missing"));
    }

    #[test]
    fn matches_acf_file_names_beneath_renamed_folders() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Aircraft/Renamed/GA/Cessna_172SP.acf", "");
        fs::create_dir(root.join("Aircraft/Renamed/Cirrus SR22.acf")).unwrap();
        let mut findings = Findings::default();
        let found = find_stock_aircraft(root, &mut findings);
        assert_eq!(
            found.keys().copied().collect::<Vec<_>>(),
            ["Cessna_172SP.acf"]
        );
        assert!(findings
            .problems
            .iter()
            .any(|item| item.ends_with("Cirrus SR22.acf: missing")));
    }

    #[test]
    fn checks_global_airfoils_and_resolves_aircraft_local_references() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(
            root,
            "Airfoils/Global.afl",
            "I\n1110 Version\n1234 device type code\n",
        );
        write(root, "Airfoils/Broken.afl", "bad header");
        write(
            root,
            "Aircraft/Custom/airfoils/Local wing.afl",
            "A\n1110 Version\n1234 device type code\n",
        );
        write(root, "Aircraft/Custom/N844X.acf",
            "P _wing/0/_afl_file_1 Local wing.afl\nP _wing/0/_afl_file_2 Global.afl\nP _wing/0/_afl_file_3 Missing wing.afl\n");
        let result = inspect(root, None, |_| Ok(None));
        assert_eq!(
            result.airfoils.problems,
            [
                "Airfoils/Broken.afl: empty or invalid airfoil header",
                "N844X.acf: missing airfoil Missing wing.afl",
            ]
        );
        assert!(result.airfoils.errors.is_empty());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires an explicitly selected X-Plane 12 reference installation"]
    fn verifies_reference_installation() {
        let root = PathBuf::from(std::env::var_os("XFAST_HEALTH_REFERENCE_INSTALLATION").unwrap());
        let report = inspect(
            &root,
            Some(&root.join("X-Plane.exe")),
            super::super::inspection::host_binary_problem,
        );
        assert!(report.root_usable, "{report:?}");
        for findings in [report.core, report.aircraft, report.airfoils] {
            assert!(
                findings.problems.is_empty() && findings.errors.is_empty(),
                "{findings:?}"
            );
        }
    }
}

//! Read only files X-Plane can load from enabled scenery packages.
use serde::Serialize;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

const TILE_LIMIT: usize = 250_000;
const SCAN_TIME_LIMIT: Duration = Duration::from_secs(20);

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SceneryFiles {
    pub(super) checked: usize,
    pub(super) problems: Vec<String>,
    pub(super) errors: Vec<String>,
    pub(super) capped: bool,
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn text_header(mut file: File, library: bool) -> io::Result<bool> {
    let mut bytes = Vec::new();
    file.by_ref().take(4096).read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = text.trim_start_matches('\u{feff}').lines().map(str::trim);
    let marker = lines.next();
    let version = lines.next().and_then(|line| line.split_whitespace().next());
    Ok(matches!(marker, Some("I" | "A"))
        && version.is_some_and(|token| token.parse::<u16>().is_ok())
        && (!library
            || lines.find(|line| !line.is_empty() && !line.starts_with('#')) == Some("LIBRARY")))
}

fn dsf_header(mut file: File) -> io::Result<bool> {
    let size = file.metadata()?.len();
    let mut header = [0; 12];
    match file.read_exact(&mut header) {
        Ok(()) => Ok((size >= 28
            && &header[..8] == b"XPLNEDSF"
            && header[8..] == 1u32.to_le_bytes())
            || (size >= 32 && header.starts_with(b"7z\xbc\xaf\x27\x1c"))),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(error),
    }
}

fn coordinates(name: &OsStr) -> Option<(i32, i32)> {
    let name = name.to_str()?;
    let bytes = name.as_bytes();
    if bytes.len() != 7
        || !matches!(bytes[0], b'+' | b'-')
        || !matches!(bytes[3], b'+' | b'-')
        || !bytes[1..3]
            .iter()
            .chain(&bytes[4..])
            .all(u8::is_ascii_digit)
        || name.starts_with("-00")
        || name.ends_with("-000")
    {
        return None;
    }
    let latitude = name[..3].parse().ok()?;
    let longitude = name[3..].parse().ok()?;
    ((-90..90).contains(&latitude) && (-180..180).contains(&longitude))
        .then_some((latitude, longitude))
}

fn is_tile_block(path: &Path) -> bool {
    path.file_name()
        .and_then(coordinates)
        .is_some_and(|(latitude, longitude)| latitude % 10 == 0 && longitude % 10 == 0)
}

fn is_tile_file(path: &Path) -> bool {
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("dsf"))
    {
        return false;
    }
    let Some((latitude, longitude)) = path.file_stem().and_then(coordinates) else {
        return false;
    };
    let expected_block = (latitude.div_euclid(10) * 10, longitude.div_euclid(10) * 10);
    path.parent()
        .and_then(Path::file_name)
        .and_then(coordinates)
        == Some(expected_block)
}

fn check(
    root: &Path,
    path: &Path,
    required: bool,
    report: &mut SceneryFiles,
    verify: impl FnOnce(File) -> io::Result<bool>,
) -> bool {
    let label = relative(root, path);
    let result = fs::metadata(path).and_then(|meta| {
        if meta.is_file() {
            File::open(path).and_then(verify)
        } else {
            Ok(false)
        }
    });
    match result {
        Ok(true) => true,
        Ok(false) => {
            report
                .problems
                .push(format!("{label}: empty or invalid file header"));
            true
        }
        Err(error) if !required && error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => {
            report.errors.push(format!("{label}: {error}"));
            true
        }
    }
}

pub(super) fn inspect(root: &Path, packages: &[PathBuf]) -> SceneryFiles {
    inspect_with_budget(root, packages, TILE_LIMIT, SCAN_TIME_LIMIT)
}

fn inspect_with_budget(
    root: &Path,
    packages: &[PathBuf],
    mut remaining_tiles: usize,
    duration: Duration,
) -> SceneryFiles {
    let mut report = SceneryFiles::default();
    let started = Instant::now();
    'packages: for package in packages {
        if started.elapsed() >= duration {
            report.capped = true;
            break;
        }
        report.checked += 1;
        let errors_before = report.errors.len();
        let library = check(
            root,
            &package.join("library.txt"),
            false,
            &mut report,
            |file| text_header(file, true),
        );
        let nav = package.join("Earth nav data");
        let airports = check(root, &nav.join("apt.dat"), false, &mut report, |file| {
            text_header(file, false)
        });
        let mut tiles = false;
        let entries = WalkDir::new(&nav)
            .max_depth(2)
            .follow_links(true)
            .into_iter()
            .filter_entry(|entry| entry.depth() != 1 || is_tile_block(entry.path()));
        for entry in entries {
            if started.elapsed() >= duration {
                report.capped = true;
                break 'packages;
            }
            match entry {
                Ok(entry) if entry.depth() == 2 && is_tile_file(entry.path()) => {
                    if remaining_tiles == 0 {
                        report.capped = true;
                        break 'packages;
                    }
                    remaining_tiles -= 1;
                    tiles = true;
                    check(root, entry.path(), true, &mut report, dsf_header);
                }
                Ok(_) => {}
                Err(error)
                    if error.depth() == 0
                        && error
                            .io_error()
                            .is_some_and(|error| error.kind() == io::ErrorKind::NotFound) => {}
                Err(error)
                    if error.path().is_none_or(|path| match error.depth() {
                        1 => is_tile_block(path),
                        2 => is_tile_file(path),
                        _ => true,
                    }) =>
                {
                    report.errors.push(error.to_string());
                }
                Err(_) => {}
            }
        }
        if !library && !airports && !tiles && report.errors.len() == errors_before {
            // Some packages intentionally load only scenery plugins.
            let plugins = package.join("plugins");
            match fs::read_dir(&plugins) {
                Ok(entries) => match entries.into_iter().next() {
                    Some(Ok(_)) => {}
                    Some(Err(error)) => report
                        .errors
                        .push(format!("{}: {error}", relative(root, &plugins))),
                    None => report.problems.push(format!(
                        "{}: no scenery data, library or plugins found",
                        relative(root, package)
                    )),
                },
                Err(error) if error.kind() != io::ErrorKind::NotFound => {
                    report
                        .errors
                        .push(format!("{}: {error}", relative(root, &plugins)));
                }
                _ => report.problems.push(format!(
                    "{}: no scenery data, library or plugins found",
                    relative(root, package)
                )),
            }
        }
    }
    report.problems.sort();
    report.errors.sort();
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_stray_dsf_files_when_the_loadable_tile_is_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let package = root.join("KJFK");
        let nav = package.join("Earth nav data");
        let block = nav.join("+40-080");
        let wrong_block = nav.join("+40-070");
        fs::create_dir_all(&block).unwrap();
        fs::create_dir_all(&wrong_block).unwrap();
        fs::write(
            nav.join("+40-074.dsf"),
            "I\n1200 Generated by WorldEditor\n",
        )
        .unwrap();
        fs::write(wrong_block.join("+40-074.dsf"), "").unwrap();
        let mut tile = [0; 36];
        tile[..8].copy_from_slice(b"XPLNEDSF");
        tile[8..12].copy_from_slice(&1u32.to_le_bytes());
        fs::write(block.join("+40-074.dsf"), tile).unwrap();

        let report = inspect_with_budget(root, &[package], 1, SCAN_TIME_LIMIT);
        assert_eq!(report.checked, 1);
        assert!(!report.capped, "{report:?}");
        assert!(
            report.problems.is_empty() && report.errors.is_empty(),
            "{report:?}"
        );
    }

    #[test]
    fn applies_the_limit_to_dsf_files_and_does_not_count_it_as_a_read_error() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let package = root.join("Test");
        let nav = package.join("Earth nav data/+00+000");
        fs::create_dir_all(&nav).unwrap();
        for index in 0..10 {
            fs::write(nav.join(format!("other-{index}.txt")), "irrelevant").unwrap();
        }
        let mut bytes = [0; 32];
        bytes[..6].copy_from_slice(b"7z\xbc\xaf\x27\x1c");
        fs::write(nav.join("+01+001.dsf"), bytes).unwrap();
        let report = inspect_with_budget(root, &[package.clone()], 1, SCAN_TIME_LIMIT);
        assert!(!report.capped && report.errors.is_empty(), "{report:?}");

        fs::write(nav.join("+01+002.dsf"), bytes).unwrap();
        let report = inspect_with_budget(root, &[package], 1, SCAN_TIME_LIMIT);
        assert!(report.capped && report.errors.is_empty(), "{report:?}");
    }

    #[test]
    fn recognizes_airport_only_library_only_and_compressed_tiles() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let airport = root.join("Airport");
        let library = root.join("Library");
        let mesh = root.join("Mesh");
        fs::create_dir_all(airport.join("Earth nav data")).unwrap();
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(mesh.join("Earth nav data/+00+000")).unwrap();
        fs::write(
            airport.join("Earth nav data/apt.dat"),
            "I\n1200 Version\n1 0 0 0 KTEST Test\n99\n",
        )
        .unwrap();
        fs::write(
            library.join("library.txt"),
            "A\n800\nLIBRARY\nEXPORT test test.obj\n",
        )
        .unwrap();
        let mut bytes = [0; 32];
        bytes[..6].copy_from_slice(b"7z\xbc\xaf\x27\x1c");
        fs::write(mesh.join("Earth nav data/+00+000/+01+001.dsf"), bytes).unwrap();
        let result = inspect(root, &[airport, library, mesh]);
        assert_eq!(result.checked, 3);
        assert!(
            result.problems.is_empty() && result.errors.is_empty(),
            "{result:?}"
        );
    }

    #[test]
    fn points_to_the_broken_file_in_an_enabled_package() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let scenery = root.join("Custom Scenery/Test");
        let nav = scenery.join("Earth nav data/+00+000");
        fs::create_dir_all(&nav).unwrap();
        fs::write(nav.join("+01+001.dsf"), [0]).unwrap();
        fs::write(scenery.join("library.txt"), "not a library").unwrap();
        let result = inspect(root, &[scenery]);
        assert_eq!(result.problems, [
            "Custom Scenery/Test/Earth nav data/+00+000/+01+001.dsf: empty or invalid file header",
            "Custom Scenery/Test/library.txt: empty or invalid file header",
        ]);
    }
}

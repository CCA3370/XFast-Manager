//! Bounded inspection of X-Plane's navigation data layers and installed AIRAC metadata.
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NavdataStatus {
    Ok,
    ExpiringSoon,
    Expired,
    NotYetEffective,
    /// Validity dates could not be determined (only cycle.json, no cycle_info.txt).
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavdataCycleReport {
    /// Directory containing metadata, relative to the installation.
    pub folder_name: String,
    pub provider_name: String,
    pub cycle: Option<String>,
    pub airac: Option<String>,
    /// "YYYY-MM-DD" effective date when known.
    pub effective_date: Option<String>,
    /// "YYYY-MM-DD" expiry date when known.
    pub expiry_date: Option<String>,
    /// Days remaining until expiry (negative if already expired). None if unknown.
    pub days_remaining: Option<i64>,
    pub status: NavdataStatus,
    pub metadata_conflict: Option<String>,
    /// Metadata files found for this package.
    pub source: String,
}

/// Current UTC date as days since the Unix epoch (date-only, to avoid TZ
/// off-by-one when comparing validity).
fn today_epoch_day() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    secs / 86_400
}

/// Convert a Y-M-D civil date to days since the Unix epoch.
/// Howard Hinnant's algorithm (proleptic Gregorian).
fn ymd_to_epoch_day(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (m as i64 + 9) % 12; // [0, 11]
    let doy = (153 * mp + 2) / 5 + d as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

/// Format an epoch-day count back to "YYYY-MM-DD".
fn epoch_day_to_ymd_string(z: i64) -> String {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn month_from_abbrev(s: &str) -> Option<u32> {
    match s.to_ascii_uppercase().as_str() {
        "JAN" => Some(1),
        "FEB" => Some(2),
        "MAR" => Some(3),
        "APR" => Some(4),
        "MAY" => Some(5),
        "JUN" => Some(6),
        "JUL" => Some(7),
        "AUG" => Some(8),
        "SEP" => Some(9),
        "OCT" => Some(10),
        "NOV" => Some(11),
        "DEC" => Some(12),
        _ => None,
    }
}

fn days_in_month(year: i64, month: u32) -> Option<u32> {
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => return None,
    };
    Some(days)
}

/// Parse a "DD/MMM/YYYY" token (e.g. "14/MAY/2026") to epoch day.
fn parse_dmy(token: &str) -> Option<i64> {
    let parts: Vec<&str> = token.trim().split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let d: u32 = parts[0].trim().parse().ok()?;
    let m = month_from_abbrev(parts[1].trim())?;
    let y: i64 = parts[2].trim().parse().ok()?;
    if d == 0 || d > days_in_month(y, m)? {
        return None;
    }
    Some(ymd_to_epoch_day(y, m, d))
}

/// Extract (from, to) epoch days from a cycle_info.txt body containing a line
/// like: `Valid (from/to): 14/MAY/2026 - 11/JUN/2026`.
fn parse_validity_range(body: &str) -> Option<(i64, i64)> {
    for line in body.lines() {
        let lower = line.to_lowercase();
        if !lower.contains("valid") {
            continue;
        }
        // Take everything after the first ':' then split on '-'.
        let after = line.split_once(':').map_or(line, |(_, value)| value);
        let halves: Vec<&str> = after.split('-').map(|s| s.trim()).collect();
        if halves.len() < 2 {
            continue;
        }
        if let (Some(from), Some(to)) = (parse_dmy(halves[0]), parse_dmy(halves[1])) {
            if to >= from {
                return Some((from, to));
            }
        }
    }
    None
}

fn status_for(expiry_day: i64, today: i64) -> (NavdataStatus, i64) {
    let days_remaining = expiry_day - today;
    let status = if days_remaining <= 0 {
        NavdataStatus::Expired
    } else if days_remaining <= EXPIRING_SOON_DAYS {
        NavdataStatus::ExpiringSoon
    } else {
        NavdataStatus::Ok
    };
    (status, days_remaining)
}

fn json_scalar_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

/// Read provider/cycle/airac from a cycle.json next to a navdata set.
/// Providers in the wild use both JSON strings and numbers for cycle fields.
fn read_cycle_json(path: &Path) -> io::Result<(Option<String>, Option<String>, Option<String>)> {
    let content = read_small_text(path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    if !json.is_object() {
        return Err(io::Error::other("expected a JSON object"));
    }
    let provider = json.get("name").and_then(|v| v.as_str()).map(str::to_owned);
    let cycle = json.get("cycle").and_then(json_scalar_to_string);
    let airac = json.get("airac").and_then(json_scalar_to_string);
    Ok((provider, cycle, airac))
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavdataLayer {
    present: bool,
    problems: Vec<String>,
    errors: Vec<String>,
    headers: Vec<String>,
    #[serde(skip)]
    consistent_cycle: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorNavdataReport {
    bundled: NavdataLayer,
    override_data: NavdataLayer,
    supplemental: NavdataLayer,
    cycles: Vec<NavdataCycleReport>,
    metadata_errors: Vec<String>,
}

const EXPIRING_SOON_DAYS: i64 = 7;
const CORE_FILES: &[&str] = &["earth_fix.dat", "earth_nav.dat", "earth_awy.dat"];
const EXTRA_FILES: &[&str] = &[
    "earth_hold.dat",
    "earth_mora.dat",
    "earth_msa.dat",
    "earth_aptmeta.dat",
    "airspaces/airspace.txt",
];
const SUPPLEMENTAL_FILES: &[&str] = &["earth_424.dat", "FAACIFP18", "user_nav.dat", "user_fix.dat"];
const PROCEDURE_LIMIT: usize = 20_000;
const METADATA_ENTRY_LIMIT: usize = 40_000;

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn metadata(path: &Path) -> io::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => fs::metadata(path).map(Some),
        Ok(meta) => Ok(Some(meta)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn open_data_file(
    root: &Path,
    path: &Path,
    layer: &mut NavdataLayer,
    required: bool,
) -> Option<File> {
    let name = relative(root, path);
    let meta = match metadata(path) {
        Ok(Some(meta)) => meta,
        Ok(None) => {
            if required {
                layer.problems.push(format!("{name}: missing"));
            }
            return None;
        }
        Err(error) => {
            layer.errors.push(format!("{name}: {error}"));
            return None;
        }
    };
    layer.present = true;
    if !meta.is_file() || meta.len() == 0 {
        let problem = if meta.is_file() {
            "empty file"
        } else {
            "not a regular file"
        };
        layer.problems.push(format!("{name}: {problem}"));
        return None;
    }
    match File::open(path) {
        Ok(file) => Some(file),
        Err(error) => {
            layer.errors.push(format!("{name}: {error}"));
            None
        }
    }
}

fn navdata_file(mut file: File, full: bool) -> io::Result<Result<Option<String>, &'static str>> {
    const BLOCK: u64 = 4096;
    let size = file.metadata()?.len();
    let mut bytes = Vec::new();
    file.by_ref().take(BLOCK).read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    if !matches!(lines.next().map(str::trim), Some("I" | "A")) {
        return Ok(Err("invalid X-Plane data header"));
    }
    let Some(version) = lines.next() else {
        return Ok(Err("invalid X-Plane data header"));
    };
    let mut tokens = version.split_whitespace();
    if !matches!(tokens.next(), Some(value) if value.parse::<u16>().is_ok())
        || !matches!(tokens.next(), Some(value) if value.eq_ignore_ascii_case("version"))
    {
        return Ok(Err("invalid X-Plane data header"));
    }
    let lower = version.to_ascii_lowercase();
    let cycle = lower.split_once("data cycle ").and_then(|(_, rest)| {
        let token = rest
            .trim_start()
            .split(|c: char| !c.is_ascii_digit())
            .next()?;
        (token.len() == 4).then(|| token.to_owned())
    });
    if size <= BLOCK
        && !lines
            .map(str::trim)
            .any(|line| !line.is_empty() && !line.starts_with('#') && line != "99")
    {
        return Ok(Err("navigation data file has no records"));
    }
    if full {
        file.seek(SeekFrom::End(-(size.min(BLOCK) as i64)))?;
        bytes.clear();
        file.take(BLOCK).read_to_end(&mut bytes)?;
        let tail = String::from_utf8_lossy(&bytes);
        if tail
            .lines()
            .rev()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            != Some("99")
        {
            return Ok(Err("missing final 99 record; file may be truncated"));
        }
    }
    Ok(Ok(cycle))
}

fn inspect_procedures(root: &Path, dir: &Path, full: bool, layer: &mut NavdataLayer) {
    let path = dir.join("CIFP");
    let name = relative(root, &path);
    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            layer.problems.push(format!("{name}: missing directory"));
            return;
        }
        Err(error) => {
            layer.errors.push(format!("{name}: {error}"));
            return;
        }
    };
    let mut procedures = NavdataLayer::default();
    let mut saw_dat = false;
    for (index, entry) in entries.enumerate() {
        if index == PROCEDURE_LIMIT {
            procedures.errors.push(format!(
                "{name}: reached the {PROCEDURE_LIMIT} entry inspection limit"
            ));
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                procedures.errors.push(format!("{name}: {error}"));
                continue;
            }
        };
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dat"))
        {
            saw_dat = true;
            if open_data_file(root, &path, &mut procedures, true).is_some() && !full {
                return;
            }
        }
    }
    if !saw_dat && procedures.errors.is_empty() {
        procedures
            .problems
            .push(format!("{name}: no procedure .dat files"));
    }
    layer.errors.extend(procedures.errors);
    layer.problems.extend(procedures.problems);
}

fn inspect_base(root: &Path, dir: &Path, full: bool) -> NavdataLayer {
    let mut layer = NavdataLayer {
        present: true,
        ..Default::default()
    };
    let mut cycles = BTreeSet::new();
    for (name, required) in CORE_FILES
        .iter()
        .map(|name| (*name, true))
        .chain(EXTRA_FILES.iter().map(|name| (*name, false)))
    {
        let path = dir.join(name);
        let Some(file) = open_data_file(root, &path, &mut layer, required) else {
            continue;
        };
        if !name.ends_with(".dat") {
            continue;
        }
        let name = relative(root, &path);
        match navdata_file(file, full) {
            Ok(Ok(Some(cycle))) => {
                layer.headers.push(format!("{name}: AIRAC {cycle}"));
                cycles.insert(cycle);
            }
            Ok(Ok(None)) => {}
            Ok(Err(problem)) => layer.problems.push(format!("{name}: {problem}")),
            Err(error) => layer.errors.push(format!("{name}: {error}")),
        }
    }
    if cycles.len() > 1 {
        layer.problems.push(format!(
            "{}: conflicting AIRAC cycles in navigation data headers",
            relative(root, dir)
        ));
    } else {
        layer.consistent_cycle = cycles.into_iter().next();
    }
    inspect_procedures(root, dir, full, &mut layer);
    layer.problems.sort();
    layer.errors.sort();
    layer
}

fn read_small_text(path: &Path) -> io::Result<String> {
    const MAX_BYTES: u64 = 64 * 1024;
    if !fs::metadata(path)?.is_file() {
        return Err(io::Error::other("not a regular file"));
    }
    let mut text = String::new();
    File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > MAX_BYTES {
        return Err(io::Error::other(
            "cycle metadata exceeds the 64 KiB inspection limit",
        ));
    }
    Ok(text)
}

#[derive(Default)]
struct CycleFiles {
    json: Option<PathBuf>,
    info: Option<PathBuf>,
}

fn cycle_metadata(
    root: &Path,
    custom: &Path,
    today: i64,
    simulator_cycle: Option<&str>,
) -> (Vec<NavdataCycleReport>, Vec<String>) {
    let mut files = BTreeMap::<PathBuf, CycleFiles>::new();
    let mut errors = Vec::new();
    match metadata(custom) {
        Ok(None) => return (Vec::new(), errors),
        Ok(Some(meta)) if meta.is_dir() => {}
        Ok(Some(_)) => return (Vec::new(), vec!["Custom Data: not a directory".into()]),
        Err(error) => return (Vec::new(), vec![format!("Custom Data: {error}")]),
    }
    let walk = WalkDir::new(custom)
        .follow_links(true)
        .max_depth(10)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() != 1
                || !["Backup_Data", "CIFP", "1200 atc data"]
                    .iter()
                    .any(|name| entry.file_name().eq_ignore_ascii_case(name))
        });
    for (index, entry) in walk.enumerate() {
        if index == METADATA_ENTRY_LIMIT {
            errors.push(format!(
                "Custom Data: reached the {METADATA_ENTRY_LIMIT} entry metadata limit"
            ));
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(error.to_string());
                continue;
            }
        };
        let path = entry.path();
        let Some(parent) = path.parent() else {
            continue;
        };
        if entry.file_name().eq_ignore_ascii_case("cycle.json") {
            files.entry(parent.to_path_buf()).or_default().json = Some(path.to_path_buf());
        } else if entry.file_name().eq_ignore_ascii_case("cycle_info.txt") {
            files.entry(parent.to_path_buf()).or_default().info = Some(path.to_path_buf());
        }
    }

    let mut cycles = Vec::new();
    for (folder, files) in files {
        let json = files
            .json
            .as_ref()
            .and_then(|path| match read_cycle_json(path) {
                Ok(value) => Some(value),
                Err(error) => {
                    errors.push(format!("{}: {error}", relative(root, path)));
                    None
                }
            });
        let info = files
            .info
            .as_ref()
            .and_then(|path| match read_small_text(path) {
                Ok(body) => Some(body),
                Err(error) => {
                    errors.push(format!("{}: {error}", relative(root, path)));
                    None
                }
            });
        if json.is_none() && info.is_none() {
            continue;
        }
        let (provider_name, json_cycle, airac) = json.unwrap_or_default();
        let json_cycle = airac.as_deref().or(json_cycle.as_deref());
        let info_cycle = info.as_deref().and_then(|body| {
            body.lines()
                .filter_map(|line| line.split_once(':'))
                .find_map(|(key, value)| {
                    let value = value.trim();
                    (key.trim().eq_ignore_ascii_case("AIRAC cycle")
                        && value.len() == 4
                        && value.bytes().all(|byte| byte.is_ascii_digit()))
                    .then_some(value)
                })
        });
        let data_cycle = simulator_cycle.filter(|_| folder == custom);
        let cycle = data_cycle.or(info_cycle).or(json_cycle);
        let claims = [
            ("data headers", data_cycle),
            ("cycle_info.txt", info_cycle),
            ("cycle.json", json_cycle),
        ];
        let metadata_conflict = claims
            .iter()
            .any(|(_, claim)| claim.is_some() && *claim != cycle)
            .then(|| {
                let details = claims
                    .into_iter()
                    .filter_map(|(label, claim)| {
                        claim.map(|claim| format!("{label} AIRAC {claim}"))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{}: conflicting AIRAC metadata ({details})",
                    relative(root, &folder)
                )
            });
        let validity = info.as_deref().and_then(parse_validity_range).filter(|_| {
            info_cycle
                .or(json_cycle)
                .is_none_or(|dated_cycle| Some(dated_cycle) == cycle)
        });
        let (effective_date, expiry_date, days_remaining, status) = match validity {
            Some((from, to)) => {
                let (status, days) = if from > today {
                    (NavdataStatus::NotYetEffective, to - today)
                } else {
                    status_for(to, today)
                };
                (
                    Some(epoch_day_to_ymd_string(from)),
                    Some(epoch_day_to_ymd_string(to)),
                    Some(days),
                    status,
                )
            }
            None => (None, None, None, NavdataStatus::Unknown),
        };
        cycles.push(NavdataCycleReport {
            folder_name: relative(root, &folder),
            provider_name: provider_name.unwrap_or_else(|| "Unknown".into()),
            cycle: cycle.map(str::to_owned),
            airac,
            effective_date,
            expiry_date,
            days_remaining,
            status,
            metadata_conflict,
            source: [files.json, files.info]
                .into_iter()
                .flatten()
                .map(|path| relative(root, &path))
                .collect::<Vec<_>>()
                .join(", "),
        });
    }
    (cycles, errors)
}

pub fn inspect(root: &Path, full: bool) -> DoctorNavdataReport {
    let custom = root.join("Custom Data");
    let bundled = inspect_base(root, &root.join("Resources/default data"), full);
    let mut supplemental = NavdataLayer::default();
    let mut arinc_present = false;
    for name in SUPPLEMENTAL_FILES {
        open_data_file(root, &custom.join(name), &mut supplemental, false);
        if *name == "earth_424.dat" {
            arinc_present = supplemental.present;
        }
    }
    let mut override_data = NavdataLayer::default();
    if !arinc_present {
        let mut base_present = false;
        for name in CORE_FILES.iter().chain(EXTRA_FILES).copied().chain([
            "CIFP",
            "cycle.json",
            "cycle_info.txt",
        ]) {
            let path = custom.join(name);
            match metadata(&path) {
                Ok(Some(_)) => base_present = true,
                Ok(None) => {}
                Err(error) => override_data
                    .errors
                    .push(format!("{}: {error}", relative(root, &path))),
            }
        }
        if base_present {
            override_data = inspect_base(root, &custom, full);
        }
    }
    let (cycles, metadata_errors) = cycle_metadata(
        root,
        &custom,
        today_epoch_day(),
        override_data.consistent_cycle.as_deref(),
    );
    DoctorNavdataReport {
        bundled,
        override_data,
        supplemental,
        cycles,
        metadata_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, body: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    #[test]
    fn full_navdata_check_catches_a_truncated_file_with_a_valid_header() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let relative = "Resources/default data/earth_nav.dat";
        write(
            root,
            relative,
            "I\n1200 Version - data cycle 2611\n3 valid start of record\n",
        );
        assert!(
            navdata_file(File::open(root.join(relative)).unwrap(), false)
                .unwrap()
                .is_ok()
        );
        assert_eq!(
            navdata_file(File::open(root.join(relative)).unwrap(), true).unwrap(),
            Err("missing final 99 record; file may be truncated")
        );
    }

    #[test]
    fn checks_bundled_headers_and_procedures_independently_of_optional_data() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(
            root,
            "Resources/default data/earth_fix.dat",
            "I\n1200 Version - data cycle 2512, build 20251101\n1 sample record\n99\n",
        );
        write(
            root,
            "Resources/default data/earth_awy.dat",
            "A\n1100 Version - data cycle 2511, build 20251001\n1 sample record\n99\n",
        );
        write(
            root,
            "Resources/default data/earth_nav.dat",
            "not a data header",
        );
        write(root, "Resources/default data/CIFP/GOOD.dat", "SID:...");
        write(root, "Resources/default data/CIFP/FAIL.dat", "");
        let quick = inspect(root, false);
        assert_eq!(quick.bundled.problems.len(), 2);
        assert!(quick
            .bundled
            .problems
            .iter()
            .any(|issue| issue.contains("conflicting AIRAC")));
        assert!(quick
            .bundled
            .problems
            .iter()
            .any(|issue| issue.contains("invalid X-Plane data header")));
        assert!(!quick.override_data.present);
        let full = inspect(root, true);
        assert!(full
            .bundled
            .problems
            .iter()
            .any(|issue| issue.ends_with("FAIL.dat: empty file")));
    }

    #[test]
    fn detects_partial_custom_base_without_requiring_it_for_arinc_and_user_data() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Custom Data/user_fix.dat", "");
        write(root, "Custom Data/FAACIFP18", "FAA data");
        write(
            root,
            "Custom Data/GNS430/navdata/cycle.json",
            r#"{"cycle":2610}"#,
        );
        let report = inspect(root, false);
        assert!(!report.override_data.present);
        assert_eq!(
            report.supplemental.problems,
            ["Custom Data/user_fix.dat: empty file"]
        );
        assert_eq!(report.cycles.len(), 1);

        write(root, "Custom Data/earth_nav.dat", "");
        let report = inspect(root, false);
        assert!(report.override_data.present);
        assert!(report
            .override_data
            .problems
            .iter()
            .any(|issue| issue.ends_with("earth_fix.dat: missing")));

        write(root, "Custom Data/earth_424.dat", "ARINC data");
        let report = inspect(root, false);
        assert!(!report.override_data.present);
    }

    #[test]
    fn reads_standalone_cycle_info_and_reports_bad_json_without_including_backups() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(
            root,
            "Custom Data/Provider/CYCLE_INFO.TXT",
            "AIRAC cycle: 2605\nValid (from/to): 14/MAY/2026 - 11/JUN/2026\n",
        );
        write(root, "Custom Data/Other/cycle.json", "{");
        write(root, "Custom Data/Backup_Data/cycle.json", "{");
        let (cycles, errors) = cycle_metadata(
            root,
            &root.join("Custom Data"),
            ymd_to_epoch_day(2026, 5, 1),
            None,
        );
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].cycle.as_deref(), Some("2605"));
        assert_eq!(cycles[0].source, "Custom Data/Provider/CYCLE_INFO.TXT");
        assert_eq!(cycles[0].status, NavdataStatus::NotYetEffective);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("Custom Data/Other/cycle.json:"));
    }

    #[test]
    fn associates_dates_with_the_cycle_in_the_data_headers() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Custom Data/cycle.json", r#"{"cycle":"2408"}"#);
        write(
            root,
            "Custom Data/cycle_info.txt",
            "AIRAC cycle: 2608\nValid (from/to): 06/AUG/2026 - 03/SEP/2026\n",
        );
        let today = ymd_to_epoch_day(2026, 11, 7);
        let custom = root.join("Custom Data");
        let (cycles, errors) = cycle_metadata(root, &custom, today, Some("2608"));
        assert!(errors.is_empty());
        assert_eq!(cycles[0].cycle.as_deref(), Some("2608"));
        assert_eq!(cycles[0].expiry_date.as_deref(), Some("2026-09-03"));
        assert_eq!(cycles[0].status, NavdataStatus::Expired);
        assert!(cycles[0]
            .metadata_conflict
            .as_deref()
            .unwrap()
            .contains("cycle.json AIRAC 2408"));

        let (cycles, _) = cycle_metadata(root, &custom, today, Some("2611"));
        assert_eq!(cycles[0].cycle.as_deref(), Some("2611"));
        assert!(cycles[0].expiry_date.is_none());
        assert_eq!(cycles[0].status, NavdataStatus::Unknown);
    }

    #[test]
    fn cycle_json_accepts_numeric_cycle_values() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("cycle.json");
        fs::write(
            &path,
            r#"{"cycle":2408,"airac":2408,"revision":1,"name":"XPNavData"}"#,
        )
        .unwrap();

        let (provider, cycle, airac) = read_cycle_json(&path).expect("cycle.json should parse");
        assert_eq!(provider.as_deref(), Some("XPNavData"));
        assert_eq!(cycle.as_deref(), Some("2408"));
        assert_eq!(airac.as_deref(), Some("2408"));
    }

    #[test]
    fn ymd_roundtrip() {
        for (y, m, d) in [(2026, 5, 14), (2026, 6, 11), (2000, 1, 1), (1970, 1, 1)] {
            let ed = ymd_to_epoch_day(y, m, d);
            assert_eq!(
                epoch_day_to_ymd_string(ed),
                format!("{:04}-{:02}-{:02}", y, m, d)
            );
        }
    }

    #[test]
    fn unix_epoch_is_zero() {
        assert_eq!(ymd_to_epoch_day(1970, 1, 1), 0);
    }

    #[test]
    fn parse_dmy_basic() {
        assert_eq!(
            parse_dmy("14/MAY/2026"),
            Some(ymd_to_epoch_day(2026, 5, 14))
        );
        assert_eq!(
            parse_dmy(" 11/JUN/2026 "),
            Some(ymd_to_epoch_day(2026, 6, 11))
        );
        assert_eq!(parse_dmy("bad"), None);
        assert_eq!(parse_dmy("40/MAY/2026"), None);
        assert_eq!(parse_dmy("31/APR/2026"), None);
        assert_eq!(parse_dmy("29/FEB/2025"), None);
        assert_eq!(
            parse_dmy("29/FEB/2024"),
            Some(ymd_to_epoch_day(2024, 2, 29))
        );
    }

    #[test]
    fn parse_validity_from_real_cycle_info() {
        let body = "AIRAC cycle    : 2605\nVersion        : 1\nValid (from/to): 14/MAY/2026 - 11/JUN/2026\n";
        let (from, to) = parse_validity_range(body).expect("should parse");
        assert_eq!(epoch_day_to_ymd_string(from), "2026-05-14");
        assert_eq!(epoch_day_to_ymd_string(to), "2026-06-11");
    }

    #[test]
    fn status_thresholds() {
        let today = ymd_to_epoch_day(2026, 6, 7);
        // expiry 2026-06-11 -> 4 days -> expiring soon
        let (s, d) = status_for(ymd_to_epoch_day(2026, 6, 11), today);
        assert_eq!(s, NavdataStatus::ExpiringSoon);
        assert_eq!(d, 4);
        // expiry in the past -> expired
        let (s, _) = status_for(ymd_to_epoch_day(2026, 6, 1), today);
        assert_eq!(s, NavdataStatus::Expired);
        // expiry far future -> ok
        let (s, _) = status_for(ymd_to_epoch_day(2026, 7, 30), today);
        assert_eq!(s, NavdataStatus::Ok);
    }

    #[test]
    fn expiring_soon_serializes_with_the_frontend_contract() {
        assert_eq!(
            serde_json::to_string(&NavdataStatus::ExpiringSoon).unwrap(),
            "\"expiring_soon\""
        );
    }
}

//! Doctor — environment & navdata detection.
//!
//! This module provides the *new* detection signals the Doctor page needs that
//! are not already derivable from existing commands. It is intentionally
//! limited to read-only inspection: it never mutates the filesystem. All
//! remediation is performed by the existing, already-audited commands
//! (`sort_scenery_packs`, `clean_output_items`, `airport_flatten_apply_all_drifted`,
//! addon/gateway updaters, …) orchestrated from the frontend Doctor store.

use std::fs;
use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;

use crate::logger;

const LOG_CTX: &str = "doctor";

// ============================================================================
// Environment report
// ============================================================================

/// A detected third-party graphics injector (ReShade / ENB / overlay proxy
/// DLLs) sitting next to the X-Plane binary. X-Plane blocks these and they are
/// a leading cause of unexplained CTDs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedInjector {
    /// Stable id for i18n/UI (e.g. "reshade", "overlay_proxy").
    pub id: String,
    /// The on-disk artifact that triggered detection, relative to the X-Plane root.
    pub evidence: String,
}

/// A competing scenery-organizer install that may rewrite `scenery_packs.ini`
/// behind XFast's back (e.g. xOrganizer).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetingOrganizer {
    pub id: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorEnvironmentReport {
    /// Free bytes on the volume hosting the X-Plane install.
    pub free_bytes: u64,
    /// Total bytes on that volume.
    pub total_bytes: u64,
    /// Whether the install lives under a UAC-restricted Program Files path.
    pub in_program_files: bool,
    /// Whether this looks like a Steam install (steam_api dll / steamapps path).
    pub is_steam_install: bool,
    /// Core top-level directories that are expected but missing (broken install).
    pub missing_core_dirs: Vec<String>,
    /// Count of read-only files found under the addon directories (capped scan).
    /// A high count can block installs/updates on Windows.
    pub readonly_count: usize,
    /// Whether the read-only scan was truncated by the cap (count is a lower bound).
    pub readonly_scan_capped: bool,
    /// Detected graphics injectors (ReShade etc.).
    pub injectors: Vec<DetectedInjector>,
    /// Competing scenery organizers (xOrganizer etc.).
    pub competing_organizers: Vec<CompetingOrganizer>,
}

/// Top-level directories X-Plane needs to run. Their absence indicates a broken
/// or wrongly-pointed install.
const CORE_DIRS: &[&str] = &["Resources", "Aircraft", "Custom Scenery", "Global Scenery"];

/// Cap the read-only scan so a huge install does not stall the Doctor.
const READONLY_SCAN_FILE_CAP: usize = 60_000;
/// Directories scanned for read-only files (the ones installs/updates write to).
const READONLY_SCAN_DIRS: &[&str] = &["Aircraft", "Resources/plugins", "Custom Scenery"];

fn path_is_in_program_files(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    lower.contains("program files")
}

fn detect_steam_install(root: &Path) -> bool {
    let lower = root.to_string_lossy().to_lowercase();
    if lower.contains("steamapps") {
        return true;
    }
    root.join("steam_api64.dll").exists()
        || root.join("steam_api.dll").exists()
        || root.join("steam_appid.txt").exists()
}

/// Known injector artifacts that sit in the X-Plane root. The proxy DLLs
/// (dxgi/d3d11/d3d12/opengl32/vulkan-1) only count as injectors when a ReShade
/// sidecar (ReShade.ini / reshade-shaders) is also present, to avoid flagging
/// legitimate system libraries that some setups copy in.
fn detect_injectors(root: &Path) -> Vec<DetectedInjector> {
    let mut found: Vec<DetectedInjector> = Vec::new();

    let reshade_ini = root.join("ReShade.ini").exists();
    let reshade_shaders = root.join("reshade-shaders").is_dir();
    if reshade_ini || reshade_shaders {
        let evidence = if reshade_ini {
            "ReShade.ini"
        } else {
            "reshade-shaders/"
        };
        found.push(DetectedInjector {
            id: "reshade".to_string(),
            evidence: evidence.to_string(),
        });

        // Only attribute proxy DLLs when ReShade is confirmed present.
        for proxy in ["dxgi.dll", "d3d11.dll", "d3d12.dll", "opengl32.dll"] {
            if root.join(proxy).exists() {
                found.push(DetectedInjector {
                    id: "injector_proxy_dll".to_string(),
                    evidence: proxy.to_string(),
                });
                break;
            }
        }
    }

    found
}

/// Known competing scenery-organizer artifacts.
fn detect_competing_organizers(root: &Path) -> Vec<CompetingOrganizer> {
    let mut found: Vec<CompetingOrganizer> = Vec::new();

    // xOrganizer ships as a folder in the X-Plane root.
    for candidate in ["xOrganizer", "xOrganizer.exe", "xOrganizer.jar"] {
        let p = root.join(candidate);
        if p.exists() {
            found.push(CompetingOrganizer {
                id: "xorganizer".to_string(),
                evidence: candidate.to_string(),
            });
            break;
        }
    }

    found
}

fn count_readonly_files(root: &Path) -> (usize, bool) {
    let mut count = 0usize;
    let mut scanned = 0usize;
    let mut capped = false;

    'outer: for sub in READONLY_SCAN_DIRS {
        let dir = root.join(sub);
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir).follow_links(false).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            scanned += 1;
            if scanned > READONLY_SCAN_FILE_CAP {
                capped = true;
                break 'outer;
            }
            if let Ok(meta) = entry.metadata() {
                if meta.permissions().readonly() {
                    count += 1;
                }
            }
        }
    }

    (count, capped)
}

pub fn scan_environment(xplane_path: &str) -> DoctorEnvironmentReport {
    let root = Path::new(xplane_path);

    let free_bytes = fs2::available_space(root).unwrap_or(0);
    let total_bytes = fs2::total_space(root).unwrap_or(0);

    let missing_core_dirs: Vec<String> = CORE_DIRS
        .iter()
        .filter(|d| !root.join(d).is_dir())
        .map(|d| d.to_string())
        .collect();

    let (readonly_count, readonly_scan_capped) = count_readonly_files(root);

    let report = DoctorEnvironmentReport {
        free_bytes,
        total_bytes,
        in_program_files: path_is_in_program_files(root),
        is_steam_install: detect_steam_install(root),
        missing_core_dirs,
        readonly_count,
        readonly_scan_capped,
        injectors: detect_injectors(root),
        competing_organizers: detect_competing_organizers(root),
    };

    logger::log_info(
        &format!(
            "Environment scan: free={}GB injectors={} organizers={} readonly={} missingCore={}",
            report.free_bytes / 1_073_741_824,
            report.injectors.len(),
            report.competing_organizers.len(),
            report.readonly_count,
            report.missing_core_dirs.len()
        ),
        Some(LOG_CTX),
    );

    report
}

// ============================================================================
// Navdata report (AIRAC validity)
// ============================================================================

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NavdataStatus {
    Ok,
    ExpiringSoon,
    Expired,
    /// Validity dates could not be determined (only cycle.json, no cycle_info.txt).
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavdataCycleReport {
    /// Folder under Custom Data (empty string for the root cycle.json).
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
    /// Where validity came from: "cycle_info" | "computed" | "none".
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorNavdataReport {
    pub cycles: Vec<NavdataCycleReport>,
    /// Whether Custom Data exists at all.
    pub custom_data_exists: bool,
    /// Whether the CIFP (procedures) folder exists and is non-empty.
    pub cifp_present: bool,
    /// Core earth_*.dat files missing from Custom Data root (only meaningful when
    /// the user has placed a full navdata set there).
    pub earth_dat_missing: Vec<String>,
}

const EXPIRING_SOON_DAYS: i64 = 7;
const EARTH_DAT_FILES: &[&str] = &[
    "earth_nav.dat",
    "earth_fix.dat",
    "earth_awy.dat",
    "earth_aptmeta.dat",
];

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
    let yoe = (y - era * 400) as i64; // [0, 399]
    let mp = ((m as i64 + 9) % 12) as i64; // [0, 11]
    let doy = (153 * mp + 2) / 5 + d as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

/// Format an epoch-day count back to "YYYY-MM-DD".
fn epoch_day_to_ymd_string(z: i64) -> String {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as i64;
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

/// Parse a "DD/MMM/YYYY" token (e.g. "14/MAY/2026") to epoch day.
fn parse_dmy(token: &str) -> Option<i64> {
    let parts: Vec<&str> = token.trim().split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let d: u32 = parts[0].trim().parse().ok()?;
    let m = month_from_abbrev(parts[1].trim())?;
    let y: i64 = parts[2].trim().parse().ok()?;
    if d == 0 || d > 31 {
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
        let after = line.splitn(2, ':').nth(1).unwrap_or(line);
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

/// Read provider/cycle/airac from a cycle.json next to a navdata set.
fn read_cycle_json(path: &Path) -> Option<(String, Option<String>, Option<String>)> {
    let content = fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let provider = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();
    let cycle = json
        .get("cycle")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let airac = json
        .get("airac")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    Some((provider, cycle, airac))
}

pub fn navdata_status(xplane_path: &str) -> DoctorNavdataReport {
    let custom_data = Path::new(xplane_path).join("Custom Data");
    let custom_data_exists = custom_data.is_dir();

    let today = today_epoch_day();
    let mut cycles: Vec<NavdataCycleReport> = Vec::new();

    if custom_data_exists {
        // Find every cycle.json under Custom Data (skip Backup_Data).
        let backup = custom_data.join("Backup_Data");
        for entry in WalkDir::new(&custom_data)
            .max_depth(10)
            .into_iter()
            .filter_entry(|e| e.path() != backup)
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            if !entry
                .file_name()
                .to_str()
                .map(|n| n.eq_ignore_ascii_case("cycle.json"))
                .unwrap_or(false)
            {
                continue;
            }

            let path = entry.path();
            let parent = match path.parent() {
                Some(p) => p,
                None => continue,
            };
            let Some((provider_name, cycle, airac)) = read_cycle_json(path) else {
                continue;
            };

            let folder_name = parent
                .strip_prefix(&custom_data)
                .unwrap_or(parent)
                .to_string_lossy()
                .to_string();

            // PRIMARY: cycle_info.txt validity range next to the cycle.json.
            let validity = ["cycle_info.txt", "Cycle_Info.txt"]
                .iter()
                .map(|f| parent.join(f))
                .find(|p| p.is_file())
                .and_then(|p| fs::read_to_string(&p).ok())
                .and_then(|body| parse_validity_range(&body));

            let (effective_date, expiry_date, days_remaining, status, source) = match validity {
                Some((from, to)) => {
                    let (status, days) = status_for(to, today);
                    (
                        Some(epoch_day_to_ymd_string(from)),
                        Some(epoch_day_to_ymd_string(to)),
                        Some(days),
                        status,
                        "cycle_info".to_string(),
                    )
                }
                None => (None, None, None, NavdataStatus::Unknown, "none".to_string()),
            };

            cycles.push(NavdataCycleReport {
                folder_name,
                provider_name,
                cycle,
                airac,
                effective_date,
                expiry_date,
                days_remaining,
                status,
                source,
            });
        }
    }

    // Sort: worst status first, then by folder for stability.
    cycles.sort_by(|a, b| {
        fn rank(s: &NavdataStatus) -> u8 {
            match s {
                NavdataStatus::Expired => 0,
                NavdataStatus::ExpiringSoon => 1,
                NavdataStatus::Unknown => 2,
                NavdataStatus::Ok => 3,
            }
        }
        rank(&a.status)
            .cmp(&rank(&b.status))
            .then_with(|| a.folder_name.cmp(&b.folder_name))
    });

    let cifp_dir = custom_data.join("CIFP");
    let cifp_present = cifp_dir.is_dir()
        && fs::read_dir(&cifp_dir)
            .map(|mut it| it.next().is_some())
            .unwrap_or(false);

    // earth_*.dat are only "missing" if the user placed at least one of them
    // in Custom Data root (i.e. they intend a full override set there).
    let any_earth_present = EARTH_DAT_FILES
        .iter()
        .any(|f| custom_data.join(f).is_file());
    let earth_dat_missing: Vec<String> = if custom_data_exists && any_earth_present {
        EARTH_DAT_FILES
            .iter()
            .filter(|f| !custom_data.join(f).is_file())
            .map(|f| f.to_string())
            .collect()
    } else {
        Vec::new()
    };

    DoctorNavdataReport {
        cycles,
        custom_data_exists,
        cifp_present,
        earth_dat_missing,
    }
}

// ========== Tauri Commands ==========

#[tauri::command]
pub async fn doctor_scan_environment(xplane_path: String) -> Result<DoctorEnvironmentReport, String> {
    tokio::task::spawn_blocking(move || scan_environment(&xplane_path))
        .await
        .map_err(|e| format!("Task join error: {}", e))
}

#[tauri::command]
pub async fn doctor_navdata_status(xplane_path: String) -> Result<DoctorNavdataReport, String> {
    tokio::task::spawn_blocking(move || navdata_status(&xplane_path))
        .await
        .map_err(|e| format!("Task join error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ymd_roundtrip() {
        for (y, m, d) in [(2026, 5, 14), (2026, 6, 11), (2000, 1, 1), (1970, 1, 1)] {
            let ed = ymd_to_epoch_day(y, m, d);
            assert_eq!(epoch_day_to_ymd_string(ed), format!("{:04}-{:02}-{:02}", y, m, d));
        }
    }

    #[test]
    fn unix_epoch_is_zero() {
        assert_eq!(ymd_to_epoch_day(1970, 1, 1), 0);
    }

    #[test]
    fn parse_dmy_basic() {
        assert_eq!(parse_dmy("14/MAY/2026"), Some(ymd_to_epoch_day(2026, 5, 14)));
        assert_eq!(parse_dmy(" 11/JUN/2026 "), Some(ymd_to_epoch_day(2026, 6, 11)));
        assert_eq!(parse_dmy("bad"), None);
        assert_eq!(parse_dmy("40/MAY/2026"), None);
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
}

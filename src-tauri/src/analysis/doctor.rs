//! Doctor — environment & navdata detection.
//!
//! This module provides the *new* detection signals the Doctor page needs that
//! are not already derivable from existing commands. It is intentionally
//! limited to read-only inspection: it never mutates the filesystem. All
//! remediation is performed by the existing, already-audited commands
//! (`sort_scenery_packs`, `clean_output_items`, `airport_flatten_apply_all_drifted`,
//! addon/gateway updaters, …) orchestrated from the frontend Doctor store.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sysinfo::System;
use walkdir::WalkDir;

use crate::{app_dirs, database, logger};

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
    /// Stable, non-reversible identifier for this X-Plane installation.
    pub installation_id: String,
    pub root_exists: bool,
    pub executable_present: bool,
    pub log_present: bool,
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
    /// Quick scans intentionally skip the recursive read-only inspection.
    pub readonly_scan_performed: bool,
    /// Detected graphics injectors (ReShade etc.).
    pub injectors: Vec<DetectedInjector>,
    /// Competing scenery organizers (xOrganizer etc.).
    pub competing_organizers: Vec<CompetingOrganizer>,
    pub system: DoctorSystemSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorSystemSnapshot {
    pub os: String,
    pub os_version: Option<String>,
    pub architecture: String,
    pub cpu_model: Option<String>,
    pub logical_cores: usize,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorSceneryIndexHealth {
    pub index_exists: bool,
    pub indexed_count: usize,
    pub filesystem_count: usize,
    pub missing_from_index: Vec<String>,
    pub missing_from_disk: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorXfastHealthReport {
    pub app_data_dir: String,
    pub app_data_writable: bool,
    pub app_data_write_error: Option<String>,
    pub app_data_free_bytes: u64,
    pub app_data_total_bytes: u64,
    pub database_ok: bool,
    pub database_detail: Option<String>,
    pub schema_compatible: bool,
    pub scenery_index: DoctorSceneryIndexHealth,
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

fn installation_id(root: &Path) -> String {
    let resolved = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut normalized = resolved.to_string_lossy().replace('\\', "/");
    if cfg!(target_os = "windows") {
        normalized.make_ascii_lowercase();
    }
    let digest = Sha256::digest(normalized.as_bytes());
    format!("{:x}", digest)
}

fn collect_system_snapshot() -> DoctorSystemSnapshot {
    let mut system = System::new_all();
    system.refresh_all();

    DoctorSystemSnapshot {
        os: System::name().unwrap_or_else(|| std::env::consts::OS.to_string()),
        os_version: System::os_version(),
        architecture: std::env::consts::ARCH.to_string(),
        cpu_model: system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|value| !value.is_empty()),
        logical_cores: system.cpus().len(),
        total_memory_bytes: system.total_memory(),
        available_memory_bytes: system.available_memory(),
    }
}

pub fn scan_environment_with_depth(xplane_path: &str, full: bool) -> DoctorEnvironmentReport {
    let root = Path::new(xplane_path);

    let free_bytes = fs2::available_space(root).unwrap_or(0);
    let total_bytes = fs2::total_space(root).unwrap_or(0);

    let missing_core_dirs: Vec<String> = CORE_DIRS
        .iter()
        .filter(|d| !root.join(d).is_dir())
        .map(|d| d.to_string())
        .collect();

    let (readonly_count, readonly_scan_capped) = if full {
        count_readonly_files(root)
    } else {
        (0, false)
    };

    let report = DoctorEnvironmentReport {
        installation_id: installation_id(root),
        root_exists: root.is_dir(),
        executable_present: super::find_xplane_executable_in_root(root).is_some(),
        log_present: root.join("Log.txt").is_file(),
        free_bytes,
        total_bytes,
        in_program_files: path_is_in_program_files(root),
        is_steam_install: detect_steam_install(root),
        missing_core_dirs,
        readonly_count,
        readonly_scan_capped,
        readonly_scan_performed: full,
        injectors: detect_injectors(root),
        competing_organizers: detect_competing_organizers(root),
        system: collect_system_snapshot(),
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

fn scenery_folder_names(custom_scenery: &Path) -> HashSet<String> {
    let Ok(entries) = fs::read_dir(custom_scenery) else {
        return HashSet::new();
    };

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path
                .metadata()
                .map(|metadata| metadata.is_dir())
                .unwrap_or(false)
            {
                return entry.file_name().into_string().ok();
            }
            if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
            {
                return path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .map(str::to_string);
            }
            None
        })
        .collect()
}

fn compare_scenery_index(
    filesystem: HashSet<String>,
    indexed: HashSet<String>,
) -> DoctorSceneryIndexHealth {
    let mut missing_from_index: Vec<String> = filesystem.difference(&indexed).cloned().collect();
    let mut missing_from_disk: Vec<String> = indexed.difference(&filesystem).cloned().collect();
    missing_from_index.sort();
    missing_from_disk.sort();

    DoctorSceneryIndexHealth {
        index_exists: !indexed.is_empty(),
        indexed_count: indexed.len(),
        filesystem_count: filesystem.len(),
        missing_from_index,
        missing_from_disk,
    }
}

fn probe_app_data_writable(app_data_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(app_data_dir).map_err(|error| error.to_string())?;
    tempfile::NamedTempFile::new_in(app_data_dir)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub async fn scan_xfast_health(
    xplane_path: &str,
    conn: &sea_orm::DatabaseConnection,
) -> DoctorXfastHealthReport {
    let app_data_dir = app_dirs::get_app_data_dir();
    let app_data_write_result = probe_app_data_writable(&app_data_dir);
    let app_data_free_bytes = fs2::available_space(&app_data_dir).unwrap_or(0);
    let app_data_total_bytes = fs2::total_space(&app_data_dir).unwrap_or(0);

    let database_result = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "PRAGMA quick_check".to_string(),
        ))
        .await;
    let (database_ok, database_detail) = match database_result {
        Ok(Some(row)) => {
            let detail = row
                .try_get_by_index::<String>(0)
                .unwrap_or_else(|error| error.to_string());
            (detail.eq_ignore_ascii_case("ok"), Some(detail))
        }
        Ok(None) => (
            false,
            Some("PRAGMA quick_check returned no result".to_string()),
        ),
        Err(error) => (false, Some(error.to_string())),
    };

    let schema_compatible = database::is_schema_compatible(conn).await.unwrap_or(false);
    let indexed = if schema_compatible {
        database::SceneryQueries::load_all(conn)
            .await
            .map(|index| {
                index
                    .packages
                    .into_keys()
                    .filter(|name| name != crate::models::GLOBAL_AIRPORTS_ENTRY_NAME)
                    .collect()
            })
            .unwrap_or_default()
    } else {
        HashSet::new()
    };
    let filesystem = scenery_folder_names(&Path::new(xplane_path).join("Custom Scenery"));

    DoctorXfastHealthReport {
        app_data_dir: app_data_dir.to_string_lossy().to_string(),
        app_data_writable: app_data_write_result.is_ok(),
        app_data_write_error: app_data_write_result.err(),
        app_data_free_bytes,
        app_data_total_bytes,
        database_ok,
        database_detail,
        schema_compatible,
        scenery_index: compare_scenery_index(filesystem, indexed),
    }
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
pub async fn doctor_scan_environment(
    xplane_path: String,
    depth: Option<String>,
) -> Result<DoctorEnvironmentReport, String> {
    let full = depth.as_deref() == Some("full");
    tokio::task::spawn_blocking(move || scan_environment_with_depth(&xplane_path, full))
        .await
        .map_err(|e| format!("Task join error: {}", e))
}

#[tauri::command]
pub async fn doctor_scan_xfast_health(
    db: tauri::State<'_, crate::database::DatabaseState>,
    xplane_path: String,
) -> Result<DoctorXfastHealthReport, String> {
    Ok(scan_xfast_health(&xplane_path, &db.get()).await)
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

    #[test]
    fn quick_environment_scan_skips_recursive_readonly_work() {
        let temp = tempfile::tempdir().unwrap();
        let report = scan_environment_with_depth(&temp.path().display().to_string(), false);
        assert!(!report.readonly_scan_performed);
        assert_eq!(report.readonly_count, 0);
    }

    #[test]
    fn installation_identifier_is_stable_and_path_specific() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        assert_eq!(installation_id(first.path()), installation_id(first.path()));
        assert_ne!(
            installation_id(first.path()),
            installation_id(second.path())
        );
        assert_eq!(installation_id(first.path()).len(), 64);
    }

    #[test]
    fn scenery_index_comparison_reports_both_directions() {
        let filesystem = HashSet::from(["Present".to_string(), "New".to_string()]);
        let indexed = HashSet::from(["Present".to_string(), "Deleted".to_string()]);
        let report = compare_scenery_index(filesystem, indexed);
        assert_eq!(report.missing_from_index, vec!["New"]);
        assert_eq!(report.missing_from_disk, vec!["Deleted"]);
    }

    #[test]
    fn app_data_write_probe_cleans_up_after_itself() {
        let temp = tempfile::tempdir().unwrap();
        probe_app_data_writable(temp.path()).unwrap();
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    }
}

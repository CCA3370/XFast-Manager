//! Doctor — environment & navdata detection.
//!
//! Scans inspect installation evidence without modifying simulator files.
//! Explicit repairs use fixed targets and restorable local backups; scenery
//! changes reuse the existing management commands.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sysinfo::System;
use walkdir::WalkDir;

use crate::{app_dirs, database, logger};

#[path = "doctor_inspection.rs"]
mod inspection;
#[path = "doctor_navdata.rs"]
mod navdata;
#[path = "doctor_repairs.rs"]
mod repairs;

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
    if !root.join("ReShade.ini").is_file() && !root.join("reshade-shaders").is_dir() {
        return Vec::new();
    }
    repairs::INJECTOR_DLLS
        .iter()
        .filter(|name| root.join(name).is_file())
        .map(|name| DetectedInjector {
            id: "reshade_proxy".to_string(),
            evidence: name.to_string(),
        })
        .collect()
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

// ========== Bounded inspection and reversible repairs ==========

#[tauri::command]
pub async fn doctor_inspect_installation(
    xplane_path: String,
    depth: String,
) -> Result<inspection::InspectionReport, String> {
    tokio::task::spawn_blocking(move || {
        inspection::inspect_installation(
            Path::new(&xplane_path),
            depth == "full",
            super::scenery_index::resolve_shortcut,
        )
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn doctor_list_backups(
    xplane_path: String,
) -> Result<Vec<repairs::RepairBackup>, String> {
    tokio::task::spawn_blocking(move || {
        let _guard = repairs::REPAIR_LOCK.lock().map_err(|e| e.to_string())?;
        super::validate_xplane_root_path(Path::new(&xplane_path))?;
        repairs::list_backups(Path::new(&xplane_path))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn doctor_apply_repair(
    xplane_path: String,
    repair: String,
) -> Result<repairs::RepairBackup, String> {
    if super::is_xplane_running().await {
        return Err("Close X-Plane before applying Health repairs".into());
    }
    tokio::task::spawn_blocking(move || {
        let _guard = repairs::REPAIR_LOCK.lock().map_err(|e| e.to_string())?;
        super::validate_xplane_root_path(Path::new(&xplane_path))?;
        repairs::apply_repair(Path::new(&xplane_path), &repair)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn doctor_restore_backup(xplane_path: String, backup_id: String) -> Result<(), String> {
    if super::is_xplane_running().await {
        return Err("Close X-Plane before restoring Health backups".into());
    }
    tokio::task::spawn_blocking(move || {
        let _guard = repairs::REPAIR_LOCK.lock().map_err(|e| e.to_string())?;
        super::validate_xplane_root_path(Path::new(&xplane_path))?;
        repairs::restore_backup(Path::new(&xplane_path), &backup_id)
    })
    .await
    .map_err(|e| e.to_string())?
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

fn export_report(path: &Path, content: &str) -> Result<(), String> {
    const MAX_REPORT_BYTES: usize = 2 * 1024 * 1024;
    if content.len() > MAX_REPORT_BYTES {
        return Err("Health report is too large to export".to_string());
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("md") && !extension.eq_ignore_ascii_case("json") {
        return Err("Health reports must use a .md or .json extension".to_string());
    }
    let parent = path
        .parent()
        .filter(|parent| parent.is_dir())
        .ok_or_else(|| "Health report destination folder does not exist".to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    use std::io::Write;
    temp.write_all(content.as_bytes())
        .map_err(|error| error.to_string())?;
    temp.flush().map_err(|error| error.to_string())?;
    temp.persist(path)
        .map(|_| ())
        .map_err(|error| error.error.to_string())
}

#[tauri::command]
pub async fn doctor_export_report(export_path: String, content: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || export_report(Path::new(&export_path), &content))
        .await
        .map_err(|error| format!("Task join error: {}", error))?
}

#[tauri::command]
pub async fn doctor_navdata_status(
    xplane_path: String,
    depth: Option<String>,
) -> Result<navdata::DoctorNavdataReport, String> {
    tokio::task::spawn_blocking(move || {
        navdata::inspect(Path::new(&xplane_path), depth.as_deref() == Some("full"))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reshade_sidecars_alone_do_not_prove_an_injector_is_present() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("reshade-shaders")).unwrap();
        fs::write(temp.path().join("ReShade.ini"), "settings").unwrap();
        assert!(detect_injectors(temp.path()).is_empty());
        fs::write(temp.path().join("dxgi.dll"), "proxy").unwrap();
        let found = detect_injectors(temp.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].evidence, "dxgi.dll");
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

    #[test]
    fn report_export_accepts_only_supported_extensions() {
        let temp = tempfile::tempdir().unwrap();
        let report = temp.path().join("health.md");
        export_report(&report, "# Health").unwrap();
        assert_eq!(fs::read_to_string(report).unwrap(), "# Health");

        let error = export_report(&temp.path().join("health.txt"), "bad").unwrap_err();
        assert!(error.contains(".md or .json"));
    }
}

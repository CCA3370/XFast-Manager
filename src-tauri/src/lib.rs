mod analyzer;
mod app_dirs;
mod atomic_installer;
mod cache;
mod database;
mod error;
mod geo_regions;
mod hash_collector;
mod installer;
mod library_links;
mod livery_patterns;
mod logger;
mod management_index;
mod models;
mod performance;
mod registry;
mod scanner;
mod scenery_classifier;
mod scenery_index;
mod scenery_packs_manager;
mod task_control;
mod updater;
mod verifier;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::ToTauriError;
use analyzer::Analyzer;
use installer::Installer;
use models::{
    AircraftInfo, AnalysisResult, InstallResult, InstallTask, LiveryInfo, LuaScriptInfo,
    ManagementData, NavdataBackupInfo, NavdataManagerInfo, PluginInfo, SceneryIndexScanResult,
    SceneryIndexStats, SceneryIndexStatus, SceneryManagerData, SceneryPackageInfo,
};
use scenery_index::SceneryIndexManager;
use scenery_packs_manager::SceneryPacksManager;
use task_control::TaskControl;

use tauri::{Emitter, Manager, State};

/// Cross-platform helper to open a path in the system file explorer
fn open_in_explorer<P: AsRef<std::path::Path>>(path: P) -> Result<(), String> {
    let path = path.as_ref();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}

// ============================================================================
// System & Utility Commands
// ============================================================================

#[tauri::command]
fn get_cli_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    opener::open(&url).map_err(|e| format!("Failed to open URL: {}", e))
}

#[tauri::command]
async fn create_library_link_issue(
    library_name: String,
    download_url: String,
    referenced_by: Option<String>,
) -> Result<String, String> {
    let library_name = library_name.trim();
    let download_url = download_url.trim();

    if library_name.is_empty() {
        return Err("Library name is empty".to_string());
    }

    let parsed_url = reqwest::Url::parse(download_url)
        .map_err(|_| "Download URL is invalid".to_string())?;
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err("Download URL must be http/https".to_string());
    }

    let api_url = std::env::var("XFAST_LINK_API_URL")
        .unwrap_or_else(|_| "https://x-fast-manager.vercel.app/api/library-link".to_string());

    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&api_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "libraryName": library_name,
            "downloadUrl": download_url,
            "referencedBy": referenced_by
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to create issue: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Link API error {}: {}", status, error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let issue_url = response_json
        .get("issueUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if issue_url.is_empty() {
        return Err("Issue created but response URL missing".to_string());
    }

    Ok(issue_url)
}

// ============================================================================
// Installation Commands
// ============================================================================

#[tauri::command]
async fn analyze_addons(
    paths: Vec<String>,
    xplane_path: String,
    passwords: Option<HashMap<String, String>>,
    verification_preferences: Option<HashMap<String, bool>>,
) -> Result<AnalysisResult, String> {
    livery_patterns::ensure_patterns_loaded().await;

    // Run the analysis in a blocking thread pool to avoid blocking the async runtime
    tokio::task::spawn_blocking(move || {
        log_debug!(&format!("Analyzing paths: {:?}", paths), "analysis");
        log_debug!(
            &format!("Starting analysis with X-Plane path: {}", xplane_path),
            "analysis"
        );

        let analyzer = Analyzer::new();
        Ok(analyzer.analyze(paths, &xplane_path, passwords, verification_preferences))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn install_addons(
    app_handle: tauri::AppHandle,
    tasks: Vec<InstallTask>,
    atomic_install_enabled: Option<bool>,
    xplane_path: String,
    delete_source_after_install: Option<bool>,
    auto_sort_scenery: Option<bool>,
) -> Result<InstallResult, String> {
    // Clone app_handle for the blocking task
    let app_handle_clone = app_handle.clone();

    // Run the installation in a blocking thread pool to avoid blocking the async runtime
    tokio::task::spawn_blocking(move || {
        log_debug!(
            &format!(
                "Installing {} tasks: {}",
                tasks.len(),
                tasks
                    .iter()
                    .map(|t| &t.display_name)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            "installation"
        );

        let installer = Installer::new(app_handle_clone);
        installer
            .install(
                tasks,
                atomic_install_enabled.unwrap_or(false),
                xplane_path,
                delete_source_after_install.unwrap_or(false),
                auto_sort_scenery.unwrap_or(false),
            )
            .map_err(|e| format!("Installation failed: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

// ============================================================================
// Task Control Commands
// ============================================================================

#[tauri::command]
async fn cancel_installation(task_control: State<'_, TaskControl>) -> Result<(), String> {
    task_control.request_cancel_all();
    logger::log_info("Installation cancellation requested", Some("task_control"));
    Ok(())
}

#[tauri::command]
async fn skip_current_task(task_control: State<'_, TaskControl>) -> Result<(), String> {
    task_control.request_skip_current();
    logger::log_info("Current task skip requested", Some("task_control"));
    Ok(())
}

// ============================================================================
// Windows Registry Commands (Context Menu)
// ============================================================================

#[tauri::command]
fn register_context_menu() -> Result<(), String> {
    registry::register_context_menu().map_err(|e| format!("Failed to register context menu: {}", e))
}

#[tauri::command]
fn unregister_context_menu() -> Result<(), String> {
    registry::unregister_context_menu()
        .map_err(|e| format!("Failed to unregister context menu: {}", e))
}

#[tauri::command]
fn is_context_menu_registered() -> bool {
    registry::is_context_menu_registered()
}

#[tauri::command]
fn sync_context_menu_paths() -> Result<bool, String> {
    registry::sync_registry_paths().map_err(|e| format!("Failed to sync context menu paths: {}", e))
}

// ============================================================================
// Logging Commands
// ============================================================================

#[tauri::command]
fn log_from_frontend(level: String, message: String, context: Option<String>) {
    let ctx = context.as_deref();
    match level.to_lowercase().as_str() {
        "error" => logger::log_error(&message, ctx),
        "debug" => {
            // For frontend debug logs, we don't have file/line info, so pass None
            logger::log_debug(&message, ctx, Some("frontend"))
        }
        _ => logger::log_info(&message, ctx),
    }
}

#[tauri::command]
fn get_recent_logs(lines: Option<usize>) -> Vec<String> {
    logger::get_recent_logs(lines.unwrap_or(50))
}

#[tauri::command]
fn get_log_path() -> String {
    logger::get_log_path().to_string_lossy().to_string()
}

#[tauri::command]
fn get_all_logs() -> String {
    logger::get_all_logs()
}

#[tauri::command]
fn open_log_folder() -> Result<(), String> {
    open_in_explorer(logger::get_log_folder())
}

// ========== Scenery Folder Commands ==========

fn validate_scenery_folder_name(folder_name: &str) -> error::ApiResult<()> {
    if folder_name.is_empty()
        || folder_name.contains("..")
        || folder_name.contains('/')
        || folder_name.contains('\\')
    {
        return Err(error::ApiError::security_violation(
            "Invalid folder name: path traversal not allowed",
        ));
    }
    Ok(())
}

fn resolve_scenery_entry_path(
    xplane_path: &str,
    folder_name: &str,
) -> error::ApiResult<(PathBuf, PathBuf)> {
    validate_scenery_folder_name(folder_name)?;

    let base_path = PathBuf::from(xplane_path).join("Custom Scenery");
    let candidate = base_path.join(folder_name);
    if candidate.exists() {
        return Ok((candidate, base_path));
    }

    #[cfg(target_os = "windows")]
    {
        let lnk_path = base_path.join(format!("{}.lnk", folder_name));
        if lnk_path.exists() {
            return Ok((lnk_path, base_path));
        }
    }

    Err(error::ApiError::with_details(
        error::ApiErrorCode::NotFound,
        "Scenery folder not found",
        folder_name,
    ))
}

#[tauri::command]
fn open_scenery_folder(xplane_path: String, folder_name: String) -> error::ApiResult<()> {
    let (entry_path, base_path) = resolve_scenery_entry_path(&xplane_path, &folder_name)?;
    let metadata = fs::symlink_metadata(&entry_path)
        .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;

    // If it's a symlink, open the link itself to allow external targets
    if metadata.file_type().is_symlink() {
        return open_in_explorer(&entry_path).map_err(error::ApiError::internal);
    }

    // For regular directories/files, enforce canonical base containment
    let canonical_path = entry_path
        .canonicalize()
        .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;
    let canonical_base = base_path
        .canonicalize()
        .map_err(|e| error::ApiError::validation(format!("Invalid base path: {}", e)))?;

    if !canonical_path.starts_with(&canonical_base) {
        return Err(error::ApiError::security_violation(
            "Path traversal attempt detected",
        ));
    }

    open_in_explorer(&canonical_path).map_err(error::ApiError::internal)
}

#[tauri::command]
async fn delete_scenery_folder(xplane_path: String, folder_name: String) -> error::ApiResult<()> {
    let (entry_path, base_path) = resolve_scenery_entry_path(&xplane_path, &folder_name)?;
    let metadata = fs::symlink_metadata(&entry_path)
        .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;

    if metadata.file_type().is_symlink() {
        // Remove the symlink itself without following it
        if let Err(e) = fs::remove_file(&entry_path) {
            // Some platforms treat directory symlinks differently
            if let Err(e2) = fs::remove_dir(&entry_path) {
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    || e2.kind() == std::io::ErrorKind::PermissionDenied
                {
                    return Err(error::ApiError::permission_denied(format!(
                        "Permission denied when deleting: {}",
                        folder_name
                    )));
                }
                return Err(error::ApiError::internal(format!(
                    "Failed to delete scenery link: {} ({}; {})",
                    folder_name, e, e2
                )));
            }
        }
    } else if metadata.is_file() {
        // Handle Windows .lnk shortcuts or other file entries
        fs::remove_file(&entry_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                error::ApiError::permission_denied(format!(
                    "Permission denied when deleting: {}",
                    folder_name
                ))
            } else {
                error::ApiError::internal(format!("Failed to delete scenery file: {}", e))
            }
        })?;
    } else {
        // Security: Use canonicalize for strict path validation to prevent path traversal attacks
        let canonical_path = entry_path
            .canonicalize()
            .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;
        let canonical_base = base_path
            .canonicalize()
            .map_err(|e| error::ApiError::validation(format!("Invalid base path: {}", e)))?;

        if !canonical_path.starts_with(&canonical_base) {
            return Err(error::ApiError::security_violation(
                "Path traversal attempt detected",
            ));
        }

        // Delete the folder using the canonical path for safety
        fs::remove_dir_all(&canonical_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                error::ApiError::permission_denied(format!(
                    "Permission denied when deleting: {}",
                    folder_name
                ))
            } else {
                error::ApiError::internal(format!("Failed to delete scenery folder: {}", e))
            }
        })?;
    }

    // Remove from scenery index if it exists
    if let Err(e) = scenery_index::remove_scenery_entry(&xplane_path, &folder_name) {
        logger::log_error(
            &format!("Failed to remove scenery from index: {}", e),
            Some("scenery"),
        );
    }

    // Update scenery_packs.ini to remove the deleted entry
    let xplane_root = std::path::Path::new(&xplane_path);
    let packs_manager = scenery_packs_manager::SceneryPacksManager::new(xplane_root);
    if let Err(e) = packs_manager.apply_from_index() {
        logger::log_error(
            &format!("Failed to update scenery_packs.ini after deletion: {}", e),
            Some("scenery"),
        );
    }

    logger::log_info(
        &format!("Deleted scenery folder: {}", folder_name),
        Some("scenery"),
    );

    Ok(())
}

#[tauri::command]
fn set_log_locale(locale: String) {
    logger::set_locale(&locale);
}

#[tauri::command]
fn set_log_level(level: String) {
    let log_level = match level.to_lowercase().as_str() {
        "debug" => logger::LogLevel::Debug,
        "info" => logger::LogLevel::Info,
        "error" => logger::LogLevel::Error,
        _ => logger::LogLevel::Info, // Default to Info
    };
    logger::set_log_level(log_level);
}

// ========== Path Validation Commands ==========

#[tauri::command]
fn check_path_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[tauri::command]
fn launch_xplane(xplane_path: String, args: Option<Vec<String>>) -> Result<(), String> {
    let path = std::path::Path::new(&xplane_path);
    let extra_args = args.unwrap_or_default();

    #[cfg(target_os = "windows")]
    {
        let exe_path = path.join("X-Plane.exe");
        std::process::Command::new(exe_path)
            .args(&extra_args)
            .spawn()
            .map_err(|e| format!("Failed to launch X-Plane: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        // macOS uses 'open' command for .app bundles
        let app_path = path.join("X-Plane.app");
        let mut cmd = std::process::Command::new("open");
        cmd.arg(&app_path);
        if !extra_args.is_empty() {
            cmd.arg("--args");
            cmd.args(&extra_args);
        }
        cmd.spawn()
            .map_err(|e| format!("Failed to launch X-Plane: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let exe_path = path.join("X-Plane");
        std::process::Command::new(exe_path)
            .args(&extra_args)
            .spawn()
            .map_err(|e| format!("Failed to launch X-Plane: {}", e))?;
    }

    logger::log_info("X-Plane launched", Some("app"));
    Ok(())
}

#[tauri::command]
async fn is_xplane_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Run tasklist in a blocking task to avoid blocking the async runtime
        let result = tokio::task::spawn_blocking(|| {
            std::process::Command::new("tasklist")
                .args(["/FI", "IMAGENAME eq X-Plane.exe", "/NH"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let is_running = stdout.contains("X-Plane.exe");
                logger::log_debug(
                    &format!("X-Plane running check: {}", is_running),
                    Some("app"),
                    None,
                );
                is_running
            }
            Ok(Err(e)) => {
                logger::log_debug(&format!("Failed to run tasklist: {}", e), Some("app"), None);
                false
            }
            Err(e) => {
                logger::log_debug(&format!("Task join error: {}", e), Some("app"), None);
                false
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Run pgrep in a blocking task to avoid blocking the async runtime
        let result = tokio::task::spawn_blocking(|| {
            std::process::Command::new("pgrep")
                .args(["-x", "X-Plane"])
                .output()
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                let is_running = output.status.success();
                logger::log_debug(
                    &format!("X-Plane running check: {}", is_running),
                    Some("app"),
                    None,
                );
                return is_running;
            }
            Ok(Err(e)) => {
                logger::log_debug(&format!("Failed to run pgrep: {}", e), Some("app"), None);
                return false;
            }
            Err(e) => {
                logger::log_debug(&format!("Task join error: {}", e), Some("app"), None);
                return false;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Run pgrep in a blocking task to avoid blocking the async runtime
        let result = tokio::task::spawn_blocking(|| {
            std::process::Command::new("pgrep")
                .args(["-x", "X-Plane"])
                .output()
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                let is_running = output.status.success();
                logger::log_debug(
                    &format!("X-Plane running check: {}", is_running),
                    Some("app"),
                    None,
                );
                return is_running;
            }
            Ok(Err(e)) => {
                logger::log_debug(&format!("Failed to run pgrep: {}", e), Some("app"), None);
                return false;
            }
            Err(e) => {
                logger::log_debug(&format!("Task join error: {}", e), Some("app"), None);
                return false;
            }
        }
    }
}

#[tauri::command]
fn validate_xplane_path(path: String) -> Result<bool, String> {
    let path_obj = std::path::Path::new(&path);

    // Check if path exists
    if !path_obj.exists() {
        return Ok(false);
    }

    // Check if it's a directory
    if !path_obj.is_dir() {
        return Ok(false);
    }

    // Check for X-Plane executable
    let exe_name = if cfg!(target_os = "windows") {
        "X-Plane.exe"
    } else if cfg!(target_os = "macos") {
        "X-Plane.app"
    } else {
        "X-Plane"
    };

    let exe_path = path_obj.join(exe_name);
    Ok(exe_path.exists())
}

// ========== Update Commands ==========

#[tauri::command]
async fn check_for_updates(
    manual: bool,
    include_pre_release: bool,
) -> Result<updater::UpdateInfo, String> {
    let checker = updater::UpdateChecker::new();
    checker.check_for_updates(manual, include_pre_release).await
}

#[tauri::command]
fn get_last_check_time() -> Option<i64> {
    updater::get_last_check_time()
}

// ========== Library Download Links Commands ==========

#[tauri::command]
async fn lookup_library_links(
    library_names: Vec<String>,
) -> Result<std::collections::HashMap<String, Option<String>>, String> {
    Ok(library_links::lookup_library_links_local(library_names).await)
}

#[tauri::command]
async fn lookup_library_links_remote(
    library_names: Vec<String>,
    force_refresh: Option<bool>,
) -> Result<std::collections::HashMap<String, Option<String>>, String> {
    library_links::lookup_library_links_remote(library_names, force_refresh.unwrap_or(false)).await
}

// ========== Scenery Auto-Sorting Commands ==========

#[tauri::command]
async fn get_scenery_classification(
    xplane_path: String,
    folder_name: String,
) -> error::ApiResult<SceneryPackageInfo> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let scenery_path = xplane_path.join("Custom Scenery").join(&folder_name);

        if !scenery_path.exists() {
            return Err(error::ApiError::not_found(format!(
                "Scenery folder not found: {}",
                folder_name
            )));
        }

        let index_manager = SceneryIndexManager::new(xplane_path);
        index_manager
            .get_or_classify(&scenery_path)
            .map_err(|e| error::ApiError::internal(format!("Classification failed: {}", e)))
    })
    .await
    .map_err(|e| error::ApiError::internal(format!("Task join error: {}", e)))?
}

#[tauri::command]
async fn sort_scenery_packs(xplane_path: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let index_manager = SceneryIndexManager::new(xplane_path);

        logger::log_info("Resetting scenery index sort order", Some("scenery"));

        let has_changes = index_manager
            .reset_sort_order()
            .map_err(|e| format!("Failed to reset sort order: {}", e))?;

        logger::log_info(
            "Scenery index sort order reset successfully",
            Some("scenery"),
        );
        Ok(has_changes)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn rebuild_scenery_index(xplane_path: String) -> Result<SceneryIndexStats, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let index_manager = SceneryIndexManager::new(xplane_path);

        logger::log_info("Rebuilding scenery index", Some("scenery"));

        index_manager
            .rebuild_index()
            .map_err(|e| format!("Failed to rebuild index: {}", e))?;

        index_manager
            .get_stats()
            .map_err(|e| format!("Failed to get stats: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Reset the scenery database by deleting it entirely
/// This is useful when the database schema version is incompatible
#[tauri::command]
async fn reset_scenery_database() -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        logger::log_info("Resetting scenery database", Some("database"));
        database::delete_database().map_err(|e| format!("Failed to delete database: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn get_scenery_index_stats(xplane_path: String) -> Result<SceneryIndexStats, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let index_manager = SceneryIndexManager::new(xplane_path);

        index_manager
            .get_stats()
            .map_err(|e| format!("Failed to get stats: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn get_scenery_index_status(xplane_path: String) -> Result<SceneryIndexStatus, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let index_manager = SceneryIndexManager::new(xplane_path);

        index_manager
            .index_status()
            .map_err(|e| format!("Failed to get index status: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn quick_scan_scenery_index(xplane_path: String) -> Result<SceneryIndexScanResult, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let index_manager = SceneryIndexManager::new(xplane_path);

        index_manager
            .quick_scan_and_update()
            .map_err(|e| format!("Failed to quick scan scenery index: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn sync_scenery_packs_with_folder(xplane_path: String) -> Result<usize, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let manager = SceneryPacksManager::new(xplane_path);

        manager
            .sync_with_folder()
            .map_err(|e| format!("Failed to sync scenery packs: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn get_scenery_manager_data(xplane_path: String) -> Result<SceneryManagerData, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let index_manager = SceneryIndexManager::new(xplane_path);

        index_manager
            .get_manager_data()
            .map_err(|e| format!("Failed to get scenery manager data: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn update_scenery_entry(
    xplane_path: String,
    folder_name: String,
    enabled: Option<bool>,
    sort_order: Option<u32>,
    category: Option<models::SceneryCategory>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let index_manager = SceneryIndexManager::new(xplane_path);

        index_manager
            .update_entry(&folder_name, enabled, sort_order, category)
            .map_err(|e| format!("Failed to update scenery entry: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn move_scenery_entry(
    xplane_path: String,
    folder_name: String,
    new_sort_order: u32,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let index_manager = SceneryIndexManager::new(xplane_path);

        index_manager
            .move_entry(&folder_name, new_sort_order)
            .map_err(|e| format!("Failed to move scenery entry: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn apply_scenery_changes(
    xplane_path: String,
    entries: Vec<models::SceneryEntryUpdate>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        let index_manager = SceneryIndexManager::new(xplane_path);

        logger::log_info("Applying scenery changes to index and ini", Some("scenery"));

        // Update index with all entry changes
        index_manager
            .batch_update_entries(&entries)
            .map_err(|e| format!("Failed to update index: {}", e))?;

        // Apply to ini file
        let packs_manager = SceneryPacksManager::new(xplane_path);
        packs_manager
            .apply_from_index()
            .map_err(|e| format!("Failed to apply scenery changes: {}", e))?;

        logger::log_info("Scenery changes applied successfully", Some("scenery"));
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

// ========== Management Commands ==========

#[tauri::command]
async fn scan_aircraft(xplane_path: String) -> Result<ManagementData<AircraftInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_aircraft(xplane_path)
            .map_err(|e| format!("Failed to scan aircraft: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn check_aircraft_updates(
    mut aircraft: Vec<AircraftInfo>,
) -> Result<Vec<AircraftInfo>, String> {
    management_index::check_aircraft_updates(&mut aircraft).await;
    Ok(aircraft)
}

#[tauri::command]
async fn check_plugins_updates(mut plugins: Vec<PluginInfo>) -> Result<Vec<PluginInfo>, String> {
    management_index::check_plugins_updates(&mut plugins).await;
    Ok(plugins)
}

#[tauri::command]
async fn scan_plugins(xplane_path: String) -> Result<ManagementData<PluginInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_plugins(xplane_path)
            .map_err(|e| format!("Failed to scan plugins: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn scan_navdata(xplane_path: String) -> Result<ManagementData<NavdataManagerInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_navdata(xplane_path)
            .map_err(|e| format!("Failed to scan navdata: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn scan_navdata_backups(xplane_path: String) -> Result<Vec<NavdataBackupInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_navdata_backups(xplane_path)
            .map_err(|e| format!("Failed to scan navdata backups: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn restore_navdata_backup(
    xplane_path: String,
    backup_folder_name: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::restore_navdata_backup(xplane_path, &backup_folder_name)
            .map_err(|e| format!("Failed to restore navdata backup: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn toggle_management_item(
    xplane_path: String,
    item_type: String,
    folder_name: String,
) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::toggle_management_item(xplane_path, &item_type, &folder_name)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn delete_management_item(
    xplane_path: String,
    item_type: String,
    folder_name: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::delete_management_item(xplane_path, &item_type, &folder_name)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn open_management_folder(
    xplane_path: String,
    item_type: String,
    folder_name: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::open_management_folder(xplane_path, &item_type, &folder_name)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn get_aircraft_liveries(
    xplane_path: String,
    aircraft_folder: String,
) -> Result<Vec<LiveryInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::get_aircraft_liveries(xplane_path, &aircraft_folder)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn delete_aircraft_livery(
    xplane_path: String,
    aircraft_folder: String,
    livery_folder: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::delete_aircraft_livery(xplane_path, &aircraft_folder, &livery_folder)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn open_livery_folder(
    xplane_path: String,
    aircraft_folder: String,
    livery_folder: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::open_livery_folder(xplane_path, &aircraft_folder, &livery_folder)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn set_cfg_disabled(
    xplane_path: String,
    item_type: String,
    folder_name: String,
    disabled: bool,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::set_cfg_disabled(xplane_path, &item_type, &folder_name, disabled)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn get_lua_scripts(xplane_path: String) -> Result<Vec<LuaScriptInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_lua_scripts(xplane_path)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn toggle_lua_script(xplane_path: String, file_name: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::toggle_lua_script(xplane_path, &file_name)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn delete_lua_script(xplane_path: String, file_name: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::delete_lua_script(xplane_path, &file_name)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // When a second instance is launched, this callback is triggered
            // args[0] is the executable path, args[1..] are the actual arguments
            let file_args: Vec<String> = args.iter().skip(1).map(|s| s.to_string()).collect();

            if !file_args.is_empty() {
                logger::log_info(
                    &format!(
                        "{}: {:?}",
                        logger::tr(logger::LogMsg::LaunchedWithArgs),
                        file_args
                    ),
                    Some("app"),
                );
                // Emit event to frontend with the new file paths
                let _ = app.emit("cli-args", file_args);

                // Bring window to front
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window.set_focus() {
                        logger::log_debug(
                            &format!("Failed to focus window: {}", e),
                            Some("app"),
                            Some("lib.rs"),
                        );
                    }
                }
            }
        }))
        .invoke_handler(tauri::generate_handler![
            get_cli_args,
            get_platform,
            get_app_version,
            open_url,
            create_library_link_issue,
            analyze_addons,
            install_addons,
            cancel_installation,
            skip_current_task,
            register_context_menu,
            unregister_context_menu,
            is_context_menu_registered,
            sync_context_menu_paths,
            log_from_frontend,
            get_recent_logs,
            get_log_path,
            get_all_logs,
            open_log_folder,
            open_scenery_folder,
            delete_scenery_folder,
            set_log_locale,
            set_log_level,
            check_path_exists,
            launch_xplane,
            is_xplane_running,
            validate_xplane_path,
            check_for_updates,
            get_last_check_time,
            // Library download links
            lookup_library_links,
            lookup_library_links_remote,
            // Scenery auto-sorting commands
            get_scenery_classification,
            sort_scenery_packs,
            rebuild_scenery_index,
            reset_scenery_database,
            get_scenery_index_stats,
            get_scenery_index_status,
            quick_scan_scenery_index,
            sync_scenery_packs_with_folder,
            // Scenery manager commands
            get_scenery_manager_data,
            update_scenery_entry,
            move_scenery_entry,
            apply_scenery_changes,
            // Management commands
            scan_aircraft,
            check_aircraft_updates,
            scan_plugins,
            check_plugins_updates,
            scan_navdata,
            scan_navdata_backups,
            restore_navdata_backup,
            toggle_management_item,
            delete_management_item,
            open_management_folder,
            get_aircraft_liveries,
            delete_aircraft_livery,
            open_livery_folder,
            set_cfg_disabled,
            get_lua_scripts,
            toggle_lua_script,
            delete_lua_script
        ])
        .setup(|app| {
            // Initialize TaskControl state
            app.manage(TaskControl::new());

            // Log application startup
            logger::log_info(&logger::tr(logger::LogMsg::AppStarted), Some("app"));

            // Handle CLI arguments if present (for first launch)
            let args: Vec<String> = std::env::args().skip(1).collect();
            if !args.is_empty() {
                logger::log_info(
                    &format!(
                        "{}: {:?}",
                        logger::tr(logger::LogMsg::LaunchedWithArgs),
                        args
                    ),
                    Some("app"),
                );
                // Emit event to frontend
                app.emit("cli-args", args.clone()).ok();
            }

            // Fetch latest livery patterns on startup (non-blocking)
            tauri::async_runtime::spawn(async {
                livery_patterns::ensure_patterns_loaded().await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod analyzer;
mod atomic_installer;
mod cache;
mod database;
mod error;
mod hash_collector;
mod installer;
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

use crate::error::ToTauriError;
use analyzer::Analyzer;
use installer::Installer;
use models::{
    AircraftInfo, AnalysisResult, InstallResult, InstallTask, ManagementData,
    NavdataManagerInfo, PluginInfo, SceneryIndexScanResult, SceneryIndexStats,
    SceneryIndexStatus, SceneryManagerData, SceneryPackageInfo,
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

#[tauri::command]
fn open_scenery_folder(xplane_path: String, folder_name: String) -> error::ApiResult<()> {
    // Security: Validate folder_name doesn't contain path traversal sequences
    if folder_name.contains("..") || folder_name.contains('/') || folder_name.contains('\\') {
        return Err(error::ApiError::security_violation(
            "Invalid folder name: path traversal not allowed",
        ));
    }

    let scenery_path = std::path::PathBuf::from(&xplane_path)
        .join("Custom Scenery")
        .join(&folder_name);

    if !scenery_path.exists() {
        return Err(error::ApiError::with_details(
            error::ApiErrorCode::NotFound,
            "Scenery folder not found",
            &folder_name,
        ));
    }

    // Security: Use canonicalize for strict path validation to prevent path traversal attacks
    let canonical_path = scenery_path
        .canonicalize()
        .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;
    let canonical_base = std::path::PathBuf::from(&xplane_path)
        .join("Custom Scenery")
        .canonicalize()
        .map_err(|e| error::ApiError::validation(format!("Invalid base path: {}", e)))?;

    if !canonical_path.starts_with(&canonical_base) {
        return Err(error::ApiError::security_violation(
            "Path traversal attempt detected",
        ));
    }

    open_in_explorer(&canonical_path).map_err(|e| error::ApiError::internal(e))
}

#[tauri::command]
async fn delete_scenery_folder(
    xplane_path: String,
    folder_name: String,
) -> error::ApiResult<()> {
    // Security: Validate folder_name doesn't contain path traversal sequences
    if folder_name.contains("..") || folder_name.contains('/') || folder_name.contains('\\') {
        return Err(error::ApiError::security_violation(
            "Invalid folder name: path traversal not allowed",
        ));
    }

    let scenery_path = std::path::PathBuf::from(&xplane_path)
        .join("Custom Scenery")
        .join(&folder_name);

    if !scenery_path.exists() {
        return Err(error::ApiError::with_details(
            error::ApiErrorCode::NotFound,
            "Scenery folder not found",
            &folder_name,
        ));
    }

    // Security: Use canonicalize for strict path validation to prevent path traversal attacks
    let canonical_path = scenery_path
        .canonicalize()
        .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;
    let canonical_base = std::path::PathBuf::from(&xplane_path)
        .join("Custom Scenery")
        .canonicalize()
        .map_err(|e| error::ApiError::validation(format!("Invalid base path: {}", e)))?;

    if !canonical_path.starts_with(&canonical_base) {
        return Err(error::ApiError::security_violation(
            "Path traversal attempt detected",
        ));
    }

    // Delete the folder using the canonical path for safety
    std::fs::remove_dir_all(&canonical_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            error::ApiError::permission_denied(format!(
                "Permission denied when deleting: {}",
                folder_name
            ))
        } else {
            error::ApiError::internal(format!("Failed to delete scenery folder: {}", e))
        }
    })?;

    // Remove from scenery index if it exists
    if let Err(e) = scenery_index::remove_scenery_entry(&xplane_path, &folder_name) {
        logger::log_error(
            &format!("Failed to remove scenery from index: {}", e),
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
async fn check_plugins_updates(
    mut plugins: Vec<PluginInfo>,
) -> Result<Vec<PluginInfo>, String> {
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
async fn toggle_management_item(
    xplane_path: String,
    item_type: String,
    folder_name: String,
) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::toggle_management_item(xplane_path, &item_type, &folder_name)
            .map_err(|e| error::ApiError::from(e))
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
            .map_err(|e| error::ApiError::from(e))
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
            .map_err(|e| error::ApiError::from(e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            analyze_addons,
            install_addons,
            cancel_installation,
            skip_current_task,
            register_context_menu,
            unregister_context_menu,
            is_context_menu_registered,
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
            validate_xplane_path,
            check_for_updates,
            get_last_check_time,
            // Scenery auto-sorting commands
            get_scenery_classification,
            sort_scenery_packs,
            rebuild_scenery_index,
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
            toggle_management_item,
            delete_management_item,
            open_management_folder
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

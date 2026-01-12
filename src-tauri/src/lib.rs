mod analyzer;
mod cache;
mod hash_collector;
mod installer;
mod logger;
mod models;
mod performance;
mod registry;
mod scanner;
mod verifier;

use std::collections::HashMap;

use analyzer::Analyzer;
use installer::Installer;
use models::{AnalysisResult, InstallResult, InstallTask};

use tauri::{Emitter, Manager};

#[tauri::command]
fn get_cli_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
async fn analyze_addons(
    paths: Vec<String>,
    xplane_path: String,
    passwords: Option<HashMap<String, String>>,
) -> Result<AnalysisResult, String> {
    // Run the analysis in a blocking thread pool to avoid blocking the async runtime
    tokio::task::spawn_blocking(move || {
        log_debug!(&format!("Analyzing paths: {:?}", paths), "analysis");
        log_debug!(&format!("Starting analysis with X-Plane path: {}", xplane_path), "analysis");

        let analyzer = Analyzer::new();
        Ok(analyzer.analyze(paths, &xplane_path, passwords))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn install_addons(app_handle: tauri::AppHandle, tasks: Vec<InstallTask>) -> Result<InstallResult, String> {
    // Clone app_handle for the blocking task
    let app_handle_clone = app_handle.clone();

    // Run the installation in a blocking thread pool to avoid blocking the async runtime
    tokio::task::spawn_blocking(move || {
        log_debug!(&format!("Installing {} tasks: {}", tasks.len(), tasks.iter().map(|t| &t.display_name).cloned().collect::<Vec<_>>().join(", ")), "installation");

        let installer = Installer::new(app_handle_clone);
        installer
            .install(tasks)
            .map_err(|e| format!("Installation failed: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
fn register_context_menu() -> Result<(), String> {
    registry::register_context_menu()
        .map_err(|e| format!("Failed to register context menu: {}", e))
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
fn log_from_frontend(level: String, message: String, context: Option<String>) {
    let ctx = context.as_deref();
    match level.to_lowercase().as_str() {
        "error" => logger::log_error(&message, ctx),
        "debug" => {
            // For frontend debug logs, we don't have file/line info, so pass None
            logger::log_debug(&message, ctx, Some("frontend"))
        },
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
    let folder = logger::get_log_folder();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
fn set_log_locale(locale: String) {
    logger::set_locale(&locale);
}

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
                    &format!("{}: {:?}", logger::tr(logger::LogMsg::LaunchedWithArgs), file_args),
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
            analyze_addons,
            install_addons,
            register_context_menu,
            unregister_context_menu,
            is_context_menu_registered,
            log_from_frontend,
            get_recent_logs,
            get_log_path,
            get_all_logs,
            open_log_folder,
            set_log_locale,
            check_path_exists,
            validate_xplane_path
        ])
        .setup(|app| {
            // Log application startup
            logger::log_info(&logger::tr(logger::LogMsg::AppStarted), Some("app"));

            // Handle CLI arguments if present (for first launch)
            let args: Vec<String> = std::env::args().skip(1).collect();
            if !args.is_empty() {
                logger::log_info(
                    &format!("{}: {:?}", logger::tr(logger::LogMsg::LaunchedWithArgs), args),
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

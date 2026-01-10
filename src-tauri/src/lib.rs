mod analyzer;
mod cache;
mod installer;
mod logger;
mod models;
mod performance;
mod registry;
mod scanner;

use std::collections::HashMap;

use analyzer::Analyzer;
use installer::Installer;
use models::{AnalysisResult, InstallTask};

use tauri::Emitter;

#[tauri::command]
fn get_cli_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
fn analyze_addons(
    paths: Vec<String>,
    xplane_path: String,
    passwords: Option<HashMap<String, String>>,
) -> Result<AnalysisResult, String> {
    let analyzer = Analyzer::new();
    Ok(analyzer.analyze(paths, &xplane_path, passwords))
}

#[tauri::command]
fn install_addons(app_handle: tauri::AppHandle, tasks: Vec<InstallTask>) -> Result<(), String> {
    let installer = Installer::new(app_handle);
    installer
        .install(tasks)
        .map_err(|e| format!("Installation failed: {}", e))
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            set_log_locale
        ])
        .setup(|app| {
            // Log application startup
            logger::log_info(&logger::tr(logger::LogMsg::AppStarted), Some("app"));

            // Handle CLI arguments if present
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

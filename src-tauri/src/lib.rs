mod analyzer;
mod installer;
mod models;
mod registry;
mod scanner;

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
fn analyze_addons(paths: Vec<String>, xplane_path: String) -> Result<AnalysisResult, String> {
    let analyzer = Analyzer::new();
    Ok(analyzer.analyze(paths, &xplane_path))
}

#[tauri::command]
fn install_addons(tasks: Vec<InstallTask>) -> Result<(), String> {
    let installer = Installer::new();
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
            unregister_context_menu
        ])
        .setup(|app| {
            // Handle CLI arguments if present
            let args: Vec<String> = std::env::args().skip(1).collect();
            if !args.is_empty() {
                println!("Launched with arguments: {:?}", args);
                // Emit event to frontend
                app.emit("cli-args", args.clone()).ok();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

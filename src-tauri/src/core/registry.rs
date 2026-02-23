#[cfg(target_os = "windows")]
use anyhow::Result;
#[cfg(target_os = "windows")]
use std::env;
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
pub fn register_context_menu() -> Result<()> {
    let exe_path = env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy();

    // Use HKEY_CURRENT_USER instead of HKEY_CLASSES_ROOT to avoid requiring admin privileges
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Register for files (*)
    let (shell_key, _) = hkcu.create_subkey(r"Software\Classes\*\shell\XFast Manager")?;
    shell_key.set_value("", &"Install to X-Plane")?;
    shell_key.set_value("Icon", &exe_path_str.to_string())?;

    let (command_key, _) = hkcu.create_subkey(r"Software\Classes\*\shell\XFast Manager\command")?;
    command_key.set_value("", &format!("\"{}\" \"%1\"", exe_path_str))?;

    // Register for directories
    let (dir_shell_key, _) =
        hkcu.create_subkey(r"Software\Classes\Directory\shell\XFast Manager")?;
    dir_shell_key.set_value("", &"Install to X-Plane")?;
    dir_shell_key.set_value("Icon", &exe_path_str.to_string())?;

    let (dir_command_key, _) =
        hkcu.create_subkey(r"Software\Classes\Directory\shell\XFast Manager\command")?;
    dir_command_key.set_value("", &format!("\"{}\" \"%1\"", exe_path_str))?;

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn unregister_context_menu() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Try to delete for files
    let _ = hkcu.delete_subkey_all(r"Software\Classes\*\shell\XFast Manager");

    // Try to delete for directories
    let _ = hkcu.delete_subkey_all(r"Software\Classes\Directory\shell\XFast Manager");

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn is_context_menu_registered() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    // Check if the registry key exists
    hkcu.open_subkey(r"Software\Classes\*\shell\XFast Manager")
        .is_ok()
}

/// Get the exe path currently stored in the registry
#[cfg(target_os = "windows")]
fn get_registered_exe_path() -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let command_key = hkcu
        .open_subkey(r"Software\Classes\*\shell\XFast Manager\command")
        .ok()?;
    let value: String = command_key.get_value("").ok()?;
    // Parse "\"C:\path\to\exe.exe\" \"%1\"" format
    let path = value.trim_start_matches('"');
    let end = path.find("\" \"")?;
    Some(path[..end].to_string())
}

/// Sync registry paths on startup
/// Returns Ok(true) if paths were updated, Ok(false) if no update needed
#[cfg(target_os = "windows")]
pub fn sync_registry_paths() -> Result<bool> {
    if !is_context_menu_registered() {
        return Ok(false);
    }

    let current_path = env::current_exe()?.to_string_lossy().to_string();

    if let Some(registered_path) = get_registered_exe_path() {
        if registered_path != current_path {
            // Path mismatch, re-register to update paths
            register_context_menu()?;
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(not(target_os = "windows"))]
use anyhow::Result;

#[cfg(not(target_os = "windows"))]
pub fn register_context_menu() -> Result<()> {
    Err(anyhow::anyhow!(
        "Context menu registration is only supported on Windows"
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn unregister_context_menu() -> Result<()> {
    Err(anyhow::anyhow!(
        "Context menu unregistration is only supported on Windows"
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn is_context_menu_registered() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn sync_registry_paths() -> Result<bool> {
    Ok(false)
}

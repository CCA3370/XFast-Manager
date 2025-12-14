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

    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    // Register for files (*)
    let (shell_key, _) = hkcr.create_subkey(r"*\shell\XFastInstall")?;
    shell_key.set_value("", &"Install to X-Plane")?;
    shell_key.set_value("Icon", &format!("{}", exe_path_str))?;

    let (command_key, _) = hkcr.create_subkey(r"*\shell\XFastInstall\command")?;
    command_key.set_value("", &format!("\"{}\" \"%1\"", exe_path_str))?;

    // Register for directories
    let (dir_shell_key, _) = hkcr.create_subkey(r"Directory\shell\XFastInstall")?;
    dir_shell_key.set_value("", &"Install to X-Plane")?;
    dir_shell_key.set_value("Icon", &format!("{}", exe_path_str))?;

    let (dir_command_key, _) = hkcr.create_subkey(r"Directory\shell\XFastInstall\command")?;
    dir_command_key.set_value("", &format!("\"{}\" \"%1\"", exe_path_str))?;

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn unregister_context_menu() -> Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    // Try to delete for files
    let _ = hkcr.delete_subkey_all(r"*\shell\XFastInstall");

    // Try to delete for directories
    let _ = hkcr.delete_subkey_all(r"Directory\shell\XFastInstall");

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn register_context_menu() -> Result<(), String> {
    Err("Context menu registration is only supported on Windows".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn unregister_context_menu() -> Result<(), String> {
    Err("Context menu unregistration is only supported on Windows".to_string())
}

//! Centralized app data directory management
//!
//! All persistent data (logs, database, cache) should use paths from this module
//! to ensure consistent storage location across the application.

use std::path::PathBuf;

/// App identifier matching tauri.conf.json
const APP_IDENTIFIER: &str = "com.xfastmanager.tool";

/// Get the app data directory for persistent storage
///
/// Returns platform-specific paths:
/// - Windows: %LOCALAPPDATA%\com.xfastmanager.tool
/// - macOS: ~/Library/Application Support/com.xfastmanager.tool
/// - Linux: ~/.config/com.xfastmanager.tool
pub fn get_app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // Use LOCALAPPDATA (Local) instead of APPDATA (Roaming)
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join(APP_IDENTIFIER);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_IDENTIFIER);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join(APP_IDENTIFIER);
        }
    }

    // Fallback to current directory
    PathBuf::from(".")
}

/// Get the logs directory
pub fn get_logs_dir() -> PathBuf {
    get_app_data_dir().join("logs")
}

/// Get the log file path
pub fn get_log_file_path() -> PathBuf {
    get_logs_dir().join("xfastmanager.log")
}

/// Get the database file path
pub fn get_database_path() -> PathBuf {
    get_app_data_dir().join("scenery.db")
}

/// Get the update check cache file path
pub fn get_update_cache_path() -> PathBuf {
    get_app_data_dir().join("update_check_cache.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_data_dir_not_empty() {
        let dir = get_app_data_dir();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_paths_contain_app_identifier() {
        let data_dir = get_app_data_dir();
        assert!(data_dir.to_string_lossy().contains(APP_IDENTIFIER));
    }
}

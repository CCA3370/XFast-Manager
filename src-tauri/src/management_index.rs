//! Management index module for scanning aircraft, plugins, and navdata
//!
//! This module provides scanning functionality for X-Plane add-ons
//! to support the unified management UI.
//!
//! Enable/Disable mechanism:
//! - Aircraft: Rename .acf <-> .xfma files (not scanning subdirectories)
//! - Plugins: Rename .xpl <-> .xfmp files (including subdirectories)
//! - Navdata: Rename folder with - prefix (existing behavior)

use crate::logger;
use crate::models::{AircraftInfo, ManagementData, NavdataManagerInfo, PluginInfo};
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Scan aircraft in the X-Plane Aircraft folder
pub fn scan_aircraft(xplane_path: &Path) -> Result<ManagementData<AircraftInfo>> {
    let aircraft_path = xplane_path.join("Aircraft");
    if !aircraft_path.exists() {
        return Err(anyhow!("Aircraft folder not found"));
    }

    logger::log_info("Scanning aircraft folder...", Some("management"));

    let mut entries: Vec<AircraftInfo> = Vec::new();

    // Scan up to 3 levels deep for .acf or .xfma files
    scan_aircraft_recursive(&aircraft_path, &aircraft_path, 0, 3, &mut entries)?;

    // Sort by display name
    entries.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));

    let total_count = entries.len();
    let enabled_count = entries.iter().filter(|e| e.enabled).count();

    logger::log_info(
        &format!("Found {} aircraft ({} enabled)", total_count, enabled_count),
        Some("management"),
    );

    Ok(ManagementData {
        entries,
        total_count,
        enabled_count,
    })
}

fn scan_aircraft_recursive(
    base_path: &Path,
    current_path: &Path,
    depth: usize,
    max_depth: usize,
    entries: &mut Vec<AircraftInfo>,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    let read_dir = match fs::read_dir(current_path) {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Skip hidden folders and system folders
        if folder_name.starts_with('.') || folder_name == "Laminar Research" {
            continue;
        }

        // Check if this folder contains an .acf or .xfma file
        if let Some((acf_file, enabled)) = find_acf_or_xfma_file(&path) {
            let display_name = folder_name.clone();

            // Check for liveries folder
            let liveries_path = path.join("liveries");
            let (has_liveries, livery_count) = if liveries_path.exists() && liveries_path.is_dir() {
                let count = fs::read_dir(&liveries_path)
                    .map(|rd| rd.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count())
                    .unwrap_or(0);
                (count > 0, count)
            } else {
                (false, 0)
            };

            // Read version info
            let version = read_version_info(&path);

            // Get relative folder name from Aircraft folder
            let relative_path = path
                .strip_prefix(base_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            entries.push(AircraftInfo {
                folder_name: relative_path,
                display_name,
                acf_file,
                enabled,
                has_liveries,
                livery_count,
                version,
            });
        } else {
            // Recurse into subdirectories
            scan_aircraft_recursive(base_path, &path, depth + 1, max_depth, entries)?;
        }
    }

    Ok(())
}

/// Find .acf or .xfma file in a folder (not scanning subdirectories)
/// Returns (file_name, is_enabled)
fn find_acf_or_xfma_file(folder: &Path) -> Option<(String, bool)> {
    let read_dir = fs::read_dir(folder).ok()?;

    // First, look for .acf files (enabled)
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("acf") {
                    let file_name = path.file_name().and_then(|s| s.to_str())?.to_string();
                    return Some((file_name, true));
                }
            }
        }
    }

    // If no .acf found, look for .xfma files (disabled)
    let read_dir = fs::read_dir(folder).ok()?;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("xfma") {
                    let file_name = path.file_name().and_then(|s| s.to_str())?.to_string();
                    return Some((file_name, false));
                }
            }
        }
    }

    None
}

/// Read version information from a folder
/// Tries to read from:
/// 1. skunkcrafts_updater.cfg (reads the value after "version|") - higher priority
/// 2. version.* files (reads first line) - fallback
fn read_version_info(folder: &Path) -> Option<String> {
    // First, try skunkcrafts_updater.cfg (higher priority)
    let updater_path = folder.join("skunkcrafts_updater.cfg");
    if updater_path.exists() {
        if let Ok(content) = fs::read_to_string(&updater_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.to_lowercase().starts_with("version|") {
                    let parts: Vec<&str> = line.splitn(2, '|').collect();
                    if parts.len() == 2 {
                        let version = parts[1].trim();
                        if !version.is_empty() {
                            return Some(version.to_string());
                        }
                    }
                }
            }
        }
    }

    // Fall back to version.* files
    if let Ok(read_dir) = fs::read_dir(folder) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name.to_lowercase().starts_with("version.") || name.to_lowercase() == "version" {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let first_line = content.lines().next().unwrap_or("").trim();
                            if !first_line.is_empty() {
                                return Some(first_line.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Scan plugins in the X-Plane Resources/plugins folder
pub fn scan_plugins(xplane_path: &Path) -> Result<ManagementData<PluginInfo>> {
    let plugins_path = xplane_path.join("Resources").join("plugins");
    if !plugins_path.exists() {
        return Err(anyhow!("Plugins folder not found"));
    }

    logger::log_info("Scanning plugins folder...", Some("management"));

    let mut entries: Vec<PluginInfo> = Vec::new();

    let read_dir = fs::read_dir(&plugins_path)?;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Skip hidden folders
        if folder_name.starts_with('.') {
            continue;
        }

        // Find .xpl and .xfmp files (including subdirectories)
        let (xpl_files, xfmp_files) = find_xpl_and_xfmp_files(&path);

        // Skip if no plugin files found
        if xpl_files.is_empty() && xfmp_files.is_empty() {
            continue;
        }

        // Enabled if there are any .xpl files
        let enabled = !xpl_files.is_empty();
        let display_name = folder_name.clone();

        // Combine all files for display (show original names without disabled extension)
        let all_files: Vec<String> = if enabled {
            xpl_files.clone()
        } else {
            // Show .xfmp files but indicate they are disabled
            xfmp_files.iter().map(|f| f.replace(".xfmp", ".xpl")).collect()
        };

        // Determine platform from xpl file locations
        let platform = detect_plugin_platform(&path, &all_files);

        // Read version info
        let version = read_version_info(&path);

        entries.push(PluginInfo {
            folder_name,
            display_name,
            xpl_files: all_files,
            enabled,
            platform,
            version,
        });
    }

    // Sort by display name
    entries.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));

    let total_count = entries.len();
    let enabled_count = entries.iter().filter(|e| e.enabled).count();

    logger::log_info(
        &format!("Found {} plugins ({} enabled)", total_count, enabled_count),
        Some("management"),
    );

    Ok(ManagementData {
        entries,
        total_count,
        enabled_count,
    })
}

/// Find .xpl and .xfmp files in a folder (including subdirectories)
/// Returns (xpl_files, xfmp_files)
fn find_xpl_and_xfmp_files(folder: &Path) -> (Vec<String>, Vec<String>) {
    let mut xpl_files = Vec::new();
    let mut xfmp_files = Vec::new();

    // Use walkdir to recursively find all .xpl and .xfmp files
    for entry in WalkDir::new(folder).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension() {
            let relative_path = path
                .strip_prefix(folder)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            if ext.eq_ignore_ascii_case("xpl") {
                xpl_files.push(relative_path);
            } else if ext.eq_ignore_ascii_case("xfmp") {
                xfmp_files.push(relative_path);
            }
        }
    }

    (xpl_files, xfmp_files)
}

fn detect_plugin_platform(folder: &Path, xpl_files: &[String]) -> String {
    let mut has_win = false;
    let mut has_mac = false;
    let mut has_lin = false;

    for xpl_file in xpl_files {
        let lower = xpl_file.to_lowercase();
        if lower.contains("win") {
            has_win = true;
        }
        if lower.contains("mac") {
            has_mac = true;
        }
        if lower.contains("lin") {
            has_lin = true;
        }
    }

    // Check platform folders
    let win_folders = ["win", "win_x64"];
    let mac_folders = ["mac", "mac_x64"];
    let lin_folders = ["lin", "lin_x64"];

    for wf in &win_folders {
        if folder.join(wf).exists() {
            has_win = true;
        }
    }
    for mf in &mac_folders {
        if folder.join(mf).exists() {
            has_mac = true;
        }
    }
    for lf in &lin_folders {
        if folder.join(lf).exists() {
            has_lin = true;
        }
    }

    let count = [has_win, has_mac, has_lin].iter().filter(|&&x| x).count();
    if count >= 2 {
        "multi".to_string()
    } else if has_win {
        "win".to_string()
    } else if has_mac {
        "mac".to_string()
    } else if has_lin {
        "lin".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Scan navdata in the X-Plane Custom Data folder
pub fn scan_navdata(xplane_path: &Path) -> Result<ManagementData<NavdataManagerInfo>> {
    let custom_data_path = xplane_path.join("Custom Data");
    if !custom_data_path.exists() {
        return Err(anyhow!("Custom Data folder not found"));
    }

    logger::log_info("Scanning navdata folder...", Some("management"));

    let mut entries: Vec<NavdataManagerInfo> = Vec::new();

    // Scan up to 10 levels deep for cycle.json files
    scan_navdata_recursive(&custom_data_path, &custom_data_path, 0, 10, &mut entries)?;

    // Sort by provider name
    entries.sort_by(|a, b| a.provider_name.to_lowercase().cmp(&b.provider_name.to_lowercase()));

    let total_count = entries.len();
    let enabled_count = entries.iter().filter(|e| e.enabled).count();

    logger::log_info(
        &format!("Found {} navdata entries ({} enabled)", total_count, enabled_count),
        Some("management"),
    );

    Ok(ManagementData {
        entries,
        total_count,
        enabled_count,
    })
}

fn scan_navdata_recursive(
    base_path: &Path,
    current_path: &Path,
    depth: usize,
    max_depth: usize,
    entries: &mut Vec<NavdataManagerInfo>,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    let read_dir = match fs::read_dir(current_path) {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    for entry in read_dir.flatten() {
        let path = entry.path();

        // Check for cycle.json file
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.eq_ignore_ascii_case("cycle.json") {
                    if let Some(info) = parse_cycle_json(&path, base_path, current_path) {
                        entries.push(info);
                    }
                }
            }
            continue;
        }

        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Skip hidden folders
        if folder_name.starts_with('.') {
            continue;
        }

        // Recurse into subdirectories
        scan_navdata_recursive(base_path, &path, depth + 1, max_depth, entries)?;
    }

    Ok(())
}

fn parse_cycle_json(
    cycle_json_path: &Path,
    base_path: &Path,
    parent_folder: &Path,
) -> Option<NavdataManagerInfo> {
    let content = fs::read_to_string(cycle_json_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let provider_name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let cycle = json.get("cycle").and_then(|v| v.as_str()).map(|s| s.to_string());
    let airac = json.get("airac").and_then(|v| v.as_str()).map(|s| s.to_string());

    let folder_name = parent_folder
        .strip_prefix(base_path)
        .unwrap_or(parent_folder)
        .to_string_lossy()
        .to_string();

    // Check if folder is enabled (not starting with -)
    let enabled = !parent_folder
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.starts_with('-'))
        .unwrap_or(false);

    Some(NavdataManagerInfo {
        folder_name,
        provider_name,
        cycle,
        airac,
        enabled,
    })
}

/// Toggle enabled state for a management item
/// - Aircraft: Rename .acf <-> .xfma files (not scanning subdirectories)
/// - Plugins: Rename .xpl <-> .xfmp files (including subdirectories)
/// - Navdata: Rename folder with - prefix
pub fn toggle_management_item(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
) -> Result<bool> {
    let base_path = match item_type {
        "aircraft" => xplane_path.join("Aircraft"),
        "plugin" => xplane_path.join("Resources").join("plugins"),
        "navdata" => xplane_path.join("Custom Data"),
        _ => return Err(anyhow!("Unknown item type: {}", item_type)),
    };

    let current_path = base_path.join(folder_name);
    if !current_path.exists() {
        return Err(anyhow!("Folder not found: {}", folder_name));
    }

    match item_type {
        "aircraft" => toggle_aircraft_files(&current_path, folder_name),
        "plugin" => toggle_plugin_files(&current_path, folder_name),
        "navdata" => toggle_navdata_folder(&current_path, &base_path, folder_name),
        _ => Err(anyhow!("Unknown item type: {}", item_type)),
    }
}

/// Toggle aircraft files: .acf <-> .xfma (only in the folder, not subdirectories)
fn toggle_aircraft_files(folder_path: &Path, folder_name: &str) -> Result<bool> {
    let read_dir = fs::read_dir(folder_path)?;

    let mut acf_files: Vec<std::path::PathBuf> = Vec::new();
    let mut xfma_files: Vec<std::path::PathBuf> = Vec::new();

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("acf") {
                acf_files.push(path);
            } else if ext.eq_ignore_ascii_case("xfma") {
                xfma_files.push(path);
            }
        }
    }

    let new_enabled = if !acf_files.is_empty() {
        // Currently enabled (has .acf files), disable by renaming to .xfma
        for acf_path in &acf_files {
            let new_path = acf_path.with_extension("xfma");
            fs::rename(acf_path, &new_path)?;
        }
        logger::log_info(
            &format!(
                "Disabled aircraft '{}': renamed {} .acf file(s) to .xfma",
                folder_name,
                acf_files.len()
            ),
            Some("management"),
        );
        false
    } else if !xfma_files.is_empty() {
        // Currently disabled (has .xfma files), enable by renaming to .acf
        for xfma_path in &xfma_files {
            let new_path = xfma_path.with_extension("acf");
            fs::rename(xfma_path, &new_path)?;
        }
        logger::log_info(
            &format!(
                "Enabled aircraft '{}': renamed {} .xfma file(s) to .acf",
                folder_name,
                xfma_files.len()
            ),
            Some("management"),
        );
        true
    } else {
        return Err(anyhow!("No .acf or .xfma files found in aircraft folder"));
    };

    Ok(new_enabled)
}

/// Toggle plugin files: .xpl <-> .xfmp (including subdirectories)
fn toggle_plugin_files(folder_path: &Path, folder_name: &str) -> Result<bool> {
    let mut xpl_files: Vec<std::path::PathBuf> = Vec::new();
    let mut xfmp_files: Vec<std::path::PathBuf> = Vec::new();

    // Use walkdir to find all .xpl and .xfmp files recursively
    for entry in WalkDir::new(folder_path).max_depth(10).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("xpl") {
                xpl_files.push(path.to_path_buf());
            } else if ext.eq_ignore_ascii_case("xfmp") {
                xfmp_files.push(path.to_path_buf());
            }
        }
    }

    let new_enabled = if !xpl_files.is_empty() {
        // Currently enabled (has .xpl files), disable by renaming to .xfmp
        for xpl_path in &xpl_files {
            let new_path = xpl_path.with_extension("xfmp");
            fs::rename(xpl_path, &new_path)?;
        }
        logger::log_info(
            &format!(
                "Disabled plugin '{}': renamed {} .xpl file(s) to .xfmp",
                folder_name,
                xpl_files.len()
            ),
            Some("management"),
        );
        false
    } else if !xfmp_files.is_empty() {
        // Currently disabled (has .xfmp files), enable by renaming to .xpl
        for xfmp_path in &xfmp_files {
            let new_path = xfmp_path.with_extension("xpl");
            fs::rename(xfmp_path, &new_path)?;
        }
        logger::log_info(
            &format!(
                "Enabled plugin '{}': renamed {} .xfmp file(s) to .xpl",
                folder_name,
                xfmp_files.len()
            ),
            Some("management"),
        );
        true
    } else {
        return Err(anyhow!("No .xpl or .xfmp files found in plugin folder"));
    };

    Ok(new_enabled)
}

/// Toggle navdata folder: rename with - prefix (existing behavior)
fn toggle_navdata_folder(current_path: &Path, base_path: &Path, folder_name: &str) -> Result<bool> {
    let file_name = current_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid folder name"))?;

    let (new_name, new_enabled) = if file_name.starts_with('-') {
        // Currently disabled, enable it
        (file_name[1..].to_string(), true)
    } else {
        // Currently enabled, disable it
        (format!("-{}", file_name), false)
    };

    let new_path = current_path.parent().unwrap_or(base_path).join(&new_name);

    fs::rename(current_path, &new_path)?;

    logger::log_info(
        &format!(
            "Toggled navdata '{}' -> '{}' (enabled: {})",
            folder_name, new_name, new_enabled
        ),
        Some("management"),
    );

    Ok(new_enabled)
}

/// Delete a management item folder
pub fn delete_management_item(xplane_path: &Path, item_type: &str, folder_name: &str) -> Result<()> {
    let base_path = match item_type {
        "aircraft" => xplane_path.join("Aircraft"),
        "plugin" => xplane_path.join("Resources").join("plugins"),
        "navdata" => xplane_path.join("Custom Data"),
        _ => return Err(anyhow!("Unknown item type: {}", item_type)),
    };

    let target_path = base_path.join(folder_name);
    if !target_path.exists() {
        return Err(anyhow!("Folder not found: {}", folder_name));
    }

    // Safety check: ensure path is within the expected base directory
    if !target_path.starts_with(&base_path) {
        return Err(anyhow!("Invalid path"));
    }

    fs::remove_dir_all(&target_path)?;

    logger::log_info(
        &format!("Deleted {} folder: {}", item_type, folder_name),
        Some("management"),
    );

    Ok(())
}

/// Open a management item folder in the system file explorer
pub fn open_management_folder(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
) -> Result<()> {
    let base_path = match item_type {
        "aircraft" => xplane_path.join("Aircraft"),
        "plugin" => xplane_path.join("Resources").join("plugins"),
        "navdata" => xplane_path.join("Custom Data"),
        _ => return Err(anyhow!("Unknown item type: {}", item_type)),
    };

    let target_path = base_path.join(folder_name);
    if !target_path.exists() {
        return Err(anyhow!("Folder not found: {}", folder_name));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&target_path)
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&target_path)
            .spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&target_path)
            .spawn()?;
    }

    Ok(())
}

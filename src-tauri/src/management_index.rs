//! Management index module for scanning aircraft, plugins, and navdata
//!
//! This module provides scanning functionality for X-Plane add-ons
//! to support the unified management UI.
//!
//! Enable/Disable mechanism:
//! - Aircraft: Rename .acf <-> .xfma files (not scanning subdirectories)
//! - Plugins: Rename .xpl <-> .xfmp files (including subdirectories)

use crate::logger;
use crate::models::{AircraftInfo, ManagementData, NavdataManagerInfo, PluginInfo};
use anyhow::{anyhow, Result};
use rayon::prelude::*;
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

    // Collect subdirectories first
    let mut subdirs: Vec<(std::path::PathBuf, String)> = Vec::new();
    for entry in read_dir.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !ft.is_dir() {
            continue;
        }
        let path = entry.path();
        let folder_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        // Skip hidden folders and system folders
        if folder_name.starts_with('.') || folder_name == "Laminar Research" {
            continue;
        }
        subdirs.push((path, folder_name));
    }

    // Process subdirectories in parallel, each doing a single read_dir pass
    let results: Vec<Option<AircraftInfo>> = subdirs
        .par_iter()
        .map(|(path, folder_name)| {
            let info = scan_single_aircraft_folder(path, base_path, folder_name);
            info
        })
        .collect();

    // Collect results and recurse for non-aircraft folders
    let mut recurse_dirs: Vec<&std::path::PathBuf> = Vec::new();
    for (i, result) in results.into_iter().enumerate() {
        if let Some(info) = result {
            entries.push(info);
        } else {
            recurse_dirs.push(&subdirs[i].0);
        }
    }

    for dir in recurse_dirs {
        scan_aircraft_recursive(base_path, dir, depth + 1, max_depth, entries)?;
    }

    Ok(())
}

/// Scan a single aircraft folder in one directory read pass.
/// Returns Some(AircraftInfo) if it contains .acf/.xfma files, None otherwise.
fn scan_single_aircraft_folder(
    folder: &Path,
    base_path: &Path,
    folder_name: &str,
) -> Option<AircraftInfo> {
    let read_dir = fs::read_dir(folder).ok()?;

    let mut acf_file: Option<String> = None;
    let mut xfma_file: Option<String> = None;
    let mut has_liveries = false;
    let mut livery_count = 0;
    let mut updater_cfg_path: Option<std::path::PathBuf> = None;
    let mut version_file_paths: Vec<std::path::PathBuf> = Vec::new();

    for entry in read_dir.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let name_lower = name.to_lowercase();

        if ft.is_file() {
            // Check for .acf / .xfma
            if acf_file.is_none() && name_lower.ends_with(".acf") {
                acf_file = Some(name.clone());
            } else if xfma_file.is_none() && name_lower.ends_with(".xfma") {
                xfma_file = Some(name.clone());
            }
            // Check for version sources
            if name_lower == "skunkcrafts_updater.cfg" {
                updater_cfg_path = Some(entry.path());
            } else if name_lower.contains("version")
                || name_lower == "767.ini"
                || name_lower == "757.ini"
            {
                version_file_paths.push(entry.path());
            }
        } else if ft.is_dir() {
            if name_lower == "liveries" {
                // Count liveries
                if let Ok(liveries_rd) = fs::read_dir(entry.path()) {
                    for lv_entry in liveries_rd.flatten() {
                        if lv_entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            livery_count += 1;
                        }
                    }
                }
                has_liveries = livery_count > 0;
            }
        }
    }

    // Must have .acf or .xfma to be recognized as aircraft
    let (acf_name, enabled) = if let Some(name) = acf_file {
        (name, true)
    } else if let Some(name) = xfma_file {
        (name, false)
    } else {
        return None;
    };

    // Read version info (priority: skunkcrafts_updater.cfg > version files)
    let (version, update_url) =
        read_version_from_paths(updater_cfg_path.as_deref(), &version_file_paths);

    let relative_path = folder
        .strip_prefix(base_path)
        .unwrap_or(folder)
        .to_string_lossy()
        .to_string();

    Some(AircraftInfo {
        folder_name: relative_path,
        display_name: folder_name.to_string(),
        acf_file: acf_name,
        enabled,
        has_liveries,
        livery_count,
        version,
        update_url,
        latest_version: None, // Will be populated by check_aircraft_updates
        has_update: false,    // Will be set by check_aircraft_updates
    })
}

/// Read version from already-discovered paths (avoids extra directory reads)
/// Returns (version, update_url) tuple
fn read_version_from_paths(
    updater_cfg: Option<&Path>,
    version_files: &[std::path::PathBuf],
) -> (Option<String>, Option<String>) {
    let mut update_url: Option<String> = None;

    // First, try skunkcrafts_updater.cfg (higher priority)
    if let Some(cfg_path) = updater_cfg {
        if let Ok(content) = fs::read_to_string(cfg_path) {
            let mut cfg_version: Option<String> = None;

            for line in content.lines() {
                let line = line.trim();
                let line_lower = line.to_lowercase();

                if line_lower.starts_with("version|") {
                    let parts: Vec<&str> = line.splitn(2, '|').collect();
                    if parts.len() == 2 {
                        let version = parts[1].trim();
                        if !version.is_empty() {
                            cfg_version = Some(version.to_string());
                        }
                    }
                } else if line_lower.starts_with("module|") {
                    let parts: Vec<&str> = line.splitn(2, '|').collect();
                    if parts.len() == 2 {
                        let url = parts[1].trim();
                        if !url.is_empty() {
                            update_url = Some(url.to_string());
                        }
                    }
                }
            }

            if cfg_version.is_some() {
                return (cfg_version, update_url);
            }
        }
    }

    // Fall back to version files
    let mut version_tokens: Vec<String> = Vec::new();
    let mut first_line_fallback: Option<String> = None;

    for path in version_files {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // Record first non-empty line as fallback
                if first_line_fallback.is_none() {
                    first_line_fallback = Some(line.to_string());
                }

                if !has_version_pattern(line) {
                    continue;
                }
                for token in line.split_whitespace() {
                    if has_version_pattern(token) && !version_tokens.contains(&token.to_string()) {
                        version_tokens.push(token.to_string());
                    }
                }
            }
        }
    }
    if !version_tokens.is_empty() {
        return (Some(version_tokens.join("/")), update_url);
    }

    // Fallback: try to parse pure digit string (e.g., "020310" -> "2.3.10")
    if let Some(ref first_line) = first_line_fallback {
        if let Some(parsed) = try_parse_digit_version(first_line) {
            return (Some(parsed), update_url);
        }
        // Last resort: return the first line as-is
        return (first_line_fallback, update_url);
    }

    (None, update_url)
}

/// Read version information from a folder (used by plugins where we don't have pre-collected paths)
/// Returns (version, update_url) tuple
fn read_version_info_with_url(folder: &Path) -> (Option<String>, Option<String>) {
    let read_dir = match fs::read_dir(folder) {
        Ok(rd) => rd,
        Err(_) => return (None, None),
    };

    let mut updater_cfg_path: Option<std::path::PathBuf> = None;
    let mut version_file_paths: Vec<std::path::PathBuf> = Vec::new();

    for entry in read_dir.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !ft.is_file() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let name_lower = name.to_lowercase();
        if name_lower == "skunkcrafts_updater.cfg" {
            updater_cfg_path = Some(entry.path());
        } else if name_lower.contains("version")
            || name_lower == "767.ini"
            || name_lower == "757.ini"
        {
            version_file_paths.push(entry.path());
        }
    }

    read_version_from_paths(updater_cfg_path.as_deref(), &version_file_paths)
}

/// Check if a string contains a version-like pattern (digit(s).digit(s))
fn has_version_pattern(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < len && bytes[i] == b'.' && i > start {
                i += 1;
                if i < len && bytes[i].is_ascii_digit() {
                    return true;
                }
            }
        } else {
            i += 1;
        }
    }
    false
}

/// Try to parse a pure digit string as a version by splitting every 2 characters
/// and removing leading zeros from each part.
/// Example: "020310" -> "2.3.10"
fn try_parse_digit_version(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();

    // Must have at least 4 digits to split into meaningful parts
    if digits.len() < 4 {
        return None;
    }

    // Split every 2 characters
    let parts: Vec<String> = digits
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let s = std::str::from_utf8(chunk).unwrap_or("0");
            // Parse to remove leading zeros, then convert back to string
            s.parse::<u32>().unwrap_or(0).to_string()
        })
        .collect();

    // Must have at least 2 parts
    if parts.len() < 2 {
        return None;
    }

    Some(parts.join("."))
}

/// Scan plugins in the X-Plane Resources/plugins folder
pub fn scan_plugins(xplane_path: &Path) -> Result<ManagementData<PluginInfo>> {
    let plugins_path = xplane_path.join("Resources").join("plugins");
    if !plugins_path.exists() {
        return Err(anyhow!("Plugins folder not found"));
    }

    logger::log_info("Scanning plugins folder...", Some("management"));

    // Collect plugin subdirectories
    let mut subdirs: Vec<(std::path::PathBuf, String)> = Vec::new();
    let read_dir = fs::read_dir(&plugins_path)?;
    for entry in read_dir.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !ft.is_dir() {
            continue;
        }
        let folder_name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };
        if folder_name.starts_with('.') {
            continue;
        }
        subdirs.push((entry.path(), folder_name));
    }

    // Process plugin folders in parallel
    let mut entries: Vec<PluginInfo> = subdirs
        .par_iter()
        .filter_map(|(path, folder_name)| {
            scan_single_plugin_folder(path, folder_name)
        })
        .collect();

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

/// Scan a single plugin folder
fn scan_single_plugin_folder(path: &Path, folder_name: &str) -> Option<PluginInfo> {
    // Find .xpl and .xfmp files (including subdirectories)
    let (xpl_files, xfmp_files) = find_xpl_and_xfmp_files(path);

    // Skip if no plugin files found
    if xpl_files.is_empty() && xfmp_files.is_empty() {
        return None;
    }

    // Enabled if there are any .xpl files
    let enabled = !xpl_files.is_empty();

    // Combine all files for display
    let all_files: Vec<String> = if enabled {
        xpl_files.clone()
    } else {
        xfmp_files.iter().map(|f| f.replace(".xfmp", ".xpl")).collect()
    };

    // Determine platform from xpl file locations
    let platform = detect_plugin_platform(path, &all_files);

    // Read version info with update URL
    let (version, update_url) = read_version_info_with_url(path);

    Some(PluginInfo {
        folder_name: folder_name.to_string(),
        display_name: folder_name.to_string(),
        xpl_files: all_files,
        enabled,
        platform,
        version,
        update_url,
        latest_version: None, // Will be populated by check_plugins_updates
        has_update: false,    // Will be set by check_plugins_updates
    })
}

/// Find .xpl and .xfmp files in a folder (including subdirectories)
/// Returns (xpl_files, xfmp_files)
fn find_xpl_and_xfmp_files(folder: &Path) -> (Vec<String>, Vec<String>) {
    let mut xpl_files = Vec::new();
    let mut xfmp_files = Vec::new();

    for entry in WalkDir::new(folder).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
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

    // Check platform folders only if not already detected from file paths
    if !has_win || !has_mac || !has_lin {
        if let Ok(read_dir) = fs::read_dir(folder) {
            for entry in read_dir.flatten() {
                let ft = match entry.file_type() {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };
                if !ft.is_dir() {
                    continue;
                }
                if let Ok(name) = entry.file_name().into_string() {
                    let lower = name.to_lowercase();
                    if lower == "win" || lower == "win_x64" {
                        has_win = true;
                    } else if lower == "mac" || lower == "mac_x64" {
                        has_mac = true;
                    } else if lower == "lin" || lower == "lin_x64" {
                        has_lin = true;
                    }
                }
            }
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

    // Use WalkDir to efficiently find cycle.json files
    for entry in WalkDir::new(&custom_data_path)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(name) = entry.file_name().to_str() {
            if name.eq_ignore_ascii_case("cycle.json") {
                let path = entry.path();
                if let Some(parent) = path.parent() {
                    if let Some(info) = parse_cycle_json(path, &custom_data_path, parent) {
                        entries.push(info);
                    }
                }
            }
        }
    }

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

    Some(NavdataManagerInfo {
        folder_name,
        provider_name,
        cycle,
        airac,
        enabled: true, // Always enabled (toggle not supported)
    })
}

/// Toggle enabled state for a management item
/// - Aircraft: Rename .acf <-> .xfma files (not scanning subdirectories)
/// - Plugins: Rename .xpl <-> .xfmp files (including subdirectories)
pub fn toggle_management_item(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
) -> Result<bool> {
    let base_path = match item_type {
        "aircraft" => xplane_path.join("Aircraft"),
        "plugin" => xplane_path.join("Resources").join("plugins"),
        _ => return Err(anyhow!("Unknown item type: {}", item_type)),
    };

    let current_path = base_path.join(folder_name);
    if !current_path.exists() {
        return Err(anyhow!("Folder not found: {}", folder_name));
    }

    match item_type {
        "aircraft" => toggle_aircraft_files(&current_path, folder_name),
        "plugin" => toggle_plugin_files(&current_path, folder_name),
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

/// Check for aircraft updates by fetching remote skunkcrafts_updater.cfg files
/// This function modifies the aircraft list in place, setting latest_version and has_update
pub async fn check_aircraft_updates(aircraft: &mut [AircraftInfo]) {
    use futures::future::join_all;

    // Collect aircraft with update URLs
    let update_tasks: Vec<_> = aircraft
        .iter()
        .enumerate()
        .filter_map(|(idx, a)| a.update_url.as_ref().map(|url| (idx, url.clone())))
        .collect();

    if update_tasks.is_empty() {
        return;
    }

    // Fetch all remote configs in parallel
    let fetch_futures: Vec<_> = update_tasks
        .iter()
        .map(|(_, url)| fetch_remote_version(url.clone()))
        .collect();

    let results = join_all(fetch_futures).await;

    // Update aircraft with results
    for ((idx, _), result) in update_tasks.into_iter().zip(results) {
        if let Some(remote_version) = result {
            let local_version = aircraft[idx].version.as_deref().unwrap_or("");
            aircraft[idx].latest_version = Some(remote_version.clone());
            aircraft[idx].has_update = remote_version != local_version;
        }
    }
}

/// Check for plugin updates by fetching remote skunkcrafts_updater.cfg files
/// This function modifies the plugins list in place, setting latest_version and has_update
pub async fn check_plugins_updates(plugins: &mut [PluginInfo]) {
    use futures::future::join_all;

    // Collect plugins with update URLs
    let update_tasks: Vec<_> = plugins
        .iter()
        .enumerate()
        .filter_map(|(idx, p)| p.update_url.as_ref().map(|url| (idx, url.clone())))
        .collect();

    if update_tasks.is_empty() {
        return;
    }

    // Fetch all remote configs in parallel
    let fetch_futures: Vec<_> = update_tasks
        .iter()
        .map(|(_, url)| fetch_remote_version(url.clone()))
        .collect();

    let results = join_all(fetch_futures).await;

    // Update plugins with results
    for ((idx, _), result) in update_tasks.into_iter().zip(results) {
        if let Some(remote_version) = result {
            let local_version = plugins[idx].version.as_deref().unwrap_or("");
            plugins[idx].latest_version = Some(remote_version.clone());
            plugins[idx].has_update = remote_version != local_version;
        }
    }
}

/// Fetch remote version from skunkcrafts_updater.cfg
async fn fetch_remote_version(base_url: String) -> Option<String> {
    let url = format!("{}/skunkcrafts_updater.cfg", base_url.trim_end_matches('/'));

    // Build client with system proxy support (reads from Windows system settings)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        logger::log_debug(
            &format!("Failed to fetch remote config: {} - {}", url, response.status()),
            Some("management"),
            None,
        );
        return None;
    }

    let content = response.text().await.ok()?;

    // Parse version from config
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

    None
}

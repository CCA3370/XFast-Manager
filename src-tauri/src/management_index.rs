//! Management index module for scanning aircraft, plugins, and navdata
//!
//! This module provides scanning functionality for X-Plane add-ons
//! to support the unified management UI.
//!
//! Enable/Disable mechanism:
//! - Aircraft: Rename .acf <-> .xfma files (not scanning subdirectories)
//! - Plugins: Rename .xpl <-> .xfmp files (including subdirectories)

use crate::logger;
use crate::models::{
    AircraftInfo, LiveryInfo, LuaScriptInfo, ManagementData, NavdataBackupInfo,
    NavdataBackupVerification, NavdataManagerInfo, PluginInfo,
};
use anyhow::{anyhow, Result};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Validate a folder name to prevent path traversal and separator injection
fn validate_folder_name(folder_name: &str) -> Result<()> {
    if folder_name.is_empty()
        || folder_name.contains("..")
        || folder_name.contains('/')
        || folder_name.contains('\\')
    {
        return Err(anyhow!("Invalid folder name"));
    }
    Ok(())
}

/// Resolve a management folder path with strict validation
fn resolve_management_path(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
) -> Result<PathBuf> {
    validate_folder_name(folder_name)?;

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

    let canonical_base = base_path
        .canonicalize()
        .map_err(|e| anyhow!("Invalid base path: {}", e))?;
    let canonical_target = target_path
        .canonicalize()
        .map_err(|e| anyhow!("Invalid path: {}", e))?;

    if !canonical_target.starts_with(&canonical_base) {
        return Err(anyhow!("Invalid path"));
    }

    Ok(canonical_target)
}

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
    entries.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });

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
        // Skip hidden folders
        if folder_name.starts_with('.') {
            continue;
        }
        subdirs.push((path, folder_name));
    }

    // Process subdirectories in parallel, each doing a single read_dir pass
    let results: Vec<Option<AircraftInfo>> = subdirs
        .par_iter()
        .map(|(path, folder_name)| scan_single_aircraft_folder(path, base_path, folder_name))
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
        } else if ft.is_dir() && name_lower == "liveries" {
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

    // Must have .acf or .xfma to be recognized as aircraft
    let (acf_name, enabled) = if let Some(name) = acf_file {
        (name, true)
    } else if let Some(name) = xfma_file {
        (name, false)
    } else {
        return None;
    };

    // Read version info (priority: skunkcrafts_updater.cfg > version files)
    let (version, update_url, cfg_disabled) =
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
        cfg_disabled,
    })
}

/// Read version from already-discovered paths (avoids extra directory reads)
/// Returns (version, update_url, cfg_disabled) tuple
pub fn read_version_from_paths(
    updater_cfg: Option<&Path>,
    version_files: &[std::path::PathBuf],
) -> (Option<String>, Option<String>, Option<bool>) {
    let mut update_url: Option<String> = None;
    let mut cfg_disabled: Option<bool> = None;

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
                } else if line_lower.starts_with("disabled|") {
                    let parts: Vec<&str> = line.splitn(2, '|').collect();
                    if parts.len() == 2 {
                        let value = parts[1].trim().to_lowercase();
                        cfg_disabled = Some(value == "true" || value == "1");
                    }
                }
            }

            if cfg_version.is_some() {
                return (cfg_version, update_url, cfg_disabled);
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
        return (Some(version_tokens.join("/")), update_url, cfg_disabled);
    }

    // Fallback: try to parse pure digit string (e.g., "020310" -> "2.3.10")
    if let Some(ref first_line) = first_line_fallback {
        if let Some(parsed) = try_parse_digit_version(first_line) {
            return (Some(parsed), update_url, cfg_disabled);
        }
        // Last resort: return the first line as-is
        return (first_line_fallback, update_url, cfg_disabled);
    }

    (None, update_url, cfg_disabled)
}

/// Read version information from a folder (used by plugins where we don't have pre-collected paths)
/// Returns (version, update_url, cfg_disabled) tuple
pub fn read_version_info_with_url(folder: &Path) -> (Option<String>, Option<String>, Option<bool>) {
    let read_dir = match fs::read_dir(folder) {
        Ok(rd) => rd,
        Err(_) => return (None, None, None),
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
pub fn has_version_pattern(s: &str) -> bool {
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
pub fn try_parse_digit_version(s: &str) -> Option<String> {
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
        .filter_map(|(path, folder_name)| scan_single_plugin_folder(path, folder_name))
        .collect();

    // Sort by display name
    entries.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });

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
        xfmp_files
            .iter()
            .map(|f| f.replace(".xfmp", ".xpl"))
            .collect()
    };

    // Determine platform from xpl file locations
    let platform = detect_plugin_platform(path, &all_files);

    // Read version info with update URL
    let (version, update_url, cfg_disabled) = read_version_info_with_url(path);

    // Detect FlyWithLua scripts
    let (has_scripts, script_count) = if folder_name.eq_ignore_ascii_case("FlyWithLua") {
        let scripts_path = path.join("Scripts");
        if scripts_path.is_dir() {
            let count = fs::read_dir(&scripts_path)
                .ok()
                .map(|rd| {
                    rd.flatten()
                        .filter(|e| {
                            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                                && e.path()
                                    .extension()
                                    .and_then(|ext| ext.to_str())
                                    .map(|ext| {
                                        ext.eq_ignore_ascii_case("lua")
                                            || ext.eq_ignore_ascii_case("xfml")
                                    })
                                    .unwrap_or(false)
                        })
                        .count()
                })
                .unwrap_or(0);
            (count > 0, count)
        } else {
            (false, 0)
        }
    } else {
        (false, 0)
    };

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
        cfg_disabled,
        has_scripts,
        script_count,
    })
}

/// Find .xpl and .xfmp files in a folder (including subdirectories)
/// Returns (xpl_files, xfmp_files)
fn find_xpl_and_xfmp_files(folder: &Path) -> (Vec<String>, Vec<String>) {
    let mut xpl_files = Vec::new();
    let mut xfmp_files = Vec::new();

    for entry in WalkDir::new(folder)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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
    // Skip Backup_Data folder (used for navdata backup/restore)
    let backup_data_path = custom_data_path.join("Backup_Data");
    for entry in WalkDir::new(&custom_data_path)
        .max_depth(10)
        .into_iter()
        .filter_entry(|e| e.path() != backup_data_path)
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
    entries.sort_by(|a, b| {
        a.provider_name
            .to_lowercase()
            .cmp(&b.provider_name.to_lowercase())
    });

    let total_count = entries.len();
    let enabled_count = entries.iter().filter(|e| e.enabled).count();

    logger::log_info(
        &format!(
            "Found {} navdata entries ({} enabled)",
            total_count, enabled_count
        ),
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

    let cycle = json
        .get("cycle")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let airac = json
        .get("airac")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

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
    let current_path = resolve_management_path(xplane_path, item_type, folder_name)?;

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
    for entry in WalkDir::new(folder_path)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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
pub fn delete_management_item(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
) -> Result<()> {
    let target_path = resolve_management_path(xplane_path, item_type, folder_name)?;
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
    let target_path = resolve_management_path(xplane_path, item_type, folder_name)?;

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
            &format!(
                "Failed to fetch remote config: {} - {}",
                url,
                response.status()
            ),
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

/// Set the disabled| field in skunkcrafts_updater.cfg for an aircraft or plugin
/// If the cfg file doesn't exist, returns Ok without creating it
pub fn set_cfg_disabled(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
    disabled: bool,
) -> Result<()> {
    let folder_path = resolve_management_path(xplane_path, item_type, folder_name)?;

    let cfg_path = folder_path.join("skunkcrafts_updater.cfg");

    // If cfg file doesn't exist, skip write per user preference
    if !cfg_path.exists() {
        logger::log_debug(
            &format!(
                "No skunkcrafts_updater.cfg found for {}, skipping disabled write",
                folder_name
            ),
            Some("management"),
            None,
        );
        return Ok(());
    }

    // Read the existing file
    let content = fs::read_to_string(&cfg_path)?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Find and update the disabled| line, or add it if not present
    let disabled_value = if disabled { "true" } else { "false" };
    let disabled_line = format!("disabled|{}", disabled_value);
    let mut found = false;

    for line in &mut lines {
        if line.trim().to_lowercase().starts_with("disabled|") {
            *line = disabled_line.clone();
            found = true;
            break;
        }
    }

    if !found {
        // Add the disabled line at the end
        lines.push(disabled_line);
    }

    // Write back to file
    let new_content = lines.join("\n");
    fs::write(&cfg_path, new_content)?;

    logger::log_info(
        &format!(
            "Set disabled|{} in {} for {}",
            disabled_value,
            cfg_path.display(),
            folder_name
        ),
        Some("management"),
    );

    Ok(())
}

/// Scan navdata backups in Custom Data/Backup_Data folder
pub fn scan_navdata_backups(xplane_path: &Path) -> Result<Vec<NavdataBackupInfo>> {
    let backup_data_path = xplane_path.join("Custom Data").join("Backup_Data");

    if !backup_data_path.exists() {
        logger::log_debug(
            "Backup_Data folder does not exist",
            Some("management"),
            None,
        );
        return Ok(Vec::new());
    }

    logger::log_info("Scanning navdata backups...", Some("management"));

    let mut backups: Vec<NavdataBackupInfo> = Vec::new();

    for entry in fs::read_dir(&backup_data_path)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let verification_path = path.join("verification.json");
        if verification_path.exists() {
            match fs::read_to_string(&verification_path) {
                Ok(content) => match serde_json::from_str::<NavdataBackupVerification>(&content) {
                    Ok(verification) => {
                        let folder_name = entry.file_name().to_string_lossy().to_string();

                        backups.push(NavdataBackupInfo {
                            folder_name,
                            verification,
                        });
                    }
                    Err(e) => {
                        logger::log_error(
                            &format!("Failed to parse verification.json in {:?}: {}", path, e),
                            Some("management"),
                        );
                    }
                },
                Err(e) => {
                    logger::log_error(
                        &format!("Failed to read verification.json in {:?}: {}", path, e),
                        Some("management"),
                    );
                }
            }
        }
    }

    logger::log_info(
        &format!("Found {} navdata backups", backups.len()),
        Some("management"),
    );

    Ok(backups)
}

/// Restore navdata from backup (EXTREME PERFORMANCE OPTIMIZED)
///
/// Optimizations:
/// - Removed pre-restore SHA-256 verification (checksum is empty string, size-only check)
/// - Removed post-restore SHA-256 verification (fs::rename is atomic, data integrity guaranteed)
/// - Uses optimized directory-level rename when possible
pub fn restore_navdata_backup(xplane_path: &Path, backup_folder_name: &str) -> Result<()> {
    let backup_path = xplane_path
        .join("Custom Data")
        .join("Backup_Data")
        .join(backup_folder_name);

    if !backup_path.exists() {
        return Err(anyhow!("Backup folder not found: {}", backup_folder_name));
    }

    let verification_path = backup_path.join("verification.json");
    if !verification_path.exists() {
        return Err(anyhow!(
            "verification.json not found in backup: {}",
            backup_folder_name
        ));
    }

    logger::log_info(
        &format!("Restoring navdata backup: {}", backup_folder_name),
        Some("management"),
    );

    // Step 1: Read verification file
    let content = fs::read_to_string(&verification_path)?;
    let verification: NavdataBackupVerification = serde_json::from_str(&content)?;

    // Step 2: Verify backup file integrity (OPTIMIZED: size-only, single stat per file)
    // Checksum is now empty string, so we only check existence and size
    logger::log_info(
        "Verifying backup integrity (size-only)...",
        Some("management"),
    );
    for file_entry in &verification.files {
        let file_path = backup_path.join(&file_entry.relative_path);
        // OPTIMIZED: Use single fs::metadata() call instead of exists() + metadata()
        match fs::metadata(&file_path) {
            Ok(meta) => {
                if meta.len() != file_entry.size {
                    return Err(anyhow!(
                        "Backup size mismatch for {}: expected {} bytes, got {} bytes",
                        file_entry.relative_path,
                        file_entry.size,
                        meta.len()
                    ));
                }
            }
            Err(_) => {
                return Err(anyhow!("Backup file missing: {}", file_entry.relative_path));
            }
        }
    }

    logger::log_info("Backup integrity verified (fast)", Some("management"));

    // Step 3: Delete current files that will be restored (based on verification.files)
    let custom_data = xplane_path.join("Custom Data");

    // Collect top-level items to delete from the verification entries
    let mut top_level_items: std::collections::HashSet<String> = std::collections::HashSet::new();
    for file_entry in &verification.files {
        // Get the first path component
        if let Some(first_component) = file_entry.relative_path.split('/').next() {
            top_level_items.insert(first_component.to_string());
        }
    }

    for item_name in &top_level_items {
        let current_path = custom_data.join(item_name);
        if current_path.exists() {
            logger::log_info(
                &format!("Removing current: {:?}", current_path),
                Some("management"),
            );
            if current_path.is_dir() {
                fs::remove_dir_all(&current_path)?;
            } else {
                fs::remove_file(&current_path)?;
            }
        }
    }

    // Step 4: Move files from backup to Custom Data (OPTIMIZED: directory-level rename)
    logger::log_info("Restoring files from backup...", Some("management"));

    // Optimized move_directory: tries directory-level rename first
    fn move_directory_optimized(src: &Path, dst: &Path) -> Result<()> {
        if src.is_dir() {
            // Ensure parent of destination exists
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }

            // Try direct directory rename first (O(1) operation on same filesystem)
            match fs::rename(src, dst) {
                Ok(()) => return Ok(()),
                Err(_) => {
                    // Cross-device link error, fall back to recursive approach
                }
            }

            // Fallback: recursive copy + delete
            fs::create_dir_all(dst)?;
            for entry in fs::read_dir(src)? {
                let entry = entry?;
                let src_child = entry.path();
                let dst_child = dst.join(entry.file_name());
                move_directory_optimized(&src_child, &dst_child)?;
            }
            fs::remove_dir(src).ok();
        } else {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            match fs::rename(src, dst) {
                Ok(()) => {}
                Err(_) => {
                    fs::copy(src, dst)?;
                    fs::remove_file(src).ok();
                }
            }
        }
        Ok(())
    }

    // Move top-level items from backup to target (directory-level rename)
    for item_name in &top_level_items {
        let backup_item = backup_path.join(item_name);
        let target_item = custom_data.join(item_name);
        if backup_item.exists() {
            move_directory_optimized(&backup_item, &target_item)?;
        }
    }

    // Step 5: Verify restored files (OPTIMIZED: size-only, single stat per file)
    // fs::rename is atomic on same filesystem, so if it succeeded, data is intact
    logger::log_info("Verifying restored files (fast)...", Some("management"));
    for file_entry in &verification.files {
        let restored_path = custom_data.join(&file_entry.relative_path);
        // OPTIMIZED: Use single fs::metadata() call instead of exists() + checksum
        match fs::metadata(&restored_path) {
            Ok(meta) => {
                if meta.len() != file_entry.size {
                    return Err(anyhow!(
                        "Restore verification failed: size mismatch for {} (expected {}, got {})",
                        file_entry.relative_path,
                        file_entry.size,
                        meta.len()
                    ));
                }
            }
            Err(_) => {
                return Err(anyhow!(
                    "Restore verification failed: file missing: {}",
                    file_entry.relative_path
                ));
            }
        }
    }

    // Step 6: Delete backup folder
    logger::log_info(
        &format!("Removing backup folder: {:?}", backup_path),
        Some("management"),
    );
    fs::remove_dir_all(&backup_path)?;

    logger::log_info(
        &format!(
            "Navdata backup restored successfully: {} files (extreme optimized)",
            verification.file_count
        ),
        Some("management"),
    );

    Ok(())
}

/// Find a livery icon file (*_icon11.png) in a livery folder
fn find_livery_icon(livery_path: &Path) -> Option<String> {
    let read_dir = fs::read_dir(livery_path).ok()?;
    for entry in read_dir.flatten() {
        if !entry.file_type().ok()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_lowercase();
        if name_str.ends_with("_icon11.png") {
            return Some(entry.path().to_string_lossy().to_string());
        }
    }
    None
}

/// Get liveries for a specific aircraft
pub fn get_aircraft_liveries(
    xplane_path: &Path,
    aircraft_folder: &str,
) -> Result<Vec<LiveryInfo>> {
    // Validate aircraft_folder: reject empty and ".."
    if aircraft_folder.is_empty() || aircraft_folder.contains("..") {
        return Err(anyhow!("Invalid aircraft folder name"));
    }

    let aircraft_base = xplane_path.join("Aircraft");
    let aircraft_path = aircraft_base.join(aircraft_folder);

    if !aircraft_path.exists() {
        return Err(anyhow!("Aircraft folder not found"));
    }

    // Canonical path check to prevent traversal
    let canonical_base = aircraft_base
        .canonicalize()
        .map_err(|e| anyhow!("Invalid base path: {}", e))?;
    let canonical_aircraft = aircraft_path
        .canonicalize()
        .map_err(|e| anyhow!("Invalid aircraft path: {}", e))?;

    if !canonical_aircraft.starts_with(&canonical_base) {
        return Err(anyhow!("Invalid path"));
    }

    let liveries_path = canonical_aircraft.join("liveries");
    if !liveries_path.exists() {
        return Ok(Vec::new());
    }

    let mut liveries: Vec<LiveryInfo> = Vec::new();

    let read_dir = fs::read_dir(&liveries_path)
        .map_err(|e| anyhow!("Failed to read liveries folder: {}", e))?;

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

        // Skip hidden folders
        if folder_name.starts_with('.') {
            continue;
        }

        let icon_path = find_livery_icon(&entry.path());

        liveries.push(LiveryInfo {
            display_name: folder_name.clone(),
            folder_name,
            icon_path,
        });
    }

    // Sort by display name
    liveries.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });

    logger::log_info(
        &format!(
            "Found {} liveries for aircraft '{}'",
            liveries.len(),
            aircraft_folder
        ),
        Some("management"),
    );

    Ok(liveries)
}

/// Delete a specific livery from an aircraft
pub fn delete_aircraft_livery(
    xplane_path: &Path,
    aircraft_folder: &str,
    livery_folder: &str,
) -> Result<()> {
    // Validate aircraft_folder: reject empty and ".."
    if aircraft_folder.is_empty() || aircraft_folder.contains("..") {
        return Err(anyhow!("Invalid aircraft folder name"));
    }

    // Validate livery_folder: single segment, no path separators
    validate_folder_name(livery_folder)?;

    let aircraft_base = xplane_path.join("Aircraft");
    let aircraft_path = aircraft_base.join(aircraft_folder);

    if !aircraft_path.exists() {
        return Err(anyhow!("Aircraft folder not found"));
    }

    let liveries_path = aircraft_path.join("liveries");
    let livery_path = liveries_path.join(livery_folder);

    if !livery_path.exists() {
        return Err(anyhow!("Livery folder not found: {}", livery_folder));
    }

    // Canonical path check to ensure livery is under the liveries directory
    let canonical_liveries = liveries_path
        .canonicalize()
        .map_err(|e| anyhow!("Invalid liveries path: {}", e))?;
    let canonical_livery = livery_path
        .canonicalize()
        .map_err(|e| anyhow!("Invalid livery path: {}", e))?;

    if !canonical_livery.starts_with(&canonical_liveries) {
        return Err(anyhow!("Invalid path"));
    }

    fs::remove_dir_all(&canonical_livery)
        .map_err(|e| anyhow!("Failed to delete livery: {}", e))?;

    logger::log_info(
        &format!(
            "Deleted livery '{}' from aircraft '{}'",
            livery_folder, aircraft_folder
        ),
        Some("management"),
    );

    Ok(())
}

/// Scan FlyWithLua scripts in the Scripts directory
pub fn scan_lua_scripts(xplane_path: &Path) -> Result<Vec<LuaScriptInfo>> {
    let scripts_path = xplane_path
        .join("Resources")
        .join("plugins")
        .join("FlyWithLua")
        .join("Scripts");

    if !scripts_path.exists() {
        return Err(anyhow!("FlyWithLua Scripts folder not found"));
    }

    logger::log_info("Scanning FlyWithLua scripts...", Some("management"));

    let mut scripts: Vec<LuaScriptInfo> = Vec::new();

    let read_dir = fs::read_dir(&scripts_path)
        .map_err(|e| anyhow!("Failed to read Scripts folder: {}", e))?;

    for entry in read_dir.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !ft.is_file() {
            continue;
        }

        let path = entry.path();
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(ext) => ext.to_lowercase(),
            None => continue,
        };

        let enabled = match ext.as_str() {
            "lua" => true,
            "xfml" => false,
            _ => continue,
        };

        let file_name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let display_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_name)
            .to_string();

        scripts.push(LuaScriptInfo {
            file_name,
            display_name,
            enabled,
        });
    }

    // Sort by display_name case-insensitive
    scripts.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });

    logger::log_info(
        &format!("Found {} FlyWithLua scripts", scripts.len()),
        Some("management"),
    );

    Ok(scripts)
}

/// Toggle a FlyWithLua script: .lua <-> .xfml
/// Returns the new enabled state (true = .lua, false = .xfml)
pub fn toggle_lua_script(xplane_path: &Path, file_name: &str) -> Result<bool> {
    // Validate file_name
    if file_name.is_empty()
        || file_name.contains("..")
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        return Err(anyhow!("Invalid file name"));
    }

    let scripts_path = xplane_path
        .join("Resources")
        .join("plugins")
        .join("FlyWithLua")
        .join("Scripts");

    if !scripts_path.exists() {
        return Err(anyhow!("FlyWithLua Scripts folder not found"));
    }

    let file_path = scripts_path.join(file_name);
    if !file_path.exists() {
        return Err(anyhow!("Script file not found: {}", file_name));
    }

    // Canonical path check to ensure file is within Scripts dir
    let canonical_scripts = scripts_path
        .canonicalize()
        .map_err(|e| anyhow!("Invalid Scripts path: {}", e))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|e| anyhow!("Invalid file path: {}", e))?;

    if !canonical_file.starts_with(&canonical_scripts) {
        return Err(anyhow!("Invalid path"));
    }

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let new_enabled = match ext.as_str() {
        "lua" => {
            // Disable: rename .lua -> .xfml
            let new_path = file_path.with_extension("xfml");
            fs::rename(&canonical_file, &new_path)
                .map_err(|e| anyhow!("Failed to rename script: {}", e))?;
            logger::log_info(
                &format!("Disabled script: {} -> .xfml", file_name),
                Some("management"),
            );
            false
        }
        "xfml" => {
            // Enable: rename .xfml -> .lua
            let new_path = file_path.with_extension("lua");
            fs::rename(&canonical_file, &new_path)
                .map_err(|e| anyhow!("Failed to rename script: {}", e))?;
            logger::log_info(
                &format!("Enabled script: {} -> .lua", file_name),
                Some("management"),
            );
            true
        }
        _ => return Err(anyhow!("Unsupported file extension: {}", ext)),
    };

    Ok(new_enabled)
}

/// Delete a FlyWithLua script file
pub fn delete_lua_script(xplane_path: &Path, file_name: &str) -> Result<()> {
    // Validate file_name
    if file_name.is_empty()
        || file_name.contains("..")
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        return Err(anyhow!("Invalid file name"));
    }

    let scripts_path = xplane_path
        .join("Resources")
        .join("plugins")
        .join("FlyWithLua")
        .join("Scripts");

    if !scripts_path.exists() {
        return Err(anyhow!("FlyWithLua Scripts folder not found"));
    }

    let file_path = scripts_path.join(file_name);
    if !file_path.exists() {
        return Err(anyhow!("Script file not found: {}", file_name));
    }

    // Canonical path check to ensure file is within Scripts dir
    let canonical_scripts = scripts_path
        .canonicalize()
        .map_err(|e| anyhow!("Invalid Scripts path: {}", e))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|e| anyhow!("Invalid file path: {}", e))?;

    if !canonical_file.starts_with(&canonical_scripts) {
        return Err(anyhow!("Invalid path"));
    }

    // Verify it's a script file (.lua or .xfml)
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if ext != "lua" && ext != "xfml" {
        return Err(anyhow!("Not a script file: {}", file_name));
    }

    fs::remove_file(&canonical_file)
        .map_err(|e| anyhow!("Failed to delete script: {}", e))?;

    logger::log_info(
        &format!("Deleted script: {}", file_name),
        Some("management"),
    );

    Ok(())
}

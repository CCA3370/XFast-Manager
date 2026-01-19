//! Scenery packs.ini manager module
//!
//! This module parses, sorts, and writes the scenery_packs.ini file
//! based on scenery classifications.

use crate::logger;
use crate::models::{SceneryCategory, SceneryIndex, SceneryPackEntry};
use crate::scenery_index::SceneryIndexManager;
use anyhow::{anyhow, Result};
use chrono::Local;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const INI_HEADER: &str = "I\n1000 Version\nSCENERY\n\n";

/// Resolve Windows shortcut (.lnk) to actual path for sorting purposes
/// This is a simplified version that doesn't log (to avoid spam during sorting)
#[cfg(windows)]
fn resolve_shortcut_for_sorting(lnk_path: &Path) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use winapi::shared::guiddef::GUID;
    use winapi::shared::minwindef::MAX_PATH;
    use winapi::shared::winerror::{S_OK, S_FALSE, RPC_E_CHANGED_MODE};
    use winapi::um::combaseapi::{CoCreateInstance, CoInitializeEx, CoUninitialize};
    use winapi::um::objbase::COINIT_APARTMENTTHREADED;
    use winapi::um::objidl::IPersistFile;
    use winapi::um::shobjidl_core::IShellLinkW;
    use winapi::Interface;

    const CLSID_SHELL_LINK: GUID = GUID {
        Data1: 0x00021401,
        Data2: 0x0000,
        Data3: 0x0000,
        Data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    };

    unsafe {
        let hr = CoInitializeEx(ptr::null_mut(), COINIT_APARTMENTTHREADED);
        let need_uninit = hr == S_OK || hr == S_FALSE || hr == RPC_E_CHANGED_MODE;

        let mut result = None;
        let mut shell_link: *mut IShellLinkW = ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_SHELL_LINK,
            ptr::null_mut(),
            1, // CLSCTX_INPROC_SERVER
            &IShellLinkW::uuidof(),
            &mut shell_link as *mut *mut _ as *mut *mut _,
        );

        if hr == S_OK && !shell_link.is_null() {
            let mut persist_file: *mut IPersistFile = ptr::null_mut();
            let hr = (*shell_link).QueryInterface(
                &IPersistFile::uuidof(),
                &mut persist_file as *mut *mut _ as *mut *mut _,
            );

            if hr == S_OK && !persist_file.is_null() {
                let wide_path: Vec<u16> = lnk_path
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let hr = (*persist_file).Load(wide_path.as_ptr(), 0);
                if hr == S_OK {
                    let mut target_path = [0u16; MAX_PATH];
                    let hr = (*shell_link).GetPath(
                        target_path.as_mut_ptr(),
                        MAX_PATH as i32,
                        ptr::null_mut(),
                        0,
                    );

                    if hr == S_OK {
                        let len = target_path.iter().position(|&c| c == 0).unwrap_or(MAX_PATH);
                        if len > 0 {
                            let path_str = String::from_utf16_lossy(&target_path[..len]);
                            result = Some(PathBuf::from(path_str));
                        }
                    }
                }
                (*persist_file).Release();
            }
            (*shell_link).Release();
        }

        if need_uninit {
            CoUninitialize();
        }

        result
    }
}

#[cfg(not(windows))]
fn resolve_shortcut_for_sorting(_lnk_path: &Path) -> Option<PathBuf> {
    None
}

/// Manager for scenery_packs.ini operations
pub struct SceneryPacksManager {
    xplane_path: PathBuf,
    ini_path: PathBuf,
}

impl SceneryPacksManager {
    /// Create a new manager
    pub fn new(xplane_path: &Path) -> Self {
        let ini_path = xplane_path.join("Custom Scenery").join("scenery_packs.ini");
        Self {
            xplane_path: xplane_path.to_path_buf(),
            ini_path,
        }
    }

    /// Parse scenery_packs.ini file
    pub fn parse_ini(&self) -> Result<Vec<SceneryPackEntry>> {
        if !self.ini_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.ini_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            // Skip empty lines and header lines
            if trimmed.is_empty()
                || trimmed == "I"
                || trimmed.starts_with("1000")
                || trimmed == "SCENERY"
            {
                continue;
            }

            // Parse SCENERY_PACK or SCENERY_PACK_DISABLED lines
            if let Some(entry) = parse_entry_line(trimmed) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Sort entries by scenery category priority
    pub fn sort_entries(
        &self,
        entries: &[SceneryPackEntry],
        index: &SceneryIndex,
    ) -> Vec<SceneryPackEntry> {
        let mut sorted = entries.to_vec();

        // Sort by priority first, then by folder name within same priority
        sorted.sort_by(|a, b| {
            let priority_a = self.get_entry_priority(a, index);
            let priority_b = self.get_entry_priority(b, index);

            // First compare by priority
            match priority_a.cmp(&priority_b) {
                std::cmp::Ordering::Equal => {
                    // If priorities are equal, sort by folder name (case-insensitive)
                    // For shortcuts, use the target folder name
                    let name_a = self.get_actual_folder_name(&a.path, index);
                    let name_b = self.get_actual_folder_name(&b.path, index);
                    name_a.cmp(&name_b)
                }
                other => other,
            }
        });

        sorted
    }

    /// Get actual folder name for sorting (resolves shortcuts to target name)
    fn get_actual_folder_name(&self, path: &str, index: &SceneryIndex) -> String {
        let folder_name = extract_folder_name(path);
        let folder_name_lower = folder_name.to_lowercase();

        // If it's a .lnk file, try to get the target folder name from index
        if folder_name_lower.ends_with(".lnk") {
            let full_path = self.xplane_path.join("Custom Scenery").join(&folder_name);
            if let Some(target) = resolve_shortcut_for_sorting(&full_path) {
                if let Some(target_name) = target.file_name().and_then(|s| s.to_str()) {
                    // Check if this target exists in index
                    if index.packages.contains_key(target_name) {
                        return target_name.to_lowercase();
                    }
                }
            }
        }

        // Otherwise, use the folder name as-is
        folder_name_lower
    }

    /// Get sorting priority for an entry (category priority, sub-priority)
    /// Returns (u8, u8) where first is category, second is sub-priority within category
    /// All classification is done during indexing, not here
    fn get_entry_priority(&self, entry: &SceneryPackEntry, index: &SceneryIndex) -> (u8, u8) {
        // *GLOBAL_AIRPORTS* always goes to DefaultAirport position
        if entry.is_global_airports {
            return (SceneryCategory::DefaultAirport.priority(), 0);
        }

        // Extract folder name from path
        let folder_name = extract_folder_name(&entry.path);
        let folder_name_lower = folder_name.to_lowercase();

        // Look up in index - first try direct lookup
        let info_opt = index.packages.get(&folder_name).or_else(|| {
            // If not found and it's a .lnk file, try to resolve and lookup by target name
            if folder_name_lower.ends_with(".lnk") {
                let full_path = self.xplane_path.join("Custom Scenery").join(&folder_name);
                if let Some(target) = resolve_shortcut_for_sorting(&full_path) {
                    if let Some(target_name) = target.file_name().and_then(|s| s.to_str()) {
                        return index.packages.get(target_name);
                    }
                }
            }
            None
        });

        if let Some(info) = info_opt {
            // Use category priority and sub-priority from index
            // No classification logic here - everything is pre-computed in the index
            return (info.category.priority(), info.sub_priority);
        }

        // Default to Other category for unknown entries
        (SceneryCategory::Other.priority(), 0)
    }

    /// Write sorted entries back to scenery_packs.ini
    pub fn write_ini(&self, entries: &[SceneryPackEntry]) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = self.ini_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temp file first for atomic write
        let temp_path = self.ini_path.with_extension("ini.tmp");
        let mut file = fs::File::create(&temp_path)?;

        // Write header
        file.write_all(INI_HEADER.as_bytes())?;

        // Write entries
        for entry in entries {
            let prefix = if entry.enabled {
                "SCENERY_PACK"
            } else {
                "SCENERY_PACK_DISABLED"
            };

            // Ensure path ends with / (except for *GLOBAL_AIRPORTS*)
            let path = if entry.is_global_airports {
                // *GLOBAL_AIRPORTS* should not have trailing slash
                entry.path.clone()
            } else if entry.path.ends_with('/') || entry.path.ends_with('\\') {
                entry.path.clone()
            } else {
                format!("{}/", entry.path)
            };

            writeln!(file, "{} {}", prefix, path)?;
        }

        // Atomic rename
        fs::rename(&temp_path, &self.ini_path)?;

        Ok(())
    }

    /// Create a backup of scenery_packs.ini
    pub fn backup_ini(&self) -> Result<PathBuf> {
        if !self.ini_path.exists() {
            return Err(anyhow!("scenery_packs.ini does not exist"));
        }

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("scenery_packs.ini.backup.{}", timestamp);
        let backup_path = self.ini_path.parent().unwrap().join(backup_name);

        fs::copy(&self.ini_path, &backup_path)?;
        logger::log_info(&format!("Created backup: {:?}", backup_path), Some("scenery_packs"));

        Ok(backup_path)
    }

    /// Auto-sort scenery_packs.ini
    pub fn auto_sort(&self) -> Result<()> {
        // Load or rebuild index
        let index_manager = SceneryIndexManager::new(&self.xplane_path);
        let index = index_manager.update_index()?;

        if index.packages.is_empty() {
            logger::log_info("No scenery packages in index, nothing to sort", Some("scenery_packs"));
            return Ok(());
        }

        // Parse current ini to preserve *GLOBAL_AIRPORTS* marker and enabled/disabled state
        let existing_entries = self.parse_ini()?;

        // Separate *GLOBAL_AIRPORTS* from regular entries
        let mut global_airports_entry: Option<SceneryPackEntry> = None;
        let mut regular_entries = Vec::new();

        for entry in existing_entries {
            if entry.is_global_airports {
                global_airports_entry = Some(entry);
            } else {
                regular_entries.push(entry);
            }
        }

        // Create a map of regular entries for quick lookup
        let existing_map: std::collections::HashMap<String, SceneryPackEntry> = regular_entries
            .into_iter()
            .map(|entry| {
                let folder_name = extract_folder_name(&entry.path);
                (folder_name, entry)
            })
            .collect();

        // Create backup if ini exists
        if self.ini_path.exists() {
            if let Err(e) = self.backup_ini() {
                logger::log_info(&format!("Failed to create backup: {}", e), Some("scenery_packs"));
            }
        }

        // Build entries from index
        let mut entries: Vec<SceneryPackEntry> = index
            .packages
            .iter()
            .map(|(folder_name, _info)| {
                // Check if entry existed before to preserve enabled state
                let enabled = existing_map
                    .get(folder_name)
                    .map(|e| e.enabled)
                    .unwrap_or(true); // Default to enabled for new entries

                SceneryPackEntry {
                    enabled,
                    path: format!("Custom Scenery/{}/", folder_name),
                    is_global_airports: false,
                }
            })
            .collect();

        // Add *GLOBAL_AIRPORTS* marker
        // If it existed before, use the existing entry to preserve enabled state
        // Otherwise, create a new enabled entry
        let global_airports = global_airports_entry.unwrap_or_else(|| {
            SceneryPackEntry {
                enabled: true,
                path: "*GLOBAL_AIRPORTS*".to_string(),
                is_global_airports: true,
            }
        });
        entries.push(global_airports);

        // Sort entries
        let sorted = self.sort_entries(&entries, &index);

        // Write sorted entries
        self.write_ini(&sorted)?;

        logger::log_info(
            &format!("Sorted {} scenery entries", sorted.len()),
            Some("scenery_packs")
        );

        Ok(())
    }

    /// Add a new entry to scenery_packs.ini (used after installation)
    pub fn add_entry(&self, folder_name: &str, category: &SceneryCategory) -> Result<()> {
        let mut entries = self.parse_ini()?;

        // Check if entry already exists
        let path = format!("Custom Scenery/{}/", folder_name);
        if entries.iter().any(|e| e.path == path) {
            crate::log_debug!(&format!("Entry already exists: {}", path), "scenery_packs");
            return Ok(());
        }

        // Create new entry
        let new_entry = SceneryPackEntry {
            enabled: true,
            path,
            is_global_airports: false,
        };

        // Find insertion point based on category priority
        let index_manager = SceneryIndexManager::new(&self.xplane_path);
        let index = index_manager.load_index()?;

        let new_priority = (category.priority(), 0); // Default sub-priority
        let insert_pos = entries
            .iter()
            .position(|e| self.get_entry_priority(e, &index) > new_priority)
            .unwrap_or(entries.len());

        entries.insert(insert_pos, new_entry);

        // Write updated ini
        self.write_ini(&entries)?;

        Ok(())
    }

    /// Remove an entry from scenery_packs.ini
    pub fn remove_entry(&self, folder_name: &str) -> Result<bool> {
        let mut entries = self.parse_ini()?;
        let path = format!("Custom Scenery/{}/", folder_name);

        let initial_len = entries.len();
        entries.retain(|e| !e.path.eq_ignore_ascii_case(&path));

        if entries.len() < initial_len {
            self.write_ini(&entries)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Ensure all installed scenery is in scenery_packs.ini
    pub fn sync_with_folder(&self) -> Result<usize> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        if !custom_scenery_path.exists() {
            return Ok(0);
        }

        let mut entries = self.parse_ini()?;
        let mut added_count = 0;

        // Get existing paths (normalized)
        let existing_paths: std::collections::HashSet<String> = entries
            .iter()
            .map(|e| e.path.to_lowercase().replace('\\', "/"))
            .collect();

        // Find folders not in ini (including symlinks)
        for entry in fs::read_dir(&custom_scenery_path)? {
            let entry = entry?;
            // Use metadata() to follow symlinks
            if !entry.path().metadata().map(|m| m.is_dir()).unwrap_or(false) {
                continue;
            }

            let folder_name = entry.file_name().to_string_lossy().to_string();
            let path = format!("Custom Scenery/{}/", folder_name);
            let path_lower = path.to_lowercase();

            if !existing_paths.contains(&path_lower) {
                entries.push(SceneryPackEntry {
                    enabled: true,
                    path,
                    is_global_airports: false,
                });
                added_count += 1;
            }
        }

        if added_count > 0 {
            // Sort after adding new entries
            let index_manager = SceneryIndexManager::new(&self.xplane_path);
            let index = index_manager.update_index()?;
            let sorted = self.sort_entries(&entries, &index);
            self.write_ini(&sorted)?;
        }

        Ok(added_count)
    }

    /// Get entry count by category
    pub fn get_category_counts(&self) -> Result<std::collections::HashMap<String, usize>> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path);
        let index = index_manager.load_index()?;
        let entries = self.parse_ini()?;

        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for entry in entries {
            let folder_name = extract_folder_name(&entry.path);
            let category = if entry.is_global_airports {
                "DefaultAirport".to_string()
            } else if let Some(info) = index.packages.get(&folder_name) {
                format!("{:?}", info.category)
            } else {
                "Other".to_string()
            };

            *counts.entry(category).or_insert(0) += 1;
        }

        Ok(counts)
    }

    /// Sort scenery_packs.ini based entirely on index sort_order
    /// This is used by the scenery manager after manual reordering
    pub fn auto_sort_from_index(&self) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path);
        let index = index_manager.load_index()?;

        if index.packages.is_empty() {
            logger::log_info("No scenery packages in index, nothing to sort", Some("scenery_packs"));
            return Ok(());
        }

        // Create backup if ini exists
        if self.ini_path.exists() {
            if let Err(e) = self.backup_ini() {
                logger::log_info(&format!("Failed to create backup: {}", e), Some("scenery_packs"));
            }
        }

        // Build entries from index, sorted by sort_order
        let mut packages: Vec<_> = index.packages.values().collect();
        packages.sort_by_key(|p| p.sort_order);

        let mut entries: Vec<SceneryPackEntry> = Vec::new();
        let mut global_airports_inserted = false;

        for info in packages {
            // Insert *GLOBAL_AIRPORTS* before the first DefaultAirport entry
            // (DefaultAirport has priority 2)
            if !global_airports_inserted && info.category.priority() >= SceneryCategory::DefaultAirport.priority() {
                entries.push(SceneryPackEntry {
                    enabled: true,
                    path: "*GLOBAL_AIRPORTS*".to_string(),
                    is_global_airports: true,
                });
                global_airports_inserted = true;
            }

            entries.push(SceneryPackEntry {
                enabled: info.enabled,
                path: format!("Custom Scenery/{}/", info.folder_name),
                is_global_airports: false,
            });
        }

        // If *GLOBAL_AIRPORTS* wasn't inserted yet, add it at the end
        if !global_airports_inserted {
            entries.push(SceneryPackEntry {
                enabled: true,
                path: "*GLOBAL_AIRPORTS*".to_string(),
                is_global_airports: true,
            });
        }

        // Write sorted entries
        self.write_ini(&entries)?;

        logger::log_info(
            &format!("Sorted {} scenery entries from index", entries.len()),
            Some("scenery_packs")
        );

        Ok(())
    }

    /// Apply index state (enabled/sort_order) to scenery_packs.ini
    /// This preserves the order from the index and applies enabled states
    pub fn apply_from_index(&self) -> Result<()> {
        // This is essentially the same as auto_sort_from_index
        // but we call it explicitly to make the intent clear
        self.auto_sort_from_index()
    }
}

/// Parse a single entry line from scenery_packs.ini
fn parse_entry_line(line: &str) -> Option<SceneryPackEntry> {
    let (enabled, path_part) = if line.starts_with("SCENERY_PACK_DISABLED ") {
        (false, line.strip_prefix("SCENERY_PACK_DISABLED ")?)
    } else if line.starts_with("SCENERY_PACK ") {
        (true, line.strip_prefix("SCENERY_PACK ")?)
    } else {
        return None;
    };

    let path = path_part.trim().to_string();
    let is_global_airports = path.contains("*GLOBAL_AIRPORTS*");

    Some(SceneryPackEntry {
        enabled,
        path,
        is_global_airports,
    })
}

/// Extract folder name from scenery path
fn extract_folder_name(path: &str) -> String {
    // Path format: "Custom Scenery/FolderName/" or "Custom Scenery/FolderName"
    let path = path.trim_end_matches('/').trim_end_matches('\\');
    path.rsplit(&['/', '\\'][..])
        .next()
        .unwrap_or(path)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entry_line() {
        let entry = parse_entry_line("SCENERY_PACK Custom Scenery/MyAirport/").unwrap();
        assert!(entry.enabled);
        assert_eq!(entry.path, "Custom Scenery/MyAirport/");
        assert!(!entry.is_global_airports);

        let entry = parse_entry_line("SCENERY_PACK_DISABLED Custom Scenery/OldScenery/").unwrap();
        assert!(!entry.enabled);

        let entry = parse_entry_line("SCENERY_PACK *GLOBAL_AIRPORTS*").unwrap();
        assert!(entry.is_global_airports);
        assert_eq!(entry.path, "*GLOBAL_AIRPORTS*");

        // Test that *GLOBAL_AIRPORTS* can be parsed with or without trailing slash
        let entry_with_slash = parse_entry_line("SCENERY_PACK *GLOBAL_AIRPORTS*/").unwrap();
        assert!(entry_with_slash.is_global_airports);

        assert!(parse_entry_line("invalid line").is_none());
    }

    #[test]
    fn test_extract_folder_name() {
        assert_eq!(
            extract_folder_name("Custom Scenery/MyAirport/"),
            "MyAirport"
        );
        assert_eq!(
            extract_folder_name("Custom Scenery/MyAirport"),
            "MyAirport"
        );
        assert_eq!(
            extract_folder_name("Custom Scenery\\MyAirport\\"),
            "MyAirport"
        );
    }

    #[test]
    fn test_category_priority_order() {
        // Verify priority order matches design
        assert!(SceneryCategory::FixedHighPriority.priority() < SceneryCategory::Airport.priority());
        assert!(SceneryCategory::Airport.priority() < SceneryCategory::DefaultAirport.priority());
        assert!(SceneryCategory::DefaultAirport.priority() < SceneryCategory::Library.priority());
        assert!(SceneryCategory::Library.priority() < SceneryCategory::Other.priority());
        assert!(SceneryCategory::Other.priority() < SceneryCategory::Overlay.priority());
        assert!(SceneryCategory::Overlay.priority() < SceneryCategory::Orthophotos.priority());
        // Orthophotos and Mesh share the same priority (6), use sub-priority to distinguish
        assert_eq!(SceneryCategory::Orthophotos.priority(), SceneryCategory::Mesh.priority());
    }
}

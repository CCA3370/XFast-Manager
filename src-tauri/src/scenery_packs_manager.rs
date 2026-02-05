//! Scenery packs.ini manager module
//!
//! This module writes and sorts the scenery_packs.ini file using the index as source of truth
//! based on scenery classifications.

use crate::logger;
use crate::models::{SceneryCategory, SceneryPackEntry};
use crate::scenery_index::SceneryIndexManager;
use anyhow::{anyhow, Result};
use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const INI_HEADER: &str = "I\n1000 Version\nSCENERY\n\n";

/// Normalize a scenery path for scenery_packs.ini
/// Converts backslashes to forward slashes and ensures trailing slash
fn normalize_scenery_path(path: &str) -> String {
    // Convert all backslashes to forward slashes (X-Plane uses forward slashes)
    let normalized = path.replace('\\', "/");
    // Ensure trailing slash
    if normalized.ends_with('/') {
        normalized
    } else {
        format!("{}/", normalized)
    }
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

            // Normalize path: convert backslashes to forward slashes and ensure trailing slash
            // *GLOBAL_AIRPORTS* is special and should not be modified
            let path = if entry.is_global_airports {
                entry.path.clone()
            } else {
                normalize_scenery_path(&entry.path)
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

        let parent_dir = self
            .ini_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid ini path: no parent directory"))?;

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("scenery_packs.ini.backup.{}", timestamp);
        let backup_path = parent_dir.join(&backup_name);

        fs::rename(&self.ini_path, &backup_path)?;
        logger::log_info(
            &format!("Created backup: {:?}", backup_path),
            Some("scenery_packs"),
        );

        // Clean up old backups, keeping only the 3 most recent
        self.cleanup_old_backups(parent_dir, 3);

        Ok(backup_path)
    }

    /// Remove old backup files, keeping only the specified number of most recent backups
    fn cleanup_old_backups(&self, dir: &std::path::Path, keep_count: usize) {
        let mut backups: Vec<_> = match fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("scenery_packs.ini.backup.")
                })
                .collect(),
            Err(_) => return,
        };

        if backups.len() <= keep_count {
            return;
        }

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| {
            let time_a = a.metadata().and_then(|m| m.modified()).ok();
            let time_b = b.metadata().and_then(|m| m.modified()).ok();
            time_b.cmp(&time_a)
        });

        // Remove old backups (skip the first `keep_count`)
        for backup in backups.into_iter().skip(keep_count) {
            if let Err(e) = fs::remove_file(backup.path()) {
                logger::log_info(
                    &format!("Failed to remove old backup {:?}: {}", backup.path(), e),
                    Some("scenery_packs"),
                );
            } else {
                logger::log_info(
                    &format!("Removed old backup: {:?}", backup.path()),
                    Some("scenery_packs"),
                );
            }
        }
    }

    /// Add a new entry to scenery_packs.ini (used after installation)
    pub fn add_entry(&self, folder_name: &str, category: &SceneryCategory) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path);

        // If index hasn't been created yet, don't add to index or sort
        // User hasn't built the index, so we shouldn't automatically manage scenery order
        if !index_manager.has_index()? {
            logger::log_info(
                &format!(
                    "Skipping scenery indexing for '{}': index not yet created",
                    folder_name
                ),
                Some("scenery_packs"),
            );
            return Ok(());
        }

        let folder_path = self.xplane_path.join("Custom Scenery").join(folder_name);

        let info = index_manager.get_or_classify(&folder_path)?;
        if &info.category != category {
            index_manager.update_entry(folder_name, None, None, Some(category.clone()))?;
        }

        let _ = index_manager.reset_sort_order()?;
        self.auto_sort_from_index()
    }

    /// Ensure all installed scenery is in scenery_packs.ini
    /// Only performs incremental indexing if the index has been created
    pub fn sync_with_folder(&self) -> Result<usize> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        if !custom_scenery_path.exists() {
            return Ok(0);
        }

        let index_manager = SceneryIndexManager::new(&self.xplane_path);

        // If index hasn't been created yet, don't perform incremental indexing
        // User hasn't built the index, so we shouldn't automatically update it
        if !index_manager.has_index()? {
            logger::log_info(
                "Skipping sync_with_folder: index not yet created",
                Some("scenery_packs"),
            );
            return Ok(0);
        }

        let before_index = index_manager.load_index()?;
        let before_keys: std::collections::HashSet<String> =
            before_index.packages.keys().cloned().collect();

        let updated_index = index_manager.update_index()?;
        let after_keys: std::collections::HashSet<String> =
            updated_index.packages.keys().cloned().collect();

        let added_count = after_keys.difference(&before_keys).count();

        if before_keys != after_keys {
            self.auto_sort_from_index()?;
        }

        Ok(added_count)
    }

    /// Sort scenery_packs.ini based entirely on index sort_order
    /// This is used by the scenery manager after manual reordering
    pub fn auto_sort_from_index(&self) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path);
        let index = index_manager.load_index()?;

        if index.packages.is_empty() {
            logger::log_info(
                "No scenery packages in index, nothing to sort",
                Some("scenery_packs"),
            );
            return Ok(());
        }

        // Create backup if ini exists
        if self.ini_path.exists() {
            if let Err(e) = self.backup_ini() {
                logger::log_info(
                    &format!("Failed to create backup: {}", e),
                    Some("scenery_packs"),
                );
            }
        }

        // Build entries from index, sorted by sort_order
        let mut packages: Vec<_> = index.packages.values().collect();
        packages.sort_by_key(|p| p.sort_order);

        let mut entries: Vec<SceneryPackEntry> = Vec::new();
        let mut global_airports_inserted = false;

        for info in packages {
            // Skip disabled Unrecognized packages - they should not be written to scenery_packs.ini
            // Enabled Unrecognized packages WILL be written to ini
            if info.category == SceneryCategory::Unrecognized && !info.enabled {
                continue;
            }

            // Insert *GLOBAL_AIRPORTS* before the first DefaultAirport entry
            // (DefaultAirport has priority 2)
            if !global_airports_inserted
                && info.category.priority() >= SceneryCategory::DefaultAirport.priority()
            {
                entries.push(SceneryPackEntry {
                    enabled: true,
                    path: "*GLOBAL_AIRPORTS*".to_string(),
                    is_global_airports: true,
                });
                global_airports_inserted = true;
            }

            // Use actual_path if set (for shortcuts pointing outside Custom Scenery),
            // otherwise use the standard Custom Scenery/{folder_name}/ format
            let path = if let Some(actual_path) = &info.actual_path {
                actual_path.clone()
            } else {
                format!("Custom Scenery/{}/", info.folder_name)
            };

            entries.push(SceneryPackEntry {
                enabled: info.enabled,
                path,
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
            Some("scenery_packs"),
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

    /// Check if ini file is in sync with the index
    /// Returns true if ini order/enabled states match index for entries that exist in the index
    /// Note: Extra entries in the ini (manually added) are ignored
    pub fn is_synced_with_index(&self) -> Result<bool> {
        // If ini doesn't exist, it's not synced
        if !self.ini_path.exists() {
            return Ok(false);
        }

        let index_manager = SceneryIndexManager::new(&self.xplane_path);
        let index = index_manager.load_index()?;

        if index.packages.is_empty() {
            return Ok(true);
        }

        // Read current ini file
        let content = fs::read_to_string(&self.ini_path)?;

        // Parse ini entries (order matters)
        let mut ini_entries: Vec<(String, bool)> = Vec::new(); // (folder_name, enabled)
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("SCENERY_PACK_DISABLED ") {
                let path = line.trim_start_matches("SCENERY_PACK_DISABLED ");
                if let Some(folder) = extract_folder_name(path) {
                    ini_entries.push((folder, false));
                }
            } else if line.starts_with("SCENERY_PACK ") {
                let path = line.trim_start_matches("SCENERY_PACK ");
                // Skip *GLOBAL_AIRPORTS*
                if !path.contains("*GLOBAL_AIRPORTS*") {
                    if let Some(folder) = extract_folder_name(path) {
                        ini_entries.push((folder, true));
                    }
                }
            }
        }

        // Build expected order from index
        // Exclude disabled Unrecognized packages (they are not written to ini)
        let mut packages: Vec<_> = index
            .packages
            .values()
            .filter(|p| p.category != SceneryCategory::Unrecognized || p.enabled)
            .collect();
        packages.sort_by_key(|p| p.sort_order);

        // Filter ini entries to only include entries that exist in the index
        // This allows manually added entries in the ini to be ignored
        let ini_entries_filtered: Vec<_> = ini_entries
            .iter()
            .filter(|(folder, _)| index.packages.contains_key(folder))
            .collect();

        // Compare count of index entries
        if ini_entries_filtered.len() != packages.len() {
            return Ok(false);
        }

        // Compare order and enabled state for entries that exist in the index
        for (i, pkg) in packages.iter().enumerate() {
            let (ini_folder, ini_enabled) = ini_entries_filtered[i];
            if ini_folder != &pkg.folder_name || *ini_enabled != pkg.enabled {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Extract folder name from ini path
/// e.g., "Custom Scenery/MyScenery/" -> "MyScenery"
fn extract_folder_name(path: &str) -> Option<String> {
    let path = path.trim().trim_end_matches('/').trim_end_matches('\\');
    if let Some(idx) = path.rfind('/').or_else(|| path.rfind('\\')) {
        Some(path[idx + 1..].to_string())
    } else {
        Some(path.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_priority_order() {
        // Verify priority order matches design
        assert!(
            SceneryCategory::FixedHighPriority.priority() < SceneryCategory::Airport.priority()
        );
        assert!(SceneryCategory::Airport.priority() < SceneryCategory::DefaultAirport.priority());
        assert!(SceneryCategory::DefaultAirport.priority() < SceneryCategory::Library.priority());
        assert!(SceneryCategory::Library.priority() < SceneryCategory::Other.priority());
        assert!(SceneryCategory::Other.priority() < SceneryCategory::Overlay.priority());
        assert!(SceneryCategory::Overlay.priority() < SceneryCategory::AirportMesh.priority());
        assert!(SceneryCategory::AirportMesh.priority() < SceneryCategory::Mesh.priority());
        assert!(SceneryCategory::Mesh.priority() < SceneryCategory::Unrecognized.priority());
    }
}

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
        let backup_path = self
            .ini_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid ini path: no parent directory"))?
            .join(backup_name);

        fs::rename(&self.ini_path, &backup_path)?;
        logger::log_info(
            &format!("Created backup: {:?}", backup_path),
            Some("scenery_packs"),
        );

        Ok(backup_path)
    }

    /// Add a new entry to scenery_packs.ini (used after installation)
    pub fn add_entry(&self, folder_name: &str, category: &SceneryCategory) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path);
        let folder_path = self.xplane_path.join("Custom Scenery").join(folder_name);

        let info = index_manager.get_or_classify(&folder_path)?;
        if &info.category != category {
            index_manager.update_entry(folder_name, None, None, Some(category.clone()))?;
        }

        let _ = index_manager.reset_sort_order()?;
        self.auto_sort_from_index()
    }

    /// Ensure all installed scenery is in scenery_packs.ini
    pub fn sync_with_folder(&self) -> Result<usize> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        if !custom_scenery_path.exists() {
            return Ok(0);
        }

        let index_manager = SceneryIndexManager::new(&self.xplane_path);
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
        assert!(SceneryCategory::Overlay.priority() < SceneryCategory::Orthophotos.priority());
        // Orthophotos and Mesh share the same priority (6), use sub-priority to distinguish
        assert_eq!(
            SceneryCategory::Orthophotos.priority(),
            SceneryCategory::Mesh.priority()
        );
    }
}

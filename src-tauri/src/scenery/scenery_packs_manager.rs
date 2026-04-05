//! Scenery packs.ini manager module
//!
//! This module writes and sorts the scenery_packs.ini file using the index as source of truth
//! based on scenery classifications.

use crate::database::SceneryQueries;
use crate::logger;
use crate::models::{
    is_global_airports_folder_name, SceneryCategory, SceneryPackEntry, SceneryPackageInfo,
    GLOBAL_AIRPORTS_ENTRY_NAME,
};
use crate::scenery_index::SceneryIndexManager;
use anyhow::{anyhow, Result};
use chrono::Local;
use sea_orm::DatabaseConnection;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const INI_HEADER: &str = "I\n1000 Version\nSCENERY\n\n";
const GLOBAL_AIRPORTS_ENABLED_METADATA_KEY: &str = "global_airports_enabled";
const GLOBAL_AIRPORTS_SORT_ORDER_METADATA_KEY: &str = "global_airports_sort_order";
const GLOBAL_AIRPORTS_CATEGORY_METADATA_KEY: &str = "global_airports_category";

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

fn entry_path_for_ini(entry: &SceneryPackEntry) -> String {
    if entry.is_global_airports {
        entry.path.clone()
    } else {
        normalize_scenery_path(&entry.path)
    }
}

fn is_global_airports_package(info: &SceneryPackageInfo) -> bool {
    info.category == SceneryCategory::DefaultAirport
        || is_global_airports_folder_name(&info.folder_name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalAirportsState {
    pub enabled: bool,
    pub sort_order: u32,
    pub category: SceneryCategory,
}

fn sort_visible_entries<T>(entries: &mut [(u32, bool, T)]) {
    entries.sort_by(|(sort_a, global_a, _), (sort_b, global_b, _)| {
        sort_a.cmp(sort_b).then_with(|| global_b.cmp(global_a))
    });
}

fn build_entries_from_sorted_packages(
    packages: &[&SceneryPackageInfo],
    global_airports: &GlobalAirportsState,
) -> Vec<SceneryPackEntry> {
    let mut entries: Vec<(u32, bool, SceneryPackEntry)> = Vec::new();

    for info in packages {
        if is_global_airports_package(info) {
            continue;
        }

        // Skip disabled Unrecognized packages - they should not be written to scenery_packs.ini
        // Enabled Unrecognized packages WILL be written to ini
        if info.category == SceneryCategory::Unrecognized && !info.enabled {
            continue;
        }

        let path = if let Some(actual_path) = &info.actual_path {
            actual_path.clone()
        } else {
            format!("Custom Scenery/{}/", info.folder_name)
        };

        entries.push((
            info.sort_order,
            false,
            SceneryPackEntry {
                enabled: info.enabled,
                path,
                is_global_airports: false,
            },
        ));
    }

    entries.push((
        global_airports.sort_order,
        true,
        SceneryPackEntry {
            enabled: global_airports.enabled,
            path: GLOBAL_AIRPORTS_ENTRY_NAME.to_string(),
            is_global_airports: true,
        },
    ));

    sort_visible_entries(&mut entries);
    entries.into_iter().map(|(_, _, entry)| entry).collect()
}

fn parse_ini_entries(content: &str) -> Vec<SceneryPackEntry> {
    let mut entries = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        let (enabled, path) = if let Some(path) = line.strip_prefix("SCENERY_PACK_DISABLED ") {
            (false, path)
        } else if let Some(path) = line.strip_prefix("SCENERY_PACK ") {
            (true, path)
        } else {
            continue;
        };

        let is_global_airports = path.contains("*GLOBAL_AIRPORTS*");
        entries.push(SceneryPackEntry {
            enabled,
            path: if is_global_airports {
                GLOBAL_AIRPORTS_ENTRY_NAME.to_string()
            } else {
                path.trim().to_string()
            },
            is_global_airports,
        });
    }

    entries
}

/// Manager for scenery_packs.ini operations
pub struct SceneryPacksManager {
    xplane_path: PathBuf,
    ini_path: PathBuf,
    db: DatabaseConnection,
}

impl SceneryPacksManager {
    /// Create a new manager
    pub fn new(xplane_path: &Path, db: DatabaseConnection) -> Self {
        let ini_path = xplane_path.join("Custom Scenery").join("scenery_packs.ini");
        Self {
            xplane_path: xplane_path.to_path_buf(),
            ini_path,
            db,
        }
    }

    fn default_global_airports_enabled(packages: &[&SceneryPackageInfo]) -> bool {
        packages
            .iter()
            .find(|info| is_global_airports_package(info))
            .map(|info| info.enabled)
            .unwrap_or(true)
    }

    fn default_global_airports_sort_order(packages: &[&SceneryPackageInfo]) -> u32 {
        let mut visible_packages: Vec<_> = packages
            .iter()
            .copied()
            .filter(|info| {
                !is_global_airports_package(info)
                    && !(info.category == SceneryCategory::Unrecognized && !info.enabled)
            })
            .collect();
        visible_packages.sort_by_key(|info| info.sort_order);

        visible_packages
            .iter()
            .position(|info| info.category.priority() >= SceneryCategory::DefaultAirport.priority())
            .unwrap_or(visible_packages.len()) as u32
    }

    fn default_global_airports_category() -> SceneryCategory {
        SceneryCategory::DefaultAirport
    }

    pub async fn get_global_airports_state_for_packages(
        &self,
        packages: &[&SceneryPackageInfo],
    ) -> Result<GlobalAirportsState> {
        let enabled = match SceneryQueries::get_metadata(
            &self.db,
            GLOBAL_AIRPORTS_ENABLED_METADATA_KEY,
        )
        .await
        {
            Ok(Some(value)) => !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no"
            ),
            Ok(None) => Self::default_global_airports_enabled(packages),
            Err(error) => {
                logger::log_info(
                    &format!("Failed to load Global Airports enabled metadata: {}", error),
                    Some("scenery_packs"),
                );
                Self::default_global_airports_enabled(packages)
            }
        };

        let sort_order =
            match SceneryQueries::get_metadata(&self.db, GLOBAL_AIRPORTS_SORT_ORDER_METADATA_KEY)
                .await
            {
                Ok(Some(value)) => value
                    .trim()
                    .parse::<u32>()
                    .unwrap_or_else(|_| Self::default_global_airports_sort_order(packages)),
                Ok(None) => Self::default_global_airports_sort_order(packages),
                Err(error) => {
                    logger::log_info(
                        &format!("Failed to load Global Airports sort metadata: {}", error),
                        Some("scenery_packs"),
                    );
                    Self::default_global_airports_sort_order(packages)
                }
            };

        let category =
            match SceneryQueries::get_metadata(&self.db, GLOBAL_AIRPORTS_CATEGORY_METADATA_KEY)
                .await
            {
                Ok(Some(value)) => match value.trim() {
                    "FixedHighPriority" => SceneryCategory::FixedHighPriority,
                    "Airport" => SceneryCategory::Airport,
                    "DefaultAirport" => SceneryCategory::DefaultAirport,
                    "Library" => SceneryCategory::Library,
                    "Overlay" => SceneryCategory::Overlay,
                    "AirportMesh" => SceneryCategory::AirportMesh,
                    "Mesh" => SceneryCategory::Mesh,
                    "Other" => SceneryCategory::Other,
                    "Unrecognized" => SceneryCategory::Unrecognized,
                    _ => Self::default_global_airports_category(),
                },
                Ok(None) => Self::default_global_airports_category(),
                Err(error) => {
                    logger::log_info(
                        &format!(
                            "Failed to load Global Airports category metadata: {}",
                            error
                        ),
                        Some("scenery_packs"),
                    );
                    Self::default_global_airports_category()
                }
            };

        Ok(GlobalAirportsState {
            enabled,
            sort_order,
            category,
        })
    }

    pub async fn set_global_airports_enabled(&self, enabled: bool) -> Result<()> {
        SceneryQueries::set_metadata(
            &self.db,
            GLOBAL_AIRPORTS_ENABLED_METADATA_KEY,
            if enabled { "true" } else { "false" },
        )
        .await
        .map_err(|e| anyhow!("{}", e))
    }

    pub async fn set_global_airports_sort_order(&self, sort_order: u32) -> Result<()> {
        SceneryQueries::set_metadata(
            &self.db,
            GLOBAL_AIRPORTS_SORT_ORDER_METADATA_KEY,
            &sort_order.to_string(),
        )
        .await
        .map_err(|e| anyhow!("{}", e))
    }

    pub async fn set_global_airports_category(&self, category: &SceneryCategory) -> Result<()> {
        let value = match category {
            SceneryCategory::FixedHighPriority => "FixedHighPriority",
            SceneryCategory::Airport => "Airport",
            SceneryCategory::DefaultAirport => "DefaultAirport",
            SceneryCategory::Library => "Library",
            SceneryCategory::Overlay => "Overlay",
            SceneryCategory::AirportMesh => "AirportMesh",
            SceneryCategory::Mesh => "Mesh",
            SceneryCategory::Other => "Other",
            SceneryCategory::Unrecognized => "Unrecognized",
        };

        SceneryQueries::set_metadata(&self.db, GLOBAL_AIRPORTS_CATEGORY_METADATA_KEY, value)
            .await
            .map_err(|e| anyhow!("{}", e))
    }

    pub async fn reset_global_airports_position_to_default(&self) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());
        let index = index_manager.load_index().await?;
        let packages: Vec<_> = index.packages.values().collect();
        let default_sort_order = Self::default_global_airports_sort_order(&packages);
        self.set_global_airports_sort_order(default_sort_order)
            .await?;
        self.set_global_airports_category(&SceneryCategory::DefaultAirport)
            .await
    }

    fn write_ini_at_path(ini_path: &Path, entries: &[SceneryPackEntry]) -> Result<()> {
        Self::ensure_ini_parent_dir(ini_path)?;

        // Build content in memory first so we can retry with a fallback strategy
        let mut content: Vec<u8> = Vec::new();
        content.extend_from_slice(INI_HEADER.as_bytes());
        for entry in entries {
            let prefix = if entry.enabled {
                "SCENERY_PACK"
            } else {
                "SCENERY_PACK_DISABLED"
            };
            let path = entry_path_for_ini(entry);
            content.extend_from_slice(format!("{} {}\n", prefix, path).as_bytes());
        }

        // Strategy 1: atomic write via temp file + rename (preferred)
        let temp_path = ini_path.with_extension("ini.tmp");
        let atomic_result = (|| -> std::io::Result<()> {
            let mut file = fs::File::create(&temp_path)?;
            file.write_all(&content)?;
            file.flush()?;
            drop(file);
            fs::rename(&temp_path, ini_path)?;
            Ok(())
        })();

        if atomic_result.is_ok() {
            return Ok(());
        }
        // Clean up temp file if rename failed
        let _ = fs::remove_file(&temp_path);

        // Strategy 2: direct overwrite (works when rename is blocked but file itself is writable,
        // e.g. macOS directory permission edge cases or X-Plane holding a rename lock)
        if let Err(direct_err) = fs::write(ini_path, &content) {
            let hint = if cfg!(target_os = "macos") {
                " If X-Plane is running, close it and try again. Otherwise check permissions: right-click Custom Scenery > Get Info > Sharing & Permissions."
            } else {
                " Make sure X-Plane is not running, and that the Custom Scenery folder is writable."
            };
            return Err(anyhow!("{}{}", direct_err, hint));
        }

        Ok(())
    }

    fn ensure_ini_parent_dir(ini_path: &Path) -> Result<()> {
        let parent = ini_path.parent().ok_or_else(|| {
            anyhow!(
                "Invalid ini path: no parent directory ({})",
                ini_path.display()
            )
        })?;

        fs::create_dir_all(parent).map_err(|e| {
            let hint = if e.to_string().contains("failed to create whole tree") {
                " This usually means the configured X-Plane path is invalid (for example drive-relative like 'E:'), or the drive is unavailable."
            } else {
                ""
            };
            anyhow!(
                "Failed to prepare Custom Scenery directory '{}': {}{}",
                parent.display(),
                e,
                hint
            )
        })
    }

    /// Write sorted entries back to scenery_packs.ini
    #[allow(dead_code)]
    pub fn write_ini(&self, entries: &[SceneryPackEntry]) -> Result<()> {
        Self::write_ini_at_path(&self.ini_path, entries)
    }

    fn backup_ini_at_path(ini_path: &Path) -> Result<PathBuf> {
        if !ini_path.exists() {
            return Err(anyhow!("scenery_packs.ini does not exist"));
        }

        let parent_dir = ini_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid ini path: no parent directory"))?;

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("scenery_packs.ini.backup.{}", timestamp);
        let backup_path = parent_dir.join(&backup_name);

        fs::rename(ini_path, &backup_path)?;
        logger::log_info(
            &format!("Created backup: {:?}", backup_path),
            Some("scenery_packs"),
        );

        // Clean up old backups, keeping only the 3 most recent
        Self::cleanup_old_backups(parent_dir, 3);

        Ok(backup_path)
    }

    /// Create a backup of scenery_packs.ini
    #[allow(dead_code)]
    pub fn backup_ini(&self) -> Result<PathBuf> {
        Self::backup_ini_at_path(&self.ini_path)
    }

    /// Remove old backup files, keeping only the specified number of most recent backups
    fn cleanup_old_backups(dir: &std::path::Path, keep_count: usize) {
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
    #[allow(dead_code)]
    pub async fn add_entry(&self, folder_name: &str, category: &SceneryCategory) -> Result<()> {
        self.add_entry_with_locked_entries(folder_name, category, &[])
            .await
    }

    /// Add a new entry to scenery_packs.ini while preserving locked scenery sort positions.
    pub async fn add_entry_with_locked_entries(
        &self,
        folder_name: &str,
        category: &SceneryCategory,
        locked_folder_names: &[String],
    ) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());

        // If index hasn't been created yet, don't add to index or sort
        // User hasn't built the index, so we shouldn't automatically manage scenery order
        if !index_manager.has_index().await? {
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

        let info = index_manager.get_or_classify(&folder_path).await?;
        if &info.category != category {
            index_manager
                .update_entry(folder_name, None, None, Some(category.clone()))
                .await?;
        }

        let _ = index_manager
            .reset_sort_order_with_locked_entries(locked_folder_names.to_vec())
            .await?;
        self.auto_sort_from_index().await
    }

    /// Ensure all installed scenery is in scenery_packs.ini
    /// Only performs incremental indexing if the index has been created
    pub async fn sync_with_folder(&self) -> Result<usize> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        if !custom_scenery_path.exists() {
            return Ok(0);
        }

        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());

        // If index hasn't been created yet, don't perform incremental indexing
        // User hasn't built the index, so we shouldn't automatically update it
        if !index_manager.has_index().await? {
            logger::log_info(
                "Skipping sync_with_folder: index not yet created",
                Some("scenery_packs"),
            );
            return Ok(0);
        }

        let before_index = index_manager.load_index().await?;
        let before_keys: std::collections::HashSet<String> =
            before_index.packages.keys().cloned().collect();

        let updated_index = index_manager.update_index().await?;
        let after_keys: std::collections::HashSet<String> =
            updated_index.packages.keys().cloned().collect();

        let added_count = after_keys.difference(&before_keys).count();

        if before_keys != after_keys {
            self.auto_sort_from_index().await?;
        }

        Ok(added_count)
    }

    /// Sort scenery_packs.ini based entirely on index sort_order
    /// This is used by the scenery manager after manual reordering
    pub async fn auto_sort_from_index(&self) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());
        let index = index_manager.load_index().await?;

        if index.packages.is_empty() {
            logger::log_info(
                "No scenery packages in index, nothing to sort",
                Some("scenery_packs"),
            );
            return Ok(());
        }

        // Create backup if ini exists
        if self.ini_path.exists() {
            let ini_path = self.ini_path.clone();
            if let Err(e) = tokio::task::spawn_blocking(move || Self::backup_ini_at_path(&ini_path))
                .await
                .map_err(|e| anyhow!("Blocking task failed: {}", e))?
            {
                logger::log_info(
                    &format!("Failed to create backup: {}", e),
                    Some("scenery_packs"),
                );
            }
        }

        let mut packages: Vec<_> = index.packages.values().collect();
        packages.sort_by_key(|p| p.sort_order);
        let global_airports = self
            .get_global_airports_state_for_packages(&packages)
            .await?;
        let entries = build_entries_from_sorted_packages(&packages, &global_airports);

        // Write sorted entries
        let ini_path = self.ini_path.clone();
        let entries_len = entries.len();
        tokio::task::spawn_blocking(move || Self::write_ini_at_path(&ini_path, &entries))
            .await
            .map_err(|e| anyhow!("Blocking task failed: {}", e))??;

        logger::log_info(
            &format!("Sorted {} scenery entries from index", entries_len),
            Some("scenery_packs"),
        );

        Ok(())
    }

    /// Apply index state (enabled/sort_order) to scenery_packs.ini
    /// This preserves the order from the index and applies enabled states
    pub async fn apply_from_index(&self) -> Result<()> {
        // This is essentially the same as auto_sort_from_index
        // but we call it explicitly to make the intent clear
        self.auto_sort_from_index().await
    }

    /// Check if ini file is in sync with the index
    /// Returns true if ini order/enabled states match the entries generated from the index.
    pub async fn is_synced_with_index(&self) -> Result<bool> {
        // If ini doesn't exist, it's not synced
        if !self.ini_path.exists() {
            return Ok(false);
        }

        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());
        let index = index_manager.load_index().await?;

        if index.packages.is_empty() {
            return Ok(true);
        }

        // Read current ini file
        let ini_path = self.ini_path.clone();
        let content = tokio::task::spawn_blocking(move || fs::read_to_string(&ini_path))
            .await
            .map_err(|e| anyhow!("Blocking task failed: {}", e))??;

        let mut packages: Vec<_> = index.packages.values().collect();
        packages.sort_by_key(|p| p.sort_order);
        let global_airports = self
            .get_global_airports_state_for_packages(&packages)
            .await?;
        let expected_entries = build_entries_from_sorted_packages(&packages, &global_airports);
        let ini_entries = parse_ini_entries(&content);

        if ini_entries.len() != expected_entries.len() {
            return Ok(false);
        }

        for (ini_entry, expected_entry) in ini_entries.iter().zip(expected_entries.iter()) {
            if ini_entry.enabled != expected_entry.enabled
                || ini_entry.is_global_airports != expected_entry.is_global_airports
                || entry_path_for_ini(ini_entry) != entry_path_for_ini(expected_entry)
            {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn global_airports_state(
        enabled: bool,
        sort_order: u32,
        category: SceneryCategory,
    ) -> GlobalAirportsState {
        GlobalAirportsState {
            enabled,
            sort_order,
            category,
        }
    }

    fn make_package(
        folder_name: &str,
        category: SceneryCategory,
        sort_order: u32,
        enabled: bool,
    ) -> SceneryPackageInfo {
        SceneryPackageInfo {
            folder_name: folder_name.to_string(),
            category,
            sub_priority: 0,
            last_modified: SystemTime::UNIX_EPOCH,
            has_apt_dat: false,
            airport_id: None,
            has_dsf: false,
            has_library_txt: false,
            has_textures: false,
            has_objects: false,
            texture_count: 0,
            earth_nav_tile_count: 0,
            indexed_at: SystemTime::UNIX_EPOCH,
            required_libraries: Vec::new(),
            missing_libraries: Vec::new(),
            exported_library_names: Vec::new(),
            enabled,
            sort_order,
            actual_path: None,
            continent: None,
            original_category: None,
        }
    }

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

    #[test]
    fn writes_global_airports_once_at_default_airport_position() {
        let airport = make_package("KSEA Demo", SceneryCategory::Airport, 0, true);
        let global_airports =
            make_package("Global Airports", SceneryCategory::DefaultAirport, 1, true);
        let library = make_package("MisterX Library", SceneryCategory::Library, 2, true);
        let packages = vec![&airport, &global_airports, &library];

        let entries = build_entries_from_sorted_packages(
            &packages,
            &global_airports_state(true, 1, SceneryCategory::DefaultAirport),
        );

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, "Custom Scenery/KSEA Demo/");
        assert!(entries[1].is_global_airports);
        assert_eq!(entries[1].path, "*GLOBAL_AIRPORTS*");
        assert_eq!(entries[2].path, "Custom Scenery/MisterX Library/");
        assert!(!entries
            .iter()
            .any(|entry| { !entry.is_global_airports && entry.path.contains("Global Airports") }));
    }

    #[test]
    fn ignores_global_airports_package_sort_order_for_special_entry_position() {
        let global_airports =
            make_package("Global Airports", SceneryCategory::DefaultAirport, 0, true);
        let airport = make_package("Airport A", SceneryCategory::Airport, 0, true);
        let library = make_package("Library A", SceneryCategory::Library, 2, true);
        let packages = vec![&global_airports, &airport, &library];

        let entries = build_entries_from_sorted_packages(
            &packages,
            &global_airports_state(true, 1, SceneryCategory::DefaultAirport),
        );

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, "Custom Scenery/Airport A/");
        assert!(entries[1].is_global_airports);
        assert_eq!(entries[2].path, "Custom Scenery/Library A/");
    }

    #[test]
    fn treats_misclassified_global_airports_folder_as_special_marker() {
        let airport = make_package("Airport A", SceneryCategory::Airport, 0, true);
        let global_airports = make_package("Global Airports", SceneryCategory::Airport, 1, false);
        let overlay = make_package("Overlay Pack", SceneryCategory::Overlay, 2, true);
        let packages = vec![&airport, &global_airports, &overlay];

        let entries = build_entries_from_sorted_packages(
            &packages,
            &global_airports_state(false, 1, SceneryCategory::DefaultAirport),
        );

        assert_eq!(entries.len(), 3);
        assert!(entries[1].is_global_airports);
        assert!(!entries[1].enabled);
        assert_eq!(entries[2].path, "Custom Scenery/Overlay Pack/");
    }

    #[test]
    fn parses_global_airports_and_actual_paths_from_ini() {
        let content = concat!(
            "I\n1000 Version\nSCENERY\n\n",
            "SCENERY_PACK C:\\X-Plane\\Custom Scenery\\Airport A\n",
            "SCENERY_PACK *GLOBAL_AIRPORTS*\n",
            "SCENERY_PACK_DISABLED D:/External/Shortcut Scenery/\n"
        );

        let entries = parse_ini_entries(content);

        assert_eq!(entries.len(), 3);
        assert_eq!(
            entry_path_for_ini(&entries[0]),
            "C:/X-Plane/Custom Scenery/Airport A/"
        );
        assert!(entries[1].is_global_airports);
        assert_eq!(entries[1].path, "*GLOBAL_AIRPORTS*");
        assert!(!entries[2].enabled);
        assert_eq!(
            entry_path_for_ini(&entries[2]),
            "D:/External/Shortcut Scenery/"
        );
    }

    #[test]
    fn honors_custom_global_airports_sort_order() {
        let airport = make_package("Airport A", SceneryCategory::Airport, 0, true);
        let library = make_package("Library A", SceneryCategory::Library, 1, true);
        let overlay = make_package("Overlay A", SceneryCategory::Overlay, 2, true);
        let packages = vec![&airport, &library, &overlay];

        let entries = build_entries_from_sorted_packages(
            &packages,
            &global_airports_state(true, 2, SceneryCategory::Overlay),
        );

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].path, "Custom Scenery/Airport A/");
        assert_eq!(entries[1].path, "Custom Scenery/Library A/");
        assert!(entries[2].is_global_airports);
        assert_eq!(entries[3].path, "Custom Scenery/Overlay A/");
    }
}

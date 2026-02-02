//! Database query operations for scenery packages

use crate::error::ApiError;
use crate::logger;
use crate::models::{SceneryCategory, SceneryIndex, SceneryPackageInfo};
use rusqlite::{params, Connection, Transaction};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Convert SystemTime to Unix timestamp (seconds)
fn systemtime_to_unix(time: &SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64
}

/// Convert Unix timestamp to SystemTime
/// Handles negative timestamps by falling back to UNIX_EPOCH
fn unix_to_systemtime(timestamp: i64) -> SystemTime {
    if timestamp >= 0 {
        UNIX_EPOCH + Duration::from_secs(timestamp as u64)
    } else {
        // Negative timestamps are invalid - fall back to UNIX_EPOCH
        UNIX_EPOCH
    }
}

/// Convert SceneryCategory to database string
fn category_to_string(category: &SceneryCategory) -> &'static str {
    match category {
        SceneryCategory::FixedHighPriority => "FixedHighPriority",
        SceneryCategory::Airport => "Airport",
        SceneryCategory::DefaultAirport => "DefaultAirport",
        SceneryCategory::Library => "Library",
        SceneryCategory::Overlay => "Overlay",
        SceneryCategory::AirportMesh => "AirportMesh",
        SceneryCategory::Mesh => "Mesh",
        SceneryCategory::Other => "Other",
        SceneryCategory::Unrecognized => "Unrecognized",
    }
}

/// Convert database string to SceneryCategory
fn string_to_category(s: &str) -> SceneryCategory {
    match s {
        "FixedHighPriority" => SceneryCategory::FixedHighPriority,
        "Airport" => SceneryCategory::Airport,
        "DefaultAirport" => SceneryCategory::DefaultAirport,
        "Library" => SceneryCategory::Library,
        "Overlay" => SceneryCategory::Overlay,
        "AirportMesh" => SceneryCategory::AirportMesh,
        "Mesh" => SceneryCategory::Mesh,
        "Unrecognized" => SceneryCategory::Unrecognized,
        _ => SceneryCategory::Other,
    }
}

/// Scenery database query operations
pub struct SceneryQueries;

impl SceneryQueries {
    /// Load all scenery packages from the database into a SceneryIndex
    pub fn load_all(conn: &Connection) -> Result<SceneryIndex, ApiError> {
        let mut packages: HashMap<String, SceneryPackageInfo> = HashMap::new();

        // Query all packages
        let mut stmt = conn
            .prepare(
                "SELECT id, folder_name, category, sub_priority, last_modified, indexed_at,
                        has_apt_dat, has_dsf, has_library_txt, has_textures, has_objects,
                        texture_count, earth_nav_tile_count, enabled, sort_order, actual_path,
                        continent
                 FROM scenery_packages",
            )
            .map_err(|e| ApiError::database(format!("Failed to prepare query: {}", e)))?;

        let package_rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,  // id
                    row.get::<_, String>(1)?,  // folder_name
                    row.get::<_, String>(2)?,  // category
                    row.get::<_, u8>(3)?,      // sub_priority
                    row.get::<_, i64>(4)?,     // last_modified
                    row.get::<_, i64>(5)?,     // indexed_at
                    row.get::<_, bool>(6)?,    // has_apt_dat
                    row.get::<_, bool>(7)?,    // has_dsf
                    row.get::<_, bool>(8)?,    // has_library_txt
                    row.get::<_, bool>(9)?,    // has_textures
                    row.get::<_, bool>(10)?,   // has_objects
                    row.get::<_, usize>(11)?,  // texture_count
                    row.get::<_, u32>(12)?,    // earth_nav_tile_count
                    row.get::<_, bool>(13)?,   // enabled
                    row.get::<_, u32>(14)?,    // sort_order
                    row.get::<_, Option<String>>(15)?, // actual_path
                    row.get::<_, Option<String>>(16)?, // continent
                ))
            })
            .map_err(|e| ApiError::database(format!("Failed to query packages: {}", e)))?;

        // Collect package data with their IDs for library queries
        let mut package_data: Vec<(i64, SceneryPackageInfo)> = Vec::new();

        for row_result in package_rows {
            let row = row_result
                .map_err(|e| ApiError::database(format!("Failed to read package row: {}", e)))?;

            let (
                id,
                folder_name,
                category_str,
                sub_priority,
                last_modified,
                indexed_at,
                has_apt_dat,
                has_dsf,
                has_library_txt,
                has_textures,
                has_objects,
                texture_count,
                earth_nav_tile_count,
                enabled,
                sort_order,
                actual_path,
                continent,
            ) = row;

            let info = SceneryPackageInfo {
                folder_name: folder_name.clone(),
                category: string_to_category(&category_str),
                sub_priority,
                last_modified: unix_to_systemtime(last_modified),
                indexed_at: unix_to_systemtime(indexed_at),
                has_apt_dat,
                has_dsf,
                has_library_txt,
                has_textures,
                has_objects,
                texture_count,
                earth_nav_tile_count,
                enabled,
                sort_order,
                required_libraries: Vec::new(),
                missing_libraries: Vec::new(),
                exported_library_names: Vec::new(),
                actual_path,
                continent,
            };

            package_data.push((id, info));
        }

        // Load libraries for all packages in batch
        let required_libs = Self::load_all_libraries(conn, "required_libraries")?;
        let missing_libs = Self::load_all_libraries(conn, "missing_libraries")?;
        let exported_libs = Self::load_all_libraries(conn, "exported_libraries")?;

        // Associate libraries with packages
        for (id, mut info) in package_data {
            if let Some(libs) = required_libs.get(&id) {
                info.required_libraries = libs.clone();
            }
            if let Some(libs) = missing_libs.get(&id) {
                info.missing_libraries = libs.clone();
            }
            if let Some(libs) = exported_libs.get(&id) {
                info.exported_library_names = libs.clone();
            }
            packages.insert(info.folder_name.clone(), info);
        }

        // Load last_updated from metadata
        let last_updated = Self::get_metadata(conn, "last_updated")?
            .and_then(|s| s.parse::<i64>().ok())
            .map(unix_to_systemtime)
            .unwrap_or_else(SystemTime::now);

        // Load version from metadata
        let version = Self::get_metadata(conn, "version")?
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);

        Ok(SceneryIndex {
            version,
            packages,
            last_updated,
        })
    }

    /// Load all libraries from a library table
    fn load_all_libraries(
        conn: &Connection,
        table_name: &str,
    ) -> Result<HashMap<i64, Vec<String>>, ApiError> {
        let mut result: HashMap<i64, Vec<String>> = HashMap::new();

        let query = format!(
            "SELECT package_id, library_name FROM {} ORDER BY package_id, id",
            table_name
        );
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| ApiError::database(format!("Failed to prepare library query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| ApiError::database(format!("Failed to query libraries: {}", e)))?;

        for row_result in rows {
            let (package_id, library_name) = row_result
                .map_err(|e| ApiError::database(format!("Failed to read library row: {}", e)))?;
            result.entry(package_id).or_default().push(library_name);
        }

        Ok(result)
    }

    /// Get metadata value by key
    fn get_metadata(conn: &Connection, key: &str) -> Result<Option<String>, ApiError> {
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM index_metadata WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok();
        Ok(result)
    }

    /// Set metadata value
    fn set_metadata(conn: &Connection, key: &str, value: &str) -> Result<(), ApiError> {
        conn.execute(
            "INSERT OR REPLACE INTO index_metadata (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| ApiError::database(format!("Failed to set metadata: {}", e)))?;
        Ok(())
    }

    /// Save a complete SceneryIndex to the database (replaces all data)
    /// Uses prepared statements and batch operations for optimal performance
    pub fn save_all(conn: &mut Connection, index: &SceneryIndex) -> Result<(), ApiError> {
        let tx = conn
            .transaction()
            .map_err(|e| ApiError::database(format!("Failed to start transaction: {}", e)))?;

        // Clear existing data
        tx.execute_batch(
            "DELETE FROM required_libraries;
             DELETE FROM missing_libraries;
             DELETE FROM exported_libraries;
             DELETE FROM scenery_packages;",
        )
        .map_err(|e| ApiError::database(format!("Failed to clear existing data: {}", e)))?;

        // Prepare statements once for batch insert (performance optimization)
        let mut pkg_stmt = tx.prepare_cached(
            "INSERT INTO scenery_packages (
                folder_name, category, sub_priority, last_modified, indexed_at,
                has_apt_dat, has_dsf, has_library_txt, has_textures, has_objects,
                texture_count, earth_nav_tile_count, enabled, sort_order, actual_path,
                continent
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
        ).map_err(|e| ApiError::database(format!("Failed to prepare package statement: {}", e)))?;

        let mut req_lib_stmt = tx.prepare_cached(
            "INSERT INTO required_libraries (package_id, library_name) VALUES (?1, ?2)"
        ).map_err(|e| ApiError::database(format!("Failed to prepare required_libraries statement: {}", e)))?;

        let mut miss_lib_stmt = tx.prepare_cached(
            "INSERT INTO missing_libraries (package_id, library_name) VALUES (?1, ?2)"
        ).map_err(|e| ApiError::database(format!("Failed to prepare missing_libraries statement: {}", e)))?;

        let mut exp_lib_stmt = tx.prepare_cached(
            "INSERT INTO exported_libraries (package_id, library_name) VALUES (?1, ?2)"
        ).map_err(|e| ApiError::database(format!("Failed to prepare exported_libraries statement: {}", e)))?;

        // Insert all packages using prepared statements
        for info in index.packages.values() {
            pkg_stmt.execute(params![
                info.folder_name,
                category_to_string(&info.category),
                info.sub_priority,
                systemtime_to_unix(&info.last_modified),
                systemtime_to_unix(&info.indexed_at),
                info.has_apt_dat,
                info.has_dsf,
                info.has_library_txt,
                info.has_textures,
                info.has_objects,
                info.texture_count,
                info.earth_nav_tile_count,
                info.enabled,
                info.sort_order,
                &info.actual_path,
                &info.continent,
            ]).map_err(|e| ApiError::database(format!("Failed to insert package: {}", e)))?;

            let package_id = tx.last_insert_rowid();

            // Insert libraries using prepared statements
            for lib_name in &info.required_libraries {
                req_lib_stmt.execute(params![package_id, lib_name])
                    .map_err(|e| ApiError::database(format!("Failed to insert required library: {}", e)))?;
            }

            for lib_name in &info.missing_libraries {
                miss_lib_stmt.execute(params![package_id, lib_name])
                    .map_err(|e| ApiError::database(format!("Failed to insert missing library: {}", e)))?;
            }

            for lib_name in &info.exported_library_names {
                exp_lib_stmt.execute(params![package_id, lib_name])
                    .map_err(|e| ApiError::database(format!("Failed to insert exported library: {}", e)))?;
            }
        }

        // Drop prepared statements before committing
        drop(pkg_stmt);
        drop(req_lib_stmt);
        drop(miss_lib_stmt);
        drop(exp_lib_stmt);

        // Update metadata
        Self::set_metadata(&tx, "version", &index.version.to_string())?;
        Self::set_metadata(
            &tx,
            "last_updated",
            &systemtime_to_unix(&index.last_updated).to_string(),
        )?;

        tx.commit()
            .map_err(|e| ApiError::database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Insert a single package into the database
    fn insert_package(conn: &Connection, info: &SceneryPackageInfo) -> Result<i64, ApiError> {
        conn.execute(
            "INSERT INTO scenery_packages (
                folder_name, category, sub_priority, last_modified, indexed_at,
                has_apt_dat, has_dsf, has_library_txt, has_textures, has_objects,
                texture_count, earth_nav_tile_count, enabled, sort_order, actual_path,
                continent
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                info.folder_name,
                category_to_string(&info.category),
                info.sub_priority,
                systemtime_to_unix(&info.last_modified),
                systemtime_to_unix(&info.indexed_at),
                info.has_apt_dat,
                info.has_dsf,
                info.has_library_txt,
                info.has_textures,
                info.has_objects,
                info.texture_count,
                info.earth_nav_tile_count,
                info.enabled,
                info.sort_order,
                &info.actual_path,
                &info.continent,
            ],
        )
        .map_err(|e| ApiError::database(format!("Failed to insert package: {}", e)))?;

        let package_id = conn.last_insert_rowid();

        // Insert libraries
        Self::insert_libraries(conn, package_id, &info.required_libraries, "required_libraries")?;
        Self::insert_libraries(conn, package_id, &info.missing_libraries, "missing_libraries")?;
        Self::insert_libraries(
            conn,
            package_id,
            &info.exported_library_names,
            "exported_libraries",
        )?;

        Ok(package_id)
    }

    /// Insert libraries for a package
    fn insert_libraries(
        conn: &Connection,
        package_id: i64,
        libraries: &[String],
        table_name: &str,
    ) -> Result<(), ApiError> {
        let query = format!(
            "INSERT INTO {} (package_id, library_name) VALUES (?1, ?2)",
            table_name
        );

        for lib_name in libraries {
            conn.execute(&query, params![package_id, lib_name])
                .map_err(|e| ApiError::database(format!("Failed to insert library: {}", e)))?;
        }

        Ok(())
    }

    /// Update a single package in the database
    pub fn update_package(conn: &mut Connection, info: &SceneryPackageInfo) -> Result<(), ApiError> {
        let tx = conn
            .transaction()
            .map_err(|e| ApiError::database(format!("Failed to start transaction: {}", e)))?;

        // Get existing package ID or insert new
        let package_id: Option<i64> = tx
            .query_row(
                "SELECT id FROM scenery_packages WHERE folder_name = ?1",
                params![&info.folder_name],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = package_id {
            // Update existing package
            tx.execute(
                "UPDATE scenery_packages SET
                    category = ?2, sub_priority = ?3, last_modified = ?4, indexed_at = ?5,
                    has_apt_dat = ?6, has_dsf = ?7, has_library_txt = ?8, has_textures = ?9,
                    has_objects = ?10, texture_count = ?11, earth_nav_tile_count = ?12,
                    enabled = ?13, sort_order = ?14, actual_path = ?15, continent = ?16
                 WHERE id = ?1",
                params![
                    id,
                    category_to_string(&info.category),
                    info.sub_priority,
                    systemtime_to_unix(&info.last_modified),
                    systemtime_to_unix(&info.indexed_at),
                    info.has_apt_dat,
                    info.has_dsf,
                    info.has_library_txt,
                    info.has_textures,
                    info.has_objects,
                    info.texture_count,
                    info.earth_nav_tile_count,
                    info.enabled,
                    info.sort_order,
                    &info.actual_path,
                    &info.continent,
                ],
            )
            .map_err(|e| ApiError::database(format!("Failed to update package: {}", e)))?;

            // Update libraries
            Self::update_package_libraries(&tx, id, info)?;
        } else {
            // Insert new package
            Self::insert_package(&tx, info)?;
        }

        // Update last_updated metadata
        Self::set_metadata(
            &tx,
            "last_updated",
            &systemtime_to_unix(&SystemTime::now()).to_string(),
        )?;

        tx.commit()
            .map_err(|e| ApiError::database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Update libraries for an existing package
    fn update_package_libraries(
        conn: &Transaction,
        package_id: i64,
        info: &SceneryPackageInfo,
    ) -> Result<(), ApiError> {
        // Delete existing libraries
        conn.execute(
            "DELETE FROM required_libraries WHERE package_id = ?1",
            params![package_id],
        )
        .map_err(|e| ApiError::database(format!("Failed to delete libraries: {}", e)))?;
        conn.execute(
            "DELETE FROM missing_libraries WHERE package_id = ?1",
            params![package_id],
        )
        .map_err(|e| ApiError::database(format!("Failed to delete libraries: {}", e)))?;
        conn.execute(
            "DELETE FROM exported_libraries WHERE package_id = ?1",
            params![package_id],
        )
        .map_err(|e| ApiError::database(format!("Failed to delete libraries: {}", e)))?;

        // Insert new libraries
        Self::insert_libraries(conn, package_id, &info.required_libraries, "required_libraries")?;
        Self::insert_libraries(conn, package_id, &info.missing_libraries, "missing_libraries")?;
        Self::insert_libraries(
            conn,
            package_id,
            &info.exported_library_names,
            "exported_libraries",
        )?;

        Ok(())
    }

    /// Delete a package from the database
    pub fn delete_package(conn: &Connection, folder_name: &str) -> Result<bool, ApiError> {
        let rows_affected = conn
            .execute(
                "DELETE FROM scenery_packages WHERE folder_name = ?1",
                params![folder_name],
            )
            .map_err(|e| ApiError::database(format!("Failed to delete package: {}", e)))?;

        Ok(rows_affected > 0)
    }

    /// Get a single package by folder name (test only)
    #[cfg(test)]
    pub fn get_package(
        conn: &Connection,
        folder_name: &str,
    ) -> Result<Option<SceneryPackageInfo>, ApiError> {
        let row: Option<(i64, String, String, u8, i64, i64, bool, bool, bool, bool, bool, usize, u32, bool, u32, Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT id, folder_name, category, sub_priority, last_modified, indexed_at,
                        has_apt_dat, has_dsf, has_library_txt, has_textures, has_objects,
                        texture_count, earth_nav_tile_count, enabled, sort_order, actual_path,
                        continent
                 FROM scenery_packages WHERE folder_name = ?1",
                params![folder_name],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                        row.get(12)?,
                        row.get(13)?,
                        row.get(14)?,
                        row.get(15)?,
                        row.get(16)?,
                    ))
                },
            )
            .ok();

        match row {
            Some((
                id,
                folder_name,
                category_str,
                sub_priority,
                last_modified,
                indexed_at,
                has_apt_dat,
                has_dsf,
                has_library_txt,
                has_textures,
                has_objects,
                texture_count,
                earth_nav_tile_count,
                enabled,
                sort_order,
                actual_path,
                continent,
            )) => {
                let mut info = SceneryPackageInfo {
                    folder_name,
                    category: string_to_category(&category_str),
                    sub_priority,
                    last_modified: unix_to_systemtime(last_modified),
                    indexed_at: unix_to_systemtime(indexed_at),
                    has_apt_dat,
                    has_dsf,
                    has_library_txt,
                    has_textures,
                    has_objects,
                    texture_count,
                    earth_nav_tile_count,
                    enabled,
                    sort_order,
                    required_libraries: Vec::new(),
                    missing_libraries: Vec::new(),
                    exported_library_names: Vec::new(),
                    actual_path,
                    continent,
                };

                // Load libraries
                info.required_libraries = Self::load_package_libraries(conn, id, "required_libraries")?;
                info.missing_libraries = Self::load_package_libraries(conn, id, "missing_libraries")?;
                info.exported_library_names = Self::load_package_libraries(conn, id, "exported_libraries")?;

                Ok(Some(info))
            }
            None => Ok(None),
        }
    }

    /// Load libraries for a specific package (test only)
    #[cfg(test)]
    fn load_package_libraries(
        conn: &Connection,
        package_id: i64,
        table_name: &str,
    ) -> Result<Vec<String>, ApiError> {
        let query = format!(
            "SELECT library_name FROM {} WHERE package_id = ?1 ORDER BY id",
            table_name
        );
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| ApiError::database(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(params![package_id], |row| row.get(0))
            .map_err(|e| ApiError::database(format!("Failed to query libraries: {}", e)))?;

        let mut libraries = Vec::new();
        for row_result in rows {
            libraries.push(row_result
                .map_err(|e| ApiError::database(format!("Failed to read library: {}", e)))?);
        }

        Ok(libraries)
    }

    /// Update enabled and sort_order for a package
    pub fn update_entry(
        conn: &Connection,
        folder_name: &str,
        enabled: Option<bool>,
        sort_order: Option<u32>,
        category: Option<&SceneryCategory>,
    ) -> Result<bool, ApiError> {
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(e) = enabled {
            updates.push("enabled = ?");
            params_vec.push(Box::new(e));
        }
        if let Some(s) = sort_order {
            updates.push("sort_order = ?");
            params_vec.push(Box::new(s));
        }
        if let Some(c) = category {
            updates.push("category = ?");
            params_vec.push(Box::new(category_to_string(c).to_string()));
        }

        if updates.is_empty() {
            return Ok(false);
        }

        params_vec.push(Box::new(folder_name.to_string()));

        let query = format!(
            "UPDATE scenery_packages SET {} WHERE folder_name = ?",
            updates.join(", ")
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows_affected = conn
            .execute(&query, params_refs.as_slice())
            .map_err(|e| ApiError::database(format!("Failed to update entry: {}", e)))?;

        Ok(rows_affected > 0)
    }

    /// Batch update entries (enabled and sort_order only)
    /// Uses prepared statements for optimal performance with large batches
    /// Logs warnings for entries that don't exist in the database
    pub fn batch_update_entries(
        conn: &mut Connection,
        entries: &[crate::models::SceneryEntryUpdate],
    ) -> Result<(), ApiError> {
        let tx = conn
            .transaction()
            .map_err(|e| ApiError::database(format!("Failed to start transaction: {}", e)))?;

        // Track entries that were not found for logging
        let mut not_found: Vec<String> = Vec::new();

        // Prepare statement once for all updates (performance optimization)
        {
            let mut stmt = tx.prepare_cached(
                "UPDATE scenery_packages SET enabled = ?1, sort_order = ?2 WHERE folder_name = ?3"
            ).map_err(|e| ApiError::database(format!("Failed to prepare update statement: {}", e)))?;

            for entry in entries {
                let rows_affected = stmt.execute(params![entry.enabled, entry.sort_order, &entry.folder_name])
                    .map_err(|e| ApiError::database(format!("Failed to update entry: {}", e)))?;

                // Check if the entry existed
                if rows_affected == 0 {
                    not_found.push(entry.folder_name.clone());
                }
            }
        }

        // Log warnings for entries that were not found
        if !not_found.is_empty() {
            logger::log_info(
                &format!(
                    "batch_update_entries: {} entries not found in database: {:?}",
                    not_found.len(),
                    not_found.iter().take(5).collect::<Vec<_>>()
                ),
                Some("database"),
            );
        }

        // Update last_updated metadata
        Self::set_metadata(
            &tx,
            "last_updated",
            &systemtime_to_unix(&SystemTime::now()).to_string(),
        )?;

        tx.commit()
            .map_err(|e| ApiError::database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Get package count
    pub fn get_package_count(conn: &Connection) -> Result<usize, ApiError> {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM scenery_packages", [], |row| row.get(0))
            .map_err(|e| ApiError::database(format!("Failed to count packages: {}", e)))?;
        Ok(count as usize)
    }

    /// Check if database has any packages (uses EXISTS for optimal performance)
    pub fn has_packages(conn: &Connection) -> Result<bool, ApiError> {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM scenery_packages LIMIT 1)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| ApiError::database(format!("Failed to check packages: {}", e)))?;
        Ok(exists)
    }

    /// Clear all scenery data from the database
    /// Used before rebuilding index to ensure a completely fresh start
    pub fn clear_all(conn: &Connection) -> Result<(), ApiError> {
        conn.execute_batch(
            "DELETE FROM required_libraries;
             DELETE FROM missing_libraries;
             DELETE FROM exported_libraries;
             DELETE FROM scenery_packages;
             DELETE FROM index_metadata;",
        )
        .map_err(|e| ApiError::database(format!("Failed to clear all data: {}", e)))?;

        logger::log_info("Cleared all scenery index data", Some("database"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::open_memory_connection;
    use crate::database::migrations::apply_migrations;

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        apply_migrations(&conn).unwrap();
        conn.into_inner()
    }

    #[test]
    fn test_insert_and_load_package() {
        let mut conn = setup_test_db();

        let info = SceneryPackageInfo {
            folder_name: "TestAirport".to_string(),
            category: SceneryCategory::Airport,
            sub_priority: 0,
            last_modified: SystemTime::now(),
            indexed_at: SystemTime::now(),
            has_apt_dat: true,
            has_dsf: false,
            has_library_txt: false,
            has_textures: true,
            has_objects: false,
            texture_count: 10,
            earth_nav_tile_count: 1,
            enabled: true,
            sort_order: 5,
            required_libraries: vec!["opensceneryx".to_string()],
            missing_libraries: vec![],
            exported_library_names: vec![],
            actual_path: None,
            continent: Some("Asia".to_string()),
        };

        SceneryQueries::update_package(&mut conn, &info).unwrap();

        let loaded = SceneryQueries::get_package(&conn, "TestAirport")
            .unwrap()
            .expect("Package not found");

        assert_eq!(loaded.folder_name, "TestAirport");
        assert_eq!(loaded.category, SceneryCategory::Airport);
        assert!(loaded.has_apt_dat);
        assert_eq!(loaded.required_libraries, vec!["opensceneryx"]);
    }

    #[test]
    fn test_delete_package() {
        let mut conn = setup_test_db();

        let info = SceneryPackageInfo {
            folder_name: "ToDelete".to_string(),
            category: SceneryCategory::Library,
            sub_priority: 0,
            last_modified: SystemTime::now(),
            indexed_at: SystemTime::now(),
            has_apt_dat: false,
            has_dsf: false,
            has_library_txt: true,
            has_textures: false,
            has_objects: false,
            texture_count: 0,
            earth_nav_tile_count: 0,
            enabled: true,
            sort_order: 0,
            required_libraries: vec![],
            missing_libraries: vec![],
            exported_library_names: vec!["mylib".to_string()],
            actual_path: None,
            continent: None,
        };

        SceneryQueries::update_package(&mut conn, &info).unwrap();
        assert!(SceneryQueries::get_package(&conn, "ToDelete").unwrap().is_some());

        let deleted = SceneryQueries::delete_package(&conn, "ToDelete").unwrap();
        assert!(deleted);

        assert!(SceneryQueries::get_package(&conn, "ToDelete").unwrap().is_none());
    }

    #[test]
    fn test_load_all() {
        let mut conn = setup_test_db();

        // Insert multiple packages
        for i in 0..3 {
            let info = SceneryPackageInfo {
                folder_name: format!("Package{}", i),
                category: SceneryCategory::Other,
                sub_priority: 0,
                last_modified: SystemTime::now(),
                indexed_at: SystemTime::now(),
                has_apt_dat: false,
                has_dsf: false,
                has_library_txt: false,
                has_textures: false,
                has_objects: false,
                texture_count: 0,
                earth_nav_tile_count: 0,
                enabled: true,
                sort_order: i as u32,
                required_libraries: vec![],
                missing_libraries: vec![],
                exported_library_names: vec![],
                actual_path: None,
                continent: None,
            };
            SceneryQueries::update_package(&mut conn, &info).unwrap();
        }

        let index = SceneryQueries::load_all(&conn).unwrap();
        assert_eq!(index.packages.len(), 3);
    }
}

//! Database query operations for scenery packages

use crate::error::ApiError;
use crate::logger;
use crate::models::{SceneryCategory, SceneryIndex, SceneryPackageInfo};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::entities::{
    exported_libraries, index_metadata, missing_libraries, required_libraries, scenery_packages,
};

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
    pub async fn load_all(conn: &DatabaseConnection) -> Result<SceneryIndex, ApiError> {
        let packages = scenery_packages::Entity::find()
            .all(conn)
            .await
            .map_err(ApiError::from)?;

        let required_libs = Self::load_all_required_libraries(conn).await?;
        let missing_libs = Self::load_all_missing_libraries(conn).await?;
        let exported_libs = Self::load_all_exported_libraries(conn).await?;

        let mut package_map: HashMap<String, SceneryPackageInfo> = HashMap::new();
        for pkg in packages {
            let mut info = SceneryPackageInfo {
                folder_name: pkg.folder_name.clone(),
                category: string_to_category(&pkg.category),
                sub_priority: pkg.sub_priority as u8,
                last_modified: unix_to_systemtime(pkg.last_modified),
                indexed_at: unix_to_systemtime(pkg.indexed_at),
                has_apt_dat: pkg.has_apt_dat,
                airport_id: pkg.airport_id.clone(),
                has_dsf: pkg.has_dsf,
                has_library_txt: pkg.has_library_txt,
                has_textures: pkg.has_textures,
                has_objects: pkg.has_objects,
                texture_count: pkg.texture_count as usize,
                earth_nav_tile_count: pkg.earth_nav_tile_count as u32,
                enabled: pkg.enabled,
                sort_order: pkg.sort_order as u32,
                required_libraries: Vec::new(),
                missing_libraries: Vec::new(),
                exported_library_names: Vec::new(),
                actual_path: pkg.actual_path.clone(),
                continent: pkg.continent.clone(),
                original_category: pkg
                    .original_category
                    .as_ref()
                    .map(|s| string_to_category(s)),
            };

            if let Some(libs) = required_libs.get(&pkg.id) {
                info.required_libraries = libs.clone();
            }
            if let Some(libs) = missing_libs.get(&pkg.id) {
                info.missing_libraries = libs.clone();
            }
            if let Some(libs) = exported_libs.get(&pkg.id) {
                info.exported_library_names = libs.clone();
            }

            package_map.insert(info.folder_name.clone(), info);
        }

        let last_updated = Self::get_metadata_async(conn, "last_updated")
            .await?
            .and_then(|s| s.parse::<i64>().ok())
            .map(unix_to_systemtime)
            .unwrap_or_else(SystemTime::now);

        let version = Self::get_metadata_async(conn, "version")
            .await?
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);

        Ok(SceneryIndex {
            version,
            packages: package_map,
            last_updated,
        })
    }

    async fn load_all_required_libraries(
        conn: &DatabaseConnection,
    ) -> Result<HashMap<i64, Vec<String>>, ApiError> {
        let rows = required_libraries::Entity::find()
            .order_by_asc(required_libraries::Column::PackageId)
            .order_by_asc(required_libraries::Column::Id)
            .all(conn)
            .await
            .map_err(ApiError::from)?;

        let mut map = HashMap::new();
        for row in rows {
            map.entry(row.package_id)
                .or_insert_with(Vec::new)
                .push(row.library_name);
        }
        Ok(map)
    }

    async fn load_all_missing_libraries(
        conn: &DatabaseConnection,
    ) -> Result<HashMap<i64, Vec<String>>, ApiError> {
        let rows = missing_libraries::Entity::find()
            .order_by_asc(missing_libraries::Column::PackageId)
            .order_by_asc(missing_libraries::Column::Id)
            .all(conn)
            .await
            .map_err(ApiError::from)?;

        let mut map = HashMap::new();
        for row in rows {
            map.entry(row.package_id)
                .or_insert_with(Vec::new)
                .push(row.library_name);
        }
        Ok(map)
    }

    async fn load_all_exported_libraries(
        conn: &DatabaseConnection,
    ) -> Result<HashMap<i64, Vec<String>>, ApiError> {
        let rows = exported_libraries::Entity::find()
            .order_by_asc(exported_libraries::Column::PackageId)
            .order_by_asc(exported_libraries::Column::Id)
            .all(conn)
            .await
            .map_err(ApiError::from)?;

        let mut map = HashMap::new();
        for row in rows {
            map.entry(row.package_id)
                .or_insert_with(Vec::new)
                .push(row.library_name);
        }
        Ok(map)
    }

    async fn get_metadata_async(
        conn: &DatabaseConnection,
        key: &str,
    ) -> Result<Option<String>, ApiError> {
        let row = index_metadata::Entity::find_by_id(key.to_string())
            .one(conn)
            .await
            .map_err(ApiError::from)?;
        Ok(row.map(|m| m.value))
    }

    async fn set_metadata_async<C>(conn: &C, key: &str, value: &str) -> Result<(), ApiError>
    where
        C: ConnectionTrait,
    {
        let active = index_metadata::ActiveModel {
            key: Set(key.to_string()),
            value: Set(value.to_string()),
        };

        index_metadata::Entity::insert(active)
            .on_conflict(
                OnConflict::column(index_metadata::Column::Key)
                    .update_column(index_metadata::Column::Value)
                    .to_owned(),
            )
            .exec(conn)
            .await
            .map_err(ApiError::from)?;
        Ok(())
    }

    /// Save a complete SceneryIndex to the database (replaces all data)
    pub async fn save_all(conn: &DatabaseConnection, index: &SceneryIndex) -> Result<(), ApiError> {
        let txn = conn.begin().await.map_err(ApiError::from)?;

        required_libraries::Entity::delete_many()
            .exec(&txn)
            .await
            .map_err(ApiError::from)?;
        missing_libraries::Entity::delete_many()
            .exec(&txn)
            .await
            .map_err(ApiError::from)?;
        exported_libraries::Entity::delete_many()
            .exec(&txn)
            .await
            .map_err(ApiError::from)?;
        scenery_packages::Entity::delete_many()
            .exec(&txn)
            .await
            .map_err(ApiError::from)?;

        for info in index.packages.values() {
            let package_id = Self::insert_package_async(&txn, info).await?;
            Self::insert_libraries_async(
                &txn,
                package_id,
                &info.required_libraries,
                LibraryKind::Required,
            )
            .await?;
            Self::insert_libraries_async(
                &txn,
                package_id,
                &info.missing_libraries,
                LibraryKind::Missing,
            )
            .await?;
            Self::insert_libraries_async(
                &txn,
                package_id,
                &info.exported_library_names,
                LibraryKind::Exported,
            )
            .await?;
        }

        Self::set_metadata_async(&txn, "version", &index.version.to_string()).await?;
        Self::set_metadata_async(
            &txn,
            "last_updated",
            &systemtime_to_unix(&index.last_updated).to_string(),
        )
        .await?;

        txn.commit().await.map_err(ApiError::from)?;
        Ok(())
    }

    async fn insert_package_async<C>(conn: &C, info: &SceneryPackageInfo) -> Result<i64, ApiError>
    where
        C: ConnectionTrait,
    {
        let active = scenery_packages::ActiveModel {
            id: ActiveValue::NotSet,
            folder_name: Set(info.folder_name.clone()),
            category: Set(category_to_string(&info.category).to_string()),
            sub_priority: Set(info.sub_priority as i32),
            last_modified: Set(systemtime_to_unix(&info.last_modified)),
            indexed_at: Set(systemtime_to_unix(&info.indexed_at)),
            has_apt_dat: Set(info.has_apt_dat),
            airport_id: Set(info.airport_id.clone()),
            has_dsf: Set(info.has_dsf),
            has_library_txt: Set(info.has_library_txt),
            has_textures: Set(info.has_textures),
            has_objects: Set(info.has_objects),
            texture_count: Set(info.texture_count as i32),
            earth_nav_tile_count: Set(info.earth_nav_tile_count as i32),
            enabled: Set(info.enabled),
            sort_order: Set(info.sort_order as i32),
            actual_path: Set(info.actual_path.clone()),
            continent: Set(info.continent.clone()),
            original_category: Set(info
                .original_category
                .as_ref()
                .map(category_to_string)
                .map(|s| s.to_string())),
        };

        let result = scenery_packages::Entity::insert(active)
            .exec(conn)
            .await
            .map_err(ApiError::from)?;

        Ok(result.last_insert_id)
    }

    async fn insert_libraries_async<C>(
        conn: &C,
        package_id: i64,
        libraries: &[String],
        kind: LibraryKind,
    ) -> Result<(), ApiError>
    where
        C: ConnectionTrait,
    {
        for lib_name in libraries {
            match kind {
                LibraryKind::Required => {
                    let active = required_libraries::ActiveModel {
                        id: ActiveValue::NotSet,
                        package_id: Set(package_id),
                        library_name: Set(lib_name.clone()),
                    };
                    required_libraries::Entity::insert(active)
                        .exec(conn)
                        .await
                        .map_err(ApiError::from)?;
                }
                LibraryKind::Missing => {
                    let active = missing_libraries::ActiveModel {
                        id: ActiveValue::NotSet,
                        package_id: Set(package_id),
                        library_name: Set(lib_name.clone()),
                    };
                    missing_libraries::Entity::insert(active)
                        .exec(conn)
                        .await
                        .map_err(ApiError::from)?;
                }
                LibraryKind::Exported => {
                    let active = exported_libraries::ActiveModel {
                        id: ActiveValue::NotSet,
                        package_id: Set(package_id),
                        library_name: Set(lib_name.clone()),
                    };
                    exported_libraries::Entity::insert(active)
                        .exec(conn)
                        .await
                        .map_err(ApiError::from)?;
                }
            }
        }
        Ok(())
    }

    /// Update a single package in the database
    pub async fn update_package(
        conn: &DatabaseConnection,
        info: &SceneryPackageInfo,
    ) -> Result<(), ApiError> {
        let txn = conn.begin().await.map_err(ApiError::from)?;

        let existing = scenery_packages::Entity::find()
            .filter(scenery_packages::Column::FolderName.eq(&info.folder_name))
            .one(&txn)
            .await
            .map_err(ApiError::from)?;

        if let Some(model) = existing {
            let id = model.id;
            let mut active: scenery_packages::ActiveModel = model.into();
            active.category = Set(category_to_string(&info.category).to_string());
            active.sub_priority = Set(info.sub_priority as i32);
            active.last_modified = Set(systemtime_to_unix(&info.last_modified));
            active.indexed_at = Set(systemtime_to_unix(&info.indexed_at));
            active.has_apt_dat = Set(info.has_apt_dat);
            active.airport_id = Set(info.airport_id.clone());
            active.has_dsf = Set(info.has_dsf);
            active.has_library_txt = Set(info.has_library_txt);
            active.has_textures = Set(info.has_textures);
            active.has_objects = Set(info.has_objects);
            active.texture_count = Set(info.texture_count as i32);
            active.earth_nav_tile_count = Set(info.earth_nav_tile_count as i32);
            active.enabled = Set(info.enabled);
            active.sort_order = Set(info.sort_order as i32);
            active.actual_path = Set(info.actual_path.clone());
            active.continent = Set(info.continent.clone());

            active.update(&txn).await.map_err(ApiError::from)?;

            Self::update_package_libraries_async(&txn, id, info).await?;
        } else {
            let package_id = Self::insert_package_async(&txn, info).await?;
            Self::insert_libraries_async(
                &txn,
                package_id,
                &info.required_libraries,
                LibraryKind::Required,
            )
            .await?;
            Self::insert_libraries_async(
                &txn,
                package_id,
                &info.missing_libraries,
                LibraryKind::Missing,
            )
            .await?;
            Self::insert_libraries_async(
                &txn,
                package_id,
                &info.exported_library_names,
                LibraryKind::Exported,
            )
            .await?;
        }

        Self::set_metadata_async(
            &txn,
            "last_updated",
            &systemtime_to_unix(&SystemTime::now()).to_string(),
        )
        .await?;

        txn.commit().await.map_err(ApiError::from)?;
        Ok(())
    }

    async fn update_package_libraries_async<C>(
        conn: &C,
        package_id: i64,
        info: &SceneryPackageInfo,
    ) -> Result<(), ApiError>
    where
        C: ConnectionTrait,
    {
        required_libraries::Entity::delete_many()
            .filter(required_libraries::Column::PackageId.eq(package_id))
            .exec(conn)
            .await
            .map_err(ApiError::from)?;
        missing_libraries::Entity::delete_many()
            .filter(missing_libraries::Column::PackageId.eq(package_id))
            .exec(conn)
            .await
            .map_err(ApiError::from)?;
        exported_libraries::Entity::delete_many()
            .filter(exported_libraries::Column::PackageId.eq(package_id))
            .exec(conn)
            .await
            .map_err(ApiError::from)?;

        Self::insert_libraries_async(
            conn,
            package_id,
            &info.required_libraries,
            LibraryKind::Required,
        )
        .await?;
        Self::insert_libraries_async(
            conn,
            package_id,
            &info.missing_libraries,
            LibraryKind::Missing,
        )
        .await?;
        Self::insert_libraries_async(
            conn,
            package_id,
            &info.exported_library_names,
            LibraryKind::Exported,
        )
        .await?;

        Ok(())
    }

    /// Delete a package from the database
    pub async fn delete_package(
        conn: &DatabaseConnection,
        folder_name: &str,
    ) -> Result<bool, ApiError> {
        let result = scenery_packages::Entity::delete_many()
            .filter(scenery_packages::Column::FolderName.eq(folder_name))
            .exec(conn)
            .await
            .map_err(ApiError::from)?;
        Ok(result.rows_affected > 0)
    }

    /// Get a single package by folder name (test only)
    #[cfg(test)]
    pub async fn get_package(
        conn: &DatabaseConnection,
        folder_name: &str,
    ) -> Result<Option<SceneryPackageInfo>, ApiError> {
        let pkg = scenery_packages::Entity::find()
            .filter(scenery_packages::Column::FolderName.eq(folder_name))
            .one(conn)
            .await
            .map_err(ApiError::from)?;

        let pkg = match pkg {
            Some(pkg) => pkg,
            None => return Ok(None),
        };

        let mut info = SceneryPackageInfo {
            folder_name: pkg.folder_name.clone(),
            category: string_to_category(&pkg.category),
            sub_priority: pkg.sub_priority as u8,
            last_modified: unix_to_systemtime(pkg.last_modified),
            indexed_at: unix_to_systemtime(pkg.indexed_at),
            has_apt_dat: pkg.has_apt_dat,
            airport_id: pkg.airport_id.clone(),
            has_dsf: pkg.has_dsf,
            has_library_txt: pkg.has_library_txt,
            has_textures: pkg.has_textures,
            has_objects: pkg.has_objects,
            texture_count: pkg.texture_count as usize,
            earth_nav_tile_count: pkg.earth_nav_tile_count as u32,
            enabled: pkg.enabled,
            sort_order: pkg.sort_order as u32,
            required_libraries: Vec::new(),
            missing_libraries: Vec::new(),
            exported_library_names: Vec::new(),
            actual_path: pkg.actual_path.clone(),
            continent: pkg.continent.clone(),
            original_category: pkg
                .original_category
                .as_ref()
                .map(|s| string_to_category(s)),
        };

        info.required_libraries =
            Self::load_package_libraries(conn, pkg.id, LibraryKind::Required).await?;
        info.missing_libraries =
            Self::load_package_libraries(conn, pkg.id, LibraryKind::Missing).await?;
        info.exported_library_names =
            Self::load_package_libraries(conn, pkg.id, LibraryKind::Exported).await?;

        Ok(Some(info))
    }

    #[cfg(test)]
    async fn load_package_libraries(
        conn: &DatabaseConnection,
        package_id: i64,
        kind: LibraryKind,
    ) -> Result<Vec<String>, ApiError> {
        match kind {
            LibraryKind::Required => {
                let rows = required_libraries::Entity::find()
                    .filter(required_libraries::Column::PackageId.eq(package_id))
                    .order_by_asc(required_libraries::Column::Id)
                    .all(conn)
                    .await
                    .map_err(ApiError::from)?;
                Ok(rows.into_iter().map(|row| row.library_name).collect())
            }
            LibraryKind::Missing => {
                let rows = missing_libraries::Entity::find()
                    .filter(missing_libraries::Column::PackageId.eq(package_id))
                    .order_by_asc(missing_libraries::Column::Id)
                    .all(conn)
                    .await
                    .map_err(ApiError::from)?;
                Ok(rows.into_iter().map(|row| row.library_name).collect())
            }
            LibraryKind::Exported => {
                let rows = exported_libraries::Entity::find()
                    .filter(exported_libraries::Column::PackageId.eq(package_id))
                    .order_by_asc(exported_libraries::Column::Id)
                    .all(conn)
                    .await
                    .map_err(ApiError::from)?;
                Ok(rows.into_iter().map(|row| row.library_name).collect())
            }
        }
    }

    /// Update enabled and sort_order for a package
    pub async fn update_entry(
        conn: &DatabaseConnection,
        folder_name: &str,
        enabled: Option<bool>,
        sort_order: Option<u32>,
        category: Option<&SceneryCategory>,
    ) -> Result<bool, ApiError> {
        let mut update = scenery_packages::Entity::update_many()
            .filter(scenery_packages::Column::FolderName.eq(folder_name));
        let mut has_updates = false;

        if let Some(value) = enabled {
            update = update.col_expr(scenery_packages::Column::Enabled, Expr::value(value));
            has_updates = true;
        }
        if let Some(value) = sort_order {
            update = update.col_expr(
                scenery_packages::Column::SortOrder,
                Expr::value(value as i32),
            );
            has_updates = true;
        }
        if let Some(value) = category {
            update = update.col_expr(
                scenery_packages::Column::Category,
                Expr::value(category_to_string(value)),
            );
            has_updates = true;
        }

        if !has_updates {
            return Ok(false);
        }

        let result = update.exec(conn).await.map_err(ApiError::from)?;
        Ok(result.rows_affected > 0)
    }

    /// Batch update entries (enabled and sort_order only)
    /// Uses transaction for optimal performance
    pub async fn batch_update_entries(
        conn: &DatabaseConnection,
        entries: &[crate::models::SceneryEntryUpdate],
    ) -> Result<(), ApiError> {
        let txn = conn.begin().await.map_err(ApiError::from)?;
        let mut not_found: Vec<String> = Vec::new();

        for entry in entries {
            let result = scenery_packages::Entity::update_many()
                .filter(scenery_packages::Column::FolderName.eq(&entry.folder_name))
                .col_expr(
                    scenery_packages::Column::Enabled,
                    Expr::value(entry.enabled),
                )
                .col_expr(
                    scenery_packages::Column::SortOrder,
                    Expr::value(entry.sort_order as i32),
                )
                .exec(&txn)
                .await
                .map_err(ApiError::from)?;

            if result.rows_affected == 0 {
                not_found.push(entry.folder_name.clone());
            }
        }

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

        Self::set_metadata_async(
            &txn,
            "last_updated",
            &systemtime_to_unix(&SystemTime::now()).to_string(),
        )
        .await?;

        txn.commit().await.map_err(ApiError::from)?;
        Ok(())
    }

    /// Get package count
    pub async fn get_package_count(conn: &DatabaseConnection) -> Result<usize, ApiError> {
        let count = scenery_packages::Entity::find()
            .count(conn)
            .await
            .map_err(ApiError::from)?;
        Ok(count as usize)
    }

    /// Check if database has any packages (uses EXISTS for optimal performance)
    pub async fn has_packages(conn: &DatabaseConnection) -> Result<bool, ApiError> {
        let exists = scenery_packages::Entity::find()
            .select_only()
            .column(scenery_packages::Column::Id)
            .limit(1)
            .one(conn)
            .await
            .map_err(ApiError::from)?
            .is_some();
        Ok(exists)
    }

    /// Clear all scenery data from the database
    /// Used before rebuilding index to ensure a completely fresh start
    pub async fn clear_all(conn: &DatabaseConnection) -> Result<(), ApiError> {
        required_libraries::Entity::delete_many()
            .exec(conn)
            .await
            .map_err(ApiError::from)?;
        missing_libraries::Entity::delete_many()
            .exec(conn)
            .await
            .map_err(ApiError::from)?;
        exported_libraries::Entity::delete_many()
            .exec(conn)
            .await
            .map_err(ApiError::from)?;
        scenery_packages::Entity::delete_many()
            .exec(conn)
            .await
            .map_err(ApiError::from)?;
        index_metadata::Entity::delete_many()
            .exec(conn)
            .await
            .map_err(ApiError::from)?;

        logger::log_info("Cleared all scenery index data", Some("database"));
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
enum LibraryKind {
    Required,
    Missing,
    Exported,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{apply_migrations, open_memory_connection};

    fn setup_test_db() -> DatabaseConnection {
        let conn = open_memory_connection().unwrap();
        apply_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_insert_and_load_package() {
        tauri::async_runtime::block_on(async {
            let conn = setup_test_db();

            let info = SceneryPackageInfo {
                folder_name: "TestAirport".to_string(),
                category: SceneryCategory::Airport,
                sub_priority: 0,
                last_modified: SystemTime::now(),
                indexed_at: SystemTime::now(),
                has_apt_dat: true,
                airport_id: Some("TEST".to_string()),
                has_dsf: true,
                has_library_txt: true,
                has_textures: true,
                has_objects: true,
                texture_count: 12,
                earth_nav_tile_count: 3,
                enabled: true,
                sort_order: 10,
                required_libraries: vec!["libA".to_string()],
                missing_libraries: vec!["libB".to_string()],
                exported_library_names: vec!["libC".to_string()],
                actual_path: None,
                continent: Some("NA".to_string()),
                original_category: Some(SceneryCategory::Airport),
            };

            let index = SceneryIndex {
                version: 1,
                packages: vec![(info.folder_name.clone(), info.clone())]
                    .into_iter()
                    .collect(),
                last_updated: SystemTime::now(),
            };

            SceneryQueries::save_all(&conn, &index).await.unwrap();
            let loaded = SceneryQueries::load_all(&conn).await.unwrap();
            let loaded_info = loaded.packages.get("TestAirport").unwrap();

            assert_eq!(loaded_info.folder_name, info.folder_name);
            assert_eq!(loaded_info.category, info.category);
            assert_eq!(loaded_info.required_libraries, info.required_libraries);
            assert_eq!(loaded_info.missing_libraries, info.missing_libraries);
            assert_eq!(
                loaded_info.exported_library_names,
                info.exported_library_names
            );
        });
    }
}

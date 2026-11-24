//! Database module for SQLite-based scenery index storage
//!
//! This module provides SeaORM storage for the scenery index, replacing
//! the previous rusqlite-based access for improved portability.

mod connection;
pub mod entities;
mod legacy;
mod migration;
mod migrations;
mod queries;
mod schema;

#[cfg(test)]
pub use connection::open_memory_connection;
#[cfg(test)]
pub use connection::open_memory_connection_async;
pub use connection::{delete_database, open_connection_async};
#[cfg(test)]
pub use migrations::apply_migrations;
pub use migrations::apply_migrations_async;
pub use migrations::is_schema_compatible;
pub use migrations::reset_schema;
pub use queries::SceneryQueries;
pub use schema::CURRENT_SCHEMA_VERSION;

use sea_orm::DatabaseConnection;

/// Connections are owned by the root captured by each request, never by an
/// application-wide "current root". Existing requests keep their own pool.
pub struct DatabaseState {
    pools: tokio::sync::Mutex<std::collections::HashMap<String, DatabaseConnection>>,
    directory: std::path::PathBuf,
}

pub(crate) fn root_identity(root: &str) -> Result<String, crate::error::ApiError> {
    if root.trim().is_empty() {
        return Err(crate::error::ApiError::validation(
            "X-Plane path is required",
        ));
    }
    let path = std::fs::canonicalize(root)
        .map_err(|e| crate::error::ApiError::validation(format!("Invalid X-Plane root: {e}")))?;
    if !path.is_dir() {
        return Err(crate::error::ApiError::validation(
            "X-Plane root must be a directory",
        ));
    }
    let identity = path
        .to_str()
        .ok_or_else(|| crate::error::ApiError::validation("X-Plane root must be valid Unicode"))?
        .to_owned();
    Ok(identity)
}

impl DatabaseState {
    pub fn new() -> Self {
        Self::in_directory(crate::app_dirs::get_app_data_dir())
    }

    fn in_directory(directory: std::path::PathBuf) -> Self {
        Self {
            pools: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            directory,
        }
    }

    fn path_for_identity(&self, identity: &str) -> std::path::PathBuf {
        use sha2::{Digest, Sha256};
        self.directory
            .join("installations")
            .join(format!("{:x}", Sha256::digest(identity.as_bytes())))
            .join("scenery.db")
    }

    pub async fn get_for_root(
        &self,
        root: &str,
    ) -> Result<DatabaseConnection, crate::error::ApiError> {
        let identity = root_identity(root)?;
        let mut pools = self.pools.lock().await;
        if let Some(conn) = pools.get(&identity) {
            return Ok(conn.clone());
        }
        let conn = connection::open_connection_at(&self.path_for_identity(&identity)).await?;
        apply_migrations_async(&conn).await?;
        if let Err(error) = legacy::import_root_records(&conn, &self.directory, &identity).await {
            crate::logger::log_error(
                &format!("Could not recover previous installation records: {error}"),
                Some("database"),
            );
        }
        pools.insert(identity, conn.clone());
        Ok(conn)
    }

    pub async fn reset_for_root(&self, root: &str) -> Result<(), crate::error::ApiError> {
        let identity = root_identity(root)?;
        let mut pools = self.pools.lock().await;
        let conn = match pools.get(&identity) {
            Some(conn) => conn.clone(),
            None => connection::open_connection_at(&self.path_for_identity(&identity)).await?,
        };
        reset_schema(&conn).await?;
        let fresh = connection::open_connection_at(&self.path_for_identity(&identity)).await?;
        apply_migrations_async(&fresh).await?;
        legacy::mark_imported(&fresh).await?;
        pools.insert(identity, fresh);
        // Do not close cloned pools still held by in-flight operations.
        Ok(())
    }
}

#[cfg(test)]
mod root_isolation_tests {
    use super::*;
    use crate::scenery_index::SceneryIndexManager;
    use sea_orm::{ConnectionTrait, DatabaseBackend, EntityTrait, Set, Statement};
    use std::fs;

    #[test]
    fn roots_have_independent_persistent_data_and_delayed_writes() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let a = temp.path().join("XP-A");
            let b = temp.path().join("XP-B");
            fs::create_dir_all(&a).unwrap();
            fs::create_dir_all(&b).unwrap();
            let directory = temp.path().join("data");
            fs::create_dir_all(&directory).unwrap();
            let legacy = directory.join("scenery.db");
            fs::write(&legacy, b"legacy database must remain untouched").unwrap();
            let state = DatabaseState::in_directory(directory.clone());
            let a_conn = state.get_for_root(a.to_str().unwrap()).await.unwrap();
            SceneryQueries::set_metadata(&a_conn, "owner", "A")
                .await
                .unwrap();
            let b_conn = state.get_for_root(b.to_str().unwrap()).await.unwrap();
            assert_eq!(
                SceneryQueries::get_metadata(&b_conn, "owner")
                    .await
                    .unwrap(),
                None
            );
            // Simulate an A request completing after the UI switched to B.
            SceneryQueries::set_metadata(&a_conn, "late", "A")
                .await
                .unwrap();
            assert_eq!(
                SceneryQueries::get_metadata(&b_conn, "late").await.unwrap(),
                None
            );
            SceneryQueries::set_metadata(&b_conn, "owner", "B")
                .await
                .unwrap();
            state.reset_for_root(a.to_str().unwrap()).await.unwrap();
            assert_eq!(
                SceneryQueries::get_metadata(&b_conn, "owner")
                    .await
                    .unwrap()
                    .as_deref(),
                Some("B")
            );
            let reopened = DatabaseState::in_directory(directory);
            let b_reopened = reopened.get_for_root(b.to_str().unwrap()).await.unwrap();
            assert_eq!(
                SceneryQueries::get_metadata(&b_reopened, "owner")
                    .await
                    .unwrap()
                    .as_deref(),
                Some("B")
            );
            assert_eq!(
                fs::read(legacy).unwrap(),
                b"legacy database must remain untouched"
            );
        });
    }

    #[test]
    fn first_scan_of_b_does_not_report_a_packages_as_removed() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let a = temp.path().join("XP-A");
            let b = temp.path().join("XP-B");
            fs::create_dir_all(a.join("Custom Scenery").join("Only-A")).unwrap();
            fs::write(
                a.join("Custom Scenery").join("Only-A").join("library.txt"),
                "A\n800\nLIBRARY\n",
            )
            .unwrap();
            fs::create_dir_all(b.join("Custom Scenery")).unwrap();
            let state = DatabaseState::in_directory(temp.path().join("data"));
            let a_conn = state.get_for_root(a.to_str().unwrap()).await.unwrap();
            let a_manager = SceneryIndexManager::new(&a, a_conn);
            let index = a_manager.rebuild_index().await.unwrap();
            assert!(index.packages.contains_key("Only-A"));
            let b_conn = state.get_for_root(b.to_str().unwrap()).await.unwrap();
            let b_manager = SceneryIndexManager::new(&b, b_conn);
            let scan = b_manager.quick_scan_and_update().await.unwrap();
            assert!(!scan.index_exists);
            assert!(scan.removed.is_empty());
            b_manager.rebuild_index().await.unwrap();
            assert!(b_manager.index_status().await.unwrap().index_exists);
            let empty_scan = b_manager.quick_scan_and_update().await.unwrap();
            assert!(empty_scan.index_exists);
            assert!(empty_scan.removed.is_empty());
            let new_package = b.join("Custom Scenery").join("Only-B");
            fs::create_dir_all(&new_package).unwrap();
            fs::write(new_package.join("library.txt"), "A\n800\nLIBRARY\n").unwrap();
            let added = b_manager.quick_scan_and_update().await.unwrap();
            assert_eq!(added.added, vec!["Only-B".to_string()]);
            assert!(added.removed.is_empty());
            assert_eq!(a_manager.get_stats().await.unwrap().total_packages, 1);
            assert!(a_manager.get_package("Only-B").await.unwrap().is_none());
        });
    }

    #[tokio::test]
    async fn legacy_records_are_imported_once_to_their_own_installation() {
        use entities::{airport_flatten_overrides as flatten, gateway_installs as gateway};

        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("XP-A");
        let b = temp.path().join("XP-B");
        let data = temp.path().join("data");
        let legacy_path = data.join("scenery.db");
        let legacy = connection::open_connection_at(&legacy_path).await.unwrap();
        apply_migrations_async(&legacy).await.unwrap();
        for (root, scenery_id) in [(&a, 42), (&b, 43)] {
            fs::create_dir_all(root).unwrap();
            gateway::Entity::insert(gateway::ActiveModel {
                xplane_path: Set(crate::gateway::normalize_xplane_key(root)),
                airport_icao: Set("TEST".to_string()),
                airport_name: Set("Test Airport".to_string()),
                scenery_id: Set(scenery_id),
                folder_name: Set("Test Folder".to_string()),
                artist: Set(None),
                approved_date: Set(None),
                installed_at: Set(1),
                ..Default::default()
            })
            .exec(&legacy)
            .await
            .unwrap();
        }
        fs::write(a.join("apt.dat"), "test").unwrap();
        for (path, desired, updated) in [(a.clone(), false, 1), (a.join("."), true, 2)] {
            let xplane_path = path.to_string_lossy().into_owned();
            #[cfg(windows)]
            let xplane_path = xplane_path.to_lowercase();
            flatten::Entity::insert(flatten::ActiveModel {
                xplane_path: Set(xplane_path),
                icao: Set("TEST".to_string()),
                source_path: Set(path.join("apt.dat").display().to_string()),
                source_kind: Set("default".to_string()),
                source_label: Set("Default".to_string()),
                folder_name: Set(None),
                airport_name: Set("Test Airport".to_string()),
                desired_flattened: Set(desired),
                updated_at: Set(updated),
                ..Default::default()
            })
            .exec(&legacy)
            .await
            .unwrap();
        }
        SceneryQueries::set_metadata(&legacy, "owner", "unknown")
            .await
            .unwrap();
        // Establish the byte baseline after committed WAL frames reach the main file.
        let checkpoint = legacy
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA wal_checkpoint(TRUNCATE)",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(checkpoint.try_get_by_index::<i64>(0).unwrap(), 0);
        legacy.close().await.unwrap();
        let legacy_before = fs::read(&legacy_path).unwrap();

        let state = DatabaseState::in_directory(data.clone());
        let a_conn = state
            .get_for_root(a.join(".").to_str().unwrap())
            .await
            .unwrap();
        let b_conn = state.get_for_root(b.to_str().unwrap()).await.unwrap();
        let rows = gateway::Entity::find().all(&a_conn).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scenery_id, 42);
        let rows = gateway::Entity::find().all(&b_conn).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scenery_id, 43);
        let overrides = flatten::Entity::find().all(&a_conn).await.unwrap();
        assert_eq!(overrides.len(), 1);
        assert!(overrides[0].desired_flattened);
        assert_eq!(
            overrides[0].xplane_path,
            root_identity(a.to_str().unwrap()).unwrap()
        );
        assert!(flatten::Entity::find()
            .all(&b_conn)
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            SceneryQueries::get_metadata(&a_conn, "owner")
                .await
                .unwrap(),
            None
        );
        flatten::Entity::delete_many().exec(&a_conn).await.unwrap();
        state.reset_for_root(a.to_str().unwrap()).await.unwrap();
        let reopened = DatabaseState::in_directory(data);
        let a_again = reopened.get_for_root(a.to_str().unwrap()).await.unwrap();
        assert!(gateway::Entity::find()
            .all(&a_again)
            .await
            .unwrap()
            .is_empty());
        assert!(flatten::Entity::find()
            .all(&a_again)
            .await
            .unwrap()
            .is_empty());
        assert_eq!(fs::read(&legacy_path).unwrap(), legacy_before);
    }

    #[tokio::test]
    async fn cached_classification_does_not_skip_first_scenery_baseline() {
        let temp = tempfile::tempdir().unwrap();
        let scenery = temp.path().join("Custom Scenery");
        let package = scenery.join("Only Package");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("library.txt"), "A\n800\nLIBRARY\n").unwrap();
        let ini = "SCENERY_PACK_DISABLED Custom Scenery/Only Package/\n";
        fs::write(scenery.join("scenery_packs.ini"), ini).unwrap();
        let conn = open_memory_connection_async().await.unwrap();
        apply_migrations_async(&conn).await.unwrap();
        let manager = SceneryIndexManager::new(temp.path(), conn);

        manager.get_or_classify(&package).await.unwrap();
        assert!(!manager.index_status().await.unwrap().index_exists);
        assert!(!manager.quick_scan_and_update().await.unwrap().index_exists);

        let index = manager.rebuild_index().await.unwrap();
        assert!(!index.packages["Only Package"].enabled);
        assert!(manager.index_status().await.unwrap().index_exists);
        assert_eq!(
            fs::read_to_string(scenery.join("scenery_packs.ini")).unwrap(),
            ini
        );
    }

    #[test]
    fn root_identity_rejects_missing_roots_and_normalizes_aliases() {
        let temp = tempfile::tempdir().unwrap();
        assert!(root_identity("").is_err());
        assert!(root_identity(temp.path().join("missing").to_str().unwrap()).is_err());
        assert_eq!(
            root_identity(temp.path().to_str().unwrap()).unwrap(),
            root_identity(temp.path().join(".").to_str().unwrap()).unwrap()
        );
        #[cfg(unix)]
        {
            let alias = temp.path().join("alias");
            std::os::unix::fs::symlink(temp.path(), &alias).unwrap();
            assert_eq!(
                root_identity(temp.path().to_str().unwrap()).unwrap(),
                root_identity(alias.to_str().unwrap()).unwrap()
            );
        }
    }
}

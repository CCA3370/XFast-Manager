//! Database connection management with WAL mode configuration

use crate::app_dirs;
use crate::error::ApiError;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
use std::path::PathBuf;
use std::time::Duration;

/// Get the path to the scenery database file
pub fn get_database_path() -> PathBuf {
    app_dirs::get_database_path()
}

/// Delete the database file to allow fresh rebuild
/// Returns true if file was deleted, false if it didn't exist
pub fn delete_database() -> Result<bool, ApiError> {
    let db_path = get_database_path();

    if db_path.exists() {
        // Also delete WAL and SHM files if they exist
        let wal_path = db_path.with_extension("db-wal");
        let shm_path = db_path.with_extension("db-shm");

        std::fs::remove_file(&db_path)
            .map_err(|e| ApiError::database(format!("Failed to delete database: {}", e)))?;

        // Ignore errors for WAL/SHM files - they may not exist
        let _ = std::fs::remove_file(&wal_path);
        let _ = std::fs::remove_file(&shm_path);

        Ok(true)
    } else {
        Ok(false)
    }
}

fn sqlite_url(db_path: &PathBuf) -> String {
    let normalized = db_path.to_string_lossy().replace('\\', "/");
    format!("sqlite://{}?mode=rwc", normalized)
}

async fn execute_pragma(db: &DatabaseConnection, sql: &str) -> Result<(), ApiError> {
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        sql.to_string(),
    ))
    .await
    .map_err(ApiError::from)?;
    Ok(())
}

/// Configure database pragmas for optimal performance
async fn configure_pragmas(db: &DatabaseConnection) -> Result<(), ApiError> {
    // Performance optimizations:
    // - WAL mode: Better concurrent read/write performance
    // - Foreign keys: Referential integrity
    // - Busy timeout: Wait up to 5 seconds for locks
    // - Synchronous NORMAL: Good balance of safety and speed
    // - Cache size: 64MB cache (negative value = KB)
    // - Temp store: Keep temp tables in memory
    // - Mmap size: 256MB memory-mapped I/O for faster reads
    execute_pragma(db, "PRAGMA journal_mode=WAL;").await?;
    execute_pragma(db, "PRAGMA foreign_keys=ON;").await?;
    execute_pragma(db, "PRAGMA busy_timeout=5000;").await?;
    execute_pragma(db, "PRAGMA synchronous=NORMAL;").await?;
    execute_pragma(db, "PRAGMA cache_size=-65536;").await?;
    execute_pragma(db, "PRAGMA temp_store=MEMORY;").await?;
    execute_pragma(db, "PRAGMA mmap_size=268435456;").await?;
    Ok(())
}

/// Open a database connection with optimized settings
///
/// Configures the connection with:
/// - WAL journal mode for better concurrent access
/// - Foreign key constraints enabled
/// - Busy timeout for concurrent access
/// - NORMAL synchronous mode for better performance
/// - Large cache size for better read performance
/// - Memory-mapped I/O for faster reads
pub async fn open_connection_async() -> Result<DatabaseConnection, ApiError> {
    let db_path = get_database_path();

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ApiError::database(format!("Failed to create database directory: {}", e))
        })?;
    }

    let mut options = ConnectOptions::new(sqlite_url(&db_path));
    options
        .max_connections(5)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(10))
        .sqlx_logging(false);

    let db = Database::connect(options)
        .await
        .map_err(|e| ApiError::database(format!("Failed to open database: {}", e)))?;

    configure_pragmas(&db).await?;

    Ok(db)
}

pub fn open_connection() -> Result<DatabaseConnection, ApiError> {
    tauri::async_runtime::block_on(open_connection_async())
}

/// Open an in-memory database for testing
pub async fn open_memory_connection_async() -> Result<DatabaseConnection, ApiError> {
    let db = Database::connect("sqlite::memory:")
        .await
        .map_err(|e| ApiError::database(format!("Failed to open in-memory database: {}", e)))?;
    execute_pragma(&db, "PRAGMA foreign_keys=ON;").await?;
    Ok(db)
}

#[cfg(test)]
pub fn open_memory_connection() -> Result<DatabaseConnection, ApiError> {
    tauri::async_runtime::block_on(open_memory_connection_async())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_path() {
        let path = get_database_path();
        assert!(path.to_string_lossy().contains("scenery.db"));
    }

    #[test]
    fn test_open_memory_connection() {
        let conn = open_memory_connection().expect("Failed to open in-memory connection");
        let fk_enabled: i32 = tauri::async_runtime::block_on(async {
            let row = conn
                .query_one(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    "PRAGMA foreign_keys".to_string(),
                ))
                .await
                .expect("Failed to query PRAGMA");
            row.unwrap().try_get_by_index(0).unwrap_or(0)
        });
        assert_eq!(fk_enabled, 1);
    }
}

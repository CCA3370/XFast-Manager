//! Database connection management with WAL mode configuration

use crate::error::ApiError;
use rusqlite::Connection;
use std::path::PathBuf;

/// Wrapper around rusqlite Connection with RAII cleanup
pub struct DatabaseConnection {
    conn: Connection,
}

impl DatabaseConnection {
    /// Create a new database connection wrapper
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Consume the wrapper and return the underlying connection
    #[cfg(test)]
    pub fn into_inner(self) -> Connection {
        self.conn
    }
}

impl std::ops::Deref for DatabaseConnection {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl std::ops::DerefMut for DatabaseConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

/// Get the path to the scenery database file
pub fn get_database_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data)
                .join("XFast Manager")
                .join("scenery.db");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("XFast Manager")
                .join("scenery.db");
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("xfastmanager")
                .join("scenery.db");
        }
    }

    // Fallback to current directory
    PathBuf::from("scenery.db")
}

/// Configure database pragmas for optimal performance
fn configure_pragmas(conn: &Connection) -> Result<(), ApiError> {
    // Performance optimizations:
    // - WAL mode: Better concurrent read/write performance
    // - Foreign keys: Referential integrity
    // - Busy timeout: Wait up to 5 seconds for locks
    // - Synchronous NORMAL: Good balance of safety and speed
    // - Cache size: 64MB cache (negative value = KB)
    // - Temp store: Keep temp tables in memory
    // - Mmap size: 256MB memory-mapped I/O for faster reads
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;
        PRAGMA busy_timeout=5000;
        PRAGMA synchronous=NORMAL;
        PRAGMA cache_size=-65536;
        PRAGMA temp_store=MEMORY;
        PRAGMA mmap_size=268435456;
        ",
    )
    .map_err(|e| ApiError::database(format!("Failed to configure database: {}", e)))?;
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
pub fn open_connection() -> Result<DatabaseConnection, ApiError> {
    let db_path = get_database_path();

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ApiError::database(format!("Failed to create database directory: {}", e)))?;
    }

    // Open the database connection
    let conn = Connection::open(&db_path)
        .map_err(|e| ApiError::database(format!("Failed to open database: {}", e)))?;

    // Configure pragmas for optimal performance
    configure_pragmas(&conn)?;

    Ok(DatabaseConnection::new(conn))
}

/// Open an in-memory database for testing
#[cfg(test)]
pub fn open_memory_connection() -> Result<DatabaseConnection, ApiError> {
    let conn = Connection::open_in_memory()
        .map_err(|e| ApiError::database(format!("Failed to open in-memory database: {}", e)))?;

    conn.execute_batch(
        "
        PRAGMA foreign_keys=ON;
        ",
    )
    .map_err(|e| ApiError::database(format!("Failed to configure database: {}", e)))?;

    Ok(DatabaseConnection::new(conn))
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
        // Verify foreign keys are enabled
        let fk_enabled: i32 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk_enabled, 1);
    }
}

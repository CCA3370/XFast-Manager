//! Database schema migrations

use super::schema::{CREATE_SCHEMA, CURRENT_SCHEMA_VERSION, GET_SCHEMA_VERSION, INSERT_SCHEMA_VERSION};
use crate::error::ApiError;
use crate::logger;
use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get the current schema version from the database
fn get_current_version(conn: &Connection) -> Result<Option<i32>, ApiError> {
    // Check if schema_version table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| ApiError::database(format!("Failed to check schema_version table: {}", e)))?;

    if !table_exists {
        return Ok(None);
    }

    let version: Option<i32> = conn
        .query_row(GET_SCHEMA_VERSION, [], |row| row.get(0))
        .map_err(|e| ApiError::database(format!("Failed to get schema version: {}", e)))?;

    Ok(version)
}

/// Apply all pending migrations to bring the database up to the current schema version
pub fn apply_migrations(conn: &Connection) -> Result<(), ApiError> {
    let current_version = get_current_version(conn)?;

    match current_version {
        None => {
            // Fresh database - create initial schema
            logger::log_info("Creating initial database schema", Some("database"));
            create_initial_schema(conn)?;
        }
        Some(version) if version < CURRENT_SCHEMA_VERSION => {
            // Need to apply migrations
            logger::log_info(
                &format!(
                    "Migrating database from version {} to {}",
                    version, CURRENT_SCHEMA_VERSION
                ),
                Some("database"),
            );
            apply_version_migrations(conn, version)?;
        }
        Some(version) if version == CURRENT_SCHEMA_VERSION => {
            // Already up to date
            logger::log_info("Database schema is up to date", Some("database"));
        }
        Some(version) => {
            // Database is newer than current code - this shouldn't happen
            return Err(ApiError::migration_failed(format!(
                "Database schema version {} is newer than supported version {}",
                version, CURRENT_SCHEMA_VERSION
            )));
        }
    }

    Ok(())
}

/// Create the initial database schema
fn create_initial_schema(conn: &Connection) -> Result<(), ApiError> {
    conn.execute_batch(CREATE_SCHEMA)
        .map_err(|e| ApiError::migration_failed(format!("Failed to create database schema: {}", e)))?;

    // Record the schema version
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        INSERT_SCHEMA_VERSION,
        rusqlite::params![CURRENT_SCHEMA_VERSION, now, "Initial schema"],
    )
    .map_err(|e| ApiError::migration_failed(format!("Failed to record schema version: {}", e)))?;

    logger::log_info(
        &format!("Database schema created at version {}", CURRENT_SCHEMA_VERSION),
        Some("database"),
    );

    Ok(())
}

/// Apply incremental migrations from a given version
fn apply_version_migrations(conn: &Connection, from_version: i32) -> Result<(), ApiError> {
    // Currently no migrations needed since we're at version 1
    // Future migrations would be applied here based on from_version

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // For future migrations, add match arms like:
    // if from_version < 2 { migrate_v1_to_v2(conn)?; }
    // if from_version < 3 { migrate_v2_to_v3(conn)?; }

    // Record the final version
    conn.execute(
        INSERT_SCHEMA_VERSION,
        rusqlite::params![CURRENT_SCHEMA_VERSION, now, "Migration completed"],
    )
    .map_err(|e| {
        ApiError::migration_failed(format!("Failed to record schema version after migration: {}", e))
    })?;

    logger::log_info(
        &format!(
            "Database migrated from version {} to {}",
            from_version, CURRENT_SCHEMA_VERSION
        ),
        Some("database"),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::open_memory_connection;

    #[test]
    fn test_apply_migrations_fresh_db() {
        let conn = open_memory_connection().unwrap();
        apply_migrations(&conn).expect("Failed to apply migrations");

        // Verify schema was created
        let version = get_current_version(&conn).unwrap();
        assert_eq!(version, Some(CURRENT_SCHEMA_VERSION));
    }

    #[test]
    fn test_apply_migrations_idempotent() {
        let conn = open_memory_connection().unwrap();

        // Apply migrations twice
        apply_migrations(&conn).expect("First migration failed");
        apply_migrations(&conn).expect("Second migration failed");

        // Should still be at current version
        let version = get_current_version(&conn).unwrap();
        assert_eq!(version, Some(CURRENT_SCHEMA_VERSION));
    }
}

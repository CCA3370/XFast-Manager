//! Database schema migrations

use crate::error::ApiError;
use crate::logger;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use sea_orm_migration::MigratorTrait;

use super::migration::Migrator;

/// Apply all pending migrations to bring the database up to date
pub async fn apply_migrations_async(conn: &DatabaseConnection) -> Result<(), ApiError> {
    Migrator::up(conn, None)
        .await
        .map_err(|e| ApiError::migration_failed(format!("Migration failed: {}", e)))?;
    logger::log_info("Database schema is up to date", Some("database"));
    Ok(())
}

/// Sync wrapper for applying migrations
#[cfg(test)]
pub fn apply_migrations(conn: &DatabaseConnection) -> Result<(), ApiError> {
    tauri::async_runtime::block_on(apply_migrations_async(conn))
}

/// Returns `false` if the database schema is missing columns expected by the current entity model.
/// This can happen when an old database (created before the SeaORM migration system) is opened
/// and the `CREATE TABLE IF NOT EXISTS` in migration 001 silently skips the existing table.
pub async fn is_schema_compatible(conn: &DatabaseConnection) -> Result<bool, ApiError> {
    let row = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) FROM pragma_table_info('scenery_packages') \
             WHERE name IN ('folder_name', 'airport_id')"
                .to_owned(),
        ))
        .await
        .map_err(ApiError::from)?;

    let count: i64 = row
        .map(|r| r.try_get_by_index(0).unwrap_or(0i64))
        .unwrap_or(0);

    Ok(count >= 2)
}

/// Drop all managed tables and re-run migrations on the existing connection.
/// Used to recover from an incompatible schema without restarting the process.
pub async fn reset_schema(conn: &DatabaseConnection) -> Result<(), ApiError> {
    // Disable FK constraints so we can drop in any order.
    conn.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA foreign_keys = OFF".to_owned(),
    ))
    .await
    .map_err(ApiError::from)?;

    for table in &[
        "exported_libraries",
        "missing_libraries",
        "required_libraries",
        "index_metadata",
        "scenery_packages",
        "schema_version",   // legacy rusqlite version table
        "seaql_migrations", // reset migration tracking so migration 001 re-runs
    ] {
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("DROP TABLE IF EXISTS \"{}\"", table),
        ))
        .await
        .map_err(ApiError::from)?;
    }

    conn.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA foreign_keys = ON".to_owned(),
    ))
    .await
    .map_err(ApiError::from)?;

    // Re-run migrations to recreate all tables with the correct schema.
    Migrator::up(conn, None)
        .await
        .map_err(|e| ApiError::migration_failed(format!("Schema reset failed: {}", e)))?;

    logger::log_info("Database schema reset and recreated", Some("database"));
    Ok(())
}

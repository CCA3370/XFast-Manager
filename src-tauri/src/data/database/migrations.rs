//! Database schema migrations

use crate::error::ApiError;
use crate::logger;
use sea_orm::DatabaseConnection;
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
pub fn apply_migrations(conn: &DatabaseConnection) -> Result<(), ApiError> {
    tauri::async_runtime::block_on(apply_migrations_async(conn))
}

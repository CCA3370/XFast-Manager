//! Database module for SQLite-based scenery index storage
//!
//! This module provides SeaORM storage for the scenery index, replacing
//! the previous rusqlite-based access for improved portability.

mod connection;
mod entities;
mod migration;
mod migrations;
mod queries;
mod schema;

#[cfg(test)]
pub use connection::open_memory_connection;
pub use connection::{delete_database, open_connection_async};
#[cfg(test)]
pub use migrations::apply_migrations;
pub use migrations::apply_migrations_async;
pub use migrations::is_schema_compatible;
pub use migrations::reset_schema;
pub use queries::SceneryQueries;
pub use schema::CURRENT_SCHEMA_VERSION;

use sea_orm::DatabaseConnection;

/// Thread-safe, replaceable database connection pool.
///
/// Wraps `DatabaseConnection` in a `std::sync::RwLock` so that
/// `reset_and_reinitialize` can atomically swap the pool for a fresh one after
/// a schema reset, clearing all sqlx prepared-statement caches that would
/// otherwise return stale column-descriptor errors after DROP + CREATE TABLE.
pub struct DatabaseState(std::sync::RwLock<DatabaseConnection>);

impl DatabaseState {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self(std::sync::RwLock::new(conn))
    }

    /// Return a clone of the current connection (cheap `Arc` clone, sync).
    pub fn get(&self) -> DatabaseConnection {
        self.0.read().expect("database lock poisoned").clone()
    }

    /// Drop all managed tables, re-open a fresh pool, and run migrations.
    ///
    /// After this returns, every subsequent `get()` call returns a connection
    /// from a pool whose sqlx prepared-statement cache is completely empty,
    /// eliminating stale column-descriptor errors that persist after an
    /// in-place schema reset on a live pool.
    pub async fn reset(&self) -> Result<(), crate::error::ApiError> {
        // Step 1 — reset schema on the current connection (no lock held).
        reset_schema(&self.get()).await?;

        // Step 2 — open a brand-new pool with an empty prepared-statement cache.
        let new_conn = open_connection_async().await?;

        // Step 3 — atomically swap old pool for the new one under the write lock.
        let old_conn = {
            let mut lock = self.0.write().expect("database lock poisoned");
            std::mem::replace(&mut *lock, new_conn)
        };

        // Step 4 — close the old pool outside the lock.
        let _ = old_conn.close().await;

        crate::logger::log_info(
            "Database connection pool replaced with fresh instance",
            Some("database"),
        );
        Ok(())
    }
}

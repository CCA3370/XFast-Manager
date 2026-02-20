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
pub use queries::SceneryQueries;
pub use schema::CURRENT_SCHEMA_VERSION;

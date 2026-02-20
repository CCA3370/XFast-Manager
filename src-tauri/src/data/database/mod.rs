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

pub use connection::{
	delete_database, get_database_path, open_connection, open_connection_async,
	open_memory_connection_async,
};
#[cfg(test)]
pub use connection::open_memory_connection;
pub use migrations::{apply_migrations, apply_migrations_async};
pub use queries::SceneryQueries;
pub use schema::CURRENT_SCHEMA_VERSION;

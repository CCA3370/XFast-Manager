//! Database module for SQLite-based scenery index storage
//!
//! This module provides SQLite storage for the scenery index, replacing
//! the previous JSON file-based storage for improved performance with
//! large scenery libraries (1000+ packages).

mod connection;
mod migrations;
mod queries;
mod schema;

pub use connection::{delete_database, get_database_path, open_connection};
pub use migrations::apply_migrations;
pub use queries::SceneryQueries;
pub use schema::CURRENT_SCHEMA_VERSION;

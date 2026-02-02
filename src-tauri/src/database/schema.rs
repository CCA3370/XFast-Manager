//! Database schema definitions

/// Current schema version for migration tracking
pub const CURRENT_SCHEMA_VERSION: i32 = 2;

/// SQL statements for creating the database schema
pub const CREATE_SCHEMA: &str = r#"
-- Version tracking table
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL,
    description TEXT
);

-- Main scenery packages table
CREATE TABLE IF NOT EXISTS scenery_packages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_name TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL,
    sub_priority INTEGER NOT NULL DEFAULT 0,
    last_modified INTEGER NOT NULL,
    indexed_at INTEGER NOT NULL,
    has_apt_dat INTEGER NOT NULL DEFAULT 0,
    has_dsf INTEGER NOT NULL DEFAULT 0,
    has_library_txt INTEGER NOT NULL DEFAULT 0,
    has_textures INTEGER NOT NULL DEFAULT 0,
    has_objects INTEGER NOT NULL DEFAULT 0,
    texture_count INTEGER NOT NULL DEFAULT 0,
    earth_nav_tile_count INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    actual_path TEXT,
    continent TEXT,
    country TEXT,
    primary_latitude INTEGER,
    primary_longitude INTEGER
);

-- Required libraries (libraries that this package depends on)
CREATE TABLE IF NOT EXISTS required_libraries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id INTEGER NOT NULL,
    library_name TEXT NOT NULL,
    FOREIGN KEY (package_id) REFERENCES scenery_packages(id) ON DELETE CASCADE,
    UNIQUE(package_id, library_name)
);

-- Missing libraries (required libraries that are not installed)
CREATE TABLE IF NOT EXISTS missing_libraries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id INTEGER NOT NULL,
    library_name TEXT NOT NULL,
    FOREIGN KEY (package_id) REFERENCES scenery_packages(id) ON DELETE CASCADE,
    UNIQUE(package_id, library_name)
);

-- Exported libraries (library names this package provides via library.txt)
CREATE TABLE IF NOT EXISTS exported_libraries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id INTEGER NOT NULL,
    library_name TEXT NOT NULL,
    FOREIGN KEY (package_id) REFERENCES scenery_packages(id) ON DELETE CASCADE,
    UNIQUE(package_id, library_name)
);

-- Scenery coordinates (DSF tile coordinates for each package)
CREATE TABLE IF NOT EXISTS scenery_coordinates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id INTEGER NOT NULL,
    latitude INTEGER NOT NULL,
    longitude INTEGER NOT NULL,
    FOREIGN KEY (package_id) REFERENCES scenery_packages(id) ON DELETE CASCADE,
    UNIQUE(package_id, latitude, longitude)
);

-- Index metadata (key-value store for general index info)
CREATE TABLE IF NOT EXISTS index_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_packages_category ON scenery_packages(category);
CREATE INDEX IF NOT EXISTS idx_packages_sort_order ON scenery_packages(sort_order);
CREATE INDEX IF NOT EXISTS idx_packages_category_order ON scenery_packages(category, sort_order);
CREATE INDEX IF NOT EXISTS idx_packages_enabled ON scenery_packages(enabled);
CREATE INDEX IF NOT EXISTS idx_packages_continent ON scenery_packages(continent);
CREATE INDEX IF NOT EXISTS idx_packages_country ON scenery_packages(country);
CREATE INDEX IF NOT EXISTS idx_required_libraries_name ON required_libraries(library_name);
CREATE INDEX IF NOT EXISTS idx_exported_libraries_name ON exported_libraries(library_name);
CREATE INDEX IF NOT EXISTS idx_coordinates_package ON scenery_coordinates(package_id);
"#;

/// SQL statement to insert initial schema version
pub const INSERT_SCHEMA_VERSION: &str = r#"
INSERT OR REPLACE INTO schema_version (version, applied_at, description)
VALUES (?1, ?2, ?3)
"#;

/// SQL to get current schema version
pub const GET_SCHEMA_VERSION: &str = r#"
SELECT MAX(version) FROM schema_version
"#;

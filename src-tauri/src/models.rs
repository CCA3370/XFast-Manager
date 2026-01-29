use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum AddonType {
    Aircraft,
    /// Scenery with Earth nav data (.dsf files)
    Scenery,
    /// Scenery library with library.txt
    SceneryLibrary,
    Plugin,
    Navdata,
    /// Aircraft livery (auto-detected by pattern)
    Livery,
}

/// Represents a nested archive within another archive
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NestedArchiveInfo {
    /// Path within parent archive (e.g., "aircraft/A330.zip")
    pub internal_path: String,
    /// Password for this specific nested archive (if different from parent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Archive format: "zip", "7z", or "rar"
    pub format: String,
}

/// Extraction chain for nested archives (outer to inner order)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionChain {
    /// Ordered list of archives to extract (outer to inner)
    /// First element is the outermost archive
    pub archives: Vec<NestedArchiveInfo>,
    /// Final internal root after all extractions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_internal_root: Option<String>,
}

/// Navdata cycle information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavdataInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub airac: Option<String>,
}

/// Version information for Aircraft/Plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTask {
    pub id: String,
    #[serde(rename = "type")]
    pub addon_type: AddonType,
    pub source_path: String,
    /// Original input path (the file/folder that was dragged or right-clicked)
    /// This is used for deletion after successful installation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_input_path: Option<String>,
    pub target_path: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_exists: Option<bool>,
    /// For archives: the root folder path inside the archive to extract from
    /// e.g., "MyScenery" if archive contains "MyScenery/Earth nav data/..."
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_internal_root: Option<String>,
    /// For nested archives: extraction chain (takes precedence over archive_internal_root)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_chain: Option<ExtractionChain>,
    /// Whether to overwrite existing folder (delete before install)
    #[serde(default)]
    pub should_overwrite: bool,
    /// Password for encrypted archives
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Estimated uncompressed size in bytes (for archives)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_size: Option<u64>,
    /// Size warning message if archive is suspiciously large or has high compression ratio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_warning: Option<String>,
    /// Whether user has confirmed they trust this archive (for large/suspicious archives)
    #[serde(default)]
    pub size_confirmed: bool,
    /// For Navdata: existing cycle info (if conflict exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_navdata_info: Option<NavdataInfo>,
    /// For Navdata: new cycle info to be installed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_navdata_info: Option<NavdataInfo>,
    /// For Aircraft/Plugin: existing version info (if conflict exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_version_info: Option<VersionInfo>,
    /// For Aircraft/Plugin: new version info to be installed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_version_info: Option<VersionInfo>,
    /// Whether to backup liveries during clean install (Aircraft only)
    pub backup_liveries: bool,
    /// Whether to backup configuration files during clean install (Aircraft only)
    pub backup_config_files: bool,
    /// Glob patterns for config files to backup (Aircraft only)
    pub config_file_patterns: Vec<String>,
    /// File hashes collected during scanning (for verification)
    /// Key: relative path within addon, Value: FileHash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_hashes: Option<HashMap<String, FileHash>>,
    /// Whether hash verification is enabled for this task
    #[serde(default = "default_true")]
    pub enable_verification: bool,
    /// For Livery: the aircraft type this livery belongs to (e.g., "FF777")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub livery_aircraft_type: Option<String>,
    /// For Livery: whether the target aircraft is installed
    #[serde(default = "default_true")]
    pub livery_aircraft_found: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub tasks: Vec<InstallTask>,
    pub errors: Vec<String>,
    /// List of archive paths that require a password
    #[serde(default)]
    pub password_required: Vec<String>,
    /// Map of nested archive paths to their parent archive
    /// Key format: "parent.zip/nested.zip", Value: "parent.zip"
    #[serde(default)]
    pub nested_password_required: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct NavdataCycle {
    pub name: String,
    #[serde(default)]
    pub cycle: Option<String>,
    #[serde(default)]
    pub airac: Option<String>,
}

#[derive(Debug)]
pub struct DetectedItem {
    pub addon_type: AddonType,
    pub path: String,
    /// Original input path (the file/folder that was dragged or right-clicked)
    pub original_input_path: String,
    pub display_name: String,
    /// For archives: the root folder path inside the archive
    pub archive_internal_root: Option<String>,
    /// For nested archives: extraction chain (takes precedence over archive_internal_root)
    pub extraction_chain: Option<ExtractionChain>,
    /// For Navdata: cycle info from the new navdata to be installed
    pub navdata_info: Option<NavdataInfo>,
    /// For Livery: the aircraft type this livery belongs to (e.g., "FF777")
    pub livery_aircraft_type: Option<String>,
    /// For Aircraft/Plugin: version info from the addon to be installed
    pub version_info: Option<VersionInfo>,
}

/// Installation progress event sent to frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    /// Overall progress percentage (0.0 - 100.0)
    /// Installation takes 90%, verification takes 10%
    pub percentage: f64,
    /// Total bytes to process
    pub total_bytes: u64,
    /// Bytes processed so far
    pub processed_bytes: u64,
    /// Current task index (0-based)
    pub current_task_index: usize,
    /// Total number of tasks
    pub total_tasks: usize,
    /// Display name of current task
    pub current_task_name: String,
    /// Current file being processed
    pub current_file: Option<String>,
    /// Current phase
    pub phase: InstallPhase,
    /// Verification progress (0.0 - 100.0), only used during Verifying phase
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_progress: Option<f64>,
}

/// Installation phase
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallPhase {
    /// Calculating total size
    Calculating,
    /// Installing files
    Installing,
    /// Verifying installed files
    Verifying,
    /// Finalizing
    Finalizing,
}

/// Result of a single task installation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
    pub task_id: String,
    pub task_name: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Verification statistics (if verification was performed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_stats: Option<VerificationStats>,
}

/// Overall installation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub task_results: Vec<TaskResult>,
}

/// Hash algorithm used for verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    Crc32,  // For ZIP and RAR
    Sha256, // For 7z (computed during extraction)
}

/// File hash information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHash {
    /// Relative path within the addon (e.g., "A330/A330.acf")
    pub path: String,
    /// Hash value as hex string
    pub hash: String,
    /// Algorithm used
    pub algorithm: HashAlgorithm,
}

/// Verification result for a single file
#[derive(Debug, Clone)]
pub struct FileVerificationResult {
    pub path: String,
    pub expected_hash: String,
    pub actual_hash: Option<String>,
    pub success: bool,
    pub retry_count: u8,
    pub error: Option<String>,
}

/// Verification statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationStats {
    pub total_files: usize,
    pub verified_files: usize,
    pub failed_files: usize,
    pub retried_files: usize,
    pub skipped_files: usize,
}

/// Default value for enable_verification
fn default_true() -> bool {
    true
}

// ========== Scenery Auto-Sorting Data Structures ==========

/// Scenery classification categories for sorting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum SceneryCategory {
    /// Fixed high priority scenery (e.g., SAM libraries)
    FixedHighPriority,
    /// Airport scenery with apt.dat
    Airport,
    /// Default X-Plane airports (*GLOBAL_AIRPORTS*)
    DefaultAirport,
    /// Library scenery (library.txt without Earth nav data)
    Library,
    /// Overlay scenery (modifies default terrain/objects)
    Overlay,
    /// Airport-associated mesh scenery (small mesh that matches airport coordinates)
    AirportMesh,
    /// Mesh scenery (terrain replacement, including orthophotos)
    Mesh,
    /// Other/unknown scenery
    Other,
}

impl SceneryCategory {
    /// Get sorting priority (lower = higher priority)
    pub fn priority(&self) -> u8 {
        match self {
            SceneryCategory::FixedHighPriority => 0,
            SceneryCategory::Airport => 1,
            SceneryCategory::DefaultAirport => 2,
            SceneryCategory::Library => 3,
            SceneryCategory::Other => 4,
            SceneryCategory::Overlay => 5,
            SceneryCategory::AirportMesh => 6, // Between Overlay and regular Mesh
            SceneryCategory::Mesh => 7,
        }
    }
}

/// Information about a classified scenery package
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneryPackageInfo {
    pub folder_name: String,
    pub category: SceneryCategory,
    /// Sub-priority within the same category (0 = highest)
    /// Used for special cases like XPME scenery
    pub sub_priority: u8,
    #[serde(with = "systemtime_serde")]
    pub last_modified: SystemTime,
    pub has_apt_dat: bool,
    pub has_dsf: bool,
    pub has_library_txt: bool,
    pub has_textures: bool,
    pub has_objects: bool,
    pub texture_count: usize,
    /// Number of 10-degree tile folders under Earth nav data (e.g., "+30+110")
    #[serde(default)]
    pub earth_nav_tile_count: u32,
    #[serde(with = "systemtime_serde")]
    pub indexed_at: SystemTime,
    pub required_libraries: Vec<String>,
    pub missing_libraries: Vec<String>,
    /// Library names exported by this package (from library.txt EXPORT lines)
    /// Only populated for Library category scenery packages
    #[serde(default)]
    pub exported_library_names: Vec<String>,
    /// Whether this scenery package is enabled in scenery_packs.ini
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Sort order in scenery_packs.ini (lower = higher priority)
    #[serde(default)]
    pub sort_order: u32,
    /// Actual path for shortcuts/symlinks - if set, this path should be written to scenery_packs.ini
    /// instead of "Custom Scenery/{folder_name}/". Contains the resolved target path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_path: Option<String>,
}

/// DSF file header information
#[derive(Debug, Clone)]
pub struct DsfHeader {
    pub is_overlay: bool,
    pub creation_agent: Option<String>,
    pub object_references: Vec<String>,
    pub terrain_references: Vec<String>,
}

/// Entry in scenery_packs.ini
#[derive(Debug, Clone)]
pub struct SceneryPackEntry {
    /// true = SCENERY_PACK, false = SCENERY_PACK_DISABLED
    pub enabled: bool,
    /// Path relative to X-Plane root (e.g., "Custom Scenery/folder1/")
    pub path: String,
    /// Special marker for *GLOBAL_AIRPORTS*
    pub is_global_airports: bool,
}

/// Persistent index of scenery classifications
#[derive(Debug, Serialize, Deserialize)]
pub struct SceneryIndex {
    pub version: u32,
    pub packages: HashMap<String, SceneryPackageInfo>,
    #[serde(with = "systemtime_serde")]
    pub last_updated: SystemTime,
}

/// Statistics about scenery index
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneryIndexStats {
    pub total_packages: usize,
    pub by_category: HashMap<String, usize>,
    #[serde(with = "systemtime_serde")]
    pub last_updated: SystemTime,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneryIndexStatus {
    pub index_exists: bool,
    pub total_packages: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneryIndexScanResult {
    pub index_exists: bool,
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
}

/// Entry for scenery manager UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneryManagerEntry {
    pub folder_name: String,
    pub category: SceneryCategory,
    pub sub_priority: u8,
    pub enabled: bool,
    pub sort_order: u32,
    pub missing_libraries: Vec<String>,
    pub required_libraries: Vec<String>,
}

/// Simplified entry for batch updates (only fields that can be changed)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneryEntryUpdate {
    pub folder_name: String,
    pub enabled: bool,
    pub sort_order: u32,
}

/// Data for scenery manager UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneryManagerData {
    pub entries: Vec<SceneryManagerEntry>,
    pub total_count: usize,
    pub enabled_count: usize,
    pub missing_deps_count: usize,
    /// Whether the index differs from the ini file and needs to be synced
    pub needs_sync: bool,
}

// ========== Management Data Structures ==========

/// Aircraft information for management UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AircraftInfo {
    pub folder_name: String,
    pub display_name: String,
    pub acf_file: String,
    pub enabled: bool,
    pub has_liveries: bool,
    pub livery_count: usize,
    pub version: Option<String>,
    /// URL for checking updates (from skunkcrafts_updater.cfg module| field)
    pub update_url: Option<String>,
    /// Latest version from remote server (populated by check_aircraft_updates)
    pub latest_version: Option<String>,
    /// Whether an update is available
    pub has_update: bool,
    /// Whether disabled in skunkcrafts_updater.cfg (disabled|true)
    pub cfg_disabled: Option<bool>,
}

/// Plugin information for management UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub folder_name: String,
    pub display_name: String,
    pub xpl_files: Vec<String>,
    pub enabled: bool,
    pub platform: String,
    pub version: Option<String>,
    /// URL for checking updates (from skunkcrafts_updater.cfg module| field)
    pub update_url: Option<String>,
    /// Latest version from remote server (populated by check_plugins_updates)
    pub latest_version: Option<String>,
    /// Whether an update is available
    pub has_update: bool,
    /// Whether disabled in skunkcrafts_updater.cfg (disabled|true)
    pub cfg_disabled: Option<bool>,
}

/// Navdata manager information for management UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavdataManagerInfo {
    pub folder_name: String,
    pub provider_name: String,
    pub cycle: Option<String>,
    pub airac: Option<String>,
    pub enabled: bool,
}

/// Management data for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagementData<T> {
    pub entries: Vec<T>,
    pub total_count: usize,
    pub enabled_count: usize,
}

// SystemTime serialization helper
mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addon_type_serialization() {
        let aircraft = AddonType::Aircraft;
        let json = serde_json::to_string(&aircraft).unwrap();
        assert_eq!(json, r#""Aircraft""#);

        let scenery = AddonType::Scenery;
        let json = serde_json::to_string(&scenery).unwrap();
        assert_eq!(json, r#""Scenery""#);

        let livery = AddonType::Livery;
        let json = serde_json::to_string(&livery).unwrap();
        assert_eq!(json, r#""Livery""#);
    }

    #[test]
    fn test_addon_type_deserialization() {
        let aircraft: AddonType = serde_json::from_str(r#""Aircraft""#).unwrap();
        assert_eq!(aircraft, AddonType::Aircraft);

        let scenery_lib: AddonType = serde_json::from_str(r#""SceneryLibrary""#).unwrap();
        assert_eq!(scenery_lib, AddonType::SceneryLibrary);
    }

    #[test]
    fn test_scenery_category_priority_ordering() {
        // FixedHighPriority should have lowest number (highest priority)
        assert!(SceneryCategory::FixedHighPriority.priority() < SceneryCategory::Airport.priority());
        assert!(SceneryCategory::Airport.priority() < SceneryCategory::DefaultAirport.priority());
        assert!(SceneryCategory::DefaultAirport.priority() < SceneryCategory::Library.priority());
        assert!(SceneryCategory::Library.priority() < SceneryCategory::Other.priority());
        assert!(SceneryCategory::Other.priority() < SceneryCategory::Overlay.priority());
        assert!(SceneryCategory::Overlay.priority() < SceneryCategory::AirportMesh.priority());
        assert!(SceneryCategory::AirportMesh.priority() < SceneryCategory::Mesh.priority());
    }

    #[test]
    fn test_scenery_category_serialization() {
        let airport = SceneryCategory::Airport;
        let json = serde_json::to_string(&airport).unwrap();
        assert_eq!(json, r#""Airport""#);

        let mesh = SceneryCategory::Mesh;
        let json = serde_json::to_string(&mesh).unwrap();
        assert_eq!(json, r#""Mesh""#);
    }

    #[test]
    fn test_hash_algorithm_serialization() {
        let crc32 = HashAlgorithm::Crc32;
        let json = serde_json::to_string(&crc32).unwrap();
        assert_eq!(json, r#""crc32""#);

        let sha256 = HashAlgorithm::Sha256;
        let json = serde_json::to_string(&sha256).unwrap();
        assert_eq!(json, r#""sha256""#);
    }

    #[test]
    fn test_hash_algorithm_equality() {
        assert_eq!(HashAlgorithm::Crc32, HashAlgorithm::Crc32);
        assert_eq!(HashAlgorithm::Sha256, HashAlgorithm::Sha256);
        assert_ne!(HashAlgorithm::Crc32, HashAlgorithm::Sha256);
    }

    #[test]
    fn test_install_phase_serialization() {
        let phase = InstallPhase::Installing;
        let json = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, r#""installing""#);

        let verifying = InstallPhase::Verifying;
        let json = serde_json::to_string(&verifying).unwrap();
        assert_eq!(json, r#""verifying""#);
    }

    #[test]
    fn test_install_result_counters() {
        let result = InstallResult {
            total_tasks: 5,
            successful_tasks: 3,
            failed_tasks: 2,
            task_results: vec![],
        };

        assert_eq!(result.total_tasks, result.successful_tasks + result.failed_tasks);
    }

    #[test]
    fn test_task_result_success() {
        let success_result = TaskResult {
            task_id: "task-1".to_string(),
            task_name: "Test Aircraft".to_string(),
            success: true,
            error_message: None,
            verification_stats: None,
        };
        assert!(success_result.success);
        assert!(success_result.error_message.is_none());
    }

    #[test]
    fn test_task_result_failure() {
        let fail_result = TaskResult {
            task_id: "task-2".to_string(),
            task_name: "Test Scenery".to_string(),
            success: false,
            error_message: Some("Permission denied".to_string()),
            verification_stats: None,
        };
        assert!(!fail_result.success);
        assert!(fail_result.error_message.is_some());
    }

    #[test]
    fn test_verification_stats() {
        let stats = VerificationStats {
            total_files: 100,
            verified_files: 95,
            failed_files: 3,
            retried_files: 5,
            skipped_files: 2,
        };

        // Verified + Failed + Skipped should account for total
        assert_eq!(
            stats.verified_files + stats.failed_files + stats.skipped_files,
            stats.total_files
        );
    }

    #[test]
    fn test_file_hash_structure() {
        let hash = FileHash {
            path: "aircraft/A330/A330.acf".to_string(),
            hash: "abc123def456".to_string(),
            algorithm: HashAlgorithm::Crc32,
        };

        let json = serde_json::to_string(&hash).unwrap();
        assert!(json.contains("A330.acf"));
        assert!(json.contains("abc123def456"));
        assert!(json.contains("crc32"));
    }

    #[test]
    fn test_analysis_result_defaults() {
        let json = r#"{"tasks":[],"errors":[]}"#;
        let result: AnalysisResult = serde_json::from_str(json).unwrap();

        assert!(result.tasks.is_empty());
        assert!(result.errors.is_empty());
        assert!(result.password_required.is_empty()); // Default
        assert!(result.nested_password_required.is_empty()); // Default
    }

    #[test]
    fn test_navdata_info() {
        let navdata = NavdataInfo {
            name: "Navigraph".to_string(),
            cycle: Some("2401".to_string()),
            airac: Some("2401".to_string()),
        };

        let json = serde_json::to_string(&navdata).unwrap();
        assert!(json.contains("Navigraph"));
        assert!(json.contains("2401"));
    }

    #[test]
    fn test_default_true_helper() {
        assert!(default_true());
    }

    #[test]
    fn test_scenery_manager_entry_serialization() {
        let entry = SceneryManagerEntry {
            folder_name: "MyAirport".to_string(),
            category: SceneryCategory::Airport,
            sub_priority: 0,
            enabled: true,
            sort_order: 10,
            missing_libraries: vec![],
            required_libraries: vec!["opensceneryx".to_string()],
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("MyAirport"));
        assert!(json.contains("Airport"));
        assert!(json.contains("opensceneryx"));
    }

    #[test]
    fn test_management_data_structure() {
        let data = ManagementData {
            entries: vec![
                AircraftInfo {
                    folder_name: "A320".to_string(),
                    display_name: "Airbus A320".to_string(),
                    acf_file: "A320.acf".to_string(),
                    enabled: true,
                    has_liveries: true,
                    livery_count: 5,
                    version: Some("1.0".to_string()),
                    update_url: None,
                    latest_version: None,
                    has_update: false,
                    cfg_disabled: None,
                },
            ],
            total_count: 1,
            enabled_count: 1,
        };

        assert_eq!(data.entries.len(), data.total_count);
        assert!(data.enabled_count <= data.total_count);
    }

    #[test]
    fn test_scenery_entry_update() {
        let update = SceneryEntryUpdate {
            folder_name: "test_scenery".to_string(),
            enabled: false,
            sort_order: 42,
        };

        let json = serde_json::to_string(&update).unwrap();
        let parsed: SceneryEntryUpdate = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.folder_name, "test_scenery");
        assert!(!parsed.enabled);
        assert_eq!(parsed.sort_order, 42);
    }
}

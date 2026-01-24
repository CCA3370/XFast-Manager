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
    /// Orthophoto scenery
    Orthophotos,
    /// Mesh scenery (terrain replacement)
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
            SceneryCategory::Orthophotos => 6, // Mesh sub-category
            SceneryCategory::Mesh => 6, // Same as Orthophotos, use sub-priority to distinguish
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

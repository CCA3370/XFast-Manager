use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub display_name: String,
    /// For archives: the root folder path inside the archive
    pub archive_internal_root: Option<String>,
    /// For nested archives: extraction chain (takes precedence over archive_internal_root)
    pub extraction_chain: Option<ExtractionChain>,
    /// For Navdata: cycle info from the new navdata to be installed
    pub navdata_info: Option<NavdataInfo>,
}

/// Installation progress event sent to frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    /// Overall progress percentage (0.0 - 100.0)
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
    Crc32,   // For ZIP and RAR
    Sha256,  // For 7z (computed during extraction)
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

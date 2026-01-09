use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use rayon::prelude::*;

use crate::logger;
use crate::logger::{tr, LogMsg};
use crate::models::{AddonType, AnalysisResult, DetectedItem, InstallTask};
use crate::scanner::{Scanner, PasswordRequiredError};
use crate::installer::{MAX_EXTRACTION_SIZE, MAX_COMPRESSION_RATIO};

pub struct Analyzer {
    scanner: Scanner,
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            scanner: Scanner::new(),
        }
    }

    /// Analyze a list of paths and return installation tasks
    pub fn analyze(
        &self,
        paths: Vec<String>,
        xplane_path: &str,
        passwords: Option<HashMap<String, String>>,
    ) -> AnalysisResult {
        logger::log_info(
            &format!("{}: {} path(s)", tr(LogMsg::AnalysisStarted), paths.len()),
            Some("analyzer"),
        );

        let passwords_ref = passwords.as_ref();

        // Parallel scan all paths using rayon for better performance
        let results: Vec<_> = paths
            .par_iter()
            .map(|path_str| {
                let path = Path::new(path_str);
                let password = passwords_ref.and_then(|p| p.get(path_str).map(|s| s.as_str()));
                (path_str.clone(), self.scanner.scan_path(path, password), password.map(|s| s.to_string()))
            })
            .collect();

        // Merge results
        let mut all_detected = Vec::new();
        let mut errors = Vec::new();
        let mut password_required = Vec::new();
        // Track which archives have passwords for setting on tasks later
        let mut archive_passwords: HashMap<String, String> = HashMap::new();

        for (path_str, result, password) in results {
            match result {
                Ok(detected) => {
                    // Store password for this archive if provided
                    if let Some(pwd) = password {
                        archive_passwords.insert(path_str.clone(), pwd);
                    }
                    all_detected.extend(detected);
                }
                Err(e) => {
                    // Check if this is a password-required error
                    if let Some(pwd_err) = e.downcast_ref::<PasswordRequiredError>() {
                        logger::log_info(
                            &format!("Password required for: {}", pwd_err.archive_path),
                            Some("analyzer"),
                        );
                        password_required.push(pwd_err.archive_path.clone());
                    } else {
                        let error_msg = format!("{} {}: {}", tr(LogMsg::ScanFailed), path_str, e);
                        logger::log_error(&error_msg, Some("analyzer"));
                        errors.push(error_msg);
                    }
                }
            }
        }

        // Deduplicate detected items by source path hierarchy
        let deduplicated = self.deduplicate(all_detected);

        // Convert to install tasks, passing archive passwords
        let tasks: Vec<InstallTask> = deduplicated
            .into_iter()
            .map(|item| self.create_install_task(item, xplane_path, &archive_passwords))
            .collect();

        // Deduplicate tasks by target path (e.g., multiple .acf files in same aircraft folder)
        let tasks = self.deduplicate_by_target_path(tasks);

        logger::log_info(
            &format!("{}: {} task(s)", tr(LogMsg::AnalysisCompleted), tasks.len()),
            Some("analyzer"),
        );

        AnalysisResult { tasks, errors, password_required }
    }

    /// Deduplicate install tasks based on target_path
    /// Multiple items with the same target path are merged into one task
    fn deduplicate_by_target_path(&self, tasks: Vec<InstallTask>) -> Vec<InstallTask> {
        use std::collections::HashMap;

        let mut seen: HashMap<String, InstallTask> = HashMap::new();

        for task in tasks {
            // Use target_path as the key for deduplication
            if !seen.contains_key(&task.target_path) {
                seen.insert(task.target_path.clone(), task);
            }
            // If already exists, skip (keep the first one)
        }

        seen.into_values().collect()
    }

    /// Deduplicate detected items based on path hierarchy
    /// Different addon types are deduplicated separately to allow multiple types from one archive
    fn deduplicate(&self, items: Vec<DetectedItem>) -> Vec<DetectedItem> {
        use std::collections::HashMap;

        // Group items by addon type first
        let mut by_type: HashMap<AddonType, Vec<DetectedItem>> = HashMap::new();
        for item in items {
            by_type.entry(item.addon_type.clone()).or_default().push(item);
        }

        let mut result: Vec<DetectedItem> = Vec::new();

        // Deduplicate within each type
        for (_addon_type, type_items) in by_type {
            let deduped = self.deduplicate_same_type(type_items);
            result.extend(deduped);
        }

        result
    }

    /// Deduplicate items of the same type based on path hierarchy
    fn deduplicate_same_type(&self, items: Vec<DetectedItem>) -> Vec<DetectedItem> {
        let mut result: Vec<DetectedItem> = Vec::new();

        for item in items {
            // For archives, use archive_internal_root as the effective path for deduplication
            // For directories, use the actual path
            let item_effective_path = self.get_effective_path(&item);
            let mut should_add = true;

            // Check if this item is a subdirectory of any existing item
            for existing in &result {
                let existing_effective_path = self.get_effective_path(existing);

                // Only compare items from the same source (same archive or same root)
                if !self.same_source(&item, existing) {
                    continue;
                }

                // If item is under existing path, skip it
                if item_effective_path.starts_with(&existing_effective_path)
                   && item_effective_path != existing_effective_path {
                    should_add = false;
                    break;
                }
            }

            if should_add {
                // Remove any items that are subdirectories of this item
                result.retain(|existing| {
                    if !self.same_source(&item, existing) {
                        return true; // Keep items from different sources
                    }

                    let existing_effective_path = self.get_effective_path(existing);
                    let item_effective_path = self.get_effective_path(&item);

                    !existing_effective_path.starts_with(&item_effective_path)
                        || existing_effective_path == item_effective_path
                });

                // Check for exact duplicates
                let dominated = result.iter().any(|e| {
                    self.same_source(&item, e) && self.get_effective_path(e) == item_effective_path
                });

                if !dominated {
                    result.push(item);
                }
            }
        }

        result
    }

    /// Get the effective path for deduplication
    /// For archives, this is the internal root; for directories, it's the actual path
    fn get_effective_path(&self, item: &DetectedItem) -> PathBuf {
        if let Some(ref internal_root) = item.archive_internal_root {
            PathBuf::from(internal_root)
        } else {
            PathBuf::from(&item.path)
        }
    }

    /// Check if two items come from the same source (same archive or same root directory)
    fn same_source(&self, a: &DetectedItem, b: &DetectedItem) -> bool {
        // For archives (both have archive_internal_root or same archive path)
        if a.archive_internal_root.is_some() || b.archive_internal_root.is_some() {
            // They're from the same archive if their paths (archive paths) are the same
            a.path == b.path
        } else {
            // For directories, check if they share a common root
            let a_path = PathBuf::from(&a.path);
            let b_path = PathBuf::from(&b.path);
            a_path.starts_with(&b_path) || b_path.starts_with(&a_path)
        }
    }

    /// Create an install task from a detected item
    fn create_install_task(
        &self,
        item: DetectedItem,
        xplane_path: &str,
        archive_passwords: &HashMap<String, String>,
    ) -> InstallTask {
        let xplane_root = Path::new(xplane_path);

        let target_base = match item.addon_type {
            AddonType::Aircraft => xplane_root.join("Aircraft"),
            AddonType::Scenery | AddonType::SceneryLibrary => xplane_root.join("Custom Scenery"),
            AddonType::Plugin => xplane_root.join("Resources").join("plugins"),
            AddonType::Navdata => {
                // Determine if it's GNS430 or main Custom Data
                if item.display_name.contains("GNS430") {
                    xplane_root.join("Custom Data").join("GNS430")
                } else {
                    xplane_root.join("Custom Data")
                }
            }
        };

        // Use display_name as the target folder name
        let target_path = target_base.join(&item.display_name);

        // Check if target already exists
        let conflict_exists = target_path.exists();

        // Get password for this archive if it was provided
        let password = archive_passwords.get(&item.path).cloned();

        // Estimate size and check for warnings (for archives)
        let (estimated_size, size_warning) = self.estimate_archive_size(&item.path);

        InstallTask {
            id: Uuid::new_v4().to_string(),
            addon_type: item.addon_type,
            source_path: item.path,
            target_path: target_path.to_string_lossy().to_string(),
            display_name: item.display_name,
            conflict_exists: if conflict_exists { Some(true) } else { None },
            archive_internal_root: item.archive_internal_root,
            should_overwrite: false, // Default to false, controlled by frontend
            password,
            estimated_size,
            size_warning,
            size_confirmed: false, // User must confirm if there's a warning
        }
    }

    /// Estimate the uncompressed size of an archive and check for warnings
    fn estimate_archive_size(&self, path: &str) -> (Option<u64>, Option<String>) {
        let source_path = Path::new(path);

        // Only estimate for archive files
        if !source_path.is_file() {
            return (None, None);
        }

        let extension = source_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "zip" => self.estimate_zip_size(source_path),
            "7z" => self.estimate_7z_size(source_path),
            "rar" => self.estimate_rar_size(source_path),
            _ => (None, None),
        }
    }

    /// Estimate ZIP file uncompressed size
    fn estimate_zip_size(&self, archive_path: &Path) -> (Option<u64>, Option<String>) {
        use zip::ZipArchive;

        let file = match fs::File::open(archive_path) {
            Ok(f) => f,
            Err(_) => return (None, None),
        };

        let archive_size = match file.metadata() {
            Ok(m) => m.len(),
            Err(_) => return (None, None),
        };

        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(_) => return (None, None),
        };

        // Calculate total uncompressed size
        let mut total_uncompressed: u64 = 0;
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index_raw(i) {
                total_uncompressed = total_uncompressed.saturating_add(file.size());
            }
        }

        self.check_size_warning(archive_size, total_uncompressed)
    }

    /// Estimate 7z file uncompressed size (approximate)
    fn estimate_7z_size(&self, archive_path: &Path) -> (Option<u64>, Option<String>) {
        let archive_size = match fs::metadata(archive_path) {
            Ok(m) => m.len(),
            Err(_) => return (None, None),
        };

        // For 7z, we estimate based on typical compression ratios (3-5x)
        // This is a rough estimate since 7z doesn't store uncompressed size in headers easily
        let estimated_uncompressed = archive_size.saturating_mul(4);

        self.check_size_warning(archive_size, estimated_uncompressed)
    }

    /// Estimate RAR file uncompressed size (approximate)
    fn estimate_rar_size(&self, archive_path: &Path) -> (Option<u64>, Option<String>) {
        let archive_size = match fs::metadata(archive_path) {
            Ok(m) => m.len(),
            Err(_) => return (None, None),
        };

        // For RAR, estimate based on typical compression ratios
        let estimated_uncompressed = archive_size.saturating_mul(4);

        self.check_size_warning(archive_size, estimated_uncompressed)
    }

    /// Check if the archive size warrants a warning
    fn check_size_warning(&self, archive_size: u64, estimated_uncompressed: u64) -> (Option<u64>, Option<String>) {
        let mut warning = None;

        // Check compression ratio (potential zip bomb)
        if archive_size > 0 {
            let ratio = estimated_uncompressed / archive_size;
            if ratio > MAX_COMPRESSION_RATIO {
                warning = Some(format!(
                    "SUSPICIOUS_RATIO:{}:{}",
                    ratio,
                    estimated_uncompressed
                ));
            }
        }

        // Check absolute size
        if estimated_uncompressed > MAX_EXTRACTION_SIZE {
            let size_gb = estimated_uncompressed as f64 / 1024.0 / 1024.0 / 1024.0;
            warning = Some(format!(
                "LARGE_SIZE:{:.2}",
                size_gb
            ));
        }

        (Some(estimated_uncompressed), warning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication_same_type() {
        let analyzer = Analyzer::new();

        // Same type (Aircraft), nested paths - should deduplicate
        let items = vec![
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/A330".to_string(),
                display_name: "A330".to_string(),
                archive_internal_root: None,
            },
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/A330/variant".to_string(),
                display_name: "A330 Variant".to_string(),
                archive_internal_root: None,
            },
        ];

        let result = analyzer.deduplicate(items);

        // Should only have the parent Aircraft
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].display_name, "A330");
    }

    #[test]
    fn test_deduplication_different_types() {
        let analyzer = Analyzer::new();

        // Different types from same directory tree - should keep both
        let items = vec![
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/A330".to_string(),
                display_name: "A330".to_string(),
                archive_internal_root: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/A330/plugins/fms".to_string(),
                display_name: "FMS".to_string(),
                archive_internal_root: None,
            },
        ];

        let result = analyzer.deduplicate(items);

        // Should have both because they are different types
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_deduplication_archive_multi_type() {
        let analyzer = Analyzer::new();

        // Multiple types from same archive - should keep all
        let items = vec![
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/package.zip".to_string(),
                display_name: "A330".to_string(),
                archive_internal_root: Some("A330".to_string()),
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/package.zip".to_string(),
                display_name: "FMS Plugin".to_string(),
                archive_internal_root: Some("plugins/fms".to_string()),
            },
            DetectedItem {
                addon_type: AddonType::Scenery,
                path: "/test/package.zip".to_string(),
                display_name: "Airport".to_string(),
                archive_internal_root: Some("scenery/airport".to_string()),
            },
            DetectedItem {
                addon_type: AddonType::SceneryLibrary,
                path: "/test/package.zip".to_string(),
                display_name: "Library".to_string(),
                archive_internal_root: Some("library".to_string()),
            },
        ];

        let result = analyzer.deduplicate(items);

        // Should have all four because they are different types
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_deduplication_archive_same_type_nested() {
        let analyzer = Analyzer::new();

        // Same type, nested paths in archive - should deduplicate
        let items = vec![
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/package.zip".to_string(),
                display_name: "MainPlugin".to_string(),
                archive_internal_root: Some("plugins".to_string()),
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/package.zip".to_string(),
                display_name: "SubPlugin".to_string(),
                archive_internal_root: Some("plugins/sub".to_string()),
            },
        ];

        let result = analyzer.deduplicate(items);

        // Should only have the parent plugin
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].display_name, "MainPlugin");
    }

    #[test]
    fn test_deduplicate_by_target_path() {
        let analyzer = Analyzer::new();

        // Multiple .acf files in same aircraft folder -> same target path
        let tasks = vec![
            InstallTask {
                id: "1".to_string(),
                addon_type: AddonType::Aircraft,
                source_path: "/downloads/A330/A330.acf".to_string(),
                target_path: "/X-Plane/Aircraft/A330".to_string(),
                display_name: "A330".to_string(),
                conflict_exists: None,
                archive_internal_root: None,
                should_overwrite: false,
                password: None,
                estimated_size: None,
                size_warning: None,
                size_confirmed: false,
            },
            InstallTask {
                id: "2".to_string(),
                addon_type: AddonType::Aircraft,
                source_path: "/downloads/A330/A330_cargo.acf".to_string(),
                target_path: "/X-Plane/Aircraft/A330".to_string(),
                display_name: "A330".to_string(),
                conflict_exists: None,
                archive_internal_root: None,
                should_overwrite: false,
                password: None,
                estimated_size: None,
                size_warning: None,
                size_confirmed: false,
            },
        ];

        let result = analyzer.deduplicate_by_target_path(tasks);

        // Should only have one task since target paths are the same
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_deduplicate_by_target_path_different_targets() {
        let analyzer = Analyzer::new();

        // Different target paths should be kept
        let tasks = vec![
            InstallTask {
                id: "1".to_string(),
                addon_type: AddonType::Aircraft,
                source_path: "/downloads/A330".to_string(),
                target_path: "/X-Plane/Aircraft/A330".to_string(),
                display_name: "A330".to_string(),
                conflict_exists: None,
                archive_internal_root: None,
                should_overwrite: false,
                password: None,
                estimated_size: None,
                size_warning: None,
                size_confirmed: false,
            },
            InstallTask {
                id: "2".to_string(),
                addon_type: AddonType::Aircraft,
                source_path: "/downloads/B737".to_string(),
                target_path: "/X-Plane/Aircraft/B737".to_string(),
                display_name: "B737".to_string(),
                conflict_exists: None,
                archive_internal_root: None,
                should_overwrite: false,
                password: None,
                estimated_size: None,
                size_warning: None,
                size_confirmed: false,
            },
        ];

        let result = analyzer.deduplicate_by_target_path(tasks);

        // Should have both tasks since target paths are different
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_deduplicate_multiple_xpl_same_plugin() {
        let analyzer = Analyzer::new();

        // Multiple .xpl files in same plugin folder -> same target path
        let tasks = vec![
            InstallTask {
                id: "1".to_string(),
                addon_type: AddonType::Plugin,
                source_path: "/downloads/MyPlugin/win.xpl".to_string(),
                target_path: "/X-Plane/Resources/plugins/MyPlugin".to_string(),
                display_name: "MyPlugin".to_string(),
                conflict_exists: None,
                archive_internal_root: None,
                should_overwrite: false,
                password: None,
                estimated_size: None,
                size_warning: None,
                size_confirmed: false,
            },
            InstallTask {
                id: "2".to_string(),
                addon_type: AddonType::Plugin,
                source_path: "/downloads/MyPlugin/mac.xpl".to_string(),
                target_path: "/X-Plane/Resources/plugins/MyPlugin".to_string(),
                display_name: "MyPlugin".to_string(),
                conflict_exists: None,
                archive_internal_root: None,
                should_overwrite: false,
                password: None,
                estimated_size: None,
                size_warning: None,
                size_confirmed: false,
            },
            InstallTask {
                id: "3".to_string(),
                addon_type: AddonType::Plugin,
                source_path: "/downloads/MyPlugin/lin.xpl".to_string(),
                target_path: "/X-Plane/Resources/plugins/MyPlugin".to_string(),
                display_name: "MyPlugin".to_string(),
                conflict_exists: None,
                archive_internal_root: None,
                should_overwrite: false,
                password: None,
                estimated_size: None,
                size_warning: None,
                size_confirmed: false,
            },
        ];

        let result = analyzer.deduplicate_by_target_path(tasks);

        // Should only have one task since all target paths are the same
        assert_eq!(result.len(), 1);
    }
}

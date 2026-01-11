use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use rayon::prelude::*;

use crate::logger;
use crate::logger::{tr, LogMsg};
use crate::models::{AddonType, AnalysisResult, DetectedItem, InstallTask, NavdataCycle, NavdataInfo};
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

        crate::log_debug!(&format!("Analyzing paths: {:?}", paths), "analysis");
        crate::log_debug!(&format!("X-Plane path: {}", xplane_path), "analysis");

        let passwords_ref = passwords.as_ref();
        let xplane_root = Path::new(xplane_path);

        // Parallel scan all paths using rayon for better performance
        let results: Vec<_> = paths
            .par_iter()
            .map(|path_str| {
                let path = Path::new(path_str);

                // Check if the path is a directory inside X-Plane's installation target directories
                // This prevents users from accidentally dragging existing addon folders
                if path.is_dir() && path.starts_with(xplane_root) {
                    // Check if it's in one of the target directories
                    let is_in_target_dir = [
                        "Aircraft",
                        "Custom Scenery",
                        "Custom Data",
                    ].iter().any(|target| {
                        let target_path = xplane_root.join(target);
                        path.starts_with(&target_path)
                    }) || {
                        // Special check for Resources/plugins
                        let plugins_path = xplane_root.join("Resources").join("plugins");
                        path.starts_with(&plugins_path)
                    };

                    if is_in_target_dir {
                        let error_msg = tr(LogMsg::CannotInstallFromXPlane);
                        return (path_str.clone(), Err(anyhow::anyhow!(error_msg)), None);
                    }
                }

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
                        // Format error message for better readability
                        let error_msg = format!("{}\n  {}\n  {}",
                            tr(LogMsg::ScanFailed),
                            path_str,
                            e
                        );
                        logger::log_error(&error_msg, Some("analyzer"));

                        // For frontend display, use a cleaner format
                        let display_msg = format!("{}: {}", tr(LogMsg::ScanFailed), e);
                        errors.push(display_msg);
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

        crate::log_debug!(&format!("Analysis returned {} tasks, {} errors", tasks.len(), errors.len()), "analysis");
        if !tasks.is_empty() {
            let task_types: Vec<String> = tasks.iter().map(|t| format!("{:?}", t.addon_type)).collect();
            crate::log_debug!(&format!("Task types: {}", task_types.join(", ")), "analysis");
        }

        AnalysisResult {
            tasks,
            errors,
            password_required,
            nested_password_required: HashMap::new(),
        }
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

        // Apply priority filtering: remove plugins inside aircraft/scenery
        result = self.filter_by_priority(result);

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

    /// Filter items by priority: plugins inside aircraft/scenery are removed
    /// Priority: Aircraft, Scenery, SceneryLibrary, Navdata > Plugin
    fn filter_by_priority(&self, items: Vec<DetectedItem>) -> Vec<DetectedItem> {
        let high_priority_types = [
            AddonType::Aircraft,
            AddonType::Scenery,
            AddonType::SceneryLibrary,
            AddonType::Navdata,
        ];

        // Separate high-priority and low-priority items
        let (high_priority, low_priority): (Vec<_>, Vec<_>) = items
            .into_iter()
            .partition(|item| high_priority_types.contains(&item.addon_type));

        // Filter low-priority items: remove if nested inside any high-priority item
        let filtered_low_priority: Vec<DetectedItem> = low_priority
            .into_iter()
            .filter(|low_item| {
                let low_path = self.get_effective_path(low_item);

                // Check if this low-priority item is inside any high-priority item
                !high_priority.iter().any(|high_item| {
                    // Must be from the same source (same archive or same directory tree)
                    if !self.same_source(low_item, high_item) {
                        return false;
                    }

                    let high_path = self.get_effective_path(high_item);

                    // If low-priority item is under high-priority path, filter it out
                    low_path.starts_with(&high_path) && low_path != high_path
                })
            })
            .collect();

        // Merge results
        let mut result = high_priority;
        result.extend(filtered_low_priority);
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

    /// Read existing navdata cycle info from Custom Data/cycle.json
    /// Returns None if file doesn't exist or can't be read (graceful degradation)
    fn read_existing_navdata_cycle(&self, target_path: &Path) -> Option<NavdataInfo> {
        let cycle_path = target_path.join("cycle.json");

        if !cycle_path.exists() {
            return None;
        }

        // Try to read and parse, but don't fail if it doesn't work
        let content = fs::read_to_string(&cycle_path).ok()?;
        let cycle: NavdataCycle = serde_json::from_str(&content).ok()?;

        Some(NavdataInfo {
            name: cycle.name,
            cycle: cycle.cycle,
            airac: cycle.airac,
        })
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

        // For Navdata, install directly into target_base (don't create subfolder)
        // For other types, create a subfolder with the display_name
        let target_path = if item.addon_type == AddonType::Navdata {
            target_base.clone()
        } else {
            target_base.join(&item.display_name)
        };

        // Check if target already exists
        // For Navdata, check if cycle.json exists and read existing cycle info
        let (conflict_exists, existing_navdata_info) = if item.addon_type == AddonType::Navdata {
            let cycle_path = target_path.join("cycle.json");
            if cycle_path.exists() {
                (true, self.read_existing_navdata_cycle(&target_path))
            } else {
                (false, None)
            }
        } else {
            (target_path.exists(), None)
        };

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
            extraction_chain: item.extraction_chain,
            should_overwrite: false, // Default to false, controlled by frontend
            password,
            estimated_size,
            size_warning,
            size_confirmed: false, // User must confirm if there's a warning
            existing_navdata_info,
            new_navdata_info: item.navdata_info,
            backup_liveries: true, // Default to true (safe)
            backup_config_files: true, // Default to true (safe)
            config_file_patterns: vec!["*_prefs.txt".to_string()], // Default pattern
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
            Err(e) => {
                logger::log_debug(
                    &format!("Failed to open ZIP for size estimation: {}", e),
                    Some("analyzer"),
                    Some("analyzer.rs"),
                );
                // Fall back to conservative estimate
                if let Ok(meta) = fs::metadata(archive_path) {
                    return self.check_size_warning(meta.len(), meta.len().saturating_mul(5));
                }
                return (None, None);
            }
        };

        let archive_size = match file.metadata() {
            Ok(m) => m.len(),
            Err(e) => {
                logger::log_debug(
                    &format!("Failed to get ZIP metadata: {}", e),
                    Some("analyzer"),
                    Some("analyzer.rs"),
                );
                return (None, None);
            }
        };

        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                logger::log_debug(
                    &format!("Failed to read ZIP archive for size estimation: {}", e),
                    Some("analyzer"),
                    Some("analyzer.rs"),
                );
                // Fall back to conservative estimate (5x compressed size)
                return self.check_size_warning(archive_size, archive_size.saturating_mul(5));
            }
        };

        // Calculate total uncompressed size
        let mut total_uncompressed: u64 = 0;
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index_raw(i) {
                total_uncompressed = total_uncompressed.saturating_add(file.size());
            }
        }

        // If we couldn't get any size info, use conservative estimate
        if total_uncompressed == 0 && archive.len() > 0 {
            total_uncompressed = archive_size.saturating_mul(5);
        }

        self.check_size_warning(archive_size, total_uncompressed)
    }

    /// Estimate 7z file uncompressed size
    fn estimate_7z_size(&self, archive_path: &Path) -> (Option<u64>, Option<String>) {
        let archive_size = match fs::metadata(archive_path) {
            Ok(m) => m.len(),
            Err(e) => {
                logger::log_debug(
                    &format!("Failed to get 7z metadata: {}", e),
                    Some("analyzer"),
                    Some("analyzer.rs"),
                );
                return (None, None);
            }
        };

        // Check cache first
        if let Some(cached) = crate::cache::get_cached_metadata(archive_path) {
            return self.check_size_warning(archive_size, cached.uncompressed_size);
        }

        // Try to read actual uncompressed size from 7z archive
        let estimated_uncompressed = match sevenz_rust2::Archive::open(archive_path) {
            Ok(archive) => {
                // Sum up uncompressed sizes of all files
                let mut total: u64 = 0;
                let file_count = archive.files.len();
                for entry in &archive.files {
                    total = total.saturating_add(entry.size());
                }

                // Cache the result
                if total > 0 {
                    crate::cache::cache_metadata(archive_path, total, file_count);
                    total
                } else {
                    // Fallback to estimation if no size info
                    archive_size.saturating_mul(5)
                }
            }
            Err(e) => {
                logger::log_debug(
                    &format!("Failed to open 7z for size estimation: {}", e),
                    Some("analyzer"),
                    Some("analyzer.rs"),
                );
                // If we can't open the archive, use conservative estimate
                archive_size.saturating_mul(5)
            }
        };

        self.check_size_warning(archive_size, estimated_uncompressed)
    }

    /// Estimate RAR file uncompressed size
    fn estimate_rar_size(&self, archive_path: &Path) -> (Option<u64>, Option<String>) {
        let archive_size = match fs::metadata(archive_path) {
            Ok(m) => m.len(),
            Err(e) => {
                logger::log_debug(
                    &format!("Failed to get RAR metadata: {}", e),
                    Some("analyzer"),
                    Some("analyzer.rs"),
                );
                return (None, None);
            }
        };

        // Check cache first
        if let Some(cached) = crate::cache::get_cached_metadata(archive_path) {
            return self.check_size_warning(archive_size, cached.uncompressed_size);
        }

        // Try to read actual uncompressed size from RAR archive
        let estimated_uncompressed = match unrar::Archive::new(archive_path).open_for_listing() {
            Ok(archive) => {
                let mut total: u64 = 0;
                let mut file_count: usize = 0;
                for entry in archive {
                    if let Ok(e) = entry {
                        total = total.saturating_add(e.unpacked_size);
                        file_count += 1;
                    }
                }

                // Cache the result
                if total > 0 {
                    crate::cache::cache_metadata(archive_path, total, file_count);
                    total
                } else {
                    // Fallback to estimation if no size info
                    archive_size.saturating_mul(5)
                }
            }
            Err(e) => {
                logger::log_debug(
                    &format!("Failed to open RAR for size estimation: {:?}", e),
                    Some("analyzer"),
                    Some("analyzer.rs"),
                );
                // If we can't open the archive, use conservative estimate
                archive_size.saturating_mul(5)
            }
        };

        self.check_size_warning(archive_size, estimated_uncompressed)
    }

    /// Check if the archive size warrants a warning
    fn check_size_warning(&self, archive_size: u64, estimated_uncompressed: u64) -> (Option<u64>, Option<String>) {
        let mut warning = None;

        // Check compression ratio (potential zip bomb)
        // Only check if both sizes are non-zero to avoid division by zero
        if archive_size > 0 && estimated_uncompressed > 0 {
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
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/A330/variant".to_string(),
                display_name: "A330 Variant".to_string(),
                archive_internal_root: None,
                navdata_info: None,
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

        // Aircraft contains a plugin - plugin should be filtered out
        let items = vec![
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/A330".to_string(),
                display_name: "A330".to_string(),
                archive_internal_root: None,
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/A330/plugins/fms".to_string(),
                display_name: "FMS".to_string(),
                archive_internal_root: None,
                navdata_info: None,
            },
        ];

        let result = analyzer.deduplicate(items);

        // Should only have Aircraft, plugin is filtered out
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addon_type, AddonType::Aircraft);
    }

    #[test]
    fn test_deduplication_archive_multi_type() {
        let analyzer = Analyzer::new();

        // Archive with aircraft and independent plugin/scenery
        let items = vec![
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/package.zip".to_string(),
                display_name: "A330".to_string(),
                archive_internal_root: Some("A330".to_string()),
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/package.zip".to_string(),
                display_name: "FMS Plugin".to_string(),
                archive_internal_root: Some("A330/plugins/fms".to_string()),
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/package.zip".to_string(),
                display_name: "Standalone Plugin".to_string(),
                archive_internal_root: Some("plugins/standalone".to_string()),
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Scenery,
                path: "/test/package.zip".to_string(),
                display_name: "Airport".to_string(),
                archive_internal_root: Some("scenery/airport".to_string()),
                navdata_info: None,
            },
        ];

        let result = analyzer.deduplicate(items);

        // Should have: Aircraft, Standalone Plugin, Scenery (3 items)
        // FMS Plugin is filtered because it's inside Aircraft
        assert_eq!(result.len(), 3);

        let types: Vec<AddonType> = result.iter().map(|i| i.addon_type.clone()).collect();
        assert!(types.contains(&AddonType::Aircraft));
        assert!(types.contains(&AddonType::Scenery));

        // Check that standalone plugin is kept
        let plugins: Vec<_> = result.iter()
            .filter(|i| i.addon_type == AddonType::Plugin)
            .collect();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].display_name, "Standalone Plugin");
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
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/package.zip".to_string(),
                display_name: "SubPlugin".to_string(),
                archive_internal_root: Some("plugins/sub".to_string()),
                navdata_info: None,
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
                existing_navdata_info: None,
                new_navdata_info: None,
                backup_liveries: true,
                backup_config_files: true,
                config_file_patterns: vec!["*_prefs.txt".to_string()],
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
                existing_navdata_info: None,
                new_navdata_info: None,
                backup_liveries: true,
                backup_config_files: true,
                config_file_patterns: vec!["*_prefs.txt".to_string()],
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
                existing_navdata_info: None,
                new_navdata_info: None,
                backup_liveries: true,
                backup_config_files: true,
                config_file_patterns: vec!["*_prefs.txt".to_string()],
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
                existing_navdata_info: None,
                new_navdata_info: None,
                backup_liveries: true,
                backup_config_files: true,
                config_file_patterns: vec!["*_prefs.txt".to_string()],
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
                existing_navdata_info: None,
                new_navdata_info: None,
                backup_liveries: true,
                backup_config_files: true,
                config_file_patterns: vec!["*_prefs.txt".to_string()],
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
                existing_navdata_info: None,
                new_navdata_info: None,
                backup_liveries: true,
                backup_config_files: true,
                config_file_patterns: vec!["*_prefs.txt".to_string()],
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
                existing_navdata_info: None,
                new_navdata_info: None,
                backup_liveries: true,
                backup_config_files: true,
                config_file_patterns: vec!["*_prefs.txt".to_string()],
            },
        ];

        let result = analyzer.deduplicate_by_target_path(tasks);

        // Should only have one task since all target paths are the same
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_plugin_inside_scenery_filtered() {
        let analyzer = Analyzer::new();

        let items = vec![
            DetectedItem {
                addon_type: AddonType::Scenery,
                path: "/test/KSEA".to_string(),
                display_name: "KSEA".to_string(),
                archive_internal_root: None,
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/KSEA/plugins/lighting".to_string(),
                display_name: "Lighting".to_string(),
                archive_internal_root: None,
                navdata_info: None,
            },
        ];

        let result = analyzer.deduplicate(items);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addon_type, AddonType::Scenery);
    }

    #[test]
    fn test_standalone_plugin_kept() {
        let analyzer = Analyzer::new();

        let items = vec![
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/A330".to_string(),
                display_name: "A330".to_string(),
                archive_internal_root: None,
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/BetterPushback".to_string(),
                display_name: "BetterPushback".to_string(),
                archive_internal_root: None,
                navdata_info: None,
            },
        ];

        let result = analyzer.deduplicate(items);

        // Both should be kept
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_archive_complex_priority() {
        let analyzer = Analyzer::new();

        let items = vec![
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/pack.zip".to_string(),
                display_name: "A330".to_string(),
                archive_internal_root: Some("aircraft/A330".to_string()),
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/pack.zip".to_string(),
                display_name: "Systems".to_string(),
                archive_internal_root: Some("aircraft/A330/plugins/systems".to_string()),
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::SceneryLibrary,
                path: "/test/pack.zip".to_string(),
                display_name: "Library".to_string(),
                archive_internal_root: Some("library".to_string()),
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/pack.zip".to_string(),
                display_name: "LibPlugin".to_string(),
                archive_internal_root: Some("library/plugins/helper".to_string()),
                navdata_info: None,
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/pack.zip".to_string(),
                display_name: "Standalone".to_string(),
                archive_internal_root: Some("plugins/standalone".to_string()),
                navdata_info: None,
            },
        ];

        let result = analyzer.deduplicate(items);

        // Should have: Aircraft, SceneryLibrary, Standalone Plugin (3 items)
        assert_eq!(result.len(), 3);

        let plugin_names: Vec<String> = result.iter()
            .filter(|i| i.addon_type == AddonType::Plugin)
            .map(|i| i.display_name.clone())
            .collect();

        assert_eq!(plugin_names.len(), 1);
        assert_eq!(plugin_names[0], "Standalone");
    }
}

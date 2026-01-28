use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::installer::{MAX_COMPRESSION_RATIO, MAX_EXTRACTION_SIZE};
use crate::livery_patterns;
use crate::logger;
use crate::logger::{tr, LogMsg};
use crate::models::{
    AddonType, AnalysisResult, DetectedItem, InstallTask, NavdataCycle, NavdataInfo,
};
use crate::scanner::{NestedPasswordRequiredError, PasswordRequiredError, Scanner};

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
        verification_preferences: Option<HashMap<String, bool>>,
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
                    let is_in_target_dir = ["Aircraft", "Custom Scenery", "Custom Data"]
                        .iter()
                        .any(|target| {
                            let target_path = xplane_root.join(target);
                            path.starts_with(&target_path)
                        })
                        || {
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
                (
                    path_str.clone(),
                    self.scanner.scan_path(path, password),
                    password.map(|s| s.to_string()),
                )
            })
            .collect();

        // Merge results
        let mut all_detected = Vec::new();
        let mut errors = Vec::new();
        let mut password_required = Vec::new();
        let mut nested_password_required = HashMap::new(); // NEW: Track nested password requirements
                                                           // Track which archives have passwords for setting on tasks later
        let mut archive_passwords: HashMap<String, String> = HashMap::new();

        for (path_str, result, password) in results {
            match result {
                Ok(detected) => {
                    // Store password for this archive if provided
                    // Use the same key format as scanner (Path::to_string_lossy)
                    if let Some(pwd) = password {
                        // Store with original path_str
                        archive_passwords.insert(path_str.clone(), pwd.clone());
                        // Also store with Path-normalized format (same as scanner uses)
                        let normalized = Path::new(&path_str).to_string_lossy().to_string();
                        if normalized != path_str {
                            archive_passwords.insert(normalized, pwd.clone());
                        }
                        // Also store with the detected item's path format
                        for item in &detected {
                            if !archive_passwords.contains_key(&item.path) {
                                archive_passwords.insert(item.path.clone(), pwd.clone());
                            }
                        }
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
                    }
                    // NEW: Check if this is a nested password-required error
                    else if let Some(nested_err) = e.downcast_ref::<NestedPasswordRequiredError>()
                    {
                        logger::log_info(
                            &format!(
                                "Password required for nested archive: {} inside {}",
                                nested_err.nested_archive, nested_err.parent_archive
                            ),
                            Some("analyzer"),
                        );
                        let key = format!(
                            "{}/{}",
                            nested_err.parent_archive, nested_err.nested_archive
                        );
                        nested_password_required.insert(key, nested_err.parent_archive.clone());
                    } else {
                        // Format error message for better readability
                        let error_msg =
                            format!("{}\n  {}\n  {}", tr(LogMsg::ScanFailed), path_str, e);
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

        // Filter out items where the source path is a disk root directory
        let filtered: Vec<DetectedItem> = deduplicated
            .into_iter()
            .filter(|item| {
                let path = Path::new(&item.path);
                let is_root = Self::is_disk_root(path);

                if is_root {
                    logger::log_info(
                        &format!("Ignoring addon at disk root: {}", item.path),
                        Some("analyzer"),
                    );
                    errors.push(format!(
                        "Ignored addon at disk root: {} ({})",
                        item.display_name, item.path
                    ));
                }

                !is_root
            })
            .collect();

        // Convert to install tasks, passing archive passwords
        let tasks: Vec<InstallTask> = filtered
            .into_iter()
            .map(|item| {
                self.create_install_task(
                    item,
                    xplane_path,
                    &archive_passwords,
                    verification_preferences.as_ref(),
                )
            })
            .collect();

        // Deduplicate tasks by target path (e.g., multiple .acf files in same aircraft folder)
        let mut tasks = self.deduplicate_by_target_path(tasks);

        // Collect file hashes for verification
        self.collect_hashes_for_tasks(&mut tasks);

        logger::log_info(
            &format!("{}: {} task(s)", tr(LogMsg::AnalysisCompleted), tasks.len()),
            Some("analyzer"),
        );

        crate::log_debug!(
            &format!(
                "Analysis returned {} tasks, {} errors",
                tasks.len(),
                errors.len()
            ),
            "analysis"
        );
        if !tasks.is_empty() {
            let task_types: Vec<String> = tasks
                .iter()
                .map(|t| format!("{:?}", t.addon_type))
                .collect();
            crate::log_debug!(
                &format!("Task types: {}", task_types.join(", ")),
                "analysis"
            );
        }

        AnalysisResult {
            tasks,
            errors,
            password_required,
            nested_password_required,
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
            by_type
                .entry(item.addon_type.clone())
                .or_default()
                .push(item);
        }

        let mut result: Vec<DetectedItem> = Vec::new();

        // Deduplicate within each type
        for (_addon_type, type_items) in by_type {
            let deduped = self.deduplicate_same_type(type_items);
            result.extend(deduped);
        }

        // Handle Scenery vs SceneryLibrary conflict: if same path has both, keep Scenery
        result = self.resolve_scenery_conflicts(result);

        // Apply priority filtering: remove plugins inside aircraft/scenery
        result = self.filter_by_priority(result);

        result
    }

    /// Resolve conflicts between Scenery and SceneryLibrary
    /// If the same directory has both types, keep Scenery (higher priority)
    fn resolve_scenery_conflicts(&self, items: Vec<DetectedItem>) -> Vec<DetectedItem> {
        use std::collections::HashMap;

        // Group items by their effective path
        let mut by_path: HashMap<PathBuf, Vec<DetectedItem>> = HashMap::new();
        for item in items {
            let path = self.get_effective_path(&item);
            by_path.entry(path).or_default().push(item);
        }

        let mut result = Vec::new();

        for (_path, path_items) in by_path {
            // Check if this path has both Scenery and SceneryLibrary
            let has_scenery = path_items
                .iter()
                .any(|i| i.addon_type == AddonType::Scenery);
            let has_scenery_library = path_items
                .iter()
                .any(|i| i.addon_type == AddonType::SceneryLibrary);

            if has_scenery && has_scenery_library {
                // Keep only Scenery, filter out SceneryLibrary
                for item in path_items {
                    if item.addon_type != AddonType::SceneryLibrary {
                        result.push(item);
                    }
                }
            } else {
                // No conflict, keep all items
                result.extend(path_items);
            }
        }

        result
    }

    /// Deduplicate items of the same type based on path hierarchy
    /// Optimized algorithm: O(n log n) instead of O(n²)
    fn deduplicate_same_type(&self, items: Vec<DetectedItem>) -> Vec<DetectedItem> {
        if items.is_empty() {
            return Vec::new();
        }

        // Sort by path depth (shallow to deep) for efficient processing
        let mut sorted_items = items;
        sorted_items.sort_by_cached_key(|item| {
            let path = self.get_effective_path(item);
            path.components().count()
        });

        let mut result: Vec<DetectedItem> = Vec::new();

        for item in sorted_items {
            let item_path = self.get_effective_path(&item);
            let mut is_nested = false;

            // Check if this item is nested under any already-kept item from the same source
            for kept in &result {
                // Only compare items from the same source
                if !self.same_source(&item, kept) {
                    continue;
                }

                let kept_path = self.get_effective_path(kept);

                // Since items are sorted by depth, kept items are always shallower or equal depth
                // We only need to check if item is under kept
                if item_path.starts_with(&kept_path) && item_path != kept_path {
                    is_nested = true;
                    break;
                }
            }

            if !is_nested {
                result.push(item);
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

    /// Normalize a path for password lookup
    /// Converts backslashes to forward slashes and removes trailing slashes
    fn normalize_path_for_lookup(path: &str) -> String {
        let normalized = path.replace('\\', "/");
        normalized.trim_end_matches('/').to_string()
    }

    /// Find password for a given path, trying multiple key formats
    fn find_password(archive_passwords: &HashMap<String, String>, path: &str) -> Option<String> {
        // Try exact match first
        if let Some(pwd) = archive_passwords.get(path) {
            return Some(pwd.clone());
        }

        // Try normalized path
        let normalized = Self::normalize_path_for_lookup(path);
        if let Some(pwd) = archive_passwords.get(&normalized) {
            return Some(pwd.clone());
        }

        // Try with Path normalization
        let path_normalized = Path::new(path).to_string_lossy().to_string();
        if let Some(pwd) = archive_passwords.get(&path_normalized) {
            return Some(pwd.clone());
        }

        // Try filename only (for nested archives)
        if let Some(filename) = Path::new(path).file_name().and_then(|s| s.to_str()) {
            if let Some(pwd) = archive_passwords.get(filename) {
                return Some(pwd.clone());
            }
        }

        None
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
            // For directories, check if one is a direct ancestor of the other
            // Use canonical comparison to avoid issues like "A330" matching "A330_variant"
            let a_path = PathBuf::from(&a.path);
            let b_path = PathBuf::from(&b.path);

            // Check if paths are exactly equal
            if a_path == b_path {
                return true;
            }

            // Check if one is a proper subdirectory of the other
            // by verifying the parent-child relationship with path components
            if a_path.starts_with(&b_path) {
                // Verify it's a proper subdirectory (not just prefix match)
                let relative = a_path.strip_prefix(&b_path).ok();
                if let Some(rel) = relative {
                    // Must have at least one component (not empty)
                    return rel.components().count() > 0;
                }
            }
            if b_path.starts_with(&a_path) {
                let relative = b_path.strip_prefix(&a_path).ok();
                if let Some(rel) = relative {
                    return rel.components().count() > 0;
                }
            }

            false
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

    /// Find the aircraft folder that matches the given aircraft type ID for livery installation
    /// Returns (aircraft_folder_path, found) where found indicates if the aircraft was found
    fn find_aircraft_for_livery(
        &self,
        xplane_path: &str,
        aircraft_type_id: &str,
    ) -> Option<PathBuf> {
        let aircraft_dir = Path::new(xplane_path).join("Aircraft");
        if !aircraft_dir.exists() {
            return None;
        }

        // Recursively search for .acf files that match the aircraft type
        for entry in walkdir::WalkDir::new(&aircraft_dir)
            .max_depth(4) // Limit depth to avoid scanning too deep
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("acf") {
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            // Check if this ACF file matches our aircraft type
                            if let Some(matched_type) =
                                livery_patterns::check_acf_identifier(file_name)
                            {
                                if matched_type == aircraft_type_id {
                                    // Found matching aircraft, return its parent directory
                                    if let Some(parent) = path.parent() {
                                        logger::log_info(
                                            &format!(
                                                "Found aircraft for livery: {} -> {}",
                                                aircraft_type_id,
                                                parent.display()
                                            ),
                                            Some("analyzer"),
                                        );
                                        return Some(parent.to_path_buf());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        logger::log_info(
            &format!(
                "Aircraft not found for livery type: {}",
                aircraft_type_id
            ),
            Some("analyzer"),
        );
        None
    }

    /// Create an install task from a detected item
    fn create_install_task(
        &self,
        item: DetectedItem,
        xplane_path: &str,
        archive_passwords: &HashMap<String, String>,
        verification_preferences: Option<&HashMap<String, bool>>,
    ) -> InstallTask {
        let xplane_root = Path::new(xplane_path);

        // For Livery type, we need special handling to find the target aircraft
        let (target_path, livery_aircraft_found) = if item.addon_type == AddonType::Livery {
            // Extract the livery name from display_name (remove the aircraft name suffix)
            // The display_name format is: "{livery_name} ({aircraft_name})"
            // We need to find the LAST " (" to handle livery names that contain parentheses
            // e.g., "Qantas (XLR - PW - VH-OGA) 8K (ToLiss A321)" -> "Qantas (XLR - PW - VH-OGA) 8K"
            let livery_name = if let Some(last_paren_pos) = item.display_name.rfind(" (") {
                item.display_name[..last_paren_pos].to_string()
            } else {
                item.display_name.clone()
            };

            if let Some(ref aircraft_type_id) = item.livery_aircraft_type {
                // Try to find the target aircraft
                if let Some(aircraft_folder) =
                    self.find_aircraft_for_livery(xplane_path, aircraft_type_id)
                {
                    // Found the aircraft, install to its liveries folder
                    let liveries_path = aircraft_folder.join("liveries").join(&livery_name);
                    (liveries_path, true)
                } else {
                    // Aircraft not found, use a placeholder path
                    let placeholder = xplane_root
                        .join("Aircraft")
                        .join("[Aircraft Not Found]")
                        .join("liveries")
                        .join(&livery_name);
                    (placeholder, false)
                }
            } else {
                // No aircraft type specified, shouldn't happen but handle gracefully
                let placeholder = xplane_root
                    .join("Aircraft")
                    .join("[Unknown Aircraft]")
                    .join("liveries")
                    .join(&livery_name);
                (placeholder, false)
            }
        } else {
            // Standard handling for non-livery types
            let target_base = match item.addon_type {
                AddonType::Aircraft => xplane_root.join("Aircraft"),
                AddonType::Scenery | AddonType::SceneryLibrary => {
                    xplane_root.join("Custom Scenery")
                }
                AddonType::Plugin => xplane_root.join("Resources").join("plugins"),
                AddonType::Navdata => {
                    // Determine if it's GNS430 or main Custom Data
                    if item.display_name.contains("GNS430") {
                        xplane_root.join("Custom Data").join("GNS430")
                    } else {
                        xplane_root.join("Custom Data")
                    }
                }
                AddonType::Livery => unreachable!(), // Already handled above
            };

            // For Navdata, install directly into target_base (don't create subfolder)
            // For other types, create a subfolder with the display_name
            let path = if item.addon_type == AddonType::Navdata {
                target_base
            } else {
                target_base.join(&item.display_name)
            };
            (path, true) // Non-livery types always have aircraft_found = true
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
        let password = Self::find_password(archive_passwords, &item.path);

        // Debug: log password lookup
        if password.is_some() {
            logger::log_info(
                &format!("Password found for task: {}", item.display_name),
                Some("analyzer"),
            );
        } else if !archive_passwords.is_empty() {
            logger::log_debug(
                &format!(
                    "Password NOT found for item.path: '{}', available keys: {:?}",
                    item.path,
                    archive_passwords.keys().collect::<Vec<_>>()
                ),
                Some("analyzer"),
                None,
            );
        }

        // Estimate size and check for warnings (for archives)
        let (estimated_size, size_warning) = self.estimate_archive_size(&item.path);

        // Determine source type and check verification preferences
        let source_path = Path::new(&item.path);
        let source_type = if source_path.is_dir() {
            "directory"
        } else if let Some(ext) = source_path.extension().and_then(|s| s.to_str()) {
            match ext.to_lowercase().as_str() {
                "zip" => "zip",
                "7z" => "7z",
                "rar" => "rar",
                _ => "directory", // Unknown archive types treated as directory
            }
        } else {
            "directory"
        };

        // Check if verification is enabled for this source type
        let enable_verification = verification_preferences
            .and_then(|prefs| prefs.get(source_type).copied())
            .unwrap_or(true); // Default to true if not specified

        InstallTask {
            id: Uuid::new_v4().to_string(),
            addon_type: item.addon_type,
            source_path: item.path,
            original_input_path: Some(item.original_input_path),
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
            backup_liveries: true,     // Default to true (safe)
            backup_config_files: true, // Default to true (safe)
            config_file_patterns: vec!["*_prefs.txt".to_string()], // Default pattern
            file_hashes: None,         // Will be populated by hash collector
            enable_verification,       // Based on verification preferences
            livery_aircraft_type: item.livery_aircraft_type,
            livery_aircraft_found,
        }
    }

    /// Collect file hashes for all tasks
    fn collect_hashes_for_tasks(&self, tasks: &mut [InstallTask]) {
        let hash_collector = crate::hash_collector::HashCollector::new();

        for task in tasks.iter_mut() {
            // Skip hash collection entirely if verification is disabled
            if !task.enable_verification {
                logger::log_info(
                    &format!(
                        "Hash collection skipped for task: {} (verification disabled)",
                        task.display_name
                    ),
                    Some("analyzer"),
                );
                continue;
            }

            match hash_collector.collect_hashes(task) {
                Ok(hashes) => {
                    if !hashes.is_empty() {
                        logger::log_info(
                            &format!(
                                "Collected {} hashes for task: {}",
                                hashes.len(),
                                task.display_name
                            ),
                            Some("analyzer"),
                        );
                        task.file_hashes = Some(hashes);
                    } else {
                        logger::log_info(
                            &format!(
                                "No hashes collected for task: {} (will compute during extraction)",
                                task.display_name
                            ),
                            Some("analyzer"),
                        );
                    }
                }
                Err(e) => {
                    logger::log_error(
                        &format!("Failed to collect hashes for {}: {}", task.display_name, e),
                        Some("analyzer"),
                    );
                    // Don't fail the task, just disable verification
                    task.enable_verification = false;
                }
            }
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

        // Get metadata from already-opened file handle (optimization: single pass)
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
    fn check_size_warning(
        &self,
        archive_size: u64,
        estimated_uncompressed: u64,
    ) -> (Option<u64>, Option<String>) {
        let mut warning = None;

        // Check compression ratio (potential zip bomb)
        // Only check if both sizes are non-zero to avoid division by zero
        if archive_size > 0 && estimated_uncompressed > 0 {
            let ratio = estimated_uncompressed / archive_size;
            if ratio > MAX_COMPRESSION_RATIO {
                warning = Some(format!(
                    "SUSPICIOUS_RATIO:{}:{}",
                    ratio, estimated_uncompressed
                ));
            }
        }

        // Check absolute size
        if estimated_uncompressed > MAX_EXTRACTION_SIZE {
            let size_gb = estimated_uncompressed as f64 / 1024.0 / 1024.0 / 1024.0;
            warning = Some(format!("LARGE_SIZE:{:.2}", size_gb));
        }

        (Some(estimated_uncompressed), warning)
    }

    /// Check if a path is a disk root directory
    /// Returns true for paths like "C:\", "D:\", "/", etc.
    fn is_disk_root(path: &Path) -> bool {
        // Normalize the path
        let path_str = path.to_string_lossy();

        // Windows: Check for drive root (e.g., "C:\", "D:\")
        #[cfg(target_os = "windows")]
        {
            // Match patterns like "C:\", "D:\", etc.
            if path_str.len() == 3
                && path_str.chars().nth(1) == Some(':')
                && path_str.ends_with('\\')
            {
                return true;
            }
            // Also match "C:", "D:" without trailing backslash
            if path_str.len() == 2 && path_str.chars().nth(1) == Some(':') {
                return true;
            }
        }

        // Unix: Check for root directory "/"
        #[cfg(not(target_os = "windows"))]
        {
            if path_str == "/" {
                return true;
            }
        }

        // Use Path::parent() to check if it has no parent (which means it's a root)
        path.parent().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create DetectedItem for tests
    fn create_detected_item(
        addon_type: AddonType,
        path: &str,
        display_name: &str,
        archive_internal_root: Option<String>,
    ) -> DetectedItem {
        DetectedItem {
            addon_type,
            path: path.to_string(),
            original_input_path: path.to_string(),
            display_name: display_name.to_string(),
            archive_internal_root,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
        }
    }

    // Helper function to create InstallTask for tests
    fn create_install_task(
        id: &str,
        addon_type: AddonType,
        source_path: &str,
        target_path: &str,
        display_name: &str,
    ) -> InstallTask {
        InstallTask {
            id: id.to_string(),
            addon_type,
            source_path: source_path.to_string(),
            original_input_path: Some(source_path.to_string()),
            target_path: target_path.to_string(),
            display_name: display_name.to_string(),
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
            extraction_chain: None,
            file_hashes: None,
            enable_verification: true,
            livery_aircraft_type: None,
            livery_aircraft_found: false,
        }
    }

    #[test]
    fn test_deduplication_same_type() {
        let analyzer = Analyzer::new();

        // Same type (Aircraft), nested paths - should deduplicate
        let items = vec![
            create_detected_item(AddonType::Aircraft, "/test/A330", "A330", None),
            create_detected_item(
                AddonType::Aircraft,
                "/test/A330/variant",
                "A330 Variant",
                None,
            ),
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
            create_detected_item(AddonType::Aircraft, "/test/A330", "A330", None),
            create_detected_item(AddonType::Plugin, "/test/A330/plugins/fms", "FMS", None),
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
            create_detected_item(
                AddonType::Aircraft,
                "/test/package.zip",
                "A330",
                Some("A330".to_string()),
            ),
            create_detected_item(
                AddonType::Plugin,
                "/test/package.zip",
                "FMS Plugin",
                Some("A330/plugins/fms".to_string()),
            ),
            create_detected_item(
                AddonType::Plugin,
                "/test/package.zip",
                "Standalone Plugin",
                Some("plugins/standalone".to_string()),
            ),
            create_detected_item(
                AddonType::Scenery,
                "/test/package.zip",
                "Airport",
                Some("scenery/airport".to_string()),
            ),
        ];

        let result = analyzer.deduplicate(items);

        // Should have: Aircraft, Standalone Plugin, Scenery (3 items)
        // FMS Plugin is filtered because it's inside Aircraft
        assert_eq!(result.len(), 3);

        let types: Vec<AddonType> = result.iter().map(|i| i.addon_type.clone()).collect();
        assert!(types.contains(&AddonType::Aircraft));
        assert!(types.contains(&AddonType::Scenery));

        // Check that standalone plugin is kept
        let plugins: Vec<_> = result
            .iter()
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
            create_detected_item(
                AddonType::Plugin,
                "/test/package.zip",
                "MainPlugin",
                Some("plugins".to_string()),
            ),
            create_detected_item(
                AddonType::Plugin,
                "/test/package.zip",
                "SubPlugin",
                Some("plugins/sub".to_string()),
            ),
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
            create_install_task(
                "1",
                AddonType::Aircraft,
                "/downloads/A330/A330.acf",
                "/X-Plane/Aircraft/A330",
                "A330",
            ),
            create_install_task(
                "2",
                AddonType::Aircraft,
                "/downloads/A330/A330_cargo.acf",
                "/X-Plane/Aircraft/A330",
                "A330",
            ),
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
            create_install_task(
                "1",
                AddonType::Aircraft,
                "/downloads/A330",
                "/X-Plane/Aircraft/A330",
                "A330",
            ),
            create_install_task(
                "2",
                AddonType::Aircraft,
                "/downloads/B737",
                "/X-Plane/Aircraft/B737",
                "B737",
            ),
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
            create_install_task(
                "1",
                AddonType::Plugin,
                "/downloads/MyPlugin/win.xpl",
                "/X-Plane/Resources/plugins/MyPlugin",
                "MyPlugin",
            ),
            create_install_task(
                "2",
                AddonType::Plugin,
                "/downloads/MyPlugin/mac.xpl",
                "/X-Plane/Resources/plugins/MyPlugin",
                "MyPlugin",
            ),
            create_install_task(
                "3",
                AddonType::Plugin,
                "/downloads/MyPlugin/lin.xpl",
                "/X-Plane/Resources/plugins/MyPlugin",
                "MyPlugin",
            ),
        ];

        let result = analyzer.deduplicate_by_target_path(tasks);

        // Should only have one task since all target paths are the same
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_plugin_inside_scenery_filtered() {
        let analyzer = Analyzer::new();

        let items = vec![
            create_detected_item(AddonType::Scenery, "/test/KSEA", "KSEA", None),
            create_detected_item(
                AddonType::Plugin,
                "/test/KSEA/plugins/lighting",
                "Lighting",
                None,
            ),
        ];

        let result = analyzer.deduplicate(items);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addon_type, AddonType::Scenery);
    }

    #[test]
    fn test_standalone_plugin_kept() {
        let analyzer = Analyzer::new();

        let items = vec![
            create_detected_item(AddonType::Aircraft, "/test/A330", "A330", None),
            create_detected_item(
                AddonType::Plugin,
                "/test/BetterPushback",
                "BetterPushback",
                None,
            ),
        ];

        let result = analyzer.deduplicate(items);

        // Both should be kept
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_scenery_vs_scenery_library_same_path() {
        let analyzer = Analyzer::new();

        // Same directory has both library.txt and .dsf files
        let items = vec![
            create_detected_item(AddonType::Scenery, "/test/KSEA", "KSEA Scenery", None),
            create_detected_item(
                AddonType::SceneryLibrary,
                "/test/KSEA",
                "KSEA Library",
                None,
            ),
        ];

        let result = analyzer.deduplicate(items);

        // Should only keep Scenery, not SceneryLibrary
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addon_type, AddonType::Scenery);
        assert_eq!(result[0].display_name, "KSEA Scenery");
    }

    #[test]
    fn test_scenery_and_scenery_library_different_paths() {
        let analyzer = Analyzer::new();

        // Different directories - both should be kept
        let items = vec![
            create_detected_item(AddonType::Scenery, "/test/KSEA", "KSEA Scenery", None),
            create_detected_item(
                AddonType::SceneryLibrary,
                "/test/Library",
                "Custom Library",
                None,
            ),
        ];

        let result = analyzer.deduplicate(items);

        // Both should be kept since they're different paths
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_archive_complex_priority() {
        let analyzer = Analyzer::new();

        let items = vec![
            create_detected_item(
                AddonType::Aircraft,
                "/test/pack.zip",
                "A330",
                Some("aircraft/A330".to_string()),
            ),
            create_detected_item(
                AddonType::Plugin,
                "/test/pack.zip",
                "Systems",
                Some("aircraft/A330/plugins/systems".to_string()),
            ),
            create_detected_item(
                AddonType::SceneryLibrary,
                "/test/pack.zip",
                "Library",
                Some("library".to_string()),
            ),
            create_detected_item(
                AddonType::Plugin,
                "/test/pack.zip",
                "LibPlugin",
                Some("library/plugins/helper".to_string()),
            ),
            create_detected_item(
                AddonType::Plugin,
                "/test/pack.zip",
                "Standalone",
                Some("plugins/standalone".to_string()),
            ),
        ];

        let result = analyzer.deduplicate(items);

        // Should have: Aircraft, SceneryLibrary, Standalone Plugin (3 items)
        assert_eq!(result.len(), 3);

        let plugin_names: Vec<String> = result
            .iter()
            .filter(|i| i.addon_type == AddonType::Plugin)
            .map(|i| i.display_name.clone())
            .collect();

        assert_eq!(plugin_names.len(), 1);
        assert_eq!(plugin_names[0], "Standalone");
    }
}

//! Scenery index management module
//!
//! This module manages a persistent JSON index of scenery classifications
//! with cache invalidation based on directory modification times.

use crate::logger;
use crate::models::{SceneryIndex, SceneryIndexStats, SceneryManagerData, SceneryManagerEntry, SceneryPackageInfo};
use crate::scenery_classifier::classify_scenery;
use anyhow::{anyhow, Result};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Resolve Windows shortcut (.lnk) to actual path using Windows COM API
#[cfg(windows)]
fn resolve_shortcut(lnk_path: &Path) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use winapi::shared::guiddef::GUID;
    use winapi::shared::minwindef::MAX_PATH;
    use winapi::shared::winerror::{S_OK, S_FALSE, RPC_E_CHANGED_MODE};
    use winapi::um::combaseapi::{CoCreateInstance, CoInitializeEx, CoUninitialize};
    use winapi::um::objbase::COINIT_APARTMENTTHREADED;
    use winapi::um::objidl::IPersistFile;
    use winapi::um::shobjidl_core::IShellLinkW;
    use winapi::Interface;

    // CLSID_ShellLink: 00021401-0000-0000-C000-000000000046
    const CLSID_SHELL_LINK: GUID = GUID {
        Data1: 0x00021401,
        Data2: 0x0000,
        Data3: 0x0000,
        Data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    };

    unsafe {
        // Initialize COM - try both threading models
        let hr = CoInitializeEx(ptr::null_mut(), COINIT_APARTMENTTHREADED);
        let need_uninit = if hr == S_OK || hr == S_FALSE {
            true
        } else if hr == RPC_E_CHANGED_MODE {
            // COM already initialized with different threading model, try to continue anyway
            logger::log_info(
                &format!("  COM already initialized with different threading model"),
                Some("scenery_index")
            );
            false
        } else {
            logger::log_info(
                &format!("  Failed to initialize COM, HRESULT: 0x{:08X}", hr),
                Some("scenery_index")
            );
            return None;
        };

        let mut result = None;

        // Create IShellLink instance
        let mut shell_link: *mut IShellLinkW = ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_SHELL_LINK,
            ptr::null_mut(),
            1, // CLSCTX_INPROC_SERVER
            &IShellLinkW::uuidof(),
            &mut shell_link as *mut *mut _ as *mut *mut _,
        );

        if hr == S_OK && !shell_link.is_null() {
            // Query IPersistFile interface
            let mut persist_file: *mut IPersistFile = ptr::null_mut();
            let hr = (*shell_link).QueryInterface(
                &IPersistFile::uuidof(),
                &mut persist_file as *mut *mut _ as *mut *mut _,
            );

            if hr == S_OK && !persist_file.is_null() {
                // Convert path to wide string
                let wide_path: Vec<u16> = lnk_path
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                // Load the shortcut file
                let hr = (*persist_file).Load(wide_path.as_ptr(), 0);

                if hr == S_OK {
                    // Get the target path
                    let mut target_path = vec![0u16; MAX_PATH];
                    let hr = (*shell_link).GetPath(
                        target_path.as_mut_ptr(),
                        MAX_PATH as i32,
                        ptr::null_mut(),
                        0,
                    );

                    if hr == S_OK {
                        // Find the null terminator
                        let len = target_path.iter().position(|&c| c == 0).unwrap_or(MAX_PATH);
                        let target_str = String::from_utf16_lossy(&target_path[..len]);

                        logger::log_info(
                            &format!("  Shortcut target (COM API): {:?}", target_str),
                            Some("scenery_index")
                        );

                        let path = PathBuf::from(target_str);
                        if path.exists() && path.is_dir() {
                            result = Some(path);
                        }
                    } else {
                        logger::log_info(
                            &format!("  GetPath failed with HRESULT: 0x{:08X}", hr),
                            Some("scenery_index")
                        );
                    }
                } else {
                    logger::log_info(
                        &format!("  Failed to load shortcut file, HRESULT: 0x{:08X}", hr),
                        Some("scenery_index")
                    );
                }

                (*persist_file).Release();
            } else {
                logger::log_info(
                    &format!("  Failed to query IPersistFile, HRESULT: 0x{:08X}", hr),
                    Some("scenery_index")
                );
            }

            (*shell_link).Release();
        } else {
            logger::log_info(
                &format!("  Failed to create IShellLink, HRESULT: 0x{:08X}", hr),
                Some("scenery_index")
            );
        }

        if need_uninit {
            CoUninitialize();
        }

        result
    }
}

#[cfg(not(windows))]
fn resolve_shortcut(_lnk_path: &Path) -> Option<PathBuf> {
    None
}


const INDEX_VERSION: u32 = 1;

/// Manager for scenery index operations
pub struct SceneryIndexManager {
    xplane_path: PathBuf,
    index_path: PathBuf,
}

impl SceneryIndexManager {
    /// Create a new index manager
    pub fn new(xplane_path: &Path) -> Self {
        let index_path = get_index_file_path();
        Self {
            xplane_path: xplane_path.to_path_buf(),
            index_path,
        }
    }

    /// Load index from disk or create new empty index
    pub fn load_index(&self) -> Result<SceneryIndex> {
        if self.index_path.exists() {
            let content = fs::read_to_string(&self.index_path)?;
            let index: SceneryIndex = serde_json::from_str(&content)?;

            // Check version compatibility
            if index.version != INDEX_VERSION {
                logger::log_info("Index version mismatch, rebuilding index", Some("scenery_index"));
                return Ok(self.create_empty_index());
            }

            Ok(index)
        } else {
            Ok(self.create_empty_index())
        }
    }

    /// Save index to disk
    pub fn save_index(&self, index: &SceneryIndex) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.index_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temp file first, then rename for atomic write
        let temp_path = self.index_path.with_extension("tmp");
        let content = serde_json::to_string_pretty(index)?;
        fs::write(&temp_path, &content)?;
        fs::rename(&temp_path, &self.index_path)?;

        Ok(())
    }

    /// Update or add a single package in the index
    pub fn update_package(&self, package_info: SceneryPackageInfo) -> Result<()> {
        let mut index = self.load_index()?;
        index.packages.insert(package_info.folder_name.clone(), package_info);
        index.last_updated = SystemTime::now();
        self.save_index(&index)?;
        Ok(())
    }

    /// Remove a package from the index
    pub fn remove_package(&self, folder_name: &str) -> Result<()> {
        let mut index = self.load_index()?;
        if index.packages.remove(folder_name).is_some() {
            index.last_updated = SystemTime::now();
            self.save_index(&index)?;
        }
        Ok(())
    }

    /// Rebuild entire index by scanning all scenery packages
    pub fn rebuild_index(&self) -> Result<SceneryIndex> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        if !custom_scenery_path.exists() {
            return Err(anyhow!("Custom Scenery folder not found"));
        }

        // Read enabled states from existing ini before rebuilding
        let enabled_states = self.read_enabled_states_from_ini()?;

        // Collect all scenery folders (including symlinks and .lnk shortcuts)
        let scenery_folders: Vec<PathBuf> = fs::read_dir(&custom_scenery_path)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let path = e.path();

                // Check if it's a .lnk file (Windows shortcut)
                if path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("lnk")) {
                    logger::log_info(
                        &format!("Attempting to resolve shortcut: {:?}", path.file_name().unwrap()),
                        Some("scenery_index")
                    );

                    // Try to resolve the shortcut
                    if let Some(target) = resolve_shortcut(&path) {
                        logger::log_info(
                            &format!("✓ Resolved shortcut {:?} -> {:?}", path.file_name().unwrap(), target),
                            Some("scenery_index")
                        );
                        return Some(target);
                    } else {
                        logger::log_info(
                            &format!("✗ Failed to resolve shortcut: {:?}", path),
                            Some("scenery_index")
                        );
                        return None;
                    }
                }

                // Check if it's a directory (including symlinks)
                if path.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                    return Some(path);
                }

                None
            })
            .collect();

        logger::log_info(&format!("Rebuilding scenery index for {} packages", scenery_folders.len()), Some("scenery_index"));

        // Classify all packages
        // Use sequential processing in debug log mode for ordered logs, parallel otherwise
        let mut packages_vec: Vec<SceneryPackageInfo> = if logger::is_debug_enabled() {
            // Sequential processing for ordered debug logs
            scenery_folders
                .iter()
                .filter_map(|folder| {
                    match classify_scenery(folder, &self.xplane_path) {
                        Ok(info) => Some(info),
                        Err(e) => {
                            logger::log_info(&format!("Failed to classify {:?}: {}", folder, e), Some("scenery_index"));
                            None
                        }
                    }
                })
                .collect()
        } else {
            // Parallel processing for better performance when not in debug mode
            scenery_folders
                .par_iter()
                .filter_map(|folder| {
                    match classify_scenery(folder, &self.xplane_path) {
                        Ok(info) => Some(info),
                        Err(e) => {
                            logger::log_info(&format!("Failed to classify {:?}: {}", folder, e), Some("scenery_index"));
                            None
                        }
                    }
                })
                .collect()
        };

        // Sort packages by category priority, sub-priority, then folder name
        packages_vec.sort_by(|a, b| {
            let priority_a = (a.category.priority(), a.sub_priority);
            let priority_b = (b.category.priority(), b.sub_priority);
            match priority_a.cmp(&priority_b) {
                std::cmp::Ordering::Equal => a.folder_name.to_lowercase().cmp(&b.folder_name.to_lowercase()),
                other => other,
            }
        });

        // Assign sort_order and apply enabled states from ini
        let packages: HashMap<String, SceneryPackageInfo> = packages_vec
            .into_iter()
            .enumerate()
            .map(|(index, mut info)| {
                info.sort_order = index as u32;
                // Apply enabled state from ini (default to true for new packages)
                info.enabled = enabled_states.get(&info.folder_name).copied().unwrap_or(true);
                (info.folder_name.clone(), info)
            })
            .collect();

        let index = SceneryIndex {
            version: INDEX_VERSION,
            packages,
            last_updated: SystemTime::now(),
        };

        self.save_index(&index)?;
        logger::log_info(&format!("Scenery index rebuilt with {} packages", index.packages.len()), Some("scenery_index"));

        // Update missing libraries for all packages using the complete index
        let index = self.update_missing_libraries(index)?;

        Ok(index)
    }

    /// Update missing libraries for all packages using the complete index
    fn update_missing_libraries(&self, mut index: SceneryIndex) -> Result<SceneryIndex> {
        logger::log_info("Updating missing libraries for all packages...", Some("scenery_index"));

        // Build library index from the complete scenery index
        let library_index = build_library_index_from_scenery_index(&index);

        // Update each package's missing_libraries
        for (folder_name, package_info) in index.packages.iter_mut() {
            let mut missing = Vec::new();

            for lib_name in &package_info.required_libraries {
                // Skip self-references
                if lib_name.eq_ignore_ascii_case(folder_name) {
                    continue;
                }

                // Check if this is a subdirectory within the current scenery package
                let scenery_path = self.xplane_path.join("Custom Scenery").join(folder_name);
                let subdir_path = scenery_path.join(lib_name);
                if subdir_path.exists() && subdir_path.is_dir() {
                    continue;
                }

                // Check if this library name is in the library index
                if !library_index.contains_key(lib_name) {
                    missing.push(lib_name.clone());
                }
            }

            package_info.missing_libraries = missing;
        }

        // Save the updated index
        self.save_index(&index)?;
        logger::log_info("Missing libraries updated for all packages", Some("scenery_index"));

        Ok(index)
    }

    /// Update index incrementally - only re-classify modified packages
    pub fn update_index(&self) -> Result<SceneryIndex> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        if !custom_scenery_path.exists() {
            return Err(anyhow!("Custom Scenery folder not found"));
        }

        let mut index = self.load_index()?;

        // Get current scenery folders (including symlinks and .lnk shortcuts)
        let current_folders: HashMap<String, PathBuf> = fs::read_dir(&custom_scenery_path)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let path = e.path();

                // Check if it's a .lnk file (Windows shortcut)
                if path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("lnk")) {
                    // Try to resolve the shortcut
                    if let Some(target) = resolve_shortcut(&path) {
                        // Use the target folder name (not the .lnk filename)
                        if let Some(name) = target.file_name().and_then(|s| s.to_str()) {
                            return Some((name.to_string(), target));
                        }
                    }
                    return None;
                }

                // Check if it's a directory (including symlinks)
                if path.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                    if let Some(name) = e.file_name().to_str() {
                        return Some((name.to_string(), path));
                    }
                }

                None
            })
            .collect();

        // Remove stale entries (deleted folders)
        let stale_keys: Vec<String> = index
            .packages
            .keys()
            .filter(|name| !current_folders.contains_key(*name))
            .cloned()
            .collect();

        for key in stale_keys {
            index.packages.remove(&key);
            crate::log_debug!(&format!("Removed stale entry: {}", key), "scenery_index");
        }

        // Find packages that need updating
        let packages_to_update: Vec<PathBuf> = current_folders
            .iter()
            .filter(|(name, path)| {
                // Check if package is new or modified
                if let Some(existing) = index.packages.get(*name) {
                    // Compare modification times
                    if let Ok(metadata) = fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            return modified > existing.indexed_at;
                        }
                    }
                    false
                } else {
                    true // New package
                }
            })
            .map(|(_, path)| path.clone())
            .collect();

        if !packages_to_update.is_empty() {
            logger::log_info(&format!("Updating {} scenery packages", packages_to_update.len()), Some("scenery_index"));

            // Classify updated packages
            // Use sequential processing in debug log mode for ordered logs, parallel otherwise
            let updated_packages: Vec<SceneryPackageInfo> = if logger::is_debug_enabled() {
                // Sequential processing for ordered debug logs
                packages_to_update
                    .iter()
                    .filter_map(|folder| {
                        classify_scenery(folder, &self.xplane_path).ok()
                    })
                    .collect()
            } else {
                // Parallel processing for better performance when not in debug mode
                packages_to_update
                    .par_iter()
                    .filter_map(|folder| {
                        classify_scenery(folder, &self.xplane_path).ok()
                    })
                    .collect()
            };

            for info in updated_packages {
                index.packages.insert(info.folder_name.clone(), info);
            }
        }

        index.last_updated = SystemTime::now();
        self.save_index(&index)?;

        Ok(index)
    }

    /// Clean up stale entries (deleted packages)
    pub fn cleanup_stale_entries(&self) -> Result<usize> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        let mut index = self.load_index()?;
        let initial_count = index.packages.len();

        // Get current folders (including symlinks)
        let current_folders: std::collections::HashSet<String> = fs::read_dir(&custom_scenery_path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                // Use metadata() to follow symlinks
                e.path().metadata().map(|m| m.is_dir()).unwrap_or(false)
            })
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .collect();

        // Remove entries not in current folders
        index.packages.retain(|name, _| current_folders.contains(name));

        let removed_count = initial_count - index.packages.len();
        if removed_count > 0 {
            index.last_updated = SystemTime::now();
            self.save_index(&index)?;
            logger::log_info(&format!("Cleaned up {} stale index entries", removed_count), Some("scenery_index"));
        }

        Ok(removed_count)
    }

    /// Check if a package needs re-classification
    pub fn is_package_stale(&self, folder_name: &str, folder_path: &Path) -> Result<bool> {
        let index = self.load_index()?;

        if let Some(existing) = index.packages.get(folder_name) {
            if let Ok(metadata) = fs::metadata(folder_path) {
                if let Ok(modified) = metadata.modified() {
                    return Ok(modified > existing.indexed_at);
                }
            }
        }

        Ok(true) // Assume stale if we can't determine
    }

    /// Get package info from index
    pub fn get_package(&self, folder_name: &str) -> Result<Option<SceneryPackageInfo>> {
        let index = self.load_index()?;
        Ok(index.packages.get(folder_name).cloned())
    }

    /// Get or classify a package (uses cache if available and not stale)
    pub fn get_or_classify(&self, folder_path: &Path) -> Result<SceneryPackageInfo> {
        let folder_name = folder_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid folder name"))?;

        // Check if we have a valid cached entry
        if !self.is_package_stale(folder_name, folder_path)? {
            if let Some(info) = self.get_package(folder_name)? {
                return Ok(info);
            }
        }

        // Classify and update index
        let info = classify_scenery(folder_path, &self.xplane_path)?;
        self.update_package(info.clone())?;
        Ok(info)
    }

    /// Get index statistics
    pub fn get_stats(&self) -> Result<SceneryIndexStats> {
        let index = self.load_index()?;

        let mut by_category: HashMap<String, usize> = HashMap::new();
        for info in index.packages.values() {
            let category_name = format!("{:?}", info.category);
            *by_category.entry(category_name).or_insert(0) += 1;
        }

        Ok(SceneryIndexStats {
            total_packages: index.packages.len(),
            by_category,
            last_updated: index.last_updated,
        })
    }

    /// Read enabled states from scenery_packs.ini
    pub fn read_enabled_states_from_ini(&self) -> Result<HashMap<String, bool>> {
        let ini_path = self.xplane_path.join("Custom Scenery").join("scenery_packs.ini");
        let mut enabled_states = HashMap::new();

        if !ini_path.exists() {
            return Ok(enabled_states);
        }

        let file = fs::File::open(&ini_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            // Skip empty lines and header lines
            if trimmed.is_empty()
                || trimmed == "I"
                || trimmed.starts_with("1000")
                || trimmed == "SCENERY"
            {
                continue;
            }

            // Parse SCENERY_PACK or SCENERY_PACK_DISABLED lines
            let (enabled, path_part) = if trimmed.starts_with("SCENERY_PACK_DISABLED ") {
                (false, trimmed.strip_prefix("SCENERY_PACK_DISABLED ").unwrap_or(""))
            } else if trimmed.starts_with("SCENERY_PACK ") {
                (true, trimmed.strip_prefix("SCENERY_PACK ").unwrap_or(""))
            } else {
                continue;
            };

            // Extract folder name from path
            let path = path_part.trim().trim_end_matches('/').trim_end_matches('\\');
            if path.contains("*GLOBAL_AIRPORTS*") {
                continue;
            }

            if let Some(folder_name) = path.rsplit(&['/', '\\'][..]).next() {
                enabled_states.insert(folder_name.to_string(), enabled);
            }
        }

        Ok(enabled_states)
    }

    /// Update a single entry's enabled state and/or sort_order
    pub fn update_entry(&self, folder_name: &str, enabled: Option<bool>, sort_order: Option<u32>) -> Result<()> {
        let mut index = self.load_index()?;

        if let Some(info) = index.packages.get_mut(folder_name) {
            if let Some(e) = enabled {
                info.enabled = e;
            }
            if let Some(s) = sort_order {
                info.sort_order = s;
            }
            index.last_updated = SystemTime::now();
            self.save_index(&index)?;
        }

        Ok(())
    }

    /// Move an entry from one position to another, auto-adjusting other entries
    pub fn move_entry(&self, folder_name: &str, new_sort_order: u32) -> Result<()> {
        let mut index = self.load_index()?;

        // Get current sort_order
        let current_sort_order = match index.packages.get(folder_name) {
            Some(info) => info.sort_order,
            None => return Err(anyhow!("Package not found: {}", folder_name)),
        };

        if current_sort_order == new_sort_order {
            return Ok(()); // No change needed
        }

        // Adjust sort_orders of other entries
        if new_sort_order < current_sort_order {
            // Moving up: increment sort_order of entries in between
            for info in index.packages.values_mut() {
                if info.sort_order >= new_sort_order && info.sort_order < current_sort_order {
                    info.sort_order += 1;
                }
            }
        } else {
            // Moving down: decrement sort_order of entries in between
            for info in index.packages.values_mut() {
                if info.sort_order > current_sort_order && info.sort_order <= new_sort_order {
                    info.sort_order -= 1;
                }
            }
        }

        // Set the new sort_order for the moved entry
        if let Some(info) = index.packages.get_mut(folder_name) {
            info.sort_order = new_sort_order;
        }

        index.last_updated = SystemTime::now();
        self.save_index(&index)?;

        Ok(())
    }

    /// Get scenery manager data for UI
    pub fn get_manager_data(&self) -> Result<SceneryManagerData> {
        let index = self.load_index()?;

        // Convert to manager entries and sort by sort_order
        let mut entries: Vec<SceneryManagerEntry> = index
            .packages
            .values()
            .map(|info| SceneryManagerEntry {
                folder_name: info.folder_name.clone(),
                category: info.category.clone(),
                sub_priority: info.sub_priority,
                enabled: info.enabled,
                sort_order: info.sort_order,
                missing_libraries: info.missing_libraries.clone(),
                required_libraries: info.required_libraries.clone(),
            })
            .collect();

        // Sort by sort_order
        entries.sort_by_key(|e| e.sort_order);

        // Calculate statistics
        let total_count = entries.len();
        let enabled_count = entries.iter().filter(|e| e.enabled).count();
        let missing_deps_count = entries.iter().filter(|e| !e.missing_libraries.is_empty()).count();

        Ok(SceneryManagerData {
            entries,
            total_count,
            enabled_count,
            missing_deps_count,
        })
    }

    /// Create an empty index
    fn create_empty_index(&self) -> SceneryIndex {
        SceneryIndex {
            version: INDEX_VERSION,
            packages: HashMap::new(),
            last_updated: SystemTime::now(),
        }
    }
}

/// Get the path to the scenery index file
fn get_index_file_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data)
                .join("XFastInstall")
                .join("scenery_index.json");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("XFastInstall")
                .join("scenery_index.json");
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("xfastinstall")
                .join("scenery_index.json");
        }
    }

    // Fallback to current directory
    PathBuf::from("scenery_index.json")
}

/// Parse library.txt file and extract all exported library names
/// Returns a set of library name prefixes that this library exports
pub fn parse_library_exports(library_txt_path: &Path) -> HashSet<String> {
    let mut library_names = HashSet::new();

    // Check if library.txt exists
    if !library_txt_path.exists() {
        return library_names;
    }

    // Read and parse library.txt
    if let Ok(file) = fs::File::open(library_txt_path) {
        let reader = BufReader::new(file);

        for line in reader.lines().flatten() {
            let trimmed = line.trim();

            // Look for EXPORT lines (may have space or tab after EXPORT)
            if trimmed.starts_with("EXPORT") && trimmed.len() > 6 {
                // Skip "EXPORT" and any whitespace
                let after_export = &trimmed[6..].trim_start();

                // Format: <virtual_path> <actual_path>
                // We want the first component of <virtual_path>
                let parts: Vec<&str> = after_export.split_whitespace().collect();
                if !parts.is_empty() {
                    let virtual_path = parts[0];
                    // Extract first path component (library name)
                    // Support both forward slash and backslash
                    let first_component = virtual_path
                        .split(&['/', '\\'][..])
                        .next();

                    if let Some(component) = first_component {
                        if !component.is_empty() {
                            library_names.insert(component.to_string());
                        }
                    }
                }
            }
        }
    }

    library_names
}

/// Build a library name index from scenery index
/// Returns a HashMap mapping library names to folder names
pub fn build_library_index_from_scenery_index(index: &SceneryIndex) -> HashMap<String, String> {
    let mut library_index: HashMap<String, String> = HashMap::new();

    for (folder_name, package_info) in &index.packages {
        // Only process packages with exported library names
        if !package_info.exported_library_names.is_empty() {
            for lib_name in &package_info.exported_library_names {
                library_index.insert(lib_name.clone(), folder_name.clone());
            }
        }
    }

    logger::log_info(
        &format!("Built library index from scenery index with {} entries", library_index.len()),
        Some("library_index")
    );

    library_index
}

/// Build a library name index for all scenery packages in Custom Scenery
/// Returns a HashMap mapping library names to folder names
pub fn build_library_index(xplane_path: &Path) -> HashMap<String, String> {
    let mut library_index: HashMap<String, String> = HashMap::new();
    let custom_scenery_path = xplane_path.join("Custom Scenery");

    if !custom_scenery_path.exists() {
        return library_index;
    }

    // Scan all folders in Custom Scenery
    if let Ok(entries) = fs::read_dir(&custom_scenery_path) {
        for entry in entries.flatten() {
            let path = entry.path();

            // Check if it's a directory
            if path.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                let folder_name = match path.file_name().and_then(|s| s.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };

                // Check for library.txt
                let library_txt = path.join("library.txt");
                if library_txt.exists() {
                    // Parse library.txt and get all exported library names
                    let exported_names = parse_library_exports(&library_txt);

                    logger::log_info(
                        &format!("Library folder '{}' exports: {:?}", folder_name, exported_names),
                        Some("library_index")
                    );

                    // Map each exported library name to this folder
                    for lib_name in exported_names {
                        library_index.insert(lib_name, folder_name.clone());
                    }
                }
            }
        }
    }

    logger::log_info(
        &format!("Built library index with {} entries", library_index.len()),
        Some("library_index")
    );

    library_index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_file_path() {
        let path = get_index_file_path();
        assert!(path.to_string_lossy().contains("scenery_index.json"));
    }

    #[test]
    fn test_empty_index_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SceneryIndexManager::new(temp_dir.path());
        let index = manager.create_empty_index();

        assert_eq!(index.version, INDEX_VERSION);
        assert!(index.packages.is_empty());
    }
}

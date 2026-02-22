//! Scenery index management module
//!
//! This module manages a persistent SQLite database of scenery classifications
//! with cache invalidation based on directory modification times.

use crate::database::{SceneryQueries, CURRENT_SCHEMA_VERSION};
use crate::logger;
use crate::models::{
    SceneryCategory, SceneryIndex, SceneryIndexScanResult, SceneryIndexStats, SceneryIndexStatus,
    SceneryManagerData, SceneryManagerEntry, SceneryPackageInfo,
};
use crate::scenery_classifier::classify_scenery;
use anyhow::{anyhow, Result};
use rayon::prelude::*;
use sea_orm::DatabaseConnection;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

type AirportCoords = HashMap<(i32, i32), Vec<(String, Option<String>)>>;

// ============================================================================
// Windows Shortcut Resolution (COM API)
// ============================================================================

#[cfg(windows)]
mod shortcut_resolver {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use winapi::shared::guiddef::GUID;
    use winapi::shared::minwindef::MAX_PATH;
    use winapi::shared::winerror::{RPC_E_CHANGED_MODE, S_FALSE, S_OK};
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

    /// RAII wrapper for COM initialization
    struct ComGuard {
        should_uninit: bool,
    }

    impl ComGuard {
        /// Initialize COM with apartment threading model
        fn new() -> Option<Self> {
            unsafe {
                let hr = CoInitializeEx(ptr::null_mut(), COINIT_APARTMENTTHREADED);
                if hr == S_OK || hr == S_FALSE {
                    Some(Self {
                        should_uninit: true,
                    })
                } else if hr == RPC_E_CHANGED_MODE {
                    logger::log_info(
                        "  COM already initialized with different threading model",
                        Some("scenery_index"),
                    );
                    Some(Self {
                        should_uninit: false,
                    })
                } else {
                    logger::log_info(
                        &format!("  Failed to initialize COM, HRESULT: 0x{:08X}", hr),
                        Some("scenery_index"),
                    );
                    None
                }
            }
        }
    }

    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.should_uninit {
                unsafe { CoUninitialize() };
            }
        }
    }

    /// RAII wrapper for IShellLinkW COM interface
    struct ShellLinkGuard {
        ptr: *mut IShellLinkW,
    }

    impl ShellLinkGuard {
        fn new() -> Option<Self> {
            unsafe {
                let mut shell_link: *mut IShellLinkW = ptr::null_mut();
                let hr = CoCreateInstance(
                    &CLSID_SHELL_LINK,
                    ptr::null_mut(),
                    1, // CLSCTX_INPROC_SERVER
                    &IShellLinkW::uuidof(),
                    &mut shell_link as *mut *mut _ as *mut *mut _,
                );
                if hr == S_OK && !shell_link.is_null() {
                    Some(Self { ptr: shell_link })
                } else {
                    logger::log_info(
                        &format!("  Failed to create IShellLink, HRESULT: 0x{:08X}", hr),
                        Some("scenery_index"),
                    );
                    None
                }
            }
        }

        fn as_ptr(&self) -> *mut IShellLinkW {
            self.ptr
        }
    }

    impl Drop for ShellLinkGuard {
        fn drop(&mut self) {
            if !self.ptr.is_null() {
                unsafe { (*self.ptr).Release() };
            }
        }
    }

    /// RAII wrapper for IPersistFile COM interface
    struct PersistFileGuard {
        ptr: *mut IPersistFile,
    }

    impl PersistFileGuard {
        fn from_shell_link(shell_link: &ShellLinkGuard) -> Option<Self> {
            unsafe {
                let mut persist_file: *mut IPersistFile = ptr::null_mut();
                let hr = (*shell_link.as_ptr()).QueryInterface(
                    &IPersistFile::uuidof(),
                    &mut persist_file as *mut *mut _ as *mut *mut _,
                );
                if hr == S_OK && !persist_file.is_null() {
                    Some(Self { ptr: persist_file })
                } else {
                    logger::log_info(
                        &format!("  Failed to query IPersistFile, HRESULT: 0x{:08X}", hr),
                        Some("scenery_index"),
                    );
                    None
                }
            }
        }

        fn load(&self, path: &Path) -> bool {
            unsafe {
                let wide_path: Vec<u16> = path
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let hr = (*self.ptr).Load(wide_path.as_ptr(), 0);
                if hr != S_OK {
                    logger::log_info(
                        &format!("  Failed to load shortcut file, HRESULT: 0x{:08X}", hr),
                        Some("scenery_index"),
                    );
                }
                hr == S_OK
            }
        }
    }

    impl Drop for PersistFileGuard {
        fn drop(&mut self) {
            if !self.ptr.is_null() {
                unsafe { (*self.ptr).Release() };
            }
        }
    }

    /// Get the target path from a loaded shell link
    fn get_shell_link_target(shell_link: &ShellLinkGuard) -> Option<PathBuf> {
        unsafe {
            let mut target_path = vec![0u16; MAX_PATH];
            let hr = (*shell_link.as_ptr()).GetPath(
                target_path.as_mut_ptr(),
                MAX_PATH as i32,
                ptr::null_mut(),
                0,
            );
            if hr == S_OK {
                let len = target_path.iter().position(|&c| c == 0).unwrap_or(MAX_PATH);
                let target_str = String::from_utf16_lossy(&target_path[..len]);
                logger::log_info(
                    &format!("  Shortcut target (COM API): {:?}", target_str),
                    Some("scenery_index"),
                );
                let path = PathBuf::from(target_str);
                if path.exists() && path.is_dir() {
                    return Some(path);
                }
            } else {
                logger::log_info(
                    &format!("  GetPath failed with HRESULT: 0x{:08X}", hr),
                    Some("scenery_index"),
                );
            }
            None
        }
    }

    /// Resolve a Windows shortcut (.lnk) to its target path
    pub fn resolve(lnk_path: &Path) -> Option<PathBuf> {
        let _com = ComGuard::new()?;
        let shell_link = ShellLinkGuard::new()?;
        let persist_file = PersistFileGuard::from_shell_link(&shell_link)?;

        if !persist_file.load(lnk_path) {
            return None;
        }

        get_shell_link_target(&shell_link)
    }
}

/// Resolve Windows shortcut (.lnk) to actual path using Windows COM API
#[cfg(windows)]
fn resolve_shortcut(lnk_path: &Path) -> Option<PathBuf> {
    shortcut_resolver::resolve(lnk_path)
}

#[cfg(not(windows))]
fn resolve_shortcut(_lnk_path: &Path) -> Option<PathBuf> {
    None
}

fn is_sam_folder_name(folder_name: &str) -> bool {
    let folder_lower = folder_name.to_lowercase();

    let parts: Vec<&str> = folder_lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();

    let has_sam_word = parts.contains(&"sam");
    let has_sam_suffix = parts.iter().any(|&part| {
        part.ends_with("sam") && part.len() > 3 && {
            let prefix = &part[..part.len() - 3];
            matches!(prefix, "open" | "my" | "custom" | "new")
        }
    });

    has_sam_word || has_sam_suffix
}

/// Common sorting comparison for non-FixedHighPriority scenery packages
/// This ensures consistent ordering between rebuild_index, recalculate_sort_order, and reset_sort_order
fn compare_packages_for_sorting(
    name_a: &str,
    info_a: &SceneryPackageInfo,
    name_b: &str,
    info_b: &SceneryPackageInfo,
) -> std::cmp::Ordering {
    let priority_a = (info_a.category.priority(), info_a.sub_priority);
    let priority_b = (info_b.category.priority(), info_b.sub_priority);

    match priority_a.cmp(&priority_b) {
        std::cmp::Ordering::Equal => {
            if info_a.category == info_b.category
                && matches!(
                    info_a.category,
                    SceneryCategory::AirportMesh | SceneryCategory::Mesh
                )
            {
                // For Mesh category with sub_priority > 0 (XPME), sort only by folder name
                // XPME mesh should be at the bottom of Mesh category, sorted alphabetically
                if info_a.category == SceneryCategory::Mesh && info_a.sub_priority > 0 {
                    name_a.to_lowercase().cmp(&name_b.to_lowercase())
                } else {
                    // Non-XPME: sort by tile count first, then folder name
                    match info_a
                        .earth_nav_tile_count
                        .cmp(&info_b.earth_nav_tile_count)
                    {
                        std::cmp::Ordering::Equal => {
                            name_a.to_lowercase().cmp(&name_b.to_lowercase())
                        }
                        other => other,
                    }
                }
            } else {
                name_a.to_lowercase().cmp(&name_b.to_lowercase())
            }
        }
        other => other,
    }
}

/// Manager for scenery index operations
pub struct SceneryIndexManager {
    xplane_path: PathBuf,
    db: DatabaseConnection,
}

impl SceneryIndexManager {
    /// Create a new index manager
    pub fn new(xplane_path: &Path, db: DatabaseConnection) -> Self {
        Self {
            xplane_path: xplane_path.to_path_buf(),
            db,
        }
    }

    /// Check if the scenery index has been created (has any packages)
    /// Returns true if there are packages in the index, false otherwise
    pub async fn has_index(&self) -> Result<bool> {
        SceneryQueries::has_packages(&self.db)
            .await
            .map_err(|e| anyhow!("{}", e))
    }

    /// Load index from database or create new empty index
    pub async fn load_index(&self) -> Result<SceneryIndex> {
        // Check if database has any packages
        let has_packages = SceneryQueries::has_packages(&self.db)
            .await
            .map_err(|e| anyhow!("{}", e))?;

        if has_packages {
            SceneryQueries::load_all(&self.db)
                .await
                .map_err(|e| anyhow!("{}", e))
        } else {
            Ok(self.create_empty_index())
        }
    }

    /// Save index to database
    pub async fn save_index(&self, index: &SceneryIndex) -> Result<()> {
        SceneryQueries::save_all(&self.db, index)
            .await
            .map_err(|e| anyhow!("{}", e))
    }

    /// Update or add a single package in the index
    pub async fn update_package(&self, package_info: SceneryPackageInfo) -> Result<()> {
        SceneryQueries::update_package(&self.db, &package_info)
            .await
            .map_err(|e| anyhow!("{}", e))
    }

    /// Rebuild entire index by scanning all scenery packages
    /// This completely clears the existing index and rebuilds from scratch
    pub async fn rebuild_index(&self) -> Result<SceneryIndex> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        if !custom_scenery_path.exists() {
            return Err(anyhow!("Custom Scenery folder not found"));
        }

        // Clear all existing index data for a fresh rebuild
        SceneryQueries::clear_all(&self.db)
            .await
            .map_err(|e| anyhow!("{}", e))?;

        logger::log_info(
            "Cleared existing index, starting fresh rebuild",
            Some("scenery_index"),
        );

        let xplane_path = self.xplane_path.clone();
        let custom_scenery_path = custom_scenery_path.clone();

        let index = tokio::task::spawn_blocking(move || -> Result<SceneryIndex> {
            // Collect all scenery folders (including symlinks and .lnk shortcuts)
            // Track shortcuts by their target path to correctly map shortcut names
            // Key: canonical target path, Value: (shortcut_name without .lnk, normalized_target_path for ini)
            let mut shortcut_target_map: HashMap<PathBuf, (String, String)> = HashMap::new();

            let scenery_folders: Vec<PathBuf> = fs::read_dir(&custom_scenery_path)?
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let path = e.path();

                    // Check if it's a .lnk file (Windows shortcut)
                    if path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
                    {
                        // Use shortcut file name (without .lnk extension) as the entry name
                        // This prevents conflicts when multiple shortcuts point to folders with the same name
                        let shortcut_name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "<unknown>".to_string());
                        logger::log_info(
                            &format!("Attempting to resolve shortcut: {}.lnk", shortcut_name),
                            Some("scenery_index"),
                        );

                        // Try to resolve the shortcut
                        if let Some(target) = resolve_shortcut(&path) {
                            logger::log_info(
                                &format!(
                                    "✓ Resolved shortcut {}.lnk -> {:?}",
                                    shortcut_name, target
                                ),
                                Some("scenery_index"),
                            );

                            // Store the mapping from target path to shortcut info
                            let target_path_str = target.to_string_lossy().to_string();
                            // Convert backslashes to forward slashes for scenery_packs.ini compatibility
                            let normalized_path_str = target_path_str.replace('\\', "/");
                            shortcut_target_map
                                .insert(target.clone(), (shortcut_name, normalized_path_str));

                            return Some(target);
                        } else {
                            logger::log_info(
                                &format!("✗ Failed to resolve shortcut: {:?}", path),
                                Some("scenery_index"),
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

            logger::log_info(
                &format!(
                    "Rebuilding scenery index for {} packages",
                    scenery_folders.len()
                ),
                Some("scenery_index"),
            );

            // Classify all packages
            // Track which path each package came from to correctly handle shortcuts
            // Use sequential processing in debug log mode for ordered logs, parallel otherwise
            let packages_with_paths: Vec<(PathBuf, SceneryPackageInfo)> =
                if logger::is_debug_enabled() {
                    // Sequential processing for ordered debug logs
                    scenery_folders
                        .iter()
                        .filter_map(|folder| match classify_scenery(folder, &xplane_path) {
                            Ok(info) => Some((folder.clone(), info)),
                            Err(e) => {
                                logger::log_info(
                                    &format!("Failed to classify {:?}: {}", folder, e),
                                    Some("scenery_index"),
                                );
                                None
                            }
                        })
                        .collect()
                } else {
                    // Parallel processing for better performance when not in debug mode
                    scenery_folders
                        .par_iter()
                        .filter_map(|folder| match classify_scenery(folder, &xplane_path) {
                            Ok(info) => Some((folder.clone(), info)),
                            Err(e) => {
                                logger::log_info(
                                    &format!("Failed to classify {:?}: {}", folder, e),
                                    Some("scenery_index"),
                                );
                                None
                            }
                        })
                        .collect()
                };

            // Post-process: Set folder_name and actual_path for shortcut entries
            // For shortcuts, use the shortcut name (not target folder name) to avoid conflicts
            let mut packages_vec: Vec<SceneryPackageInfo> =
                Vec::with_capacity(packages_with_paths.len());
            for (path, mut info) in packages_with_paths {
                if let Some((shortcut_name, actual_path)) = shortcut_target_map.get(&path) {
                    // This entry came from a shortcut - use shortcut name and set actual_path
                    logger::log_info(
                        &format!(
                            "Shortcut entry: {} (target folder: {}) -> actual_path: {}",
                            shortcut_name, info.folder_name, actual_path
                        ),
                        Some("scenery_index"),
                    );
                    info.folder_name = shortcut_name.clone();
                    info.actual_path = Some(actual_path.clone());
                }
                packages_vec.push(info);
            }

            // Post-process: Detect airport-associated mesh packages
            detect_airport_mesh_packages_with_path(&xplane_path, &mut packages_vec);

            // Sort packages using the common sorting function
            packages_vec
                .sort_by(|a, b| compare_packages_for_sorting(&a.folder_name, a, &b.folder_name, b));

            // Assign sort_order and set default enabled state
            // Fresh rebuild: Unrecognized packages default to disabled, others default to enabled
            let packages: HashMap<String, SceneryPackageInfo> = packages_vec
                .into_iter()
                .enumerate()
                .map(|(index, mut info)| {
                    info.sort_order = index as u32;
                    // Unrecognized packages default to disabled, others default to enabled
                    info.enabled = info.category != SceneryCategory::Unrecognized;
                    (info.folder_name.clone(), info)
                })
                .collect();

            Ok(SceneryIndex {
                version: CURRENT_SCHEMA_VERSION as u32,
                packages,
                last_updated: SystemTime::now(),
            })
        })
        .await
        .map_err(|e| anyhow!("Blocking task failed: {}", e))??;

        self.save_index(&index).await?;
        logger::log_info(
            &format!(
                "Scenery index rebuilt with {} packages",
                index.packages.len()
            ),
            Some("scenery_index"),
        );

        // Update missing libraries for all packages using the complete index
        let index = self.update_missing_libraries(index).await?;

        Ok(index)
    }

    /// Update missing libraries for all packages using the complete index
    async fn update_missing_libraries(&self, mut index: SceneryIndex) -> Result<SceneryIndex> {
        logger::log_info(
            "Updating missing libraries for all packages...",
            Some("scenery_index"),
        );

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
                if !library_index.contains_key(&lib_name.to_lowercase()) {
                    missing.push(lib_name.clone());
                }
            }

            package_info.missing_libraries = missing;
        }

        // Save the updated index
        self.save_index(&index).await?;
        logger::log_info(
            "Missing libraries updated for all packages",
            Some("scenery_index"),
        );

        Ok(index)
    }

    /// Recalculate sort_order for all packages using the same sorting logic as rebuild_index
    /// This ensures incremental updates produce consistent ordering with full rebuilds
    fn recalculate_sort_order(index: &mut SceneryIndex) {
        if index.packages.is_empty() {
            return;
        }

        // Promote SAM libraries to FixedHighPriority before sorting
        for (name, info) in index.packages.iter_mut() {
            if is_sam_folder_name(name)
                && info.has_library_txt
                && !info.has_dsf
                && !info.has_apt_dat
                && info.category != SceneryCategory::FixedHighPriority
            {
                info.category = SceneryCategory::FixedHighPriority;
                info.sub_priority = 0;
            }
        }

        // Separate FixedHighPriority packages (preserve their relative order)
        let mut fixed_packages: Vec<(&String, &SceneryPackageInfo)> = index
            .packages
            .iter()
            .filter(|(_, info)| info.category == SceneryCategory::FixedHighPriority)
            .collect();

        fixed_packages.sort_by(|(name_a, info_a), (name_b, info_b)| {
            let sam_a = is_sam_folder_name(name_a);
            let sam_b = is_sam_folder_name(name_b);
            match sam_b.cmp(&sam_a) {
                std::cmp::Ordering::Equal => {}
                other => return other,
            }
            match info_a.sort_order.cmp(&info_b.sort_order) {
                std::cmp::Ordering::Equal => name_a.to_lowercase().cmp(&name_b.to_lowercase()),
                other => other,
            }
        });

        // Sort other packages using the common sorting function
        let mut other_packages: Vec<(&String, &SceneryPackageInfo)> = index
            .packages
            .iter()
            .filter(|(_, info)| info.category != SceneryCategory::FixedHighPriority)
            .collect();

        other_packages.sort_by(|(name_a, info_a), (name_b, info_b)| {
            compare_packages_for_sorting(name_a, info_a, name_b, info_b)
        });

        // Collect sorted names and update sort_order
        let sorted_names: Vec<String> = fixed_packages
            .iter()
            .map(|(name, _)| (*name).clone())
            .chain(other_packages.iter().map(|(name, _)| (*name).clone()))
            .collect();

        for (new_order, folder_name) in sorted_names.iter().enumerate() {
            if let Some(info) = index.packages.get_mut(folder_name) {
                info.sort_order = new_order as u32;
            }
        }

        logger::log_info(
            &format!(
                "Recalculated sort order for {} packages",
                sorted_names.len()
            ),
            Some("scenery_index"),
        );
    }

    /// Update index incrementally - only re-classify modified packages
    pub async fn update_index(&self) -> Result<SceneryIndex> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        if !custom_scenery_path.exists() {
            return Err(anyhow!("Custom Scenery folder not found"));
        }

        let index = self.load_index().await?;
        let xplane_path = self.xplane_path.clone();
        let custom_scenery_path = custom_scenery_path.clone();

        let index = tokio::task::spawn_blocking(move || -> Result<SceneryIndex> {
            let mut index = index;

            // Track shortcuts by their target path
            // Key: target path, Value: (shortcut_name without .lnk, normalized_target_path for ini)
            let mut shortcut_target_map: HashMap<PathBuf, (String, String)> = HashMap::new();

            // Get current scenery folders (including symlinks and .lnk shortcuts)
            // Key: entry name (shortcut name for shortcuts, folder name for directories)
            // Value: actual path to scan
            let current_folders: HashMap<String, PathBuf> = fs::read_dir(&custom_scenery_path)?
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let path = e.path();

                    // Check if it's a .lnk file (Windows shortcut)
                    if path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
                    {
                        // Try to resolve the shortcut
                        if let Some(target) = resolve_shortcut(&path) {
                            // Use shortcut file name (without .lnk) as the entry name
                            // This prevents conflicts when multiple shortcuts point to folders with the same name
                            let shortcut_name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "<unknown>".to_string());
                            // Track the resolved target path for writing to scenery_packs.ini
                            let target_path_str = target.to_string_lossy().to_string();
                            // Convert backslashes to forward slashes for scenery_packs.ini compatibility
                            let normalized_path_str = target_path_str.replace('\\', "/");
                            shortcut_target_map.insert(
                                target.clone(),
                                (shortcut_name.clone(), normalized_path_str),
                            );
                            return Some((shortcut_name, target));
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
                    // Skip dynamic content packages (e.g., AutoOrtho XPME_* packages)
                    // These packages generate content on-the-fly and their modification time
                    // changes frequently, which would cause unnecessary re-indexing
                    if name.starts_with("XPME_") {
                        // Only update if not in index (new package)
                        return !index.packages.contains_key(*name);
                    }

                    // Check if package is new or modified
                    if let Some(existing) = index.packages.get(*name) {
                        // Recovery path: if a package has library.txt but currently no exported
                        // library names in index, force one re-classification so parser fixes
                        // are applied during incremental scans (e.g., non-UTF8 library.txt).
                        if existing.has_library_txt && existing.exported_library_names.is_empty() {
                            return true;
                        }

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
                logger::log_info(
                    &format!("Updating {} scenery packages", packages_to_update.len()),
                    Some("scenery_index"),
                );

                // Classify updated packages
                // Track which path each package came from to correctly handle shortcuts
                // Use sequential processing in debug log mode for ordered logs, parallel otherwise
                let packages_with_paths: Vec<(PathBuf, SceneryPackageInfo)> =
                    if logger::is_debug_enabled() {
                        // Sequential processing for ordered debug logs
                        packages_to_update
                            .iter()
                            .filter_map(|folder| {
                                classify_scenery(folder, &xplane_path)
                                    .ok()
                                    .map(|info| (folder.clone(), info))
                            })
                            .collect()
                    } else {
                        // Parallel processing for better performance when not in debug mode
                        packages_to_update
                            .par_iter()
                            .filter_map(|folder| {
                                classify_scenery(folder, &xplane_path)
                                    .ok()
                                    .map(|info| (folder.clone(), info))
                            })
                            .collect()
                    };

                for (path, mut info) in packages_with_paths {
                    // Check if this entry came from a shortcut
                    if let Some((shortcut_name, actual_path)) = shortcut_target_map.get(&path) {
                        // Use shortcut name and set actual_path
                        info.folder_name = shortcut_name.clone();
                        info.actual_path = Some(actual_path.clone());
                    }
                    index.packages.insert(info.folder_name.clone(), info);
                }

                // Before recalculate_sort_order, detect airport mesh packages (same as rebuild_index)
                let mut packages_vec: Vec<SceneryPackageInfo> =
                    index.packages.drain().map(|(_, v)| v).collect();
                detect_airport_mesh_packages_with_path(&xplane_path, &mut packages_vec);
                index.packages = packages_vec
                    .into_iter()
                    .map(|info| (info.folder_name.clone(), info))
                    .collect();

                // After adding new packages, recalculate sort_order using the same logic as rebuild_index
                // This ensures incremental updates produce the same ordering as full rebuilds
                Self::recalculate_sort_order(&mut index);
            }

            // Also update actual_path for existing entries that are shortcuts
            // (in case they weren't updated but the shortcut info needs to be preserved)
            for (folder_name, info) in index.packages.iter_mut() {
                // Look up by folder_name in shortcut_target_map values (shortcut names)
                for (_, (shortcut_name, actual_path)) in shortcut_target_map.iter() {
                    if folder_name == shortcut_name {
                        if info.actual_path.is_none()
                            || info.actual_path.as_ref() != Some(actual_path)
                        {
                            info.actual_path = Some(actual_path.clone());
                        }
                        break;
                    }
                }
            }

            index.last_updated = SystemTime::now();

            Ok(index)
        })
        .await
        .map_err(|e| anyhow!("Blocking task failed: {}", e))??;
        let index = self.update_missing_libraries(index).await?;

        Ok(index)
    }

    /// Check if a package needs re-classification
    pub async fn is_package_stale(&self, folder_name: &str, folder_path: &Path) -> Result<bool> {
        let index = self.load_index().await?;
        let folder_name = folder_name.to_string();
        let folder_path = folder_path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            if let Some(existing) = index.packages.get(&folder_name) {
                if let Ok(metadata) = fs::metadata(&folder_path) {
                    if let Ok(modified) = metadata.modified() {
                        return Ok(modified > existing.indexed_at);
                    }
                }
            }

            Ok(true) // Assume stale if we can't determine
        })
        .await
        .map_err(|e| anyhow!("Blocking task failed: {}", e))?
    }

    /// Get package info from index
    pub async fn get_package(&self, folder_name: &str) -> Result<Option<SceneryPackageInfo>> {
        let index = self.load_index().await?;
        Ok(index.packages.get(folder_name).cloned())
    }

    /// Get or classify a package (uses cache if available and not stale)
    pub async fn get_or_classify(&self, folder_path: &Path) -> Result<SceneryPackageInfo> {
        let folder_name = folder_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid folder name"))?;

        // Check if we have a valid cached entry
        if !self.is_package_stale(folder_name, folder_path).await? {
            if let Some(info) = self.get_package(folder_name).await? {
                return Ok(info);
            }
        }

        // Classify and update index
        let folder_path = folder_path.to_path_buf();
        let xplane_path = self.xplane_path.clone();
        let info =
            tokio::task::spawn_blocking(move || classify_scenery(&folder_path, &xplane_path))
                .await
                .map_err(|e| anyhow!("Blocking task failed: {}", e))??;
        self.update_package(info.clone()).await?;
        Ok(info)
    }

    pub async fn index_status(&self) -> Result<SceneryIndexStatus> {
        let total_packages = SceneryQueries::get_package_count(&self.db)
            .await
            .map_err(|e| anyhow!("{}", e))?;
        let index_exists = total_packages > 0;

        Ok(SceneryIndexStatus {
            index_exists,
            total_packages,
        })
    }

    pub async fn quick_scan_and_update(&self) -> Result<SceneryIndexScanResult> {
        let has_packages = SceneryQueries::has_packages(&self.db)
            .await
            .map_err(|e| anyhow!("{}", e))?;

        if !has_packages {
            return Ok(SceneryIndexScanResult {
                index_exists: false,
                added: Vec::new(),
                removed: Vec::new(),
                updated: Vec::new(),
            });
        }

        let before_index = self.load_index().await?;
        let before_keys: HashSet<String> = before_index.packages.keys().cloned().collect();

        let after_index = self.update_index().await?;
        let after_keys: HashSet<String> = after_index.packages.keys().cloned().collect();

        let mut added: Vec<String> = after_keys.difference(&before_keys).cloned().collect();
        added.sort();
        let mut removed: Vec<String> = before_keys.difference(&after_keys).cloned().collect();
        removed.sort();

        // For "updated", compare actual content changes (not just indexed_at timestamp)
        // Some folders get re-indexed every time due to mtime changes from antivirus/sync tools,
        // but their actual classification hasn't changed - don't report those as updates.
        let mut updated: Vec<String> = after_index
            .packages
            .iter()
            .filter(|(name, info)| {
                // Must exist in both before and after (not added/removed)
                if let Some(before_info) = before_index.packages.get(*name) {
                    // Only count as updated if indexed_at changed AND actual content differs
                    info.indexed_at > before_info.indexed_at
                        && (info.category != before_info.category
                            || info.sub_priority != before_info.sub_priority
                            || info.enabled != before_info.enabled
                            || info.has_apt_dat != before_info.has_apt_dat
                            || info.has_dsf != before_info.has_dsf
                            || info.has_library_txt != before_info.has_library_txt
                            || info.missing_libraries != before_info.missing_libraries
                            || info.exported_library_names != before_info.exported_library_names
                            || info.earth_nav_tile_count != before_info.earth_nav_tile_count)
                } else {
                    false
                }
            })
            .map(|(name, _)| name.clone())
            .collect();
        updated.sort();

        Ok(SceneryIndexScanResult {
            index_exists: true,
            added,
            removed,
            updated,
        })
    }

    /// Get index statistics
    pub async fn get_stats(&self) -> Result<SceneryIndexStats> {
        let index = self.load_index().await?;

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

    /// Batch update multiple entries' enabled state and sort_order from UI
    pub async fn batch_update_entries(
        &self,
        entries: &[crate::models::SceneryEntryUpdate],
    ) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        SceneryQueries::batch_update_entries(&self.db, entries)
            .await
            .map_err(|e| anyhow!("{}", e))?;

        logger::log_info(
            &format!("Batch updated {} entries in scenery index", entries.len()),
            Some("scenery_index"),
        );

        Ok(())
    }

    /// Update a single entry's enabled state, sort_order, and/or category
    pub async fn update_entry(
        &self,
        folder_name: &str,
        enabled: Option<bool>,
        sort_order: Option<u32>,
        category: Option<SceneryCategory>,
    ) -> Result<()> {
        SceneryQueries::update_entry(
            &self.db,
            folder_name,
            enabled,
            sort_order,
            category.as_ref(),
        )
        .await
        .map_err(|e| anyhow!("{}", e))?;
        Ok(())
    }

    /// Remove an entry from the index
    pub async fn remove_entry(&self, folder_name: &str) -> Result<()> {
        let deleted = SceneryQueries::delete_package(&self.db, folder_name)
            .await
            .map_err(|e| anyhow!("{}", e))?;

        if deleted {
            logger::log_info(
                &format!("Removed entry from scenery index: {}", folder_name),
                Some("scenery_index"),
            );
        }

        Ok(())
    }

    /// Move an entry from one position to another, auto-adjusting other entries
    pub async fn move_entry(&self, folder_name: &str, new_sort_order: u32) -> Result<()> {
        let mut index = self.load_index().await?;

        // Get current sort_order
        let current_sort_order = match index.packages.get(folder_name) {
            Some(info) => info.sort_order,
            None => return Err(anyhow!("Package not found: {}", folder_name)),
        };

        // Validate and clamp new_sort_order to valid range [0, packages.len() - 1]
        let max_valid_order = index.packages.len().saturating_sub(1) as u32;
        let new_sort_order = new_sort_order.min(max_valid_order);

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
        self.save_index(&index).await?;

        Ok(())
    }

    /// Reset sort_order for all packages based on category priority
    /// This recalculates the sort order using the classification algorithm
    /// without writing to the ini file
    /// Returns true if the sort order was changed, false if it was already correct
    pub async fn reset_sort_order(&self) -> Result<bool> {
        let mut index = self.load_index().await?;

        if index.packages.is_empty() {
            return Ok(false);
        }

        // Promote SAM libraries to FixedHighPriority before sorting
        let mut category_changed = false;
        for (name, info) in index.packages.iter_mut() {
            if is_sam_folder_name(name)
                && info.has_library_txt
                && !info.has_dsf
                && !info.has_apt_dat
                && info.category != SceneryCategory::FixedHighPriority
            {
                info.category = SceneryCategory::FixedHighPriority;
                info.sub_priority = 0;
                category_changed = true;
            }
        }

        // Preserve FixedHighPriority order, but keep SAM entries at the top
        let mut fixed_packages: Vec<(&String, &SceneryPackageInfo)> = index
            .packages
            .iter()
            .filter(|(_, info)| info.category == SceneryCategory::FixedHighPriority)
            .collect();

        fixed_packages.sort_by(|(name_a, info_a), (name_b, info_b)| {
            let sam_a = is_sam_folder_name(name_a);
            let sam_b = is_sam_folder_name(name_b);
            match sam_b.cmp(&sam_a) {
                std::cmp::Ordering::Equal => {}
                other => return other,
            }

            match info_a.sort_order.cmp(&info_b.sort_order) {
                std::cmp::Ordering::Equal => name_a.to_lowercase().cmp(&name_b.to_lowercase()),
                other => other,
            }
        });

        let mut other_packages: Vec<(&String, &SceneryPackageInfo)> = index
            .packages
            .iter()
            .filter(|(_, info)| info.category != SceneryCategory::FixedHighPriority)
            .collect();

        other_packages.sort_by(|(name_a, info_a), (name_b, info_b)| {
            compare_packages_for_sorting(name_a, info_a, name_b, info_b)
        });

        // Update sort_order based on sorted position and check for changes
        let sorted_names: Vec<String> = fixed_packages
            .iter()
            .map(|(name, _)| (*name).clone())
            .chain(other_packages.iter().map(|(name, _)| (*name).clone()))
            .collect();
        let mut has_changes = category_changed;

        for (new_order, folder_name) in sorted_names.iter().enumerate() {
            if let Some(info) = index.packages.get_mut(folder_name) {
                let new_order_u32 = new_order as u32;
                if info.sort_order != new_order_u32 {
                    has_changes = true;
                    info.sort_order = new_order_u32;
                }
            }
        }

        if has_changes {
            index.last_updated = SystemTime::now();
            self.save_index(&index).await?;

            logger::log_info(
                &format!("Reset sort order for {} packages", sorted_names.len()),
                Some("scenery_index"),
            );
        } else {
            logger::log_info(
                "Sort order is already correct, no changes needed",
                Some("scenery_index"),
            );
        }

        Ok(has_changes)
    }

    /// Get scenery manager data for UI
    pub async fn get_manager_data(&self) -> Result<SceneryManagerData> {
        let index = self.load_index().await?;

        // Check if ini is synced with index
        let packs_manager = crate::scenery_packs_manager::SceneryPacksManager::new(
            &self.xplane_path,
            self.db.clone(),
        );
        let needs_sync = !packs_manager.is_synced_with_index().await.unwrap_or(true);

        // Detect duplicate tiles within Mesh and AirportMesh categories
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        let raw_tile_overlaps = detect_raw_tile_overlaps(&index.packages, &custom_scenery_path);
        let duplicate_tiles_map =
            filter_tile_overlaps_with_xpme_rules(&raw_tile_overlaps, &index.packages);

        // Detect duplicate airports (same airport_id across multiple packages)
        let duplicate_airports_map = detect_duplicate_airports(&index.packages);

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
                continent: info.continent.clone(),
                duplicate_tiles: duplicate_tiles_map
                    .get(&info.folder_name)
                    .cloned()
                    .unwrap_or_default(),
                duplicate_airports: duplicate_airports_map
                    .get(&info.folder_name)
                    .cloned()
                    .unwrap_or_default(),
                airport_id: info.airport_id.clone(),
                original_category: info.original_category.clone(),
            })
            .collect();

        // Sort by sort_order
        entries.sort_by_key(|e| e.sort_order);

        // Calculate statistics
        let total_count = entries.len();
        let enabled_count = entries.iter().filter(|e| e.enabled).count();
        let missing_deps_count = entries
            .iter()
            .filter(|e| !e.missing_libraries.is_empty())
            .count();
        let duplicate_tiles_count = entries
            .iter()
            .filter(|e| !e.duplicate_tiles.is_empty())
            .count();
        let duplicate_airports_count = entries
            .iter()
            .filter(|e| !e.duplicate_airports.is_empty())
            .count();

        Ok(SceneryManagerData {
            entries,
            total_count,
            enabled_count,
            missing_deps_count,
            duplicate_tiles_count,
            duplicate_airports_count,
            needs_sync,
            tile_overlaps: raw_tile_overlaps,
        })
    }

    /// Create an empty index
    fn create_empty_index(&self) -> SceneryIndex {
        SceneryIndex {
            version: CURRENT_SCHEMA_VERSION as u32,
            packages: HashMap::new(),
            last_updated: SystemTime::now(),
        }
    }
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
    // Use lossy UTF-8 decoding so ANSI / non-UTF8 comments don't abort parsing.
    if let Ok(bytes) = fs::read(library_txt_path) {
        let content = String::from_utf8_lossy(&bytes);

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Format: EXPORT <virtual_path> <actual_path>
            //     or EXPORT_EXTEND <virtual_path> <actual_path>
            let mut parts = trimmed.split_whitespace();
            let directive = match parts.next() {
                Some(value) => value,
                None => continue,
            };

            if !directive.eq_ignore_ascii_case("EXPORT")
                && !directive.eq_ignore_ascii_case("EXPORT_EXTEND")
            {
                continue;
            }

            let virtual_path = match parts.next() {
                Some(value) => value,
                None => continue,
            };

            // Extract first path component (library name)
            // Support both forward slash and backslash
            if let Some(component) = virtual_path.split(&['/', '\\'][..]).next() {
                if !component.is_empty() {
                    library_names.insert(component.to_string());
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
                library_index.insert(lib_name.to_lowercase(), folder_name.clone());
            }
        }
    }

    logger::log_info(
        &format!(
            "Built library index from scenery index with {} entries",
            library_index.len()
        ),
        Some("library_index"),
    );

    library_index
}

/// Remove a scenery entry from the index (public helper function)
pub async fn remove_scenery_entry(
    db: &DatabaseConnection,
    xplane_path: &str,
    folder_name: &str,
) -> Result<()> {
    let manager = SceneryIndexManager::new(Path::new(xplane_path), db.clone());
    manager.remove_entry(folder_name).await
}

/// Parse airport apt.dat to extract coordinates and ICAO code
/// Returns (latitude_floor, longitude_floor, Option<icao_code>)
/// Tries datum_lat/datum_lon first, falls back to runway coordinates
fn parse_airport_coords(scenery_path: &Path) -> Option<(i32, i32, Option<String>)> {
    // Find apt.dat file
    let apt_dat_path = scenery_path.join("Earth nav data").join("apt.dat");
    if !apt_dat_path.exists() {
        return None;
    }

    let file = fs::File::open(&apt_dat_path).ok()?;
    let reader = BufReader::new(file);

    let mut datum_lat: Option<f64> = None;
    let mut datum_lon: Option<f64> = None;
    let mut icao_code: Option<String> = None;
    let mut runway_lat: Option<f64> = None;
    let mut runway_lon: Option<f64> = None;

    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();

        // Look for 1302 metadata lines
        if trimmed.starts_with("1302 ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                match parts[1] {
                    "datum_lat" => {
                        if let Ok(lat) = parts[2].parse::<f64>() {
                            // Validate latitude range
                            if (-90.0..=90.0).contains(&lat) {
                                datum_lat = Some(lat);
                            }
                        }
                    }
                    "datum_lon" => {
                        if let Ok(lon) = parts[2].parse::<f64>() {
                            // Validate longitude range
                            if (-180.0..=180.0).contains(&lon) {
                                datum_lon = Some(lon);
                            }
                        }
                    }
                    "icao_code" => {
                        icao_code = Some(parts[2].to_string());
                    }
                    _ => {}
                }
            }
        }
        // Fallback: parse runway line (row code 100) for coordinates
        // Format: 100 width surface shoulder smoothness centerline edge autosign runway_number lat lon ...
        // See X-Plane apt.dat specification for full format
        else if trimmed.starts_with("100 ") && runway_lat.is_none() {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            // Runway line requires at least 11 parts to have lat/lon at indices 9 and 10
            if parts.len() >= 11 {
                if let (Ok(lat), Ok(lon)) = (parts[9].parse::<f64>(), parts[10].parse::<f64>()) {
                    // Validate coordinate ranges to catch malformed data
                    // Latitude: -90 to 90, Longitude: -180 to 180
                    if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
                        runway_lat = Some(lat);
                        runway_lon = Some(lon);
                    }
                }
            }
        }

        // If we have datum coords and icao, we can stop early
        if datum_lat.is_some() && datum_lon.is_some() && icao_code.is_some() {
            break;
        }
    }

    // Prefer datum coordinates, fall back to runway coordinates
    let (lat, lon) = match (datum_lat, datum_lon) {
        (Some(lat), Some(lon)) => (lat, lon),
        _ => match (runway_lat, runway_lon) {
            (Some(lat), Some(lon)) => (lat, lon),
            _ => return None,
        },
    };

    // Floor the coordinates to get the tile
    let lat_floor = lat.floor() as i32;
    let lon_floor = lon.floor() as i32;
    Some((lat_floor, lon_floor, icao_code))
}

/// Detect and reclassify mesh packages that are associated with airports
/// Uses an explicit X-Plane path to avoid borrowing &self in blocking contexts.
fn detect_airport_mesh_packages_with_path(xplane_path: &Path, packages: &mut [SceneryPackageInfo]) {
    logger::log_info(
        "Detecting airport-associated mesh packages...",
        Some("scenery_index"),
    );

    // Step 1: Collect all airports with their coordinates, ICAO codes, and folder names
    let mut airport_coords: AirportCoords = HashMap::new();

    // Also collect airport folder name prefixes for prefix matching
    let mut airport_prefixes: HashSet<String> = HashSet::new();

    for pkg in packages.iter() {
        if pkg.category == SceneryCategory::Airport && pkg.has_apt_dat {
            // Parse apt.dat to get coordinates and ICAO code
            let scenery_path = xplane_path.join("Custom Scenery").join(&pkg.folder_name);
            if let Some((lat, lon, icao)) = parse_airport_coords(&scenery_path) {
                let coord_key = (lat, lon);
                airport_coords
                    .entry(coord_key)
                    .or_default()
                    .push((pkg.folder_name.clone(), icao));

                // Extract common prefix (e.g., "ACS_Singapore" from "ACS_Singapore_0_Airport")
                if let Some(prefix) = extract_scenery_prefix(&pkg.folder_name) {
                    airport_prefixes.insert(prefix);
                }
            }
        }
    }

    logger::log_info(
        &format!("Found {} airport coordinate tiles", airport_coords.len()),
        Some("scenery_index"),
    );

    // Step 2: Find mesh packages with small DSF count and matching coordinates
    let custom_scenery_path = xplane_path.join("Custom Scenery");
    let mut mesh_candidates: Vec<(usize, i32, i32)> = Vec::new(); // (package index, lat, lon)

    for (idx, pkg) in packages.iter().enumerate() {
        if pkg.category != SceneryCategory::Mesh {
            continue;
        }

        // Skip Ortho4XP packages - they are regional orthophotos, not airport-specific
        if pkg.folder_name.starts_with("zOrtho4XP") {
            continue;
        }

        let scenery_path = custom_scenery_path.join(&pkg.folder_name);

        // Count DSF files and get their coordinates
        if let Some(dsf_coords) = get_mesh_dsf_coordinates(&scenery_path) {
            // Only consider meshes with 4 or fewer DSF files
            if dsf_coords.len() > 4 {
                continue;
            }

            // Check if any DSF coordinate matches an airport
            for (lat, lon) in &dsf_coords {
                if airport_coords.contains_key(&(*lat, *lon)) {
                    mesh_candidates.push((idx, *lat, *lon));
                    crate::log_debug!(
                        &format!(
                            "  Mesh candidate: {} at ({}, {})",
                            pkg.folder_name, lat, lon
                        ),
                        "scenery_index"
                    );
                }
            }
        }
    }

    // Step 3: Resolve candidates and apply classifications
    for (idx, lat, lon) in mesh_candidates {
        let pkg = &packages[idx];
        if let Some(airports) = airport_coords.get(&(lat, lon)) {
            let mut best_match: Option<(String, Option<String>)> = None;

            // Prefer packages whose folder name contains the ICAO code
            for (airport_folder, icao) in airports {
                if let Some(ref code) = icao {
                    if pkg.folder_name.to_uppercase().contains(code) {
                        best_match = Some((airport_folder.clone(), icao.clone()));
                        break;
                    }
                }
            }

            // If no ICAO match, try prefix matching
            if best_match.is_none() {
                if let Some(prefix) = extract_scenery_prefix(&pkg.folder_name) {
                    if airport_prefixes.contains(&prefix) {
                        best_match = airports.first().cloned();
                    }
                }
            }

            if best_match.is_some() {
                if let Some(pkg) = packages.get_mut(idx) {
                    pkg.category = SceneryCategory::AirportMesh;
                }
            }
        }
    }
}

/// Get DSF file coordinates from a mesh scenery package
/// Returns list of (latitude, longitude) tuples extracted from DSF filenames
fn get_mesh_dsf_coordinates(scenery_path: &Path) -> Option<Vec<(i32, i32)>> {
    let earth_nav_data = scenery_path.join("Earth nav data");
    if !earth_nav_data.exists() {
        return None;
    }

    let mut coordinates: Vec<(i32, i32)> = Vec::new();

    // Iterate through subdirectories (e.g., +30+135)
    if let Ok(entries) = fs::read_dir(&earth_nav_data) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // Look for .dsf files in this directory
            if let Ok(dsf_entries) = fs::read_dir(&path) {
                for dsf_entry in dsf_entries.flatten() {
                    let dsf_path = dsf_entry.path();
                    if let Some(ext) = dsf_path.extension() {
                        if ext.eq_ignore_ascii_case("dsf") {
                            // Parse coordinates from filename (e.g., +30+135.dsf)
                            if let Some(coord) = parse_dsf_filename(&dsf_path) {
                                coordinates.push(coord);
                            }
                        }
                    }
                }
            }
        }
    }

    if coordinates.is_empty() {
        None
    } else {
        Some(coordinates)
    }
}

/// Parse DSF filename to extract coordinates
/// Format: +30+135.dsf or -45-073.dsf (latitude + longitude)
fn parse_dsf_filename(dsf_path: &Path) -> Option<(i32, i32)> {
    let stem = dsf_path.file_stem()?.to_str()?;

    // DSF filenames are in format: [+-]NN[+-]NNN
    // e.g., +30+135, -45-073, +09-079
    if stem.len() < 7 {
        return None;
    }

    // Find the second sign character (start of longitude)
    let chars: Vec<char> = stem.chars().collect();
    let mut lon_start = None;

    for (i, ch) in chars.iter().enumerate().skip(1) {
        if *ch == '+' || *ch == '-' {
            lon_start = Some(i);
            break;
        }
    }

    let lon_start = lon_start?;

    let lat_str = &stem[0..lon_start];
    let lon_str = &stem[lon_start..];

    let lat: i32 = lat_str.parse().ok()?;
    let lon: i32 = lon_str.parse().ok()?;

    Some((lat, lon))
}

/// Detect duplicate airports: packages that define the same airport identifier.
/// Returns a map of folder_name -> list of other folder names sharing the same airport_id.
fn detect_duplicate_airports(
    packages: &HashMap<String, SceneryPackageInfo>,
) -> HashMap<String, Vec<String>> {
    // Build map: airport_id -> Vec<folder_name>
    let mut id_map: HashMap<String, Vec<String>> = HashMap::new();
    for (folder_name, info) in packages {
        if let Some(ref airport_id) = info.airport_id {
            id_map
                .entry(airport_id.clone())
                .or_default()
                .push(folder_name.clone());
        }
    }

    // For each airport_id with >1 package, record overlaps
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    for folders in id_map.values() {
        if folders.len() > 1 {
            for folder in folders {
                let others: Vec<String> =
                    folders.iter().filter(|f| *f != folder).cloned().collect();
                result.entry(folder.clone()).or_default().extend(others);
            }
        }
    }

    // Deduplicate
    for v in result.values_mut() {
        v.sort();
        v.dedup();
    }

    result
}

/// Detect all DSF tile overlaps within Mesh and AirportMesh categories separately.
/// Returns a raw map of folder_name -> list of ALL overlapping folder names (no XPME filtering).
/// Cross-category duplicates (Mesh vs AirportMesh) are NOT flagged.
fn detect_raw_tile_overlaps(
    packages: &HashMap<String, SceneryPackageInfo>,
    custom_scenery_path: &Path,
) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    // Process each category separately
    for category in [SceneryCategory::Mesh, SceneryCategory::AirportMesh] {
        // Build coordinate index: (lat, lon) -> list of folder names
        let mut coord_map: HashMap<(i32, i32), Vec<String>> = HashMap::new();

        for (folder_name, info) in packages.iter() {
            if info.category != category {
                continue;
            }

            // Use actual_path if available (for shortcuts/symlinks), otherwise use folder_name
            let scenery_path = if let Some(ref actual_path) = info.actual_path {
                PathBuf::from(actual_path)
            } else {
                custom_scenery_path.join(folder_name)
            };

            if let Some(coords) = get_mesh_dsf_coordinates(&scenery_path) {
                for coord in coords {
                    coord_map
                        .entry(coord)
                        .or_default()
                        .push(folder_name.clone());
                }
            }
        }

        // For each coordinate with multiple packages, record all overlaps
        for (_coord, folder_names) in coord_map.iter() {
            if folder_names.len() > 1 {
                for folder in folder_names {
                    let others: Vec<String> = folder_names
                        .iter()
                        .filter(|f| *f != folder)
                        .cloned()
                        .collect();

                    if !others.is_empty() {
                        result.entry(folder.clone()).or_default().extend(others);
                    }
                }
            }
        }
    }

    // Deduplicate each folder's overlap list
    for duplicates in result.values_mut() {
        duplicates.sort();
        duplicates.dedup();
    }

    result
}

/// Apply XPME filtering rules to raw tile overlaps based on current sort_order.
/// - XPME_ packages never receive duplicate warnings themselves.
/// - Non-XPME packages are only warned about XPME overlap when they are
///   sorted below (higher sort_order = lower priority than) the XPME package.
fn filter_tile_overlaps_with_xpme_rules(
    raw_overlaps: &HashMap<String, Vec<String>>,
    packages: &HashMap<String, SceneryPackageInfo>,
) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    for (folder, overlaps) in raw_overlaps {
        // XPME_ packages never get flagged
        if folder.starts_with("XPME_") {
            continue;
        }

        let folder_sort = packages
            .get(folder.as_str())
            .map(|i| i.sort_order)
            .unwrap_or(u32::MAX);

        let filtered: Vec<String> = overlaps
            .iter()
            .filter(|other| {
                if other.starts_with("XPME_") {
                    let xpme_sort = packages
                        .get(other.as_str())
                        .map(|i| i.sort_order)
                        .unwrap_or(0);
                    return folder_sort > xpme_sort;
                }
                true
            })
            .cloned()
            .collect();

        if !filtered.is_empty() {
            result.insert(folder.clone(), filtered);
        }
    }

    result
}

/// Extract scenery package naming prefix for matching related packages
/// Examples:
///   "ACS_Singapore_0_Airport" -> "ACS_Singapore"
///   "ACS_Singapore_3_Orthos" -> "ACS_Singapore"
///   "Taimodels_WSSS_Singapore_Changi-MESH" -> "Taimodels_WSSS_Singapore_Changi"
///   "FlyTampa_Amsterdam_3_mesh" -> "FlyTampa_Amsterdam"
/// The prefix is the part before "_<number>_" pattern (if found)
fn extract_scenery_prefix(folder_name: &str) -> Option<String> {
    // Look for pattern: prefix_<number>_suffix
    // We want to extract "prefix" part
    let parts: Vec<&str> = folder_name.split('_').collect();

    // Need at least 3 parts to have "prefix_number_suffix" pattern
    if parts.len() >= 3 {
        // Find index of numeric part
        for i in 1..parts.len() - 1 {
            if parts[i].chars().all(|c| c.is_ascii_digit()) && !parts[i].is_empty() {
                // Found numeric part, prefix is everything before it
                let prefix = parts[..i].join("_");
                if !prefix.is_empty() {
                    return Some(prefix);
                }
            }
        }
    }

    // Fallback: if no "_<number>_" pattern found, try to extract meaningful prefix
    // by taking everything before common suffixes like "-MESH", "_Mesh", "_Orthos", "_Airport"
    let folder_lower = folder_name.to_lowercase();
    let suffixes = ["-mesh", "_mesh", "_orthos", "_orthophoto", "_airport"];

    for suffix in suffixes {
        if let Some(pos) = folder_lower.rfind(suffix) {
            let prefix = &folder_name[..pos];
            // Strip trailing underscore or dash if present
            let prefix = prefix.trim_end_matches(['_', '-']);
            if !prefix.is_empty() {
                return Some(prefix.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_index_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db = crate::database::open_memory_connection().unwrap();
        crate::database::apply_migrations(&db).unwrap();
        let manager = SceneryIndexManager::new(temp_dir.path(), db);
        let index = manager.create_empty_index();

        assert_eq!(index.version, CURRENT_SCHEMA_VERSION as u32);
        assert!(index.packages.is_empty());
    }

    #[test]
    fn test_extract_scenery_prefix() {
        // Test "_<number>_" pattern extraction
        assert_eq!(
            extract_scenery_prefix("ACS_Singapore_0_Airport"),
            Some("ACS_Singapore".to_string())
        );
        assert_eq!(
            extract_scenery_prefix("ACS_Singapore_3_Orthos"),
            Some("ACS_Singapore".to_string())
        );
        assert_eq!(
            extract_scenery_prefix("FlyTampa_Amsterdam_3_mesh"),
            Some("FlyTampa_Amsterdam".to_string())
        );

        // Test suffix-based extraction fallback
        assert_eq!(
            extract_scenery_prefix("Taimodels_WSSS_Singapore_Changi-MESH"),
            Some("Taimodels_WSSS_Singapore_Changi".to_string())
        );

        // Test names without patterns
        assert_eq!(extract_scenery_prefix("SimpleFolder"), None);
    }
}

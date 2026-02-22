use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::livery_patterns;
use crate::logger;
use crate::models::{
    AddonType, DetectedItem, ExtractionChain, NavdataCycle, NavdataInfo, NestedArchiveInfo,
};

#[path = "rar.rs"]
mod rar;
#[path = "sevenz.rs"]
mod sevenz;
#[path = "zip.rs"]
mod zip;

/// Error indicating that password is required for an encrypted archive
#[derive(Debug)]
pub struct PasswordRequiredError {
    pub archive_path: String,
}

impl std::fmt::Display for PasswordRequiredError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Password required for archive: {}", self.archive_path)
    }
}

impl std::error::Error for PasswordRequiredError {}

/// Error indicating that password is required for a nested archive
#[derive(Debug)]
pub struct NestedPasswordRequiredError {
    pub parent_archive: String,
    pub nested_archive: String,
}

impl std::fmt::Display for NestedPasswordRequiredError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Password required for nested archive: {} inside {}",
            self.nested_archive, self.parent_archive
        )
    }
}

impl std::error::Error for NestedPasswordRequiredError {}

/// Context for nested archive scanning
pub(super) struct ScanContext {
    /// Current nesting depth (0 = top level, 1 = nested once, 2 = max)
    depth: u8,
    /// Maximum allowed depth (2 levels: archive → archive → addon)
    max_depth: u8,
    /// Chain of parent archives (for building ExtractionChain)
    parent_chain: Vec<NestedArchiveInfo>,
    /// Password map for archives (key: archive path, value: password)
    passwords: HashMap<String, String>,
}

impl ScanContext {
    fn new() -> Self {
        Self {
            depth: 0,
            max_depth: 2,
            parent_chain: Vec::new(),
            passwords: HashMap::new(),
        }
    }

    fn can_recurse(&self) -> bool {
        self.depth < self.max_depth
    }

    fn push_archive(&mut self, info: NestedArchiveInfo) {
        self.parent_chain.push(info);
        self.depth += 1;
    }

    fn pop_archive(&mut self) {
        self.parent_chain.pop();
        self.depth = self.depth.saturating_sub(1);
    }

    /// Get password for a nested archive by checking the password map
    /// Tries multiple key formats: full path, nested path, and filename
    fn get_nested_password(&self, parent_path: &str, nested_path: &str) -> Option<String> {
        // Try full nested path: "parent.zip/nested.zip"
        let full_key = format!("{}/{}", parent_path, nested_path);
        if let Some(pwd) = self.passwords.get(&full_key) {
            return Some(pwd.clone());
        }

        // Try just the nested path
        if let Some(pwd) = self.passwords.get(nested_path) {
            return Some(pwd.clone());
        }

        // Try just the filename
        if let Some(filename) = Path::new(nested_path).file_name() {
            if let Some(filename_str) = filename.to_str() {
                if let Some(pwd) = self.passwords.get(filename_str) {
                    return Some(pwd.clone());
                }
            }
        }

        None
    }
}

struct NestedZipScanParams<'a> {
    parent_archive: &'a mut ::zip::ZipArchive<fs::File>,
    file_index: usize,
    nested_path: &'a str,
    parent_path: &'a Path,
    ctx: &'a mut ScanContext,
    parent_password: Option<&'a [u8]>,
    is_encrypted: bool,
}

/// Check if a filename is an archive file
fn is_archive_file(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".zip") || lower.ends_with(".7z") || lower.ends_with(".rar")
}

/// Get archive format from filename
fn get_archive_format(filename: &str) -> Option<String> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".zip") {
        Some("zip".to_string())
    } else if lower.ends_with(".7z") {
        Some("7z".to_string())
    } else if lower.ends_with(".rar") {
        Some("rar".to_string())
    } else {
        None
    }
}

/// Scans a directory or archive and detects addon types based on markers
///
/// Scanner is thread-safe as it contains no mutable state.
/// All methods are stateless and can be called concurrently.
///
/// Note: Scanner automatically implements Send + Sync as it's an empty struct
/// with no internal state, so no unsafe impl is needed.
pub struct Scanner;

impl Scanner {
    pub fn new() -> Self {
        Scanner
    }

    /// Check if a path should be ignored during scanning
    fn should_ignore_path(path: &Path) -> bool {
        // Check each component of the path
        for component in path.components() {
            if let Some(name) = component.as_os_str().to_str() {
                // Ignore __MACOSX folders (macOS metadata)
                if name == "__MACOSX" {
                    return true;
                }
                // Ignore .DS_Store files (macOS metadata)
                if name == ".DS_Store" {
                    return true;
                }
                // Ignore Thumbs.db (Windows thumbnail cache)
                if name == "Thumbs.db" {
                    return true;
                }
                // Ignore desktop.ini (Windows folder settings)
                if name == "desktop.ini" {
                    return true;
                }
            }
        }
        false
    }

    /// Fast check for archive paths (string-based, avoids Path allocation)
    #[inline]
    fn should_ignore_archive_path(path: &str) -> bool {
        path.contains("__MACOSX")
            || path.contains(".DS_Store")
            || path.ends_with("Thumbs.db")
            || path.ends_with("desktop.ini")
    }

    /// Check if a path should be skipped based on skip_dirs
    /// Optimized: O(1) average case with HashSet lookup for exact matches,
    /// O(n) worst case for prefix checking (but only when needed)
    #[inline]
    fn should_skip_path(path: &Path, skip_dirs: &HashSet<PathBuf>) -> bool {
        // Fast path: check if path is exactly in skip_dirs
        if skip_dirs.contains(path) {
            return false; // Don't skip the root itself, only its children
        }

        // Check if path is a child of any skip_dir
        // This is still O(n) but we can optimize by checking ancestors
        for ancestor in path.ancestors().skip(1) {
            if skip_dirs.contains(ancestor) {
                return true;
            }
        }

        false
    }

    /// Check if a file path is inside any plugin directory
    /// Optimized: Uses path ancestors for efficient checking
    #[inline]
    fn is_path_inside_plugin_dirs(file_path: &Path, plugin_dirs: &HashSet<PathBuf>) -> bool {
        // Check each ancestor to see if it's a plugin directory
        for ancestor in file_path.ancestors().skip(1) {
            if plugin_dirs.contains(ancestor) {
                return true;
            }
        }
        false
    }

    /// Check if a string path is inside any plugin directory (for archive paths)
    /// Optimized: Direct string prefix checking with HashSet
    #[inline]
    fn is_archive_path_inside_plugin_dirs(file_path: &str, plugin_dirs: &HashSet<String>) -> bool {
        // Check if file_path starts with any plugin_dir
        // Since plugin_dirs is typically small, this is acceptable
        for plugin_dir in plugin_dirs {
            if file_path.starts_with(plugin_dir) && file_path.len() > plugin_dir.len() {
                return true;
            }
        }
        false
    }

    /// Check if a file path is inside any aircraft directory (for filesystem paths)
    /// Used to skip .xpl files that are embedded inside aircraft packages
    #[inline]
    fn is_path_inside_aircraft_dirs(file_path: &Path, aircraft_dirs: &HashSet<PathBuf>) -> bool {
        for ancestor in file_path.ancestors().skip(1) {
            if aircraft_dirs.contains(ancestor) {
                return true;
            }
        }
        false
    }

    /// Check if a string path is inside any aircraft directory (for archive paths)
    /// Used to skip .xpl files that are embedded inside aircraft packages
    #[inline]
    fn is_archive_path_inside_aircraft_dirs(
        file_path: &str,
        aircraft_dirs: &HashSet<String>,
    ) -> bool {
        for aircraft_dir in aircraft_dirs {
            let prefix = if aircraft_dir.ends_with('/') {
                aircraft_dir.clone()
            } else {
                format!("{}/", aircraft_dir)
            };
            if file_path.starts_with(&prefix) {
                return true;
            }
        }
        false
    }

    /// Get marker type priority (lower number = higher priority)
    /// Aircraft (.acf) has highest priority to ensure plugins inside aircraft are skipped
    /// LuaScript has lowest priority to ensure lua files inside other addons are skipped
    #[inline]
    fn marker_type_priority(marker_type: &str) -> u8 {
        match marker_type {
            "acf" => 0,     // Aircraft - highest priority
            "library" => 1, // Scenery library
            "dsf" => 2,     // Scenery DSF
            "navdata" => 3, // Navigation data
            "xpl" => 4,     // Plugin
            "livery" => 5,  // Livery
            "lua" => 6,     // Lua script - lowest priority
            _ => 7,
        }
    }

    /// Scan a path (file or directory) and detect all addon types
    pub fn scan_path(&self, path: &Path, password: Option<&str>) -> Result<Vec<DetectedItem>> {
        let original_input_path = path.to_string_lossy().to_string();
        let mut ctx = ScanContext::new();
        if let Some(pwd) = password {
            ctx.passwords
                .insert(path.to_string_lossy().to_string(), pwd.to_string());
        }
        let mut items = self.scan_path_with_context(path, &mut ctx)?;

        // Set original_input_path for all detected items
        for item in &mut items {
            item.original_input_path = original_input_path.clone();
        }

        Ok(items)
    }

    /// Internal method: Scan a path with context (supports nested archives)
    pub(super) fn scan_path_with_context(
        &self,
        path: &Path,
        ctx: &mut ScanContext,
    ) -> Result<Vec<DetectedItem>> {
        let mut detected_items = Vec::new();

        if path.is_dir() {
            detected_items.extend(self.scan_directory(path)?);
        } else if path.is_file() {
            // Check if it's a standalone .lua file
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                if let Some(item) = self.check_lua_script(path, path)? {
                    detected_items.push(item);
                }
            } else {
                detected_items.extend(self.scan_archive_with_context(path, ctx)?);
            }
        }

        Ok(detected_items)
    }

    /// Internal method: Scan an archive with context (routes to format-specific scanners)
    fn scan_archive_with_context(
        &self,
        archive_path: &Path,
        ctx: &mut ScanContext,
    ) -> Result<Vec<DetectedItem>> {
        let extension = archive_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let password = ctx
            .passwords
            .get(&archive_path.to_string_lossy().to_string())
            .cloned(); // Clone the password to avoid borrow issues

        match extension.as_str() {
            "zip" => self.scan_zip_with_context(archive_path, ctx, password.as_deref()),
            "7z" => self.scan_7z_with_context(archive_path, ctx, password.as_deref()),
            "rar" => self.scan_rar_with_context(archive_path, ctx, password.as_deref()),
            _ => Ok(Vec::new()),
        }
    }

    /// Scan a directory using breadth-first (level-by-level) traversal
    /// When a marker file is found, the entire addon root directory is skipped
    fn scan_directory(&self, dir: &Path) -> Result<Vec<DetectedItem>> {
        use std::collections::VecDeque;

        let mut detected = Vec::new();
        let mut plugin_dirs: HashSet<PathBuf> = HashSet::new();
        let mut aircraft_dirs: HashSet<PathBuf> = HashSet::new();
        let mut skip_dirs: HashSet<PathBuf> = HashSet::new();

        // Queue for breadth-first traversal: (directory_path, current_depth)
        let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
        queue.push_back((dir.to_path_buf(), 0));

        const MAX_DEPTH: usize = 15;

        while let Some((current_dir, depth)) = queue.pop_front() {
            if depth > MAX_DEPTH {
                continue;
            }

            // Check if this directory should be skipped
            if Self::should_skip_path(&current_dir, &skip_dirs) {
                continue;
            }

            // Skip ignored paths
            if Self::should_ignore_path(&current_dir) {
                continue;
            }

            // Read directory entries
            let entries = match fs::read_dir(&current_dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            // Separate files and subdirectories
            let mut files: Vec<PathBuf> = Vec::new();
            let mut subdirs: Vec<PathBuf> = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();

                // Skip ignored paths
                if Self::should_ignore_path(&path) {
                    continue;
                }

                if path.is_file() {
                    files.push(path);
                } else if path.is_dir() {
                    subdirs.push(path);
                }
            }

            // First pass on files: identify plugin directories and aircraft directories
            for file_path in &files {
                let file_ext = file_path.extension().and_then(|s| s.to_str());

                if file_ext == Some("xpl") {
                    if let Some(parent) = file_path.parent() {
                        let parent_name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");

                        // Check if parent is platform-specific folder
                        let plugin_root = if matches!(
                            parent_name,
                            "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
                        ) {
                            parent.parent().unwrap_or(parent).to_path_buf()
                        } else {
                            parent.to_path_buf()
                        };

                        plugin_dirs.insert(plugin_root);
                    }
                } else if file_ext == Some("acf") {
                    // Track aircraft directories to skip embedded plugins
                    if let Some(parent) = file_path.parent() {
                        aircraft_dirs.insert(parent.to_path_buf());
                    }
                }
            }

            // Second pass on files: detect addons
            for file_path in &files {
                // Check if inside a detected plugin directory
                let is_inside_plugin = Self::is_path_inside_plugin_dirs(file_path, &plugin_dirs);
                // Check if inside a detected aircraft directory
                let is_inside_aircraft =
                    Self::is_path_inside_aircraft_dirs(file_path, &aircraft_dirs);

                // Check if inside a skip directory
                if Self::should_skip_path(file_path, &skip_dirs) {
                    continue;
                }

                let file_ext = file_path.extension().and_then(|s| s.to_str());

                // Skip .acf/.dsf files inside plugin directories
                if (file_ext == Some("acf") || file_ext == Some("dsf")) && is_inside_plugin {
                    continue;
                }

                // Skip .xpl files inside aircraft directories (embedded plugins)
                if file_ext == Some("xpl") && is_inside_aircraft {
                    continue;
                }

                // Check for addon markers
                if let Some(item) = self.check_aircraft(file_path, dir)? {
                    let root = PathBuf::from(&item.path);
                    skip_dirs.insert(root);
                    detected.push(item);
                    continue;
                }

                if let Some(item) = self.check_scenery(file_path, dir)? {
                    let root = PathBuf::from(&item.path);
                    skip_dirs.insert(root);
                    detected.push(item);
                    continue;
                }

                if let Some(item) = self.check_plugin(file_path, dir)? {
                    let root = PathBuf::from(&item.path);
                    skip_dirs.insert(root);
                    detected.push(item);
                    continue;
                }

                if let Some(item) = self.check_navdata(file_path, dir)? {
                    let root = PathBuf::from(&item.path);
                    skip_dirs.insert(root);
                    detected.push(item);
                    continue;
                }

                if let Some(item) = self.check_livery(file_path, dir)? {
                    let root = PathBuf::from(&item.path);
                    skip_dirs.insert(root);
                    detected.push(item);
                    continue;
                }

                // Lua script detection (lowest priority - only standalone .lua files)
                if let Some(item) = self.check_lua_script(file_path, dir)? {
                    // Don't add to skip_dirs - lua files are single files, not directories
                    detected.push(item);
                    continue;
                }
            }

            // Add subdirectories to queue (only if not in skip_dirs)
            for subdir in subdirs {
                if !Self::should_skip_path(&subdir, &skip_dirs) {
                    queue.push_back((subdir, depth + 1));
                }
            }
        }

        Ok(detected)
    }

    /// Scan archive without full extraction
    #[allow(dead_code)]
    fn scan_archive(
        &self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        let extension = archive_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "zip" => self.scan_zip(archive_path, password),
            "7z" => self.scan_7z(archive_path, password),
            "rar" => self.scan_rar(archive_path, password),
            _ => {
                // Silently skip non-archive files (no extension or unsupported format)
                // Return empty result instead of error
                Ok(Vec::new())
            }
        }
    }

    // Type A: Aircraft Detection
    fn check_aircraft(&self, file_path: &Path, root: &Path) -> Result<Option<DetectedItem>> {
        if file_path.extension().and_then(|s| s.to_str()) != Some("acf") {
            return Ok(None);
        }

        // Get the parent directory of the .acf file
        let parent = file_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?;

        // If .acf is in root, use the root folder name
        // Otherwise, use the immediate parent folder
        let mut install_path = if parent == root {
            root.to_path_buf()
        } else {
            parent.to_path_buf()
        };

        // Special case: if the parent folder is named "_TCAS_AI_", go up one more level
        // This is for AI traffic aircraft that are part of a larger aircraft package
        if let Some(parent_name) = install_path.file_name().and_then(|s| s.to_str()) {
            if parent_name == "_TCAS_AI_" {
                if let Some(grandparent) = install_path.parent() {
                    install_path = grandparent.to_path_buf();
                }
            }
        }

        let display_name = install_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Aircraft")
            .to_string();

        // Read version info from the install folder
        let (version, _, _) = crate::management_index::read_version_info_with_url(&install_path);
        let version_info = version.map(|v| crate::models::VersionInfo { version: Some(v) });

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::Aircraft,
            path: install_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: None,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
            version_info,
        }))
    }

    fn detect_aircraft_in_archive(
        &self,
        file_path: &str,
        archive_path: &Path,
    ) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);
        let parent = path.parent();

        // Determine the aircraft root folder inside the archive
        let (display_name, internal_root) = if let Some(mut p) = parent {
            if p.as_os_str().is_empty() {
                // .acf is in archive root, use archive name as display name
                // Internal root is empty (extract all to target)
                (
                    archive_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown Aircraft")
                        .to_string(),
                    None,
                )
            } else {
                // Special case: if the parent folder is named "_TCAS_AI_", go up one more level
                if let Some(parent_name) = p.file_name().and_then(|s| s.to_str()) {
                    if parent_name == "_TCAS_AI_" {
                        if let Some(grandparent) = p.parent() {
                            if !grandparent.as_os_str().is_empty() {
                                p = grandparent;
                            }
                        }
                    }
                }

                // Get the top-level folder in the archive
                let components: Vec<_> = p.components().collect();
                let top_folder = components
                    .first()
                    .map(|c| c.as_os_str().to_string_lossy().to_string());

                // Use parent folder name as display name
                let name = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Aircraft")
                    .to_string();

                (name, top_folder)
            }
        } else {
            (
                archive_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Aircraft")
                    .to_string(),
                None,
            )
        };

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::Aircraft,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
            version_info: None, // Archive version detection requires extraction, done separately
        }))
    }

    // Type B: Scenery Detection
    fn check_scenery(&self, file_path: &Path, _root: &Path) -> Result<Option<DetectedItem>> {
        let file_name = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        if file_name == "library.txt" {
            return self.detect_scenery_by_library(file_path);
        }

        if file_path.extension().and_then(|s| s.to_str()) == Some("dsf") {
            return self.detect_scenery_by_dsf(file_path);
        }

        Ok(None)
    }

    fn detect_scenery_by_library(&self, file_path: &Path) -> Result<Option<DetectedItem>> {
        // Install the immediate folder containing library.txt
        let parent = file_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?;

        let display_name = parent
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Library")
            .to_string();

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::SceneryLibrary,
            path: parent.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: None,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
            version_info: None,
        }))
    }

    fn detect_scenery_by_dsf(&self, file_path: &Path) -> Result<Option<DetectedItem>> {
        // DSF structure: {Scenery}/Earth nav data/{...}/{file}.dsf
        // Search upward for "Earth nav data" folder, then go one more level up
        let install_dir = self.find_scenery_root_from_dsf(file_path);

        if let Some(install_dir) = install_dir {
            let display_name = install_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Scenery")
                .to_string();

            Ok(Some(DetectedItem {
                original_input_path: String::new(),
                addon_type: AddonType::Scenery,
                path: install_dir.to_string_lossy().to_string(),
                display_name,
                archive_internal_root: None,
                extraction_chain: None,
                navdata_info: None,
                livery_aircraft_type: None,
                version_info: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Find scenery root by searching upward for "Earth nav data" folder
    fn find_scenery_root_from_dsf(&self, dsf_path: &Path) -> Option<PathBuf> {
        let mut current = dsf_path.parent()?;

        // Search upward for "Earth nav data" folder (max 20 levels for deeply nested structures)
        for level in 0..20 {
            if let Some(name) = current.file_name().and_then(|s| s.to_str()) {
                if name == "Earth nav data" {
                    // Found it! Go one level up to get scenery root
                    return current.parent().map(|p| p.to_path_buf());
                }
            }

            // Log warning if we're getting deep
            if level == 15 {
                crate::logger::log_info(
                    &format!("Deep directory nesting detected while searching for 'Earth nav data': {:?}", dsf_path),
                    Some("scanner")
                );
            }

            current = current.parent()?;
        }

        None
    }

    fn detect_scenery_library(
        &self,
        file_path: &str,
        archive_path: &Path,
    ) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);
        let parent = path.parent();

        // Get the scenery library folder name (parent of library.txt)
        let (display_name, internal_root) = if let Some(p) = parent {
            if p.as_os_str().is_empty() {
                // library.txt is in archive root
                (
                    archive_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown Library")
                        .to_string(),
                    None,
                )
            } else {
                // The folder containing library.txt is the library root
                let library_root = p.to_string_lossy().to_string();
                let name = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Library")
                    .to_string();

                (name, Some(library_root))
            }
        } else {
            (
                archive_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Library")
                    .to_string(),
                None,
            )
        };

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::SceneryLibrary,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
            version_info: None,
        }))
    }

    fn detect_scenery_dsf(
        &self,
        file_path: &str,
        archive_path: &Path,
    ) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);

        // DSF structure: {Scenery}/Earth nav data/{...}/{file}.dsf
        // Search upward for "Earth nav data" folder, then go one more level up
        let scenery_root = self.find_scenery_root_from_archive_path(&path);

        let (display_name, internal_root) = if let Some(root) = scenery_root {
            if root.as_os_str().is_empty() {
                // Scenery is at archive root level
                (
                    archive_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown Scenery")
                        .to_string(),
                    None,
                )
            } else {
                let root_str = root.to_string_lossy().to_string();
                let name = root
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Scenery")
                    .to_string();

                (name, Some(root_str))
            }
        } else {
            // Couldn't find "Earth nav data" folder, skip this file
            return Ok(None);
        };

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::Scenery,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
            version_info: None,
        }))
    }

    /// Find scenery root from archive path by searching for "Earth nav data"
    fn find_scenery_root_from_archive_path(&self, dsf_path: &Path) -> Option<PathBuf> {
        let mut current = dsf_path.parent()?;

        // Search upward for "Earth nav data" folder (max 20 levels for deeply nested structures)
        for level in 0..20 {
            if let Some(name) = current.file_name().and_then(|s| s.to_str()) {
                if name == "Earth nav data" {
                    // Found it! Go one level up to get scenery root
                    return current.parent().map(|p| p.to_path_buf());
                }
            }

            // Log warning if we're getting deep
            if level == 15 {
                crate::logger::log_info(
                    &format!("Deep directory nesting in archive while searching for 'Earth nav data': {:?}", dsf_path),
                    Some("scanner")
                );
            }

            match current.parent() {
                Some(p) if !p.as_os_str().is_empty() => current = p,
                _ => break,
            }
        }

        None
    }

    // Type C: Plugin Detection
    fn check_plugin(&self, file_path: &Path, _root: &Path) -> Result<Option<DetectedItem>> {
        if file_path.extension().and_then(|s| s.to_str()) != Some("xpl") {
            return Ok(None);
        }

        let parent = file_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?;

        let parent_name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Check if parent is a platform-specific folder
        let install_path = if matches!(
            parent_name,
            "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
        ) {
            // Go up one more level
            parent.parent().unwrap_or(parent).to_path_buf()
        } else {
            parent.to_path_buf()
        };

        let display_name = install_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Plugin")
            .to_string();

        // Read version info from the install folder
        let (version, _, _) = crate::management_index::read_version_info_with_url(&install_path);
        let version_info = version.map(|v| crate::models::VersionInfo { version: Some(v) });

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::Plugin,
            path: install_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: None,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
            version_info,
        }))
    }

    fn detect_plugin_in_archive(
        &self,
        file_path: &str,
        archive_path: &Path,
    ) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);
        let parent = path.parent();

        let (display_name, internal_root) = if let Some(p) = parent {
            let parent_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");

            // Check if parent is platform-specific
            let plugin_root = if matches!(
                parent_name,
                "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
            ) {
                // Go up one more level
                p.parent()
            } else {
                Some(p)
            };

            if let Some(root) = plugin_root {
                if root.as_os_str().is_empty() {
                    (
                        archive_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown Plugin")
                            .to_string(),
                        None,
                    )
                } else {
                    let root_str = root.to_string_lossy().to_string();
                    let name = root
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown Plugin")
                        .to_string();
                    (name, Some(root_str))
                }
            } else {
                ("Unknown Plugin".to_string(), None)
            }
        } else {
            ("Unknown Plugin".to_string(), None)
        };

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::Plugin,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
            version_info: None, // Archive version detection requires extraction, done separately
        }))
    }

    // Type D: Navdata Detection

    /// Known navdata format keywords.
    /// A cycle.json `name` is accepted if it contains any of these strings.
    const NAVDATA_KNOWN_FORMATS: &[&str] = &["X-Plane", "FlightFactor Boeing 777v2"];

    fn is_known_navdata_format(name: &str) -> bool {
        Self::NAVDATA_KNOWN_FORMATS
            .iter()
            .any(|fmt| name.contains(fmt))
    }

    fn check_navdata(&self, file_path: &Path, _root: &Path) -> Result<Option<DetectedItem>> {
        if file_path.file_name().and_then(|s| s.to_str()) != Some("cycle.json") {
            return Ok(None);
        }

        let content = fs::read_to_string(file_path).context("Failed to read cycle.json")?;

        let cycle: NavdataCycle =
            serde_json::from_str(&content).context("Failed to parse cycle.json")?;

        let parent = file_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?;

        // Create NavdataInfo from parsed cycle
        let navdata_info = NavdataInfo {
            name: cycle.name.clone(),
            cycle: cycle.cycle.clone(),
            airac: cycle.airac.clone(),
        };

        // Validate navdata format
        if !Self::is_known_navdata_format(&cycle.name) {
            return Err(anyhow::anyhow!("Unknown Navdata Format: {}", cycle.name));
        }

        let install_path = parent.to_path_buf();
        let display_name = cycle.name.clone();

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::Navdata,
            path: install_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: None,
            extraction_chain: None,
            navdata_info: Some(navdata_info),
            livery_aircraft_type: None,
            version_info: None,
        }))
    }

    fn detect_navdata_in_archive(
        &self,
        file_path: &str,
        content: &str,
        archive_path: &Path,
    ) -> Result<Option<DetectedItem>> {
        let cycle: NavdataCycle =
            serde_json::from_str(content).context("Failed to parse cycle.json")?;

        let path = PathBuf::from(file_path);
        let parent = path.parent();

        // Create NavdataInfo from parsed cycle
        let navdata_info = NavdataInfo {
            name: cycle.name.clone(),
            cycle: cycle.cycle.clone(),
            airac: cycle.airac.clone(),
        };

        // Validate navdata format
        if !Self::is_known_navdata_format(&cycle.name) {
            return Err(anyhow::anyhow!("Unknown Navdata Format: {}", cycle.name));
        }

        let display_name = cycle.name.clone();
        let is_gns430 = cycle.name.contains("GNS430");

        // Get the navdata folder root inside the archive
        // For GNS430: use grandparent (parent's parent) to preserve the folder structure
        // For regular navdata: use parent (direct parent of cycle.json)
        let internal_root = if let Some(p) = parent {
            if p.as_os_str().is_empty() {
                None
            } else if is_gns430 {
                // GNS430: use grandparent directory
                p.parent()
                    .filter(|gp| !gp.as_os_str().is_empty())
                    .map(|gp| gp.to_string_lossy().to_string())
            } else {
                Some(p.to_string_lossy().to_string())
            }
        } else {
            None
        };

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::Navdata,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            extraction_chain: None,
            navdata_info: Some(navdata_info),
            livery_aircraft_type: None,
            version_info: None,
        }))
    }

    // Type E: Livery Detection
    fn check_livery(&self, file_path: &Path, _root: &Path) -> Result<Option<DetectedItem>> {
        let file_path_str = file_path.to_string_lossy();

        // Check if path matches any livery pattern
        if let Some((aircraft_type_id, livery_root)) =
            livery_patterns::check_livery_pattern(&file_path_str)
        {
            let livery_path = if livery_root.is_empty() {
                file_path.to_path_buf()
            } else {
                PathBuf::from(&livery_root)
            };

            let display_name = livery_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Livery")
                .to_string();

            // Get the aircraft name for display
            let aircraft_name = livery_patterns::get_aircraft_name(&aircraft_type_id)
                .unwrap_or_else(|| aircraft_type_id.clone());

            Ok(Some(DetectedItem {
                original_input_path: String::new(),
                addon_type: AddonType::Livery,
                path: livery_path.to_string_lossy().to_string(),
                display_name: format!("{} ({})", display_name, aircraft_name),
                archive_internal_root: None,
                extraction_chain: None,
                navdata_info: None,
                livery_aircraft_type: Some(aircraft_type_id.clone()),
                version_info: None,
            }))
        } else {
            Ok(None)
        }
    }

    fn detect_livery_in_archive(
        &self,
        file_path: &str,
        archive_path: &Path,
    ) -> Result<Option<DetectedItem>> {
        // Check if path matches any livery pattern
        if let Some((aircraft_type_id, livery_root)) =
            livery_patterns::check_livery_pattern(file_path)
        {
            let internal_root = if livery_root.is_empty() {
                None
            } else {
                Some(livery_root.clone())
            };

            let display_name = if livery_root.is_empty() {
                archive_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Livery")
                    .to_string()
            } else {
                Path::new(&livery_root)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Livery")
                    .to_string()
            };

            // Get the aircraft name for display
            let aircraft_name = livery_patterns::get_aircraft_name(&aircraft_type_id)
                .unwrap_or_else(|| aircraft_type_id.clone());

            Ok(Some(DetectedItem {
                original_input_path: String::new(),
                addon_type: AddonType::Livery,
                path: archive_path.to_string_lossy().to_string(),
                display_name: format!("{} ({})", display_name, aircraft_name),
                archive_internal_root: internal_root,
                extraction_chain: None,
                navdata_info: None,
                livery_aircraft_type: Some(aircraft_type_id.clone()),
                version_info: None,
            }))
        } else {
            Ok(None)
        }
    }

    // Type G: Lua Script Detection (for FlyWithLua)
    fn check_lua_script(&self, file_path: &Path, _root: &Path) -> Result<Option<DetectedItem>> {
        // Only detect .lua files
        if file_path.extension().and_then(|s| s.to_str()) != Some("lua") {
            return Ok(None);
        }

        // Get the file name (without extension) as display name
        let display_name = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Script")
            .to_string();

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::LuaScript,
            path: file_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: None,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
            version_info: None,
        }))
    }

    fn detect_lua_script_in_archive(
        &self,
        file_path: &str,
        archive_path: &Path,
    ) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);

        // Get the file name as display name
        let display_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Script")
            .to_string();

        // For archives, the internal root is the parent directory of the lua file (if any)
        let internal_root = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_string_lossy().to_string());

        Ok(Some(DetectedItem {
            original_input_path: String::new(),
            addon_type: AddonType::LuaScript,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            extraction_chain: None,
            navdata_info: None,
            livery_aircraft_type: None,
            version_info: None,
        }))
    }
}

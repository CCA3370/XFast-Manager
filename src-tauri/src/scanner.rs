use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::livery_patterns;
use crate::logger;
use crate::models::{
    AddonType, DetectedItem, ExtractionChain, NavdataCycle, NavdataInfo, NestedArchiveInfo,
};

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
struct ScanContext {
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
            "acf" => 0,      // Aircraft - highest priority
            "library" => 1,  // Scenery library
            "dsf" => 2,      // Scenery DSF
            "navdata" => 3,  // Navigation data
            "xpl" => 4,      // Plugin
            "livery" => 5,   // Livery
            "lua" => 6,      // Lua script - lowest priority
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
    fn scan_path_with_context(
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
                let is_inside_aircraft = Self::is_path_inside_aircraft_dirs(file_path, &aircraft_dirs);

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

    /// Scan a 7z archive with context (supports nested archives)
    /// OPTIMIZED: Single pass to collect both addon markers and nested archives
    fn scan_7z_with_context(
        &self,
        archive_path: &Path,
        ctx: &mut ScanContext,
        password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        let scan_start = std::time::Instant::now();
        crate::log_debug!(
            &format!("[TIMING] 7z scan started: {}", archive_path.display()),
            "scanner_timing"
        );

        // Open archive to read file list (fast, no decompression)
        let open_start = std::time::Instant::now();
        let archive = sevenz_rust2::Archive::open(archive_path)
            .map_err(|e| anyhow::anyhow!("Failed to open 7z archive: {}", e))?;
        crate::log_debug!(
            &format!(
                "[TIMING] 7z open completed in {:.2}ms: {}",
                open_start.elapsed().as_secs_f64() * 1000.0,
                archive_path.display()
            ),
            "scanner_timing"
        );

        // Check for empty archive
        if archive.files.is_empty() {
            logger::log_info(
                &format!("Empty 7z archive: {}", archive_path.display()),
                Some("scanner"),
            );
            return Ok(Vec::new());
        }

        // Check if archive has encrypted files by examining headers
        let has_encrypted_headers = archive
            .files
            .iter()
            .any(|f| f.has_stream() && !f.is_directory());

        // Only do the slow encryption check if no password provided and archive might be encrypted
        if password.is_none() && has_encrypted_headers {
            let encrypt_check_start = std::time::Instant::now();
            crate::log_debug!("[TIMING] 7z encryption check started", "scanner_timing");

            match sevenz_rust2::ArchiveReader::open(archive_path, sevenz_rust2::Password::empty()) {
                Ok(mut reader) => {
                    let mut encryption_detected = false;
                    let _ = reader.for_each_entries(|entry, reader| {
                        if !entry.is_directory() {
                            let mut buf = [0u8; 1];
                            if std::io::Read::read(reader, &mut buf).is_err() {
                                encryption_detected = true;
                            }
                            return Ok(false);
                        }
                        Ok(true)
                    });

                    if encryption_detected {
                        return Err(anyhow::anyhow!(PasswordRequiredError {
                            archive_path: archive_path.to_string_lossy().to_string(),
                        }));
                    }
                }
                Err(e) => {
                    let err_str = format!("{:?}", e);
                    if err_str.contains("password")
                        || err_str.contains("Password")
                        || err_str.contains("encrypted")
                        || err_str.contains("WrongPassword")
                    {
                        return Err(anyhow::anyhow!(PasswordRequiredError {
                            archive_path: archive_path.to_string_lossy().to_string(),
                        }));
                    }
                }
            }

            crate::log_debug!(
                &format!(
                    "[TIMING] 7z encryption check completed in {:.2}ms",
                    encrypt_check_start.elapsed().as_secs_f64() * 1000.0
                ),
                "scanner_timing"
            );
        }

        // SINGLE PASS: collect addon markers AND nested archives
        let enumerate_start = std::time::Instant::now();
        crate::log_debug!(
            &format!(
                "[TIMING] 7z enumeration started: {} files",
                archive.files.len()
            ),
            "scanner_timing"
        );

        let mut plugin_dirs: HashSet<String> = HashSet::new();
        let mut aircraft_dirs: HashSet<String> = HashSet::new();
        let mut marker_files: Vec<(String, &str)> = Vec::new();
        let mut nested_archives: Vec<String> = Vec::new();
        let mut detected_livery_roots: HashSet<String> = HashSet::new();

        for entry in &archive.files {
            let file_path = entry.name().to_string();
            let normalized = file_path.replace('\\', "/");

            if Self::should_ignore_archive_path(&normalized) {
                continue;
            }

            // Check for nested archives (only if we can recurse)
            if ctx.can_recurse() && !entry.is_directory() && is_archive_file(&normalized) {
                nested_archives.push(normalized.clone());
            }

            // Check for livery patterns first (before any potential moves)
            if let Some((_, livery_root)) = livery_patterns::check_livery_pattern(&normalized) {
                if !detected_livery_roots.contains(&livery_root) {
                    detected_livery_roots.insert(livery_root.clone());
                    marker_files.push((normalized.clone(), "livery"));
                }
            }

            // Identify plugin directories, aircraft directories, and marker files
            if normalized.ends_with(".xpl") {
                if let Some(parent) = Path::new(&normalized).parent() {
                    let parent_str = parent.to_string_lossy();
                    let parent_name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");

                    let plugin_root = if matches!(
                        parent_name,
                        "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
                    ) {
                        parent
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(parent_str.to_string())
                    } else {
                        parent_str.to_string()
                    };
                    plugin_dirs.insert(plugin_root);
                }
                marker_files.push((normalized, "xpl"));
            } else if normalized.ends_with(".acf") {
                // Track aircraft directories to skip embedded plugins
                if let Some(parent) = Path::new(&normalized).parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if !parent_str.is_empty() {
                        aircraft_dirs.insert(parent_str);
                    }
                }
                marker_files.push((normalized, "acf"));
            } else if normalized.ends_with("library.txt") {
                marker_files.push((normalized, "library"));
            } else if normalized.ends_with(".dsf") {
                marker_files.push((normalized, "dsf"));
            } else if normalized.ends_with("cycle.json") {
                marker_files.push((normalized, "navdata"));
            } else if normalized.ends_with(".lua") {
                // Lua script detection (lowest priority)
                marker_files.push((normalized, "lua"));
            }
        }

        crate::log_debug!(
            &format!("[TIMING] 7z enumeration completed in {:.2}ms: {} files, {} markers, {} nested archives",
                enumerate_start.elapsed().as_secs_f64() * 1000.0,
                archive.files.len(),
                marker_files.len(),
                nested_archives.len()
            ),
            "scanner_timing"
        );

        // Sort marker files by depth, then by type priority (aircraft first)
        let sort_start = std::time::Instant::now();
        marker_files.sort_by(|a, b| {
            let depth_a = a.0.matches('/').count();
            let depth_b = b.0.matches('/').count();
            match depth_a.cmp(&depth_b) {
                std::cmp::Ordering::Equal => {
                    // Same depth: sort by marker type priority (aircraft first)
                    Self::marker_type_priority(a.1).cmp(&Self::marker_type_priority(b.1))
                }
                other => other,
            }
        });
        crate::log_debug!(
            &format!(
                "[TIMING] 7z marker sorting completed in {:.2}ms",
                sort_start.elapsed().as_secs_f64() * 1000.0
            ),
            "scanner_timing"
        );

        let mut detected = Vec::new();
        let mut skip_prefixes: Vec<String> = Vec::new();

        // Process marker files
        let process_start = std::time::Instant::now();
        crate::log_debug!(
            &format!(
                "[TIMING] 7z marker processing started: {} markers",
                marker_files.len()
            ),
            "scanner_timing"
        );

        for (file_path, marker_type) in marker_files {
            let should_skip = skip_prefixes
                .iter()
                .any(|prefix| file_path.starts_with(prefix));
            if should_skip {
                continue;
            }

            // Skip .acf/.dsf inside plugin directories
            if marker_type == "acf" || marker_type == "dsf" {
                if Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs) {
                    continue;
                }
            }

            // Skip .xpl inside aircraft directories (embedded plugins)
            if marker_type == "xpl" {
                if Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs) {
                    continue;
                }
            }

            let item = match marker_type {
                "acf" => self.detect_aircraft_in_archive(&file_path, archive_path)?,
                "library" => self.detect_scenery_library(&file_path, archive_path)?,
                "dsf" => self.detect_scenery_dsf(&file_path, archive_path)?,
                "xpl" => self.detect_plugin_in_archive(&file_path, archive_path)?,
                "navdata" => {
                    if let Ok(content) = self.read_file_from_7z(archive_path, &file_path, password)
                    {
                        self.detect_navdata_in_archive(&file_path, &content, archive_path)?
                    } else {
                        None
                    }
                }
                "livery" => self.detect_livery_in_archive(&file_path, archive_path)?,
                "lua" => self.detect_lua_script_in_archive(&file_path, archive_path)?,
                _ => None,
            };

            if let Some(item) = item {
                if let Some(ref internal_root) = item.archive_internal_root {
                    let prefix = if internal_root.ends_with('/') {
                        internal_root.clone()
                    } else {
                        format!("{}/", internal_root)
                    };
                    skip_prefixes.push(prefix);
                } else if item.addon_type == AddonType::Aircraft {
                    // Aircraft at archive root: skip all other markers in this archive
                    // by adding an empty prefix that matches everything
                    skip_prefixes.push(String::new());
                }
                detected.push(item);
            }
        }

        crate::log_debug!(
            &format!(
                "[TIMING] 7z marker processing completed in {:.2}ms: {} addons detected",
                process_start.elapsed().as_secs_f64() * 1000.0,
                detected.len()
            ),
            "scanner_timing"
        );

        // Scan nested archives
        if !nested_archives.is_empty() {
            let nested_start = std::time::Instant::now();
            let total_nested = nested_archives.len();

            // Filter out nested archives that are inside already detected addon directories
            let filtered_nested: Vec<_> = nested_archives
                .into_iter()
                .filter(|nested_path| {
                    // Check if this nested archive is inside any detected addon directory
                    let is_inside_addon = skip_prefixes
                        .iter()
                        .any(|prefix| nested_path.starts_with(prefix));
                    !is_inside_addon
                })
                .collect();

            let filtered_count = filtered_nested.len();
            let skipped_count = total_nested - filtered_count;

            crate::log_debug!(
                &format!("[TIMING] 7z nested archive processing started: {} nested archives ({} skipped as inside detected addons)",
                    filtered_count,
                    skipped_count
                ),
                "scanner_timing"
            );

            for nested_path in filtered_nested {
                if Self::should_ignore_path(Path::new(&nested_path)) {
                    continue;
                }

                match self.scan_nested_archive_in_7z(archive_path, &nested_path, ctx, password) {
                    Ok(nested_items) => {
                        detected.extend(nested_items);
                    }
                    Err(e) => {
                        if e.downcast_ref::<PasswordRequiredError>().is_some() {
                            return Err(anyhow::anyhow!(NestedPasswordRequiredError {
                                parent_archive: archive_path.to_string_lossy().to_string(),
                                nested_archive: nested_path.clone(),
                            }));
                        }
                        crate::logger::log_info(
                            &format!("Failed to scan nested archive {}: {}", nested_path, e),
                            Some("scanner"),
                        );
                    }
                }
            }

            crate::log_debug!(
                &format!(
                    "[TIMING] 7z nested archive processing completed in {:.2}ms",
                    nested_start.elapsed().as_secs_f64() * 1000.0
                ),
                "scanner_timing"
            );
        }

        crate::log_debug!(
            &format!(
                "[TIMING] 7z scan completed in {:.2}ms: {} total addons detected",
                scan_start.elapsed().as_secs_f64() * 1000.0,
                detected.len()
            ),
            "scanner_timing"
        );

        Ok(detected)
    }

    /// Scan a nested archive within a 7z file (extract to temp)
    /// Optimized: If nested archive is ZIP, load into memory for faster scanning
    fn scan_nested_archive_in_7z(
        &self,
        parent_path: &Path,
        nested_path: &str,
        ctx: &mut ScanContext,
        parent_password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        use tempfile::Builder;

        // Create temp directory for extraction
        let temp_dir = Builder::new()
            .prefix("xfi_7z_nested_")
            .tempdir()
            .context("Failed to create temp directory")?;

        // Extract using 7z library
        if let Some(pwd) = parent_password {
            let mut reader =
                sevenz_rust2::ArchiveReader::open(parent_path, sevenz_rust2::Password::from(pwd))
                    .map_err(|e| anyhow::anyhow!("Failed to open 7z with password: {}", e))?;
            reader
                .for_each_entries(|entry, reader| {
                    let dest_path = temp_dir.path().join(entry.name());
                    if entry.is_directory() {
                        std::fs::create_dir_all(&dest_path)?;
                    } else {
                        if let Some(parent) = dest_path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        let mut file = std::fs::File::create(&dest_path)?;
                        std::io::copy(reader, &mut file)?;
                    }
                    Ok(true)
                })
                .map_err(|e| anyhow::anyhow!("Failed to extract 7z with password: {}", e))?;
        } else {
            sevenz_rust2::decompress_file(parent_path, temp_dir.path())
                .map_err(|e| anyhow::anyhow!("Failed to extract 7z: {}", e))?;
        }

        // Find the nested archive in extracted files
        let temp_archive_path = temp_dir.path().join(nested_path);

        if !temp_archive_path.exists() {
            // Provide detailed error with directory listing
            let mut available_files = Vec::new();
            if let Ok(entries) = fs::read_dir(temp_dir.path()) {
                for entry in entries.flatten().take(10) {
                    if let Some(name) = entry.file_name().to_str() {
                        available_files.push(name.to_string());
                    }
                }
            }

            return Err(anyhow::anyhow!(
                "Nested archive not found after extraction: {}\nExpected at: {:?}\nAvailable files: {}",
                nested_path,
                temp_archive_path,
                if available_files.is_empty() {
                    "(none)".to_string()
                } else {
                    available_files.join(", ")
                }
            ));
        }

        // Get archive format
        let format = get_archive_format(nested_path)
            .ok_or_else(|| anyhow::anyhow!("Unknown archive format: {}", nested_path))?;

        // Check if this nested archive has its own password
        let nested_password =
            ctx.get_nested_password(&parent_path.to_string_lossy().to_string(), nested_path);

        // Build nested archive info with password if available
        let nested_info = NestedArchiveInfo {
            internal_path: nested_path.to_string(),
            password: nested_password.clone(),
            format: format.clone(),
        };

        // Push to context chain
        ctx.push_archive(nested_info.clone());

        // OPTIMIZATION: If nested archive is ZIP, try to load into memory
        let nested_result = if format == "zip" {
            crate::logger::log_info(
                &format!("Optimizing: Loading nested ZIP from 7z into memory for scanning"),
                Some("scanner"),
            );

            match self.try_scan_zip_from_file_to_memory(&temp_archive_path, parent_path, ctx) {
                Ok(items) => Ok(items),
                Err(e) => {
                    crate::logger::log_info(
                        &format!("Memory optimization failed, using standard scan: {}", e),
                        Some("scanner"),
                    );
                    // Fallback to standard scan
                    self.scan_path_with_context(&temp_archive_path, ctx)
                }
            }
        } else {
            // For non-ZIP, use standard scan
            self.scan_path_with_context(&temp_archive_path, ctx)
        };

        // Pop from context chain
        ctx.pop_archive();

        // Process results
        match nested_result {
            Ok(mut items) => {
                // Update each detected item with extraction chain
                for item in &mut items {
                    let mut chain = ExtractionChain {
                        archives: ctx.parent_chain.clone(),
                        final_internal_root: item.archive_internal_root.clone(),
                    };
                    chain.archives.push(nested_info.clone());
                    item.path = parent_path.to_string_lossy().to_string();
                    item.extraction_chain = Some(chain);
                    item.archive_internal_root = None;

                    // Update display_name to use the nested archive's filename (without extension)
                    // This prevents creating folders like "Scenery/.zip"
                    if let Some(nested_filename) = Path::new(nested_path).file_stem() {
                        if let Some(name_str) = nested_filename.to_str() {
                            item.display_name = name_str.to_string();
                        }
                    }
                }
                Ok(items)
            }
            Err(e) => Err(e),
        }
    }

    /// Try to scan a ZIP file by loading it into memory (optimization)
    fn try_scan_zip_from_file_to_memory(
        &self,
        zip_path: &Path,
        parent_path: &Path,
        ctx: &mut ScanContext,
    ) -> Result<Vec<DetectedItem>> {
        use std::io::{Cursor, Read};
        use zip::ZipArchive;

        // Check file size before loading into memory (limit: 200MB)
        let metadata = fs::metadata(zip_path)?;
        if metadata.len() > crate::installer::MAX_MEMORY_ZIP_SIZE {
            return Err(anyhow::anyhow!(
                "ZIP file too large for memory optimization ({} MB > 200 MB)",
                metadata.len() / 1024 / 1024
            ));
        }

        // Read ZIP file into memory
        let mut zip_data = Vec::new();
        let mut file = fs::File::open(zip_path)?;
        file.read_to_end(&mut zip_data)?;

        // Create in-memory ZIP archive
        let cursor = Cursor::new(zip_data);
        let mut archive = ZipArchive::new(cursor)?;

        // Scan using in-memory method
        self.scan_zip_in_memory(
            &mut archive,
            parent_path,
            ctx,
            zip_path.to_string_lossy().as_ref(),
        )
    }

    /// Scan a RAR archive with context (supports nested archives via temp extraction)
    fn scan_rar_with_context(
        &self,
        archive_path: &Path,
        ctx: &mut ScanContext,
        password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        let scan_start = std::time::Instant::now();
        crate::log_debug!(
            &format!("[TIMING] RAR scan started: {}", archive_path.display()),
            "scanner_timing"
        );

        // First, scan the archive normally for direct addon markers
        let scan_markers_start = std::time::Instant::now();
        let mut detected = self.scan_rar(archive_path, password)?;
        crate::log_debug!(
            &format!(
                "[TIMING] RAR marker scan completed in {:.2}ms: {} addons detected",
                scan_markers_start.elapsed().as_secs_f64() * 1000.0,
                detected.len()
            ),
            "scanner_timing"
        );

        // If we can recurse and there are nested archives, extract and scan them
        if ctx.can_recurse() {
            let nested_enum_start = std::time::Instant::now();
            crate::log_debug!(
                "[TIMING] RAR nested archive enumeration started",
                "scanner_timing"
            );

            // Open archive to list files
            let archive_builder = if let Some(pwd) = password {
                unrar::Archive::with_password(archive_path, pwd)
            } else {
                unrar::Archive::new(archive_path)
            };

            let archive = archive_builder
                .open_for_listing()
                .map_err(|e| anyhow::anyhow!("Failed to open RAR archive: {:?}", e))?;

            // Find nested archives
            let nested_archives: Vec<String> = archive
                .filter_map(|entry| {
                    if let Ok(e) = entry {
                        let name = e.filename.to_string_lossy().to_string();
                        if !e.is_directory() && is_archive_file(&name) {
                            Some(name)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            crate::log_debug!(
                &format!("[TIMING] RAR nested archive enumeration completed in {:.2}ms: {} nested archives found",
                    nested_enum_start.elapsed().as_secs_f64() * 1000.0,
                    nested_archives.len()
                ),
                "scanner_timing"
            );

            // Scan each nested archive
            if !nested_archives.is_empty() {
                let nested_process_start = std::time::Instant::now();
                let total_nested = nested_archives.len();

                // Build skip prefixes from already detected addons
                let skip_prefixes: Vec<String> = detected
                    .iter()
                    .filter_map(|item| {
                        item.archive_internal_root.as_ref().map(|root| {
                            if root.ends_with('/') {
                                root.clone()
                            } else {
                                format!("{}/", root)
                            }
                        })
                    })
                    .collect();

                // Filter out nested archives that are inside already detected addon directories
                let filtered_nested: Vec<_> = nested_archives
                    .into_iter()
                    .filter(|nested_path| {
                        // Check if this nested archive is inside any detected addon directory
                        let is_inside_addon = skip_prefixes
                            .iter()
                            .any(|prefix| nested_path.starts_with(prefix));
                        !is_inside_addon
                    })
                    .collect();

                let filtered_count = filtered_nested.len();
                let skipped_count = total_nested - filtered_count;

                crate::log_debug!(
                    &format!("[TIMING] RAR nested archive processing started: {} nested archives ({} skipped as inside detected addons)",
                        filtered_count,
                        skipped_count
                    ),
                    "scanner_timing"
                );

                for nested_path in filtered_nested {
                    if Self::should_ignore_path(Path::new(&nested_path)) {
                        continue;
                    }

                    match self.scan_nested_archive_in_rar(archive_path, &nested_path, ctx, password)
                    {
                        Ok(nested_items) => {
                            detected.extend(nested_items);
                        }
                        Err(e) => {
                            if let Some(_) = e.downcast_ref::<PasswordRequiredError>() {
                                return Err(anyhow::anyhow!(NestedPasswordRequiredError {
                                    parent_archive: archive_path.to_string_lossy().to_string(),
                                    nested_archive: nested_path.clone(),
                                }));
                            }
                            crate::logger::log_info(
                                &format!("Failed to scan nested archive {}: {}", nested_path, e),
                                Some("scanner"),
                            );
                        }
                    }
                }

                crate::log_debug!(
                    &format!(
                        "[TIMING] RAR nested archive processing completed in {:.2}ms",
                        nested_process_start.elapsed().as_secs_f64() * 1000.0
                    ),
                    "scanner_timing"
                );
            }
        }

        crate::log_debug!(
            &format!(
                "[TIMING] RAR scan completed in {:.2}ms: {} total addons detected",
                scan_start.elapsed().as_secs_f64() * 1000.0,
                detected.len()
            ),
            "scanner_timing"
        );

        Ok(detected)
    }

    /// Scan a nested archive within a RAR file (extract to temp)
    /// Optimized: If nested archive is ZIP, load into memory for faster scanning
    fn scan_nested_archive_in_rar(
        &self,
        parent_path: &Path,
        nested_path: &str,
        ctx: &mut ScanContext,
        parent_password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        use tempfile::Builder;

        // Create temp directory for extraction
        let temp_dir = Builder::new()
            .prefix("xfi_rar_nested_")
            .tempdir()
            .context("Failed to create temp directory")?;

        // Extract the RAR archive to temp using the typestate pattern
        let archive_builder = if let Some(pwd) = parent_password {
            unrar::Archive::with_password(parent_path, pwd)
        } else {
            unrar::Archive::new(parent_path)
        };

        let mut archive = archive_builder
            .open_for_processing()
            .map_err(|e| anyhow::anyhow!("Failed to open RAR for processing: {:?}", e))?;

        while let Some(header) = archive
            .read_header()
            .map_err(|e| anyhow::anyhow!("Failed to read RAR header: {:?}", e))?
        {
            archive = if header.entry().is_file() {
                header
                    .extract_with_base(temp_dir.path())
                    .map_err(|e| anyhow::anyhow!("Failed to extract RAR entry: {:?}", e))?
            } else {
                header
                    .skip()
                    .map_err(|e| anyhow::anyhow!("Failed to skip RAR entry: {:?}", e))?
            };
        }

        // Find the nested archive in extracted files
        let temp_archive_path = temp_dir.path().join(nested_path);

        if !temp_archive_path.exists() {
            // Provide detailed error with directory listing
            let mut available_files = Vec::new();
            if let Ok(entries) = fs::read_dir(temp_dir.path()) {
                for entry in entries.flatten().take(10) {
                    if let Some(name) = entry.file_name().to_str() {
                        available_files.push(name.to_string());
                    }
                }
            }

            return Err(anyhow::anyhow!(
                "Nested archive not found after extraction: {}\nExpected at: {:?}\nAvailable files: {}",
                nested_path,
                temp_archive_path,
                if available_files.is_empty() {
                    "(none)".to_string()
                } else {
                    available_files.join(", ")
                }
            ));
        }

        // Get archive format
        let format = get_archive_format(nested_path)
            .ok_or_else(|| anyhow::anyhow!("Unknown archive format: {}", nested_path))?;

        // Check if this nested archive has its own password
        let nested_password =
            ctx.get_nested_password(&parent_path.to_string_lossy().to_string(), nested_path);

        // Build nested archive info with password if available
        let nested_info = NestedArchiveInfo {
            internal_path: nested_path.to_string(),
            password: nested_password.clone(),
            format: format.clone(),
        };

        // Push to context chain
        ctx.push_archive(nested_info.clone());

        // OPTIMIZATION: If nested archive is ZIP, try to load into memory
        let nested_result = if format == "zip" {
            crate::logger::log_info(
                &format!("Optimizing: Loading nested ZIP from RAR into memory for scanning"),
                Some("scanner"),
            );

            match self.try_scan_zip_from_file_to_memory(&temp_archive_path, parent_path, ctx) {
                Ok(items) => Ok(items),
                Err(e) => {
                    crate::logger::log_info(
                        &format!("Memory optimization failed, using standard scan: {}", e),
                        Some("scanner"),
                    );
                    // Fallback to standard scan
                    self.scan_path_with_context(&temp_archive_path, ctx)
                }
            }
        } else {
            // For non-ZIP, use standard scan
            self.scan_path_with_context(&temp_archive_path, ctx)
        };

        // Pop from context chain
        ctx.pop_archive();

        // Process results
        match nested_result {
            Ok(mut items) => {
                // Update each detected item with extraction chain
                for item in &mut items {
                    let mut chain = ExtractionChain {
                        archives: ctx.parent_chain.clone(),
                        final_internal_root: item.archive_internal_root.clone(),
                    };
                    chain.archives.push(nested_info.clone());
                    item.path = parent_path.to_string_lossy().to_string();
                    item.extraction_chain = Some(chain);
                    item.archive_internal_root = None;

                    // Update display_name to use the nested archive's filename (without extension)
                    // This prevents creating folders like "Scenery/.zip"
                    if let Some(nested_filename) = Path::new(nested_path).file_stem() {
                        if let Some(name_str) = nested_filename.to_str() {
                            item.display_name = name_str.to_string();
                        }
                    }
                }
                Ok(items)
            }
            Err(e) => Err(e),
        }
    }

    /// Scan a 7z archive without extraction
    fn scan_7z(&self, archive_path: &Path, password: Option<&str>) -> Result<Vec<DetectedItem>> {
        // Open archive to read file list (fast, no decompression)
        let archive = sevenz_rust2::Archive::open(archive_path)
            .map_err(|e| anyhow::anyhow!("Failed to open 7z archive: {}", e))?;

        // Check for empty archive
        if archive.files.is_empty() {
            logger::log_info(
                &format!("Empty 7z archive: {}", archive_path.display()),
                Some("scanner"),
            );
            return Ok(Vec::new());
        }

        // Check if archive has encrypted files by examining headers
        // This is faster than trying to read file content
        let has_encrypted_headers = archive
            .files
            .iter()
            .any(|f| f.has_stream() && !f.is_directory());

        // Check encryption and password validity
        if has_encrypted_headers {
            // Determine which password to use for testing
            let test_password = match password {
                Some(pwd) => sevenz_rust2::Password::from(pwd),
                None => sevenz_rust2::Password::empty(),
            };

            // Try a quick open test - if it fails with password error, we know it's encrypted
            match sevenz_rust2::ArchiveReader::open(archive_path, test_password) {
                Ok(mut reader) => {
                    // Try to read first non-directory entry to verify password
                    let mut encryption_detected = false;
                    let mut wrong_password = false;
                    let _ = reader.for_each_entries(|entry, reader| {
                        if !entry.is_directory() {
                            let mut buf = [0u8; 1];
                            if std::io::Read::read(reader, &mut buf).is_err() {
                                if password.is_some() {
                                    // Password provided but still can't read - wrong password
                                    wrong_password = true;
                                } else {
                                    // No password provided - encryption detected
                                    encryption_detected = true;
                                }
                            }
                            return Ok(false); // Stop after first file
                        }
                        Ok(true)
                    });

                    if wrong_password {
                        return Err(anyhow::anyhow!(
                            "Wrong password for archive: {}",
                            archive_path.display()
                        ));
                    }

                    if encryption_detected {
                        return Err(anyhow::anyhow!(PasswordRequiredError {
                            archive_path: archive_path.to_string_lossy().to_string(),
                        }));
                    }
                }
                Err(e) => {
                    let err_str = format!("{:?}", e);
                    if err_str.contains("password")
                        || err_str.contains("Password")
                        || err_str.contains("encrypted")
                        || err_str.contains("WrongPassword")
                    {
                        if password.is_some() {
                            // Password provided but still failed - wrong password
                            return Err(anyhow::anyhow!(
                                "Wrong password for archive: {}",
                                archive_path.display()
                            ));
                        }
                        return Err(anyhow::anyhow!(PasswordRequiredError {
                            archive_path: archive_path.to_string_lossy().to_string(),
                        }));
                    }
                }
            }
        }

        let mut detected = Vec::new();

        // Collect file paths and identify markers in a single pass
        let mut plugin_dirs: HashSet<String> = HashSet::new();
        let mut aircraft_dirs: HashSet<String> = HashSet::new();
        let mut marker_files: Vec<(String, &str)> = Vec::new(); // (path, marker_type)
        let mut detected_livery_roots: HashSet<String> = HashSet::new();

        for entry in &archive.files {
            let file_path = entry.name().to_string();
            let normalized = file_path.replace('\\', "/");

            // Skip ignored paths
            if Self::should_ignore_archive_path(&normalized) {
                continue;
            }

            // Check for livery patterns first (before any potential moves)
            if let Some((_, livery_root)) = livery_patterns::check_livery_pattern(&normalized) {
                if !detected_livery_roots.contains(&livery_root) {
                    detected_livery_roots.insert(livery_root.clone());
                    marker_files.push((normalized.clone(), "livery"));
                }
            }

            // Identify plugin directories, aircraft directories, and marker files
            if normalized.ends_with(".xpl") {
                if let Some(parent) = Path::new(&normalized).parent() {
                    let parent_str = parent.to_string_lossy();
                    let parent_name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");

                    let plugin_root = if matches!(
                        parent_name,
                        "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
                    ) {
                        parent
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(parent_str.to_string())
                    } else {
                        parent_str.to_string()
                    };
                    plugin_dirs.insert(plugin_root);
                }
                marker_files.push((normalized, "xpl"));
            } else if normalized.ends_with(".acf") {
                // Track aircraft directories to skip embedded plugins
                if let Some(parent) = Path::new(&normalized).parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if !parent_str.is_empty() {
                        aircraft_dirs.insert(parent_str);
                    }
                }
                marker_files.push((normalized, "acf"));
            } else if normalized.ends_with("library.txt") {
                marker_files.push((normalized, "library"));
            } else if normalized.ends_with(".dsf") {
                marker_files.push((normalized, "dsf"));
            } else if normalized.ends_with("cycle.json") {
                marker_files.push((normalized, "navdata"));
            } else if normalized.ends_with(".lua") {
                // Lua script detection (lowest priority)
                marker_files.push((normalized, "lua"));
            }
        }

        // Sort marker files by depth, then by type priority (aircraft first)
        marker_files.sort_by(|a, b| {
            let depth_a = a.0.matches('/').count();
            let depth_b = b.0.matches('/').count();
            match depth_a.cmp(&depth_b) {
                std::cmp::Ordering::Equal => {
                    // Same depth: sort by marker type priority (aircraft first)
                    Self::marker_type_priority(a.1).cmp(&Self::marker_type_priority(b.1))
                }
                other => other,
            }
        });

        // Track detected addon roots to skip
        let mut skip_prefixes: Vec<String> = Vec::new();

        // Process marker files
        for (file_path, marker_type) in marker_files {
            // Check if inside a skip prefix (already detected addon)
            let should_skip = skip_prefixes
                .iter()
                .any(|prefix| file_path.starts_with(prefix));
            if should_skip {
                continue;
            }

            // Check if .acf/.dsf is inside a plugin directory
            if marker_type == "acf" || marker_type == "dsf" {
                if Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs) {
                    continue;
                }
            }

            // Skip .xpl inside aircraft directories (embedded plugins)
            if marker_type == "xpl" {
                if Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs) {
                    continue;
                }
            }

            // Detect addon based on marker type
            let item = match marker_type {
                "acf" => self.detect_aircraft_in_archive(&file_path, archive_path)?,
                "library" => self.detect_scenery_library(&file_path, archive_path)?,
                "dsf" => self.detect_scenery_dsf(&file_path, archive_path)?,
                "xpl" => self.detect_plugin_in_archive(&file_path, archive_path)?,
                "navdata" => {
                    if let Ok(content) = self.read_file_from_7z(archive_path, &file_path, password)
                    {
                        self.detect_navdata_in_archive(&file_path, &content, archive_path)?
                    } else {
                        None
                    }
                }
                "livery" => self.detect_livery_in_archive(&file_path, archive_path)?,
                "lua" => self.detect_lua_script_in_archive(&file_path, archive_path)?,
                _ => None,
            };

            if let Some(item) = item {
                // Add to skip prefixes
                if let Some(ref internal_root) = item.archive_internal_root {
                    let prefix = if internal_root.ends_with('/') {
                        internal_root.clone()
                    } else {
                        format!("{}/", internal_root)
                    };
                    skip_prefixes.push(prefix);
                } else if item.addon_type == AddonType::Aircraft {
                    // Aircraft at archive root: skip all other markers in this archive
                    skip_prefixes.push(String::new());
                }
                detected.push(item);
            }
        }

        Ok(detected)
    }

    /// Read a single file content from a 7z archive
    fn read_file_from_7z(
        &self,
        archive_path: &Path,
        file_path: &str,
        password: Option<&str>,
    ) -> Result<String> {
        // Create secure temp directory using tempfile crate
        let temp_dir = tempfile::Builder::new()
            .prefix("xfi_7z_read_")
            .tempdir()
            .context("Failed to create secure temp directory")?;

        // Extract to temp (with password if provided)
        if let Some(pwd) = password {
            let mut reader =
                sevenz_rust2::ArchiveReader::open(archive_path, sevenz_rust2::Password::from(pwd))
                    .map_err(|e| anyhow::anyhow!("Failed to open 7z with password: {}", e))?;
            reader
                .for_each_entries(|entry, reader| {
                    let dest_path = temp_dir.path().join(entry.name());
                    if entry.is_directory() {
                        std::fs::create_dir_all(&dest_path)?;
                    } else {
                        if let Some(parent) = dest_path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        let mut file = std::fs::File::create(&dest_path)?;
                        std::io::copy(reader, &mut file)?;
                    }
                    Ok(true)
                })
                .map_err(|e| anyhow::anyhow!("Failed to extract 7z with password: {}", e))?;
        } else {
            sevenz_rust2::decompress_file(archive_path, temp_dir.path())
                .map_err(|e| anyhow::anyhow!("Failed to extract 7z: {}", e))?;
        }

        // Sanitize the file path to prevent path traversal using proper sanitization
        let safe_path = crate::installer::sanitize_path(Path::new(file_path))
            .ok_or_else(|| anyhow::anyhow!("Unsafe path in 7z archive: {}", file_path))?;
        let target_file = temp_dir.path().join(safe_path);
        let content = fs::read_to_string(&target_file).context("Failed to read file from 7z")?;

        // TempDir automatically cleans up when dropped
        Ok(content)
    }

    /// Scan a RAR archive without extraction
    fn scan_rar(&self, archive_path: &Path, password: Option<&str>) -> Result<Vec<DetectedItem>> {
        let open_start = std::time::Instant::now();
        crate::log_debug!(
            &format!("[TIMING] RAR open started: {}", archive_path.display()),
            "scanner_timing"
        );

        // Create archive with or without password
        let archive_builder = if let Some(pwd) = password {
            unrar::Archive::with_password(archive_path, pwd)
        } else {
            unrar::Archive::new(archive_path)
        };

        let archive = archive_builder.open_for_listing().map_err(|e| {
            let err_str = format!("{:?}", e);
            // Check for password-related errors
            if err_str.contains("password")
                || err_str.contains("Password")
                || err_str.contains("encrypted")
                || err_str.contains("ERAR_MISSING_PASSWORD")
            {
                if password.is_none() {
                    anyhow::anyhow!(PasswordRequiredError {
                        archive_path: archive_path.to_string_lossy().to_string(),
                    })
                } else {
                    anyhow::anyhow!("Wrong password for archive: {}", archive_path.display())
                }
            } else {
                anyhow::anyhow!("Failed to open RAR archive: {:?}", e)
            }
        })?;

        crate::log_debug!(
            &format!(
                "[TIMING] RAR open completed in {:.2}ms",
                open_start.elapsed().as_secs_f64() * 1000.0
            ),
            "scanner_timing"
        );

        let mut files: Vec<String> = Vec::new();

        // Collect all file paths
        let enumerate_start = std::time::Instant::now();
        for entry in archive {
            if let Ok(e) = entry {
                files.push(e.filename.to_string_lossy().to_string().replace('\\', "/"));
            }
        }

        crate::log_debug!(
            &format!(
                "[TIMING] RAR enumeration completed in {:.2}ms: {} files",
                enumerate_start.elapsed().as_secs_f64() * 1000.0,
                files.len()
            ),
            "scanner_timing"
        );

        // Check for empty archive
        if files.is_empty() {
            logger::log_info(
                &format!("Empty RAR archive: {}", archive_path.display()),
                Some("scanner"),
            );
            return Ok(Vec::new());
        }

        // Single pass: identify plugin directories, aircraft directories, and marker files
        let marker_identify_start = std::time::Instant::now();
        let mut plugin_dirs: HashSet<String> = HashSet::new();
        let mut aircraft_dirs: HashSet<String> = HashSet::new();
        let mut marker_files: Vec<(String, &str)> = Vec::new(); // (path, marker_type)
        let mut detected_livery_roots: HashSet<String> = HashSet::new();

        for file_path in &files {
            // Skip ignored paths
            if Self::should_ignore_archive_path(file_path) {
                continue;
            }

            // Identify plugin directories, aircraft directories, and marker files
            if file_path.ends_with(".xpl") {
                if let Some(parent) = Path::new(file_path).parent() {
                    let parent_str = parent.to_string_lossy();
                    let parent_name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");

                    let plugin_root = if matches!(
                        parent_name,
                        "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
                    ) {
                        parent
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(parent_str.to_string())
                    } else {
                        parent_str.to_string()
                    };
                    plugin_dirs.insert(plugin_root);
                }
                marker_files.push((file_path.clone(), "xpl"));
            } else if file_path.ends_with(".acf") {
                // Track aircraft directories to skip embedded plugins
                if let Some(parent) = Path::new(file_path).parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if !parent_str.is_empty() {
                        aircraft_dirs.insert(parent_str);
                    }
                }
                marker_files.push((file_path.clone(), "acf"));
            } else if file_path.ends_with("library.txt") {
                marker_files.push((file_path.clone(), "library"));
            } else if file_path.ends_with(".dsf") {
                marker_files.push((file_path.clone(), "dsf"));
            } else if file_path.ends_with("cycle.json") {
                marker_files.push((file_path.clone(), "navdata"));
            } else if file_path.ends_with(".lua") {
                // Lua script detection (lowest priority)
                marker_files.push((file_path.clone(), "lua"));
            }

            // Check for livery patterns
            if let Some((_, livery_root)) = livery_patterns::check_livery_pattern(file_path) {
                if !detected_livery_roots.contains(&livery_root) {
                    detected_livery_roots.insert(livery_root.clone());
                    marker_files.push((file_path.clone(), "livery"));
                }
            }
        }

        crate::log_debug!(
            &format!(
                "[TIMING] RAR marker identification completed in {:.2}ms: {} markers",
                marker_identify_start.elapsed().as_secs_f64() * 1000.0,
                marker_files.len()
            ),
            "scanner_timing"
        );

        // Sort marker files by depth, then by type priority (aircraft first)
        let sort_start = std::time::Instant::now();
        marker_files.sort_by(|a, b| {
            let depth_a = a.0.matches('/').count();
            let depth_b = b.0.matches('/').count();
            match depth_a.cmp(&depth_b) {
                std::cmp::Ordering::Equal => {
                    // Same depth: sort by marker type priority (aircraft first)
                    Self::marker_type_priority(a.1).cmp(&Self::marker_type_priority(b.1))
                }
                other => other,
            }
        });
        crate::log_debug!(
            &format!(
                "[TIMING] RAR marker sorting completed in {:.2}ms",
                sort_start.elapsed().as_secs_f64() * 1000.0
            ),
            "scanner_timing"
        );

        let mut detected = Vec::new();
        let mut skip_prefixes: Vec<String> = Vec::new();

        // Process marker files
        let process_start = std::time::Instant::now();
        crate::log_debug!(
            &format!(
                "[TIMING] RAR marker processing started: {} markers",
                marker_files.len()
            ),
            "scanner_timing"
        );

        for (file_path, marker_type) in marker_files {
            // Check if inside a skip prefix (already detected addon)
            let should_skip = skip_prefixes
                .iter()
                .any(|prefix| file_path.starts_with(prefix));
            if should_skip {
                continue;
            }

            // Check if .acf/.dsf is inside a plugin directory
            if marker_type == "acf" || marker_type == "dsf" {
                if Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs) {
                    continue;
                }
            }

            // Skip .xpl inside aircraft directories (embedded plugins)
            if marker_type == "xpl" {
                if Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs) {
                    continue;
                }
            }

            // Detect addon based on marker type
            let item = match marker_type {
                "acf" => self.detect_aircraft_in_archive(&file_path, archive_path)?,
                "library" => self.detect_scenery_library(&file_path, archive_path)?,
                "dsf" => self.detect_scenery_dsf(&file_path, archive_path)?,
                "xpl" => self.detect_plugin_in_archive(&file_path, archive_path)?,
                "navdata" => {
                    if let Ok(content) = self.read_file_from_rar(archive_path, &file_path, password)
                    {
                        self.detect_navdata_in_archive(&file_path, &content, archive_path)?
                    } else {
                        None
                    }
                }
                "livery" => self.detect_livery_in_archive(&file_path, archive_path)?,
                "lua" => self.detect_lua_script_in_archive(&file_path, archive_path)?,
                _ => None,
            };

            if let Some(item) = item {
                // Add to skip prefixes
                if let Some(ref internal_root) = item.archive_internal_root {
                    let prefix = if internal_root.ends_with('/') {
                        internal_root.clone()
                    } else {
                        format!("{}/", internal_root)
                    };
                    skip_prefixes.push(prefix);
                } else if item.addon_type == AddonType::Aircraft {
                    // Aircraft at archive root: skip all other markers in this archive
                    skip_prefixes.push(String::new());
                }
                detected.push(item);
            }
        }

        crate::log_debug!(
            &format!(
                "[TIMING] RAR marker processing completed in {:.2}ms: {} addons detected",
                process_start.elapsed().as_secs_f64() * 1000.0,
                detected.len()
            ),
            "scanner_timing"
        );

        Ok(detected)
    }

    /// Read a single file content from a RAR archive
    fn read_file_from_rar(
        &self,
        archive_path: &Path,
        target_file: &str,
        password: Option<&str>,
    ) -> Result<String> {
        // Create secure temp directory using tempfile crate
        let temp_dir = tempfile::Builder::new()
            .prefix("xfi_rar_read_")
            .tempdir()
            .context("Failed to create secure temp directory")?;

        // Extract to temp using the typestate pattern (with password if provided)
        let archive_builder = if let Some(pwd) = password {
            unrar::Archive::with_password(archive_path, pwd)
        } else {
            unrar::Archive::new(archive_path)
        };

        let mut archive = archive_builder
            .open_for_processing()
            .map_err(|e| anyhow::anyhow!("Failed to open RAR for processing: {:?}", e))?;

        while let Some(header) = archive
            .read_header()
            .map_err(|e| anyhow::anyhow!("Failed to read RAR header: {:?}", e))?
        {
            archive = if header.entry().is_file() {
                header
                    .extract_with_base(temp_dir.path())
                    .map_err(|e| anyhow::anyhow!("Failed to extract RAR entry: {:?}", e))?
            } else {
                header
                    .skip()
                    .map_err(|e| anyhow::anyhow!("Failed to skip RAR entry: {:?}", e))?
            };
        }

        // Sanitize the file path to prevent path traversal using proper sanitization
        let safe_path = crate::installer::sanitize_path(Path::new(target_file))
            .ok_or_else(|| anyhow::anyhow!("Unsafe path in RAR archive: {}", target_file))?;
        let file_path = temp_dir.path().join(safe_path);
        let content = fs::read_to_string(&file_path).context("Failed to read file from RAR")?;

        // TempDir automatically cleans up when dropped
        Ok(content)
    }

    /// Scan a ZIP archive with context (supports nested archives)
    /// OPTIMIZED: Single pass to collect both addon markers and nested archives
    fn scan_zip_with_context(
        &self,
        zip_path: &Path,
        ctx: &mut ScanContext,
        password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        use std::io::Read;
        use zip::ZipArchive;

        let scan_start = std::time::Instant::now();
        crate::log_debug!(
            &format!("[TIMING] ZIP scan started: {}", zip_path.display()),
            "scanner_timing"
        );

        let open_start = std::time::Instant::now();
        let file = fs::File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;
        crate::log_debug!(
            &format!(
                "[TIMING] ZIP open completed in {:.2}ms: {}",
                open_start.elapsed().as_secs_f64() * 1000.0,
                zip_path.display()
            ),
            "scanner_timing"
        );

        // Check for empty archive
        if archive.len() == 0 {
            logger::log_info(
                &format!("Empty ZIP archive: {}", zip_path.display()),
                Some("scanner"),
            );
            return Ok(Vec::new());
        }

        // Convert password to bytes if provided
        let password_bytes = password.map(|p| p.as_bytes());

        // SINGLE PASS: collect file info, check encryption, identify markers, AND find nested archives
        let enumerate_start = std::time::Instant::now();
        crate::log_debug!(
            &format!("[TIMING] ZIP enumeration started: {} files", archive.len()),
            "scanner_timing"
        );

        let mut plugin_dirs: HashSet<String> = HashSet::new();
        let mut aircraft_dirs: HashSet<String> = HashSet::new();
        let mut marker_files: Vec<(usize, String, bool, &str)> = Vec::new(); // (index, path, encrypted, marker_type)
        let mut nested_archives: Vec<(usize, String, bool)> = Vec::new(); // (index, path, encrypted)
        let mut has_encrypted = false;
        let mut detected_livery_roots: HashSet<String> = HashSet::new();

        for i in 0..archive.len() {
            let file = match archive.by_index_raw(i) {
                Ok(f) => f,
                Err(e) => {
                    let err_str = format!("{:?}", e);
                    if err_str.contains("password")
                        || err_str.contains("Password")
                        || err_str.contains("encrypted")
                        || err_str.contains("InvalidPassword")
                    {
                        if password_bytes.is_none() {
                            return Err(anyhow::anyhow!(PasswordRequiredError {
                                archive_path: zip_path.to_string_lossy().to_string(),
                            }));
                        } else {
                            return Err(anyhow::anyhow!(
                                "Wrong password for archive: {}",
                                zip_path.display()
                            ));
                        }
                    }
                    return Err(anyhow::anyhow!("Failed to read ZIP entry {}: {}", i, e));
                }
            };

            let is_encrypted = file.encrypted();
            if is_encrypted {
                has_encrypted = true;
            }

            let file_path = file.name().replace('\\', "/");

            // Skip ignored paths
            if Self::should_ignore_archive_path(&file_path) {
                continue;
            }

            // Check if this is a nested archive (for recursive scanning)
            if !file.is_dir() && is_archive_file(&file_path) {
                nested_archives.push((i, file_path.clone(), is_encrypted));
            }

            // Check for livery patterns first (before any potential moves)
            if let Some((_, livery_root)) = livery_patterns::check_livery_pattern(&file_path) {
                if !detected_livery_roots.contains(&livery_root) {
                    detected_livery_roots.insert(livery_root.clone());
                    marker_files.push((i, file_path.clone(), is_encrypted, "livery"));
                }
            }

            // Identify plugin directories, aircraft directories, and marker files
            if file_path.ends_with(".xpl") {
                if let Some(parent) = Path::new(&file_path).parent() {
                    let parent_str = parent.to_string_lossy();
                    let parent_name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");

                    let plugin_root = if matches!(
                        parent_name,
                        "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
                    ) {
                        parent
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(parent_str.to_string())
                    } else {
                        parent_str.to_string()
                    };
                    plugin_dirs.insert(plugin_root);
                }
                marker_files.push((i, file_path, is_encrypted, "xpl"));
            } else if file_path.ends_with(".acf") {
                // Track aircraft directories to skip embedded plugins
                if let Some(parent) = Path::new(&file_path).parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if !parent_str.is_empty() {
                        aircraft_dirs.insert(parent_str);
                    }
                }
                marker_files.push((i, file_path, is_encrypted, "acf"));
            } else if file_path.ends_with("library.txt") {
                marker_files.push((i, file_path, is_encrypted, "library"));
            } else if file_path.ends_with(".dsf") {
                marker_files.push((i, file_path, is_encrypted, "dsf"));
            } else if file_path.ends_with("cycle.json") {
                marker_files.push((i, file_path, is_encrypted, "navdata"));
            } else if file_path.ends_with(".lua") {
                // Lua script detection (lowest priority)
                marker_files.push((i, file_path, is_encrypted, "lua"));
            }
        }

        crate::log_debug!(
            &format!("[TIMING] ZIP enumeration completed in {:.2}ms: {} files, {} markers, {} nested archives",
                enumerate_start.elapsed().as_secs_f64() * 1000.0,
                archive.len(),
                marker_files.len(),
                nested_archives.len()
            ),
            "scanner_timing"
        );

        // If any file is encrypted but no password provided, request password
        if has_encrypted && password_bytes.is_none() {
            return Err(anyhow::anyhow!(PasswordRequiredError {
                archive_path: zip_path.to_string_lossy().to_string(),
            }));
        }

        // If password was provided and archive has encrypted files, verify password by trying to read first encrypted file
        if has_encrypted && password_bytes.is_some() {
            let pwd = password_bytes.unwrap();
            // Find first encrypted file index
            let mut encrypted_index: Option<usize> = None;
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index_raw(i) {
                    if file.encrypted() && !file.is_dir() {
                        encrypted_index = Some(i);
                        break;
                    }
                }
            }
            // Try to decrypt the first encrypted file
            if let Some(idx) = encrypted_index {
                use std::io::Read;
                match archive.by_index_decrypt(idx, pwd) {
                    Ok(mut f) => {
                        let mut buf = [0u8; 1];
                        if f.read(&mut buf).is_err() {
                            return Err(anyhow::anyhow!(
                                "Wrong password for archive: {}",
                                zip_path.display()
                            ));
                        }
                    }
                    Err(e) => {
                        let err_str = format!("{:?}", e);
                        if err_str.contains("password")
                            || err_str.contains("Password")
                            || err_str.contains("InvalidPassword")
                        {
                            return Err(anyhow::anyhow!(
                                "Wrong password for archive: {}",
                                zip_path.display()
                            ));
                        }
                    }
                }
            }
        }

        // Sort marker files by depth, then by type priority (aircraft first)
        let sort_start = std::time::Instant::now();
        marker_files.sort_by(|a, b| {
            let depth_a = a.1.matches('/').count();
            let depth_b = b.1.matches('/').count();
            match depth_a.cmp(&depth_b) {
                std::cmp::Ordering::Equal => {
                    // Same depth: sort by marker type priority (aircraft first)
                    Self::marker_type_priority(a.3).cmp(&Self::marker_type_priority(b.3))
                }
                other => other,
            }
        });
        crate::log_debug!(
            &format!(
                "[TIMING] ZIP marker sorting completed in {:.2}ms",
                sort_start.elapsed().as_secs_f64() * 1000.0
            ),
            "scanner_timing"
        );

        let mut detected = Vec::new();
        let mut skip_prefixes: Vec<String> = Vec::new();

        // Process marker files to detect addons
        let process_start = std::time::Instant::now();
        crate::log_debug!(
            &format!(
                "[TIMING] ZIP marker processing started: {} markers",
                marker_files.len()
            ),
            "scanner_timing"
        );

        for (i, file_path, is_encrypted, marker_type) in marker_files {
            // Check if inside a skip prefix (already detected addon)
            let should_skip = skip_prefixes
                .iter()
                .any(|prefix| file_path.starts_with(prefix));
            if should_skip {
                continue;
            }

            // Check if .acf/.dsf is inside a plugin directory
            if marker_type == "acf" || marker_type == "dsf" {
                if Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs) {
                    continue;
                }
            }

            // Skip .xpl inside aircraft directories (embedded plugins)
            if marker_type == "xpl" {
                if Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs) {
                    continue;
                }
            }

            // Detect addon based on marker type
            let item = match marker_type {
                "acf" => self.detect_aircraft_in_archive(&file_path, zip_path)?,
                "library" => self.detect_scenery_library(&file_path, zip_path)?,
                "dsf" => self.detect_scenery_dsf(&file_path, zip_path)?,
                "xpl" => self.detect_plugin_in_archive(&file_path, zip_path)?,
                "navdata" => {
                    // Need to read cycle.json content
                    let mut content = String::new();

                    let read_ok = if is_encrypted {
                        if let Some(pwd) = password_bytes {
                            match archive.by_index_decrypt(i, pwd) {
                                Ok(mut f) => f.read_to_string(&mut content).is_ok(),
                                Err(_) => false,
                            }
                        } else {
                            continue;
                        }
                    } else {
                        match archive.by_index(i) {
                            Ok(mut f) => f.read_to_string(&mut content).is_ok(),
                            Err(_) => false,
                        }
                    };

                    if read_ok {
                        self.detect_navdata_in_archive(&file_path, &content, zip_path)?
                    } else {
                        None
                    }
                }
                "livery" => self.detect_livery_in_archive(&file_path, zip_path)?,
                "lua" => self.detect_lua_script_in_archive(&file_path, zip_path)?,
                _ => None,
            };

            if let Some(item) = item {
                // Add to skip prefixes
                if let Some(ref internal_root) = item.archive_internal_root {
                    let prefix = if internal_root.ends_with('/') {
                        internal_root.clone()
                    } else {
                        format!("{}/", internal_root)
                    };
                    skip_prefixes.push(prefix);
                } else if item.addon_type == AddonType::Aircraft {
                    // Aircraft at archive root: skip all other markers in this archive
                    skip_prefixes.push(String::new());
                }
                detected.push(item);
            }
        }

        crate::log_debug!(
            &format!(
                "[TIMING] ZIP marker processing completed in {:.2}ms: {} addons detected",
                process_start.elapsed().as_secs_f64() * 1000.0,
                detected.len()
            ),
            "scanner_timing"
        );

        // Recursively scan nested archives if depth allows
        if ctx.can_recurse() && !nested_archives.is_empty() {
            let nested_start = std::time::Instant::now();
            let total_nested = nested_archives.len();

            // Filter out nested archives that are inside already detected addon directories
            let filtered_nested: Vec<_> = nested_archives
                .into_iter()
                .filter(|(_, nested_path, _)| {
                    // Check if this nested archive is inside any detected addon directory
                    let is_inside_addon = skip_prefixes
                        .iter()
                        .any(|prefix| nested_path.starts_with(prefix));
                    !is_inside_addon
                })
                .collect();

            let filtered_count = filtered_nested.len();
            let skipped_count = total_nested - filtered_count;

            crate::log_debug!(
                &format!("[TIMING] ZIP nested archive processing started: {} nested archives ({} skipped as inside detected addons)",
                    filtered_count,
                    skipped_count
                ),
                "scanner_timing"
            );

            for (index, nested_path, is_encrypted) in filtered_nested {
                // Skip if inside ignored paths
                if Self::should_ignore_path(Path::new(&nested_path)) {
                    continue;
                }

                match self.scan_nested_archive_in_zip(
                    &mut archive,
                    index,
                    &nested_path,
                    zip_path,
                    ctx,
                    password_bytes,
                    is_encrypted,
                ) {
                    Ok(nested_items) => {
                        detected.extend(nested_items);
                    }
                    Err(e) => {
                        // Check if it's a password error for nested archive
                        if let Some(_pwd_err) = e.downcast_ref::<PasswordRequiredError>() {
                            // Convert to nested password error
                            return Err(anyhow::anyhow!(NestedPasswordRequiredError {
                                parent_archive: zip_path.to_string_lossy().to_string(),
                                nested_archive: nested_path.clone(),
                            }));
                        }
                        // Log other errors but continue scanning
                        crate::logger::log_info(
                            &format!("Failed to scan nested archive {}: {}", nested_path, e),
                            Some("scanner"),
                        );
                    }
                }
            }

            crate::log_debug!(
                &format!(
                    "[TIMING] ZIP nested archive processing completed in {:.2}ms",
                    nested_start.elapsed().as_secs_f64() * 1000.0
                ),
                "scanner_timing"
            );
        }

        crate::log_debug!(
            &format!(
                "[TIMING] ZIP scan completed in {:.2}ms: {} total addons detected",
                scan_start.elapsed().as_secs_f64() * 1000.0,
                detected.len()
            ),
            "scanner_timing"
        );

        Ok(detected)
    }

    /// Scan a nested archive within a ZIP file (in-memory)
    fn scan_nested_archive_in_zip(
        &self,
        parent_archive: &mut zip::ZipArchive<fs::File>,
        file_index: usize,
        nested_path: &str,
        parent_path: &Path,
        ctx: &mut ScanContext,
        parent_password: Option<&[u8]>,
        is_encrypted: bool,
    ) -> Result<Vec<DetectedItem>> {
        use std::io::Read;

        // Read nested archive into memory
        let mut nested_data = Vec::new();

        if is_encrypted {
            if let Some(pwd) = parent_password {
                let mut nested_file = parent_archive
                    .by_index_decrypt(file_index, pwd)
                    .map_err(|e| anyhow::anyhow!("Failed to decrypt nested archive: {}", e))?;
                nested_file.read_to_end(&mut nested_data)?;
            } else {
                return Err(anyhow::anyhow!(PasswordRequiredError {
                    archive_path: format!("{}/{}", parent_path.display(), nested_path),
                }));
            }
        } else {
            let mut nested_file = parent_archive.by_index(file_index)?;
            nested_file.read_to_end(&mut nested_data)?;
        }

        // Get archive format
        let format = get_archive_format(nested_path)
            .ok_or_else(|| anyhow::anyhow!("Unknown archive format: {}", nested_path))?;

        // Check if this nested archive has its own password
        let nested_password =
            ctx.get_nested_password(&parent_path.to_string_lossy().to_string(), nested_path);

        // Build nested archive info with password if available
        let nested_info = NestedArchiveInfo {
            internal_path: nested_path.to_string(),
            password: nested_password.clone(),
            format,
        };

        // Push to context chain
        ctx.push_archive(nested_info.clone());

        // For ZIP nested archives, scan in-memory
        let nested_result = if nested_path.to_lowercase().ends_with(".zip") {
            // Create in-memory ZIP archive
            let cursor = std::io::Cursor::new(nested_data);
            match zip::ZipArchive::new(cursor) {
                Ok(mut nested_archive) => {
                    // Scan the nested ZIP archive
                    self.scan_zip_in_memory(&mut nested_archive, parent_path, ctx, nested_path)
                }
                Err(e) => Err(anyhow::anyhow!("Failed to open nested ZIP: {}", e)),
            }
        } else {
            // For 7z/RAR nested in ZIP, write to temp file and scan
            crate::logger::log_info(
                &format!(
                    "Scanning nested {} archive from ZIP (using temp file)",
                    nested_info.format
                ),
                Some("scanner"),
            );
            self.scan_nested_non_zip_from_memory(nested_data, &nested_info.format, parent_path, ctx)
        };

        // Pop from context chain
        ctx.pop_archive();

        // Process results
        match nested_result {
            Ok(mut items) => {
                // Update each detected item with extraction chain
                for item in &mut items {
                    // Build extraction chain from context
                    let mut chain = ExtractionChain {
                        archives: ctx.parent_chain.clone(),
                        final_internal_root: item.archive_internal_root.clone(),
                    };

                    // Add current nested archive to chain
                    chain.archives.push(nested_info.clone());

                    // Update item
                    item.path = parent_path.to_string_lossy().to_string();
                    item.extraction_chain = Some(chain);
                    item.archive_internal_root = None; // Replaced by extraction_chain

                    // Update display_name to use the nested archive's filename (without extension)
                    // This prevents creating folders like "Scenery/.zip"
                    if let Some(nested_filename) = Path::new(nested_path).file_stem() {
                        if let Some(name_str) = nested_filename.to_str() {
                            item.display_name = name_str.to_string();
                        }
                    }
                }
                Ok(items)
            }
            Err(e) => Err(e),
        }
    }

    /// Scan a ZIP archive that's already in memory
    /// Refactored to use marker file approach with aircraft directory tracking
    fn scan_zip_in_memory(
        &self,
        archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
        parent_path: &Path,
        _ctx: &mut ScanContext,
        _nested_path: &str,
    ) -> Result<Vec<DetectedItem>> {
        use std::io::Read;

        // First pass: collect all file paths and identify directories/markers
        let mut plugin_dirs: HashSet<String> = HashSet::new();
        let mut aircraft_dirs: HashSet<String> = HashSet::new();
        let mut marker_files: Vec<(usize, String, &str)> = Vec::new(); // (index, path, marker_type)
        let mut detected_livery_roots: HashSet<String> = HashSet::new();

        for i in 0..archive.len() {
            // Use by_index_raw to avoid triggering decryption errors when reading metadata
            let file = archive.by_index_raw(i)?;
            let file_path = file.name().replace('\\', "/");

            // Skip ignored paths
            if Self::should_ignore_archive_path(&file_path) {
                continue;
            }

            // Check for livery patterns
            if let Some((_, livery_root)) = livery_patterns::check_livery_pattern(&file_path) {
                if !detected_livery_roots.contains(&livery_root) {
                    detected_livery_roots.insert(livery_root.clone());
                    marker_files.push((i, file_path.clone(), "livery"));
                }
            }

            // Identify plugin directories, aircraft directories, and marker files
            if file_path.ends_with(".xpl") {
                if let Some(parent) = Path::new(&file_path).parent() {
                    let parent_str = parent.to_string_lossy();
                    let parent_name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");

                    let plugin_root = if matches!(
                        parent_name,
                        "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
                    ) {
                        parent
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(parent_str.to_string())
                    } else {
                        parent_str.to_string()
                    };
                    plugin_dirs.insert(plugin_root);
                }
                marker_files.push((i, file_path, "xpl"));
            } else if file_path.ends_with(".acf") {
                // Track aircraft directories to skip embedded plugins
                if let Some(parent) = Path::new(&file_path).parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if !parent_str.is_empty() {
                        aircraft_dirs.insert(parent_str);
                    }
                }
                marker_files.push((i, file_path, "acf"));
            } else if file_path.ends_with("library.txt") {
                marker_files.push((i, file_path, "library"));
            } else if file_path.ends_with(".dsf") {
                marker_files.push((i, file_path, "dsf"));
            } else if file_path.ends_with("cycle.json") {
                marker_files.push((i, file_path, "navdata"));
            } else if file_path.ends_with(".lua") {
                // Lua script detection (lowest priority)
                marker_files.push((i, file_path, "lua"));
            }
        }

        // Sort marker files by depth, then by type priority (aircraft first)
        marker_files.sort_by(|a, b| {
            let depth_a = a.1.matches('/').count();
            let depth_b = b.1.matches('/').count();
            match depth_a.cmp(&depth_b) {
                std::cmp::Ordering::Equal => {
                    // Same depth: sort by marker type priority (aircraft first)
                    Self::marker_type_priority(a.2).cmp(&Self::marker_type_priority(b.2))
                }
                other => other,
            }
        });

        let mut detected = Vec::new();
        let mut skip_prefixes: Vec<String> = Vec::new();

        // Process marker files
        for (file_index, file_path, marker_type) in marker_files {
            // Check if inside a skip prefix (already detected addon)
            let should_skip = skip_prefixes
                .iter()
                .any(|prefix| file_path.starts_with(prefix));
            if should_skip {
                continue;
            }

            // Check if .acf/.dsf is inside a plugin directory
            if marker_type == "acf" || marker_type == "dsf" {
                if Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs) {
                    continue;
                }
            }

            // Skip .xpl inside aircraft directories (embedded plugins)
            if marker_type == "xpl" {
                if Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs) {
                    continue;
                }
            }

            // Detect addon based on marker type
            let item = match marker_type {
                "acf" => self.detect_aircraft_in_archive(&file_path, parent_path)?,
                "library" => self.detect_scenery_library(&file_path, parent_path)?,
                "dsf" => self.detect_scenery_dsf(&file_path, parent_path)?,
                "xpl" => self.detect_plugin_in_archive(&file_path, parent_path)?,
                "navdata" => {
                    // Read cycle.json from nested archive
                    if let Ok(mut file) = archive.by_index(file_index) {
                        let mut content = String::new();
                        if file.read_to_string(&mut content).is_ok() {
                            self.detect_navdata_in_archive(&file_path, &content, parent_path)?
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                "livery" => self.detect_livery_in_archive(&file_path, parent_path)?,
                "lua" => self.detect_lua_script_in_archive(&file_path, parent_path)?,
                _ => None,
            };

            if let Some(item) = item {
                // Add to skip prefixes
                if let Some(ref internal_root) = item.archive_internal_root {
                    let prefix = if internal_root.ends_with('/') {
                        internal_root.clone()
                    } else {
                        format!("{}/", internal_root)
                    };
                    skip_prefixes.push(prefix);
                } else if item.addon_type == AddonType::Aircraft {
                    // Aircraft at archive root: skip all other markers in this archive
                    skip_prefixes.push(String::new());
                }
                detected.push(item);
            }
        }

        Ok(detected)
    }

    /// Scan a non-ZIP archive (7z/RAR) that was extracted from memory
    /// Writes the data to a temp file, scans it, then cleans up
    fn scan_nested_non_zip_from_memory(
        &self,
        archive_data: Vec<u8>,
        format: &str,
        _parent_path: &Path,
        ctx: &mut ScanContext,
    ) -> Result<Vec<DetectedItem>> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary file with appropriate extension
        let extension = match format {
            "7z" => ".7z",
            "rar" => ".rar",
            _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
        };

        let mut temp_file = NamedTempFile::with_suffix(extension)
            .context("Failed to create temp file for nested archive")?;

        // Write archive data to temp file
        temp_file
            .write_all(&archive_data)
            .context("Failed to write nested archive to temp file")?;
        temp_file.flush()?;

        // Get the temp file path
        let temp_path = temp_file.path();

        // Scan the temp file
        let result = self.scan_path_with_context(temp_path, ctx);

        // Temp file is automatically deleted when NamedTempFile drops
        result
    }

    /// Scan a ZIP archive
    fn scan_zip(&self, zip_path: &Path, password: Option<&str>) -> Result<Vec<DetectedItem>> {
        use zip::ZipArchive;

        let file = fs::File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        // Check for empty archive
        if archive.len() == 0 {
            logger::log_info(
                &format!("Empty ZIP archive: {}", zip_path.display()),
                Some("scanner"),
            );
            return Ok(Vec::new());
        }

        // Convert password to bytes if provided
        let password_bytes = password.map(|p| p.as_bytes());

        // Single pass: collect file info, check encryption, and identify markers
        let enumerate_start = std::time::Instant::now();
        let mut plugin_dirs: HashSet<String> = HashSet::new();
        let mut aircraft_dirs: HashSet<String> = HashSet::new();
        let mut marker_files: Vec<(usize, String, bool, &str)> = Vec::new(); // (index, path, encrypted, marker_type)
        let mut has_encrypted = false;
        let mut detected_livery_roots: HashSet<String> = HashSet::new();

        for i in 0..archive.len() {
            let file = match archive.by_index_raw(i) {
                Ok(f) => f,
                Err(e) => {
                    let err_str = format!("{:?}", e);
                    if err_str.contains("password")
                        || err_str.contains("Password")
                        || err_str.contains("encrypted")
                        || err_str.contains("InvalidPassword")
                    {
                        if password_bytes.is_none() {
                            return Err(anyhow::anyhow!(PasswordRequiredError {
                                archive_path: zip_path.to_string_lossy().to_string(),
                            }));
                        } else {
                            return Err(anyhow::anyhow!(
                                "Wrong password for archive: {}",
                                zip_path.display()
                            ));
                        }
                    }
                    return Err(anyhow::anyhow!("Failed to read ZIP entry {}: {}", i, e));
                }
            };

            let is_encrypted = file.encrypted();
            if is_encrypted {
                has_encrypted = true;
            }

            let file_path = file.name().replace('\\', "/");

            // Skip ignored paths
            if Self::should_ignore_archive_path(&file_path) {
                continue;
            }

            // Check for livery patterns first (before any potential moves)
            if let Some((_, livery_root)) = livery_patterns::check_livery_pattern(&file_path) {
                if !detected_livery_roots.contains(&livery_root) {
                    detected_livery_roots.insert(livery_root.clone());
                    marker_files.push((i, file_path.clone(), is_encrypted, "livery"));
                }
            }

            // Identify plugin directories, aircraft directories, and marker files
            if file_path.ends_with(".xpl") {
                if let Some(parent) = Path::new(&file_path).parent() {
                    let parent_str = parent.to_string_lossy();
                    let parent_name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");

                    let plugin_root = if matches!(
                        parent_name,
                        "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
                    ) {
                        parent
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(parent_str.to_string())
                    } else {
                        parent_str.to_string()
                    };
                    plugin_dirs.insert(plugin_root);
                }
                marker_files.push((i, file_path, is_encrypted, "xpl"));
            } else if file_path.ends_with(".acf") {
                // Track aircraft directories to skip embedded plugins
                if let Some(parent) = Path::new(&file_path).parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if !parent_str.is_empty() {
                        aircraft_dirs.insert(parent_str);
                    }
                }
                marker_files.push((i, file_path, is_encrypted, "acf"));
            } else if file_path.ends_with("library.txt") {
                marker_files.push((i, file_path, is_encrypted, "library"));
            } else if file_path.ends_with(".dsf") {
                marker_files.push((i, file_path, is_encrypted, "dsf"));
            } else if file_path.ends_with("cycle.json") {
                marker_files.push((i, file_path, is_encrypted, "navdata"));
            } else if file_path.ends_with(".lua") {
                // Lua script detection (lowest priority)
                marker_files.push((i, file_path, is_encrypted, "lua"));
            }
        }

        crate::log_debug!(
            &format!(
                "[TIMING] ZIP enumeration completed in {:.2}ms: {} files, {} markers",
                enumerate_start.elapsed().as_secs_f64() * 1000.0,
                archive.len(),
                marker_files.len(),
            ),
            "scanner_timing"
        );

        // If any file is encrypted but no password provided, request password
        if has_encrypted && password_bytes.is_none() {
            return Err(anyhow::anyhow!(PasswordRequiredError {
                archive_path: zip_path.to_string_lossy().to_string(),
            }));
        }

        // If password was provided and archive has encrypted files, verify password by trying to read first encrypted file
        if has_encrypted && password_bytes.is_some() {
            let pwd = password_bytes.unwrap();
            // Find first encrypted file index
            let mut encrypted_index: Option<usize> = None;
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index_raw(i) {
                    if file.encrypted() && !file.is_dir() {
                        encrypted_index = Some(i);
                        break;
                    }
                }
            }
            // Try to decrypt the first encrypted file
            if let Some(idx) = encrypted_index {
                use std::io::Read;
                match archive.by_index_decrypt(idx, pwd) {
                    Ok(mut f) => {
                        let mut buf = [0u8; 1];
                        if f.read(&mut buf).is_err() {
                            return Err(anyhow::anyhow!(
                                "Wrong password for archive: {}",
                                zip_path.display()
                            ));
                        }
                    }
                    Err(e) => {
                        let err_str = format!("{:?}", e);
                        if err_str.contains("password")
                            || err_str.contains("Password")
                            || err_str.contains("InvalidPassword")
                        {
                            return Err(anyhow::anyhow!(
                                "Wrong password for archive: {}",
                                zip_path.display()
                            ));
                        }
                    }
                }
            }
        }

        // Sort marker files by depth, then by type priority (aircraft first)
        marker_files.sort_by(|a, b| {
            let depth_a = a.1.matches('/').count();
            let depth_b = b.1.matches('/').count();
            match depth_a.cmp(&depth_b) {
                std::cmp::Ordering::Equal => {
                    // Same depth: sort by marker type priority (aircraft first)
                    Self::marker_type_priority(a.3).cmp(&Self::marker_type_priority(b.3))
                }
                other => other,
            }
        });

        let mut detected = Vec::new();
        let mut skip_prefixes: Vec<String> = Vec::new();

        // Process marker files
        for (i, file_path, is_encrypted, marker_type) in marker_files {
            // Check if inside a skip prefix (already detected addon)
            let should_skip = skip_prefixes
                .iter()
                .any(|prefix| file_path.starts_with(prefix));
            if should_skip {
                continue;
            }

            // Check if .acf/.dsf is inside a plugin directory
            if marker_type == "acf" || marker_type == "dsf" {
                if Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs) {
                    continue;
                }
            }

            // Skip .xpl inside aircraft directories (embedded plugins)
            if marker_type == "xpl" {
                if Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs) {
                    continue;
                }
            }

            // Detect addon based on marker type
            let item = match marker_type {
                "acf" => self.detect_aircraft_in_archive(&file_path, zip_path)?,
                "library" => self.detect_scenery_library(&file_path, zip_path)?,
                "dsf" => self.detect_scenery_dsf(&file_path, zip_path)?,
                "xpl" => self.detect_plugin_in_archive(&file_path, zip_path)?,
                "navdata" => {
                    // Need to read cycle.json content
                    let mut content = String::new();
                    use std::io::Read;

                    let read_ok = if is_encrypted {
                        if let Some(pwd) = password_bytes {
                            match archive.by_index_decrypt(i, pwd) {
                                Ok(mut f) => f.read_to_string(&mut content).is_ok(),
                                Err(_) => false,
                            }
                        } else {
                            continue;
                        }
                    } else {
                        match archive.by_index(i) {
                            Ok(mut f) => f.read_to_string(&mut content).is_ok(),
                            Err(_) => false,
                        }
                    };

                    if read_ok {
                        self.detect_navdata_in_archive(&file_path, &content, zip_path)?
                    } else {
                        None
                    }
                }
                "livery" => self.detect_livery_in_archive(&file_path, zip_path)?,
                "lua" => self.detect_lua_script_in_archive(&file_path, zip_path)?,
                _ => None,
            };

            if let Some(item) = item {
                // Add to skip prefixes
                if let Some(ref internal_root) = item.archive_internal_root {
                    let prefix = if internal_root.ends_with('/') {
                        internal_root.clone()
                    } else {
                        format!("{}/", internal_root)
                    };
                    skip_prefixes.push(prefix);
                } else if item.addon_type == AddonType::Aircraft {
                    // Aircraft at archive root: skip all other markers in this archive
                    skip_prefixes.push(String::new());
                }
                detected.push(item);
            }
        }

        Ok(detected)
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
        if !cycle.name.contains("X-Plane") && !cycle.name.contains("X-Plane 11") && !cycle.name.contains("X-Plane GNS430") {
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
        if !cycle.name.contains("X-Plane") && !cycle.name.contains("X-Plane 11") && !cycle.name.contains("X-Plane GNS430") {
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
            let aircraft_name = livery_patterns::get_aircraft_name(aircraft_type_id)
                .unwrap_or(aircraft_type_id);

            Ok(Some(DetectedItem {
                original_input_path: String::new(),
                addon_type: AddonType::Livery,
                path: livery_path.to_string_lossy().to_string(),
                display_name: format!("{} ({})", display_name, aircraft_name),
                archive_internal_root: None,
                extraction_chain: None,
                navdata_info: None,
                livery_aircraft_type: Some(aircraft_type_id.to_string()),
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
            let aircraft_name = livery_patterns::get_aircraft_name(aircraft_type_id)
                .unwrap_or(aircraft_type_id);

            Ok(Some(DetectedItem {
                original_input_path: String::new(),
                addon_type: AddonType::Livery,
                path: archive_path.to_string_lossy().to_string(),
                display_name: format!("{} ({})", display_name, aircraft_name),
                archive_internal_root: internal_root,
                extraction_chain: None,
                navdata_info: None,
                livery_aircraft_type: Some(aircraft_type_id.to_string()),
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

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::models::{AddonType, DetectedItem, NavdataCycle, NavdataInfo};

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

/// Scans a directory or archive and detects addon types based on markers
///
/// Scanner is thread-safe as it contains no mutable state.
/// All methods are stateless and can be called concurrently.
pub struct Scanner;

// Explicitly mark Scanner as Send and Sync for thread safety
// This is safe because Scanner has no internal state
unsafe impl Send for Scanner {}
unsafe impl Sync for Scanner {}

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

    /// Scan a path (file or directory) and detect all addon types
    pub fn scan_path(&self, path: &Path, password: Option<&str>) -> Result<Vec<DetectedItem>> {
        let mut detected_items = Vec::new();

        if path.is_dir() {
            detected_items.extend(self.scan_directory(path)?);
        } else if path.is_file() {
            // For archives, we need to extract to temp first or scan without extraction
            // For now, we'll handle archives by treating them as potential root containers
            detected_items.extend(self.scan_archive(path, password)?);
        }

        Ok(detected_items)
    }

    /// Scan a directory recursively
    fn scan_directory(&self, dir: &Path) -> Result<Vec<DetectedItem>> {
        let mut detected = Vec::new();

        // Walk through the directory with depth limit for performance
        // Most X-Plane addons are within 15 levels deep
        for entry in WalkDir::new(dir)
            .follow_links(false)
            .max_depth(15)
        {
            let entry = entry?;
            let path = entry.path();

            // Skip ignored paths (__MACOSX, .DS_Store, etc.)
            if Self::should_ignore_path(path) {
                continue;
            }

            if !entry.file_type().is_file() {
                continue;
            }

            // Check for different addon types based on file markers
            if let Some(item) = self.check_aircraft(path, dir)? {
                detected.push(item);
            } else if let Some(item) = self.check_scenery(path, dir)? {
                detected.push(item);
            } else if let Some(item) = self.check_plugin(path, dir)? {
                detected.push(item);
            } else if let Some(item) = self.check_navdata(path, dir)? {
                detected.push(item);
            }
        }

        Ok(detected)
    }

    /// Scan archive without full extraction
    fn scan_archive(&self, archive_path: &Path, password: Option<&str>) -> Result<Vec<DetectedItem>> {
        let extension = archive_path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "zip" => self.scan_zip(archive_path, password),
            "7z" => self.scan_7z(archive_path, password),
            "rar" => self.scan_rar(archive_path, password),
            "" => {
                // No extension
                let msg = format!("File has no extension: {}", archive_path.display());
                crate::logger::log_error(&msg, Some("scanner"));
                Err(anyhow::anyhow!(msg))
            }
            _ => {
                // Unsupported format
                let msg = format!(
                    "Unsupported archive format '.{}' for file: {}. Supported formats: .zip, .7z, .rar",
                    extension,
                    archive_path.display()
                );
                crate::logger::log_error(&msg, Some("scanner"));
                Err(anyhow::anyhow!(msg))
            }
        }
    }

    /// Scan a 7z archive without extraction
    fn scan_7z(&self, archive_path: &Path, password: Option<&str>) -> Result<Vec<DetectedItem>> {
        // sevenz-rust2 requires password at decompression time, not for listing
        // Try to open the archive for listing first
        let archive = sevenz_rust2::Archive::open(archive_path)
            .map_err(|e| {
                let err_str = format!("{:?}", e);
                // Check for password-related errors
                if err_str.contains("password") || err_str.contains("Password") || err_str.contains("encrypted") {
                    // If we don't have a password, request one
                    if password.is_none() {
                        anyhow::anyhow!(PasswordRequiredError {
                            archive_path: archive_path.to_string_lossy().to_string(),
                        })
                    } else {
                        // Password was provided but still failed - wrong password
                        anyhow::anyhow!("Wrong password for archive: {}", archive_path.display())
                    }
                } else {
                    anyhow::anyhow!("Failed to open 7z archive: {}", e)
                }
            })?;

        let mut detected = Vec::new();

        // Collect all file paths from the archive
        let files: Vec<String> = archive.files
            .iter()
            .map(|entry| entry.name().to_string())
            .collect();

        // Process files using the same logic as ZIP
        for file_path in &files {
            // Skip ignored paths (__MACOSX, .DS_Store, etc.)
            let path = Path::new(file_path);
            if Self::should_ignore_path(path) {
                continue;
            }

            if file_path.ends_with(".acf") {
                if let Some(item) = self.detect_aircraft_in_archive(file_path, archive_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with("library.txt") {
                if let Some(item) = self.detect_scenery_library(file_path, archive_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with(".dsf") {
                if let Some(item) = self.detect_scenery_dsf(file_path, archive_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with(".xpl") {
                if let Some(item) = self.detect_plugin_in_archive(file_path, archive_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with("cycle.json") {
                // For 7z, we need to extract this single file to read its content
                if let Ok(content) = self.read_file_from_7z(archive_path, file_path, password) {
                    if let Some(item) = self.detect_navdata_in_archive(file_path, &content, archive_path)? {
                        detected.push(item);
                    }
                }
            }
        }

        Ok(detected)
    }

    /// Read a single file content from a 7z archive
    fn read_file_from_7z(&self, archive_path: &Path, file_path: &str, password: Option<&str>) -> Result<String> {
        // Create secure temp directory using tempfile crate
        let temp_dir = tempfile::Builder::new()
            .prefix("xfi_7z_read_")
            .tempdir()
            .context("Failed to create secure temp directory")?;

        // Extract to temp (with password if provided)
        if let Some(pwd) = password {
            let mut reader = sevenz_rust2::SevenZReader::open(archive_path, sevenz_rust2::Password::from(pwd))
                .map_err(|e| anyhow::anyhow!("Failed to open 7z with password: {}", e))?;
            reader.for_each_entries(|entry, reader| {
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
            }).map_err(|e| anyhow::anyhow!("Failed to extract 7z with password: {}", e))?;
        } else {
            sevenz_rust2::decompress_file(archive_path, temp_dir.path())
                .map_err(|e| anyhow::anyhow!("Failed to extract 7z: {}", e))?;
        }

        // Sanitize the file path to prevent path traversal using proper sanitization
        let safe_path = crate::installer::sanitize_path(Path::new(file_path))
            .ok_or_else(|| anyhow::anyhow!("Unsafe path in 7z archive: {}", file_path))?;
        let target_file = temp_dir.path().join(safe_path);
        let content = fs::read_to_string(&target_file)
            .context("Failed to read file from 7z")?;

        // TempDir automatically cleans up when dropped
        Ok(content)
    }

    /// Scan a RAR archive without extraction
    fn scan_rar(&self, archive_path: &Path, password: Option<&str>) -> Result<Vec<DetectedItem>> {
        // Create archive with or without password
        let archive_builder = if let Some(pwd) = password {
            unrar::Archive::with_password(archive_path, pwd)
        } else {
            unrar::Archive::new(archive_path)
        };

        let archive = archive_builder
            .open_for_listing()
            .map_err(|e| {
                let err_str = format!("{:?}", e);
                // Check for password-related errors
                if err_str.contains("password") || err_str.contains("Password")
                    || err_str.contains("encrypted") || err_str.contains("ERAR_MISSING_PASSWORD") {
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

        let mut detected = Vec::new();
        let mut files: Vec<String> = Vec::new();

        // Collect all file paths
        for entry in archive {
            if let Ok(e) = entry {
                files.push(e.filename.to_string_lossy().to_string());
            }
        }

        // Process files using the same logic as ZIP
        for file_path in &files {
            // Skip ignored paths (__MACOSX, .DS_Store, etc.)
            let path = Path::new(file_path);
            if Self::should_ignore_path(path) {
                continue;
            }

            if file_path.ends_with(".acf") {
                if let Some(item) = self.detect_aircraft_in_archive(file_path, archive_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with("library.txt") {
                if let Some(item) = self.detect_scenery_library(file_path, archive_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with(".dsf") {
                if let Some(item) = self.detect_scenery_dsf(file_path, archive_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with(".xpl") {
                if let Some(item) = self.detect_plugin_in_archive(file_path, archive_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with("cycle.json") {
                // For RAR, we need to extract this single file to read its content
                if let Ok(content) = self.read_file_from_rar(archive_path, file_path, password) {
                    if let Some(item) = self.detect_navdata_in_archive(file_path, &content, archive_path)? {
                        detected.push(item);
                    }
                }
            }
        }

        Ok(detected)
    }

    /// Read a single file content from a RAR archive
    fn read_file_from_rar(&self, archive_path: &Path, target_file: &str, password: Option<&str>) -> Result<String> {
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

        while let Some(header) = archive.read_header()
            .map_err(|e| anyhow::anyhow!("Failed to read RAR header: {:?}", e))?
        {
            archive = if header.entry().is_file() {
                header.extract_with_base(temp_dir.path())
                    .map_err(|e| anyhow::anyhow!("Failed to extract RAR entry: {:?}", e))?
            } else {
                header.skip()
                    .map_err(|e| anyhow::anyhow!("Failed to skip RAR entry: {:?}", e))?
            };
        }

        // Sanitize the file path to prevent path traversal using proper sanitization
        let safe_path = crate::installer::sanitize_path(Path::new(target_file))
            .ok_or_else(|| anyhow::anyhow!("Unsafe path in RAR archive: {}", target_file))?;
        let file_path = temp_dir.path().join(safe_path);
        let content = fs::read_to_string(&file_path)
            .context("Failed to read file from RAR")?;

        // TempDir automatically cleans up when dropped
        Ok(content)
    }

    /// Scan a ZIP archive
    fn scan_zip(&self, zip_path: &Path, password: Option<&str>) -> Result<Vec<DetectedItem>> {
        use zip::ZipArchive;

        let file = fs::File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut detected = Vec::new();

        // Convert password to bytes if provided
        let password_bytes = password.map(|p| p.as_bytes());

        // First pass: collect file info and check for encryption
        let mut files_info = Vec::new();
        let mut has_encrypted = false;
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            if file.encrypted() {
                has_encrypted = true;
            }
            files_info.push((i, file.name().to_string(), file.encrypted()));
        }

        // If any file is encrypted but no password provided, request password
        if has_encrypted && password_bytes.is_none() {
            return Err(anyhow::anyhow!(PasswordRequiredError {
                archive_path: zip_path.to_string_lossy().to_string(),
            }));
        }

        // Second pass: process files
        for (i, file_path, is_encrypted) in files_info {
            // Skip ignored paths (__MACOSX, .DS_Store, etc.)
            let path = Path::new(&file_path);
            if Self::should_ignore_path(path) {
                continue;
            }

            // Check for markers in the archive
            if file_path.ends_with(".acf") {
                if let Some(item) = self.detect_aircraft_in_archive(&file_path, zip_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with("library.txt") {
                if let Some(item) = self.detect_scenery_library(&file_path, zip_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with(".dsf") {
                if let Some(item) = self.detect_scenery_dsf(&file_path, zip_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with(".xpl") {
                if let Some(item) = self.detect_plugin_in_archive(&file_path, zip_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with("cycle.json") {
                // Need to read the file content
                let mut content = String::new();
                use std::io::Read;

                // Try to read with or without password
                if is_encrypted {
                    if let Some(pwd) = password_bytes {
                        match archive.by_index_decrypt(i, pwd) {
                            Ok(mut file) => {
                                file.read_to_string(&mut content)?;
                            }
                            Err(_) => {
                                // Wrong password
                                return Err(anyhow::anyhow!("Wrong password for archive: {}", zip_path.display()));
                            }
                        }
                    } else {
                        // Should not reach here due to earlier check
                        continue;
                    }
                } else {
                    let mut file = archive.by_index(i)?;
                    file.read_to_string(&mut content)?;
                }

                if let Some(item) = self.detect_navdata_in_archive(&file_path, &content, zip_path)? {
                    detected.push(item);
                }
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
        let parent = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Failed to get parent directory")
        })?;

        // If .acf is in root, use the root folder name
        // Otherwise, use the immediate parent folder
        let install_path = if parent == root {
            root.to_path_buf()
        } else {
            parent.to_path_buf()
        };

        let display_name = install_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Aircraft")
            .to_string();

        Ok(Some(DetectedItem {
            addon_type: AddonType::Aircraft,
            path: install_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: None,
            navdata_info: None,
        }))
    }

    fn detect_aircraft_in_archive(&self, file_path: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);
        let parent = path.parent();

        // Determine the aircraft root folder inside the archive
        let (display_name, internal_root) = if let Some(p) = parent {
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
                // Get the top-level folder in the archive
                let components: Vec<_> = p.components().collect();
                let top_folder = components.first()
                    .map(|c| c.as_os_str().to_string_lossy().to_string());

                // Use parent folder name as display name
                let name = p.file_name()
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
            addon_type: AddonType::Aircraft,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            navdata_info: None,
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
        let parent = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Failed to get parent directory")
        })?;

        let display_name = parent
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Library")
            .to_string();

        Ok(Some(DetectedItem {
            addon_type: AddonType::SceneryLibrary,
            path: parent.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: None,
            navdata_info: None,
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
                addon_type: AddonType::Scenery,
                path: install_dir.to_string_lossy().to_string(),
                display_name,
                archive_internal_root: None,
                navdata_info: None,
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

    fn detect_scenery_library(&self, file_path: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
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
                let name = p.file_name()
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
            addon_type: AddonType::SceneryLibrary,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            navdata_info: None,
        }))
    }

    fn detect_scenery_dsf(&self, file_path: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
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
                let name = root.file_name()
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
            addon_type: AddonType::Scenery,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            navdata_info: None,
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

        let parent = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Failed to get parent directory")
        })?;

        let parent_name = parent
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

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

        Ok(Some(DetectedItem {
            addon_type: AddonType::Plugin,
            path: install_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: None,
            navdata_info: None,
        }))
    }

    fn detect_plugin_in_archive(&self, file_path: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
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
                    let name = root.file_name()
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
            addon_type: AddonType::Plugin,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            navdata_info: None,
        }))
    }

    // Type D: Navdata Detection
    fn check_navdata(&self, file_path: &Path, _root: &Path) -> Result<Option<DetectedItem>> {
        if file_path.file_name().and_then(|s| s.to_str()) != Some("cycle.json") {
            return Ok(None);
        }

        let content = fs::read_to_string(file_path)
            .context("Failed to read cycle.json")?;

        let cycle: NavdataCycle = serde_json::from_str(&content)
            .context("Failed to parse cycle.json")?;

        let parent = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Failed to get parent directory")
        })?;

        // Create NavdataInfo from parsed cycle
        let navdata_info = NavdataInfo {
            name: cycle.name.clone(),
            cycle: cycle.cycle.clone(),
            airac: cycle.airac.clone(),
        };

        // Determine install path based on name
        let (install_path, display_name) = if cycle.name.contains("X-Plane") || cycle.name.contains("X-Plane 11") {
            (parent.to_path_buf(), format!("Navdata: {}", cycle.name))
        } else if cycle.name.contains("X-Plane GNS430") {
            (parent.to_path_buf(), format!("Navdata GNS430: {}", cycle.name))
        } else {
            return Err(anyhow::anyhow!("Unknown Navdata Format: {}", cycle.name));
        };

        Ok(Some(DetectedItem {
            addon_type: AddonType::Navdata,
            path: install_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: None,
            navdata_info: Some(navdata_info),
        }))
    }

    fn detect_navdata_in_archive(&self, file_path: &str, content: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
        let cycle: NavdataCycle = serde_json::from_str(content)
            .context("Failed to parse cycle.json")?;

        let path = PathBuf::from(file_path);
        let parent = path.parent();

        // Create NavdataInfo from parsed cycle
        let navdata_info = NavdataInfo {
            name: cycle.name.clone(),
            cycle: cycle.cycle.clone(),
            airac: cycle.airac.clone(),
        };

        let display_name = if cycle.name.contains("X-Plane") || cycle.name.contains("X-Plane 11") {
            format!("Navdata: {}", cycle.name)
        } else if cycle.name.contains("X-Plane GNS430") {
            format!("Navdata GNS430: {}", cycle.name)
        } else {
            return Err(anyhow::anyhow!("Unknown Navdata Format: {}", cycle.name));
        };

        // Get the navdata folder root inside the archive
        let internal_root = if let Some(p) = parent {
            if p.as_os_str().is_empty() {
                None
            } else {
                Some(p.to_string_lossy().to_string())
            }
        } else {
            None
        };

        Ok(Some(DetectedItem {
            addon_type: AddonType::Navdata,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
            archive_internal_root: internal_root,
            navdata_info: Some(navdata_info),
        }))
    }
}

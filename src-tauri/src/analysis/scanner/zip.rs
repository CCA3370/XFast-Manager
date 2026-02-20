use super::*;

impl Scanner {
    /// Try to scan a ZIP file by loading it into memory (optimization)
    pub(super) fn try_scan_zip_from_file_to_memory(
        &self,
        zip_path: &Path,
        parent_path: &Path,
        ctx: &mut ScanContext,
    ) -> Result<Vec<DetectedItem>> {
        use std::io::{Cursor, Read};
        use ::zip::ZipArchive;

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

    /// Scan a ZIP archive with context (supports nested archives)
    /// OPTIMIZED: Single pass to collect both addon markers and nested archives
    pub(super) fn scan_zip_with_context(
        &self,
        zip_path: &Path,
        ctx: &mut ScanContext,
        password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        use std::io::Read;
        use ::zip::ZipArchive;

        let scan_start = std::time::Instant::now();
        crate::log_debug!(
            &format!("[TIMING] ZIP scan started: {}", zip_path.display()),
            "scanner_timing"
        );

        let open_start = std::time::Instant::now();
        let file = fs::File::open(zip_path)?;
        let mut archive: ZipArchive<fs::File> = ZipArchive::new(file)?;
        crate::log_debug!(
            &format!(
                "[TIMING] ZIP open completed in {:.2}ms: {}",
                open_start.elapsed().as_secs_f64() * 1000.0,
                zip_path.display()
            ),
            "scanner_timing"
        );

        // Check for empty archive
        if archive.is_empty() {
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
            let file: ::zip::read::ZipFile<'_> = match archive.by_index_raw(i) {
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
        if has_encrypted {
            if let Some(pwd) = password_bytes {
                // Find first encrypted file index
                let mut encrypted_index: Option<usize> = None;
                for i in 0..archive.len() {
                    if let Ok(file) = archive.by_index_raw(i) {
                        let file: ::zip::read::ZipFile<'_> = file;
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
            if (marker_type == "acf" || marker_type == "dsf")
                && Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs)
            {
                continue;
            }

            // Skip .xpl inside aircraft directories (embedded plugins)
            if marker_type == "xpl"
                && Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs)
            {
                continue;
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
                    let prefix: String = if internal_root.ends_with('/') {
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

                let params = NestedZipScanParams {
                    parent_archive: &mut archive,
                    file_index: index,
                    nested_path: nested_path.as_str(),
                    parent_path: zip_path,
                    ctx,
                    parent_password: password_bytes,
                    is_encrypted,
                };
                match self.scan_nested_archive_in_zip(params) {
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
    pub(super) fn scan_nested_archive_in_zip(
        &self,
        params: NestedZipScanParams<'_>,
    ) -> Result<Vec<DetectedItem>> {
        let NestedZipScanParams {
            parent_archive,
            file_index,
            nested_path,
            parent_path,
            ctx,
            parent_password,
            is_encrypted,
        } = params;
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
            ctx.get_nested_password(parent_path.to_string_lossy().as_ref(), nested_path);

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
            match ::zip::ZipArchive::new(cursor) {
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
    pub(super) fn scan_zip_in_memory(
        &self,
        archive: &mut ::zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
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
            let file: ::zip::read::ZipFile<'_> = archive.by_index_raw(i)?;
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
            if (marker_type == "acf" || marker_type == "dsf")
                && Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs)
            {
                continue;
            }

            // Skip .xpl inside aircraft directories (embedded plugins)
            if marker_type == "xpl"
                && Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs)
            {
                continue;
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
                    let prefix: String = if internal_root.ends_with('/') {
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
    pub(super) fn scan_nested_non_zip_from_memory(
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

        // Temp file is automatically deleted when NamedTempFile drops
        self.scan_path_with_context(temp_path, ctx)
    }

    /// Scan a ZIP archive
    pub(super) fn scan_zip(
        &self,
        zip_path: &Path,
        password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        use ::zip::ZipArchive;

        let file = fs::File::open(zip_path)?;
        let mut archive: ZipArchive<fs::File> = ZipArchive::new(file)?;

        // Check for empty archive
        if archive.is_empty() {
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
            let file: ::zip::read::ZipFile<'_> = match archive.by_index_raw(i) {
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
        if has_encrypted {
            if let Some(pwd) = password_bytes {
                // Find first encrypted file index
                let mut encrypted_index: Option<usize> = None;
                for i in 0..archive.len() {
                    if let Ok(file) = archive.by_index_raw(i) {
                        let file: ::zip::read::ZipFile<'_> = file;
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
            if (marker_type == "acf" || marker_type == "dsf")
                && Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs)
            {
                continue;
            }

            // Skip .xpl inside aircraft directories (embedded plugins)
            if marker_type == "xpl"
                && Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs)
            {
                continue;
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
                    let prefix: String = if internal_root.ends_with('/') {
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
}

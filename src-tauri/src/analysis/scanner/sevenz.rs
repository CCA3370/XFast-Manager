use super::*;

impl Scanner {
    /// Scan a 7z archive with context (supports nested archives)
    /// OPTIMIZED: Single pass to collect both addon markers and nested archives
    pub(super) fn scan_7z_with_context(
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
    pub(super) fn scan_nested_archive_in_7z(
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
            ctx.get_nested_password(parent_path.to_string_lossy().as_ref(), nested_path);

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
                "Optimizing: Loading nested ZIP from 7z into memory for scanning",
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
    pub(super) fn scan_7z(
        &self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
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
    pub(super) fn read_file_from_7z(
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
}

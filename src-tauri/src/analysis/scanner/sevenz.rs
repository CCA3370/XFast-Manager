use super::*;

impl Scanner {
    /// Fast metadata-only check for encrypted 7z content.
    /// Avoids probing archive streams during scan.
    fn archive_has_encrypted_blocks(archive: &sevenz_rust2::Archive) -> bool {
        archive.blocks.iter().any(|block| {
            block.coders.iter().any(|coder| {
                coder.encoder_method_id() == sevenz_rust2::EncoderMethod::ID_AES256_SHA256
            })
        })
    }

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

        let prepared_archive = crate::archive_input::prepare_archive_for_read(
            archive_path,
            crate::archive_input::ArchiveFormat::SevenZ,
        )?;
        let read_archive_path = prepared_archive.read_path();

        // Open archive to read file list (fast, no decompression)
        let open_start = std::time::Instant::now();
        let archive = match sevenz_rust2::Archive::open(read_archive_path) {
            Ok(a) => a,
            Err(e) => {
                let err_str = format!("{:?}", e);
                if password.is_none()
                    && (err_str.contains("password")
                        || err_str.contains("Password")
                        || err_str.contains("encrypted")
                        || err_str.contains("WrongPassword"))
                {
                    return Err(anyhow::anyhow!(PasswordRequiredError {
                        archive_path: archive_path.to_string_lossy().to_string(),
                    }));
                }
                return Err(anyhow::anyhow!("Failed to open 7z archive: {}", e));
            }
        };
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

        // Fast path: encryption detection from metadata only.
        if password.is_none() && Self::archive_has_encrypted_blocks(&archive) {
            return Err(anyhow::anyhow!(PasswordRequiredError {
                archive_path: archive_path.to_string_lossy().to_string(),
            }));
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
        let mut archive_entries: Vec<String> = Vec::new();

        for entry in &archive.files {
            let file_path = entry.name().to_string();
            let normalized = file_path.replace('\\', "/");

            if !entry.is_directory() && entry.has_stream() {
                archive_entries.push(normalized.clone());
            }

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
        let mut marker_text_cache: HashMap<String, String> = HashMap::new();
        let mut text_reader: Option<sevenz_rust2::ArchiveReader<std::fs::File>> = None;

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
                    let content = if let Some(cached) = marker_text_cache.get(&file_path) {
                        Some(cached.clone())
                    } else {
                        if text_reader.is_none() {
                            let pwd = match password {
                                Some(pwd) => sevenz_rust2::Password::from(pwd),
                                None => sevenz_rust2::Password::empty(),
                            };
                            text_reader =
                                sevenz_rust2::ArchiveReader::open(read_archive_path, pwd).ok();
                        }

                        let read_result = if let Some(reader) = text_reader.as_mut() {
                            reader
                                .read_file(&file_path)
                                .ok()
                                .and_then(|bytes| String::from_utf8(bytes).ok())
                        } else {
                            None
                        };

                        let final_content = read_result.or_else(|| {
                            self.read_file_from_7z(archive_path, &file_path, password)
                                .ok()
                        });
                        if let Some(ref content) = final_content {
                            marker_text_cache.insert(file_path.clone(), content.clone());
                        }
                        final_content
                    };

                    if let Some(content) = content {
                        self.detect_navdata_in_archive(&file_path, &content, archive_path)?
                    } else {
                        None
                    }
                }
                "livery" => self.detect_livery_in_archive(&file_path, archive_path)?,
                "lua" => {
                    // Solid 7z random-read is expensive; defer companion parsing to install stage.
                    let lua_content = if archive.is_solid {
                        Some(String::new())
                    } else if let Some(cached) = marker_text_cache.get(&file_path) {
                        Some(cached.clone())
                    } else {
                        if text_reader.is_none() {
                            let pwd = match password {
                                Some(pwd) => sevenz_rust2::Password::from(pwd),
                                None => sevenz_rust2::Password::empty(),
                            };
                            text_reader =
                                sevenz_rust2::ArchiveReader::open(read_archive_path, pwd).ok();
                        }

                        let read_result = if let Some(reader) = text_reader.as_mut() {
                            reader
                                .read_file(&file_path)
                                .ok()
                                .and_then(|bytes| String::from_utf8(bytes).ok())
                        } else {
                            None
                        };

                        let final_content = read_result.or_else(|| {
                            self.read_file_from_7z(archive_path, &file_path, password)
                                .ok()
                        });
                        if let Some(ref content) = final_content {
                            marker_text_cache.insert(file_path.clone(), content.clone());
                        }
                        final_content
                    };

                    self.detect_lua_script_in_archive_with_data(
                        &file_path,
                        archive_path,
                        lua_content.as_deref(),
                        Some(&archive_entries),
                    )?
                }
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

        // Scan nested archives.
        // Performance optimization: if we already found a concrete top-level addon
        // (non-Lua), skip nested scans for this 7z.
        let has_top_level_concrete_addon = detected
            .iter()
            .any(|item| item.addon_type != AddonType::LuaScript);

        if has_top_level_concrete_addon && !nested_archives.is_empty() {
            crate::log_debug!(
                &format!(
                    "[TIMING] 7z nested archive scan skipped: top-level addon already detected ({} nested archives)",
                    nested_archives.len()
                ),
                "scanner_timing"
            );
        } else if !nested_archives.is_empty() {
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

        // Extract only the nested archive entry (avoid full 7z extraction).
        let safe_nested_path = crate::installer::sanitize_path(Path::new(nested_path))
            .ok_or_else(|| anyhow::anyhow!("Unsafe nested archive path in 7z: {}", nested_path))?;
        let normalized_target = safe_nested_path.to_string_lossy().replace('\\', "/");
        let fallback_target = nested_path.replace('\\', "/");
        let temp_archive_path = temp_dir.path().join(&safe_nested_path);

        if let Some(parent) = temp_archive_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let prepared_parent = crate::archive_input::prepare_archive_for_read(
            parent_path,
            crate::archive_input::ArchiveFormat::SevenZ,
        )?;
        let read_parent_path = prepared_parent.read_path().to_path_buf();

        let read_nested_entry = |entry_name: &str| -> Result<Vec<u8>> {
            let pwd = match parent_password {
                Some(pwd) => sevenz_rust2::Password::from(pwd),
                None => sevenz_rust2::Password::empty(),
            };

            let mut reader =
                sevenz_rust2::ArchiveReader::open(&read_parent_path, pwd).map_err(|e| {
                let err_str = format!("{:?}", e);
                if err_str.contains("password")
                    || err_str.contains("Password")
                    || err_str.contains("encrypted")
                    || err_str.contains("WrongPassword")
                {
                    if parent_password.is_some() {
                        anyhow::anyhow!("Wrong password for archive: {}", parent_path.display())
                    } else {
                        anyhow::anyhow!(PasswordRequiredError {
                            archive_path: parent_path.to_string_lossy().to_string(),
                        })
                    }
                } else {
                    anyhow::anyhow!("Failed to open 7z archive: {}", e)
                }
            })?;

            reader.read_file(entry_name).map_err(|e| {
                let err_str = format!("{:?}", e);
                if err_str.contains("password")
                    || err_str.contains("Password")
                    || err_str.contains("encrypted")
                    || err_str.contains("WrongPassword")
                {
                    if parent_password.is_some() {
                        anyhow::anyhow!("Wrong password for archive: {}", parent_path.display())
                    } else {
                        anyhow::anyhow!(PasswordRequiredError {
                            archive_path: parent_path.to_string_lossy().to_string(),
                        })
                    }
                } else {
                    anyhow::anyhow!("Failed to read nested archive from 7z: {}", e)
                }
            })
        };

        let nested_bytes = match read_nested_entry(&normalized_target) {
            Ok(data) => data,
            Err(primary_err) if fallback_target != normalized_target => {
                match read_nested_entry(&fallback_target) {
                    Ok(data) => data,
                    Err(_) => return Err(primary_err),
                }
            }
            Err(e) => return Err(e),
        };

        fs::write(&temp_archive_path, nested_bytes).context(format!(
            "Failed to write nested archive to temp file: {:?}",
            temp_archive_path
        ))?;

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
        let prepared_archive = crate::archive_input::prepare_archive_for_read(
            archive_path,
            crate::archive_input::ArchiveFormat::SevenZ,
        )?;
        let read_archive_path = prepared_archive.read_path();

        // Open archive to read file list (fast, no decompression)
        let archive = match sevenz_rust2::Archive::open(read_archive_path) {
            Ok(a) => a,
            Err(e) => {
                let err_str = format!("{:?}", e);
                if password.is_none()
                    && (err_str.contains("password")
                        || err_str.contains("Password")
                        || err_str.contains("encrypted")
                        || err_str.contains("WrongPassword"))
                {
                    return Err(anyhow::anyhow!(PasswordRequiredError {
                        archive_path: archive_path.to_string_lossy().to_string(),
                    }));
                }
                return Err(anyhow::anyhow!("Failed to open 7z archive: {}", e));
            }
        };

        // Check for empty archive
        if archive.files.is_empty() {
            logger::log_info(
                &format!("Empty 7z archive: {}", archive_path.display()),
                Some("scanner"),
            );
            return Ok(Vec::new());
        }

        // Fast metadata-only encryption detection.
        let has_encrypted_content = Self::archive_has_encrypted_blocks(&archive);
        if has_encrypted_content && password.is_none() {
            return Err(anyhow::anyhow!(PasswordRequiredError {
                archive_path: archive_path.to_string_lossy().to_string(),
            }));
        }

        // If a password is provided, validate it with a single entry read.
        if has_encrypted_content {
            if let Some(first_file) = archive
                .files
                .iter()
                .find(|f| !f.is_directory() && f.has_stream())
                .map(|f| f.name().to_string())
            {
                let mut reader = sevenz_rust2::ArchiveReader::open(
                    read_archive_path,
                    sevenz_rust2::Password::from(password.unwrap_or_default()),
                )
                .map_err(|_| {
                    anyhow::anyhow!("Wrong password for archive: {}", archive_path.display())
                })?;

                if let Err(e) = reader.read_file(&first_file) {
                    let err_str = format!("{:?}", e);
                    if err_str.contains("password")
                        || err_str.contains("Password")
                        || err_str.contains("encrypted")
                        || err_str.contains("WrongPassword")
                    {
                        return Err(anyhow::anyhow!(
                            "Wrong password for archive: {}",
                            archive_path.display()
                        ));
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
                "lua" => {
                    if archive.is_solid {
                        self.detect_lua_script_in_archive_with_data(
                            &file_path,
                            archive_path,
                            Some(""),
                            None,
                        )?
                    } else {
                        self.detect_lua_script_in_archive(&file_path, archive_path)?
                    }
                }
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
        use sevenz_rust2::{ArchiveReader, Password};

        let prepared_archive = crate::archive_input::prepare_archive_for_read(
            archive_path,
            crate::archive_input::ArchiveFormat::SevenZ,
        )?;
        let read_archive_path = prepared_archive.read_path().to_path_buf();

        // Sanitize target path to avoid traversal-like patterns
        let safe_path = crate::installer::sanitize_path(Path::new(file_path))
            .ok_or_else(|| anyhow::anyhow!("Unsafe path in 7z archive: {}", file_path))?;
        let target_normalized = safe_path.to_string_lossy().replace('\\', "/");
        let fallback_normalized = file_path.replace('\\', "/");

        // Helper: open reader with optional password
        let open_reader = || -> Result<ArchiveReader<std::fs::File>> {
            let pwd = match password {
                Some(pwd) => Password::from(pwd),
                None => Password::empty(),
            };
            ArchiveReader::open(&read_archive_path, pwd)
                .map_err(|e| anyhow::anyhow!("Failed to open 7z archive: {}", e))
        };

        // ArchiveReader::read_file is significantly faster than extracting whole archive to disk.
        let bytes = {
            let mut reader = open_reader()?;
            match reader.read_file(&target_normalized) {
                Ok(data) => data,
                Err(_) if fallback_normalized != target_normalized => {
                    let mut reader = open_reader()?;
                    reader
                        .read_file(&fallback_normalized)
                        .map_err(|e| anyhow::anyhow!("Failed to read file from 7z: {}", e))?
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to read file from 7z: {}", e));
                }
            }
        };

        String::from_utf8(bytes)
            .map_err(|e| anyhow::anyhow!("Failed to decode UTF-8 file from 7z: {}", e))
    }
}

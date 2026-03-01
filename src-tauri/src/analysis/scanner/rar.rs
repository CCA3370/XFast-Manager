use super::*;

impl Scanner {
    /// Scan a RAR archive with context (supports nested archives via temp extraction)
    pub(super) fn scan_rar_with_context(
        &self,
        archive_path: &Path,
        ctx: &mut ScanContext,
        password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        let normalized_archive_path = crate::archive_input::normalize_archive_entry_path(archive_path);

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
                unrar::Archive::with_password(&normalized_archive_path, pwd)
            } else {
                unrar::Archive::new(&normalized_archive_path)
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
    pub(super) fn scan_nested_archive_in_rar(
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
        let normalized_parent_path = crate::archive_input::normalize_archive_entry_path(parent_path);
        let archive_builder = if let Some(pwd) = parent_password {
            unrar::Archive::with_password(&normalized_parent_path, pwd)
        } else {
            unrar::Archive::new(&normalized_parent_path)
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
                "Optimizing: Loading nested ZIP from RAR into memory for scanning",
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

    /// Scan a RAR archive without extraction
    pub(super) fn scan_rar(
        &self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        let normalized_archive_path = crate::archive_input::normalize_archive_entry_path(archive_path);

        let open_start = std::time::Instant::now();
        crate::log_debug!(
            &format!("[TIMING] RAR open started: {}", archive_path.display()),
            "scanner_timing"
        );

        // Create archive with or without password
        let archive_builder = if let Some(pwd) = password {
            unrar::Archive::with_password(&normalized_archive_path, pwd)
        } else {
            unrar::Archive::new(&normalized_archive_path)
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
        for e in archive.flatten() {
            files.push(e.filename.to_string_lossy().to_string().replace('\\', "/"));
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
    pub(super) fn read_file_from_rar(
        &self,
        archive_path: &Path,
        target_file: &str,
        password: Option<&str>,
    ) -> Result<String> {
        let normalized_archive_path = crate::archive_input::normalize_archive_entry_path(archive_path);

        // Create secure temp directory using tempfile crate
        let temp_dir = tempfile::Builder::new()
            .prefix("xfi_rar_read_")
            .tempdir()
            .context("Failed to create secure temp directory")?;

        // Extract to temp using the typestate pattern (with password if provided)
        let archive_builder = if let Some(pwd) = password {
            unrar::Archive::with_password(&normalized_archive_path, pwd)
        } else {
            unrar::Archive::new(&normalized_archive_path)
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
}

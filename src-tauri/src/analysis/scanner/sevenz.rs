use super::*;
use crate::sevenz_package::SevenzPackage;

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

    pub(super) fn scan_7z_package(
        &self,
        archive_path: &Path,
        package: &SevenzPackage,
        ctx: &mut ScanContext,
        password: Option<&str>,
        scan_start: std::time::Instant,
    ) -> Result<Vec<DetectedItem>> {
        let archive = &package.archive;
        // Check for empty archive
        if archive.files.is_empty() {
            logger::log_info(
                &format!("Empty 7z archive: {}", archive_path.display()),
                Some("scanner"),
            );
            return Ok(Vec::new());
        }

        // Fast path: encryption detection from metadata only.
        if password.is_none() && Self::archive_has_encrypted_blocks(archive) {
            return Err(anyhow::anyhow!(PasswordRequiredError {
                archive_path: archive_path.to_string_lossy().to_string(),
            }));
        }

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
        let mut sound_packs = AircraftSoundPacks::default();
        let mut marker_files: Vec<(String, &str)> = Vec::new();
        let mut nested_archives: Vec<String> = Vec::new();
        let mut detected_livery_roots: HashSet<String> = HashSet::new();
        let mut archive_entries: Vec<String> = Vec::new();

        for entry in &archive.files {
            let file_path = entry.name().to_string();
            let normalized = file_path.replace('\\', "/");
            if !entry.is_directory() {
                sound_packs.record_file(&normalized);
            }

            if !entry.is_directory() && entry.has_stream() {
                archive_entries.push(normalized.clone());
            }

            if Self::should_ignore_archive_path(&normalized) {
                continue;
            }

            if ctx.can_recurse() && !entry.is_directory() && is_archive_file(&normalized) {
                nested_archives.push(normalized.clone());
            }

            if let Some((_, livery_root)) = livery_patterns::check_livery_pattern(&normalized) {
                if !detected_livery_roots.contains(&livery_root) {
                    detected_livery_roots.insert(livery_root.clone());
                    marker_files.push((normalized.clone(), "livery"));
                }
            }

            if normalized.ends_with(".xpl") {
                if let Some(root) = plugin_root(Path::new(&normalized)) {
                    plugin_dirs.insert(root.to_string_lossy().into_owned());
                }
                marker_files.push((normalized, "xpl"));
            } else if normalized.ends_with(".acf") {
                if let Some(parent) = Path::new(&normalized).parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if !parent_str.is_empty() {
                        aircraft_dirs.insert(parent_str);
                    }
                }
                marker_files.push((normalized, "acf"));
            } else if Self::is_library_archive_path(&normalized) {
                marker_files.push((normalized, "library"));
            } else if normalized.ends_with(".dsf") {
                marker_files.push((normalized, "dsf"));
            } else if Self::is_apt_dat_archive_path(&normalized) {
                marker_files.push((normalized, "apt"));
            } else if normalized.ends_with("cycle.json") {
                marker_files.push((normalized, "navdata"));
            } else if normalized.ends_with(".lua") {
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

        let sort_start = std::time::Instant::now();
        marker_files.sort_by(|a, b| {
            let depth_a = a.0.matches('/').count();
            let depth_b = b.0.matches('/').count();
            match depth_a.cmp(&depth_b) {
                std::cmp::Ordering::Equal => {
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
        let marker_text_cache = package.read_texts(
            marker_files
                .iter()
                .filter(|(_, kind)| *kind == "navdata" || (*kind == "lua" && !archive.is_solid))
                .map(|(path, _)| path.clone()),
            None,
        );

        let process_start = std::time::Instant::now();
        crate::log_debug!(
            &format!(
                "[TIMING] 7z marker processing started: {} markers",
                marker_files.len()
            ),
            "scanner_timing"
        );

        let xlua_roots = XluaRuntimeRoots::from_archive_markers(
            marker_files
                .iter()
                .map(|(name, kind)| (name.as_str(), *kind)),
        );
        for (file_path, marker_type) in marker_files {
            if marker_type == "lua" && xlua_roots.contains(Path::new(&file_path)) {
                continue;
            }

            let should_skip = skip_prefixes
                .iter()
                .any(|prefix| file_path.starts_with(prefix));
            if should_skip {
                continue;
            }

            if (marker_type == "acf" || Self::is_scenery_marker_type(marker_type))
                && Self::is_archive_path_inside_plugin_dirs(&file_path, &plugin_dirs)
            {
                continue;
            }

            if sound_packs.contains_plugin_marker(&file_path, marker_type) {
                continue;
            }

            if marker_type == "xpl"
                && Self::is_archive_path_inside_aircraft_dirs(&file_path, &aircraft_dirs)
            {
                continue;
            }

            let item = match marker_type {
                "acf" => {
                    self.detect_aircraft_in_archive_without_version(&file_path, archive_path)?
                }
                "library" => self.detect_scenery_library(&file_path, archive_path)?,
                "dsf" | "apt" => self.detect_scenery_in_archive(&file_path, archive_path)?,
                "xpl" => self.detect_plugin_in_archive_without_version(&file_path, archive_path)?,
                "navdata" => {
                    if let Some(content) = marker_text_cache.get(&file_path) {
                        self.detect_navdata_in_archive(&file_path, content, archive_path)?
                    } else {
                        None
                    }
                }
                "livery" => self.detect_livery_in_archive(&file_path, archive_path)?,
                "lua" => {
                    let lua_content = if archive.is_solid {
                        Some("")
                    } else {
                        marker_text_cache.get(&file_path).map(String::as_str)
                    };

                    self.detect_lua_script_in_archive_with_data(
                        &file_path,
                        archive_path,
                        lua_content,
                        Some(&archive_entries),
                    )?
                }
                _ => None,
            };

            if let Some(mut item) = item {
                xlua_roots.annotate_plugin(&mut item, true);
                if let Some(ref internal_root) = item.archive_internal_root {
                    if item.addon_type != AddonType::LuaScript {
                        let prefix = if internal_root.ends_with('/') {
                            internal_root.clone()
                        } else {
                            format!("{}/", internal_root)
                        };
                        skip_prefixes.push(prefix);
                    }
                } else if item.addon_type == AddonType::Aircraft {
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

        scenery_options::attach_archive_options(
            archive
                .files
                .iter()
                .filter(|entry| !entry.is_directory() && !entry.is_anti_item())
                .map(|entry| entry.name()),
            &mut detected,
        );

        let version_start = std::time::Instant::now();
        let archive_name_hint = ctx
            .parent_chain
            .last()
            .map(|info| info.internal_path.as_str());
        if ctx.is_nested_archive_scan() {
            for item in &mut detected {
                if !matches!(item.addon_type, AddonType::Aircraft | AddonType::Plugin) {
                    continue;
                }

                item.version_info = self.infer_version_info_from_archive_context(
                    item,
                    archive_path,
                    archive_name_hint,
                );
            }

            crate::log_debug!(
                &format!(
                    "[TIMING] 7z version population skipped for nested archive in {:.2}ms",
                    version_start.elapsed().as_secs_f64() * 1000.0
                ),
                "scanner_timing"
            );
        } else {
            self.populate_version_info_from_7z_package(
                &mut detected,
                package,
                archive_path,
                archive_name_hint,
            );
            crate::log_debug!(
                &format!(
                    "[TIMING] 7z version population completed in {:.2}ms",
                    version_start.elapsed().as_secs_f64() * 1000.0
                ),
                "scanner_timing"
            );
        }

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

            let filtered_nested: Vec<_> = nested_archives
                .into_iter()
                .filter(|nested_path| {
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

                match self.scan_nested_archive_in_7z(
                    package,
                    archive_path,
                    &nested_path,
                    ctx,
                    password,
                ) {
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

        let package = SevenzPackage::open(archive_path, password)
            .map_err(|error| classify_7z_password(error, archive_path, password))?;
        let detected = self.scan_7z_package(archive_path, &package, ctx, password, scan_start)?;
        if !ctx.is_nested_archive_scan() {
            ctx.sevenz_summary = Some(ZipSummary {
                source: Some(package.revision()),
                compressed_size: package.compressed_size(),
                uncompressed_size: package
                    .archive
                    .files
                    .iter()
                    .fold(0u64, |size, entry| size.saturating_add(entry.size)),
            });
        }
        package.check_source()?;
        Ok(detected)
    }

    /// Scan a nested archive within a 7z file (extract to temp)
    /// Optimized: If nested archive is ZIP, load into memory for faster scanning
    fn scan_nested_archive_in_7z(
        &self,
        package: &SevenzPackage,
        parent_path: &Path,
        nested_path: &str,
        ctx: &mut ScanContext,
        parent_password: Option<&str>,
    ) -> Result<Vec<DetectedItem>> {
        crate::installer::sanitize_path(Path::new(nested_path)).context("Unsafe nested member")?;
        let format = get_archive_format(nested_path).context("Unknown nested archive format")?;
        let name = nested_path.replace('\\', "/");
        let index = package
            .index(&name)
            .with_context(|| format!("7z member not found: {name}"))?;
        // Check if this nested archive has its own password
        let nested_password = ctx
            .get_nested_password(parent_path.to_string_lossy().as_ref(), nested_path)
            .or_else(|| parent_password.map(str::to_owned));

        // Build nested archive info with password if available
        let nested_info = NestedArchiveInfo {
            internal_path: nested_path.to_string(),
            password: nested_password.clone(),
            format: format.clone(),
        };

        // Push to context chain
        ctx.push_archive(nested_info.clone());

        let nested_result = (|| {
            if matches!(nested_info.format.as_str(), "zip" | "7z") {
                if let Some(input) = package.stored_member(index)? {
                    return self.scan_nested_reader(input, parent_path, ctx, &nested_info);
                }
            }
            let mut temp =
                tempfile::NamedTempFile::with_suffix(&format!(".{}", nested_info.format))?;
            crate::archive_input::archive_workspace_space(
                temp.path(),
                package.archive.files[index].size,
            )?;
            package
                .copy_index(index, &mut temp, || Ok(()))
                .map_err(|error| classify_7z_password(error, parent_path, parent_password))?;
            self.scan_nested_file(temp.path(), parent_path, ctx, &nested_info)
        })();

        // Pop from context chain
        ctx.pop_archive();

        // Process results
        match nested_result {
            Ok(mut items) => {
                // Update each detected item with extraction chain
                for item in &mut items {
                    super::attach_nested_chain(item, parent_path, &nested_info);

                    // Preserve the detected addon name when available.
                    // Only fall back to the nested archive filename for blank/unknown names.
                    item.display_name =
                        super::resolve_nested_display_name(&item.display_name, nested_path);

                    if item.version_info.is_none() {
                        item.version_info = infer_version_from_name(
                            Path::new(nested_path)
                                .file_stem()
                                .and_then(|stem| stem.to_str())
                                .unwrap_or(nested_path),
                        )
                        .map(|version| crate::models::VersionInfo {
                            version: Some(version),
                        });
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
        let mut ctx = ScanContext {
            max_depth: 0,
            ..ScanContext::new()
        };
        self.scan_7z_with_context(archive_path, &mut ctx, password)
    }
    pub(super) fn read_file_from_7z(
        &self,
        archive_path: &Path,
        path: &str,
        password: Option<&str>,
    ) -> Result<String> {
        let package = SevenzPackage::open(archive_path, password)?;
        let name = path.replace('\\', "/");
        package
            .read_texts([name.clone()], None)
            .remove(&name)
            .with_context(|| format!("Failed to read 7z text member: {path}"))
    }
}

pub(super) fn classify_7z_password(
    error: anyhow::Error,
    path: &Path,
    password: Option<&str>,
) -> anyhow::Error {
    use sevenz_rust2::Error;
    let requires_password = error
        .downcast_ref::<Error>()
        .is_some_and(|error| matches!(error, Error::PasswordRequired | Error::MaybeBadPassword(_)));
    if requires_password {
        if password.is_none() {
            return PasswordRequiredError {
                archive_path: path.to_string_lossy().into_owned(),
            }
            .into();
        }
        return error.context(format!("Wrong password for archive: {}", path.display()));
    }
    error
}

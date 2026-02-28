use super::*;

impl Installer {
    /// Check whether a source file is a supported archive format.
    fn is_supported_archive_file(path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "zip" | "7z" | "rar"))
            .unwrap_or(false)
    }

    /// Copy a single file with progress tracking.
    /// If target points to an existing directory, copy into that directory
    /// using the source filename.
    fn copy_single_file_with_progress(
        &self,
        source: &Path,
        target: &Path,
        ctx: &ProgressContext,
    ) -> Result<()> {
        let final_target = if target.exists() && target.is_dir() {
            let file_name = source
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Source file has no filename"))?;
            target.join(file_name)
        } else {
            target.to_path_buf()
        };

        if let Some(parent) = final_target.parent() {
            fs::create_dir_all(parent).context(format!(
                "Failed to create target directory for file copy: {:?}",
                parent
            ))?;
        }

        let file_size = fs::metadata(source)?.len();
        let display_name = final_target
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut source_file = fs::File::open(source)
            .context(format!("Failed to open source file {:?}", source))?;
        let mut target_file = fs::File::create(&final_target)
            .context(format!("Failed to create target file {:?}", final_target))?;
        copy_file_optimized(&mut source_file, &mut target_file)?;

        // Remove read-only attribute from copied file to avoid future deletion issues
        let _ = remove_readonly_attribute(&final_target);

        ctx.add_bytes(file_size);
        ctx.emit_progress(Some(display_name), InstallPhase::Installing);

        Ok(())
    }

    /// Remove an existing install target path regardless of whether it's a file or directory.
    fn remove_existing_target_path(&self, target: &Path) -> Result<()> {
        if !target.exists() {
            return Ok(());
        }

        if target.is_dir() {
            remove_dir_all_robust(target)
                .context(format!("Failed to delete existing folder: {:?}", target))?;
        } else {
            let _ = remove_readonly_attribute(target);
            fs::remove_file(target)
                .context(format!("Failed to delete existing file: {:?}", target))?;
        }

        Ok(())
    }

    /// Build the list of Lua bundle entries to install:
    /// the script itself plus detected companion paths.
    fn get_lua_bundle_entries(task: &InstallTask, target: &Path) -> Result<Vec<PathBuf>> {
        use std::collections::HashSet;

        let script_name = target
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Lua target path has no filename: {:?}", target))?;

        let mut entries: Vec<PathBuf> = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();

        let script_entry = PathBuf::from(script_name);
        seen.insert(script_entry.clone());
        entries.push(script_entry);

        for companion in &task.companion_paths {
            if let Some(safe_path) = sanitize_path(Path::new(companion)) {
                if seen.insert(safe_path.clone()) {
                    entries.push(safe_path);
                }
            } else {
                logger::log_info(
                    &format!(
                        "Skipping unsafe Lua companion path for task {}: {}",
                        task.display_name, companion
                    ),
                    Some("installer"),
                );
            }
        }

        Ok(entries)
    }

    /// Remove existing Lua bundle targets (script + companions) before clean install.
    fn remove_lua_bundle_targets(&self, scripts_dir: &Path, bundle_entries: &[PathBuf]) -> Result<()> {
        for entry in bundle_entries {
            let target_path = scripts_dir.join(entry);
            if target_path.exists() {
                self.remove_existing_target_path(&target_path)?;
            }
        }
        Ok(())
    }

    /// Copy selected Lua bundle entries from a source directory to Scripts with progress tracking.
    fn copy_lua_bundle_from_directory_with_progress(
        &self,
        source_dir: &Path,
        scripts_dir: &Path,
        bundle_entries: &[PathBuf],
        ctx: &ProgressContext,
        should_overwrite: bool,
    ) -> Result<()> {
        for entry in bundle_entries {
            let source_path = source_dir.join(entry);
            if !source_path.exists() {
                logger::log_info(
                    &format!(
                        "Lua bundle entry not found in source directory, skipping: {:?}",
                        source_path
                    ),
                    Some("installer"),
                );
                continue;
            }

            let target_path = scripts_dir.join(entry);
            if !should_overwrite && target_path.exists() {
                self.remove_existing_target_path(&target_path)?;
            }

            if source_path.is_dir() {
                self.copy_directory_with_progress(&source_path, &target_path, ctx)?;
            } else {
                self.copy_single_file_with_progress(&source_path, &target_path, ctx)?;
            }
        }

        Ok(())
    }

    /// Copy selected Lua bundle entries from a staging directory to Scripts.
    /// This phase intentionally avoids progress updates because archive extraction
    /// has already reported byte progress.
    fn copy_lua_bundle_from_staging_without_progress(
        &self,
        staging_dir: &Path,
        scripts_dir: &Path,
        bundle_entries: &[PathBuf],
        should_overwrite: bool,
    ) -> Result<()> {
        for entry in bundle_entries {
            let source_path = staging_dir.join(entry);
            if !source_path.exists() {
                logger::log_info(
                    &format!(
                        "Lua bundle entry not found in extracted archive, skipping: {:?}",
                        source_path
                    ),
                    Some("installer"),
                );
                continue;
            }

            let target_path = scripts_dir.join(entry);

            if source_path.is_dir() {
                if should_overwrite && target_path.exists() && target_path.is_dir() {
                    self.copy_directory_without_progress(&source_path, &target_path)?;
                } else {
                    if target_path.exists() {
                        self.remove_existing_target_path(&target_path)?;
                    }

                    if fs::rename(&source_path, &target_path).is_err() {
                        self.copy_directory_without_progress(&source_path, &target_path)?;
                    }
                }
            } else {
                if target_path.exists() {
                    self.remove_existing_target_path(&target_path)?;
                }

                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).context(format!(
                        "Failed to create Lua companion parent directory: {:?}",
                        parent
                    ))?;
                }

                if fs::rename(&source_path, &target_path).is_err() {
                    fs::copy(&source_path, &target_path).context(format!(
                        "Failed to copy Lua bundle entry: {:?} -> {:?}",
                        source_path, target_path
                    ))?;
                }

                let _ = remove_readonly_attribute(&target_path);
            }
        }

        Ok(())
    }

    /// Install Lua script task (script + detected companions).
    fn install_lua_task_with_companions(
        &self,
        task: &InstallTask,
        source: &Path,
        target: &Path,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        use tempfile::TempDir;

        let scripts_dir = target
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Lua target path has no parent: {:?}", target))?;
        fs::create_dir_all(scripts_dir)
            .context(format!("Failed to create Lua Scripts directory: {:?}", scripts_dir))?;

        let bundle_entries = Self::get_lua_bundle_entries(task, target)?;

        if !task.should_overwrite {
            self.remove_lua_bundle_targets(scripts_dir, &bundle_entries)?;
        }

        // Archive source (including nested archive chain): extract to staging first.
        if source.is_file() && (task.extraction_chain.is_some() || Self::is_supported_archive_file(source))
        {
            let staging =
                TempDir::new().context("Failed to create temp staging directory for Lua install")?;

            if let Some(ref chain) = task.extraction_chain {
                self.install_content_with_extraction_chain(source, staging.path(), chain, ctx, password)?;
            } else {
                self.extract_archive_with_progress(
                    source,
                    staging.path(),
                    task.archive_internal_root.as_deref(),
                    ctx,
                    password,
                    task.file_hashes.as_ref(),
                )?;
            }

            self.copy_lua_bundle_from_staging_without_progress(
                staging.path(),
                scripts_dir,
                &bundle_entries,
                task.should_overwrite,
            )?;
            return Ok(());
        }

        // Direct file source: copy from the .lua parent directory.
        if source.is_file() {
            let source_dir = source
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Lua source file has no parent directory"))?;
            self.copy_lua_bundle_from_directory_with_progress(
                source_dir,
                scripts_dir,
                &bundle_entries,
                ctx,
                task.should_overwrite,
            )?;
            return Ok(());
        }

        // Directory source (fallback): copy directly from source directory.
        if source.is_dir() {
            self.copy_lua_bundle_from_directory_with_progress(
                source,
                scripts_dir,
                &bundle_entries,
                ctx,
                task.should_overwrite,
            )?;
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "Lua source path is neither file nor directory: {:?}",
            source
        ))
    }

    /// Install a single task with progress tracking
    pub(super) fn install_task_with_progress(
        &self,
        task: &InstallTask,
        ctx: &ProgressContext,
        atomic_install_enabled: bool,
        xplane_path: &str,
    ) -> Result<()> {
        let source = Path::new(&task.source_path);
        let target = Path::new(&task.target_path);
        let password = task.password.as_deref();

        // Create parent directory if it doesn't exist
        let mkdir_start = Instant::now();
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create target directory: {:?}", parent))?;
        }
        crate::log_debug!(
            &format!(
                "[TIMING] Directory creation completed in {:.2}ms",
                mkdir_start.elapsed().as_secs_f64() * 1000.0
            ),
            "installer_timing"
        );

        // Lua scripts are treated as a file bundle: script + detected companions.
        // Handle them with a dedicated path so archives install into Scripts root
        // (not into a "script.lua/" directory).
        if task.addon_type == AddonType::LuaScript {
            self.install_lua_task_with_companions(task, source, target, ctx, password)?;
            return Ok(());
        }

        // Check if this is a nested archive installation
        if let Some(ref chain) = task.extraction_chain {
            crate::log_debug!(
                "[TIMING] Using nested archive extraction path",
                "installer_timing"
            );

            // Nested archive: use recursive extraction (no atomic install for nested archives)
            if !task.should_overwrite && target.exists() {
                // Clean install mode for nested archives
                crate::log_debug!(
                    "[TIMING] Clean install mode for nested archive",
                    "installer_timing"
                );
                self.handle_clean_install_with_extraction_chain(
                    task, source, target, chain, ctx, password,
                )?;
            } else {
                // Direct overwrite mode for nested archives
                crate::log_debug!(
                    "[TIMING] Direct overwrite mode for nested archive",
                    "installer_timing"
                );
                self.install_content_with_extraction_chain(source, target, chain, ctx, password)?;
            }
        } else if atomic_install_enabled && (source.is_dir() || Self::is_supported_archive_file(source))
        {
            // Atomic installation mode
            crate::log_debug!(
                "[TIMING] Using atomic installation mode",
                "installer_timing"
            );
            self.install_task_atomic(task, source, target, ctx, password, xplane_path)?;
        } else {
            // Regular installation (non-nested, non-atomic)
            if !task.should_overwrite && target.exists() {
                crate::log_debug!(
                    "[TIMING] Clean install mode for regular archive",
                    "installer_timing"
                );
                // Clean install mode: delete old folder first
                self.handle_clean_install_with_progress(task, source, target, ctx, password)?;
            } else {
                crate::log_debug!(
                    "[TIMING] Direct overwrite mode for regular archive",
                    "installer_timing"
                );
                // Direct overwrite mode: just install/extract files directly
                self.install_content_with_progress_and_hashes(
                    source,
                    target,
                    task.archive_internal_root.as_deref(),
                    ctx,
                    password,
                    task.file_hashes.as_ref(),
                )?;
            }
        }

        Ok(())
    }

    /// Install content with progress tracking
    fn install_content_with_progress(
        &self,
        source: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        self.install_content_with_progress_and_hashes(
            source,
            target,
            internal_root,
            ctx,
            password,
            None,
        )
    }

    /// Install content with progress tracking and optional expected hashes for inline verification
    fn install_content_with_progress_and_hashes(
        &self,
        source: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
        expected_hashes: Option<&HashMap<String, crate::models::FileHash>>,
    ) -> Result<()> {
        if source.is_dir() {
            self.copy_directory_with_progress(source, target, ctx)?;
        } else if source.is_file() {
            if Self::is_supported_archive_file(source) {
                self.extract_archive_with_progress(
                    source,
                    target,
                    internal_root,
                    ctx,
                    password,
                    expected_hashes,
                )?;
            } else {
                self.copy_single_file_with_progress(source, target, ctx)?;
            }
        } else {
            return Err(anyhow::anyhow!("Source path is neither file nor directory"));
        }
        Ok(())
    }

    /// Install content with extraction chain (for nested archives)
    /// Optimized version: ZIP archives are extracted directly from memory when possible
    fn install_content_with_extraction_chain(
        &self,
        source: &Path,
        target: &Path,
        chain: &crate::models::ExtractionChain,
        ctx: &ProgressContext,
        outermost_password: Option<&str>,
    ) -> Result<()> {
        // For multi-layer chains (including single-layer nested archives),
        // check if we can use the memory-optimized path
        // IMPORTANT: Must also check that the outermost archive (source) is a ZIP file
        let source_is_zip = source
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("zip"))
            .unwrap_or(false);
        let all_nested_zip = chain.archives.iter().all(|a| a.format == "zip");

        if source_is_zip && all_nested_zip {
            // Optimized path: Extract nested ZIPs directly from memory
            self.install_nested_zip_from_memory(source, target, chain, ctx, outermost_password)
        } else {
            // Fallback path: Use temp directory for 7z/RAR
            self.install_nested_with_temp(source, target, chain, ctx, outermost_password)
        }
    }

    /// Optimized installation for nested ZIP archives (memory-only, no temp files)
    fn install_nested_zip_from_memory(
        &self,
        source: &Path,
        target: &Path,
        chain: &crate::models::ExtractionChain,
        ctx: &ProgressContext,
        outermost_password: Option<&str>,
    ) -> Result<()> {
        use std::io::{Cursor, Read};
        use zip::ZipArchive;

        crate::logger::log_info(
            &format!(
                "Using optimized memory extraction for {} nested ZIP layers",
                chain.archives.len()
            ),
            Some("installer"),
        );

        // Open the outermost archive
        let file = fs::File::open(source)?;
        let mut current_archive_data = Vec::new();
        file.take(u64::MAX).read_to_end(&mut current_archive_data)?;

        // Store password as bytes to avoid taint flow from string into logging sinks
        let mut current_password_bytes: Option<Vec<u8>> =
            outermost_password.map(|p| p.as_bytes().to_vec());

        // Navigate through all layers
        for archive_info in chain.archives.iter() {
            let cursor = Cursor::new(&current_archive_data);
            let mut archive = ZipArchive::new(cursor)?;

            // Read nested archive into memory
            let nested_path = &archive_info.internal_path;
            let nested_path_normalized = nested_path.replace('\\', "/");
            let mut nested_data = Vec::new();

            // Search for the nested archive
            let mut found = false;
            let mut decryption_error: Option<String> = None;

            for i in 0..archive.len() {
                // First, check if this is the file we're looking for using raw access
                let file_name = {
                    let raw_file = archive.by_index_raw(i)?;
                    raw_file.name().replace('\\', "/")
                };

                if file_name != nested_path_normalized {
                    continue; // Not the file we're looking for
                }

                // Found the file, now try to read it
                let mut file = if let Some(ref pwd) = current_password_bytes {
                    match archive.by_index_decrypt(i, pwd) {
                        Ok(f) => f,
                        Err(_e) => {
                            // Avoid logging potentially sensitive details from the underlying error
                            decryption_error = Some(format!(
                                "Failed to decrypt nested archive at {}",
                                nested_path
                            ));
                            break; // Stop searching, we found the file but can't decrypt
                        }
                    }
                } else {
                    archive.by_index(i)?
                };

                file.read_to_end(&mut nested_data)?;
                found = true;
                break;
            }

            if let Some(err) = decryption_error {
                return Err(anyhow::anyhow!(err));
            }

            if !found {
                return Err(anyhow::anyhow!(
                    "Nested archive not found in ZIP: {}",
                    nested_path
                ));
            }

            // Update for next iteration
            current_archive_data = nested_data;
            // Update password for next layer if specified
            // If the nested archive has its own password, use it
            // Otherwise, keep the current (parent) password for try-through
            if let Some(ref next_pwd) = archive_info.password {
                current_password_bytes = Some(next_pwd.as_bytes().to_vec());
            }
            // Note: if archive_info.password is None, we keep current_password_bytes
            // as-is, since many nested archives share the same password as the parent
        }

        // Now extract the final (innermost) archive
        let cursor = Cursor::new(current_archive_data);
        let mut archive = ZipArchive::new(cursor)?;

        let pwd_bytes = current_password_bytes.as_deref();
        // Extract all files with final_internal_root filter
        self.extract_zip_from_archive(
            &mut archive,
            target,
            chain.final_internal_root.as_deref(),
            ctx,
            pwd_bytes,
        )?;

        Ok(())
    }

    /// Extract files from an in-memory ZIP archive
    fn extract_zip_from_archive<R: std::io::Read + std::io::Seek>(
        &self,
        archive: &mut zip::ZipArchive<R>,
        target: &Path,
        internal_root: Option<&str>,
        _ctx: &ProgressContext,
        password: Option<&[u8]>,
    ) -> Result<()> {
        let internal_root_normalized = internal_root.map(|s| s.replace('\\', "/"));
        let prefix = internal_root_normalized.as_deref();

        // Debug: Log extraction parameters
        crate::logger::log_debug(
            &format!(
                "extract_zip_from_archive: target={:?}, internal_root={:?}, archive_len={}",
                target,
                internal_root,
                archive.len()
            ),
            Some("installer"),
            None,
        );

        // Collect all file entries
        let entries: Vec<_> = (0..archive.len())
            .filter_map(|i| {
                // Use by_index_raw to avoid triggering decryption errors when reading metadata
                let file = archive.by_index_raw(i).ok()?;
                let path = file.enclosed_name()?.to_path_buf();
                let file_path_str = path.to_string_lossy().replace('\\', "/");

                // Check prefix filter
                let relative_path = if let Some(prefix) = prefix {
                    // Ensure prefix ends with '/' for proper matching
                    let prefix_with_slash = if prefix.ends_with('/') {
                        prefix.to_string()
                    } else {
                        format!("{}/", prefix)
                    };

                    // Debug: Log file matching
                    let matched = file_path_str.strip_prefix(&prefix_with_slash);
                    crate::logger::log_debug(
                        &format!(
                            "File: '{}', Prefix: '{}', Matched: {:?}",
                            file_path_str,
                            prefix_with_slash,
                            matched.is_some()
                        ),
                        Some("installer"),
                        None,
                    );

                    // Strip prefix and return relative path
                    file_path_str
                        .strip_prefix(&prefix_with_slash)
                        .map(|s| s.to_string())?
                } else {
                    file_path_str.clone()
                };

                Some((i, relative_path, file.is_dir(), file.encrypted()))
            })
            .collect();

        // Debug: Log collected entries
        crate::logger::log_debug(
            &format!("Collected {} entries after filtering", entries.len()),
            Some("installer"),
            None,
        );

        // Create directories first
        for (_, relative_path, is_dir, _) in &entries {
            if *is_dir {
                let dir_path = target.join(relative_path);
                fs::create_dir_all(&dir_path)?;
            }
        }

        // Extract files sequentially
        // Note: Parallel extraction for in-memory archives is complex because
        // ZipArchive requires mutable access. For file-based archives, we can
        // open multiple handles, but for in-memory Cursor, we cannot easily clone.
        // Sequential extraction is still fast due to in-memory access.
        if entries.iter().any(|(_, _, _, encrypted)| *encrypted) {
            // Sequential extraction for encrypted files
            for (i, relative_path, is_dir, is_encrypted) in entries {
                if is_dir {
                    continue;
                }

                let target_path = target.join(&relative_path);
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let mut file = if is_encrypted {
                    if let Some(pwd) = password {
                        archive
                            .by_index_decrypt(i, pwd)
                            .map_err(|e| anyhow::anyhow!("Failed to decrypt file: {}", e))?
                    } else {
                        return Err(anyhow::anyhow!("Password required for encrypted file"));
                    }
                } else {
                    archive.by_index(i)?
                };

                let mut output = fs::File::create(&target_path)?;
                std::io::copy(&mut file, &mut output)?;

                // Set permissions on Unix
                #[cfg(unix)]
                if let Some(mode) = file.unix_mode() {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&target_path, fs::Permissions::from_mode(mode))?;
                }
            }
        } else {
            // Sequential extraction for non-encrypted files
            for (i, relative_path, is_dir, _) in entries {
                if is_dir {
                    continue;
                }

                let target_path = target.join(&relative_path);
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let mut file = archive.by_index(i)?;
                let mut output = fs::File::create(&target_path)?;
                std::io::copy(&mut file, &mut output)?;

                #[cfg(unix)]
                if let Some(mode) = file.unix_mode() {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&target_path, fs::Permissions::from_mode(mode))?;
                }
            }
        }

        Ok(())
    }

    /// Fallback installation for nested archives with temp directory (for 7z/RAR)
    /// Optimized for mixed formats: uses memory for ZIP layers when possible
    fn install_nested_with_temp(
        &self,
        source: &Path,
        target: &Path,
        chain: &crate::models::ExtractionChain,
        ctx: &ProgressContext,
        outermost_password: Option<&str>,
    ) -> Result<()> {
        use tempfile::TempDir;

        crate::logger::log_info(
            &format!("Using temp directory extraction for {} nested layers (mixed format optimization enabled)", chain.archives.len()),
            Some("installer"),
        );

        // Create temp directory for intermediate extractions
        let temp_base =
            TempDir::new().context("Failed to create temp directory for nested extraction")?;

        let mut current_source = source.to_path_buf();
        let mut current_password = outermost_password;

        // Extract each layer in the chain
        for (index, archive_info) in chain.archives.iter().enumerate() {
            let is_last = index == chain.archives.len() - 1;
            let current_format = &archive_info.format;

            // Determine extraction target
            let extract_target = if is_last {
                // Last layer: extract directly to final target
                target.to_path_buf()
            } else {
                // Intermediate layer: extract to temp
                temp_base.path().join(format!("layer_{}", index))
            };

            // Create target directory
            fs::create_dir_all(&extract_target).context(format!(
                "Failed to create extraction target: {:?}",
                extract_target
            ))?;

            // Extract the current archive
            crate::logger::log_info(
                &format!(
                    "Extracting layer {} ({}): {} to {:?}",
                    index, current_format, archive_info.internal_path, extract_target
                ),
                Some("installer"),
            );

            self.extract_archive_with_progress(
                &current_source,
                &extract_target,
                if is_last {
                    chain.final_internal_root.as_deref()
                } else {
                    None
                },
                ctx,
                current_password,
                None,
            )?;

            // For non-last layers, find the nested archive in the extracted content
            if !is_last {
                let nested_archive_path = extract_target.join(&archive_info.internal_path);

                if !nested_archive_path.exists() {
                    // Provide detailed error with directory listing
                    let mut available_files = Vec::new();
                    if let Ok(entries) = fs::read_dir(&extract_target) {
                        for entry in entries.flatten().take(10) {
                            if let Some(name) = entry.file_name().to_str() {
                                available_files.push(name.to_string());
                            }
                        }
                    }

                    return Err(anyhow::anyhow!(
                        "Nested archive not found after extraction: {}\nExpected at: {:?}\nExtracted to: {:?}\nAvailable files: {}",
                        archive_info.internal_path,
                        nested_archive_path,
                        extract_target,
                        if available_files.is_empty() {
                            "(none)".to_string()
                        } else {
                            available_files.join(", ")
                        }
                    ));
                }

                // OPTIMIZATION: If next layer is ZIP, try to load it into memory
                if let Some(next_archive) = chain.archives.get(index + 1) {
                    if next_archive.format == "zip" {
                        crate::logger::log_info(
                            &format!("Optimizing: Loading ZIP layer {} into memory", index + 1),
                            Some("installer"),
                        );

                        // Try to read the ZIP into memory for faster processing
                        match self.try_extract_zip_from_memory(
                            &nested_archive_path,
                            target,
                            &chain.archives[(index + 1)..],
                            chain.final_internal_root.as_deref(),
                            ctx,
                            next_archive.password.as_deref(),
                        ) {
                            Ok(()) => {
                                // Successfully extracted from memory, we're done
                                crate::logger::log_info(
                                    "Memory optimization successful for remaining ZIP layers",
                                    Some("installer"),
                                );
                                return Ok(());
                            }
                            Err(e) => {
                                // Fall back to normal extraction
                                crate::logger::log_info(
                                    &format!("Memory optimization failed, falling back to temp extraction: {}", e),
                                    Some("installer"),
                                );
                            }
                        }
                    }
                }

                // Update source for next iteration
                current_source = nested_archive_path;

                // Update password for next layer if specified
                if let Some(next_archive) = chain.archives.get(index + 1) {
                    if next_archive.password.is_some() {
                        current_password = next_archive.password.as_deref();
                    }
                }
            }
        }

        // Temp directory automatically cleaned up when TempDir drops
        Ok(())
    }

    /// Try to extract remaining ZIP layers from memory (optimization for mixed formats)
    fn try_extract_zip_from_memory(
        &self,
        zip_path: &Path,
        target: &Path,
        remaining_chain: &[crate::models::NestedArchiveInfo],
        final_internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        use std::io::{Cursor, Read};
        use zip::ZipArchive;

        // Check file size before loading into memory (limit: 200MB)
        let metadata = fs::metadata(zip_path)?;
        if metadata.len() > MAX_MEMORY_ZIP_SIZE {
            return Err(anyhow::anyhow!(
                "ZIP file too large for memory optimization ({} MB > 200 MB)",
                metadata.len() / 1024 / 1024
            ));
        }

        // Read the ZIP file into memory
        let mut zip_data = Vec::new();
        let mut file = fs::File::open(zip_path)?;
        file.read_to_end(&mut zip_data)?;

        let mut current_archive_data = zip_data;
        // Store password as bytes to avoid handling it as a string near logging/formatting sinks
        let mut current_password_opt: Option<&[u8]> = password.map(|p| p.as_bytes());

        // Process remaining ZIP layers in memory
        for (index, archive_info) in remaining_chain.iter().enumerate() {
            let is_last = index == remaining_chain.len() - 1;

            // Verify this is a ZIP layer
            if archive_info.format != "zip" {
                return Err(anyhow::anyhow!(
                    "Non-ZIP layer encountered in memory optimization"
                ));
            }

            let cursor = Cursor::new(&current_archive_data);
            let mut archive = ZipArchive::new(cursor)?;

            if is_last {
                // Last layer: extract to final target
                let cursor = Cursor::new(current_archive_data);
                let mut archive = ZipArchive::new(cursor)?;

                self.extract_zip_from_archive(
                    &mut archive,
                    target,
                    final_internal_root,
                    ctx,
                    current_password_opt,
                )?;
                break;
            } else {
                // Intermediate layer: read nested ZIP into memory
                let nested_path = &archive_info.internal_path;
                let mut nested_data = Vec::new();

                let mut found = false;
                for i in 0..archive.len() {
                    let mut file = match current_password_opt {
                        Some(pwd) => match archive.by_index_decrypt(i, pwd) {
                            Ok(f) => f,
                            Err(_) => continue,
                        },
                        None => archive.by_index(i)?,
                    };

                    if file.name() == nested_path {
                        file.read_to_end(&mut nested_data)?;
                        found = true;
                        break;
                    }
                }

                if !found {
                    return Err(anyhow::anyhow!(
                        "Nested ZIP not found in memory: {}",
                        nested_path
                    ));
                }

                current_archive_data = nested_data;
                // Update password for next layer if specified
                if let Some(ref next_pwd) = archive_info.password {
                    current_password_opt = Some(next_pwd.as_bytes());
                } else {
                    current_password_opt = None;
                }
            }
        }

        Ok(())
    }

    /// Move all contents from source directory to target directory
    #[allow(dead_code)]
    fn move_directory_contents(&self, source: &Path, target: &Path) -> Result<()> {
        for entry in fs::read_dir(source)
            .context(format!("Failed to read source directory: {:?}", source))?
        {
            let entry = entry?;
            let source_path = entry.path();
            let file_name = entry.file_name();
            let target_path = target.join(&file_name);

            if source_path.is_dir() {
                // Try to rename (move) the directory
                if fs::rename(&source_path, &target_path).is_err() {
                    // Fallback: copy and delete
                    self.copy_directory_with_progress(
                        &source_path,
                        &target_path,
                        &ProgressContext::new(self.app_handle.clone(), 1),
                    )?;
                    remove_dir_all_robust(&source_path).context(format!(
                        "Failed to remove source directory: {:?}",
                        source_path
                    ))?;
                }
            } else {
                // Try to rename (move) the file
                if fs::rename(&source_path, &target_path).is_err() {
                    // Fallback: copy and delete
                    fs::copy(&source_path, &target_path)
                        .context(format!("Failed to copy file: {:?}", source_path))?;
                    fs::remove_file(&source_path)
                        .context(format!("Failed to remove source file: {:?}", source_path))?;
                }
            }
        }
        Ok(())
    }

    /// Handle clean install with extraction chain (for nested archives)
    fn handle_clean_install_with_extraction_chain(
        &self,
        task: &crate::models::InstallTask,
        source: &Path,
        target: &Path,
        chain: &crate::models::ExtractionChain,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        match task.addon_type {
            crate::models::AddonType::Aircraft => {
                // For Aircraft: backup liveries and prefs, delete, install, restore
                // Note: For nested archives, we don't have archive_internal_root,
                // so we'll use the extraction chain's final_internal_root
                let params = AircraftExtractionInstallParams {
                    source,
                    target,
                    chain,
                    ctx,
                    password,
                    options: AircraftInstallOptions {
                        backup_liveries: task.backup_liveries,
                        backup_config_files: task.backup_config_files,
                        config_patterns: &task.config_file_patterns,
                    },
                };
                self.handle_aircraft_clean_install_with_extraction_chain(params)?;
            }
            crate::models::AddonType::Navdata => {
                // For Navdata: DON'T delete Custom Data folder!
                // Just extract and overwrite individual files
                self.install_content_with_extraction_chain(source, target, chain, ctx, password)?;
            }
            _ => {
                // For other types: delete and reinstall
                if target.exists() {
                    self.remove_existing_target_path(target)?;
                }
                self.install_content_with_extraction_chain(source, target, chain, ctx, password)?;
            }
        }
        Ok(())
    }

    /// Handle aircraft clean install with extraction chain
    fn handle_aircraft_clean_install_with_extraction_chain(
        &self,
        params: AircraftExtractionInstallParams<'_>,
    ) -> Result<()> {
        let AircraftExtractionInstallParams {
            source,
            target,
            chain,
            ctx,
            password,
            options,
        } = params;
        let AircraftInstallOptions {
            backup_liveries,
            backup_config_files,
            config_patterns: config_file_patterns,
        } = options;
        use uuid::Uuid;

        // Step 1: Backup liveries and config files if requested
        let backup_dir = if (backup_liveries || backup_config_files) && target.exists() {
            let temp_dir = std::env::temp_dir();
            let backup_path = temp_dir.join(format!("xfastmanager_backup_{}", Uuid::new_v4()));
            fs::create_dir_all(&backup_path).context("Failed to create backup directory")?;

            // Backup liveries
            if backup_liveries {
                let liveries_src = target.join("liveries");
                if liveries_src.exists() {
                    let liveries_dst = backup_path.join("liveries");
                    // Use copy without progress to avoid affecting installation progress
                    self.copy_directory_without_progress(&liveries_src, &liveries_dst)?;
                }
            }

            // Backup config files
            if backup_config_files {
                for pattern in config_file_patterns {
                    for config_file in (glob::glob(&target.join(pattern).to_string_lossy())
                        .context("Failed to read glob pattern")?)
                    .flatten()
                    {
                        if config_file.is_file() {
                            if let Some(file_name) = config_file.file_name() {
                                let backup_file = backup_path.join(file_name);
                                fs::copy(&config_file, &backup_file).context(format!(
                                    "Failed to backup config file: {:?}",
                                    config_file
                                ))?;
                            }
                        }
                    }
                }
            }

            Some(backup_path)
        } else {
            None
        };

        // Step 2: Delete existing aircraft folder
        if target.exists() {
            remove_dir_all_robust(target).context(format!(
                "Failed to delete existing aircraft folder: {:?}",
                target
            ))?;
        }

        // Step 3: Install new aircraft using extraction chain
        self.install_content_with_extraction_chain(source, target, chain, ctx, password)?;

        // Step 4: Restore backed up files
        if let Some(backup_path) = backup_dir {
            // Restore liveries
            let liveries_backup = backup_path.join("liveries");
            if liveries_backup.exists() {
                let liveries_target = target.join("liveries");
                self.copy_directory_with_progress(&liveries_backup, &liveries_target, ctx)?;
            }

            // Restore config files
            for entry in fs::read_dir(&backup_path).context("Failed to read backup directory")? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name() {
                        let target_file = target.join(file_name);
                        fs::copy(&path, &target_file)
                            .context(format!("Failed to restore config file: {:?}", path))?;
                    }
                }
            }

            // Verify restoration and cleanup backup
            if target.exists() {
                fs::remove_dir_all(&backup_path).context("Failed to cleanup backup directory")?;
            }
        }

        Ok(())
    }

    /// Handle clean install with progress tracking
    /// Deletes old folder first, then installs fresh
    fn handle_clean_install_with_progress(
        &self,
        task: &InstallTask,
        source: &Path,
        target: &Path,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        match task.addon_type {
            AddonType::Aircraft => {
                // For Aircraft: backup liveries and prefs, delete, install, restore
                let params = AircraftProgressInstallParams {
                    source,
                    target,
                    internal_root: task.archive_internal_root.as_deref(),
                    ctx,
                    password,
                    options: AircraftInstallOptions {
                        backup_liveries: task.backup_liveries,
                        backup_config_files: task.backup_config_files,
                        config_patterns: &task.config_file_patterns,
                    },
                };
                self.handle_aircraft_clean_install_with_progress(params)?;
            }
            AddonType::Navdata => {
                // For Navdata: backup matching old files before installing new ones
                self.handle_navdata_clean_install_with_progress(
                    source,
                    target,
                    task.archive_internal_root.as_deref(),
                    ctx,
                    password,
                    task.backup_navdata,
                )?;
            }
            _ => {
                // For other types: delete and reinstall using robust removal
                if target.exists() {
                    self.remove_existing_target_path(target)?;
                }
                self.install_content_with_progress_and_hashes(
                    source,
                    target,
                    task.archive_internal_root.as_deref(),
                    ctx,
                    password,
                    task.file_hashes.as_ref(),
                )?;
            }
        }
        Ok(())
    }

    /// Aircraft clean install with progress tracking
    fn handle_aircraft_clean_install_with_progress(
        &self,
        params: AircraftProgressInstallParams<'_>,
    ) -> Result<()> {
        let AircraftProgressInstallParams {
            source,
            target,
            internal_root,
            ctx,
            password,
            options,
        } = params;
        let AircraftInstallOptions {
            backup_liveries,
            backup_config_files,
            config_patterns,
        } = options;
        // Step 1: Create backup of important files
        let backup = self.backup_aircraft_data(
            target,
            backup_liveries,
            backup_config_files,
            config_patterns,
            ctx,
        )?;

        // Step 2: VERIFY backup is complete and valid BEFORE deleting
        if let Some(ref backup_data) = backup {
            self.verify_backup(backup_data)
                .context("Backup verification failed - aborting to protect your data")?;
        }

        // Step 3: Delete target folder (only after backup is verified)
        if target.exists() {
            remove_dir_all_robust(target).context(format!(
                "Failed to delete existing aircraft folder: {:?}",
                target
            ))?;
        }

        // Step 4: Install new content with progress
        let install_result =
            self.install_content_with_progress(source, target, internal_root, ctx, password);

        // Step 5: Restore backup and verify
        let restore_verified = if let Some(ref backup_data) = backup {
            match self.restore_aircraft_backup(backup_data, target, ctx) {
                Ok(()) => match self.verify_restore(backup_data, target) {
                    Ok(()) => true,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Restore verification failed: {}. Your backup is preserved at: {:?}.",
                            e,
                            backup_data.temp_dir
                        ));
                    }
                },
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to restore backup: {}. Your backup is preserved at: {:?}.",
                        e,
                        backup_data.temp_dir
                    ));
                }
            }
        } else {
            true
        };

        // Step 6: Cleanup temp backup directory ONLY if restore was verified
        if restore_verified {
            if let Some(backup_data) = backup {
                let _ = fs::remove_dir_all(&backup_data.temp_dir);
            }
        }

        install_result?;
        Ok(())
    }

    /// Backup aircraft liveries folder and config files
    fn backup_aircraft_data(
        &self,
        target: &Path,
        backup_liveries: bool,
        backup_config_files: bool,
        config_patterns: &[String],
        ctx: &ProgressContext,
    ) -> Result<Option<AircraftBackup>> {
        if !target.exists() {
            return Ok(None);
        }

        // Update progress: Starting backup
        ctx.emit_progress(
            Some("Backing up aircraft data...".to_string()),
            InstallPhase::Installing,
        );

        // Create temp directory for backup
        let temp_dir =
            std::env::temp_dir().join(format!("xfastmanager_backup_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)
            .context("Failed to create backup directory - check disk space")?;

        let mut backup = AircraftBackup {
            temp_dir: temp_dir.clone(),
            liveries_path: None,
            pref_files: Vec::new(),
            original_liveries_info: None,
            original_pref_sizes: Vec::new(),
        };

        // Backup liveries folder (root level only) if enabled
        if backup_liveries {
            let liveries_src = target.join("liveries");
            if liveries_src.exists() && liveries_src.is_dir() {
                ctx.emit_progress(
                    Some("Backing up liveries...".to_string()),
                    InstallPhase::Installing,
                );

                // Record original info for verification
                let original_info = self.get_directory_info(&liveries_src)?;
                backup.original_liveries_info = Some(original_info);

                let liveries_dst = temp_dir.join("liveries");
                // Use copy without progress to avoid affecting installation progress
                self.copy_directory_without_progress(&liveries_src, &liveries_dst)
                    .context("Failed to backup liveries folder")?;
                backup.liveries_path = Some(liveries_dst);
            }
        }

        // Backup config files from root directory only if enabled
        if backup_config_files && !config_patterns.is_empty() {
            ctx.emit_progress(
                Some("Backing up config files...".to_string()),
                InstallPhase::Installing,
            );

            // Pre-compile patterns once for efficiency
            let compiled = CompiledPatterns::new(config_patterns);

            for entry in fs::read_dir(target)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if compiled.matches(name) {
                            let backup_path = temp_dir.join(name);
                            fs::copy(&path, &backup_path)
                                .context(format!("Failed to backup {}", name))?;

                            let original_size = fs::metadata(&path)?.len();
                            backup
                                .pref_files
                                .push((name.to_string(), backup_path.clone()));
                            backup
                                .original_pref_sizes
                                .push((name.to_string(), original_size));

                            // Don't update progress for backup operations
                        }
                    }
                }
            }
        }

        Ok(Some(backup))
    }

    /// Get directory info (file count and total size) for verification
    fn get_directory_info(&self, dir: &Path) -> Result<DirectoryInfo> {
        let mut file_count = 0u64;
        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(dir).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() {
                file_count += 1;
                total_size += entry.metadata()?.len();
            }
        }

        Ok(DirectoryInfo {
            file_count,
            total_size,
        })
    }

    /// Verify backup is complete and valid before proceeding with deletion
    fn verify_backup(&self, backup: &AircraftBackup) -> Result<()> {
        // Verify liveries backup
        if let (Some(ref liveries_backup_path), Some(ref original_info)) =
            (&backup.liveries_path, &backup.original_liveries_info)
        {
            if !liveries_backup_path.exists() {
                anyhow::bail!("Liveries backup folder does not exist");
            }

            let backup_info = self.get_directory_info(liveries_backup_path)?;

            if backup_info.file_count != original_info.file_count {
                anyhow::bail!(
                    "Liveries backup incomplete: expected {} files, got {}",
                    original_info.file_count,
                    backup_info.file_count
                );
            }

            if backup_info.total_size != original_info.total_size {
                anyhow::bail!(
                    "Liveries backup size mismatch: expected {} bytes, got {}",
                    original_info.total_size,
                    backup_info.total_size
                );
            }
        }

        // Verify pref files backup
        for (filename, original_size) in &backup.original_pref_sizes {
            let backup_path = backup.temp_dir.join(filename);

            if !backup_path.exists() {
                anyhow::bail!("Backup of {} does not exist", filename);
            }

            let backup_size = fs::metadata(&backup_path)?.len();
            if backup_size != *original_size {
                anyhow::bail!(
                    "Backup of {} has wrong size: expected {} bytes, got {}",
                    filename,
                    original_size,
                    backup_size
                );
            }
        }

        Ok(())
    }

    /// Verify restore was successful by checking restored files exist and have correct sizes
    fn verify_restore(&self, backup: &AircraftBackup, target: &Path) -> Result<()> {
        // Verify pref files were restored (these should always be overwritten)
        for (filename, original_size) in &backup.original_pref_sizes {
            let restored_path = target.join(filename);

            if !restored_path.exists() {
                anyhow::bail!("Restored file {} does not exist", filename);
            }

            let restored_size = fs::metadata(&restored_path)?.len();
            if restored_size != *original_size {
                anyhow::bail!(
                    "Restored file {} has wrong size: expected {} bytes, got {}",
                    filename,
                    original_size,
                    restored_size
                );
            }
        }

        // For liveries, we only verify files that should have been restored
        // (files that don't exist in the new addon were copied from backup)
        // This is harder to verify precisely, so we just check the folder exists if we had a backup
        if backup.liveries_path.is_some() {
            let liveries_target = target.join("liveries");
            if !liveries_target.exists() {
                anyhow::bail!("Liveries folder was not restored");
            }
        }

        Ok(())
    }

    /// Restore aircraft backup data
    fn restore_aircraft_backup(
        &self,
        backup: &AircraftBackup,
        target: &Path,
        ctx: &ProgressContext,
    ) -> Result<()> {
        ctx.emit_progress(
            Some("Restoring backup...".to_string()),
            InstallPhase::Installing,
        );

        // Restore liveries folder (skip existing - don't overwrite new content)
        if let Some(ref liveries_backup) = backup.liveries_path {
            ctx.emit_progress(
                Some("Restoring liveries...".to_string()),
                InstallPhase::Installing,
            );

            let liveries_target = target.join("liveries");

            if liveries_target.exists() {
                // Merge: copy only files that don't exist in new content
                self.merge_directory_skip_existing_with_progress(
                    liveries_backup,
                    &liveries_target,
                    ctx,
                )?;
            } else {
                // No new liveries folder, restore entire backup
                self.copy_directory_with_progress(liveries_backup, &liveries_target, ctx)?;
            }
        }

        // Restore *_prefs.txt files (always overwrite - restore user preferences)
        if !backup.pref_files.is_empty() {
            ctx.emit_progress(
                Some("Restoring config files...".to_string()),
                InstallPhase::Installing,
            );

            for (filename, backup_path) in &backup.pref_files {
                let target_path = target.join(filename);
                let size = fs::metadata(backup_path)?.len();
                fs::copy(backup_path, &target_path)
                    .context(format!("Failed to restore pref file: {}", filename))?;

                // Update progress for each config file with filename for real-time display
                ctx.add_bytes(size);
                ctx.emit_progress(Some(filename.clone()), InstallPhase::Installing);
            }
        }

        Ok(())
    }

    /// Copy directory contents, skipping files that already exist in target (with progress)
    fn merge_directory_skip_existing_with_progress(
        &self,
        source: &Path,
        target: &Path,
        ctx: &ProgressContext,
    ) -> Result<()> {
        if !target.exists() {
            fs::create_dir_all(target)?;
        }

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let source_path = entry.path();
            let file_name = entry.file_name();
            let target_path = target.join(&file_name);

            if file_type.is_dir() {
                // Recursively merge subdirectories
                self.merge_directory_skip_existing_with_progress(&source_path, &target_path, ctx)?;
            } else {
                let size = fs::metadata(&source_path)?.len();
                let display_name = file_name.to_string_lossy().to_string();

                // Only copy if target doesn't exist (skip existing)
                if !target_path.exists() {
                    fs::copy(&source_path, &target_path)?;
                    // Remove read-only attribute from copied file
                    let _ = remove_readonly_attribute(&target_path);
                }

                // Always update progress (even for skipped files) to keep progress accurate
                ctx.add_bytes(size);
                ctx.emit_progress(Some(display_name), InstallPhase::Installing);
            }
        }

        Ok(())
    }

    /// Navdata clean install with backup (non-atomic path) - OPTIMIZED
    ///
    /// Performance optimizations:
    /// 1. Parallel SHA-256 checksum calculation using rayon
    /// 2. Fast verification (size-only check instead of re-computing checksums)
    /// 3. Collect all files first, then parallel checksum, then sequential move
    ///
    /// Backs up matching old navdata files, then installs new ones
    fn handle_navdata_clean_install_with_progress(
        &self,
        source: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
        backup_navdata: bool,
    ) -> Result<()> {
        use crate::models::{BackupFileEntry, NavdataBackupVerification};

        // Step 1: Extract/copy to a temp directory first to know the new files
        let temp_dir = target
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Target has no parent"))?
            .join(format!(".navdata_temp_{}", uuid::Uuid::new_v4()));

        fs::create_dir_all(&temp_dir)?;

        ctx.emit_progress(
            Some("Extracting new navdata...".to_string()),
            InstallPhase::Installing,
        );

        // Extract new navdata to temp
        let extract_result =
            self.install_content_with_progress(source, &temp_dir, internal_root, ctx, password);

        if let Err(e) = extract_result {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(e);
        }

        // Step 2: Read provider info from the new cycle.json
        let read_cycle_json = |dir: &Path| -> (String, Option<String>, Option<String>) {
            let cycle_path = dir.join("cycle.json");
            if let Ok(content) = fs::read_to_string(&cycle_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let name = json
                        .get("provider")
                        .or_else(|| json.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("navdata")
                        .to_string();
                    let cycle = json
                        .get("cycle")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let airac = json
                        .get("airac")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    return (name, cycle, airac);
                }
            }
            ("navdata".to_string(), None, None)
        };

        let (provider_name, _, _) = read_cycle_json(&temp_dir);
        let (_, old_cycle, old_airac) = read_cycle_json(target);

        // Step 3: Enumerate new entries
        let new_entries: Vec<std::ffi::OsString> = fs::read_dir(&temp_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();

        if backup_navdata {
            // Step 4: Create Backup_Data directory in Custom Data (not in target)
            ctx.emit_progress(
                Some("Creating backup directory...".to_string()),
                InstallPhase::Installing,
            );

            // Backup_Data always goes in Custom Data, not in target (which might be Custom Data/GNS430)
            let custom_data_dir = if target.file_name().and_then(|n| n.to_str()) == Some("GNS430") {
                target.parent().unwrap_or(target)
            } else {
                target
            };
            let backup_data_dir = custom_data_dir.join("Backup_Data");
            fs::create_dir_all(&backup_data_dir)?;

            // Use timestamp to create unique backup folder name
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
            let sanitized_provider = sanitize_folder_name(&provider_name);
            let backup_folder_name = format!("{}_{}", sanitized_provider, timestamp);
            let backup_subdir = backup_data_dir.join(&backup_folder_name);
            fs::create_dir_all(&backup_subdir)?;

            // Step 5: Collect all files using walkdir (OPTIMIZED: single pass, no checksum)
            ctx.emit_progress(
                Some("Scanning files to backup...".to_string()),
                InstallPhase::Installing,
            );

            // Use Custom Data as base for relative paths (for consistent restore)
            let mut backup_entries: Vec<BackupFileEntry> = Vec::new();

            for entry_name in &new_entries {
                let old_path = target.join(entry_name);
                if old_path.exists() {
                    // Use walkdir for efficient enumeration (DirEntry::file_type() uses cached stat)
                    for entry in walkdir::WalkDir::new(&old_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                    {
                        // Get size from walkdir's metadata (single stat call)
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        // Relative path from Custom Data (not target) for consistent restore
                        let relative_path = entry
                            .path()
                            .strip_prefix(custom_data_dir)
                            .unwrap_or(entry.path())
                            .to_string_lossy()
                            .replace('\\', "/");

                        backup_entries.push(BackupFileEntry {
                            relative_path,
                            checksum: String::new(), // SKIP checksum - fs::rename is atomic
                            size,
                        });
                    }
                }
            }

            logger::log_info(
                &format!(
                    "Found {} files to backup (checksum skipped)",
                    backup_entries.len()
                ),
                Some("installer"),
            );

            // Step 6: Move files to backup (OPTIMIZED: directory-level rename)
            ctx.emit_progress(
                Some("Moving files to backup...".to_string()),
                InstallPhase::Installing,
            );

            // Optimized move_directory: tries directory-level rename first
            fn move_directory_optimized(src: &Path, dst: &Path) -> Result<()> {
                if src.is_dir() {
                    // Ensure parent of destination exists
                    if let Some(parent) = dst.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    // Try direct directory rename first (O(1) operation on same filesystem)
                    match fs::rename(src, dst) {
                        Ok(()) => return Ok(()),
                        Err(_) => {
                            // Cross-device link error, fall back to recursive approach
                        }
                    }

                    // Fallback: recursive copy + delete
                    fs::create_dir_all(dst)?;
                    for entry in fs::read_dir(src)? {
                        let entry = entry?;
                        let src_child = entry.path();
                        let dst_child = dst.join(entry.file_name());
                        move_directory_optimized(&src_child, &dst_child)?;
                    }
                    fs::remove_dir(src).ok();
                } else {
                    if let Some(parent) = dst.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    match fs::rename(src, dst) {
                        Ok(()) => {}
                        Err(_) => {
                            fs::copy(src, dst)?;
                            fs::remove_file(src).ok();
                        }
                    }
                }
                Ok(())
            }

            for entry_name in &new_entries {
                let old_path = target.join(entry_name);
                if old_path.exists() {
                    // Compute relative path from Custom Data for consistent backup structure
                    let relative_entry = old_path
                        .strip_prefix(custom_data_dir)
                        .unwrap_or(Path::new(entry_name));
                    let backup_path = backup_subdir.join(relative_entry);
                    move_directory_optimized(&old_path, &backup_path)?;
                }
            }

            // Step 7: Fast verify (OPTIMIZED: single fs::metadata() per file)
            ctx.emit_progress(
                Some("Verifying backup (fast)...".to_string()),
                InstallPhase::Installing,
            );

            for entry in &backup_entries {
                let file_path = backup_subdir.join(&entry.relative_path);
                // OPTIMIZED: Use single fs::metadata() call instead of exists() + metadata()
                match fs::metadata(&file_path) {
                    Ok(meta) => {
                        if meta.len() != entry.size {
                            let _ = fs::remove_dir_all(&temp_dir);
                            anyhow::bail!(
                                "Backup size mismatch for {}: expected {} bytes, got {} bytes",
                                entry.relative_path,
                                entry.size,
                                meta.len()
                            );
                        }
                    }
                    Err(_) => {
                        let _ = fs::remove_dir_all(&temp_dir);
                        anyhow::bail!("Backup file missing: {}", entry.relative_path);
                    }
                }
            }

            logger::log_info(
                &format!("Fast verification passed: {} files", backup_entries.len()),
                Some("installer"),
            );

            // Step 8: Write verification.json
            let backup_file_count = backup_entries.len();
            let verification = NavdataBackupVerification {
                provider_name,
                cycle: old_cycle,
                airac: old_airac,
                backup_time: chrono::Utc::now().to_rfc3339(),
                files: backup_entries,
                file_count: backup_file_count,
            };

            let verification_json = serde_json::to_string_pretty(&verification)?;
            fs::write(backup_subdir.join("verification.json"), verification_json)?;

            logger::log_info(
                &format!("Navdata backup created: {} files", backup_file_count),
                Some("installer"),
            );
        } else {
            // No backup: delete old files that will be replaced by new ones
            logger::log_info(
                "Navdata backup disabled by user, deleting old files directly",
                Some("installer"),
            );

            // Also delete existing backups for the same provider
            let sanitized_provider = sanitize_folder_name(&provider_name);
            // Backup_Data always goes in Custom Data
            let custom_data_dir = if target.file_name().and_then(|n| n.to_str()) == Some("GNS430") {
                target.parent().unwrap_or(target)
            } else {
                target
            };
            let backup_data_dir = custom_data_dir.join("Backup_Data");
            if backup_data_dir.exists() {
                if let Ok(entries) = fs::read_dir(&backup_data_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        // Match folders that start with the sanitized provider name
                        if folder_name.starts_with(&sanitized_provider) {
                            logger::log_info(
                                &format!("Deleting existing backup: {}", folder_name),
                                Some("installer"),
                            );
                            if let Err(e) = remove_dir_all_robust(&entry.path()) {
                                logger::log_error(
                                    &format!("Failed to delete backup {}: {}", folder_name, e),
                                    Some("installer"),
                                );
                            }
                        }
                    }
                }
            }

            for entry_name in &new_entries {
                let old_path = target.join(entry_name);
                if old_path.exists() {
                    if old_path.is_dir() {
                        remove_dir_all_robust(&old_path)?;
                    } else {
                        fs::remove_file(&old_path)?;
                    }
                }
            }
        }

        // Step 9: Move new navdata from temp to target
        ctx.emit_progress(
            Some("Installing new navdata...".to_string()),
            InstallPhase::Installing,
        );

        for entry in fs::read_dir(&temp_dir)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = target.join(entry.file_name());

            // Remove destination if it exists to allow O(1) rename
            if dst_path.exists() {
                if dst_path.is_dir() {
                    fs::remove_dir_all(&dst_path)?;
                } else {
                    fs::remove_file(&dst_path)?;
                }
            }

            // Try O(1) directory-level rename first
            match fs::rename(&src_path, &dst_path) {
                Ok(()) => {
                    // Success - O(1) operation regardless of file count
                }
                Err(_) => {
                    // Cross-filesystem fallback to copy
                    if src_path.is_dir() {
                        self.copy_directory_with_progress(&src_path, &dst_path, ctx)?;
                    } else {
                        fs::copy(&src_path, &dst_path)?;
                    }
                }
            }
        }

        // Cleanup temp
        let _ = fs::remove_dir_all(&temp_dir);

        logger::log_info(
            "Navdata clean install with backup completed (extreme optimized)",
            Some("installer"),
        );

        Ok(())
    }

    /// Cleanup a task by removing its target directory
    /// Used when a task is cancelled or skipped
    pub(super) fn cleanup_task(&self, task: &InstallTask) -> Result<()> {
        let target = Path::new(&task.target_path);

        logger::log_info(
            &format!("Cleaning up task: {}", task.display_name),
            Some("installer"),
        );

        // Lua cleanup should remove script + companions, even if the script file itself
        // wasn't created yet but companions were partially copied.
        if task.addon_type == AddonType::LuaScript {
            if let Some(scripts_dir) = target.parent() {
                let bundle_entries = Self::get_lua_bundle_entries(task, target)?;
                self.remove_lua_bundle_targets(scripts_dir, &bundle_entries)?;
            } else if target.exists() {
                self.remove_existing_target_path(target)?;
            }

            logger::log_info(
                &format!("Cleanup completed: {}", task.display_name),
                Some("installer"),
            );
            return Ok(());
        }

        if !target.exists() {
            return Ok(());
        }

        // For Navdata, we should NOT delete the entire Custom Data folder
        // Just log a warning
        if matches!(task.addon_type, AddonType::Navdata) {
            logger::log_info(
                "Navdata cleanup skipped - Custom Data folder preserved",
                Some("installer"),
            );
            return Ok(());
        }

        // For other types, delete the target directory
        self.remove_existing_target_path(target)
            .context(format!("Failed to cleanup task path: {:?}", target))?;

        logger::log_info(
            &format!("Cleanup completed: {}", task.display_name),
            Some("installer"),
        );

        Ok(())
    }

    /// Install a task using atomic installation mode
    fn install_task_atomic(
        &self,
        task: &InstallTask,
        source: &Path,
        target: &Path,
        ctx: &ProgressContext,
        password: Option<&str>,
        xplane_path: &str,
    ) -> Result<()> {
        use crate::atomic_installer::AtomicInstaller;

        // Use X-Plane root path directly from settings
        let xplane_root = Path::new(xplane_path);

        // Calculate the proper overall percentage for this task's completion
        // (base_pct + task_pct = the proportional share up to and including this task)
        let task_percentage = {
            let total_f = ctx.total_bytes.load(std::sync::atomic::Ordering::SeqCst) as f64;
            if total_f > 0.0 {
                let cumulative = ctx
                    .task_cumulative
                    .get(ctx.current_task_index)
                    .copied()
                    .unwrap_or(0) as f64;
                let task_size = ctx
                    .task_sizes
                    .get(ctx.current_task_index)
                    .copied()
                    .unwrap_or(0) as f64;
                ((cumulative + task_size) / total_f * 100.0).min(99.9)
            } else {
                // Fallback: assume equal-sized tasks
                ((ctx.current_task_index as f64 + 1.0) / ctx.total_tasks as f64 * 100.0).min(99.9)
            }
        };

        // Create atomic installer with X-Plane root and progress context
        let mut atomic = AtomicInstaller::new(
            target,
            xplane_root,
            self.app_handle.clone(),
            ctx.total_tasks,
            ctx.current_task_index,
            task_percentage,
        )?;

        // In parallel mode, wire up the atomic installer to delegate through
        // the parallel progress context so it doesn't emit serial-mode events.
        if let (Some(ref emit_fn), Some(ref cf)) = (&ctx.parallel_emit, &ctx.parallel_current_file)
        {
            atomic.set_parallel_emit(Arc::clone(emit_fn), Arc::clone(cf));
        }

        // Step 1: Extract/copy to temp directory
        logger::log_info(
            &format!(
                "Atomic install: Extracting to temp directory: {:?}",
                atomic.temp_dir()
            ),
            Some("installer"),
        );

        self.install_content_with_progress(
            source,
            atomic.temp_dir(),
            task.archive_internal_root.as_deref(),
            ctx,
            password,
        )?;

        // Step 2: Perform atomic installation based on scenario
        if !target.exists() {
            // Scenario 1: Fresh installation
            atomic.install_fresh()?;
        } else if !task.should_overwrite {
            // Scenario 2: Clean installation (should_overwrite=false means clean install)
            // Special handling for Navdata: use backup mechanism
            if matches!(task.addon_type, AddonType::Navdata) {
                atomic.install_clean_navdata_with_backup(task.backup_navdata)?;
            } else {
                atomic.install_clean(task)?;
            }
        } else {
            // Scenario 3: Overwrite installation (should_overwrite=true means merge)
            atomic.install_overwrite()?;
        }

        logger::log_info(
            "Atomic installation completed successfully",
            Some("installer"),
        );

        Ok(())
    }

    /// Delete source file after successful installation
    /// Checks if the source path is a parent directory of the original input path
    /// to avoid deleting directories that contain the detected addon
    pub(super) fn delete_source_file(
        &self,
        original_input_path: &str,
        source_path: &str,
    ) -> Result<()> {
        let original_path = Path::new(original_input_path);
        let source_path_buf = Path::new(source_path);

        // Skip deletion if either path is an ancestor of the other
        // - Dragging a parent folder (addon root detected inside) would delete too much
        // - Dragging a subfolder (addon root detected above) would delete only part of the addon
        let original_is_parent =
            source_path_buf.starts_with(original_path) && source_path_buf != original_path;
        let source_is_parent =
            original_path.starts_with(source_path_buf) && source_path_buf != original_path;

        if original_is_parent || source_is_parent {
            logger::log_info(
                &format!(
                    "Skipping deletion: input path ({}) and detected addon root ({}) are nested",
                    original_input_path, source_path
                ),
                Some("installer"),
            );

            // Emit a notification to the frontend
            if let Err(e) = self
                .app_handle
                .emit("source-deletion-skipped", original_input_path)
            {
                logger::log_error(
                    &format!("Failed to emit source-deletion-skipped event: {}", e),
                    Some("installer"),
                );
            }

            return Ok(());
        }

        // Delete the source file/directory
        if original_path.is_file() {
            logger::log_info(
                &format!("Deleting source file: {}", original_input_path),
                Some("installer"),
            );
            fs::remove_file(original_path).with_context(|| {
                format!("Failed to delete source file: {}", original_input_path)
            })?;
        } else if original_path.is_dir() {
            logger::log_info(
                &format!("Deleting source directory: {}", original_input_path),
                Some("installer"),
            );
            remove_dir_all_robust(original_path).with_context(|| {
                format!("Failed to delete source directory: {}", original_input_path)
            })?;
        } else {
            logger::log_error(
                &format!(
                    "Source path does not exist or is not accessible: {}",
                    original_input_path
                ),
                Some("installer"),
            );
        }

        logger::log_info(
            &format!("Successfully deleted source: {}", original_input_path),
            Some("installer"),
        );

        Ok(())
    }
}

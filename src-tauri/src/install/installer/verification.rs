use super::*;

impl Installer {
    /// Verify installation by checking marker files, verifying file hashes,
    /// and optionally verifying file hashes with retry logic
    /// Returns verification statistics if hash verification was performed
    pub(super) fn verify_installation(
        &self,
        task: &InstallTask,
        ctx: &ProgressContext,
    ) -> Result<Option<crate::models::VerificationStats>> {
        let target = Path::new(&task.target_path);

        // Phase 1: Basic marker file verification (10% of verification progress)
        ctx.set_verification_progress(0.0);
        ctx.emit_progress(
            Some("Checking marker files...".to_string()),
            InstallPhase::Verifying,
        );
        self.verify_marker_files(task)?;
        ctx.set_verification_progress(10.0);
        ctx.emit_progress(Some("Marker files OK".to_string()), InstallPhase::Verifying);

        // Check if inline verification already passed during extraction
        if ctx
            .inline_verified
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            let count = ctx.inline_verified_count.load(Ordering::SeqCst) as usize;
            logger::log_info(
                &format!(
                    "Skipping verification re-read: {} files verified inline during extraction",
                    count
                ),
                Some("installer"),
            );
            ctx.set_verification_progress(100.0);
            ctx.emit_progress(
                Some("Inline verification passed".to_string()),
                InstallPhase::Verifying,
            );
            return Ok(Some(crate::models::VerificationStats {
                total_files: count,
                verified_files: count,
                failed_files: 0,
                retried_files: 0,
                skipped_files: 0,
            }));
        }

        // Phase 2: Hash verification (if enabled and hashes available)
        // IMPORTANT: When verification is disabled, skip ALL hash operations to save time
        if !task.enable_verification {
            logger::log_info(
                "Hash verification disabled for this task - skipping all hash operations",
                Some("installer"),
            );
            return Ok(None);
        }

        // Get expected hashes (must be available at this point)
        // Note: For 7z archives, hashes should have been computed during extraction if verification was enabled
        let expected_hashes = match &task.file_hashes {
            Some(hashes) if !hashes.is_empty() => hashes.clone(),
            _ => {
                // No hashes available - this can happen for:
                // 1. 7z/RAR archives (hashes computed during extraction)
                // 2. Hash collection failed during analysis
                // 3. Empty archives
                logger::log_info(
                    "No hashes available for verification - skipping hash verification",
                    Some("installer"),
                );
                return Ok(None);
            }
        };

        let total_expected = expected_hashes.len();

        logger::log_info(
            &format!("Verifying {} files with hash checking", total_expected),
            Some("installer"),
        );

        // Update progress: starting hash verification (10% -> 70%)
        ctx.set_verification_progress(15.0);
        ctx.emit_progress(
            Some(format!("Verifying {} files...", total_expected)),
            InstallPhase::Verifying,
        );

        let verifier = crate::verifier::FileVerifier::new();

        // Use verification with progress callback
        // Progress range: 15% -> 70% (55% range for hash verification)
        let ctx_clone = ctx.clone();
        let mut failed_files = verifier.verify_files_with_progress(
            target,
            &expected_hashes,
            move |verified, total| {
                if total > 0 {
                    // Map verified/total to 15% -> 70% range
                    let progress = 15.0 + (verified as f64 / total as f64) * 55.0;
                    ctx_clone.set_verification_progress(progress);
                    ctx_clone.emit_progress(
                        Some(format!("Verified {}/{} files", verified, total)),
                        InstallPhase::Verifying,
                    );
                }
            },
        )?;

        // Update progress: initial verification done (70%)
        ctx.set_verification_progress(70.0);

        let _initial_failed_count = failed_files.len();
        let mut retried_count = 0;

        // Phase 3: Retry failed files (up to 3 times) (70% -> 95%)
        if !failed_files.is_empty() {
            logger::log_info(
                &format!(
                    "Retrying {} failed files (max 3 attempts)",
                    failed_files.len()
                ),
                Some("installer"),
            );

            ctx.emit_progress(
                Some(format!("Retrying {} files...", failed_files.len())),
                InstallPhase::Verifying,
            );

            retried_count = failed_files.len();
            failed_files = self.retry_failed_files(task, failed_files, &expected_hashes)?;

            ctx.set_verification_progress(95.0);
        } else {
            ctx.set_verification_progress(95.0);
        }

        // Phase 4: Final check and build statistics (95% -> 100%)
        if !failed_files.is_empty() {
            self.log_verification_failures(&failed_files);

            let _stats = crate::models::VerificationStats {
                total_files: total_expected,
                verified_files: total_expected - failed_files.len(),
                failed_files: failed_files.len(),
                retried_files: retried_count,
                skipped_files: 0,
            };

            return Err(anyhow::anyhow!(
                "Verification failed: {} files still failing after retries",
                failed_files.len()
            ));
        }

        logger::log_info(
            &format!("All {} files verified successfully", total_expected),
            Some("installer"),
        );

        ctx.set_verification_progress(100.0);
        ctx.emit_progress(
            Some("Verification complete".to_string()),
            InstallPhase::Verifying,
        );

        // Build success statistics
        let stats = crate::models::VerificationStats {
            total_files: total_expected,
            verified_files: total_expected,
            failed_files: 0,
            retried_files: retried_count,
            skipped_files: 0,
        };

        Ok(Some(stats))
    }

    /// Verify marker files (existing logic, extracted)
    fn verify_marker_files(&self, task: &InstallTask) -> Result<()> {
        use walkdir::WalkDir;

        let target = Path::new(&task.target_path);

        // Check if target directory exists
        if !target.exists() {
            return Err(anyhow::anyhow!(
                "Installation verification failed: Target directory does not exist: {:?}",
                target
            ));
        }

        // Check if target directory is empty
        let mut has_files = false;
        for entry in WalkDir::new(target)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                has_files = true;
                break;
            }
        }

        if !has_files {
            return Err(anyhow::anyhow!(
                "Installation verification failed: Target directory is empty: {:?}",
                target
            ));
        }

        // Type-specific verification: check for typical marker files
        match task.addon_type {
            crate::models::AddonType::Aircraft => {
                // Check for .acf files
                let mut found_acf = false;
                for entry in WalkDir::new(target)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        if let Some(ext) = entry.path().extension() {
                            if ext == "acf" {
                                found_acf = true;
                                break;
                            }
                        }
                    }
                }
                if !found_acf {
                    return Err(anyhow::anyhow!(
                        "Installation verification failed: No .acf file found in aircraft directory: {:?}",
                        target
                    ));
                }
            }
            crate::models::AddonType::Scenery => {
                // Check for Earth nav data folder and .dsf files
                let earth_nav_data = target.join("Earth nav data");
                if !earth_nav_data.exists() {
                    return Err(anyhow::anyhow!(
                        "Installation verification failed: No 'Earth nav data' folder found in scenery directory: {:?}",
                        target
                    ));
                }

                // Check for at least one .dsf file
                let mut found_dsf = false;
                for entry in WalkDir::new(&earth_nav_data)
                    .max_depth(5)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        if let Some(ext) = entry.path().extension() {
                            if ext == "dsf" {
                                found_dsf = true;
                                break;
                            }
                        }
                    }
                }
                if !found_dsf {
                    return Err(anyhow::anyhow!(
                        "Installation verification failed: No .dsf file found in scenery directory: {:?}",
                        target
                    ));
                }
            }
            crate::models::AddonType::SceneryLibrary => {
                // Check for library.txt
                let library_txt = target.join("library.txt");
                if !library_txt.exists() {
                    return Err(anyhow::anyhow!(
                        "Installation verification failed: No library.txt found in scenery library directory: {:?}",
                        target
                    ));
                }
            }
            crate::models::AddonType::Plugin => {
                // Check for .xpl files (in platform-specific folders or root)
                let mut found_xpl = false;
                for entry in WalkDir::new(target)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        if let Some(ext) = entry.path().extension() {
                            if ext == "xpl" {
                                found_xpl = true;
                                break;
                            }
                        }
                    }
                }
                if !found_xpl {
                    return Err(anyhow::anyhow!(
                        "Installation verification failed: No .xpl file found in plugin directory: {:?}",
                        target
                    ));
                }
            }
            crate::models::AddonType::Navdata => {
                // Check for cycle.json
                // For GNS430: cycle.json may be in a subfolder (due to grandparent extraction)
                // For regular navdata: cycle.json is directly in target
                let found = if task.display_name.contains("GNS430") {
                    WalkDir::new(target)
                        .max_depth(3)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .any(|e| {
                            e.file_type().is_file() && e.file_name().to_str() == Some("cycle.json")
                        })
                } else {
                    target.join("cycle.json").exists()
                };
                if !found {
                    return Err(anyhow::anyhow!(
                        "Installation verification failed: No cycle.json found in navdata directory: {:?}",
                        target
                    ));
                }
            }
            crate::models::AddonType::Livery => {
                // For liveries, just check that the directory exists and has some content
                // No specific marker file required
                if !target.exists() || !target.is_dir() {
                    return Err(anyhow::anyhow!(
                        "Installation verification failed: Livery directory not found: {:?}",
                        target
                    ));
                }
            }
            crate::models::AddonType::LuaScript => {
                // For Lua scripts, check that the file exists
                if !target.exists() {
                    return Err(anyhow::anyhow!(
                        "Installation verification failed: Lua script not found: {:?}",
                        target
                    ));
                }
            }
        }

        Ok(())
    }

    /// Retry extraction for failed files only (up to 3 times)
    fn retry_failed_files(
        &self,
        task: &InstallTask,
        mut failed_files: Vec<crate::models::FileVerificationResult>,
        expected_hashes: &std::collections::HashMap<String, crate::models::FileHash>,
    ) -> Result<Vec<crate::models::FileVerificationResult>> {
        const MAX_RETRIES: u8 = 3;
        let source = Path::new(&task.source_path);
        let target = Path::new(&task.target_path);

        // Reuse verifier instance across retries for better performance
        let verifier = crate::verifier::FileVerifier::new();

        for retry_attempt in 1..=MAX_RETRIES {
            if failed_files.is_empty() {
                break;
            }

            logger::log_info(
                &format!(
                    "Retry attempt {}/{} for {} files",
                    retry_attempt,
                    MAX_RETRIES,
                    failed_files.len()
                ),
                Some("installer"),
            );

            // Track which files were successfully re-extracted
            let mut re_extracted_files = Vec::new();

            // Re-extract failed files
            for failed in &mut failed_files {
                logger::log_debug(
                    &format!("Retrying file: {}", failed.path),
                    Some("installer"),
                    None,
                );

                match self.re_extract_single_file(
                    source,
                    target,
                    &failed.path,
                    task.archive_internal_root.as_deref(),
                    task.extraction_chain.as_ref(),
                    task.password.as_deref(),
                ) {
                    Ok(_) => {
                        failed.retry_count = retry_attempt;
                        re_extracted_files.push(failed.path.clone());
                        logger::log_debug(
                            &format!(
                                "Re-extracted file: {} (attempt {})",
                                failed.path, retry_attempt
                            ),
                            Some("installer"),
                            None,
                        );
                    }
                    Err(e) => {
                        logger::log_error(
                            &format!("Failed to re-extract {}: {}", failed.path, e),
                            Some("installer"),
                        );
                        failed.error = Some(e.to_string());
                    }
                }
            }

            // Re-verify only the files that were successfully re-extracted
            let still_failed: Vec<crate::models::FileVerificationResult> = failed_files
                .into_iter()
                .filter_map(|mut result| {
                    // Skip files that failed to re-extract
                    if !re_extracted_files.contains(&result.path) {
                        return Some(result);
                    }

                    let file_path = target.join(&result.path);
                    let expected = expected_hashes.get(&result.path)?;

                    let verification =
                        verifier.verify_single_file(&file_path, &result.path, expected);

                    if verification.success {
                        logger::log_info(
                            &format!("File verified after retry: {}", result.path),
                            Some("installer"),
                        );
                        None // Success, remove from failed list
                    } else {
                        result.actual_hash = verification.actual_hash;
                        result.success = false;
                        Some(result)
                    }
                })
                .collect();

            failed_files = still_failed;

            if failed_files.is_empty() {
                logger::log_info(
                    &format!(
                        "All files verified successfully after {} retries",
                        retry_attempt
                    ),
                    Some("installer"),
                );
                break;
            }
        }

        Ok(failed_files)
    }

    /// Re-extract a single file from archive
    fn re_extract_single_file(
        &self,
        source: &Path,
        target: &Path,
        relative_path: &str,
        internal_root: Option<&str>,
        extraction_chain: Option<&crate::models::ExtractionChain>,
        password: Option<&str>,
    ) -> Result<()> {
        // For directories, just copy the file again
        if source.is_dir() {
            let source_file = source.join(relative_path);
            let target_file = target.join(relative_path);

            if let Some(parent) = target_file.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(&source_file, &target_file)?;
            return Ok(());
        }

        // For archives, extract based on format
        let ext = source
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

        match ext {
            "zip" => self.re_extract_from_zip(
                source,
                target,
                relative_path,
                internal_root,
                extraction_chain,
                password,
            ),
            "7z" => self.re_extract_from_7z(
                source,
                target,
                relative_path,
                internal_root,
                extraction_chain,
                password,
            ),
            "rar" => self.re_extract_from_rar(
                source,
                target,
                relative_path,
                internal_root,
                extraction_chain,
                password,
            ),
            _ => Err(anyhow::anyhow!(
                "Unsupported archive format for retry: {}",
                ext
            )),
        }
    }

    /// Re-extract single file from ZIP
    /// Note: For nested archives (extraction_chain), this only works if the file is in the outermost ZIP.
    /// True nested ZIPs would require re-extracting through all layers, which is not implemented.
    /// In practice, this limitation is acceptable because:
    /// 1. Initial extraction handles nested archives correctly
    /// 2. Retry is only needed for corrupted files, which is rare
    /// 3. If a nested ZIP itself is corrupted, the entire task would fail anyway
    fn re_extract_from_zip(
        &self,
        archive_path: &Path,
        target: &Path,
        relative_path: &str,
        internal_root: Option<&str>,
        _extraction_chain: Option<&crate::models::ExtractionChain>,
        password: Option<&str>,
    ) -> Result<()> {
        use std::io::copy;
        use zip::ZipArchive;

        let file = fs::File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;

        // Build full path in archive
        let archive_path_str = if let Some(root) = internal_root {
            format!("{}/{}", root.trim_end_matches('/'), relative_path)
        } else {
            relative_path.to_string()
        };

        let archive_path_normalized = archive_path_str.replace('\\', "/");

        // Find the file index first
        let mut file_index = None;
        let mut is_encrypted = false;
        for i in 0..archive.len() {
            // Use by_index_raw to avoid triggering decryption errors when reading metadata
            let file = archive.by_index_raw(i)?;
            let name = file.name().replace('\\', "/");

            if name == archive_path_normalized {
                file_index = Some(i);
                is_encrypted = file.encrypted();
                break;
            }
        }

        let i = file_index
            .ok_or_else(|| anyhow::anyhow!("File not found in ZIP: {}", archive_path_normalized))?;

        // Now extract the file
        let target_path = target.join(relative_path);

        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Extract file
        let mut outfile = fs::File::create(&target_path)?;

        if is_encrypted {
            if let Some(pwd) = password {
                let mut decrypted = archive
                    .by_index_decrypt(i, pwd.as_bytes())
                    .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;
                copy(&mut decrypted, &mut outfile)?;
            } else {
                return Err(anyhow::anyhow!("Password required for encrypted file"));
            }
        } else {
            let mut file = archive.by_index(i)?;
            copy(&mut file, &mut outfile)?;
        }

        Ok(())
    }

    /// Re-extract single file from 7z (requires full re-extraction to temp)
    /// Note: 7z library doesn't support single-file extraction, so we extract the entire archive
    /// to a temp directory and then copy the specific file. This is inefficient but necessary.
    fn re_extract_from_7z(
        &self,
        archive_path: &Path,
        target: &Path,
        relative_path: &str,
        _internal_root: Option<&str>,
        _extraction_chain: Option<&crate::models::ExtractionChain>,
        password: Option<&str>,
    ) -> Result<()> {
        use tempfile::TempDir;

        let sanitized_relative = sanitize_path(Path::new(relative_path))
            .ok_or_else(|| anyhow::anyhow!("Unsafe path in archive: {}", relative_path))?;

        // 7z doesn't support single-file extraction easily
        // Extract to temp, then copy the specific file
        let temp_dir = TempDir::new()?;
        let mut skipped_count = 0usize;

        // Extract entire archive to temp
        if let Some(pwd) = password {
            let mut reader =
                sevenz_rust2::ArchiveReader::open(archive_path, sevenz_rust2::Password::from(pwd))
                    .map_err(|e| anyhow::anyhow!("Failed to open 7z with password: {}", e))?;
            reader
                .for_each_entries(|entry, reader| {
                    let entry_path = match sanitize_path(Path::new(entry.name())) {
                        Some(path) => path,
                        None => {
                            skipped_count += 1;
                            logger::log_debug(
                                &format!("Skipping 7z entry with unsafe path: {}", entry.name()),
                                Some("installer"),
                                None,
                            );
                            return Ok(true);
                        }
                    };
                    let dest_path = temp_dir.path().join(&entry_path);
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
                .map_err(|e| anyhow::anyhow!("7z extraction failed: {}", e))?;
        } else {
            let mut reader =
                sevenz_rust2::ArchiveReader::open(archive_path, sevenz_rust2::Password::empty())
                    .map_err(|e| anyhow::anyhow!("Failed to open 7z: {}", e))?;
            reader
                .for_each_entries(|entry, reader| {
                    let entry_path = match sanitize_path(Path::new(entry.name())) {
                        Some(path) => path,
                        None => {
                            skipped_count += 1;
                            logger::log_debug(
                                &format!("Skipping 7z entry with unsafe path: {}", entry.name()),
                                Some("installer"),
                                None,
                            );
                            return Ok(true);
                        }
                    };
                    let dest_path = temp_dir.path().join(&entry_path);
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
                .map_err(|e| anyhow::anyhow!("7z extraction failed: {}", e))?;
        }

        if skipped_count > 0 {
            logger::log_info(
                &format!(
                    "Skipped {} unsafe 7z entries during re-extract",
                    skipped_count
                ),
                Some("installer"),
            );
        }

        // Find and copy the specific file
        let temp_file = temp_dir.path().join(&sanitized_relative);
        if !temp_file.exists() {
            return Err(anyhow::anyhow!(
                "File not found after 7z extraction: {}",
                relative_path
            ));
        }

        let target_file = target.join(&sanitized_relative);
        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(&temp_file, &target_file)?;

        Ok(())
    }

    /// Re-extract single file from RAR (requires full re-extraction to temp)
    fn re_extract_from_rar(
        &self,
        archive_path: &Path,
        target: &Path,
        relative_path: &str,
        internal_root: Option<&str>,
        _extraction_chain: Option<&crate::models::ExtractionChain>,
        password: Option<&str>,
    ) -> Result<()> {
        // Create secure temp directory
        let temp_dir = tempfile::Builder::new()
            .prefix("xfi_rar_retry_")
            .tempdir()
            .context("Failed to create temp directory for RAR retry")?;

        // Extract using the typestate pattern (with password if provided)
        let archive_builder = if let Some(pwd) = password {
            unrar::Archive::with_password(archive_path, pwd)
        } else {
            unrar::Archive::new(archive_path)
        };

        let mut arch = archive_builder
            .open_for_processing()
            .map_err(|e| anyhow::anyhow!("Failed to open RAR for retry: {:?}", e))?;

        // Extract all files to temp directory
        while let Some(header) = arch
            .read_header()
            .map_err(|e| anyhow::anyhow!("Failed to read RAR header: {:?}", e))?
        {
            arch = if header.entry().is_file() {
                header
                    .extract_with_base(temp_dir.path())
                    .map_err(|e| anyhow::anyhow!("Failed to extract RAR entry: {:?}", e))?
            } else {
                header
                    .skip()
                    .map_err(|e| anyhow::anyhow!("Failed to skip RAR entry: {:?}", e))?
            };
        }

        // Determine the source path in temp directory
        let source_file = if let Some(root) = internal_root {
            let root_normalized = root.replace('\\', "/");
            temp_dir.path().join(&root_normalized).join(relative_path)
        } else {
            temp_dir.path().join(relative_path)
        };

        if !source_file.exists() {
            return Err(anyhow::anyhow!(
                "File not found after RAR extraction: {}",
                relative_path
            ));
        }

        // Copy to target
        let target_file = target.join(relative_path);
        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(&source_file, &target_file)
            .context(format!("Failed to copy RAR file: {}", relative_path))?;

        Ok(())
    }

    /// Log verification failures with appropriate detail level
    fn log_verification_failures(&self, failed: &[crate::models::FileVerificationResult]) {
        // Basic level: summary
        logger::log_error(
            &format!("Verification failed: {} files", failed.len()),
            Some("installer"),
        );

        // Full level: file names
        let file_names: Vec<&str> = failed
            .iter()
            .take(10) // Limit to first 10 files
            .map(|f| f.path.as_str())
            .collect();

        if !file_names.is_empty() {
            logger::log_info(
                &format!(
                    "Failed files: {}{}",
                    file_names.join(", "),
                    if failed.len() > 10 {
                        format!(" (and {} more)", failed.len() - 10)
                    } else {
                        String::new()
                    }
                ),
                Some("installer"),
            );
        }

        // Debug level: full details
        for result in failed {
            logger::log_debug(
                &format!(
                    "File: {}, Expected: {}, Actual: {:?}, Retries: {}, Error: {:?}",
                    result.path,
                    result.expected_hash,
                    result.actual_hash,
                    result.retry_count,
                    result.error
                ),
                Some("installer"),
                None,
            );
        }
    }
}

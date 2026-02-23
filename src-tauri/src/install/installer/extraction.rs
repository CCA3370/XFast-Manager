use super::*;

impl Installer {
    /// Copy a directory recursively with progress tracking
    /// Uses parallel processing for better performance on multi-core systems
    pub(super) fn copy_directory_with_progress(
        &self,
        source: &Path,
        target: &Path,
        ctx: &ProgressContext,
    ) -> Result<()> {
        self.copy_directory_internal(source, target, Some(ctx))
    }

    /// Copy a directory recursively without progress tracking
    /// Used for backup operations that shouldn't affect installation progress
    pub(super) fn copy_directory_without_progress(
        &self,
        source: &Path,
        target: &Path,
    ) -> Result<()> {
        self.copy_directory_internal(source, target, None)
    }

    /// Internal implementation for directory copying
    /// Uses parallel processing for better performance on multi-core systems
    fn copy_directory_internal(
        &self,
        source: &Path,
        target: &Path,
        ctx: Option<&ProgressContext>,
    ) -> Result<()> {
        if !target.exists() {
            fs::create_dir_all(target)?;
        }

        // Collect all entries first
        let entries: Vec<_> = walkdir::WalkDir::new(source)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .collect();

        // Create all directories first (must be sequential)
        for entry in &entries {
            if entry.file_type().is_dir() {
                let relative = entry
                    .path()
                    .strip_prefix(source)
                    .context("Failed to strip prefix")?;
                let target_path = target.join(relative);
                fs::create_dir_all(&target_path)?;
            }
        }

        // Copy files in parallel using rayon
        use rayon::prelude::*;

        entries
            .par_iter()
            .filter(|entry| entry.file_type().is_file())
            .try_for_each(|entry| -> Result<()> {
                let source_path = entry.path();
                let relative = source_path.strip_prefix(source)?;
                let target_path = target.join(relative);

                let file_size = entry.metadata()?.len();
                let file_name = source_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                // Use optimized buffered copy
                let mut source_file = fs::File::open(source_path)
                    .context(format!("Failed to open source file {:?}", source_path))?;
                let mut target_file = fs::File::create(&target_path)
                    .context(format!("Failed to create target file {:?}", target_path))?;
                copy_file_optimized(&mut source_file, &mut target_file)?;

                // Remove read-only attribute from copied file to avoid future deletion issues
                let _ = remove_readonly_attribute(&target_path);

                // Only update progress if context is provided
                if let Some(ctx) = ctx {
                    ctx.add_bytes(file_size);
                    ctx.emit_progress(Some(file_name), InstallPhase::Installing);
                }

                Ok(())
            })?;

        Ok(())
    }

    /// Extract an archive with progress tracking
    pub(super) fn extract_archive_with_progress(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
        expected_hashes: Option<&HashMap<String, crate::models::FileHash>>,
    ) -> Result<()> {
        let extract_start = Instant::now();
        let extension = archive
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

        crate::log_debug!(
            &format!("[TIMING] Archive extraction started: {} format", extension),
            "installer_timing"
        );

        match extension {
            "zip" => {
                self.extract_zip_with_progress(archive, target, internal_root, ctx, password, expected_hashes)?
            }
            "7z" => self.extract_7z_with_progress(archive, target, internal_root, ctx, password)?,
            "rar" => {
                self.extract_rar_with_progress(archive, target, internal_root, ctx, password)?
            }
            _ => return Err(anyhow::anyhow!("Unsupported archive format: {}", extension)),
        }

        crate::log_debug!(
            &format!(
                "[TIMING] Archive extraction completed in {:.2}ms: {} format",
                extract_start.elapsed().as_secs_f64() * 1000.0,
                extension
            ),
            "installer_timing"
        );

        Ok(())
    }

    /// Extract ZIP archive with progress tracking
    /// Supports password-protected ZIP files (both ZipCrypto and AES encryption)
    /// Uses parallel extraction for better performance on multi-core systems
    fn extract_zip_with_progress(
        &self,
        archive_path: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
        expected_hashes: Option<&HashMap<String, crate::models::FileHash>>,
    ) -> Result<()> {
        use std::sync::Arc;
        use zip::ZipArchive;

        // Open archive and collect file metadata
        let file = fs::File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;

        let internal_root_normalized = internal_root.map(|s| s.replace('\\', "/"));
        let prefix = internal_root_normalized.as_deref();
        let password_bytes = password.map(|p| p.as_bytes().to_vec());

        // Collect all file entries with their metadata
        let mut skipped_count = 0;
        let entries: Vec<_> = (0..archive.len())
            .filter_map(|i| {
                // Use by_index_raw to avoid triggering decryption errors when reading metadata
                let file = match archive.by_index_raw(i) {
                    Ok(f) => f,
                    Err(e) => {
                        logger::log_error(
                            &format!("Failed to read ZIP entry {}: {}", i, e),
                            Some("installer"),
                        );
                        skipped_count += 1;
                        return None;
                    }
                };

                let is_encrypted = file.encrypted();
                let is_dir = file.is_dir();
                let size = file.size();

                let path = match file.enclosed_name() {
                    Some(p) => p.to_path_buf(),
                    None => {
                        logger::log_debug(
                            &format!("Skipping ZIP entry {} with unsafe path: {}", i, file.name()),
                            Some("installer"),
                            None,
                        );
                        skipped_count += 1;
                        return None;
                    }
                };

                let file_path_str = path.to_string_lossy().replace('\\', "/");

                // Check prefix filter
                let relative_path = if let Some(prefix) = prefix {
                    // Ensure prefix ends with '/' for proper directory matching
                    // This prevents "A330" from matching "A330_variant"
                    let prefix_with_slash = if prefix.ends_with('/') {
                        prefix.to_string()
                    } else {
                        format!("{}/", prefix)
                    };

                    // Check if file is inside the prefix directory or is the prefix directory itself
                    if file_path_str == prefix.trim_end_matches('/') {
                        // This is the root directory itself, skip it
                        return None;
                    }

                    if !file_path_str.starts_with(&prefix_with_slash) {
                        return None;
                    }

                    let stripped = file_path_str
                        .strip_prefix(&prefix_with_slash)
                        .unwrap_or(&file_path_str);
                    if stripped.is_empty() {
                        return None;
                    }
                    match sanitize_path(Path::new(stripped)) {
                        Some(p) => p,
                        None => {
                            logger::log_debug(
                                &format!(
                                    "Skipping ZIP entry with unsafe path after sanitization: {}",
                                    stripped
                                ),
                                Some("installer"),
                                None,
                            );
                            skipped_count += 1;
                            return None;
                        }
                    }
                } else {
                    match sanitize_path(&path) {
                        Some(p) => p,
                        None => {
                            logger::log_debug(
                                &format!("Skipping ZIP entry with unsafe path: {}", file_path_str),
                                Some("installer"),
                                None,
                            );
                            skipped_count += 1;
                            return None;
                        }
                    }
                };

                Some((i, relative_path, is_dir, is_encrypted, size))
            })
            .collect();

        if skipped_count > 0 {
            logger::log_info(
                &format!("Skipped {} unsafe or invalid ZIP entries", skipped_count),
                Some("installer"),
            );
        }

        drop(archive); // Close the archive before parallel processing

        // Create all directories first (sequential)
        let file = fs::File::open(archive_path)?;
        let archive = ZipArchive::new(file)?;

        for (_index, relative_path, is_dir, _, _) in &entries {
            if *is_dir {
                let outpath = target.join(relative_path);
                fs::create_dir_all(&outpath)?;
            }
        }

        drop(archive);

        // Extract files in parallel
        use rayon::prelude::*;

        let archive_path = archive_path.to_path_buf();
        let target = target.to_path_buf();
        let password_bytes = Arc::new(password_bytes);
        let expected_hashes_arc = expected_hashes.map(|h| Arc::new(h.clone()));

        // Collect non-directory file entries for chunked processing
        let file_entries: Vec<_> = entries
            .iter()
            .filter(|(_, _, is_dir, _, _)| !is_dir)
            .collect();

        // Calculate chunk size: aim for ~50-500 files per chunk to balance
        // ZipArchive reuse vs parallelism. Each chunk opens ZipArchive once.
        let num_threads = rayon::current_num_threads().max(1);
        let chunk_size = (file_entries.len() / num_threads).clamp(50, 500);

        // Process files in chunks - each chunk shares one ZipArchive instance
        let expected_hashes_ref = expected_hashes_arc.clone();
        file_entries
            .par_chunks(chunk_size)
            .try_for_each(|chunk| -> Result<()> {
                // Each chunk opens ZipArchive only once (instead of per-file)
                let file = fs::File::open(&archive_path)?;
                let mut archive = ZipArchive::new(file)?;

                for (index, relative_path, _, is_encrypted, _) in chunk {
                    let outpath = target.join(relative_path);

                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }

                    // Extract file with or without password, computing CRC32 inline
                    let (file_size, computed_crc) = if *is_encrypted {
                        if let Some(ref pwd) = password_bytes.as_ref() {
                            match archive.by_index_decrypt(*index, pwd) {
                                Ok(mut file) => {
                                    let mut outfile = fs::File::create(&outpath)?;
                                    copy_file_with_crc32(&mut file, &mut outfile)?
                                }
                                Err(e) => {
                                    return Err(e.into());
                                }
                            }
                        } else {
                            return Err(anyhow::anyhow!(
                                "Password required for encrypted file: {}",
                                relative_path.display()
                            ));
                        }
                    } else {
                        let mut file = archive.by_index(*index)?;
                        let mut outfile = fs::File::create(&outpath)?;
                        copy_file_with_crc32(&mut file, &mut outfile)?
                    };

                    // Inline CRC32 verification
                    if let Some(ref hashes) = expected_hashes_ref {
                        let rel_path_str = relative_path.to_string_lossy().replace('\\', "/");
                        if let Some(expected) = hashes.get(&rel_path_str) {
                            let computed_hex = format!("{:08x}", computed_crc);
                            if computed_hex == expected.hash {
                                ctx.inline_verified_count.fetch_add(1, Ordering::SeqCst);
                            }
                            // On mismatch: don't flag error, let normal verification handle it
                        }
                    }

                    let file_name = relative_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    ctx.add_bytes(file_size);
                    ctx.emit_progress(Some(file_name), InstallPhase::Installing);

                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        // Use by_index_raw to get metadata without triggering decryption
                        let file = archive.by_index_raw(*index)?;
                        if let Some(mode) = file.unix_mode() {
                            fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                        }
                    }
                }

                Ok(())
            })?;

        // After all chunks complete, check if ALL expected files were verified inline
        if let Some(ref hashes) = expected_hashes_arc {
            let verified = ctx.inline_verified_count.load(Ordering::SeqCst) as usize;
            if verified == hashes.len() {
                ctx.inline_verified
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }

        Ok(())
    }

    /// Extract 7z archive with progress tracking
    /// Extracts directly to target directory with inline SHA256 hash computation
    fn extract_7z_with_progress(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        use sha2::{Digest, Sha256};

        // Normalize internal_root for path matching
        let internal_root_normalized = internal_root.map(|s| {
            let normalized = s.replace('\\', "/");
            if normalized.ends_with('/') {
                normalized
            } else {
                format!("{}/", normalized)
            }
        });

        // Create target directory
        fs::create_dir_all(target)?;

        // Open archive with or without password
        let mut reader = if let Some(pwd) = password {
            sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::from(pwd))
                .map_err(|e| anyhow::anyhow!("Failed to open 7z with password: {}", e))?
        } else {
            sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty())
                .map_err(|e| anyhow::anyhow!("Failed to open 7z: {}", e))?
        };

        // Extract directly to target with progress reporting and inline SHA256
        reader
            .for_each_entries(|entry, entry_reader| {
                let entry_name = entry.name().replace('\\', "/");

                // Apply internal_root filter
                let relative_path = if let Some(ref prefix) = internal_root_normalized {
                    if entry_name.starts_with(prefix) {
                        entry_name.strip_prefix(prefix).unwrap_or(&entry_name)
                    } else if entry_name == prefix.trim_end_matches('/') {
                        // Skip the root directory itself
                        return Ok(true);
                    } else {
                        // Skip entries outside internal_root
                        return Ok(true);
                    }
                } else {
                    &entry_name
                };

                // Skip empty paths
                if relative_path.is_empty() {
                    return Ok(true);
                }

                // Sanitize path to prevent path traversal
                let sanitized = match sanitize_path(Path::new(relative_path)) {
                    Some(p) => p,
                    None => return Ok(true), // Skip unsafe paths
                };

                let dest_path = target.join(&sanitized);

                if entry.is_directory() {
                    std::fs::create_dir_all(&dest_path)?;
                } else {
                    if let Some(parent) = dest_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    // Compute SHA256 inline while writing
                    let mut file = std::fs::File::create(&dest_path)?;
                    let mut hasher = Sha256::new();
                    let mut buffer = vec![0u8; IO_BUFFER_SIZE];
                    loop {
                        let bytes_read = entry_reader.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        hasher.update(&buffer[..bytes_read]);
                        std::io::Write::write_all(&mut file, &buffer[..bytes_read])?;
                    }
                    let hash = format!("{:x}", hasher.finalize());

                    // Store computed hash for inline verification
                    let relative_str = sanitized.to_string_lossy().replace('\\', "/");
                    ctx.inline_hashes.lock().unwrap().insert(
                        relative_str.clone(),
                        crate::models::FileHash {
                            path: relative_str,
                            hash,
                            algorithm: crate::models::HashAlgorithm::Sha256,
                        },
                    );

                    // Remove read-only attribute
                    let _ = remove_readonly_attribute(&dest_path);

                    // Report progress
                    let file_size = entry.size();
                    let file_name = sanitized
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    ctx.add_bytes(file_size);
                    ctx.emit_progress(Some(file_name), InstallPhase::Installing);
                }
                Ok(true)
            })
            .map_err(|e| anyhow::anyhow!("Failed to extract 7z: {}", e))?;

        Ok(())
    }

    /// Compute SHA256 hashes for all files in installed directory
    /// Used for 7z archives where hashes aren't available from metadata
    #[allow(dead_code)]
    fn compute_installed_file_hashes(
        &self,
        target_dir: &Path,
    ) -> Result<HashMap<String, crate::models::FileHash>> {
        use walkdir::WalkDir;

        let mut hashes = HashMap::new();

        for entry in WalkDir::new(target_dir).follow_links(false) {
            let entry = entry?;

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let relative = path.strip_prefix(target_dir)?;
            let relative_str = relative.to_string_lossy().replace('\\', "/");

            // Compute SHA256
            let hash = self.compute_file_sha256(path)?;

            hashes.insert(
                relative_str.clone(),
                crate::models::FileHash {
                    path: relative_str,
                    hash,
                    algorithm: crate::models::HashAlgorithm::Sha256,
                },
            );
        }

        Ok(hashes)
    }

    /// Compute SHA256 hash of a file
    fn compute_file_sha256(&self, path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        use std::io::Read;

        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; IO_BUFFER_SIZE];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Extract RAR archive with progress tracking
    /// When no internal_root is needed, extracts directly to target (no temp dir)
    fn extract_rar_with_progress(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        if internal_root.is_none() {
            // Direct extraction to target - no temp dir needed
            fs::create_dir_all(target)?;

            let archive_builder = if let Some(pwd) = password {
                unrar::Archive::with_password(archive, pwd)
            } else {
                unrar::Archive::new(archive)
            };

            let mut arch = archive_builder
                .open_for_processing()
                .map_err(|e| anyhow::anyhow!("Failed to open RAR for extraction: {:?}", e))?;

            while let Some(header) = arch
                .read_header()
                .map_err(|e| anyhow::anyhow!("Failed to read RAR header: {:?}", e))?
            {
                arch = if header.entry().is_file() {
                    let size = header.entry().unpacked_size;
                    let result = header
                        .extract_with_base(target)
                        .map_err(|e| anyhow::anyhow!("Failed to extract RAR entry: {:?}", e))?;

                    // Report progress
                    ctx.add_bytes(size);
                    ctx.emit_progress(None, InstallPhase::Installing);

                    result
                } else {
                    header
                        .skip()
                        .map_err(|e| anyhow::anyhow!("Failed to skip RAR entry: {:?}", e))?
                };
            }

            return Ok(());
        }

        // When internal_root is Some, use temp dir approach to strip the root prefix
        let temp_dir = tempfile::Builder::new()
            .prefix("xfastmanager_rar_")
            .tempdir()
            .context("Failed to create secure temp directory")?;

        // Extract using the typestate pattern (with password if provided)
        let archive_builder = if let Some(pwd) = password {
            unrar::Archive::with_password(archive, pwd)
        } else {
            unrar::Archive::new(archive)
        };

        let mut arch = archive_builder
            .open_for_processing()
            .map_err(|e| anyhow::anyhow!("Failed to open RAR for extraction: {:?}", e))?;

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

        // Determine source path with internal_root
        let internal_root_normalized = internal_root.unwrap().replace('\\', "/");
        let source_path = {
            let path = temp_dir.path().join(&internal_root_normalized);
            if path.exists() && path.is_dir() {
                path
            } else {
                temp_dir.path().to_path_buf()
            }
        };

        // Copy with progress tracking
        self.copy_directory_with_progress(&source_path, target, ctx)?;

        // TempDir automatically cleans up when dropped
        Ok(())
    }
}

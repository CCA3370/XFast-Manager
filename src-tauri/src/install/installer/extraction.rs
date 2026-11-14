use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum External7zExtractorKind {
    SevenZipStyle,
    TarStyle,
}

#[derive(Clone, Debug)]
struct External7zExtractorCandidate {
    executable: PathBuf,
    kind: External7zExtractorKind,
}

fn build_external_7z_candidates(password: Option<&str>) -> Vec<External7zExtractorCandidate> {
    let mut candidates = Vec::<External7zExtractorCandidate>::new();

    if let Ok(custom) = std::env::var("XFAST_7Z_PATH") {
        if !custom.trim().is_empty() {
            candidates.push(External7zExtractorCandidate {
                executable: PathBuf::from(custom),
                kind: External7zExtractorKind::SevenZipStyle,
            });
        }
    }

    #[cfg(windows)]
    {
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            candidates.push(External7zExtractorCandidate {
                executable: PathBuf::from(program_files).join("7-Zip").join("7z.exe"),
                kind: External7zExtractorKind::SevenZipStyle,
            });
        }
        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            candidates.push(External7zExtractorCandidate {
                executable: PathBuf::from(program_files_x86)
                    .join("7-Zip")
                    .join("7z.exe"),
                kind: External7zExtractorKind::SevenZipStyle,
            });
        }
        if password.is_none() {
            if let Ok(system_root) = std::env::var("SystemRoot") {
                candidates.push(External7zExtractorCandidate {
                    executable: PathBuf::from(&system_root).join("System32").join("tar.exe"),
                    kind: External7zExtractorKind::TarStyle,
                });
                candidates.push(External7zExtractorCandidate {
                    executable: PathBuf::from(system_root)
                        .join("System32")
                        .join("bsdtar.exe"),
                    kind: External7zExtractorKind::TarStyle,
                });
            }
        }
    }

    candidates.push(External7zExtractorCandidate {
        executable: PathBuf::from("7z"),
        kind: External7zExtractorKind::SevenZipStyle,
    });
    candidates.push(External7zExtractorCandidate {
        executable: PathBuf::from("7za"),
        kind: External7zExtractorKind::SevenZipStyle,
    });
    candidates.push(External7zExtractorCandidate {
        executable: PathBuf::from("7zr"),
        kind: External7zExtractorKind::SevenZipStyle,
    });

    if password.is_none() {
        candidates.push(External7zExtractorCandidate {
            executable: PathBuf::from("tar"),
            kind: External7zExtractorKind::TarStyle,
        });
        candidates.push(External7zExtractorCandidate {
            executable: PathBuf::from("bsdtar"),
            kind: External7zExtractorKind::TarStyle,
        });
    }

    candidates
}

impl Installer {
    /// Copy a directory recursively with progress tracking
    /// Uses parallel processing for better performance on multi-core systems
    pub(super) fn copy_directory_with_progress(
        &self,
        source: &Path,
        target: &Path,
        ctx: &ProgressContext,
    ) -> Result<()> {
        if let Some(plan) = ctx
            .directory_plan
            .as_ref()
            .filter(|plan| plan.base.source == source)
        {
            let hashes = self.copy_planned_directory(&plan.base, target, ctx, plan.verify)?;
            *ctx.inline_hashes.lock().unwrap() = hashes;
            return Ok(());
        }
        self.copy_directory_internal(source, target, Some(ctx))
    }

    pub(super) fn copy_planned_directory(
        &self,
        plan: &DirectoryPlan,
        target: &Path,
        ctx: &ProgressContext,
        verify: bool,
    ) -> Result<HashMap<String, FileHash>> {
        plan.copy(target, &self.task_control, verify, |path, bytes| {
            ctx.add_bytes(bytes);
            ctx.emit_progress(
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned()),
                InstallPhase::Installing,
            );
        })
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
            .filter_entry(|entry| {
                entry.path().strip_prefix(source).map_or(true, |relative| {
                    !crate::package_artifacts::is_ignored_package_artifact_path(relative)
                })
            })
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
                let mut source_file = open_file_for_read_with_retry(source_path)
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
    ) -> Result<()> {
        let extract_start = Instant::now();
        let indexed_zip = ctx
            .zip_package
            .as_ref()
            .filter(|zip| zip.source_path == archive);
        let format = indexed_zip
            .map(|_| crate::archive_input::ArchiveFormat::Zip)
            .or_else(|| crate::archive_input::detect_archive_format(archive))
            .ok_or_else(|| anyhow::anyhow!("Unsupported archive format: {}", archive.display()))?;
        if format == crate::archive_input::ArchiveFormat::Zip {
            let standalone;
            let zip = match indexed_zip {
                Some(zip) => zip,
                None => {
                    standalone = ZipPackage::open(archive, None)?;
                    &standalone
                }
            };
            self.extract_zip_with_progress(zip, target, internal_root, ctx, password)?;
            crate::log_debug!(
                &format!(
                    "[TIMING] ZIP extraction completed in {:.2}ms",
                    extract_start.elapsed().as_secs_f64() * 1000.0
                ),
                "installer_timing"
            );
            return Ok(());
        }
        crate::log_debug!(
            &format!(
                "[TIMING] Archive extraction started: {} format",
                format.as_str()
            ),
            "installer_timing"
        );

        match format {
            crate::archive_input::ArchiveFormat::Zip => unreachable!(),
            crate::archive_input::ArchiveFormat::SevenZ => {
                let standalone;
                let package = if let Some(package) = ctx
                    .sevenz_package
                    .as_deref()
                    .filter(|package| package.source_path.as_deref() == Some(archive))
                {
                    package
                } else {
                    standalone = SevenzPackage::open(archive, password)?;
                    &standalone
                };
                let primary_result = self.extract_7z_with_progress(
                    package,
                    target,
                    internal_root,
                    ctx,
                    Some(archive),
                );
                if let Err(primary_err) = primary_result {
                    if Self::should_try_external_7z_fallback(&primary_err) {
                        logger::log_error(
                            &format!(
                                "Built-in 7z extractor hit error, trying external 7z fallback: {}",
                                primary_err
                            ),
                            Some("installer"),
                        );
                        self.extract_7z_with_external_fallback(
                            package
                                .prepared
                                .as_ref()
                                .expect("file-backed package")
                                .read_path(),
                            target,
                            internal_root,
                            ctx,
                            password,
                        )
                        .map_err(|fallback_err| {
                            anyhow::anyhow!(
                                "Failed to extract 7z: {} (fallback failed: {})",
                                primary_err,
                                fallback_err
                            )
                        })?;
                    } else {
                        return Err(primary_err);
                    }
                }
            }
            crate::archive_input::ArchiveFormat::Rar => {
                let prepared = crate::archive_input::prepare_archive_for_read(archive, format)?;
                self.extract_rar_with_progress(
                    prepared.read_path(),
                    target,
                    internal_root,
                    ctx,
                    password,
                )?
            }
        }

        crate::log_debug!(
            &format!(
                "[TIMING] Archive extraction completed in {:.2}ms: {} format",
                extract_start.elapsed().as_secs_f64() * 1000.0,
                format.as_str()
            ),
            "installer_timing"
        );

        Ok(())
    }

    /// Extract ZIP archive with progress tracking
    /// Supports password-protected ZIP files (both ZipCrypto and AES encryption)
    /// Uses parallel extraction for better performance on multi-core systems
    pub(super) fn extract_zip_with_progress(
        &self,
        zip: &ZipPackage,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        use rayon::prelude::*;

        let entries = zip_install_entries(zip, internal_root);
        let files: Vec<_> = entries
            .iter()
            .filter(|entry| !entry.source.is_directory)
            .collect();
        let directories: std::collections::HashSet<_> = entries
            .iter()
            .filter_map(|entry| {
                if entry.source.is_directory {
                    Some(entry.destination.as_path())
                } else {
                    entry.destination.parent()
                }
            })
            .collect();
        for directory in directories {
            fs::create_dir_all(target.join(directory))?;
        }

        let write_file = |entry: &ZipInstallEntry<'_>,
                          archive: &mut zip::ZipArchive<crate::zip_package::ZipReader>,
                          buffer: &mut [u8]|
         -> Result<()> {
            check_archive_cancelled(&self.task_control)?;
            let index = entry.source.index;
            let mut file = if entry.source.encrypted {
                let password = password.context("Password required for encrypted ZIP entry")?;
                archive.by_index_decrypt(index, password.as_bytes())?
            } else {
                archive.by_index(index)?
            };
            let outpath = target.join(&entry.destination);
            let mut output = tempfile::NamedTempFile::new_in(
                outpath.parent().context("Missing ZIP output parent")?,
            )?;
            let display = entry
                .destination
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            // Reading through EOF runs the decoder's CRC or authentication check.
            let bytes = copy_with_buffer(&mut file, &mut output, buffer, |bytes| {
                check_archive_cancelled(&self.task_control)?;
                ctx.add_bytes(bytes.len() as u64);
                ctx.emit_progress(display.clone(), InstallPhase::Installing);
                Ok(())
            })?;
            anyhow::ensure!(
                bytes == entry.source.size,
                "ZIP entry size changed: {}",
                entry.source.name
            );
            #[cfg(unix)]
            if let Some(mode) = entry.source.unix_mode {
                use std::os::unix::fs::PermissionsExt;
                output
                    .as_file()
                    .set_permissions(fs::Permissions::from_mode(mode))?;
            }
            check_archive_cancelled(&self.task_control)?;
            if outpath.is_file() {
                let _ = remove_readonly_attribute(&outpath);
            }
            output.persist(&outpath)?;
            Ok(())
        };

        // Different archive names can normalize to the same path. In that case
        // write in archive order so workers never truncate the same file concurrently.
        let mut destinations = std::collections::HashSet::with_capacity(files.len());
        let unique = files.iter().all(|entry| {
            let name = entry.destination.to_string_lossy();
            let key = if cfg!(any(windows, target_os = "macos")) {
                name.to_lowercase()
            } else {
                name.into_owned()
            };
            destinations.insert(key)
        });
        if let Some(pool) =
            archive_worker_pool(&zip.source_path, target).filter(|_| unique && files.len() > 1)
        {
            pool.install(|| {
                files.par_iter().try_for_each_init(
                    || (zip.reader(), vec![0; STREAM_BUFFER_SIZE]),
                    |(archive, buffer), entry| write_file(entry, archive, buffer),
                )
            })?;
        } else {
            let mut archive = zip.reader();
            let mut buffer = vec![0; STREAM_BUFFER_SIZE];
            for entry in files {
                write_file(entry, &mut archive, &mut buffer)?;
            }
        }
        zip.check_source()?;
        Ok(())
    }

    /// Extract 7z archive with progress tracking
    /// Extracts directly to target directory with inline SHA256 hash computation
    pub(super) fn extract_7z_with_progress(
        &self,
        package: &SevenzPackage,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        source_path: Option<&Path>,
    ) -> Result<()> {
        use rayon::prelude::*;
        use sha2::{Digest, Sha256};
        package.check_source()?;
        fs::create_dir_all(target)?;
        let entries = sevenz_install_entries(package, internal_root);
        anyhow::ensure!(!entries.is_empty(), "No files match the selected 7z root");
        let mut files = HashMap::new();
        for (index, relative) in entries {
            let entry = &package.archive.files[index];
            if entry.is_directory() {
                fs::create_dir_all(target.join(relative))?;
            } else {
                if let Some(parent) = relative.parent() {
                    fs::create_dir_all(target.join(parent))?;
                }
                files.insert(index, relative);
            }
        }
        let groups = package.groups(files.keys().copied());
        let compute_hashes = ctx.inline_hash_collection_enabled.load(Ordering::SeqCst);
        let write_group = |block, indices: &[usize], discard: &mut [u8], buffer: &mut [u8]| {
            package.read_group(
                block,
                indices,
                discard,
                || check_archive_cancelled(&self.task_control),
                |index, entry, input| {
                    let relative = &files[&index];
                    let destination = target.join(relative);
                    let mut output = tempfile::NamedTempFile::new_in(
                        destination.parent().expect("target parent"),
                    )?;
                    let mut hasher = compute_hashes.then(Sha256::new);
                    let display = relative
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned());
                    let written = copy_with_buffer(input, &mut output, buffer, |bytes| {
                        check_archive_cancelled(&self.task_control)?;
                        if let Some(hasher) = &mut hasher {
                            hasher.update(bytes);
                        }
                        ctx.add_bytes(bytes.len() as u64);
                        ctx.emit_progress(display.clone(), InstallPhase::Installing);
                        Ok(())
                    })?;
                    if written != entry.size {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Truncated 7z member",
                        ));
                    }
                    check_archive_cancelled(&self.task_control)?;
                    if destination.is_file() {
                        let _ = remove_readonly_attribute(&destination);
                    }
                    output.persist(&destination).map_err(|error| error.error)?;
                    if let Some(hasher) = hasher {
                        let path = relative.to_string_lossy().replace('\\', "/");
                        ctx.inline_hashes.lock().unwrap().insert(
                            path.clone(),
                            crate::models::FileHash {
                                path,
                                hash: format!("{:x}", hasher.finalize()),
                                algorithm: crate::models::HashAlgorithm::Sha256,
                            },
                        );
                    }
                    Ok(())
                },
            )
        };
        let groups: Vec<_> = groups.into_iter().collect();
        let mut destinations = std::collections::HashSet::with_capacity(files.len());
        let unique = files.values().all(|path| {
            let name = path.to_string_lossy();
            destinations.insert(if cfg!(any(windows, target_os = "macos")) {
                name.to_lowercase()
            } else {
                name.into_owned()
            })
        });
        let pool = (unique && groups.len() > 1)
            .then(|| source_path.and_then(|source| archive_worker_pool(source, target)))
            .flatten();
        if let Some(pool) = pool {
            pool.install(|| {
                groups.par_iter().try_for_each_init(
                    || (vec![0; STREAM_BUFFER_SIZE], vec![0; STREAM_BUFFER_SIZE]),
                    |(discard, buffer), (block, indices)| {
                        write_group(*block, indices, discard, buffer)
                    },
                )
            })?;
        } else {
            let mut discard = vec![0; STREAM_BUFFER_SIZE];
            let mut buffer = vec![0; STREAM_BUFFER_SIZE];
            // Empty files follow streamed blocks, matching archive extraction order.
            for (block, indices) in groups
                .iter()
                .filter(|(block, _)| block.is_some())
                .chain(groups.iter().filter(|(block, _)| block.is_none()))
            {
                write_group(*block, indices, &mut discard, &mut buffer)?;
            }
        }
        package.check_source()?;
        Ok(())
    }

    fn should_try_external_7z_fallback(error: &anyhow::Error) -> bool {
        use sevenz_rust2::Error;
        error.downcast_ref::<Error>().is_some_and(|error| {
            matches!(
                error,
                Error::UnsupportedCompressionMethod(_)
                    | Error::Unsupported(_)
                    | Error::ExternalUnsupported
            )
        })
    }

    fn extract_7z_with_external_fallback(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        let temp_dir = tempfile::Builder::new()
            .prefix("xfastmanager_7z_fallback_")
            .tempdir()
            .context("Failed to create temp directory for 7z fallback")?;

        let extracted_root = temp_dir.path();
        self.run_external_7z_extract(archive, extracted_root, password)?;

        let source_path = if let Some(root) = internal_root {
            let normalized = root.replace('\\', "/").trim_matches('/').to_string();
            let candidate =
                extracted_root.join(normalized.replace('/', std::path::MAIN_SEPARATOR_STR));
            if !candidate.exists() {
                return Err(anyhow::anyhow!(
                    "7z fallback extracted successfully, but internal root not found: {}",
                    root
                ));
            }
            candidate
        } else {
            extracted_root.to_path_buf()
        };

        if source_path.is_file() {
            let final_target = if target.exists() && target.is_dir() {
                let file_name = source_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Source file has no filename"))?;
                target.join(file_name)
            } else {
                target.to_path_buf()
            };

            if let Some(parent) = final_target.parent() {
                fs::create_dir_all(parent).context(format!(
                    "Failed to create target directory for 7z fallback file copy: {:?}",
                    parent
                ))?;
            }

            let file_size = fs::metadata(&source_path)?.len();
            let display_name = final_target
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            let mut source_file = open_file_for_read_with_retry(&source_path).context(format!(
                "Failed to open source file for 7z fallback copy: {:?}",
                source_path
            ))?;
            let mut target_file = fs::File::create(&final_target).context(format!(
                "Failed to create target file for 7z fallback copy: {:?}",
                final_target
            ))?;
            copy_file_optimized(&mut source_file, &mut target_file)?;

            let _ = remove_readonly_attribute(&final_target);
            ctx.add_bytes(file_size);
            ctx.emit_progress(Some(display_name), InstallPhase::Installing);
        } else {
            self.copy_directory_with_progress(&source_path, target, ctx)?;
        }

        logger::log_info(
            "External 7z fallback extraction completed",
            Some("installer"),
        );
        Ok(())
    }

    fn run_external_7z_extract(
        &self,
        archive: &Path,
        out_dir: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        std::fs::create_dir_all(out_dir)?;

        let mut attempted = Vec::<String>::new();
        let mut failed = Vec::<String>::new();

        for candidate in build_external_7z_candidates(password) {
            let key = candidate.executable.to_string_lossy().to_string();
            if attempted.iter().any(|v| v == &key) {
                continue;
            }
            attempted.push(key.clone());

            let mut cmd = std::process::Command::new(&candidate.executable);
            match candidate.kind {
                External7zExtractorKind::SevenZipStyle => {
                    cmd.arg("x")
                        .arg(archive.as_os_str())
                        .arg("-y")
                        .arg("-aoa")
                        .arg(format!("-o{}", out_dir.display()))
                        .arg("-bb0")
                        .arg("-bso0")
                        .arg("-bsp0");

                    if let Some(pwd) = password {
                        cmd.arg(format!("-p{}", pwd));
                    } else {
                        // Prevent interactive password prompt in non-interactive execution.
                        cmd.arg("-p");
                    }
                }
                External7zExtractorKind::TarStyle => {
                    cmd.arg("-xf")
                        .arg(archive.as_os_str())
                        .arg("-C")
                        .arg(out_dir.as_os_str());
                }
            }

            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        logger::log_info(
                            &format!("7z fallback succeeded with executable: {}", key),
                            Some("installer"),
                        );
                        return Ok(());
                    }

                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    failed.push(format!(
                        "{} -> exit {} stderr='{}' stdout='{}'",
                        key,
                        output.status,
                        truncate_for_log(stderr.as_ref(), 240),
                        truncate_for_log(stdout.as_ref(), 240)
                    ));
                }
                Err(err) => {
                    failed.push(format!("{} -> {}", key, err));
                }
            }
        }

        Err(anyhow::anyhow!(
            "No usable external 7z extractor succeeded. Attempts: {}",
            failed.join(" | ")
        ))
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
            if crate::package_artifacts::is_ignored_package_artifact_path(relative) {
                continue;
            }
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
    pub(super) fn extract_rar_with_progress(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        fs::create_dir_all(target)?;
        let mut reader = crate::rar_stream::RarStream::open(archive, password)?;
        let mut files = 0;
        while let Some(entry) = reader.next()? {
            check_archive_cancelled(&self.task_control)?;
            let relative = archive_entry_destination(&entry.name, internal_root);
            let Some(relative) = relative.filter(|_| !entry.directory && !entry.redirected) else {
                reader.skip()?;
                continue;
            };
            let path = target.join(&relative);
            let parent = path.parent().context("Missing RAR output parent")?;
            fs::create_dir_all(parent)?;
            let mut output = tempfile::NamedTempFile::new_in(parent)?;
            let mut remaining = entry.size;
            let display = relative
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            use sha2::{Digest, Sha256};
            use std::io::Write;
            let mut hasher = ctx
                .inline_hash_collection_enabled
                .load(Ordering::SeqCst)
                .then(Sha256::new);
            reader.read(|bytes| {
                check_archive_cancelled(&self.task_control)?;
                remaining = remaining
                    .checked_sub(bytes.len() as u64)
                    .ok_or_else(|| std::io::Error::other("RAR member exceeds its declared size"))?;
                output.write_all(bytes)?;
                if let Some(hasher) = &mut hasher {
                    hasher.update(bytes);
                }
                ctx.add_bytes(bytes.len() as u64);
                ctx.emit_progress(display.clone(), InstallPhase::Installing);
                Ok(())
            })?;
            anyhow::ensure!(remaining == 0, "RAR member was truncated");
            if path.is_file() {
                let _ = remove_readonly_attribute(&path);
            }
            output.persist(path)?;
            if let Some(hasher) = hasher {
                let path = relative.to_string_lossy().replace('\\', "/");
                ctx.inline_hashes.lock().unwrap().insert(
                    path.clone(),
                    crate::models::FileHash {
                        path,
                        hash: format!("{:x}", hasher.finalize()),
                        algorithm: crate::models::HashAlgorithm::Sha256,
                    },
                );
            }
            files += 1;
        }
        anyhow::ensure!(files > 0, "No files match the selected RAR root");
        Ok(())
    }
}

fn truncate_for_log(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    format!("{}...", truncated)
}

#[cfg(test)]
mod tests {
    use super::{build_external_7z_candidates, External7zExtractorKind, Installer};

    #[test]
    fn external_7z_candidates_include_tar_when_no_password() {
        let candidates = build_external_7z_candidates(None);
        assert!(candidates
            .iter()
            .any(|candidate| candidate.kind == External7zExtractorKind::TarStyle));
    }

    #[test]
    fn external_7z_candidates_skip_tar_when_password_provided() {
        let candidates = build_external_7z_candidates(Some("secret"));
        assert!(candidates
            .iter()
            .all(|candidate| candidate.kind != External7zExtractorKind::TarStyle));
    }

    #[test]
    fn unsupported_codecs_trigger_external_7z_fallback() {
        let error = sevenz_rust2::Error::UnsupportedCompressionMethod("fixture".into()).into();
        assert!(Installer::should_try_external_7z_fallback(&error));
        let bad_data = anyhow::anyhow!("Checksum verification failed");
        assert!(!Installer::should_try_external_7z_fallback(&bad_data));
    }
}

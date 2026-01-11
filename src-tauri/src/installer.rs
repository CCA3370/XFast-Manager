use anyhow::{Context, Result};
use glob::Pattern;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

use crate::logger;
use crate::logger::{tr, LogMsg};
use crate::models::{AddonType, InstallPhase, InstallProgress, InstallResult, InstallTask, TaskResult};

/// Maximum allowed extraction size (20 GB) - archives larger than this will show a warning
pub const MAX_EXTRACTION_SIZE: u64 = 20 * 1024 * 1024 * 1024;

/// Maximum compression ratio to detect zip bombs (100:1)
pub const MAX_COMPRESSION_RATIO: u64 = 100;

/// Check if a filename matches any of the given glob patterns
fn matches_any_pattern(filename: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        Pattern::new(pattern)
            .map(|p| p.matches(filename))
            .unwrap_or(false)
    })
}

/// Sanitize a file path to prevent path traversal attacks
/// Returns None if the path is unsafe (contains `..` or is absolute)
pub fn sanitize_path(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(c) => result.push(c),
            Component::CurDir => {} // Skip "."
            Component::ParentDir => return None, // Reject ".."
            Component::Prefix(_) | Component::RootDir => return None, // Reject absolute paths
        }
    }
    if result.as_os_str().is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Optimized file copy with buffering for better performance
/// Uses a larger buffer (4MB) for faster I/O operations
fn copy_file_optimized<R: std::io::Read + ?Sized, W: std::io::Write>(
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<u64> {
    const BUFFER_SIZE: usize = 4 * 1024 * 1024; // 4MB buffer for better performance
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut total_bytes = 0u64;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read])?;
        total_bytes += bytes_read as u64;
    }

    Ok(total_bytes)
}

/// Remove read-only attribute from a file (Windows only)
#[cfg(target_os = "windows")]
fn remove_readonly_attribute(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();

    // Check if file is read-only
    if permissions.readonly() {
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)
            .context(format!("Failed to remove read-only attribute from: {:?}", path))?;
    }
    Ok(())
}

/// Remove read-only attribute from a file (non-Windows platforms)
#[cfg(not(target_os = "windows"))]
fn remove_readonly_attribute(_path: &Path) -> Result<()> {
    // On Unix-like systems, we handle permissions differently
    Ok(())
}

/// Robustly remove a directory and all its contents, handling read-only files
fn remove_dir_all_robust(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    // First pass: remove read-only attributes from all files
    for entry in walkdir::WalkDir::new(path).follow_links(false) {
        if let Ok(entry) = entry {
            let entry_path = entry.path();
            if entry_path.is_file() {
                // Try to remove read-only attribute, but don't fail if it doesn't work
                let _ = remove_readonly_attribute(entry_path);
            }
        }
    }

    // Second pass: try to delete the directory
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // If deletion still fails, provide detailed error information
            let err_msg = format!(
                "Failed to delete directory: {:?}\nError: {}\n\
                This may be caused by:\n\
                - Files being used by another program (X-Plane, file explorer, antivirus)\n\
                - Insufficient permissions\n\
                - System files or protected folders\n\
                Please close any programs that might be using these files and try again.",
                path, e
            );
            Err(anyhow::anyhow!(err_msg))
        }
    }
}

/// Directory statistics for backup verification
struct DirectoryInfo {
    file_count: u64,
    total_size: u64,
}

/// Backup data for Aircraft overwrites
struct AircraftBackup {
    temp_dir: PathBuf,
    liveries_path: Option<PathBuf>,
    pref_files: Vec<(String, PathBuf)>, // (filename, temp_path)
    // For verification
    original_liveries_info: Option<DirectoryInfo>,
    original_pref_sizes: Vec<(String, u64)>, // (filename, original_size)
}

/// Progress tracking context
struct ProgressContext {
    app_handle: AppHandle,
    total_bytes: Arc<AtomicU64>,
    processed_bytes: Arc<AtomicU64>,
    last_emit: Arc<Mutex<Instant>>,
    current_task_index: usize,
    total_tasks: usize,
    current_task_name: String,
}

impl ProgressContext {
    fn new(app_handle: AppHandle, total_tasks: usize) -> Self {
        Self {
            app_handle,
            total_bytes: Arc::new(AtomicU64::new(0)),
            processed_bytes: Arc::new(AtomicU64::new(0)),
            last_emit: Arc::new(Mutex::new(Instant::now())),
            current_task_index: 0,
            total_tasks,
            current_task_name: String::new(),
        }
    }

    fn set_total_bytes(&self, total: u64) {
        self.total_bytes.store(total, Ordering::SeqCst);
    }

    fn add_bytes(&self, bytes: u64) {
        self.processed_bytes.fetch_add(bytes, Ordering::SeqCst);
    }

    fn emit_progress(&self, current_file: Option<String>, phase: InstallPhase) {
        // Throttle: emit at most every 16ms (60fps for smooth animation)
        let mut last = self.last_emit.lock().unwrap();
        let now = Instant::now();
        if now.duration_since(*last).as_millis() < 16 {
            return;
        }
        *last = now;
        drop(last);

        let total = self.total_bytes.load(Ordering::SeqCst);
        let processed = self.processed_bytes.load(Ordering::SeqCst);
        let percentage = if total > 0 {
            (processed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let progress = InstallProgress {
            percentage,
            total_bytes: total,
            processed_bytes: processed,
            current_task_index: self.current_task_index,
            total_tasks: self.total_tasks,
            current_task_name: self.current_task_name.clone(),
            current_file,
            phase,
        };

        let _ = self.app_handle.emit("install-progress", &progress);
    }

    fn emit_final(&self, phase: InstallPhase) {
        let total = self.total_bytes.load(Ordering::SeqCst);
        let processed = self.processed_bytes.load(Ordering::SeqCst);
        let percentage = if total > 0 {
            (processed as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        let progress = InstallProgress {
            percentage,
            total_bytes: total,
            processed_bytes: processed,
            current_task_index: self.current_task_index,
            total_tasks: self.total_tasks,
            current_task_name: self.current_task_name.clone(),
            current_file: None,
            phase,
        };

        let _ = self.app_handle.emit("install-progress", &progress);
    }
}

pub struct Installer {
    app_handle: AppHandle,
}

impl Installer {
    pub fn new(app_handle: AppHandle) -> Self {
        Installer { app_handle }
    }

    /// Install a list of tasks with progress reporting
    pub fn install(&self, tasks: Vec<InstallTask>) -> Result<InstallResult> {
        logger::log_info(
            &format!("{}: {} task(s)", tr(LogMsg::InstallationStarted), tasks.len()),
            Some("installer"),
        );

        let mut ctx = ProgressContext::new(self.app_handle.clone(), tasks.len());
        let mut task_results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;

        // Phase 1: Calculate total size
        ctx.emit_progress(None, InstallPhase::Calculating);
        let total_size = self.calculate_total_size(&tasks)?;
        ctx.set_total_bytes(total_size);

        // Phase 2: Install each task
        for (index, task) in tasks.iter().enumerate() {
            ctx.current_task_index = index;
            ctx.current_task_name = task.display_name.clone();
            ctx.emit_progress(None, InstallPhase::Installing);

            logger::log_info(
                &format!("{}: {} -> {}", tr(LogMsg::Installing), task.display_name, task.target_path),
                Some("installer"),
            );

            match self.install_task_with_progress(task, &ctx) {
                Ok(_) => {
                    successful += 1;
                    task_results.push(TaskResult {
                        task_id: task.id.clone(),
                        task_name: task.display_name.clone(),
                        success: true,
                        error_message: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    let error_msg = format!("{}", e);
                    logger::log_error(
                        &format!("{} {}: {}", tr(LogMsg::InstallationFailed), task.display_name, error_msg),
                        Some("installer"),
                    );
                    task_results.push(TaskResult {
                        task_id: task.id.clone(),
                        task_name: task.display_name.clone(),
                        success: false,
                        error_message: Some(error_msg),
                    });
                }
            }
        }

        // Phase 3: Finalize
        ctx.emit_final(InstallPhase::Finalizing);
        logger::log_info(&tr(LogMsg::InstallationCompleted), Some("installer"));

        Ok(InstallResult {
            total_tasks: tasks.len(),
            successful_tasks: successful,
            failed_tasks: failed,
            task_results,
        })
    }

    /// Calculate total size of all tasks for progress tracking
    fn calculate_total_size(&self, tasks: &[InstallTask]) -> Result<u64> {
        let mut total = 0u64;
        for task in tasks {
            let source = Path::new(&task.source_path);
            if source.is_dir() {
                total += self.get_directory_size(source)?;
            } else if source.is_file() {
                total += self.get_archive_size(source, task.archive_internal_root.as_deref())?;
            }
        }
        Ok(total)
    }

    /// Get total size of files in a directory
    fn get_directory_size(&self, dir: &Path) -> Result<u64> {
        let mut size = 0u64;
        for entry in walkdir::WalkDir::new(dir).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() {
                size += entry.metadata()?.len();
            }
        }
        Ok(size)
    }

    /// Get uncompressed size of archive
    fn get_archive_size(&self, archive: &Path, internal_root: Option<&str>) -> Result<u64> {
        let ext = archive.extension().and_then(|s| s.to_str());
        match ext {
            Some("zip") => self.get_zip_size(archive, internal_root),
            Some("7z") => self.get_7z_size(archive),
            Some("rar") => self.get_rar_size(archive),
            _ => Ok(0),
        }
    }

    /// Get uncompressed size of ZIP archive
    fn get_zip_size(&self, archive: &Path, internal_root: Option<&str>) -> Result<u64> {
        use zip::ZipArchive;
        let file = fs::File::open(archive)?;
        let mut archive_reader = ZipArchive::new(file)?;
        let prefix = internal_root.map(|s| s.replace('\\', "/"));

        let mut total = 0u64;
        for i in 0..archive_reader.len() {
            if let Ok(file) = archive_reader.by_index_raw(i) {
                let name = file.name().replace('\\', "/");
                if let Some(ref p) = prefix {
                    if !name.starts_with(p) {
                        continue;
                    }
                }
                total += file.size();
            }
        }
        Ok(total)
    }

    /// Get uncompressed size of 7z archive (estimate from file size)
    fn get_7z_size(&self, archive: &Path) -> Result<u64> {
        // sevenz-rust2 doesn't have easy size query, use compressed size * 3 as estimate
        let meta = fs::metadata(archive)?;
        Ok(meta.len() * 3)
    }

    /// Get uncompressed size of RAR archive
    fn get_rar_size(&self, archive: &Path) -> Result<u64> {
        let arch = unrar::Archive::new(archive)
            .open_for_listing()
            .map_err(|e| anyhow::anyhow!("Failed to open RAR for size query: {:?}", e))?;

        let mut total = 0u64;
        for entry in arch {
            if let Ok(e) = entry {
                total += e.unpacked_size;
            }
        }
        Ok(total)
    }

    /// Install a single task with progress tracking
    fn install_task_with_progress(&self, task: &InstallTask, ctx: &ProgressContext) -> Result<()> {
        let source = Path::new(&task.source_path);
        let target = Path::new(&task.target_path);
        let password = task.password.as_deref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create target directory: {:?}", parent))?;
        }

        // Handle installation based on overwrite mode
        // should_overwrite = false (default): Clean install - delete old folder first
        // should_overwrite = true: Direct overwrite - keep existing files and overwrite
        if !task.should_overwrite && target.exists() {
            // Clean install mode: delete old folder first
            self.handle_clean_install_with_progress(task, source, target, ctx, password)?;
        } else {
            // Direct overwrite mode: just install/extract files directly
            self.install_content_with_progress(source, target, task.archive_internal_root.as_deref(), ctx, password)?;
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
        if source.is_dir() {
            self.copy_directory_with_progress(source, target, ctx)?;
        } else if source.is_file() {
            self.extract_archive_with_progress(source, target, internal_root, ctx, password)?;
        } else {
            return Err(anyhow::anyhow!("Source path is neither file nor directory"));
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
                self.handle_aircraft_clean_install_with_progress(
                    source,
                    target,
                    task.archive_internal_root.as_deref(),
                    ctx,
                    password,
                    task.backup_liveries,
                    task.backup_config_files,
                    &task.config_file_patterns,
                )?;
            }
            AddonType::Navdata => {
                // For Navdata: DON'T delete Custom Data folder!
                // Just extract and overwrite individual files (same as direct overwrite)
                self.install_content_with_progress(source, target, task.archive_internal_root.as_deref(), ctx, password)?;
            }
            _ => {
                // For other types: delete and reinstall using robust removal
                if target.exists() {
                    remove_dir_all_robust(target)
                        .context(format!("Failed to delete existing folder: {:?}", target))?;
                }
                self.install_content_with_progress(source, target, task.archive_internal_root.as_deref(), ctx, password)?;
            }
        }
        Ok(())
    }

    /// Aircraft clean install with progress tracking
    fn handle_aircraft_clean_install_with_progress(
        &self,
        source: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
        backup_liveries: bool,
        backup_config_files: bool,
        config_patterns: &[String],
    ) -> Result<()> {
        // Step 1: Create backup of important files
        let backup = self.backup_aircraft_data(target, backup_liveries, backup_config_files, config_patterns)?;

        // Step 2: VERIFY backup is complete and valid BEFORE deleting
        if let Some(ref backup_data) = backup {
            self.verify_backup(backup_data)
                .context("Backup verification failed - aborting to protect your data")?;
        }

        // Step 3: Delete target folder (only after backup is verified)
        if target.exists() {
            remove_dir_all_robust(target)
                .context(format!("Failed to delete existing aircraft folder: {:?}", target))?;
        }

        // Step 4: Install new content with progress
        let install_result = self.install_content_with_progress(source, target, internal_root, ctx, password);

        // Step 5: Restore backup and verify
        let restore_verified = if let Some(ref backup_data) = backup {
            match self.restore_aircraft_backup(backup_data, target) {
                Ok(()) => {
                    match self.verify_restore(backup_data, target) {
                        Ok(()) => true,
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Restore verification failed: {}. Your backup is preserved at: {:?}.",
                                e, backup_data.temp_dir
                            ));
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to restore backup: {}. Your backup is preserved at: {:?}.",
                        e, backup_data.temp_dir
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
    ) -> Result<Option<AircraftBackup>> {
        if !target.exists() {
            return Ok(None);
        }

        // Create temp directory for backup
        let temp_dir = std::env::temp_dir().join(format!("xfastinstall_backup_{}", uuid::Uuid::new_v4()));
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
                // Record original info for verification
                let original_info = self.get_directory_info(&liveries_src)?;
                backup.original_liveries_info = Some(original_info);

                let liveries_dst = temp_dir.join("liveries");
                self.copy_directory(&liveries_src, &liveries_dst)
                    .context("Failed to backup liveries folder")?;
                backup.liveries_path = Some(liveries_dst);
            }
        }

        // Backup config files from root directory only if enabled
        if backup_config_files && !config_patterns.is_empty() {
            for entry in fs::read_dir(target)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if matches_any_pattern(name, config_patterns) {
                            let original_size = fs::metadata(&path)?.len();
                            let backup_path = temp_dir.join(name);
                            fs::copy(&path, &backup_path)
                                .context(format!("Failed to backup {}", name))?;
                            backup.pref_files.push((name.to_string(), backup_path.clone()));
                            backup.original_pref_sizes.push((name.to_string(), original_size));
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

        Ok(DirectoryInfo { file_count, total_size })
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
    fn restore_aircraft_backup(&self, backup: &AircraftBackup, target: &Path) -> Result<()> {
        // Restore liveries folder (skip existing - don't overwrite new content)
        if let Some(ref liveries_backup) = backup.liveries_path {
            let liveries_target = target.join("liveries");

            if liveries_target.exists() {
                // Merge: copy only files that don't exist in new content
                self.merge_directory_skip_existing(liveries_backup, &liveries_target)?;
            } else {
                // No new liveries folder, restore entire backup
                self.copy_directory(liveries_backup, &liveries_target)?;
            }
        }

        // Restore *_prefs.txt files (always overwrite - restore user preferences)
        for (filename, backup_path) in &backup.pref_files {
            let target_path = target.join(filename);
            fs::copy(backup_path, &target_path)
                .context(format!("Failed to restore pref file: {}", filename))?;
        }

        Ok(())
    }

    /// Copy directory contents, skipping files that already exist in target
    fn merge_directory_skip_existing(&self, source: &Path, target: &Path) -> Result<()> {
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
                self.merge_directory_skip_existing(&source_path, &target_path)?;
            } else {
                // Only copy if target doesn't exist (skip existing)
                if !target_path.exists() {
                    fs::copy(&source_path, &target_path)?;
                    // Remove read-only attribute from copied file
                    let _ = remove_readonly_attribute(&target_path);
                }
            }
        }

        Ok(())
    }

    /// Copy a directory recursively
    fn copy_directory(&self, source: &Path, target: &Path) -> Result<()> {
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
                self.copy_directory(&source_path, &target_path)?;
            } else {
                fs::copy(&source_path, &target_path)
                    .context(format!("Failed to copy {:?} to {:?}", source_path, target_path))?;

                // Remove read-only attribute from copied file to avoid future deletion issues
                let _ = remove_readonly_attribute(&target_path);
            }
        }

        Ok(())
    }

    /// Copy a directory recursively with progress tracking
    /// Uses parallel processing for better performance on multi-core systems
    fn copy_directory_with_progress(
        &self,
        source: &Path,
        target: &Path,
        ctx: &ProgressContext,
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
                let relative = entry.path().strip_prefix(source)
                    .context("Failed to strip prefix")?;
                let target_path = target.join(relative);
                fs::create_dir_all(&target_path)?;
            }
        }

        // Copy files in parallel using rayon
        use rayon::prelude::*;

        entries.par_iter()
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

                ctx.add_bytes(file_size);
                ctx.emit_progress(Some(file_name), InstallPhase::Installing);

                Ok(())
            })?;

        Ok(())
    }

    /// Extract an archive with progress tracking
    fn extract_archive_with_progress(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        let extension = archive
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

        match extension {
            "zip" => self.extract_zip_with_progress(archive, target, internal_root, ctx, password)?,
            "7z" => self.extract_7z_with_progress(archive, target, internal_root, ctx, password)?,
            "rar" => self.extract_rar_with_progress(archive, target, internal_root, ctx, password)?,
            _ => return Err(anyhow::anyhow!("Unsupported archive format: {}", extension)),
        }

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
    ) -> Result<()> {
        use zip::ZipArchive;
        use std::sync::Arc;

        // Open archive and collect file metadata
        let file = fs::File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;

        let internal_root_normalized = internal_root.map(|s| s.replace('\\', "/"));
        let prefix = internal_root_normalized.as_deref();
        let password_bytes = password.map(|p| p.as_bytes().to_vec());

        // Collect all file entries with their metadata
        let entries: Vec<_> = (0..archive.len())
            .filter_map(|i| {
                let file = archive.by_index(i).ok()?;
                let path = file.enclosed_name()?.to_path_buf();
                let file_path_str = path.to_string_lossy().replace('\\', "/");

                // Check prefix filter
                let relative_path = if let Some(prefix) = prefix {
                    if !file_path_str.starts_with(prefix) {
                        return None;
                    }
                    let stripped = file_path_str
                        .strip_prefix(prefix)
                        .unwrap_or(&file_path_str)
                        .trim_start_matches('/');
                    if stripped.is_empty() {
                        return None;
                    }
                    sanitize_path(Path::new(stripped))?
                } else {
                    sanitize_path(&path)?
                };

                Some((i, relative_path, file.is_dir(), file.encrypted(), file.size()))
            })
            .collect();

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

        entries.par_iter()
            .filter(|(_, _, is_dir, _, _)| !is_dir)
            .try_for_each(|(index, relative_path, _, is_encrypted, _)| -> Result<()> {
                // Each thread opens its own ZipArchive instance
                let file = fs::File::open(&archive_path)?;
                let mut archive = ZipArchive::new(file)?;

                let outpath = target.join(relative_path);

                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }

                // Extract file with or without password
                let file_size = if *is_encrypted {
                    if let Some(ref pwd) = password_bytes.as_ref() {
                        match archive.by_index_decrypt(*index, pwd) {
                            Ok(mut file) => {
                                let size = file.size();
                                let mut outfile = fs::File::create(&outpath)?;
                                copy_file_optimized(&mut file, &mut outfile)?;
                                size
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
                    let size = file.size();
                    let mut outfile = fs::File::create(&outpath)?;
                    copy_file_optimized(&mut file, &mut outfile)?;
                    size
                };

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
                    let file = archive.by_index(*index)?;
                    if let Some(mode) = file.unix_mode() {
                        fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                    }
                }

                Ok(())
            })?;

        Ok(())
    }

    /// Extract 7z archive with progress tracking
    /// Since sevenz-rust2 doesn't provide per-file callbacks, we extract to temp then copy with progress
    fn extract_7z_with_progress(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        // Create secure temp directory using tempfile crate
        let temp_dir = tempfile::Builder::new()
            .prefix("xfastinstall_7z_")
            .tempdir()
            .context("Failed to create secure temp directory")?;

        // Extract with password if provided
        if let Some(pwd) = password {
            let mut reader = sevenz_rust2::SevenZReader::open(archive, sevenz_rust2::Password::from(pwd))
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
                    copy_file_optimized(reader, &mut file)?;
                }
                Ok(true)
            }).map_err(|e| anyhow::anyhow!("Failed to extract 7z with password: {}", e))?;
        } else {
            sevenz_rust2::decompress_file(archive, temp_dir.path())
                .map_err(|e| anyhow::anyhow!("Failed to extract 7z: {}", e))?;
        }

        // Determine source path (with or without internal_root)
        let source_path = if let Some(internal_root) = internal_root {
            let internal_root_normalized = internal_root.replace('\\', "/");
            let path = temp_dir.path().join(&internal_root_normalized);
            if path.exists() && path.is_dir() {
                path
            } else {
                temp_dir.path().to_path_buf()
            }
        } else {
            temp_dir.path().to_path_buf()
        };

        // Copy with progress tracking
        self.copy_directory_with_progress(&source_path, target, ctx)?;

        // TempDir automatically cleans up when dropped
        Ok(())
    }

    /// Extract RAR archive with progress tracking
    /// Similar to 7z - extract to temp then copy with progress
    fn extract_rar_with_progress(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
        // Create secure temp directory using tempfile crate
        let temp_dir = tempfile::Builder::new()
            .prefix("xfastinstall_rar_")
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

        while let Some(header) = arch.read_header()
            .map_err(|e| anyhow::anyhow!("Failed to read RAR header: {:?}", e))?
        {
            arch = if header.entry().is_file() {
                header.extract_with_base(temp_dir.path())
                    .map_err(|e| anyhow::anyhow!("Failed to extract RAR entry: {:?}", e))?
            } else {
                header.skip()
                    .map_err(|e| anyhow::anyhow!("Failed to skip RAR entry: {:?}", e))?
            };
        }

        // Determine source path (with or without internal_root)
        let source_path = if let Some(internal_root) = internal_root {
            let internal_root_normalized = internal_root.replace('\\', "/");
            let path = temp_dir.path().join(&internal_root_normalized);
            if path.exists() && path.is_dir() {
                path
            } else {
                temp_dir.path().to_path_buf()
            }
        } else {
            temp_dir.path().to_path_buf()
        };

        // Copy with progress tracking
        self.copy_directory_with_progress(&source_path, target, ctx)?;

        // TempDir automatically cleans up when dropped
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path_normal() {
        let path = Path::new("folder/subfolder/file.txt");
        let result = sanitize_path(path);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), PathBuf::from("folder/subfolder/file.txt"));
    }

    #[test]
    fn test_sanitize_path_rejects_parent_dir() {
        let path = Path::new("folder/../../../etc/passwd");
        let result = sanitize_path(path);
        assert!(result.is_none(), "Path with .. should be rejected");
    }

    #[test]
    fn test_sanitize_path_rejects_absolute_unix() {
        let path = Path::new("/etc/passwd");
        let result = sanitize_path(path);
        assert!(result.is_none(), "Absolute Unix path should be rejected");
    }

    #[cfg(windows)]
    #[test]
    fn test_sanitize_path_rejects_absolute_windows() {
        let path = Path::new("C:\\Windows\\System32");
        let result = sanitize_path(path);
        assert!(result.is_none(), "Absolute Windows path should be rejected");
    }

    #[test]
    fn test_sanitize_path_handles_current_dir() {
        let path = Path::new("./folder/./file.txt");
        let result = sanitize_path(path);
        assert!(result.is_some());
        // Current dir markers should be skipped
        assert_eq!(result.unwrap(), PathBuf::from("folder/file.txt"));
    }

    #[test]
    fn test_sanitize_path_empty() {
        let path = Path::new("");
        let result = sanitize_path(path);
        assert!(result.is_none(), "Empty path should be rejected");
    }

    #[test]
    fn test_sanitize_path_only_parent() {
        let path = Path::new("..");
        let result = sanitize_path(path);
        assert!(result.is_none(), "Only parent dir should be rejected");
    }

    #[test]
    fn test_zip_bomb_constants() {
        // Verify constants are reasonable
        assert_eq!(MAX_EXTRACTION_SIZE, 20 * 1024 * 1024 * 1024); // 20 GB
        assert_eq!(MAX_COMPRESSION_RATIO, 100); // 100:1
    }
}

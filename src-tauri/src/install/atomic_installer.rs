use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::installer::sanitize_folder_name;
use crate::logger;
use crate::models::{
    BackupFileEntry, InstallPhase, InstallProgress, InstallTask, NavdataBackupVerification,
};

/// Minimum required free space (1 GB) as a safety buffer
const MIN_FREE_SPACE_BYTES: u64 = 1024 * 1024 * 1024;

/// Maximum symlink resolution depth to prevent infinite loops
const MAX_SYMLINK_DEPTH: usize = 40;

/// Atomic installer for safer installation operations
pub struct AtomicInstaller {
    /// Temporary directory for staging files (same drive as target)
    temp_dir: PathBuf,
    /// Target installation directory
    target_dir: PathBuf,
    /// X-Plane root directory
    xplane_root: PathBuf,
    /// Backup directory for original files (if exists)
    backup_dir: Option<PathBuf>,
    /// App handle for emitting progress events
    app_handle: AppHandle,
    /// Total number of tasks (for progress calculation)
    total_tasks: usize,
    /// Current task index (for progress calculation)
    current_task: usize,
    /// The overall percentage this task should show (task-proportional, not 100%)
    task_percentage: f64,
    /// In parallel mode, delegate to parent's emit_aggregated instead of emitting directly
    parallel_emit: Option<Arc<dyn Fn() + Send + Sync>>,
    /// In parallel mode, update tracker's current_file
    parallel_current_file: Option<Arc<Mutex<Option<String>>>>,
}

impl AtomicInstaller {
    /// Create a new atomic installer
    /// The temp directory will be created in the X-Plane root directory
    ///
    /// # Arguments
    /// * `target_dir` - The target installation directory (e.g., C:\X-Plane\Aircraft\A330)
    /// * `xplane_root` - The X-Plane root directory (e.g., C:\X-Plane)
    /// * `app_handle` - Tauri app handle for emitting progress events
    /// * `total_tasks` - Total number of tasks for progress calculation
    /// * `current_task` - Current task index for progress calculation
    /// * `task_percentage` - The overall percentage this task should show when complete
    pub fn new(
        target_dir: &Path,
        xplane_root: &Path,
        app_handle: AppHandle,
        total_tasks: usize,
        current_task: usize,
        task_percentage: f64,
    ) -> Result<Self> {
        // Check available disk space
        check_disk_space(xplane_root)?;

        // Create temp directory in X-Plane root directory
        let temp_dir = xplane_root.join(format!(".xfastmanager_temp_{}", Uuid::new_v4()));

        fs::create_dir_all(&temp_dir)
            .context(format!("Failed to create temp directory: {:?}", temp_dir))?;

        logger::log_info(
            &format!("Created atomic install temp directory: {:?}", temp_dir),
            Some("atomic_installer"),
        );

        Ok(Self {
            temp_dir,
            target_dir: target_dir.to_path_buf(),
            xplane_root: xplane_root.to_path_buf(),
            backup_dir: None,
            app_handle,
            total_tasks,
            current_task,
            task_percentage,
            parallel_emit: None,
            parallel_current_file: None,
        })
    }

    /// Set parallel mode callbacks so atomic installer delegates to the parallel
    /// progress context instead of emitting serial-mode events directly.
    pub fn set_parallel_emit(
        &mut self,
        emit_fn: Arc<dyn Fn() + Send + Sync>,
        current_file: Arc<Mutex<Option<String>>>,
    ) {
        self.parallel_emit = Some(emit_fn);
        self.parallel_current_file = Some(current_file);
    }

    /// Emit progress event to frontend
    fn emit_progress(&self, message: &str, phase: InstallPhase) {
        // In parallel mode, update tracker's current_file and delegate to parent's emit_aggregated
        if let Some(ref emit_fn) = self.parallel_emit {
            if let Some(ref cf) = self.parallel_current_file {
                if let Ok(mut f) = cf.lock() {
                    *f = Some(message.to_string());
                }
            }
            emit_fn();
            return;
        }

        let progress = InstallProgress {
            percentage: self.task_percentage, // Use task-proportional percentage
            total_bytes: 0,
            processed_bytes: 0,
            current_task_index: self.current_task,
            total_tasks: self.total_tasks,
            current_task_name: String::new(),
            current_file: Some(message.to_string()),
            phase,
            verification_progress: None,
            current_task_percentage: 100.0, // Task extraction is complete during atomic operations
            current_task_total_bytes: 0,
            current_task_processed_bytes: 0,
            active_tasks: None,
            completed_task_count: None,
            completed_task_ids: None,
        };

        let _ = self.app_handle.emit("install-progress", &progress);
    }

    /// Get the temporary directory path
    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    /// Get the target directory path
    #[allow(dead_code)]
    pub fn target_dir(&self) -> &Path {
        &self.target_dir
    }

    /// Scenario 1: First-time installation (target doesn't exist)
    /// 1. Extract/copy to temp
    /// 2. Verify
    /// 3. Atomic move temp -> target
    pub fn install_fresh(&mut self) -> Result<()> {
        logger::log_info(
            "Atomic install: Fresh installation (target doesn't exist)",
            Some("atomic_installer"),
        );

        // Verify temp directory has content
        if !self.temp_dir.exists() || fs::read_dir(&self.temp_dir)?.next().is_none() {
            anyhow::bail!("Temp directory is empty, nothing to install");
        }

        // Atomic move: temp -> target
        self.emit_progress(
            "Moving files to target directory...",
            InstallPhase::Installing,
        );
        atomic_move(&self.temp_dir, &self.target_dir)?;

        logger::log_info(
            &format!("Fresh installation completed: {:?}", self.target_dir),
            Some("atomic_installer"),
        );

        // Explicitly cleanup temp directory (it should be empty now, but ensure it's removed)
        self.cleanup_temp_dir();

        Ok(())
    }

    /// Scenario 2: Clean installation (target exists, delete and reinstall)
    /// 1. Extract/copy to temp
    /// 2. Verify
    /// 3. Rename target -> target.backup_<uuid>
    /// 4. Atomic move temp -> target
    /// 5. Restore backup files from backup
    /// 6. Delete backup
    pub fn install_clean(&mut self, task: &InstallTask) -> Result<()> {
        logger::log_info(
            "Atomic install: Clean installation (delete old, install new)",
            Some("atomic_installer"),
        );

        // Create unique backup directory name to avoid conflicts
        let backup_dir = self
            .target_dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Target has no parent"))?
            .join(format!(
                "{}.backup_{}",
                self.target_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown"),
                Uuid::new_v4()
            ));

        // Step 1: Attempt to rename target -> backup
        // Use atomic operation with TOCTOU protection - handle race condition
        // where target might be deleted between check and rename
        self.emit_progress("Backing up original directory...", InstallPhase::Installing);
        logger::log_info(
            &format!(
                "Backing up original directory: {:?} -> {:?}",
                self.target_dir, backup_dir
            ),
            Some("atomic_installer"),
        );

        match fs::rename(&self.target_dir, &backup_dir) {
            Ok(()) => {
                // Successfully backed up
                self.backup_dir = Some(backup_dir.clone());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Target doesn't exist (race condition: deleted between check and rename)
                // Treat as fresh install
                logger::log_info(
                    "Target no longer exists, treating as fresh install",
                    Some("atomic_installer"),
                );
                return self.install_fresh();
            }
            Err(e) => {
                return Err(e).context(format!(
                    "Failed to rename target to backup: {:?}",
                    self.target_dir
                ));
            }
        }

        // Step 2: Atomic move temp -> target
        self.emit_progress(
            "Moving new files to target directory...",
            InstallPhase::Installing,
        );
        match atomic_move(&self.temp_dir, &self.target_dir) {
            Ok(()) => {}
            Err(e) => {
                // Rollback: restore backup
                logger::log_error(
                    &format!("Atomic move failed, rolling back: {}", e),
                    Some("atomic_installer"),
                );

                if let Err(rollback_err) = fs::rename(&backup_dir, &self.target_dir) {
                    logger::log_error(
                        &format!("CRITICAL: Rollback failed: {}", rollback_err),
                        Some("atomic_installer"),
                    );
                }

                return Err(e);
            }
        }

        // Step 3: Restore backup files (liveries, config files)
        if task.backup_liveries || task.backup_config_files {
            self.emit_progress("Restoring backup files...", InstallPhase::Installing);
            if let Err(e) = self.restore_backup_files(task, &backup_dir) {
                logger::log_error(
                    &format!("Failed to restore backup files: {}", e),
                    Some("atomic_installer"),
                );
                // Don't fail the installation, just log the error
            }
        }

        // Step 4: Delete backup directory
        self.emit_progress("Cleaning up backup directory...", InstallPhase::Installing);
        logger::log_info(
            &format!("Removing backup directory: {:?}", backup_dir),
            Some("atomic_installer"),
        );

        if let Err(e) = fs::remove_dir_all(&backup_dir) {
            logger::log_error(
                &format!("Failed to remove backup directory: {}", e),
                Some("atomic_installer"),
            );
            // Don't fail the installation if backup cleanup fails
        }

        logger::log_info(
            &format!("Clean installation completed: {:?}", self.target_dir),
            Some("atomic_installer"),
        );

        // Explicitly cleanup temp directory
        self.cleanup_temp_dir();

        Ok(())
    }

    /// Scenario 3: Overwrite installation (target exists, merge files)
    /// 1. Extract/copy to temp
    /// 2. Verify
    /// 3. For each file in temp, atomic move to target (overwrite)
    /// 4. Keep files in target that don't exist in temp
    pub fn install_overwrite(&mut self) -> Result<()> {
        logger::log_info(
            "Atomic install: Overwrite installation (merge with existing)",
            Some("atomic_installer"),
        );

        // TOCTOU-safe: Try merge directly, handle non-existent target in merge_directories
        // This avoids race condition between exists() check and actual operation
        self.emit_progress(
            "Merging files with existing installation...",
            InstallPhase::Installing,
        );

        match merge_directories(&self.temp_dir, &self.target_dir) {
            Ok(()) => {}
            Err(e) => {
                // Check if error is because target doesn't exist
                if !self.target_dir.exists() {
                    // Target was deleted (or never existed), treat as fresh install
                    logger::log_info(
                        "Target doesn't exist during merge, treating as fresh install",
                        Some("atomic_installer"),
                    );
                    return self.install_fresh();
                }
                return Err(e);
            }
        }

        logger::log_info(
            &format!("Overwrite installation completed: {:?}", self.target_dir),
            Some("atomic_installer"),
        );

        // Explicitly cleanup temp directory
        self.cleanup_temp_dir();

        Ok(())
    }

    /// Scenario 4: Navdata clean install with backup (EXTREME PERFORMANCE OPTIMIZED)
    ///
    /// Performance optimizations:
    /// 1. Uses walkdir for efficient single-pass file enumeration (no extra stat calls)
    /// 2. SKIPS SHA-256 checksum calculation entirely (fs::rename is atomic, checksums redundant)
    /// 3. Directory-level rename for O(1) backup moves
    /// 4. Fast verification uses single fs::metadata() call (no double stat)
    ///
    /// Steps:
    /// 1. Enumerate new navdata entries from temp_dir
    /// 2. Create Backup_Data/<provider_name>/ folder (one backup per provider)
    /// 3. Collect all files to backup using walkdir (path + size, NO checksum)
    /// 4. Move directories to backup (O(1) directory rename when possible)
    /// 5. Fast verify (single stat per file)
    /// 6. Write verification.json
    /// 7. Merge new navdata to target
    ///
    /// If backup_navdata is false, skips backup steps and just deletes old files.
    pub fn install_clean_navdata_with_backup(&mut self, backup_navdata: bool) -> Result<()> {
        logger::log_info(
            "Atomic install: Navdata clean install with backup (extreme optimized)",
            Some("atomic_installer"),
        );

        // Step 1: Enumerate top-level entries in temp_dir (new navdata files/folders)
        self.emit_progress("Scanning new navdata files...", InstallPhase::Installing);
        let new_entries: Vec<std::ffi::OsString> = fs::read_dir(&self.temp_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();

        if new_entries.is_empty() {
            logger::log_info(
                "No new navdata entries found in temp directory",
                Some("atomic_installer"),
            );
            return self.install_fresh();
        }

        logger::log_info(
            &format!("Found {} new navdata entries", new_entries.len()),
            Some("atomic_installer"),
        );

        // Read provider name from new navdata (needed for both backup and cleanup)
        let provider_name = self
            .read_navdata_info(&self.temp_dir)
            .map(|(name, _, _)| name)
            .or_else(|_| {
                self.read_navdata_info(&self.target_dir)
                    .map(|(name, _, _)| name)
            })
            .unwrap_or_else(|_| "navdata".to_string());

        if backup_navdata {
            // Read old cycle/airac from target_dir (the data being backed up)
            let (old_cycle, old_airac) = self
                .read_navdata_info(&self.target_dir)
                .map(|(_, c, a)| (c, a))
                .unwrap_or((None, None));

            // Step 3: Create Backup_Data/<provider_name_timestamp>/ directory in Custom Data
            self.emit_progress("Creating backup directory...", InstallPhase::Installing);
            let backup_data_dir = self.xplane_root.join("Custom Data").join("Backup_Data");
            fs::create_dir_all(&backup_data_dir)?;

            // Use timestamp to create unique backup folder name
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
            let sanitized_provider = sanitize_folder_name(&provider_name);
            let backup_folder_name = format!("{}_{}", sanitized_provider, timestamp);
            let backup_subdir = backup_data_dir.join(&backup_folder_name);

            fs::create_dir_all(&backup_subdir)?;

            logger::log_info(
                &format!("Backup directory created: {:?}", backup_subdir),
                Some("atomic_installer"),
            );

            // Step 4: Collect all files using walkdir (OPTIMIZED: single pass, no extra stat)
            // Skip SHA-256 checksum calculation - fs::rename is atomic, checksums are redundant
            self.emit_progress("Scanning files to backup...", InstallPhase::Installing);
            let custom_data_dir = self.xplane_root.join("Custom Data");
            let mut backup_entries: Vec<BackupFileEntry> = Vec::new();

            for entry_name in &new_entries {
                let old_path = self.target_dir.join(entry_name);
                if old_path.exists() {
                    // Use walkdir for efficient enumeration (DirEntry::file_type() uses cached stat)
                    for entry in WalkDir::new(&old_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                    {
                        // Get size from walkdir's metadata (single stat call)
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        // Relative path from Custom Data (not target_dir) for consistent restore
                        let relative_path = entry
                            .path()
                            .strip_prefix(&custom_data_dir)
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
                Some("atomic_installer"),
            );

            // Step 5: Move files to backup directory (OPTIMIZED: directory-level rename)
            self.emit_progress("Moving files to backup...", InstallPhase::Installing);
            for entry_name in &new_entries {
                let old_path = self.target_dir.join(entry_name);
                if old_path.exists() {
                    // Compute relative path from Custom Data for consistent backup structure
                    let relative_entry = old_path
                        .strip_prefix(&custom_data_dir)
                        .unwrap_or(Path::new(entry_name));
                    let backup_path = backup_subdir.join(relative_entry);
                    logger::log_info(
                        &format!("Moving to backup: {:?}", entry_name),
                        Some("atomic_installer"),
                    );
                    move_directory(&old_path, &backup_path)?;
                }
            }

            // Step 6: Fast verify (OPTIMIZED: single fs::metadata() call per file)
            self.emit_progress("Verifying backup (fast)...", InstallPhase::Installing);
            verify_backup_fast(&backup_subdir, &backup_entries)?;

            logger::log_info(
                &format!(
                    "Fast verification passed: {} files verified",
                    backup_entries.len()
                ),
                Some("atomic_installer"),
            );

            // Step 7: Write verification.json
            let verification = NavdataBackupVerification {
                provider_name: provider_name.clone(),
                cycle: old_cycle,
                airac: old_airac,
                backup_time: chrono::Utc::now().to_rfc3339(),
                files: backup_entries.clone(),
                file_count: backup_entries.len(),
            };

            let verification_path = backup_subdir.join("verification.json");
            let verification_json = serde_json::to_string_pretty(&verification)
                .context("Failed to serialize verification data")?;
            fs::write(&verification_path, verification_json)
                .context("Failed to write verification.json")?;

            logger::log_info(
                &format!(
                    "Backup verification written: {} files backed up",
                    backup_entries.len()
                ),
                Some("atomic_installer"),
            );
        } else {
            // No backup: delete old files that will be replaced by new ones
            logger::log_info(
                "Navdata backup disabled by user, deleting old files directly",
                Some("atomic_installer"),
            );

            // Also delete existing backups for the same provider
            let sanitized_provider = sanitize_folder_name(&provider_name);
            let backup_data_dir = self.xplane_root.join("Custom Data").join("Backup_Data");
            if backup_data_dir.exists() {
                if let Ok(entries) = fs::read_dir(&backup_data_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        if folder_name.starts_with(&sanitized_provider) {
                            logger::log_info(
                                &format!("Deleting existing backup: {}", folder_name),
                                Some("atomic_installer"),
                            );
                            if let Err(e) = fs::remove_dir_all(entry.path()) {
                                logger::log_error(
                                    &format!("Failed to delete backup {}: {}", folder_name, e),
                                    Some("atomic_installer"),
                                );
                            }
                        }
                    }
                }
            }

            for entry_name in &new_entries {
                let old_path = self.target_dir.join(entry_name);
                if old_path.exists() {
                    if old_path.is_dir() {
                        fs::remove_dir_all(&old_path)?;
                    } else {
                        fs::remove_file(&old_path)?;
                    }
                }
            }
        }

        // Step 8: Merge new navdata to target
        self.emit_progress("Installing new navdata...", InstallPhase::Installing);
        merge_directories(&self.temp_dir, &self.target_dir)?;

        logger::log_info(
            &format!("Navdata clean install completed: {:?}", self.target_dir),
            Some("atomic_installer"),
        );

        // Explicitly cleanup temp directory
        self.cleanup_temp_dir();

        Ok(())
    }

    /// Read navdata info from cycle.json (searches recursively)
    fn read_navdata_info(&self, dir: &Path) -> Result<(String, Option<String>, Option<String>)> {
        // Search recursively for cycle.json (handles GNS430 nested structure)
        let cycle_json_path = WalkDir::new(dir)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
            .find(|e| e.file_type().is_file() && e.file_name().to_str() == Some("cycle.json"))
            .map(|e| e.into_path());

        let cycle_json_path = match cycle_json_path {
            Some(p) => p,
            None => anyhow::bail!("cycle.json not found"),
        };

        let content = fs::read_to_string(&cycle_json_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        // Use "name" field to match management_index::parse_cycle_json
        let provider_name = json
            .get("name")
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

        Ok((provider_name, cycle, airac))
    }

    /// Restore backup files (liveries and config files) from backup directory
    fn restore_backup_files(&self, task: &InstallTask, backup_dir: &Path) -> Result<()> {
        use glob::Pattern;

        logger::log_info(
            "Restoring backup files from original installation",
            Some("atomic_installer"),
        );

        // Restore liveries
        if task.backup_liveries {
            let liveries_backup = backup_dir.join("liveries");
            if liveries_backup.exists() {
                let liveries_target = self.target_dir.join("liveries");

                logger::log_info(
                    &format!(
                        "Restoring liveries: {:?} -> {:?}",
                        liveries_backup, liveries_target
                    ),
                    Some("atomic_installer"),
                );

                // Merge liveries (skip existing files to preserve new liveries)
                merge_directories_skip_existing(&liveries_backup, &liveries_target)?;
            }
        }

        // Restore config files (only in root directory)
        if task.backup_config_files && !task.config_file_patterns.is_empty() {
            logger::log_info(
                &format!(
                    "Restoring config files matching patterns: {:?}",
                    task.config_file_patterns
                ),
                Some("atomic_installer"),
            );

            for entry in fs::read_dir(backup_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                        // Check if filename matches any pattern
                        let matches = task.config_file_patterns.iter().any(|pattern| {
                            Pattern::new(pattern)
                                .map(|p| p.matches(filename))
                                .unwrap_or(false)
                        });

                        if matches {
                            let target_file = self.target_dir.join(filename);
                            logger::log_info(
                                &format!("Restoring config file: {}", filename),
                                Some("atomic_installer"),
                            );

                            fs::copy(&path, &target_file)
                                .context(format!("Failed to restore config file: {}", filename))?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Rollback installation if something goes wrong
    #[allow(dead_code)]
    pub fn rollback(&mut self) -> Result<()> {
        logger::log_error("Rolling back atomic installation", Some("atomic_installer"));

        // If we have a backup, restore it
        if let Some(backup_dir) = &self.backup_dir {
            if backup_dir.exists() {
                // Remove the partially installed target
                if self.target_dir.exists() {
                    fs::remove_dir_all(&self.target_dir)
                        .context("Failed to remove partial installation during rollback")?;
                }

                // Restore backup
                fs::rename(backup_dir, &self.target_dir)
                    .context("Failed to restore backup during rollback")?;

                logger::log_info(
                    "Rollback completed: Original files restored",
                    Some("atomic_installer"),
                );
            }
        }

        Ok(())
    }

    /// Explicitly cleanup temp directory
    fn cleanup_temp_dir(&mut self) {
        if self.temp_dir.exists() {
            logger::log_info(
                &format!("Cleaning up temp directory: {:?}", self.temp_dir),
                Some("atomic_installer"),
            );

            match fs::remove_dir_all(&self.temp_dir) {
                Ok(()) => {
                    logger::log_info(
                        "Temp directory cleaned up successfully",
                        Some("atomic_installer"),
                    );
                }
                Err(e) => {
                    logger::log_error(
                        &format!("Failed to cleanup temp directory: {}", e),
                        Some("atomic_installer"),
                    );
                }
            }
        }
    }
}

impl Drop for AtomicInstaller {
    fn drop(&mut self) {
        // Cleanup temp directory
        if self.temp_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&self.temp_dir) {
                logger::log_error(
                    &format!("Failed to cleanup temp directory: {}", e),
                    Some("atomic_installer"),
                );
            } else {
                logger::log_info(
                    &format!("Cleaned up temp directory: {:?}", self.temp_dir),
                    Some("atomic_installer"),
                );
            }
        }
    }
}

/// Atomic move operation (rename on same filesystem)
/// Falls back to copy+delete if rename fails (different filesystems)
/// Note: If copy succeeds but delete fails, logs a warning but still returns Ok
/// to prevent orphan files from blocking installation
fn atomic_move(src: &Path, dst: &Path) -> Result<()> {
    logger::log_info(
        &format!("Atomic move: {:?} -> {:?}", src, dst),
        Some("atomic_installer"),
    );

    // Try atomic rename first (only works on same filesystem)
    match fs::rename(src, dst) {
        Ok(()) => {
            logger::log_info(
                "Atomic move completed successfully (rename)",
                Some("atomic_installer"),
            );
            Ok(())
        }
        Err(e) => {
            logger::log_info(
                &format!("Rename failed ({}), falling back to copy+delete", e),
                Some("atomic_installer"),
            );

            // Fallback: copy then delete
            copy_directory_recursive(src, dst)?;

            // Attempt to remove source, but don't fail if it doesn't work
            // (prevents orphan source files from blocking installation)
            match fs::remove_dir_all(src) {
                Ok(()) => {
                    logger::log_info(
                        "Atomic move completed (copy+delete fallback)",
                        Some("atomic_installer"),
                    );
                }
                Err(delete_err) => {
                    // Log warning but don't fail - the copy succeeded
                    // User may need to manually clean up the source
                    logger::log_error(
                        &format!(
                            "Warning: Failed to remove source after copy: {}. \
                             Manual cleanup of {:?} may be required.",
                            delete_err, src
                        ),
                        Some("atomic_installer"),
                    );
                }
            }

            Ok(())
        }
    }
}

/// Recursively copy a directory
/// Handles regular files, directories, and symbolic links
/// Validates symlink targets to prevent path traversal attacks
fn copy_directory_recursive(src: &Path, dst: &Path) -> Result<()> {
    let visited = HashSet::new();
    copy_directory_recursive_internal(src, dst, src, 0, &visited)
}

/// Internal recursive copy with base directory tracking for symlink validation
/// Includes depth tracking and cycle detection for symlink safety
fn copy_directory_recursive_internal(
    src: &Path,
    dst: &Path,
    base_dir: &Path,
    depth: usize,
    visited: &HashSet<PathBuf>,
) -> Result<()> {
    // Security: Prevent infinite recursion from symlink cycles
    if depth > MAX_SYMLINK_DEPTH {
        return Err(anyhow::anyhow!(
            "Maximum directory depth ({}) exceeded, possible symlink loop at: {:?}",
            MAX_SYMLINK_DEPTH,
            src
        ));
    }

    // Security: Detect symlink cycles by tracking canonical paths
    let canonical_src = src.canonicalize().unwrap_or_else(|_| src.to_path_buf());
    if visited.contains(&canonical_src) {
        logger::log_error(
            &format!("Symlink cycle detected, skipping: {:?}", src),
            Some("atomic_installer"),
        );
        return Ok(()); // Skip this directory to prevent infinite loop
    }

    // Add current path to visited set for children
    let mut new_visited = visited.clone();
    new_visited.insert(canonical_src);

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        // Use symlink_metadata to detect symlinks without following them
        let metadata = fs::symlink_metadata(&src_path)?;

        if metadata.file_type().is_symlink() {
            // Handle symbolic link with path validation
            copy_symlink(&src_path, &dst_path, base_dir, depth)?;
        } else if metadata.is_dir() {
            // Handle directory with incremented depth
            copy_directory_recursive_internal(
                &src_path,
                &dst_path,
                base_dir,
                depth + 1,
                &new_visited,
            )?;
        } else {
            // Handle regular file
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Copy a symbolic link from src to dst
/// Preserves the symlink target (doesn't follow the link)
/// Validates that the symlink target is within the base directory (security check)
/// Includes depth tracking to prevent infinite symlink loops
#[cfg(unix)]
fn copy_symlink(src: &Path, dst: &Path, base_dir: &Path, depth: usize) -> Result<()> {
    use std::os::unix::fs::symlink;

    // Security: Check depth limit for symlink resolution
    if depth > MAX_SYMLINK_DEPTH {
        return Err(anyhow::anyhow!(
            "Maximum symlink depth ({}) exceeded at: {:?}",
            MAX_SYMLINK_DEPTH,
            src
        ));
    }

    let target = fs::read_link(src).context(format!("Failed to read symlink: {:?}", src))?;

    // Security check: validate symlink target doesn't escape base directory
    let resolved = if target.is_relative() {
        src.parent().unwrap_or(src).join(&target)
    } else {
        target.clone()
    };

    // Attempt to canonicalize - if it fails (target doesn't exist), check path components
    let is_safe = if let Ok(canonical) = resolved.canonicalize() {
        // Canonicalized path should be within base_dir
        if let Ok(canonical_base) = base_dir.canonicalize() {
            canonical.starts_with(&canonical_base)
        } else {
            // If base_dir can't be canonicalized, be conservative
            false
        }
    } else {
        // Target doesn't exist - check for path traversal patterns
        !target
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    };

    if !is_safe {
        logger::log_error(
            &format!(
                "Symlink security check failed: {:?} -> {:?} (base: {:?})",
                src, target, base_dir
            ),
            Some("atomic_installer"),
        );
        return Err(anyhow::anyhow!(
            "Symlink target outside installation directory: {:?}",
            target
        ));
    }

    logger::log_info(
        &format!(
            "Copying symlink: {:?} -> {:?} (target: {:?})",
            src, dst, target
        ),
        Some("atomic_installer"),
    );

    // Remove destination if it exists
    if dst.exists() || fs::symlink_metadata(dst).is_ok() {
        let _ = fs::remove_file(dst);
    }

    symlink(&target, dst).context(format!(
        "Failed to create symlink: {:?} -> {:?}",
        dst, target
    ))?;

    Ok(())
}

/// Copy a symbolic link from src to dst (Windows version)
/// Windows requires different functions for file vs directory symlinks
/// Validates that the symlink target is within the base directory (security check)
/// Includes depth tracking to prevent infinite symlink loops
#[cfg(windows)]
fn copy_symlink(src: &Path, dst: &Path, base_dir: &Path, depth: usize) -> Result<()> {
    use std::os::windows::fs::{symlink_dir, symlink_file};

    // Security: Check depth limit for symlink resolution
    if depth > MAX_SYMLINK_DEPTH {
        return Err(anyhow::anyhow!(
            "Maximum symlink depth ({}) exceeded at: {:?}",
            MAX_SYMLINK_DEPTH,
            src
        ));
    }

    let target = fs::read_link(src).context(format!("Failed to read symlink: {:?}", src))?;

    // Security check: validate symlink target doesn't escape base directory
    let resolved = if target.is_relative() {
        src.parent().unwrap_or(src).join(&target)
    } else {
        target.clone()
    };

    // Attempt to canonicalize - if it fails (target doesn't exist), check path components
    let is_safe = if let Ok(canonical) = resolved.canonicalize() {
        // Canonicalized path should be within base_dir
        if let Ok(canonical_base) = base_dir.canonicalize() {
            canonical.starts_with(&canonical_base)
        } else {
            // If base_dir can't be canonicalized, be conservative
            false
        }
    } else {
        // Target doesn't exist - check for path traversal patterns
        !target
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    };

    if !is_safe {
        logger::log_error(
            &format!(
                "Symlink security check failed: {:?} -> {:?} (base: {:?})",
                src, target, base_dir
            ),
            Some("atomic_installer"),
        );
        return Err(anyhow::anyhow!(
            "Symlink target outside installation directory: {:?}",
            target
        ));
    }

    logger::log_info(
        &format!(
            "Copying symlink: {:?} -> {:?} (target: {:?})",
            src, dst, target
        ),
        Some("atomic_installer"),
    );

    // Remove destination if it exists
    if dst.exists() || fs::symlink_metadata(dst).is_ok() {
        let _ = fs::remove_file(dst);
    }

    // Determine if target is a directory or file
    // We need to check the target's metadata to know which symlink function to use
    let target_is_dir = if target.is_absolute() {
        target.is_dir()
    } else {
        // Relative symlink - resolve relative to source directory
        src.parent()
            .map(|p| p.join(&target).is_dir())
            .unwrap_or(false)
    };

    if target_is_dir {
        if let Err(e) = symlink_dir(&target, dst) {
            logger::log_error(
                &format!(
                    "Failed to create directory symlink: {:?} -> {:?} ({}). Falling back to copy.",
                    dst, target, e
                ),
                Some("atomic_installer"),
            );
            if resolved.exists() {
                copy_directory_recursive(&resolved, dst)?;
            } else {
                return Err(anyhow::anyhow!(format!(
                    "Failed to create directory symlink and target missing: {:?} -> {:?} ({})",
                    dst, target, e
                )));
            }
        }
    } else if let Err(e) = symlink_file(&target, dst) {
        logger::log_error(
            &format!(
                "Failed to create file symlink: {:?} -> {:?} ({}). Falling back to copy.",
                dst, target, e
            ),
            Some("atomic_installer"),
        );
        if resolved.exists() {
            fs::copy(&resolved, dst).context(format!(
                "Failed to copy symlink target: {:?} -> {:?}",
                resolved, dst
            ))?;
        } else {
            return Err(anyhow::anyhow!(format!(
                "Failed to create file symlink and target missing: {:?} -> {:?} ({})",
                dst, target, e
            )));
        }
    }

    Ok(())
}

/// Merge directories: move all files from src to dst, overwriting existing files
/// TOCTOU-safe: Uses atomic operations and handles race conditions gracefully
fn merge_directories(src: &Path, dst: &Path) -> Result<()> {
    // Create destination if it doesn't exist (atomic - no TOCTOU issue)
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            // Recursively merge subdirectories
            merge_directories(&src_path, &dst_path)?;
            // Remove the now-empty source directory
            if let Err(e) = fs::remove_dir(&src_path) {
                logger::log_error(
                    &format!("Failed to remove source directory after merge: {}", e),
                    Some("atomic_installer"),
                );
            }
        } else {
            // TOCTOU-safe: Try remove then rename, handle errors gracefully
            // Instead of checking exists() first, just try to remove and ignore NotFound
            let _ = fs::remove_file(&dst_path); // Ignore error if file doesn't exist

            match fs::rename(&src_path, &dst_path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // Source was deleted (race condition), skip this file
                    logger::log_info(
                        &format!("Source file no longer exists, skipping: {:?}", src_path),
                        Some("atomic_installer"),
                    );
                }
                Err(_) => {
                    // Fallback to copy (cross-device or other error)
                    fs::copy(&src_path, &dst_path)?;
                    let _ = fs::remove_file(&src_path); // Best effort cleanup
                }
            }
        }
    }

    Ok(())
}

/// Merge directories but skip files that already exist in destination
/// Used for restoring liveries (don't overwrite new liveries)
fn merge_directories_skip_existing(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            merge_directories_skip_existing(&src_path, &dst_path)?;
        } else {
            // Only copy if destination doesn't exist
            if !dst_path.exists() {
                match fs::rename(&src_path, &dst_path) {
                    Ok(()) => {}
                    Err(_) => {
                        fs::copy(&src_path, &dst_path)?;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Move a directory or file from src to dst (OPTIMIZED)
/// First attempts directory-level rename (O(1) operation on same filesystem)
/// Falls back to recursive copy+delete for cross-filesystem moves
fn move_directory(src: &Path, dst: &Path) -> Result<()> {
    // Optimization: Try to rename the entire directory at once (O(1) operation)
    // This avoids per-file syscalls when source and destination are on same filesystem
    if src.is_dir() {
        // Ensure parent of destination exists
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        // Try direct directory rename first
        match fs::rename(src, dst) {
            Ok(()) => {
                logger::log_info(
                    &format!("Directory moved via rename: {:?} -> {:?}", src, dst),
                    Some("atomic_installer"),
                );
                return Ok(());
            }
            Err(e) => {
                // Cross-device link error (EXDEV on Unix, different error on Windows)
                // Fall back to recursive approach
                logger::log_info(
                    &format!(
                        "Directory rename failed ({}), falling back to recursive copy",
                        e
                    ),
                    Some("atomic_installer"),
                );
            }
        }

        // Fallback: recursive copy + delete
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_child = entry.path();
            let dst_child = dst.join(entry.file_name());
            move_directory(&src_child, &dst_child)?;
        }
        // Remove source directory after moving all contents
        fs::remove_dir(src).ok();
    } else {
        // For files, create parent directory if needed
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        // Try atomic rename first
        match fs::rename(src, dst) {
            Ok(()) => {}
            Err(_) => {
                // Fallback to copy + delete for cross-filesystem
                fs::copy(src, dst)?;
                fs::remove_file(src).ok();
            }
        }
    }

    Ok(())
}

/// Fast verification: check file existence and size only (OPTIMIZED: single stat per file)
/// Skips re-computing checksums since fs::rename is atomic on same filesystem
fn verify_backup_fast(backup_dir: &Path, entries: &[BackupFileEntry]) -> Result<()> {
    for entry in entries {
        let file_path = backup_dir.join(&entry.relative_path);
        // OPTIMIZED: Use single fs::metadata() call instead of exists() + metadata()
        match fs::metadata(&file_path) {
            Ok(meta) => {
                if meta.len() != entry.size {
                    anyhow::bail!(
                        "Backup size mismatch for {}: expected {} bytes, got {} bytes",
                        entry.relative_path,
                        entry.size,
                        meta.len()
                    );
                }
            }
            Err(_) => {
                anyhow::bail!("Backup file missing: {}", entry.relative_path);
            }
        }
    }
    Ok(())
}

/// Check if there's sufficient disk space for atomic installation
/// Requires at least MIN_FREE_SPACE_BYTES (1 GB) of free space
#[cfg(target_os = "windows")]
fn check_disk_space(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::GetDiskFreeSpaceExW;

    // Get the root path (drive letter)
    let root_path = path.ancestors().last().unwrap_or(path);

    // Convert to wide string for Windows API
    let wide_path: Vec<u16> = root_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free_bytes: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_free_bytes: u64 = 0;

    unsafe {
        let result = GetDiskFreeSpaceExW(
            wide_path.as_ptr(),
            &mut free_bytes as *mut u64 as *mut _,
            &mut total_bytes as *mut u64 as *mut _,
            &mut total_free_bytes as *mut u64 as *mut _,
        );

        if result == 0 {
            return Err(anyhow::anyhow!("Failed to check disk space"));
        }
    }

    let free_gb = free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    logger::log_info(
        &format!("Available disk space: {:.2} GB", free_gb),
        Some("atomic_installer"),
    );

    if free_bytes < MIN_FREE_SPACE_BYTES {
        return Err(anyhow::anyhow!(
            "Insufficient disk space: {:.2} GB available, at least 1 GB required",
            free_gb
        ));
    }

    Ok(())
}

/// Check disk space (Unix/Linux/macOS - using statvfs)
#[cfg(not(target_os = "windows"))]
fn check_disk_space(path: &Path) -> Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    // Convert path to C string
    let path_bytes = path.as_os_str().as_bytes();
    let c_path = CString::new(path_bytes).context("Failed to convert path to C string")?;

    // Call statvfs
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };

    if result != 0 {
        return Err(anyhow::anyhow!("Failed to get filesystem statistics"));
    }

    // Calculate available space: f_bavail * f_frsize
    // f_bavail is the number of free blocks available to non-privileged process
    // f_frsize is the fragment size (preferred block size)
    // Cast needed for macOS where f_bavail is u32, but on Linux it's already u64
    #[allow(clippy::unnecessary_cast)]
    let available_bytes = (stat.f_bavail as u64) * (stat.f_frsize as u64);
    let available_gb = available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    logger::log_info(
        &format!("Available disk space: {:.2} GB", available_gb),
        Some("atomic_installer"),
    );

    if available_bytes < MIN_FREE_SPACE_BYTES {
        return Err(anyhow::anyhow!(
            "Insufficient disk space: {:.2} GB available, at least 1 GB required",
            available_gb
        ));
    }

    Ok(())
}

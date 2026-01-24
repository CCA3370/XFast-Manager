use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::logger;
use crate::models::{InstallPhase, InstallProgress, InstallTask};

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
    /// Backup directory for original files (if exists)
    backup_dir: Option<PathBuf>,
    /// App handle for emitting progress events
    app_handle: AppHandle,
    /// Total number of tasks (for progress calculation)
    total_tasks: usize,
    /// Current task index (for progress calculation)
    current_task: usize,
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
    pub fn new(
        target_dir: &Path,
        xplane_root: &Path,
        app_handle: AppHandle,
        total_tasks: usize,
        current_task: usize,
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
            backup_dir: None,
            app_handle,
            total_tasks,
            current_task,
        })
    }

    /// Emit progress event to frontend
    /// Note: We use percentage=90.0 for atomic operations since extraction (0-90%) is already done
    /// This prevents the progress bar from resetting to 0% during atomic move/backup/restore phases
    fn emit_progress(&self, message: &str, phase: InstallPhase) {
        let progress = InstallProgress {
            percentage: 90.0, // Atomic operations happen after extraction (which uses 0-90%)
            total_bytes: 0,
            processed_bytes: 0,
            current_task_index: self.current_task,
            total_tasks: self.total_tasks,
            current_task_name: String::new(),
            current_file: Some(message.to_string()),
            phase,
            verification_progress: None,
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
            copy_directory_recursive_internal(&src_path, &dst_path, base_dir, depth + 1, &new_visited)?;
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
        !target.components().any(|c| matches!(c, std::path::Component::ParentDir))
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
        !target.components().any(|c| matches!(c, std::path::Component::ParentDir))
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
        symlink_dir(&target, dst).context(format!(
            "Failed to create directory symlink: {:?} -> {:?}",
            dst, target
        ))?;
    } else {
        symlink_file(&target, dst).context(format!(
            "Failed to create file symlink: {:?} -> {:?}",
            dst, target
        ))?;
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
    let available_bytes = stat.f_bavail as u64 * stat.f_frsize as u64;
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

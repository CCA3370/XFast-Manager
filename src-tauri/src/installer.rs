use anyhow::{Context, Result};
use glob::Pattern;
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

use crate::logger;
use crate::logger::{tr, LogMsg};
use crate::models::{AddonType, InstallPhase, InstallProgress, InstallResult, InstallTask, TaskResult};
use crate::task_control::TaskControl;

/// Maximum allowed extraction size (20 GB) - archives larger than this will show a warning
pub const MAX_EXTRACTION_SIZE: u64 = 20 * 1024 * 1024 * 1024;

/// Maximum compression ratio to detect zip bombs (100:1)
pub const MAX_COMPRESSION_RATIO: u64 = 100;

/// Maximum size for in-memory ZIP optimization (200 MB)
/// Larger files are extracted via temp directory to avoid memory pressure
pub const MAX_MEMORY_ZIP_SIZE: u64 = 200 * 1024 * 1024;

/// Buffer size for file I/O operations (4 MB)
/// Optimized for modern SSDs and network storage
const IO_BUFFER_SIZE: usize = 4 * 1024 * 1024;

/// Pre-compiled glob patterns for efficient matching
struct CompiledPatterns {
    patterns: Vec<Pattern>,
}

impl CompiledPatterns {
    /// Create new compiled patterns from string patterns
    fn new(pattern_strings: &[String]) -> Self {
        let patterns = pattern_strings
            .iter()
            .filter_map(|s| Pattern::new(s).ok())
            .collect();
        CompiledPatterns { patterns }
    }

    /// Check if filename matches any of the compiled patterns
    fn matches(&self, filename: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(filename))
    }
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
    let mut buffer = vec![0u8; IO_BUFFER_SIZE];
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
/// Includes retry logic with exponential backoff for Windows file locking issues
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

    // Try to delete with retries (handles temporary file locks from antivirus, indexing, etc.)
    const MAX_RETRIES: u32 = 3;
    const INITIAL_DELAY_MS: u64 = 100;

    let mut last_error = None;
    for attempt in 0..=MAX_RETRIES {
        match fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = Some(e);
                if attempt < MAX_RETRIES {
                    // Exponential backoff: 100ms, 200ms, 400ms
                    let delay = INITIAL_DELAY_MS * (1 << attempt);
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                }
            }
        }
    }

    // All retries failed, provide detailed error information
    let e = last_error.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "Unknown error during directory removal")
    });
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
#[derive(Clone)]
struct ProgressContext {
    app_handle: AppHandle,
    total_bytes: Arc<AtomicU64>,
    processed_bytes: Arc<AtomicU64>,
    last_emit: Arc<Mutex<Instant>>,
    current_task_index: usize,
    total_tasks: usize,
    current_task_name: String,
    /// Verification progress (0-100), stored as integer percentage * 100 for atomic ops
    verification_progress: Arc<AtomicU64>,
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
            verification_progress: Arc::new(AtomicU64::new(0)),
        }
    }

    fn set_total_bytes(&self, total: u64) {
        self.total_bytes.store(total, Ordering::SeqCst);
    }

    fn add_bytes(&self, bytes: u64) {
        self.processed_bytes.fetch_add(bytes, Ordering::SeqCst);
    }

    /// Set verification progress (0.0 - 100.0)
    fn set_verification_progress(&self, progress: f64) {
        // Store as integer (progress * 100) for atomic operations
        let stored = (progress * 100.0) as u64;
        self.verification_progress.store(stored, Ordering::SeqCst);
    }

    /// Get verification progress (0.0 - 100.0)
    fn get_verification_progress(&self) -> f64 {
        let stored = self.verification_progress.load(Ordering::SeqCst);
        stored as f64 / 100.0
    }

    fn emit_progress(&self, current_file: Option<String>, phase: InstallPhase) {
        // Throttle: emit at most every 16ms (60fps for smooth animation)
        let mut last = match self.last_emit.lock() {
            Ok(guard) => guard,
            Err(e) => {
                logger::log_error(
                    &format!("Progress mutex poisoned, skipping update: {}", e),
                    Some("installer")
                );
                return; // Skip progress update if lock is poisoned
            }
        };
        let now = Instant::now();
        if now.duration_since(*last).as_millis() < 16 {
            return;
        }
        *last = now;
        drop(last);

        let total = self.total_bytes.load(Ordering::SeqCst);
        let processed = self.processed_bytes.load(Ordering::SeqCst);

        // Calculate percentage: installation is 0-90%, verification is 90-100%
        let (percentage, verification_progress) = match phase {
            InstallPhase::Verifying => {
                let verify_progress = self.get_verification_progress();
                // 90% + (verification_progress / 100) * 10%
                let pct = 90.0 + (verify_progress / 100.0) * 10.0;
                (pct, Some(verify_progress))
            }
            InstallPhase::Finalizing => {
                (100.0, None)
            }
            _ => {
                // Installation phase: 0-90%
                let install_pct = if total > 0 {
                    (processed as f64 / total as f64) * 90.0
                } else {
                    0.0
                };
                (install_pct, None)
            }
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
            verification_progress,
        };

        let _ = self.app_handle.emit("install-progress", &progress);
    }

    fn emit_final(&self, phase: InstallPhase) {
        let total = self.total_bytes.load(Ordering::SeqCst);
        let processed = self.processed_bytes.load(Ordering::SeqCst);

        // Final progress is always 100%
        let progress = InstallProgress {
            percentage: 100.0,
            total_bytes: total,
            processed_bytes: processed,
            current_task_index: self.current_task_index,
            total_tasks: self.total_tasks,
            current_task_name: self.current_task_name.clone(),
            current_file: None,
            phase,
            verification_progress: None,
        };

        let _ = self.app_handle.emit("install-progress", &progress);
    }
}

pub struct Installer {
    app_handle: AppHandle,
    task_control: TaskControl,
}

impl Installer {
    pub fn new(app_handle: AppHandle) -> Self {
        // Get TaskControl from app state
        let task_control = app_handle.state::<TaskControl>().inner().clone();
        Installer { app_handle, task_control }
    }

    /// Install a list of tasks with progress reporting
    pub fn install(&self, tasks: Vec<InstallTask>, atomic_install_enabled: bool, xplane_path: String, delete_source_after_install: bool) -> Result<InstallResult> {
        let install_start = Instant::now();
        crate::log_debug!(
            &format!("[TIMING] Installation started: {} tasks (atomic: {})", tasks.len(), atomic_install_enabled),
            "installer_timing"
        );

        logger::log_info(
            &format!("{}: {} task(s) (atomic mode: {})", tr(LogMsg::InstallationStarted), tasks.len(), atomic_install_enabled),
            Some("installer"),
        );

        // Reset task control at start of installation
        self.task_control.reset();

        let mut ctx = ProgressContext::new(self.app_handle.clone(), tasks.len());
        let mut task_results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;
        let mut cancelled = 0;
        let mut skipped = 0;

        // Phase 1: Calculate total size
        let calc_start = Instant::now();
        crate::log_debug!(
            "[TIMING] Size calculation started",
            "installer_timing"
        );
        ctx.emit_progress(None, InstallPhase::Calculating);
        let total_size = self.calculate_total_size(&tasks)?;
        ctx.set_total_bytes(total_size);
        crate::log_debug!(
            &format!("[TIMING] Size calculation completed in {:.2}ms: {} bytes ({:.2} MB)",
                calc_start.elapsed().as_secs_f64() * 1000.0,
                total_size,
                total_size as f64 / (1024.0 * 1024.0)
            ),
            "installer_timing"
        );

        // Phase 2: Install each task
        let install_phase_start = Instant::now();
        crate::log_debug!(
            "[TIMING] Installation phase started",
            "installer_timing"
        );

        for (index, task) in tasks.iter().enumerate() {
            // Check for cancellation before starting each task
            if self.task_control.is_cancelled() {
                logger::log_info("Installation cancelled by user", Some("installer"));

                // Mark remaining tasks as cancelled
                for remaining_task in tasks.iter().skip(index) {
                    cancelled += 1;
                    task_results.push(TaskResult {
                        task_id: remaining_task.id.clone(),
                        task_name: remaining_task.display_name.clone(),
                        success: false,
                        error_message: Some("Cancelled by user".to_string()),
                        verification_stats: None,
                    });
                }
                break;
            }

            let task_start = Instant::now();
            crate::log_debug!(
                &format!("[TIMING] Task {} started: {}", index + 1, task.display_name),
                "installer_timing"
            );

            ctx.current_task_index = index;
            ctx.current_task_name = task.display_name.clone();
            ctx.emit_progress(None, InstallPhase::Installing);

            logger::log_info(
                &format!("{}: {} -> {}", tr(LogMsg::Installing), task.display_name, task.target_path),
                Some("installer"),
            );

            // Track target path for potential cleanup
            self.task_control.add_processed_path(PathBuf::from(&task.target_path));

            match self.install_task_with_progress(task, &ctx, atomic_install_enabled, &xplane_path) {
                Ok(_) => {
                    // Check for skip request after installation but before verification
                    if self.task_control.is_skip_requested() {
                        logger::log_info(
                            &format!("Task skipped by user: {}", task.display_name),
                            Some("installer")
                        );

                        // Cleanup the installed files
                        if let Err(e) = self.cleanup_task(task) {
                            logger::log_error(
                                &format!("Failed to cleanup skipped task: {}", e),
                                Some("installer")
                            );
                        }

                        skipped += 1;
                        task_results.push(TaskResult {
                            task_id: task.id.clone(),
                            task_name: task.display_name.clone(),
                            success: false,
                            error_message: Some("Skipped by user".to_string()),
                            verification_stats: None,
                        });

                        // Reset skip flag for next task
                        self.task_control.reset_skip();
                        continue;
                    }

                    crate::log_debug!(
                        &format!("[TIMING] Task {} installation completed in {:.2}ms: {}",
                            index + 1,
                            task_start.elapsed().as_secs_f64() * 1000.0,
                            task.display_name
                        ),
                        "installer_timing"
                    );

                    // Verify installation by checking for typical files
                    let verify_start = Instant::now();
                    crate::log_debug!(
                        &format!("[TIMING] Task {} verification started: {}", index + 1, task.display_name),
                        "installer_timing"
                    );

                    // Reset verification progress for this task
                    ctx.set_verification_progress(0.0);
                    ctx.emit_progress(Some("Verifying...".to_string()), InstallPhase::Verifying);

                    match self.verify_installation(task, &ctx) {
                        Ok(verification_stats) => {
                            crate::log_debug!(
                                &format!("[TIMING] Task {} verification completed in {:.2}ms: {} (verified: {}, failed: {})",
                                    index + 1,
                                    verify_start.elapsed().as_secs_f64() * 1000.0,
                                    task.display_name,
                                    verification_stats.as_ref().map(|s| s.verified_files).unwrap_or(0),
                                    verification_stats.as_ref().map(|s| s.failed_files).unwrap_or(0)
                                ),
                                "installer_timing"
                            );

                            crate::log_debug!(
                                &format!("[TIMING] Task {} total time: {:.2}ms: {}",
                                    index + 1,
                                    task_start.elapsed().as_secs_f64() * 1000.0,
                                    task.display_name
                                ),
                                "installer_timing"
                            );

                            // Set verification to 100% for this task
                            ctx.set_verification_progress(100.0);
                            ctx.emit_progress(None, InstallPhase::Verifying);

                            successful += 1;
                            logger::log_info(
                                &format!("{}: {}", tr(LogMsg::InstallationCompleted), task.display_name),
                                Some("installer"),
                            );
                            task_results.push(TaskResult {
                                task_id: task.id.clone(),
                                task_name: task.display_name.clone(),
                                success: true,
                                error_message: None,
                                verification_stats,
                            });

                            // Delete source file after successful installation if enabled
                            if delete_source_after_install {
                                if let Some(original_path) = &task.original_input_path {
                                    if let Err(e) = self.delete_source_file(original_path, &task.source_path) {
                                        logger::log_error(
                                            &format!("Failed to delete source file {}: {}", original_path, e),
                                            Some("installer"),
                                        );
                                    }
                                }
                            }
                        }
                        Err(verify_err) => {
                            crate::log_debug!(
                                &format!("[TIMING] Task {} verification failed in {:.2}ms: {} - {}",
                                    index + 1,
                                    verify_start.elapsed().as_secs_f64() * 1000.0,
                                    task.display_name,
                                    verify_err
                                ),
                                "installer_timing"
                            );

                            failed += 1;
                            let error_msg = format!("Verification failed: {}", verify_err);
                            logger::log_error(
                                &format!("{} {}: {}", tr(LogMsg::InstallationFailed), task.display_name, error_msg),
                                Some("installer"),
                            );
                            task_results.push(TaskResult {
                                task_id: task.id.clone(),
                                task_name: task.display_name.clone(),
                                success: false,
                                error_message: Some(error_msg),
                                verification_stats: None,
                            });
                        }
                    }
                }
                Err(e) => {
                    crate::log_debug!(
                        &format!("[TIMING] Task {} installation failed in {:.2}ms: {} - {}",
                            index + 1,
                            task_start.elapsed().as_secs_f64() * 1000.0,
                            task.display_name,
                            e
                        ),
                        "installer_timing"
                    );

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
                        verification_stats: None,
                    });
                }
            }
        }

        crate::log_debug!(
            &format!("[TIMING] Installation phase completed in {:.2}ms: {} successful, {} failed, {} skipped, {} cancelled",
                install_phase_start.elapsed().as_secs_f64() * 1000.0,
                successful,
                failed,
                skipped,
                cancelled
            ),
            "installer_timing"
        );

        // Phase 3: Finalize
        let finalize_start = Instant::now();
        ctx.emit_final(InstallPhase::Finalizing);
        logger::log_info(&tr(LogMsg::InstallationCompleted), Some("installer"));
        crate::log_debug!(
            &format!("[TIMING] Finalization completed in {:.2}ms",
                finalize_start.elapsed().as_secs_f64() * 1000.0
            ),
            "installer_timing"
        );

        crate::log_debug!(
            &format!("[TIMING] Installation completed in {:.2}ms: {} total tasks, {} successful, {} failed, {} skipped, {} cancelled",
                install_start.elapsed().as_secs_f64() * 1000.0,
                tasks.len(),
                successful,
                failed,
                skipped,
                cancelled
            ),
            "installer_timing"
        );

        Ok(InstallResult {
            total_tasks: tasks.len(),
            successful_tasks: successful,
            failed_tasks: failed + skipped + cancelled,
            task_results,
        })
    }

    /// Calculate total size of all tasks for progress tracking
    /// Includes extra size for backup/restore operations during clean install
    fn calculate_total_size(&self, tasks: &[InstallTask]) -> Result<u64> {
        let mut total = 0u64;
        for task in tasks {
            let source = Path::new(&task.source_path);
            let target = Path::new(&task.target_path);

            // Add source size (archive or directory)
            if source.is_dir() {
                total += self.get_directory_size(source)?;
            } else if source.is_file() {
                total += self.get_archive_size(source, task.archive_internal_root.as_deref())?;
            }

            // For clean install with existing target, add backup/restore overhead
            // This accounts for: backup liveries + backup configs + restore liveries + restore configs
            if !task.should_overwrite && target.exists() {
                match task.addon_type {
                    AddonType::Aircraft => {
                        // Backup and restore liveries (2x: backup + restore)
                        if task.backup_liveries {
                            let liveries_path = target.join("liveries");
                            if liveries_path.exists() && liveries_path.is_dir() {
                                let liveries_size = self.get_directory_size(&liveries_path).unwrap_or(0);
                                total += liveries_size * 2; // backup + restore
                            }
                        }

                        // Backup and restore config files (2x: backup + restore)
                        if task.backup_config_files {
                            let config_size = self.get_config_files_size(target, &task.config_file_patterns);
                            total += config_size * 2; // backup + restore
                        }
                    }
                    _ => {
                        // Other addon types don't have backup/restore overhead
                    }
                }
            }
        }
        Ok(total)
    }

    /// Get total size of config files matching patterns in a directory
    fn get_config_files_size(&self, dir: &Path, patterns: &[String]) -> u64 {
        // Pre-compile patterns once for efficiency
        let compiled = CompiledPatterns::new(patterns);

        let mut total = 0u64;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if compiled.matches(name) {
                            if let Ok(metadata) = fs::metadata(&path) {
                                total += metadata.len();
                            }
                        }
                    }
                }
            }
        }
        total
    }

    /// Verify that the installation was successful by checking for typical files
    /// and optionally verifying file hashes with retry logic
    /// Returns verification statistics if hash verification was performed
    fn verify_installation(&self, task: &InstallTask, ctx: &ProgressContext) -> Result<Option<crate::models::VerificationStats>> {
        let target = Path::new(&task.target_path);

        // Phase 1: Basic marker file verification (10% of verification progress)
        ctx.set_verification_progress(0.0);
        ctx.emit_progress(Some("Checking marker files...".to_string()), InstallPhase::Verifying);
        self.verify_marker_files(task)?;
        ctx.set_verification_progress(10.0);
        ctx.emit_progress(Some("Marker files OK".to_string()), InstallPhase::Verifying);

        // Phase 2: Hash verification (if enabled and hashes available)
        // IMPORTANT: When verification is disabled, skip ALL hash operations to save time
        if !task.enable_verification {
            logger::log_info(
                "Hash verification disabled for this task - skipping all hash operations",
                Some("installer")
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
                    Some("installer")
                );
                return Ok(None);
            }
        };

        let total_expected = expected_hashes.len();

        logger::log_info(
            &format!("Verifying {} files with hash checking", total_expected),
            Some("installer")
        );

        // Update progress: starting hash verification (10% -> 70%)
        ctx.set_verification_progress(15.0);
        ctx.emit_progress(Some(format!("Verifying {} files...", total_expected)), InstallPhase::Verifying);

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
                        InstallPhase::Verifying
                    );
                }
            }
        )?;

        // Update progress: initial verification done (70%)
        ctx.set_verification_progress(70.0);

        let _initial_failed_count = failed_files.len();
        let mut retried_count = 0;

        // Phase 3: Retry failed files (up to 3 times) (70% -> 95%)
        if !failed_files.is_empty() {
            logger::log_info(
                &format!("Retrying {} failed files (max 3 attempts)", failed_files.len()),
                Some("installer")
            );

            ctx.emit_progress(Some(format!("Retrying {} files...", failed_files.len())), InstallPhase::Verifying);

            retried_count = failed_files.len();
            failed_files = self.retry_failed_files(
                task,
                failed_files,
                &expected_hashes,
            )?;

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
            Some("installer")
        );

        ctx.set_verification_progress(100.0);
        ctx.emit_progress(Some("Verification complete".to_string()), InstallPhase::Verifying);

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
        for entry in WalkDir::new(target).max_depth(5).into_iter().filter_map(|e| e.ok()) {
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
                for entry in WalkDir::new(target).max_depth(3).into_iter().filter_map(|e| e.ok()) {
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
                for entry in WalkDir::new(&earth_nav_data).max_depth(5).into_iter().filter_map(|e| e.ok()) {
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
                for entry in WalkDir::new(target).max_depth(3).into_iter().filter_map(|e| e.ok()) {
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
                let cycle_json = target.join("cycle.json");
                if !cycle_json.exists() {
                    return Err(anyhow::anyhow!(
                        "Installation verification failed: No cycle.json found in navdata directory: {:?}",
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
                &format!("Retry attempt {}/{} for {} files", retry_attempt, MAX_RETRIES, failed_files.len()),
                Some("installer")
            );

            // Track which files were successfully re-extracted
            let mut re_extracted_files = Vec::new();

            // Re-extract failed files
            for failed in &mut failed_files {
                logger::log_debug(
                    &format!("Retrying file: {}", failed.path),
                    Some("installer"),
                    None
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
                            &format!("Re-extracted file: {} (attempt {})", failed.path, retry_attempt),
                            Some("installer"),
                            None
                        );
                    }
                    Err(e) => {
                        logger::log_error(
                            &format!("Failed to re-extract {}: {}", failed.path, e),
                            Some("installer")
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

                    let verification = verifier.verify_single_file(
                        &file_path,
                        &result.path,
                        expected
                    );

                    if verification.success {
                        logger::log_info(
                            &format!("File verified after retry: {}", result.path),
                            Some("installer")
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
                    &format!("All files verified successfully after {} retries", retry_attempt),
                    Some("installer")
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
        let ext = source.extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

        match ext {
            "zip" => self.re_extract_from_zip(source, target, relative_path, internal_root, extraction_chain, password),
            "7z" => self.re_extract_from_7z(source, target, relative_path, internal_root, extraction_chain, password),
            "rar" => self.re_extract_from_rar(source, target, relative_path, internal_root, extraction_chain, password),
            _ => Err(anyhow::anyhow!("Unsupported archive format for retry: {}", ext)),
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
        use zip::ZipArchive;
        use std::io::copy;

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

        let i = file_index.ok_or_else(|| anyhow::anyhow!("File not found in ZIP: {}", archive_path_normalized))?;

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
                let mut decrypted = archive.by_index_decrypt(i, pwd.as_bytes())
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

        // 7z doesn't support single-file extraction easily
        // Extract to temp, then copy the specific file
        let temp_dir = TempDir::new()?;

        // Extract entire archive to temp
        if let Some(pwd) = password {
            let mut reader = sevenz_rust2::SevenZReader::open(archive_path, sevenz_rust2::Password::from(pwd))
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
                    std::io::copy(reader, &mut file)?;
                }
                Ok(true)
            }).map_err(|e| anyhow::anyhow!("7z extraction failed: {}", e))?;
        } else {
            let mut reader = sevenz_rust2::SevenZReader::open(archive_path, sevenz_rust2::Password::empty())
                .map_err(|e| anyhow::anyhow!("Failed to open 7z: {}", e))?;
            reader.for_each_entries(|entry, reader| {
                let dest_path = temp_dir.path().join(entry.name());
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
            }).map_err(|e| anyhow::anyhow!("7z extraction failed: {}", e))?;
        }

        // Find and copy the specific file
        let temp_file = temp_dir.path().join(relative_path);
        if !temp_file.exists() {
            return Err(anyhow::anyhow!("File not found after 7z extraction: {}", relative_path));
        }

        let target_file = target.join(relative_path);
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
            Some("installer")
        );

        // Full level: file names
        let file_names: Vec<&str> = failed.iter()
            .take(10) // Limit to first 10 files
            .map(|f| f.path.as_str())
            .collect();

        if !file_names.is_empty() {
            logger::log_info(
                &format!("Failed files: {}{}",
                    file_names.join(", "),
                    if failed.len() > 10 { format!(" (and {} more)", failed.len() - 10) } else { String::new() }
                ),
                Some("installer")
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
                None
            );
        }
    }

    /// Get total size of files in a directory
    fn get_directory_size(&self, dir: &Path) -> Result<u64> {
        // Check cache first
        if let Some(cached) = crate::cache::get_cached_directory_metadata(dir) {
            return Ok(cached.total_size);
        }

        // Calculate size if not cached
        let mut size = 0u64;
        let mut file_count = 0usize;
        for entry in walkdir::WalkDir::new(dir).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() {
                size += entry.metadata()?.len();
                file_count += 1;
            }
        }

        // Cache the result
        crate::cache::cache_directory_metadata(dir, size, file_count);

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
    fn install_task_with_progress(&self, task: &InstallTask, ctx: &ProgressContext, atomic_install_enabled: bool, xplane_path: &str) -> Result<()> {
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
            &format!("[TIMING] Directory creation completed in {:.2}ms",
                mkdir_start.elapsed().as_secs_f64() * 1000.0
            ),
            "installer_timing"
        );

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
                self.handle_clean_install_with_extraction_chain(task, source, target, chain, ctx, password)?;
            } else {
                // Direct overwrite mode for nested archives
                crate::log_debug!(
                    "[TIMING] Direct overwrite mode for nested archive",
                    "installer_timing"
                );
                self.install_content_with_extraction_chain(source, target, chain, ctx, password)?;
            }
        } else if atomic_install_enabled {
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
                self.install_content_with_progress(source, target, task.archive_internal_root.as_deref(), ctx, password)?;
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
        if source.is_dir() {
            self.copy_directory_with_progress(source, target, ctx)?;
        } else if source.is_file() {
            self.extract_archive_with_progress(source, target, internal_root, ctx, password)?;
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
        let all_zip = chain.archives.iter().all(|a| a.format == "zip");

        if all_zip {
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
        use zip::ZipArchive;
        use std::io::{Cursor, Read};

        crate::logger::log_info(
            &format!("Using optimized memory extraction for {} nested ZIP layers", chain.archives.len()),
            Some("installer"),
        );

        // Open the outermost archive
        let file = fs::File::open(source)?;
        let mut current_archive_data = Vec::new();
        file.take(u64::MAX).read_to_end(&mut current_archive_data)?;

        let mut current_password = outermost_password.map(|s| s.as_bytes().to_vec());

        // Navigate through all layers
        for (_index, archive_info) in chain.archives.iter().enumerate() {
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
                let mut file = if let Some(pwd) = current_password.as_deref() {
                    match archive.by_index_decrypt(i, pwd) {
                        Ok(f) => f,
                        Err(e) => {
                            decryption_error = Some(format!("Failed to decrypt {}: {}", nested_path, e));
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
            current_password = archive_info.password.as_ref().map(|s| s.as_bytes().to_vec());
        }

        // Now extract the final (innermost) archive
        let cursor = Cursor::new(current_archive_data);
        let mut archive = ZipArchive::new(cursor)?;

        // Extract all files with final_internal_root filter
        self.extract_zip_from_archive(
            &mut archive,
            target,
            chain.final_internal_root.as_deref(),
            ctx,
            current_password.as_deref(),
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
            &format!("extract_zip_from_archive: target={:?}, internal_root={:?}, archive_len={}",
                target, internal_root, archive.len()),
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
                        &format!("File: '{}', Prefix: '{}', Matched: {:?}",
                            file_path_str, prefix_with_slash, matched.is_some()),
                        Some("installer"),
                        None,
                    );

                    // Strip prefix and return relative path
                    file_path_str.strip_prefix(&prefix_with_slash)
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
                        archive.by_index_decrypt(i, pwd)
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
        let temp_base = TempDir::new()
            .context("Failed to create temp directory for nested extraction")?;

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
            fs::create_dir_all(&extract_target)
                .context(format!("Failed to create extraction target: {:?}", extract_target))?;

            // Extract the current archive
            crate::logger::log_info(
                &format!("Extracting layer {} ({}): {} to {:?}", index, current_format, archive_info.internal_path, extract_target),
                Some("installer"),
            );

            self.extract_archive_with_progress(
                &current_source,
                &extract_target,
                if is_last { chain.final_internal_root.as_deref() } else { None },
                ctx,
                current_password,
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
        use zip::ZipArchive;
        use std::io::{Cursor, Read};

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
        let mut current_password = password.map(|s| s.as_bytes().to_vec());

        // Process remaining ZIP layers in memory
        for (index, archive_info) in remaining_chain.iter().enumerate() {
            let is_last = index == remaining_chain.len() - 1;

            // Verify this is a ZIP layer
            if archive_info.format != "zip" {
                return Err(anyhow::anyhow!("Non-ZIP layer encountered in memory optimization"));
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
                    current_password.as_deref(),
                )?;
                break;
            } else {
                // Intermediate layer: read nested ZIP into memory
                let nested_path = &archive_info.internal_path;
                let mut nested_data = Vec::new();

                let mut found = false;
                for i in 0..archive.len() {
                    let mut file = if let Some(pwd) = current_password.as_deref() {
                        match archive.by_index_decrypt(i, pwd) {
                            Ok(f) => f,
                            Err(_) => continue,
                        }
                    } else {
                        archive.by_index(i)?
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
                current_password = archive_info.password.as_ref().map(|s| s.as_bytes().to_vec());
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
                if let Err(_) = fs::rename(&source_path, &target_path) {
                    // Fallback: copy and delete
                    self.copy_directory_with_progress(
                        &source_path,
                        &target_path,
                        &ProgressContext::new(self.app_handle.clone(), 1),
                    )?;
                    remove_dir_all_robust(&source_path)
                        .context(format!("Failed to remove source directory: {:?}", source_path))?;
                }
            } else {
                // Try to rename (move) the file
                if let Err(_) = fs::rename(&source_path, &target_path) {
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
                self.handle_aircraft_clean_install_with_extraction_chain(
                    source,
                    target,
                    chain,
                    ctx,
                    password,
                    task.backup_liveries,
                    task.backup_config_files,
                    &task.config_file_patterns,
                )?;
            }
            crate::models::AddonType::Navdata => {
                // For Navdata: DON'T delete Custom Data folder!
                // Just extract and overwrite individual files
                self.install_content_with_extraction_chain(source, target, chain, ctx, password)?;
            }
            _ => {
                // For other types: delete and reinstall
                if target.exists() {
                    remove_dir_all_robust(target)
                        .context(format!("Failed to delete existing folder: {:?}", target))?;
                }
                self.install_content_with_extraction_chain(source, target, chain, ctx, password)?;
            }
        }
        Ok(())
    }

    /// Handle aircraft clean install with extraction chain
    fn handle_aircraft_clean_install_with_extraction_chain(
        &self,
        source: &Path,
        target: &Path,
        chain: &crate::models::ExtractionChain,
        ctx: &ProgressContext,
        password: Option<&str>,
        backup_liveries: bool,
        backup_config_files: bool,
        config_file_patterns: &[String],
    ) -> Result<()> {
        
        use uuid::Uuid;

        // Step 1: Backup liveries and config files if requested
        let backup_dir = if (backup_liveries || backup_config_files) && target.exists() {
            let temp_dir = std::env::temp_dir();
            let backup_path = temp_dir.join(format!("xfastinstall_backup_{}", Uuid::new_v4()));
            fs::create_dir_all(&backup_path)
                .context("Failed to create backup directory")?;

            // Backup liveries
            if backup_liveries {
                let liveries_src = target.join("liveries");
                if liveries_src.exists() {
                    let liveries_dst = backup_path.join("liveries");
                    self.copy_directory_with_progress(&liveries_src, &liveries_dst, ctx)?;
                }
            }

            // Backup config files
            if backup_config_files {
                for pattern in config_file_patterns {
                    for entry in glob::glob(&target.join(pattern).to_string_lossy())
                        .context("Failed to read glob pattern")?
                    {
                        if let Ok(config_file) = entry {
                            if config_file.is_file() {
                                if let Some(file_name) = config_file.file_name() {
                                    let backup_file = backup_path.join(file_name);
                                    fs::copy(&config_file, &backup_file)
                                        .context(format!("Failed to backup config file: {:?}", config_file))?;
                                }
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
            remove_dir_all_robust(target)
                .context(format!("Failed to delete existing aircraft folder: {:?}", target))?;
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
            for entry in fs::read_dir(&backup_path)
                .context("Failed to read backup directory")?
            {
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
                fs::remove_dir_all(&backup_path)
                    .context("Failed to cleanup backup directory")?;
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
        let backup = self.backup_aircraft_data(target, backup_liveries, backup_config_files, config_patterns, ctx)?;

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
            match self.restore_aircraft_backup(backup_data, target, ctx) {
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
        ctx: &ProgressContext,
    ) -> Result<Option<AircraftBackup>> {
        if !target.exists() {
            return Ok(None);
        }

        // Update progress: Starting backup
        ctx.emit_progress(Some("Backing up aircraft data...".to_string()), InstallPhase::Installing);

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
                ctx.emit_progress(Some("Backing up liveries...".to_string()), InstallPhase::Installing);

                // Record original info for verification
                let original_info = self.get_directory_info(&liveries_src)?;
                backup.original_liveries_info = Some(original_info);

                let liveries_dst = temp_dir.join("liveries");
                self.copy_directory_with_progress(&liveries_src, &liveries_dst, ctx)
                    .context("Failed to backup liveries folder")?;
                backup.liveries_path = Some(liveries_dst);
            }
        }

        // Backup config files from root directory only if enabled
        if backup_config_files && !config_patterns.is_empty() {
            ctx.emit_progress(Some("Backing up config files...".to_string()), InstallPhase::Installing);

            // Pre-compile patterns once for efficiency
            let compiled = CompiledPatterns::new(config_patterns);

            for entry in fs::read_dir(target)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if compiled.matches(name) {
                            let original_size = fs::metadata(&path)?.len();
                            let backup_path = temp_dir.join(name);
                            fs::copy(&path, &backup_path)
                                .context(format!("Failed to backup {}", name))?;
                            backup.pref_files.push((name.to_string(), backup_path.clone()));
                            backup.original_pref_sizes.push((name.to_string(), original_size));

                            // Update progress for each config file with filename for real-time display
                            ctx.add_bytes(original_size);
                            ctx.emit_progress(Some(name.to_string()), InstallPhase::Installing);
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
    fn restore_aircraft_backup(&self, backup: &AircraftBackup, target: &Path, ctx: &ProgressContext) -> Result<()> {
        ctx.emit_progress(Some("Restoring backup...".to_string()), InstallPhase::Installing);

        // Restore liveries folder (skip existing - don't overwrite new content)
        if let Some(ref liveries_backup) = backup.liveries_path {
            ctx.emit_progress(Some("Restoring liveries...".to_string()), InstallPhase::Installing);

            let liveries_target = target.join("liveries");

            if liveries_target.exists() {
                // Merge: copy only files that don't exist in new content
                self.merge_directory_skip_existing_with_progress(liveries_backup, &liveries_target, ctx)?;
            } else {
                // No new liveries folder, restore entire backup
                self.copy_directory_with_progress(liveries_backup, &liveries_target, ctx)?;
            }
        }

        // Restore *_prefs.txt files (always overwrite - restore user preferences)
        if !backup.pref_files.is_empty() {
            ctx.emit_progress(Some("Restoring config files...".to_string()), InstallPhase::Installing);

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
    fn merge_directory_skip_existing_with_progress(&self, source: &Path, target: &Path, ctx: &ProgressContext) -> Result<()> {
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
            "zip" => self.extract_zip_with_progress(archive, target, internal_root, ctx, password)?,
            "7z" => self.extract_7z_with_progress(archive, target, internal_root, ctx, password)?,
            "rar" => self.extract_rar_with_progress(archive, target, internal_root, ctx, password)?,
            _ => return Err(anyhow::anyhow!("Unsupported archive format: {}", extension)),
        }

        crate::log_debug!(
            &format!("[TIMING] Archive extraction completed in {:.2}ms: {} format",
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
        let mut skipped_count = 0;
        let entries: Vec<_> = (0..archive.len())
            .filter_map(|i| {
                // Use by_index_raw to avoid triggering decryption errors when reading metadata
                let file = match archive.by_index_raw(i) {
                    Ok(f) => f,
                    Err(e) => {
                        logger::log_error(
                            &format!("Failed to read ZIP entry {}: {}", i, e),
                            Some("installer")
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
                            None
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
                                &format!("Skipping ZIP entry with unsafe path after sanitization: {}", stripped),
                                Some("installer"),
                                None
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
                                None
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
                Some("installer")
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
                    // Use by_index_raw to get metadata without triggering decryption
                    let file = archive.by_index_raw(*index)?;
                    if let Some(mode) = file.unix_mode() {
                        fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                    }
                }

                Ok(())
            })?;

        Ok(())
    }

    /// Extract 7z archive with progress tracking
    /// Extracts directly to target directory for better performance
    fn extract_7z_with_progress(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
    ) -> Result<()> {
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
            sevenz_rust2::SevenZReader::open(archive, sevenz_rust2::Password::from(pwd))
                .map_err(|e| anyhow::anyhow!("Failed to open 7z with password: {}", e))?
        } else {
            sevenz_rust2::SevenZReader::open(archive, sevenz_rust2::Password::empty())
                .map_err(|e| anyhow::anyhow!("Failed to open 7z: {}", e))?
        };

        // Extract directly to target with progress reporting
        reader.for_each_entries(|entry, entry_reader| {
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
                let mut file = std::fs::File::create(&dest_path)?;
                copy_file_optimized(entry_reader, &mut file)?;

                // Remove read-only attribute
                let _ = remove_readonly_attribute(&dest_path);

                // Report progress
                let file_size = entry.size() as u64;
                let file_name = sanitized
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                ctx.add_bytes(file_size);
                ctx.emit_progress(Some(file_name), InstallPhase::Installing);
            }
            Ok(true)
        }).map_err(|e| anyhow::anyhow!("Failed to extract 7z: {}", e))?;

        Ok(())
    }

    /// Extract 7z archive with hash calculation for verification
    /// This is called when we need to compute hashes during extraction
    #[allow(dead_code)]
    fn extract_7z_with_hash_calculation(
        &self,
        archive: &Path,
        target: &Path,
        internal_root: Option<&str>,
        ctx: &ProgressContext,
        password: Option<&str>,
        task: &mut InstallTask,
    ) -> Result<()> {
        use sha2::{Sha256, Digest};
        #[allow(unused_imports)]
        use std::io::Read;

        // Create secure temp directory using tempfile crate
        let temp_dir = tempfile::Builder::new()
            .prefix("xfastinstall_7z_")
            .tempdir()
            .context("Failed to create secure temp directory")?;

        let mut computed_hashes = std::collections::HashMap::new();

        // Extract with password if provided (must use for_each_entries for password support)
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

                    // Create file and compute hash while writing
                    let mut file = std::fs::File::create(&dest_path)?;
                    let mut hasher = Sha256::new();
                    let mut buffer = vec![0u8; IO_BUFFER_SIZE];

                    loop {
                        let bytes_read = reader.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        hasher.update(&buffer[..bytes_read]);
                        std::io::Write::write_all(&mut file, &buffer[..bytes_read])?;
                    }

                    // Store hash
                    let hash = format!("{:x}", hasher.finalize());
                    let relative_path = entry.name().replace('\\', "/");

                    // Apply internal_root filter
                    if let Some(root) = internal_root {
                        let root_normalized = root.replace('\\', "/");
                        if let Some(stripped) = relative_path.strip_prefix(&format!("{}/", root_normalized)) {
                            computed_hashes.insert(
                                stripped.to_string(),
                                crate::models::FileHash {
                                    path: stripped.to_string(),
                                    hash,
                                    algorithm: crate::models::HashAlgorithm::Sha256,
                                }
                            );
                        }
                    } else {
                        computed_hashes.insert(
                            relative_path.clone(),
                            crate::models::FileHash {
                                path: relative_path,
                                hash,
                                algorithm: crate::models::HashAlgorithm::Sha256,
                            }
                        );
                    }
                }
                Ok(true)
            }).map_err(|e| anyhow::anyhow!("Failed to extract 7z with password: {}", e))?;
        } else {
            // Use fast batch extraction for non-password archives
            sevenz_rust2::decompress_file(archive, temp_dir.path())
                .map_err(|e| anyhow::anyhow!("Failed to extract 7z: {}", e))?;

            // Compute hashes for all extracted files
            use walkdir::WalkDir;
            for entry in WalkDir::new(temp_dir.path()).follow_links(false) {
                let entry = entry?;
                if !entry.file_type().is_file() {
                    continue;
                }

                let file_path = entry.path();
                let relative = file_path.strip_prefix(temp_dir.path())?;
                let relative_str = relative.to_string_lossy().replace('\\', "/");

                // Compute SHA256
                let hash = self.compute_file_sha256(file_path)?;

                // Apply internal_root filter
                if let Some(root) = internal_root {
                    let root_normalized = root.replace('\\', "/");
                    if let Some(stripped) = relative_str.strip_prefix(&format!("{}/", root_normalized)) {
                        computed_hashes.insert(
                            stripped.to_string(),
                            crate::models::FileHash {
                                path: stripped.to_string(),
                                hash,
                                algorithm: crate::models::HashAlgorithm::Sha256,
                            }
                        );
                    }
                } else {
                    computed_hashes.insert(
                        relative_str.clone(),
                        crate::models::FileHash {
                            path: relative_str,
                            hash,
                            algorithm: crate::models::HashAlgorithm::Sha256,
                        }
                    );
                }
            }
        }

        // Store computed hashes in task
        if !computed_hashes.is_empty() {
            logger::log_info(
                &format!("Computed {} SHA256 hashes during 7z extraction", computed_hashes.len()),
                Some("installer")
            );
            task.file_hashes = Some(computed_hashes);
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

    /// Compute SHA256 hashes for all files in installed directory
    /// Used for 7z archives where hashes aren't available from metadata
    #[allow(dead_code)]
    fn compute_installed_file_hashes(&self, target_dir: &Path) -> Result<HashMap<String, crate::models::FileHash>> {
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
                }
            );
        }

        Ok(hashes)
    }

    /// Compute SHA256 hash of a file
    fn compute_file_sha256(&self, path: &Path) -> Result<String> {
        use sha2::{Sha256, Digest};
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

    /// Cleanup a task by removing its target directory
    /// Used when a task is cancelled or skipped
    fn cleanup_task(&self, task: &InstallTask) -> Result<()> {
        let target = Path::new(&task.target_path);

        if !target.exists() {
            return Ok(());
        }

        logger::log_info(
            &format!("Cleaning up task: {}", task.display_name),
            Some("installer")
        );

        // For Navdata, we should NOT delete the entire Custom Data folder
        // Just log a warning
        if matches!(task.addon_type, AddonType::Navdata) {
            logger::log_info(
                "Navdata cleanup skipped - Custom Data folder preserved",
                Some("installer")
            );
            return Ok(());
        }

        // For other types, delete the target directory
        remove_dir_all_robust(target)
            .context(format!("Failed to cleanup task directory: {:?}", target))?;

        logger::log_info(
            &format!("Cleanup completed: {}", task.display_name),
            Some("installer")
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

        // Create atomic installer with X-Plane root and progress context
        let mut atomic = AtomicInstaller::new(
            target,
            xplane_root,
            self.app_handle.clone(),
            ctx.total_tasks,
            ctx.current_task_index,
        )?;

        // Step 1: Extract/copy to temp directory
        logger::log_info(
            &format!("Atomic install: Extracting to temp directory: {:?}", atomic.temp_dir()),
            Some("installer")
        );

        self.install_content_with_progress(
            source,
            atomic.temp_dir(),
            task.archive_internal_root.as_deref(),
            ctx,
            password
        )?;

        // Step 2: Perform atomic installation based on scenario
        if !target.exists() {
            // Scenario 1: Fresh installation
            atomic.install_fresh()?;
        } else if !task.should_overwrite {
            // Scenario 2: Clean installation (should_overwrite=false means clean install)
            atomic.install_clean(task)?;
        } else {
            // Scenario 3: Overwrite installation (should_overwrite=true means merge)
            atomic.install_overwrite()?;
        }

        logger::log_info(
            "Atomic installation completed successfully",
            Some("installer")
        );

        Ok(())
    }

    /// Delete source file after successful installation
    /// Checks if the source path is a parent directory of the original input path
    /// to avoid deleting directories that contain the detected addon
    fn delete_source_file(&self, original_input_path: &str, source_path: &str) -> Result<()> {
        let original_path = Path::new(original_input_path);
        let source_path_buf = Path::new(source_path);

        // Check if source_path is a parent (or ancestor) of original_input_path
        // This happens when we drag a subdirectory but the addon root is detected higher up
        // Example: drag "PLUGIN/win_x64" but addon root is "PLUGIN"
        if source_path_buf.starts_with(original_path) && source_path_buf != original_path {
            logger::log_info(
                &format!("Skipping deletion: detected addon root ({}) is a parent of input path ({})",
                    source_path, original_input_path),
                Some("installer"),
            );

            // Emit a notification to the frontend
            if let Err(e) = self.app_handle.emit("source-deletion-skipped", original_input_path) {
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
            fs::remove_file(original_path)
                .with_context(|| format!("Failed to delete source file: {}", original_input_path))?;
        } else if original_path.is_dir() {
            logger::log_info(
                &format!("Deleting source directory: {}", original_input_path),
                Some("installer"),
            );
            remove_dir_all_robust(original_path)
                .with_context(|| format!("Failed to delete source directory: {}", original_input_path))?;
        } else {
            logger::log_error(
                &format!("Source path does not exist or is not accessible: {}", original_input_path),
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

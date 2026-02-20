use anyhow::{Context, Result};
use glob::Pattern;
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use sea_orm::DatabaseConnection;
use tauri::{AppHandle, Emitter, Manager};

use crate::logger;
use crate::logger::{tr, LogMsg};
use crate::models::{
    AddonType, InstallPhase, InstallProgress, InstallResult, InstallTask, TaskResult,
};
use crate::task_control::TaskControl;

mod extraction;
mod handlers;
mod verification;

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

/// Generate a fixed-length folder name from a provider name using SHA-256.
/// Produces a 16-character hex string (first 8 bytes of hash) that is
/// deterministic: the same provider name always yields the same result.
pub fn sanitize_folder_name(name: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(name.as_bytes());
    // Take first 8 bytes → 16 hex chars, enough to avoid collisions in practice
    hash.iter().take(8).map(|b| format!("{:02x}", b)).collect()
}

/// Sanitize a file path to prevent path traversal attacks
/// Returns None if the path is unsafe (contains `..` or is absolute)
pub fn sanitize_path(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(c) => result.push(c),
            Component::CurDir => {}              // Skip "."
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
#[allow(clippy::permissions_set_readonly_false)]
fn remove_readonly_attribute(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();

    // Check if file is read-only
    if permissions.readonly() {
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions).context(format!(
            "Failed to remove read-only attribute from: {:?}",
            path
        ))?;
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
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        let entry_path = entry.path();
        if entry_path.is_file() {
            // Try to remove read-only attribute, but don't fail if it doesn't work
            let _ = remove_readonly_attribute(entry_path);
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
    let e = last_error
        .unwrap_or_else(|| std::io::Error::other("Unknown error during directory removal"));
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

struct AircraftInstallOptions<'a> {
    backup_liveries: bool,
    backup_config_files: bool,
    config_patterns: &'a [String],
}

struct AircraftExtractionInstallParams<'a> {
    source: &'a Path,
    target: &'a Path,
    chain: &'a crate::models::ExtractionChain,
    ctx: &'a ProgressContext,
    password: Option<&'a str>,
    options: AircraftInstallOptions<'a>,
}

struct AircraftProgressInstallParams<'a> {
    source: &'a Path,
    target: &'a Path,
    internal_root: Option<&'a str>,
    ctx: &'a ProgressContext,
    password: Option<&'a str>,
    options: AircraftInstallOptions<'a>,
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
    /// Size of each task in bytes (for proportional progress calculation)
    task_sizes: Arc<Vec<u64>>,
    /// Cumulative bytes at the start of each task
    task_cumulative: Arc<Vec<u64>>,
    /// Maximum percentage reached, stored as percentage * 100 for atomic ops
    /// Used to ensure progress never goes backward during task transitions
    max_percentage: Arc<AtomicU64>,
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
            task_sizes: Arc::new(Vec::new()),
            task_cumulative: Arc::new(Vec::new()),
            max_percentage: Arc::new(AtomicU64::new(0)),
        }
    }

    fn set_total_bytes(&self, total: u64) {
        self.total_bytes.store(total, Ordering::SeqCst);
    }

    /// Set task sizes and compute cumulative bytes for each task
    fn set_task_sizes(&mut self, sizes: Vec<u64>) {
        // Calculate cumulative bytes at the start of each task
        let mut cumulative = Vec::with_capacity(sizes.len());
        let mut sum = 0u64;
        for size in &sizes {
            cumulative.push(sum);
            sum += size;
        }
        self.task_sizes = Arc::new(sizes);
        self.task_cumulative = Arc::new(cumulative);
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
                    Some("installer"),
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

        // Calculate percentage based on phase
        // Each task gets a proportional share of 0-100% based on its size
        // Within each task: 90% for installation, 10% for verification
        let (raw_percentage, verification_progress) = match phase {
            InstallPhase::Finalizing => (100.0, None),
            InstallPhase::Verifying => {
                let verify_progress = self.get_verification_progress();
                let total_f = total as f64;
                if total_f == 0.0 {
                    return;
                }

                // Get cumulative bytes at start of current task
                let cumulative = self
                    .task_cumulative
                    .get(self.current_task_index)
                    .copied()
                    .unwrap_or(0) as f64;
                let base_pct = (cumulative / total_f) * 100.0;

                // Get current task's size and its contribution to total progress
                let task_size = self
                    .task_sizes
                    .get(self.current_task_index)
                    .copied()
                    .unwrap_or(0) as f64;
                let task_pct = (task_size / total_f) * 100.0;

                // Installation part is complete (90% of task's share), verification in progress (10%)
                let install_pct = task_pct * 0.9;
                let verify_pct = (verify_progress / 100.0) * (task_pct * 0.1);

                (base_pct + install_pct + verify_pct, Some(verify_progress))
            }
            _ => {
                // Calculating/Installing phase
                let total_f = total as f64;
                if total_f == 0.0 {
                    return;
                }

                // Get cumulative bytes at start of current task
                let cumulative = self
                    .task_cumulative
                    .get(self.current_task_index)
                    .copied()
                    .unwrap_or(0) as f64;
                let base_pct = (cumulative / total_f) * 100.0;

                // Get current task's size
                let task_size = self
                    .task_sizes
                    .get(self.current_task_index)
                    .copied()
                    .unwrap_or(1) as f64;
                let task_pct = (task_size / total_f) * 100.0;

                // Calculate progress within current task (only 90% for installation phase)
                let current_processed = ((processed as f64) - cumulative).max(0.0);
                let task_progress = (current_processed / task_size).min(1.0);
                let install_pct = task_progress * (task_pct * 0.9);

                (base_pct + install_pct, None)
            }
        };

        // Ensure progress never goes backward by tracking max percentage
        // This prevents jumps when transitioning between tasks
        let stored_max = self.max_percentage.load(Ordering::SeqCst) as f64 / 100.0;
        let percentage = raw_percentage.max(stored_max);
        // Update max_percentage if current is higher
        let new_max = (percentage * 100.0) as u64;
        self.max_percentage.fetch_max(new_max, Ordering::SeqCst);

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
    db: DatabaseConnection,
}

impl Installer {
    pub fn new(app_handle: AppHandle) -> Self {
        // Get TaskControl from app state
        let task_control = app_handle.state::<TaskControl>().inner().clone();
        let db = app_handle.state::<DatabaseConnection>().inner().clone();
        Installer {
            app_handle,
            task_control,
            db,
        }
    }

    /// Install a list of tasks with progress reporting
    pub async fn install(
        &self,
        tasks: Vec<InstallTask>,
        atomic_install_enabled: bool,
        xplane_path: String,
        delete_source_after_install: bool,
        auto_sort_scenery: bool,
    ) -> Result<InstallResult> {
        let install_start = Instant::now();
        crate::log_debug!(
            &format!(
                "[TIMING] Installation started: {} tasks (atomic: {})",
                tasks.len(),
                atomic_install_enabled
            ),
            "installer_timing"
        );

        logger::log_info(
            &format!(
                "{}: {} task(s) (atomic mode: {})",
                tr(LogMsg::InstallationStarted),
                tasks.len(),
                atomic_install_enabled
            ),
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
        crate::log_debug!("[TIMING] Size calculation started", "installer_timing");
        ctx.emit_progress(None, InstallPhase::Calculating);
        let (total_size, task_sizes) = self.calculate_total_size(&tasks)?;
        ctx.set_total_bytes(total_size);
        ctx.set_task_sizes(task_sizes);
        crate::log_debug!(
            &format!(
                "[TIMING] Size calculation completed in {:.2}ms: {} bytes ({:.2} MB)",
                calc_start.elapsed().as_secs_f64() * 1000.0,
                total_size,
                total_size as f64 / (1024.0 * 1024.0)
            ),
            "installer_timing"
        );

        // Phase 2: Install each task
        let install_phase_start = Instant::now();
        crate::log_debug!("[TIMING] Installation phase started", "installer_timing");

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
                &format!(
                    "{}: {} -> {}",
                    tr(LogMsg::Installing),
                    task.display_name,
                    task.target_path
                ),
                Some("installer"),
            );

            // Track target path for potential cleanup
            self.task_control
                .add_processed_path(PathBuf::from(&task.target_path));

            match self.install_task_with_progress(task, &ctx, atomic_install_enabled, &xplane_path)
            {
                Ok(_) => {
                    // Check for skip request after installation but before verification
                    if self.task_control.is_skip_requested() {
                        logger::log_info(
                            &format!("Task skipped by user: {}", task.display_name),
                            Some("installer"),
                        );

                        // Cleanup the installed files
                        if let Err(e) = self.cleanup_task(task) {
                            logger::log_error(
                                &format!("Failed to cleanup skipped task: {}", e),
                                Some("installer"),
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
                        &format!(
                            "[TIMING] Task {} installation completed in {:.2}ms: {}",
                            index + 1,
                            task_start.elapsed().as_secs_f64() * 1000.0,
                            task.display_name
                        ),
                        "installer_timing"
                    );

                    // Verify installation by checking for typical files
                    let verify_start = Instant::now();
                    crate::log_debug!(
                        &format!(
                            "[TIMING] Task {} verification started: {}",
                            index + 1,
                            task.display_name
                        ),
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
                                &format!(
                                    "[TIMING] Task {} total time: {:.2}ms: {}",
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
                                &format!(
                                    "{}: {}",
                                    tr(LogMsg::InstallationCompleted),
                                    task.display_name
                                ),
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
                                    if let Err(e) =
                                        self.delete_source_file(original_path, &task.source_path)
                                    {
                                        logger::log_error(
                                            &format!(
                                                "Failed to delete source file {}: {}",
                                                original_path, e
                                            ),
                                            Some("installer"),
                                        );
                                    }
                                }
                            }

                            // Auto-sort scenery if enabled and this is a scenery task
                            if auto_sort_scenery
                                && (task.addon_type == AddonType::Scenery
                                    || task.addon_type == AddonType::SceneryLibrary)
                            {
                                use crate::scenery_classifier::classify_scenery;
                                use crate::scenery_packs_manager::SceneryPacksManager;
                                use std::path::Path;

                                // Extract folder name from target path
                                let target_path = Path::new(&task.target_path);
                                if let Some(folder_name) =
                                    target_path.file_name().and_then(|n| n.to_str())
                                {
                                    // Classify the newly installed scenery
                                    let xplane_path_buf = PathBuf::from(&xplane_path);
                                    match classify_scenery(target_path, &xplane_path_buf) {
                                        Ok(scenery_info) => {
                                            // Add entry to scenery_packs.ini at correct position
                                            let manager = SceneryPacksManager::new(
                                                &xplane_path_buf,
                                                self.db.clone(),
                                            );
                                            if let Err(e) = manager
                                                .add_entry(folder_name, &scenery_info.category)
                                                .await
                                            {
                                                logger::log_error(
                                                    &format!("Failed to add scenery to scenery_packs.ini: {}", e),
                                                    Some("installer"),
                                                );
                                            } else {
                                                logger::log_info(
                                                    &format!("Added {} to scenery_packs.ini (category: {:?})", folder_name, scenery_info.category),
                                                    Some("installer"),
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            logger::log_error(
                                                &format!(
                                                    "Failed to classify scenery {}: {}",
                                                    folder_name, e
                                                ),
                                                Some("installer"),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(verify_err) => {
                            crate::log_debug!(
                                &format!(
                                    "[TIMING] Task {} verification failed in {:.2}ms: {} - {}",
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
                                &format!(
                                    "{} {}: {}",
                                    tr(LogMsg::InstallationFailed),
                                    task.display_name,
                                    error_msg
                                ),
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
                        &format!(
                            "[TIMING] Task {} installation failed in {:.2}ms: {} - {}",
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
                        &format!(
                            "{} {}: {}",
                            tr(LogMsg::InstallationFailed),
                            task.display_name,
                            error_msg
                        ),
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
            &format!(
                "[TIMING] Finalization completed in {:.2}ms",
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
    /// Returns (total_size, per_task_sizes) for proportional progress calculation
    /// Includes extra size for backup/restore operations during clean install
    fn calculate_total_size(&self, tasks: &[InstallTask]) -> Result<(u64, Vec<u64>)> {
        let mut total = 0u64;
        let mut task_sizes = Vec::with_capacity(tasks.len());

        for task in tasks {
            let mut task_size = 0u64;
            let source = Path::new(&task.source_path);
            let target = Path::new(&task.target_path);

            // Add source size (archive or directory)
            if source.is_dir() {
                task_size += self.get_directory_size(source)?;
            } else if source.is_file() {
                task_size +=
                    self.get_archive_size(source, task.archive_internal_root.as_deref())?;
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
                                let liveries_size =
                                    self.get_directory_size(&liveries_path).unwrap_or(0);
                                task_size += liveries_size * 2; // backup + restore
                            }
                        }

                        // Backup and restore config files (2x: backup + restore)
                        if task.backup_config_files {
                            let config_size =
                                self.get_config_files_size(target, &task.config_file_patterns);
                            task_size += config_size * 2; // backup + restore
                        }
                    }
                    _ => {
                        // Other addon types don't have backup/restore overhead
                    }
                }
            }

            task_sizes.push(task_size);
            total += task_size;
        }
        Ok((total, task_sizes))
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
        for e in arch.flatten() {
            total += e.unpacked_size;
        }
        Ok(total)
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

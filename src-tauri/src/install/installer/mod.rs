use anyhow::{Context, Result};
use glob::Pattern;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

use crate::database::DatabaseState;
use crate::logger;
use crate::logger::{tr, LogMsg};
use crate::models::{
    AddonType, InstallPhase, InstallProgress, InstallResult, InstallTask, ParallelTaskProgress,
    TaskResult,
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

/// Buffer size for file I/O operations (8 MB)
/// Optimized for modern SSDs and network storage
const IO_BUFFER_SIZE: usize = 8 * 1024 * 1024;

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
/// Uses a larger buffer (8MB) for faster I/O operations
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

/// Copy file while computing CRC32 hash inline
/// Returns (bytes_written, crc32_hash) for inline verification
fn copy_file_with_crc32<R: std::io::Read + ?Sized, W: std::io::Write>(
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<(u64, u32)> {
    use crc32fast::Hasher;
    let mut buffer = vec![0u8; IO_BUFFER_SIZE];
    let mut total_bytes = 0u64;
    let mut hasher = Hasher::new();
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
        writer.write_all(&buffer[..bytes_read])?;
        total_bytes += bytes_read as u64;
    }
    Ok((total_bytes, hasher.finalize()))
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
    /// Whether all files were verified inline during extraction (skip post-extraction verification)
    inline_verified: Arc<std::sync::atomic::AtomicBool>,
    /// Inline verification stats: total files verified inline
    inline_verified_count: Arc<AtomicU64>,
    /// Hashes computed inline during extraction (for 7z SHA256)
    inline_hashes: Arc<Mutex<HashMap<String, crate::models::FileHash>>>,
    /// Optional parallel emit callback: when set, emit_progress updates tracker data
    /// and calls this instead of emitting directly
    parallel_emit: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Optional tracker current_file reference for parallel mode
    parallel_current_file: Option<Arc<Mutex<Option<String>>>>,
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
            inline_verified: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            inline_verified_count: Arc::new(AtomicU64::new(0)),
            inline_hashes: Arc::new(Mutex::new(HashMap::new())),
            parallel_emit: None,
            parallel_current_file: None,
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
        self.emit_progress_internal(current_file, phase, false);
    }

    fn emit_progress_force(&self, current_file: Option<String>, phase: InstallPhase) {
        self.emit_progress_internal(current_file, phase, true);
    }

    fn emit_progress_internal(
        &self,
        current_file: Option<String>,
        phase: InstallPhase,
        force: bool,
    ) {
        // In parallel mode, update tracker data and delegate to parent's emit_aggregated
        if let Some(ref emit_fn) = self.parallel_emit {
            // Update the tracker's current_file
            if let Some(ref cf) = self.parallel_current_file {
                if let Ok(mut f) = cf.lock() {
                    *f = current_file;
                }
            }
            emit_fn();
            return;
        }

        // Throttle: emit at most every 16ms (60fps for smooth animation), unless force=true
        if !force {
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
        } else {
            // Force mode: update last_emit time but don't check throttle
            if let Ok(mut last) = self.last_emit.lock() {
                *last = Instant::now();
            }
        }

        let total = self.total_bytes.load(Ordering::SeqCst);
        let processed = self.processed_bytes.load(Ordering::SeqCst);

        // Get current task's size and cumulative bytes
        let task_size = self
            .task_sizes
            .get(self.current_task_index)
            .copied()
            .unwrap_or(0);
        let cumulative = self
            .task_cumulative
            .get(self.current_task_index)
            .copied()
            .unwrap_or(0);

        // Calculate current task's processed bytes
        let current_task_processed = processed.saturating_sub(cumulative).min(task_size);

        // Calculate percentage based on phase
        // Each task gets a proportional share of 0-100% based on its size
        let (raw_percentage, verification_progress, current_task_percentage) = match phase {
            InstallPhase::Finalizing => (100.0, None, 100.0),
            InstallPhase::Verifying => {
                let verify_progress = self.get_verification_progress();
                let total_f = total as f64;
                if total_f == 0.0 {
                    return;
                }

                // Use task-proportional percentage (same formula as installing phase at 100%)
                // This prevents the total percentage from exceeding the current task's
                // proportional share when actual extracted bytes exceed estimated bytes
                let cumulative_v = self
                    .task_cumulative
                    .get(self.current_task_index)
                    .copied()
                    .unwrap_or(0) as f64;
                let base_pct = (cumulative_v / total_f) * 100.0;
                let task_size_v = self
                    .task_sizes
                    .get(self.current_task_index)
                    .copied()
                    .unwrap_or(1) as f64;
                let task_pct = (task_size_v / total_f) * 100.0;

                // Task extraction is complete during verification, so use full task proportion
                let total_pct = base_pct + task_pct;

                // Current task percentage: 100% (verification doesn't affect task progress display)
                let task_progress = 100.0;

                (total_pct, Some(verify_progress), task_progress)
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

                // Calculate progress within current task
                let current_processed = ((processed as f64) - cumulative).max(0.0);
                let task_progress = (current_processed / task_size).min(1.0);
                let install_pct = task_progress * task_pct;

                // Current task percentage
                let task_progress_pct = task_progress * 100.0;

                (base_pct + install_pct, None, task_progress_pct)
            }
        };

        // Cap percentage at 99.9% - only emit_final should set 100%
        // This prevents premature 100% display when actual bytes exceed estimates
        let raw_percentage = raw_percentage.min(99.9);

        // Ensure progress never goes backward by tracking max percentage
        // This prevents jumps when transitioning between tasks
        let stored_max = self.max_percentage.load(Ordering::SeqCst) as f64 / 100.0;
        let percentage = raw_percentage.max(stored_max);
        // Update max_percentage if current is higher
        let new_max = (percentage * 100.0) as u64;
        self.max_percentage.fetch_max(new_max, Ordering::SeqCst);

        // Debug log for serial mode progress (before creating progress struct to avoid borrow issues)
        if force {
            crate::log_debug!(
                &format!(
                    "[PROGRESS] Force emit: task {}/{}, phase: {:?}, percentage: {:.1}%",
                    self.current_task_index + 1,
                    self.total_tasks,
                    &phase,
                    percentage
                ),
                "installer_progress"
            );
        }

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
            current_task_percentage,
            current_task_total_bytes: task_size,
            current_task_processed_bytes: current_task_processed,
            active_tasks: None,
            // In serial mode, provide completed count as current task index
            // This helps frontend show completed tasks more reliably
            completed_task_count: Some(self.current_task_index),
            completed_task_ids: None,
        };

        let _ = self.app_handle.emit("install-progress", &progress);
    }

    fn emit_final(&self, phase: InstallPhase) {
        // In parallel mode, delegate to parent's emit_aggregated instead of emitting directly
        if let Some(ref emit_fn) = self.parallel_emit {
            if let Some(ref cf) = self.parallel_current_file {
                if let Ok(mut f) = cf.lock() {
                    *f = None;
                }
            }
            emit_fn();
            return;
        }

        let total = self.total_bytes.load(Ordering::SeqCst);
        let processed = self.processed_bytes.load(Ordering::SeqCst);

        // Get current task's size for final emission
        let task_size = self
            .task_sizes
            .get(self.current_task_index)
            .copied()
            .unwrap_or(0);

        // For serial mode: set current_task_index to total_tasks to indicate all tasks completed
        // This ensures the frontend's "index < currentTaskIndex" check marks all tasks as completed
        let final_task_index = if self.parallel_emit.is_none() {
            self.total_tasks // Serial mode: all tasks done, index points beyond last task
        } else {
            self.current_task_index // Parallel mode: keep original index
        };

        // Final progress is always 100%
        let progress = InstallProgress {
            percentage: 100.0,
            total_bytes: total,
            processed_bytes: processed,
            current_task_index: final_task_index,
            total_tasks: self.total_tasks,
            current_task_name: self.current_task_name.clone(),
            current_file: None,
            phase,
            verification_progress: None,
            current_task_percentage: 100.0,
            current_task_total_bytes: task_size,
            current_task_processed_bytes: task_size,
            active_tasks: None,
            completed_task_count: Some(self.total_tasks), // All tasks completed
            completed_task_ids: None,
        };

        let _ = self.app_handle.emit("install-progress", &progress);
    }
}

// ========== Parallel Installation Support ==========

/// Per-task progress tracker for parallel installation
struct TaskTracker {
    index: usize,
    id: String,
    name: String,
    total_bytes: u64,
    processed_bytes: Arc<AtomicU64>,
    /// 0=waiting, 1=installing, 2=verifying, 3=done, 4=failed
    phase: std::sync::atomic::AtomicU8,
    verification_progress: Arc<AtomicU64>,
    current_file: Arc<Mutex<Option<String>>>,
    inline_verified: Arc<std::sync::atomic::AtomicBool>,
    inline_verified_count: Arc<AtomicU64>,
    inline_hashes: Arc<Mutex<HashMap<String, crate::models::FileHash>>>,
}

/// Aggregated parallel progress context
#[allow(dead_code)]
struct ParallelProgressContext {
    app_handle: AppHandle,
    total_bytes: u64,
    total_tasks: usize,
    trackers: Vec<Arc<TaskTracker>>,
    completed_count: AtomicU64,
    last_emit: Arc<Mutex<Instant>>,
    /// Maximum percentage reached, prevents progress from going backward
    max_percentage: AtomicU64,
}

impl ParallelProgressContext {
    fn new(
        app_handle: AppHandle,
        total_bytes: u64,
        total_tasks: usize,
        task_sizes: &[u64],
        tasks: &[InstallTask],
    ) -> Self {
        let trackers = tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                Arc::new(TaskTracker {
                    index: i,
                    id: task.id.clone(),
                    name: task.display_name.clone(),
                    total_bytes: task_sizes.get(i).copied().unwrap_or(0),
                    processed_bytes: Arc::new(AtomicU64::new(0)),
                    phase: std::sync::atomic::AtomicU8::new(0),
                    verification_progress: Arc::new(AtomicU64::new(0)),
                    current_file: Arc::new(Mutex::new(None)),
                    inline_verified: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    inline_verified_count: Arc::new(AtomicU64::new(0)),
                    inline_hashes: Arc::new(Mutex::new(HashMap::new())),
                })
            })
            .collect();

        ParallelProgressContext {
            app_handle,
            total_bytes,
            total_tasks,
            trackers,
            completed_count: AtomicU64::new(0),
            last_emit: Arc::new(Mutex::new(Instant::now())),
            max_percentage: AtomicU64::new(0),
        }
    }

    fn get_task_view(self: &Arc<Self>, index: usize) -> TaskProgressView {
        TaskProgressView {
            tracker: Arc::clone(&self.trackers[index]),
            parent: Arc::clone(self),
        }
    }

    fn mark_completed(&self, index: usize) {
        self.trackers[index]
            .phase
            .store(3, std::sync::atomic::Ordering::SeqCst);
        let new_count = self.completed_count.fetch_add(1, Ordering::SeqCst) + 1;
        crate::log_debug!(
            &format!(
                "[PARALLEL] Task {} ({}) marked COMPLETED, completed_count: {}/{}",
                index, self.trackers[index].name, new_count, self.total_tasks
            ),
            "parallel_progress"
        );
    }

    fn mark_failed(&self, index: usize) {
        self.trackers[index]
            .phase
            .store(4, std::sync::atomic::Ordering::SeqCst);
        crate::log_debug!(
            &format!(
                "[PARALLEL] Task {} ({}) marked FAILED",
                index, self.trackers[index].name
            ),
            "parallel_progress"
        );
    }

    fn emit_aggregated(&self) {
        // Throttle: emit at most every 16ms
        let mut last = match self.last_emit.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let now = Instant::now();
        if now.duration_since(*last).as_millis() < 16 {
            return;
        }
        *last = now;
        drop(last);

        let total_bytes = self.total_bytes;
        let mut total_processed = 0u64;
        let mut active_tasks = Vec::new();
        let mut completed_task_ids = Vec::new();
        let completed = self.completed_count.load(Ordering::SeqCst) as usize;

        // Find the first active task to use as "current" for backwards-compat fields
        let mut first_active_index = 0usize;
        let mut first_active_name = String::new();
        let mut first_active_file: Option<String> = None;
        let mut found_active = false;

        for tracker in &self.trackers {
            let processed = tracker.processed_bytes.load(Ordering::SeqCst);

            let phase_val = tracker.phase.load(std::sync::atomic::Ordering::SeqCst);
            if phase_val == 1 || phase_val == 2 {
                // installing or verifying
                let task_total = tracker.total_bytes;

                // For overall percentage: if verifying/done, extraction is complete
                // so count full task size as processed to avoid percentage stalling
                if phase_val == 2 {
                    total_processed += task_total;
                } else {
                    total_processed += processed;
                }

                let pct = if phase_val == 2 {
                    // Extraction is fully complete during verification phase.
                    // Always show 100% to avoid percentage dropping backward.
                    // The "Verifying" phase label already communicates the current activity.
                    100.0
                } else if task_total > 0 {
                    (processed as f64 / task_total as f64 * 100.0).min(100.0)
                } else {
                    0.0
                };

                let current_file = tracker.current_file.lock().ok().and_then(|f| f.clone());

                let phase = if phase_val == 2 {
                    InstallPhase::Verifying
                } else {
                    InstallPhase::Installing
                };

                active_tasks.push(ParallelTaskProgress {
                    task_id: tracker.id.clone(),
                    task_index: tracker.index,
                    task_name: tracker.name.clone(),
                    phase,
                    percentage: pct,
                    current_file: current_file.clone(),
                });

                if !found_active {
                    first_active_index = tracker.index;
                    first_active_name = tracker.name.clone();
                    first_active_file = current_file;
                    found_active = true;
                }
            } else if phase_val == 3 {
                // Task completed successfully — count full size as processed
                total_processed += tracker.total_bytes;
                completed_task_ids.push(tracker.id.clone());
            } else if phase_val == 4 {
                // Task failed — still count processed bytes for overall
                total_processed += tracker.processed_bytes.load(Ordering::SeqCst);
            }
        }

        let raw_percentage = if total_bytes > 0 {
            (total_processed as f64 / total_bytes as f64 * 100.0).min(99.9)
        } else {
            0.0
        };

        // Ensure progress never goes backward
        let stored_max = self.max_percentage.load(Ordering::SeqCst) as f64 / 100.0;
        let percentage = raw_percentage.max(stored_max);
        let new_max = (percentage * 100.0) as u64;
        self.max_percentage.fetch_max(new_max, Ordering::SeqCst);

        // Current task fields for backwards compatibility
        let current_task_percentage = if !active_tasks.is_empty() {
            active_tasks[0].percentage
        } else {
            0.0
        };

        crate::log_debug!(
            &format!(
                "[PARALLEL] emit_aggregated: overall {:.1}%, active_tasks: [{}], completed: {}/{}, completed_ids: [{}]",
                percentage,
                active_tasks.iter().map(|t| format!("{}({})={:.1}%/{:?}", t.task_index, t.task_name, t.percentage, t.phase)).collect::<Vec<_>>().join(", "),
                completed,
                self.total_tasks,
                completed_task_ids.join(", ")
            ),
            "parallel_progress"
        );

        let progress = InstallProgress {
            percentage,
            total_bytes,
            processed_bytes: total_processed,
            current_task_index: first_active_index,
            total_tasks: self.total_tasks,
            current_task_name: first_active_name,
            current_file: first_active_file,
            phase: InstallPhase::Installing,
            verification_progress: None,
            current_task_percentage,
            current_task_total_bytes: 0,
            current_task_processed_bytes: 0,
            active_tasks: Some(active_tasks),
            completed_task_count: Some(completed),
            completed_task_ids: Some(completed_task_ids),
        };

        let _ = self.app_handle.emit("install-progress", &progress);
    }

    fn emit_final(&self) {
        let completed = self.completed_count.load(Ordering::SeqCst) as usize;

        let completed_task_ids: Vec<String> = self
            .trackers
            .iter()
            .filter(|t| t.phase.load(std::sync::atomic::Ordering::SeqCst) == 3)
            .map(|t| t.id.clone())
            .collect();

        crate::log_debug!(
            &format!(
                "[PARALLEL] emit_final: completed={}/{}, completed_ids=[{}]",
                completed,
                self.total_tasks,
                completed_task_ids.join(", ")
            ),
            "parallel_progress"
        );

        let progress = InstallProgress {
            percentage: 100.0,
            total_bytes: self.total_bytes,
            processed_bytes: self.total_bytes,
            current_task_index: 0,
            total_tasks: self.total_tasks,
            current_task_name: String::new(),
            current_file: None,
            phase: InstallPhase::Finalizing,
            verification_progress: None,
            current_task_percentage: 100.0,
            current_task_total_bytes: 0,
            current_task_processed_bytes: 0,
            active_tasks: Some(Vec::new()),
            completed_task_count: Some(completed),
            completed_task_ids: Some(completed_task_ids),
        };

        let _ = self.app_handle.emit("install-progress", &progress);
    }
}

/// A view into a single task's progress within a parallel installation.
/// Implements the same interface as ProgressContext so existing extraction
/// code can use it without changes.
#[allow(dead_code)]
struct TaskProgressView {
    tracker: Arc<TaskTracker>,
    parent: Arc<ParallelProgressContext>,
}

#[allow(dead_code)]
impl TaskProgressView {
    fn add_bytes(&self, bytes: u64) {
        self.tracker
            .processed_bytes
            .fetch_add(bytes, Ordering::SeqCst);
    }

    fn emit_progress(&self, current_file: Option<String>, _phase: InstallPhase) {
        if let Ok(mut f) = self.tracker.current_file.lock() {
            *f = current_file;
        }
        self.parent.emit_aggregated();
    }

    fn set_verification_progress(&self, progress: f64) {
        let stored = (progress * 100.0) as u64;
        self.tracker
            .verification_progress
            .store(stored, Ordering::SeqCst);
    }

    fn get_verification_progress(&self) -> f64 {
        let stored = self.tracker.verification_progress.load(Ordering::SeqCst);
        stored as f64 / 100.0
    }

    /// Create a ProgressContext that wraps this task view for use with existing code.
    /// The returned ProgressContext routes add_bytes through shared atomics and
    /// emit_progress through the parent's emit_aggregated (no direct event emission).
    fn as_progress_context(&self) -> ProgressContext {
        // Create a ProgressContext that shares this tracker's atomics
        let mut ctx = ProgressContext::new(self.parent.app_handle.clone(), self.parent.total_tasks);

        // Override the shared atomics with this task's tracker values
        ctx.total_bytes = Arc::new(AtomicU64::new(self.tracker.total_bytes));
        ctx.processed_bytes = Arc::clone(&self.tracker.processed_bytes);
        ctx.current_task_index = self.tracker.index;
        ctx.current_task_name = self.tracker.name.clone();
        ctx.verification_progress = Arc::clone(&self.tracker.verification_progress);
        ctx.inline_verified = Arc::clone(&self.tracker.inline_verified);
        ctx.inline_verified_count = Arc::clone(&self.tracker.inline_verified_count);
        ctx.inline_hashes = Arc::clone(&self.tracker.inline_hashes);

        // Set up task sizes so percentage calculations work correctly
        let mut sizes = vec![0u64; self.parent.total_tasks];
        sizes[self.tracker.index] = self.tracker.total_bytes;
        ctx.task_sizes = Arc::new(sizes);

        let mut cumulative = vec![0u64; self.parent.total_tasks];
        cumulative[self.tracker.index] = 0;
        ctx.task_cumulative = Arc::new(cumulative);

        // Share the parent's throttle
        ctx.last_emit = Arc::clone(&self.parent.last_emit);

        // Set up parallel emit: delegate to parent's emit_aggregated instead of emitting directly
        let parent_clone = Arc::clone(&self.parent);
        ctx.parallel_emit = Some(Arc::new(move || {
            parent_clone.emit_aggregated();
        }));
        ctx.parallel_current_file = Some(Arc::clone(&self.tracker.current_file));

        ctx
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
        let db = app_handle.state::<DatabaseState>().get();
        Installer {
            app_handle,
            task_control,
            db,
        }
    }

    /// Install a list of tasks with progress reporting
    pub async fn install(
        &self,
        mut tasks: Vec<InstallTask>,
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
        // Force emit at start of installation to ensure frontend gets initial state
        ctx.emit_progress_force(None, InstallPhase::Calculating);
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

        for (index, task) in tasks.iter_mut().enumerate() {
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

            // Reset processed bytes to the cumulative baseline for this task.
            // This prevents over/under-extraction from a previous task (especially
            // 7z archives where estimated size = compressed_size * 3 may be inaccurate)
            // from bleeding into the current task's progress calculation.
            let cumulative_start = ctx.task_cumulative.get(index).copied().unwrap_or(0);
            ctx.processed_bytes
                .store(cumulative_start, Ordering::SeqCst);

            // Reset inline verification state for this task
            ctx.inline_verified
                .store(false, std::sync::atomic::Ordering::SeqCst);
            ctx.inline_verified_count.store(0, Ordering::SeqCst);
            ctx.inline_hashes.lock().unwrap().clear();

            // Force emit progress to ensure frontend sees the task state change immediately
            ctx.emit_progress_force(None, InstallPhase::Installing);

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
                    // Transfer inline-computed hashes to the task (for 7z SHA256)
                    {
                        let inline = ctx.inline_hashes.lock().unwrap();
                        if !inline.is_empty() {
                            task.file_hashes = Some(inline.clone());
                            ctx.inline_verified
                                .store(true, std::sync::atomic::Ordering::SeqCst);
                            ctx.inline_verified_count
                                .store(inline.len() as u64, Ordering::SeqCst);
                        }
                    }

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
                    // Force emit to ensure frontend sees verification phase immediately
                    ctx.emit_progress_force(
                        Some("Verifying...".to_string()),
                        InstallPhase::Verifying,
                    );

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
        let total_failed = failed + skipped + cancelled;
        if total_failed == 0 {
            logger::log_info(&tr(LogMsg::InstallationCompleted), Some("installer"));
        } else if successful > 0 {
            logger::log_info(
                &format!(
                    "Installation completed with partial failures: {}/{} successful, {} failed (failed={}, skipped={}, cancelled={})",
                    successful,
                    tasks.len(),
                    total_failed,
                    failed,
                    skipped,
                    cancelled
                ),
                Some("installer"),
            );
        } else {
            logger::log_error(
                &format!(
                    "{}: 0/{} successful (failed={}, skipped={}, cancelled={})",
                    tr(LogMsg::InstallationFailed),
                    tasks.len(),
                    failed,
                    skipped,
                    cancelled
                ),
                Some("installer"),
            );
        }
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

    /// Install tasks in parallel with a configurable concurrency limit
    pub async fn install_parallel(
        &self,
        tasks: Vec<InstallTask>,
        max_concurrent: usize,
        atomic_install_enabled: bool,
        xplane_path: String,
        delete_source_after_install: bool,
        auto_sort_scenery: bool,
    ) -> Result<InstallResult> {
        let install_start = Instant::now();
        let max_concurrent = max_concurrent.clamp(2, 10);

        crate::log_debug!(
            &format!(
                "[TIMING] Parallel installation started: {} tasks, max_concurrent={}",
                tasks.len(),
                max_concurrent
            ),
            "installer_timing"
        );

        logger::log_info(
            &format!(
                "{}: {} task(s) (parallel, max_concurrent: {}, atomic: {})",
                tr(LogMsg::InstallationStarted),
                tasks.len(),
                max_concurrent,
                atomic_install_enabled
            ),
            Some("installer"),
        );

        // Reset task control at start
        self.task_control.reset();

        // Phase 1: Calculate total size
        let calc_start = Instant::now();
        let (total_size, task_sizes) = self.calculate_total_size(&tasks)?;
        crate::log_debug!(
            &format!(
                "[TIMING] Parallel size calculation completed in {:.2}ms: {} bytes",
                calc_start.elapsed().as_secs_f64() * 1000.0,
                total_size
            ),
            "installer_timing"
        );

        // Emit calculating phase (include activeTasks so frontend detects parallel mode)
        {
            let progress = InstallProgress {
                percentage: 0.0,
                total_bytes: total_size,
                processed_bytes: 0,
                current_task_index: 0,
                total_tasks: tasks.len(),
                current_task_name: String::new(),
                current_file: None,
                phase: InstallPhase::Calculating,
                verification_progress: None,
                current_task_percentage: 0.0,
                current_task_total_bytes: 0,
                current_task_processed_bytes: 0,
                active_tasks: Some(Vec::new()),
                completed_task_count: Some(0),
                completed_task_ids: Some(Vec::new()),
            };
            let _ = self.app_handle.emit("install-progress", &progress);
        }

        // Create parallel progress context
        let ctx = Arc::new(ParallelProgressContext::new(
            self.app_handle.clone(),
            total_size,
            tasks.len(),
            &task_sizes,
            &tasks,
        ));

        // Phase 2: Parallel installation
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
        let task_control = self.task_control.clone();
        let app_handle = self.app_handle.clone();

        let mut handles = Vec::new();

        // Save task metadata for post-install operations (scenery sorting, etc.)
        // since tasks will be moved into spawn_blocking
        #[allow(dead_code)]
        struct TaskMeta {
            addon_type: AddonType,
            target_path: String,
        }
        let task_metas: Vec<TaskMeta> = tasks
            .iter()
            .map(|t| TaskMeta {
                addon_type: t.addon_type.clone(),
                target_path: t.target_path.clone(),
            })
            .collect();

        for (index, task) in tasks.into_iter().enumerate() {
            let sem = semaphore.clone();
            let ctx = ctx.clone();
            let tc = task_control.clone();
            let ah = app_handle.clone();
            let xp = xplane_path.clone();
            let atomic = atomic_install_enabled;
            let delete_source = delete_source_after_install;

            let handle = tokio::spawn(async move {
                // Acquire semaphore permit asynchronously
                let _permit = match sem.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        return TaskResult {
                            task_id: task.id.clone(),
                            task_name: task.display_name.clone(),
                            success: false,
                            error_message: Some("Semaphore closed".to_string()),
                            verification_stats: None,
                        };
                    }
                };

                // Check cancel
                if tc.is_cancelled() {
                    return TaskResult {
                        task_id: task.id.clone(),
                        task_name: task.display_name.clone(),
                        success: false,
                        error_message: Some("Cancelled by user".to_string()),
                        verification_stats: None,
                    };
                }

                // Mark task as installing
                crate::log_debug!(
                    &format!(
                        "[PARALLEL] Task {} starting installation (semaphore acquired)",
                        index
                    ),
                    "parallel_progress"
                );
                ctx.trackers[index]
                    .phase
                    .store(1, std::sync::atomic::Ordering::SeqCst);

                // Run blocking I/O work in spawn_blocking
                let result = tokio::task::spawn_blocking(move || {
                    let task_view = ctx.get_task_view(index);
                    let progress_ctx = task_view.as_progress_context();
                    let installer = Installer::new(ah);

                    let mut task = task;
                    match installer.install_task_with_progress(&task, &progress_ctx, atomic, &xp) {
                        Ok(_) => {
                            {
                                let inline = progress_ctx.inline_hashes.lock().unwrap();
                                if !inline.is_empty() {
                                    task.file_hashes = Some(inline.clone());
                                    progress_ctx
                                        .inline_verified
                                        .store(true, std::sync::atomic::Ordering::SeqCst);
                                    progress_ctx
                                        .inline_verified_count
                                        .store(inline.len() as u64, Ordering::SeqCst);
                                }
                            }

                            ctx.trackers[index]
                                .phase
                                .store(2, std::sync::atomic::Ordering::SeqCst);

                            progress_ctx.set_verification_progress(0.0);
                            progress_ctx.emit_progress(
                                Some("Verifying...".to_string()),
                                InstallPhase::Verifying,
                            );

                            match installer.verify_installation(&task, &progress_ctx) {
                                Ok(verification_stats) => {
                                    ctx.mark_completed(index);
                                    logger::log_info(
                                        &format!(
                                            "{}: {}",
                                            tr(LogMsg::InstallationCompleted),
                                            task.display_name
                                        ),
                                        Some("installer"),
                                    );

                                    if delete_source {
                                        if let Some(original_path) = &task.original_input_path {
                                            if let Err(e) = installer.delete_source_file(
                                                original_path,
                                                &task.source_path,
                                            ) {
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

                                    TaskResult {
                                        task_id: task.id.clone(),
                                        task_name: task.display_name.clone(),
                                        success: true,
                                        error_message: None,
                                        verification_stats,
                                    }
                                }
                                Err(e) => {
                                    ctx.mark_failed(index);
                                    let error_msg = format!("Verification failed: {}", e);
                                    logger::log_error(
                                        &format!(
                                            "{} {}: {}",
                                            tr(LogMsg::InstallationFailed),
                                            task.display_name,
                                            error_msg
                                        ),
                                        Some("installer"),
                                    );
                                    TaskResult {
                                        task_id: task.id.clone(),
                                        task_name: task.display_name.clone(),
                                        success: false,
                                        error_message: Some(error_msg),
                                        verification_stats: None,
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            ctx.mark_failed(index);
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
                            TaskResult {
                                task_id: task.id.clone(),
                                task_name: task.display_name.clone(),
                                success: false,
                                error_message: Some(error_msg),
                                verification_stats: None,
                            }
                        }
                    }
                })
                .await;

                match result {
                    Ok(task_result) => task_result,
                    Err(e) => TaskResult {
                        task_id: String::new(),
                        task_name: String::new(),
                        success: false,
                        error_message: Some(format!("Task panicked: {}", e)),
                        verification_stats: None,
                    },
                }
            });
            handles.push(handle);
        }

        // Collect results
        let mut task_results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => task_results.push(result),
                Err(e) => {
                    task_results.push(TaskResult {
                        task_id: String::new(),
                        task_name: String::new(),
                        success: false,
                        error_message: Some(format!("Task panicked: {}", e)),
                        verification_stats: None,
                    });
                }
            }
        }

        // Phase 3: Finalize
        ctx.emit_final();

        // Auto-sort scenery for successful scenery tasks
        if auto_sort_scenery {
            for (i, result) in task_results.iter().enumerate() {
                if !result.success {
                    continue;
                }
                if let Some(meta) = task_metas.get(i) {
                    if meta.addon_type == AddonType::Scenery
                        || meta.addon_type == AddonType::SceneryLibrary
                    {
                        use crate::scenery_classifier::classify_scenery;
                        use crate::scenery_packs_manager::SceneryPacksManager;

                        let target_path = Path::new(&meta.target_path);
                        if let Some(folder_name) = target_path.file_name().and_then(|n| n.to_str())
                        {
                            let xplane_path_buf = PathBuf::from(&xplane_path);
                            match classify_scenery(target_path, &xplane_path_buf) {
                                Ok(scenery_info) => {
                                    let manager =
                                        SceneryPacksManager::new(&xplane_path_buf, self.db.clone());
                                    if let Err(e) =
                                        manager.add_entry(folder_name, &scenery_info.category).await
                                    {
                                        logger::log_error(
                                            &format!(
                                                "Failed to add scenery to scenery_packs.ini: {}",
                                                e
                                            ),
                                            Some("installer"),
                                        );
                                    } else {
                                        logger::log_info(
                                            &format!(
                                                "Added {} to scenery_packs.ini (category: {:?})",
                                                folder_name, scenery_info.category
                                            ),
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
            }
        }

        let successful = task_results.iter().filter(|r| r.success).count();
        let failed = task_results.iter().filter(|r| !r.success).count();

        crate::log_debug!(
            &format!(
                "[TIMING] Parallel installation completed in {:.2}ms: {} successful, {} failed",
                install_start.elapsed().as_secs_f64() * 1000.0,
                successful,
                failed
            ),
            "installer_timing"
        );

        if failed == 0 {
            logger::log_info(&tr(LogMsg::InstallationCompleted), Some("installer"));
        } else if successful > 0 {
            logger::log_info(
                &format!(
                    "Installation completed with partial failures: {}/{} successful, {} failed",
                    successful,
                    task_results.len(),
                    failed
                ),
                Some("installer"),
            );
        } else {
            logger::log_error(
                &format!(
                    "{}: 0/{} successful, {} failed",
                    tr(LogMsg::InstallationFailed),
                    task_results.len(),
                    failed
                ),
                Some("installer"),
            );
        }

        Ok(InstallResult {
            total_tasks: task_results.len(),
            successful_tasks: successful,
            failed_tasks: failed,
            task_results,
        })
    }

    /// Calculate total size of all tasks for progress tracking
    /// Returns (total_size, per_task_sizes) for proportional progress calculation
    /// Only counts source extraction bytes — restore overhead is excluded because
    /// atomic installer doesn't track restore bytes, and the restore phase is fast.
    fn calculate_total_size(&self, tasks: &[InstallTask]) -> Result<(u64, Vec<u64>)> {
        let mut total = 0u64;
        let mut task_sizes = Vec::with_capacity(tasks.len());

        for task in tasks {
            let mut task_size = 0u64;
            let source = Path::new(&task.source_path);

            // LuaScript from direct file source may include companion files/folders.
            // Include those sizes so progress remains accurate.
            let source_ext = source
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let source_is_archive = matches!(source_ext.as_str(), "zip" | "7z" | "rar");

            if task.addon_type == AddonType::LuaScript && source.is_file() {
                if source_is_archive {
                    task_size += self.get_lua_bundle_size_in_archive(task, source)?;
                } else if let Some(parent_dir) = source.parent() {
                    task_size += self.get_lua_bundle_size_from_directory(task, parent_dir)?;
                } else {
                    task_size += fs::metadata(source)?.len();
                }
            } else if task.addon_type == AddonType::LuaScript && source.is_dir() {
                task_size += self.get_lua_bundle_size_from_directory(task, source)?;
            } else if source.is_dir() {
                task_size += self.get_directory_size(source)?;
            } else if source.is_file() {
                task_size +=
                    self.get_archive_size(source, task.archive_internal_root.as_deref())?;
            }

            task_sizes.push(task_size);
            total += task_size;
        }
        Ok((total, task_sizes))
    }

    /// Build Lua bundle entries from task info: script file + companions.
    fn get_lua_bundle_entries_for_size(&self, task: &InstallTask) -> Vec<PathBuf> {
        use std::collections::HashSet;

        let mut entries = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();

        if let Some(script_name) = Path::new(&task.target_path).file_name() {
            let script_entry = PathBuf::from(script_name);
            seen.insert(script_entry.clone());
            entries.push(script_entry);
        }

        for companion in &task.companion_paths {
            if let Some(safe_path) = sanitize_path(Path::new(companion)) {
                if seen.insert(safe_path.clone()) {
                    entries.push(safe_path);
                }
            }
        }

        entries
    }

    /// Calculate Lua bundle size from an on-disk source directory.
    fn get_lua_bundle_size_from_directory(
        &self,
        task: &InstallTask,
        source_dir: &Path,
    ) -> Result<u64> {
        let mut total = 0u64;
        for entry in self.get_lua_bundle_entries_for_size(task) {
            total = total.saturating_add(self.get_path_size(&source_dir.join(entry))?);
        }
        Ok(total)
    }

    /// Whether an archive-relative path belongs to a Lua bundle entry.
    fn lua_bundle_entry_matches(relative_path: &str, bundle_entries: &[String]) -> bool {
        let rel = relative_path.trim_start_matches('/').trim_end_matches('/');
        if rel.is_empty() {
            return false;
        }

        bundle_entries.iter().any(|entry| {
            rel == entry
                || rel
                    .strip_prefix(entry)
                    .map(|suffix| suffix.starts_with('/'))
                    .unwrap_or(false)
        })
    }

    /// Calculate Lua bundle size from archive metadata only (no extraction).
    fn get_lua_bundle_size_in_archive(&self, task: &InstallTask, archive: &Path) -> Result<u64> {
        let source_ext = archive
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if task.extraction_chain.is_some() {
            return self.get_archive_size(archive, task.archive_internal_root.as_deref());
        }

        // For 7z fast-scan fallback (solid archives), companion_paths may be empty at scan time.
        // Use internal_root size to avoid underestimating progress bytes.
        if source_ext == "7z" && task.companion_paths.is_empty() {
            return self.get_archive_size(archive, task.archive_internal_root.as_deref());
        }

        let bundle_entries_raw = self.get_lua_bundle_entries_for_size(task);
        let bundle_entries: Vec<String> = bundle_entries_raw
            .into_iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();

        if bundle_entries.is_empty() {
            return self.get_archive_size(archive, task.archive_internal_root.as_deref());
        }

        let prefix = task.archive_internal_root.as_ref().map(|root| {
            let normalized = root.replace('\\', "/").trim_matches('/').to_string();
            if normalized.is_empty() {
                String::new()
            } else {
                format!("{}/", normalized)
            }
        });

        let mut total = 0u64;

        match source_ext.as_str() {
            "zip" => {
                use zip::ZipArchive;
                let file = fs::File::open(archive)?;
                let mut archive_reader = ZipArchive::new(file)?;
                for i in 0..archive_reader.len() {
                    let file = match archive_reader.by_index_raw(i) {
                        Ok(f) => f,
                        Err(_) => continue,
                    };
                    let name = file.name().replace('\\', "/");
                    let relative = if let Some(ref p) = prefix {
                        if !name.starts_with(p) {
                            continue;
                        }
                        name[p.len()..].to_string()
                    } else {
                        name
                    };
                    if Self::lua_bundle_entry_matches(&relative, &bundle_entries) {
                        total = total.saturating_add(file.size());
                    }
                }
            }
            "7z" => {
                let archive_meta = sevenz_rust2::Archive::open(archive)?;
                for entry in &archive_meta.files {
                    if entry.is_directory() || !entry.has_stream() {
                        continue;
                    }
                    let name = entry.name().replace('\\', "/");
                    let relative = if let Some(ref p) = prefix {
                        if !name.starts_with(p) {
                            continue;
                        }
                        name[p.len()..].to_string()
                    } else {
                        name
                    };
                    if Self::lua_bundle_entry_matches(&relative, &bundle_entries) {
                        total = total.saturating_add(entry.size());
                    }
                }
            }
            _ => {
                return self.get_archive_size(archive, task.archive_internal_root.as_deref());
            }
        }

        if total == 0 {
            self.get_archive_size(archive, task.archive_internal_root.as_deref())
        } else {
            Ok(total)
        }
    }

    /// Get size of a file or a directory path. Missing paths return 0.
    fn get_path_size(&self, path: &Path) -> Result<u64> {
        if path.is_dir() {
            self.get_directory_size(path)
        } else if path.is_file() {
            Ok(fs::metadata(path)?.len())
        } else {
            Ok(0)
        }
    }

    /// Get total size of config files matching patterns in a directory
    #[allow(dead_code)]
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
        let ext = archive
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        match ext.as_deref() {
            Some("zip") => self.get_zip_size(archive, internal_root),
            Some("7z") => self.get_7z_size(archive, internal_root),
            Some("rar") => self.get_rar_size(archive),
            // Non-archive files (e.g. standalone Lua scripts) are installed by direct copy.
            _ => Ok(fs::metadata(archive)?.len()),
        }
    }

    /// Get uncompressed size of ZIP archive
    fn get_zip_size(&self, archive: &Path, internal_root: Option<&str>) -> Result<u64> {
        use zip::ZipArchive;
        let file = fs::File::open(archive)?;
        let mut archive_reader = ZipArchive::new(file)?;
        let prefix = internal_root.map(|s| {
            let normalized = s.replace('\\', "/").trim_matches('/').to_string();
            if normalized.is_empty() {
                String::new()
            } else {
                format!("{}/", normalized)
            }
        });

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

    /// Get uncompressed size of 7z archive.
    /// Uses archive metadata directly and supports internal_root filtering.
    fn get_7z_size(&self, archive: &Path, internal_root: Option<&str>) -> Result<u64> {
        // For full-archive queries, use cache when available.
        if internal_root.is_none() {
            if let Some(cached) = crate::cache::get_cached_metadata(archive) {
                return Ok(cached.uncompressed_size);
            }
        }

        let prefix = internal_root.map(|s| {
            let normalized = s.replace('\\', "/").trim_matches('/').to_string();
            if normalized.is_empty() {
                String::new()
            } else {
                format!("{}/", normalized)
            }
        });

        let archive_meta = match sevenz_rust2::Archive::open(archive) {
            Ok(a) => a,
            Err(_) => {
                // Fallback for corrupted/unsupported 7z metadata:
                // keep previous conservative behavior.
                let meta = fs::metadata(archive)?;
                return Ok(meta.len() * 3);
            }
        };

        let mut total = 0u64;
        let mut file_count = 0usize;

        for entry in &archive_meta.files {
            if entry.is_directory() || !entry.has_stream() {
                continue;
            }

            let entry_name = entry.name().replace('\\', "/");
            if let Some(ref p) = prefix {
                if !entry_name.starts_with(p) {
                    continue;
                }
            }

            total = total.saturating_add(entry.size());
            file_count += 1;
        }

        if internal_root.is_none() && total > 0 {
            crate::cache::cache_metadata(archive, total, file_count);
        }

        Ok(total)
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

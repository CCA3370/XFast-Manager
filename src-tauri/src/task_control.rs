use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

/// Task control state for managing installation cancellation and skipping
#[derive(Clone)]
pub struct TaskControl {
    /// Flag to cancel all remaining tasks
    cancel_all: Arc<AtomicBool>,
    /// Flag to skip the current task
    skip_current: Arc<AtomicBool>,
    /// List of files/directories created during installation (for cleanup)
    processed_paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl TaskControl {
    pub fn new() -> Self {
        Self {
            cancel_all: Arc::new(AtomicBool::new(false)),
            skip_current: Arc::new(AtomicBool::new(false)),
            processed_paths: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Request cancellation of all tasks
    pub fn request_cancel_all(&self) {
        self.cancel_all.store(true, Ordering::SeqCst);
    }

    /// Request skipping the current task
    pub fn request_skip_current(&self) {
        self.skip_current.store(true, Ordering::SeqCst);
    }

    /// Check if cancellation was requested
    pub fn is_cancelled(&self) -> bool {
        self.cancel_all.load(Ordering::SeqCst)
    }

    /// Check if skip was requested
    pub fn is_skip_requested(&self) -> bool {
        self.skip_current.load(Ordering::SeqCst)
    }

    /// Reset skip flag (called after handling skip)
    pub fn reset_skip(&self) {
        self.skip_current.store(false, Ordering::SeqCst);
    }

    /// Reset all flags (called at start of installation)
    pub fn reset(&self) {
        self.cancel_all.store(false, Ordering::SeqCst);
        self.skip_current.store(false, Ordering::SeqCst);
        if let Ok(mut paths) = self.processed_paths.lock() {
            paths.clear();
        }
    }

    /// Add a processed path for potential cleanup
    pub fn add_processed_path(&self, path: PathBuf) {
        if let Ok(mut paths) = self.processed_paths.lock() {
            paths.push(path);
        }
    }

    /// Get all processed paths
    pub fn get_processed_paths(&self) -> Vec<PathBuf> {
        self.processed_paths
            .lock()
            .map(|paths| paths.clone())
            .unwrap_or_default()
    }

    /// Clear processed paths
    pub fn clear_processed_paths(&self) {
        if let Ok(mut paths) = self.processed_paths.lock() {
            paths.clear();
        }
    }
}

impl Default for TaskControl {
    fn default() -> Self {
        Self::new()
    }
}

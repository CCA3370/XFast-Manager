use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
    // Exposed for future atomic rollback: collect installed paths for cleanup on failure
    #[allow(dead_code)]
    pub fn get_processed_paths(&self) -> Vec<PathBuf> {
        self.processed_paths
            .lock()
            .map(|paths| paths.clone())
            .unwrap_or_default()
    }

    /// Clear processed paths
    // Exposed for future atomic rollback: reset path list between installation phases
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_control_initial_state() {
        let control = TaskControl::new();
        assert!(!control.is_cancelled());
        assert!(!control.is_skip_requested());
        assert!(control.get_processed_paths().is_empty());
    }

    #[test]
    fn test_task_control_default() {
        let control = TaskControl::default();
        assert!(!control.is_cancelled());
        assert!(!control.is_skip_requested());
    }

    #[test]
    fn test_cancel_all_request() {
        let control = TaskControl::new();
        assert!(!control.is_cancelled());

        control.request_cancel_all();
        assert!(control.is_cancelled());

        // Cancel flag persists until reset
        assert!(control.is_cancelled());
    }

    #[test]
    fn test_skip_current_request() {
        let control = TaskControl::new();
        assert!(!control.is_skip_requested());

        control.request_skip_current();
        assert!(control.is_skip_requested());
    }

    #[test]
    fn test_reset_skip() {
        let control = TaskControl::new();
        control.request_skip_current();
        assert!(control.is_skip_requested());

        control.reset_skip();
        assert!(!control.is_skip_requested());
    }

    #[test]
    fn test_reset_all() {
        let control = TaskControl::new();

        // Set various states
        control.request_cancel_all();
        control.request_skip_current();
        control.add_processed_path(PathBuf::from("/test/path"));

        // Verify states are set
        assert!(control.is_cancelled());
        assert!(control.is_skip_requested());
        assert!(!control.get_processed_paths().is_empty());

        // Reset all
        control.reset();

        // Verify all states are cleared
        assert!(!control.is_cancelled());
        assert!(!control.is_skip_requested());
        assert!(control.get_processed_paths().is_empty());
    }

    #[test]
    fn test_processed_paths() {
        let control = TaskControl::new();

        control.add_processed_path(PathBuf::from("/path/1"));
        control.add_processed_path(PathBuf::from("/path/2"));
        control.add_processed_path(PathBuf::from("/path/3"));

        let paths = control.get_processed_paths();
        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&PathBuf::from("/path/1")));
        assert!(paths.contains(&PathBuf::from("/path/2")));
        assert!(paths.contains(&PathBuf::from("/path/3")));
    }

    #[test]
    fn test_clear_processed_paths() {
        let control = TaskControl::new();

        control.add_processed_path(PathBuf::from("/path/1"));
        assert!(!control.get_processed_paths().is_empty());

        control.clear_processed_paths();
        assert!(control.get_processed_paths().is_empty());
    }

    #[test]
    fn test_task_control_clone() {
        let control1 = TaskControl::new();
        control1.request_cancel_all();

        // Clone shares the same atomic state
        let control2 = control1.clone();
        assert!(control2.is_cancelled());

        // Changes in clone affect original (shared Arc)
        control2.reset();
        assert!(!control1.is_cancelled());
    }

    #[test]
    fn test_cancel_and_skip_independent() {
        let control = TaskControl::new();

        // Cancel and skip are independent flags
        control.request_cancel_all();
        assert!(control.is_cancelled());
        assert!(!control.is_skip_requested());

        control.request_skip_current();
        assert!(control.is_cancelled());
        assert!(control.is_skip_requested());

        // Reset skip doesn't affect cancel
        control.reset_skip();
        assert!(control.is_cancelled());
        assert!(!control.is_skip_requested());
    }
}

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

/// Task control state for managing installation cancellation and skipping
#[derive(Clone)]
pub struct TaskControl {
    /// Flag to cancel all remaining tasks
    cancel_all: Arc<AtomicBool>,
    /// Flag to skip the current task
    skip_current: Arc<AtomicBool>,
    /// List of files/directories created during installation (for cleanup)
    processed_paths: Arc<Mutex<Vec<PathBuf>>>,
    /// Child controls that should receive global cancel/skip requests.
    child_controls: Arc<Mutex<Vec<Weak<TaskControlState>>>>,
    /// Keeps this control session alive for parent propagation.
    _session_state: Arc<TaskControlState>,
}

struct TaskControlState {
    cancel_all: Arc<AtomicBool>,
    skip_current: Arc<AtomicBool>,
}

impl TaskControl {
    pub fn new() -> Self {
        let state = Arc::new(TaskControlState {
            cancel_all: Arc::new(AtomicBool::new(false)),
            skip_current: Arc::new(AtomicBool::new(false)),
        });

        Self {
            cancel_all: Arc::clone(&state.cancel_all),
            skip_current: Arc::clone(&state.skip_current),
            processed_paths: Arc::new(Mutex::new(Vec::new())),
            child_controls: Arc::new(Mutex::new(Vec::new())),
            _session_state: state,
        }
    }

    fn for_state(state: &Arc<TaskControlState>) -> Self {
        Self {
            cancel_all: Arc::clone(&state.cancel_all),
            skip_current: Arc::clone(&state.skip_current),
            processed_paths: Arc::new(Mutex::new(Vec::new())),
            child_controls: Arc::new(Mutex::new(Vec::new())),
            _session_state: Arc::clone(state),
        }
    }

    /// Create an isolated child control for one task session.
    ///
    /// A child starts with clean flags and an independent processed path list, but global
    /// cancel/skip requests on this parent are propagated to all active children.
    pub fn child_session(&self) -> Self {
        let state = Arc::new(TaskControlState {
            cancel_all: Arc::new(AtomicBool::new(false)),
            skip_current: Arc::new(AtomicBool::new(false)),
        });

        if let Ok(mut children) = self.child_controls.lock() {
            children.retain(|child| child.strong_count() > 0);
            children.push(Arc::downgrade(&state));
        }

        Self::for_state(&state)
    }

    fn propagate_to_children<F>(&self, apply: F)
    where
        F: Fn(&TaskControlState),
    {
        if let Ok(mut children) = self.child_controls.lock() {
            children.retain(|child| {
                if let Some(state) = child.upgrade() {
                    apply(&state);
                    true
                } else {
                    false
                }
            });
        }
    }

    /// Request cancellation of all tasks
    pub fn request_cancel_all(&self) {
        self.cancel_all.store(true, Ordering::SeqCst);
        self.propagate_to_children(|state| {
            state.cancel_all.store(true, Ordering::SeqCst);
        });
    }

    /// Request skipping the current task
    pub fn request_skip_current(&self) {
        self.skip_current.store(true, Ordering::SeqCst);
        self.propagate_to_children(|state| {
            state.skip_current.store(true, Ordering::SeqCst);
        });
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

    #[test]
    fn test_child_session_is_independent_from_reset() {
        let parent = TaskControl::new();
        let child = parent.child_session();

        child.request_cancel_all();
        child.add_processed_path(PathBuf::from("/child/path"));

        parent.reset();

        assert!(!parent.is_cancelled());
        assert!(!parent.is_skip_requested());
        assert!(child.is_cancelled());
        assert!(!child.is_skip_requested());
        assert_eq!(
            child.get_processed_paths(),
            vec![PathBuf::from("/child/path")]
        );
    }

    #[test]
    fn test_global_cancel_propagates_to_child_sessions() {
        let parent = TaskControl::new();
        let child1 = parent.child_session();
        let child2 = parent.child_session();

        parent.request_cancel_all();

        assert!(parent.is_cancelled());
        assert!(child1.is_cancelled());
        assert!(child2.is_cancelled());
    }

    #[test]
    fn test_global_skip_propagates_to_child_sessions() {
        let parent = TaskControl::new();
        let child = parent.child_session();

        parent.request_skip_current();

        assert!(parent.is_skip_requested());
        assert!(child.is_skip_requested());
    }
}

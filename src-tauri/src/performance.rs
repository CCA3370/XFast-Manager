use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Global cache performance counters
static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);

/// Global operation counters
static TOTAL_FILES_SCANNED: AtomicU64 = AtomicU64::new(0);
static TOTAL_ARCHIVES_PROCESSED: AtomicU64 = AtomicU64::new(0);
static TOTAL_BYTES_PROCESSED: AtomicU64 = AtomicU64::new(0);

/// Record a cache hit
pub fn record_cache_hit() {
    CACHE_HITS.fetch_add(1, Ordering::Relaxed);
}

/// Record a cache miss
pub fn record_cache_miss() {
    CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
}

/// Record files scanned
pub fn record_files_scanned(count: u64) {
    TOTAL_FILES_SCANNED.fetch_add(count, Ordering::Relaxed);
}

/// Record archive processed
pub fn record_archive_processed() {
    TOTAL_ARCHIVES_PROCESSED.fetch_add(1, Ordering::Relaxed);
}

/// Record bytes processed
pub fn record_bytes_processed(bytes: u64) {
    TOTAL_BYTES_PROCESSED.fetch_add(bytes, Ordering::Relaxed);
}

/// Get cache hit rate (0.0 to 1.0)
pub fn get_cache_hit_rate() -> f64 {
    let hits = CACHE_HITS.load(Ordering::Relaxed);
    let misses = CACHE_MISSES.load(Ordering::Relaxed);
    let total = hits + misses;

    if total == 0 {
        0.0
    } else {
        hits as f64 / total as f64
    }
}

/// Get performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
    pub total_files_scanned: u64,
    pub total_archives_processed: u64,
    pub total_bytes_processed: u64,
}

/// Get current performance statistics
pub fn get_stats() -> PerformanceStats {
    PerformanceStats {
        cache_hits: CACHE_HITS.load(Ordering::Relaxed),
        cache_misses: CACHE_MISSES.load(Ordering::Relaxed),
        cache_hit_rate: get_cache_hit_rate(),
        total_files_scanned: TOTAL_FILES_SCANNED.load(Ordering::Relaxed),
        total_archives_processed: TOTAL_ARCHIVES_PROCESSED.load(Ordering::Relaxed),
        total_bytes_processed: TOTAL_BYTES_PROCESSED.load(Ordering::Relaxed),
    }
}

/// Reset all performance counters
pub fn reset_stats() {
    CACHE_HITS.store(0, Ordering::Relaxed);
    CACHE_MISSES.store(0, Ordering::Relaxed);
    TOTAL_FILES_SCANNED.store(0, Ordering::Relaxed);
    TOTAL_ARCHIVES_PROCESSED.store(0, Ordering::Relaxed);
    TOTAL_BYTES_PROCESSED.store(0, Ordering::Relaxed);
}

/// Timer for measuring operation duration
pub struct OperationTimer {
    start: Instant,
    operation_name: String,
}

impl OperationTimer {
    /// Start a new timer
    pub fn new(operation_name: impl Into<String>) -> Self {
        OperationTimer {
            start: Instant::now(),
            operation_name: operation_name.into(),
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Log elapsed time (for debugging)
    pub fn log_elapsed(&self) {
        let elapsed = self.elapsed();
        crate::log_debug!(
            &format!("{} completed in {:.2}ms", self.operation_name, elapsed.as_secs_f64() * 1000.0),
            "performance"
        );
    }
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        // Optionally log on drop
        if cfg!(debug_assertions) {
            self.log_elapsed();
        }
    }
}

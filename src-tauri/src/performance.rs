use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};

/// Performance metrics for operations
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operation: String,
    pub duration: Duration,
    pub bytes_processed: u64,
    pub throughput_mbps: f64,
}

/// Global performance counters
static TOTAL_BYTES_PROCESSED: AtomicU64 = AtomicU64::new(0);
static TOTAL_FILES_PROCESSED: AtomicU64 = AtomicU64::new(0);
static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);

/// Performance timer for measuring operation duration
pub struct PerformanceTimer {
    operation: String,
    start: Instant,
    bytes: u64,
}

impl PerformanceTimer {
    /// Start a new performance timer
    pub fn start(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            start: Instant::now(),
            bytes: 0,
        }
    }

    /// Record bytes processed
    pub fn add_bytes(&mut self, bytes: u64) {
        self.bytes += bytes;
        TOTAL_BYTES_PROCESSED.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a file processed
    pub fn add_file(&self) {
        TOTAL_FILES_PROCESSED.fetch_add(1, Ordering::Relaxed);
    }

    /// Finish the timer and return metrics
    pub fn finish(self) -> PerformanceMetrics {
        let duration = self.start.elapsed();
        let throughput_mbps = if duration.as_secs_f64() > 0.0 {
            (self.bytes as f64 / 1024.0 / 1024.0) / duration.as_secs_f64()
        } else {
            0.0
        };

        PerformanceMetrics {
            operation: self.operation,
            duration,
            bytes_processed: self.bytes,
            throughput_mbps,
        }
    }

    /// Log the metrics
    pub fn log_and_finish(self) {
        let metrics = self.finish();
        crate::logger::log_info(
            &format!(
                "Performance: {} completed in {:.2}s, processed {:.2}MB at {:.2}MB/s",
                metrics.operation,
                metrics.duration.as_secs_f64(),
                metrics.bytes_processed as f64 / 1024.0 / 1024.0,
                metrics.throughput_mbps
            ),
            Some("performance"),
        );
    }
}

/// Record a cache hit
pub fn record_cache_hit() {
    CACHE_HITS.fetch_add(1, Ordering::Relaxed);
}

/// Record a cache miss
pub fn record_cache_miss() {
    CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
}

/// Get cache hit rate
pub fn get_cache_hit_rate() -> f64 {
    let hits = CACHE_HITS.load(Ordering::Relaxed);
    let misses = CACHE_MISSES.load(Ordering::Relaxed);
    let total = hits + misses;

    if total > 0 {
        (hits as f64 / total as f64) * 100.0
    } else {
        0.0
    }
}

/// Get total bytes processed
pub fn get_total_bytes_processed() -> u64 {
    TOTAL_BYTES_PROCESSED.load(Ordering::Relaxed)
}

/// Get total files processed
pub fn get_total_files_processed() -> u64 {
    TOTAL_FILES_PROCESSED.load(Ordering::Relaxed)
}

/// Get performance statistics
pub fn get_stats() -> String {
    let bytes = get_total_bytes_processed();
    let files = get_total_files_processed();
    let cache_rate = get_cache_hit_rate();
    let hits = CACHE_HITS.load(Ordering::Relaxed);
    let misses = CACHE_MISSES.load(Ordering::Relaxed);

    format!(
        "Performance Stats:\n\
         - Total bytes processed: {:.2} MB\n\
         - Total files processed: {}\n\
         - Cache hit rate: {:.1}% ({} hits, {} misses)",
        bytes as f64 / 1024.0 / 1024.0,
        files,
        cache_rate,
        hits,
        misses
    )
}

/// Reset all performance counters
pub fn reset_stats() {
    TOTAL_BYTES_PROCESSED.store(0, Ordering::Relaxed);
    TOTAL_FILES_PROCESSED.store(0, Ordering::Relaxed);
    CACHE_HITS.store(0, Ordering::Relaxed);
    CACHE_MISSES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_performance_timer() {
        let mut timer = PerformanceTimer::start("test_operation");
        timer.add_bytes(1024 * 1024); // 1MB
        thread::sleep(Duration::from_millis(10));

        let metrics = timer.finish();
        assert_eq!(metrics.operation, "test_operation");
        assert_eq!(metrics.bytes_processed, 1024 * 1024);
        assert!(metrics.duration.as_millis() >= 10);
    }

    #[test]
    fn test_cache_metrics() {
        reset_stats();

        record_cache_hit();
        record_cache_hit();
        record_cache_miss();

        assert_eq!(get_cache_hit_rate(), 66.66666666666666);
    }

    #[test]
    fn test_stats() {
        reset_stats();

        let mut timer = PerformanceTimer::start("test");
        timer.add_bytes(2048);
        timer.add_file();

        assert_eq!(get_total_bytes_processed(), 2048);
        assert_eq!(get_total_files_processed(), 1);
    }
}

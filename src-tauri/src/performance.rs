use std::sync::atomic::{AtomicU64, Ordering};

/// Global cache performance counters
static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);

/// Record a cache hit
pub fn record_cache_hit() {
    CACHE_HITS.fetch_add(1, Ordering::Relaxed);
}

/// Record a cache miss
pub fn record_cache_miss() {
    CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
}

/// Get cache statistics (hits, misses)
#[allow(dead_code)]
pub fn get_cache_stats() -> (u64, u64) {
    let hits = CACHE_HITS.load(Ordering::Relaxed);
    let misses = CACHE_MISSES.load(Ordering::Relaxed);
    (hits, misses)
}

/// Get cache hit rate as a percentage (0.0 - 1.0)
#[allow(dead_code)]
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

/// Reset cache statistics (useful for testing)
#[allow(dead_code)]
pub fn reset_cache_stats() {
    CACHE_HITS.store(0, Ordering::Relaxed);
    CACHE_MISSES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_metrics() {
        // Reset counters for test
        reset_cache_stats();

        record_cache_hit();
        record_cache_hit();
        record_cache_miss();

        let (hits, misses) = get_cache_stats();
        assert_eq!(hits, 2);
        assert_eq!(misses, 1);

        let hit_rate = get_cache_hit_rate();
        assert!((hit_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_hit_rate_zero_total() {
        reset_cache_stats();
        let hit_rate = get_cache_hit_rate();
        assert_eq!(hit_rate, 0.0);
    }
}

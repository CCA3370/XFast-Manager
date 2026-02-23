//! Cache performance metrics collection
//!
//! Tracks cache hit/miss rates for monitoring purposes.

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

/// Get cache hit rate (0.0 to 1.0)
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

use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Cached metadata for an archive file
#[derive(Clone, Debug)]
pub struct ArchiveMetadata {
    pub uncompressed_size: u64,
    pub file_count: usize,
    pub cached_at: SystemTime,
}

/// Global cache for archive metadata
/// Uses DashMap for thread-safe concurrent access without locks
static ARCHIVE_CACHE: Lazy<DashMap<String, ArchiveMetadata>> = Lazy::new(DashMap::new);

/// Cache TTL (Time To Live) - 5 minutes
const CACHE_TTL: Duration = Duration::from_secs(300);

/// Get cached metadata for an archive
pub fn get_cached_metadata(path: &Path) -> Option<ArchiveMetadata> {
    let key = path.to_string_lossy().to_string();

    if let Some(entry) = ARCHIVE_CACHE.get(&key) {
        let metadata = entry.value();

        // Check if cache is still valid
        if let Ok(elapsed) = metadata.cached_at.elapsed() {
            if elapsed < CACHE_TTL {
                crate::performance::record_cache_hit();
                return Some(metadata.clone());
            } else {
                // Cache expired, remove it
                drop(entry);
                ARCHIVE_CACHE.remove(&key);
            }
        }
    }

    crate::performance::record_cache_miss();
    None
}

/// Store metadata in cache
pub fn cache_metadata(path: &Path, uncompressed_size: u64, file_count: usize) {
    let key = path.to_string_lossy().to_string();
    let metadata = ArchiveMetadata {
        uncompressed_size,
        file_count,
        cached_at: SystemTime::now(),
    };

    ARCHIVE_CACHE.insert(key, metadata);
}

/// Clear expired cache entries
pub fn cleanup_cache() {
    ARCHIVE_CACHE.retain(|_, metadata| {
        if let Ok(elapsed) = metadata.cached_at.elapsed() {
            elapsed < CACHE_TTL
        } else {
            false
        }
    });
}

/// Clear all cache entries
pub fn clear_cache() {
    ARCHIVE_CACHE.clear();
}

/// Get cache statistics
pub fn get_cache_stats() -> (usize, usize) {
    let total = ARCHIVE_CACHE.len();
    let expired = ARCHIVE_CACHE
        .iter()
        .filter(|entry| {
            if let Ok(elapsed) = entry.value().cached_at.elapsed() {
                elapsed >= CACHE_TTL
            } else {
                true
            }
        })
        .count();

    (total, expired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::thread;

    #[test]
    fn test_cache_basic() {
        let path = PathBuf::from("/test/archive.zip");

        // Initially no cache
        assert!(get_cached_metadata(&path).is_none());

        // Cache some data
        cache_metadata(&path, 1000, 10);

        // Should retrieve cached data
        let cached = get_cached_metadata(&path).unwrap();
        assert_eq!(cached.uncompressed_size, 1000);
        assert_eq!(cached.file_count, 10);
    }

    #[test]
    fn test_cache_expiration() {
        let path = PathBuf::from("/test/expire.zip");

        // Cache with old timestamp
        let old_metadata = ArchiveMetadata {
            uncompressed_size: 500,
            file_count: 5,
            cached_at: SystemTime::now() - Duration::from_secs(400), // Older than TTL
        };

        ARCHIVE_CACHE.insert(path.to_string_lossy().to_string(), old_metadata);

        // Should return None due to expiration
        assert!(get_cached_metadata(&path).is_none());
    }

    #[test]
    fn test_cache_thread_safety() {
        let path = PathBuf::from("/test/concurrent.zip");

        // Spawn multiple threads writing to cache
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let p = path.clone();
                thread::spawn(move || {
                    cache_metadata(&p, i * 100, i);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Should have cached data (last write wins)
        assert!(get_cached_metadata(&path).is_some());
    }
}

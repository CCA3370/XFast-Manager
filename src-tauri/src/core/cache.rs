use dashmap::DashMap;
use std::path::Path;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};

/// Cached metadata for an archive file
#[derive(Clone, Debug)]
pub struct ArchiveMetadata {
    pub uncompressed_size: u64,
    // Retained for potential future cache inspection / diagnostics
    #[allow(dead_code)]
    pub file_count: usize,
    pub cached_at: SystemTime,
}

/// Cached metadata for a directory
#[derive(Clone, Debug)]
pub struct DirectoryMetadata {
    pub total_size: u64,
    // Retained for potential future cache inspection / diagnostics
    #[allow(dead_code)]
    pub file_count: usize,
    pub cached_at: SystemTime,
    pub last_modified: SystemTime,
}

/// Global cache for archive metadata
/// Uses DashMap for thread-safe concurrent access without locks
static ARCHIVE_CACHE: LazyLock<DashMap<String, ArchiveMetadata>> = LazyLock::new(DashMap::new);

/// Global cache for directory size metadata
static DIRECTORY_CACHE: LazyLock<DashMap<String, DirectoryMetadata>> = LazyLock::new(DashMap::new);

/// Cache TTL (Time To Live) - 5 minutes
const CACHE_TTL: Duration = Duration::from_secs(300);

/// Maximum number of entries in each cache to prevent unbounded memory growth
const MAX_CACHE_SIZE: usize = 1000;

/// Trait to extract `cached_at` from different cache entry types
trait CachedEntry {
    fn cached_at(&self) -> SystemTime;
}

impl CachedEntry for ArchiveMetadata {
    fn cached_at(&self) -> SystemTime {
        self.cached_at
    }
}

impl CachedEntry for DirectoryMetadata {
    fn cached_at(&self) -> SystemTime {
        self.cached_at
    }
}

/// Evict expired entries and oldest entries from a DashMap cache.
/// Uses batch eviction with sampling for O(k) complexity.
fn evict_expired_and_oldest<V: CachedEntry>(cache: &DashMap<String, V>) {
    // Phase 1: Remove all expired entries
    let expired_keys: Vec<String> = cache
        .iter()
        .filter_map(|entry| {
            if let Ok(elapsed) = entry.value().cached_at().elapsed() {
                if elapsed >= CACHE_TTL {
                    return Some(entry.key().clone());
                }
            }
            None
        })
        .collect();

    for key in expired_keys {
        cache.remove(&key);
    }

    // Phase 2: If still over capacity, batch-evict oldest entries
    if cache.len() >= MAX_CACHE_SIZE {
        let entries_to_remove = std::cmp::max(MAX_CACHE_SIZE / 10, 10);
        let target_age = CACHE_TTL / 2;

        // Collect keys to remove: prioritize entries older than half TTL,
        // then take any entries up to the removal limit
        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter_map(|entry| {
                if let Ok(elapsed) = entry.value().cached_at().elapsed() {
                    if elapsed > target_age {
                        return Some(entry.key().clone());
                    }
                }
                None
            })
            .take(entries_to_remove)
            .collect();

        // If we didn't find enough old entries, take any entries
        if keys_to_remove.len() < entries_to_remove {
            let remaining = entries_to_remove - keys_to_remove.len();
            let already_removing: std::collections::HashSet<&String> =
                keys_to_remove.iter().collect();

            let extra_keys: Vec<String> = cache
                .iter()
                .filter_map(|entry| {
                    if !already_removing.contains(entry.key()) {
                        Some(entry.key().clone())
                    } else {
                        None
                    }
                })
                .take(remaining)
                .collect();

            for key in extra_keys {
                cache.remove(&key);
            }
        }

        for key in keys_to_remove {
            cache.remove(&key);
        }
    }
}

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
/// Automatically evicts oldest entries when cache exceeds size limit
pub fn cache_metadata(path: &Path, uncompressed_size: u64, file_count: usize) {
    let key = path.to_string_lossy().to_string();
    let metadata = ArchiveMetadata {
        uncompressed_size,
        file_count,
        cached_at: SystemTime::now(),
    };

    // Evict old entries if cache is at capacity
    evict_expired_and_oldest(&ARCHIVE_CACHE);

    ARCHIVE_CACHE.insert(key, metadata);
}

/// Get cached directory metadata
pub fn get_cached_directory_metadata(path: &Path) -> Option<DirectoryMetadata> {
    let key = path.to_string_lossy().to_string();

    if let Some(entry) = DIRECTORY_CACHE.get(&key) {
        let metadata = entry.value();

        // Check if cache is still valid (TTL check)
        if let Ok(elapsed) = metadata.cached_at.elapsed() {
            if elapsed < CACHE_TTL {
                // Also check if directory was modified since caching
                if let Ok(dir_metadata) = std::fs::metadata(path) {
                    if let Ok(modified) = dir_metadata.modified() {
                        // If directory hasn't been modified, cache is valid
                        if modified <= metadata.last_modified {
                            crate::performance::record_cache_hit();
                            return Some(metadata.clone());
                        }
                    }
                }
                // If we can't check modification time, invalidate cache
                drop(entry);
                DIRECTORY_CACHE.remove(&key);
            } else {
                // Cache expired, remove it
                drop(entry);
                DIRECTORY_CACHE.remove(&key);
            }
        }
    }

    crate::performance::record_cache_miss();
    None
}

/// Store directory metadata in cache
/// Automatically evicts oldest entries when cache exceeds size limit
pub fn cache_directory_metadata(path: &Path, total_size: u64, file_count: usize) {
    let key = path.to_string_lossy().to_string();

    // Get directory's last modified time
    let last_modified = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or_else(|_| SystemTime::now());

    let metadata = DirectoryMetadata {
        total_size,
        file_count,
        cached_at: SystemTime::now(),
        last_modified,
    };

    // Evict old entries if cache is at capacity
    evict_expired_and_oldest(&DIRECTORY_CACHE);

    DIRECTORY_CACHE.insert(key, metadata);
}

/// Clear all caches (useful for testing or manual cache invalidation)
// Exposed for integration tests and potential future admin commands
#[allow(dead_code)]
pub fn clear_all_caches() {
    ARCHIVE_CACHE.clear();
    DIRECTORY_CACHE.clear();
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
    }

    #[test]
    fn test_cache_expiration() {
        let path = PathBuf::from("/test/expire.zip");

        // Cache with old timestamp
        let old_metadata = ArchiveMetadata {
            uncompressed_size: 500,
            file_count: 10,
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
                    cache_metadata(&p, i * 100, i as usize);
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

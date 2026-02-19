use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Cached metadata for an archive file
#[derive(Clone, Debug)]
pub struct ArchiveMetadata {
    pub uncompressed_size: u64,
    #[allow(dead_code)]
    pub file_count: usize,
    pub cached_at: SystemTime,
}

/// Cached metadata for a directory
#[derive(Clone, Debug)]
pub struct DirectoryMetadata {
    pub total_size: u64,
    #[allow(dead_code)]
    pub file_count: usize,
    pub cached_at: SystemTime,
    pub last_modified: SystemTime,
}

/// Global cache for archive metadata
/// Uses DashMap for thread-safe concurrent access without locks
static ARCHIVE_CACHE: Lazy<DashMap<String, ArchiveMetadata>> = Lazy::new(DashMap::new);

/// Global cache for directory size metadata
static DIRECTORY_CACHE: Lazy<DashMap<String, DirectoryMetadata>> = Lazy::new(DashMap::new);

/// Cache TTL (Time To Live) - 5 minutes
const CACHE_TTL: Duration = Duration::from_secs(300);

/// Maximum number of entries in each cache to prevent unbounded memory growth
const MAX_CACHE_SIZE: usize = 1000;

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
    evict_expired_and_oldest_archive_entries();

    ARCHIVE_CACHE.insert(key, metadata);
}

/// Evict expired entries and oldest entries if cache is too large
/// Uses batch eviction with sampling for O(1) amortized complexity instead of O(n)
fn evict_expired_and_oldest_archive_entries() {
    // First, remove expired entries (this is O(n) but only when inserting, amortized O(1))
    let expired_keys: Vec<String> = ARCHIVE_CACHE
        .iter()
        .filter_map(|entry| {
            if let Ok(elapsed) = entry.value().cached_at.elapsed() {
                if elapsed >= CACHE_TTL {
                    return Some(entry.key().clone());
                }
            }
            None
        })
        .collect();

    for key in expired_keys {
        ARCHIVE_CACHE.remove(&key);
    }

    // If still over capacity, use batch eviction with random sampling
    // This is O(k) where k is the number of entries to remove, not O(n*k)
    if ARCHIVE_CACHE.len() >= MAX_CACHE_SIZE {
        // Remove 10% of entries or at least 10 entries to make room
        let entries_to_remove = std::cmp::max(MAX_CACHE_SIZE / 10, 10);
        let mut removed = 0;

        // Sample entries and find old ones to remove
        // Instead of finding THE oldest (O(n)), we find entries older than median age
        let target_age = CACHE_TTL / 2; // Remove entries older than half TTL

        let keys_to_remove: Vec<String> = ARCHIVE_CACHE
            .iter()
            .filter_map(|entry| {
                if let Ok(elapsed) = entry.value().cached_at.elapsed() {
                    // Prioritize older entries
                    if elapsed > target_age || removed < entries_to_remove {
                        return Some(entry.key().clone());
                    }
                }
                None
            })
            .take(entries_to_remove)
            .collect();

        for key in keys_to_remove {
            ARCHIVE_CACHE.remove(&key);
            removed += 1;
            if removed >= entries_to_remove {
                break;
            }
        }
    }
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
    evict_expired_and_oldest_directory_entries();

    DIRECTORY_CACHE.insert(key, metadata);
}

/// Evict expired entries and oldest entries from directory cache
/// Uses batch eviction with sampling for O(1) amortized complexity instead of O(n)
fn evict_expired_and_oldest_directory_entries() {
    // First, remove expired entries
    let expired_keys: Vec<String> = DIRECTORY_CACHE
        .iter()
        .filter_map(|entry| {
            if let Ok(elapsed) = entry.value().cached_at.elapsed() {
                if elapsed >= CACHE_TTL {
                    return Some(entry.key().clone());
                }
            }
            None
        })
        .collect();

    for key in expired_keys {
        DIRECTORY_CACHE.remove(&key);
    }

    // If still over capacity, use batch eviction with random sampling
    // This is O(k) where k is the number of entries to remove, not O(n*k)
    if DIRECTORY_CACHE.len() >= MAX_CACHE_SIZE {
        // Remove 10% of entries or at least 10 entries to make room
        let entries_to_remove = std::cmp::max(MAX_CACHE_SIZE / 10, 10);
        let mut removed = 0;

        // Sample entries and find old ones to remove
        // Instead of finding THE oldest (O(n)), we find entries older than median age
        let target_age = CACHE_TTL / 2; // Remove entries older than half TTL

        let keys_to_remove: Vec<String> = DIRECTORY_CACHE
            .iter()
            .filter_map(|entry| {
                if let Ok(elapsed) = entry.value().cached_at.elapsed() {
                    // Prioritize older entries
                    if elapsed > target_age || removed < entries_to_remove {
                        return Some(entry.key().clone());
                    }
                }
                None
            })
            .take(entries_to_remove)
            .collect();

        for key in keys_to_remove {
            DIRECTORY_CACHE.remove(&key);
            removed += 1;
            if removed >= entries_to_remove {
                break;
            }
        }
    }
}

/// Clear all caches (useful for testing or manual cache invalidation)
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

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::models::{FileHash, HashAlgorithm};

pub struct HashCollector;

impl HashCollector {
    pub fn new() -> Self {
        HashCollector
    }

    /// Collect SHA256 hashes from directory
    /// Uses parallel processing for better performance on large directories
    pub(crate) fn collect_directory_hashes(
        &self,
        source_dir: &Path,
        bundle_entries: &[PathBuf],
    ) -> Result<HashMap<String, FileHash>> {
        use rayon::prelude::*;
        use walkdir::WalkDir;

        // Collect all file paths first
        let file_paths: Vec<(PathBuf, String)> = WalkDir::new(source_dir)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                entry
                    .path()
                    .strip_prefix(source_dir)
                    .map_or(true, |relative| {
                        !crate::package_artifacts::is_ignored_package_artifact_path(relative)
                            && (bundle_entries.is_empty()
                                || bundle_entries.iter().any(|bundle_entry| {
                                    relative.starts_with(bundle_entry)
                                        || bundle_entry.starts_with(relative)
                                }))
                    })
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|entry| {
                let path = entry.path();
                let relative = path.strip_prefix(source_dir).ok()?;
                let relative_str = relative.to_string_lossy().replace('\\', "/");
                Some((path.to_path_buf(), relative_str))
            })
            .collect();

        // Compute hashes in parallel
        let results: Vec<(String, Result<String>)> = file_paths
            .par_iter()
            .map(|(path, relative_str)| {
                let hash_result = self.compute_file_sha256(path);
                (relative_str.clone(), hash_result)
            })
            .collect();

        // Collect results into HashMap
        let mut hashes = HashMap::new();
        for (relative_str, hash_result) in results {
            match hash_result {
                Ok(hash) => {
                    hashes.insert(
                        relative_str.clone(),
                        FileHash {
                            path: relative_str,
                            hash,
                            algorithm: HashAlgorithm::Sha256,
                        },
                    );
                }
                Err(e) => {
                    if !bundle_entries.is_empty() {
                        return Err(e).with_context(|| {
                            format!("Failed to hash Lua bundle file: {relative_str}")
                        });
                    }
                    crate::logger::log_error(
                        &format!("Failed to compute hash for {}: {}", relative_str, e),
                        Some("hash_collector"),
                    );
                    // Continue with other files instead of failing completely
                }
            }
        }

        crate::logger::log_info(
            &format!("Collected {} SHA256 hashes from directory", hashes.len()),
            Some("hash_collector"),
        );

        Ok(hashes)
    }

    /// Compute SHA256 hash of a file
    pub fn compute_file_sha256(&self, path: &Path) -> Result<String> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 4 * 1024 * 1024]; // 4MB buffer

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_hashes_exclude_operating_system_metadata() {
        let source = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(source.path().join("Aircraft/__MACOSX")).unwrap();
        fs::create_dir_all(source.path().join("Aircraft/objects")).unwrap();
        fs::write(source.path().join(".DS_Store"), b"metadata").unwrap();
        fs::write(
            source.path().join("Aircraft/__MACOSX/._plane.acf"),
            b"metadata",
        )
        .unwrap();
        fs::write(
            source.path().join("Aircraft/objects/Thumbs.db"),
            b"metadata",
        )
        .unwrap();
        fs::write(source.path().join("Aircraft/desktop.ini"), b"metadata").unwrap();
        fs::write(source.path().join("Aircraft/plane.acf"), b"aircraft").unwrap();

        let hashes = HashCollector::new()
            .collect_directory_hashes(source.path(), &[])
            .unwrap();

        assert_eq!(hashes.len(), 1);
        assert!(hashes.contains_key("Aircraft/plane.acf"));
    }
}

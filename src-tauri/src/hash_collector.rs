use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};

use crate::models::{FileHash, HashAlgorithm, InstallTask};

pub struct HashCollector;

impl HashCollector {
    pub fn new() -> Self {
        HashCollector
    }

    /// Collect hashes for a task based on source type
    pub fn collect_hashes(&self, task: &InstallTask) -> Result<HashMap<String, FileHash>> {
        let source = Path::new(&task.source_path);

        if source.is_dir() {
            // Direct directory: compute SHA256 for all files
            self.collect_directory_hashes(source)
        } else if source.is_file() {
            // Archive: extract hashes based on format
            let ext = source.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            match ext {
                "zip" => self.collect_zip_hashes(
                    source,
                    task.archive_internal_root.as_deref(),
                    task.extraction_chain.as_ref(),
                    task.password.as_deref()
                ),
                "7z" => {
                    // 7z: Return empty map, compute during extraction
                    crate::logger::log_info(
                        "7z archives: hashes will be computed during extraction",
                        Some("hash_collector")
                    );
                    Ok(HashMap::new())
                },
                "rar" => self.collect_rar_hashes(
                    source,
                    task.archive_internal_root.as_deref(),
                    task.extraction_chain.as_ref(),
                    task.password.as_deref()
                ),
                _ => Ok(HashMap::new()),
            }
        } else {
            Ok(HashMap::new())
        }
    }

    /// Collect CRC32 hashes from ZIP archive
    /// Note: For nested archives (extraction_chain), this collects hashes from the outermost ZIP only.
    /// The final_internal_root is used to filter files, but nested ZIPs are not traversed.
    /// This is a known limitation - nested ZIP hash collection would require extracting intermediate layers.
    fn collect_zip_hashes(
        &self,
        archive_path: &Path,
        internal_root: Option<&str>,
        extraction_chain: Option<&crate::models::ExtractionChain>,
        _password: Option<&str>,
    ) -> Result<HashMap<String, FileHash>> {
        use zip::ZipArchive;

        let file = fs::File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut hashes = HashMap::new();

        // Determine the prefix to filter files
        let prefix = if let Some(chain) = extraction_chain {
            // For nested archives, use final_internal_root
            // Note: This only works if the nested structure is flat (no actual nesting)
            // True nested ZIPs (outer.zip containing inner.zip) are not fully supported
            chain.final_internal_root.as_ref().map(|s| s.replace('\\', "/"))
        } else {
            // For simple archives, use internal_root
            internal_root.map(|s| s.replace('\\', "/"))
        };

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().replace('\\', "/");

            // Skip directories
            if file.is_dir() {
                continue;
            }

            // Apply prefix filter
            let relative_path = if let Some(ref p) = prefix {
                if !name.starts_with(p) {
                    continue;
                }
                name.strip_prefix(p)
                    .and_then(|s| s.strip_prefix('/').or(Some(s)))
                    .unwrap_or(&name)
            } else {
                &name
            };

            // Get CRC32 from ZIP central directory
            let crc32 = file.crc32();

            hashes.insert(
                relative_path.to_string(),
                FileHash {
                    path: relative_path.to_string(),
                    hash: format!("{:08x}", crc32),
                    algorithm: HashAlgorithm::Crc32,
                }
            );
        }

        crate::logger::log_info(
            &format!("Collected {} CRC32 hashes from ZIP", hashes.len()),
            Some("hash_collector")
        );

        Ok(hashes)
    }

    /// Collect CRC32 hashes from RAR archive
    /// Note: unrar crate doesn't provide CRC32 in listing mode
    /// Hash verification for RAR archives is disabled
    fn collect_rar_hashes(
        &self,
        _archive_path: &Path,
        _internal_root: Option<&str>,
        _extraction_chain: Option<&crate::models::ExtractionChain>,
        _password: Option<&str>,
    ) -> Result<HashMap<String, FileHash>> {
        // Note: unrar crate doesn't provide CRC32 in listing mode
        // We'll compute hashes during extraction instead
        crate::logger::log_info(
            "RAR archives: hashes will be computed during extraction (unrar limitation)",
            Some("hash_collector")
        );
        Ok(HashMap::new())
    }

    /// Collect SHA256 hashes from directory
    /// Uses parallel processing for better performance on large directories
    fn collect_directory_hashes(
        &self,
        source_dir: &Path,
    ) -> Result<HashMap<String, FileHash>> {
        use walkdir::WalkDir;
        use rayon::prelude::*;

        // Collect all file paths first
        let file_paths: Vec<(PathBuf, String)> = WalkDir::new(source_dir)
            .follow_links(false)
            .into_iter()
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
                        }
                    );
                }
                Err(e) => {
                    crate::logger::log_error(
                        &format!("Failed to compute hash for {}: {}", relative_str, e),
                        Some("hash_collector")
                    );
                    // Continue with other files instead of failing completely
                }
            }
        }

        crate::logger::log_info(
            &format!("Collected {} SHA256 hashes from directory", hashes.len()),
            Some("hash_collector")
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

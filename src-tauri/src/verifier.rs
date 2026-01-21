use anyhow::Result;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::models::{FileHash, FileVerificationResult, HashAlgorithm, VerificationStats};

pub struct FileVerifier {
    /// Maximum retry attempts per file
    #[allow(dead_code)]
    max_retries: u8,
}

impl FileVerifier {
    pub fn new() -> Self {
        FileVerifier { max_retries: 3 }
    }

    /// Verify all files in target directory against expected hashes
    /// Returns list of failed files (empty if all passed)
    #[allow(dead_code)]
    pub fn verify_files(
        &self,
        target_dir: &Path,
        expected_hashes: &HashMap<String, FileHash>,
    ) -> Result<Vec<FileVerificationResult>> {
        self.verify_files_with_progress(target_dir, expected_hashes, |_, _| {})
    }

    /// Verify all files with progress callback
    /// progress_callback receives (verified_count, total_count)
    pub fn verify_files_with_progress<F>(
        &self,
        target_dir: &Path,
        expected_hashes: &HashMap<String, FileHash>,
        progress_callback: F,
    ) -> Result<Vec<FileVerificationResult>>
    where
        F: Fn(usize, usize) + Send + Sync,
    {
        use walkdir::WalkDir;

        crate::logger::log_info(
            &format!(
                "Verifying {} files in {:?}",
                expected_hashes.len(),
                target_dir
            ),
            Some("verifier"),
        );

        // Collect all files to verify
        let files_to_verify: Vec<(PathBuf, String)> = WalkDir::new(target_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|entry| {
                let path = entry.path();
                let relative = path.strip_prefix(target_dir).ok()?;
                let relative_str = relative.to_string_lossy().replace('\\', "/");

                // Only verify files we have hashes for
                if expected_hashes.contains_key(&relative_str) {
                    Some((path.to_path_buf(), relative_str))
                } else {
                    None
                }
            })
            .collect();

        let total = files_to_verify.len();
        let verified_count = Arc::new(AtomicUsize::new(0));

        // Parallel verification with progress tracking
        let results: Vec<FileVerificationResult> = files_to_verify
            .par_iter()
            .filter_map(|(path, relative_path)| {
                // Get expected hash (should always exist since we built files_to_verify from expected_hashes)
                let expected = expected_hashes.get(relative_path)?;
                let result = self.verify_single_file(path, relative_path, expected);

                // Update progress
                let count = verified_count.fetch_add(1, Ordering::SeqCst) + 1;
                progress_callback(count, total);

                Some(result)
            })
            .collect();

        // Filter failed files
        let failed: Vec<FileVerificationResult> =
            results.into_iter().filter(|r| !r.success).collect();

        if failed.is_empty() {
            crate::logger::log_info(
                &format!("All {} files verified successfully", files_to_verify.len()),
                Some("verifier"),
            );
        } else {
            crate::logger::log_error(
                &format!("{} files failed verification", failed.len()),
                Some("verifier"),
            );
        }

        Ok(failed)
    }

    /// Verify a single file
    pub fn verify_single_file(
        &self,
        file_path: &Path,
        relative_path: &str,
        expected: &FileHash,
    ) -> FileVerificationResult {
        match self.compute_file_hash(file_path, &expected.algorithm) {
            Ok(actual_hash) => {
                let success = actual_hash == expected.hash;

                if !success {
                    crate::logger::log_debug(
                        &format!(
                            "Hash mismatch: {} (expected: {}, actual: {})",
                            relative_path, expected.hash, actual_hash
                        ),
                        Some("verifier"),
                        None,
                    );
                }

                FileVerificationResult {
                    path: relative_path.to_string(),
                    expected_hash: expected.hash.clone(),
                    actual_hash: Some(actual_hash),
                    success,
                    retry_count: 0,
                    error: None,
                }
            }
            Err(e) => {
                crate::logger::log_error(
                    &format!("Failed to compute hash for {}: {}", relative_path, e),
                    Some("verifier"),
                );

                FileVerificationResult {
                    path: relative_path.to_string(),
                    expected_hash: expected.hash.clone(),
                    actual_hash: None,
                    success: false,
                    retry_count: 0,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    /// Compute hash of a file based on algorithm
    fn compute_file_hash(&self, path: &Path, algorithm: &HashAlgorithm) -> Result<String> {
        match algorithm {
            HashAlgorithm::Crc32 => self.compute_crc32(path),
            HashAlgorithm::Sha256 => self.compute_sha256(path),
        }
    }

    /// Compute CRC32 hash
    fn compute_crc32(&self, path: &Path) -> Result<String> {
        use crc32fast::Hasher;

        let mut file = fs::File::open(path)?;
        let mut hasher = Hasher::new();
        let mut buffer = vec![0u8; 4 * 1024 * 1024]; // 4MB buffer

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:08x}", hasher.finalize()))
    }

    /// Compute SHA256 hash
    pub fn compute_sha256(&self, path: &Path) -> Result<String> {
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

    /// Build verification statistics
    #[allow(dead_code)]
    pub fn build_stats(
        &self,
        total_expected: usize,
        failed: &[FileVerificationResult],
        retried: usize,
    ) -> VerificationStats {
        let failed_count = failed.len();
        let verified_count = total_expected - failed_count;

        VerificationStats {
            total_files: total_expected,
            verified_files: verified_count,
            failed_files: failed_count,
            retried_files: retried,
            skipped_files: 0,
        }
    }
}

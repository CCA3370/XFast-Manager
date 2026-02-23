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
    // Reserved for future retry logic on transient I/O failures
    #[allow(dead_code)]
    max_retries: u8,
}

impl FileVerifier {
    pub fn new() -> Self {
        FileVerifier { max_retries: 3 }
    }

    /// Verify all files in target directory against expected hashes
    /// Returns list of failed files (empty if all passed)
    // Kept for callers that don't need a progress callback
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
    // Exposed for callers that aggregate multiple verification passes
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_verifier_creation() {
        let verifier = FileVerifier::new();
        assert_eq!(verifier.max_retries, 3);
    }

    #[test]
    fn test_build_stats_all_passed() {
        let verifier = FileVerifier::new();
        let failed: Vec<FileVerificationResult> = vec![];

        let stats = verifier.build_stats(100, &failed, 0);

        assert_eq!(stats.total_files, 100);
        assert_eq!(stats.verified_files, 100);
        assert_eq!(stats.failed_files, 0);
        assert_eq!(stats.retried_files, 0);
        assert_eq!(stats.skipped_files, 0);
    }

    #[test]
    fn test_build_stats_some_failed() {
        let verifier = FileVerifier::new();
        let failed = vec![
            FileVerificationResult {
                path: "file1.txt".to_string(),
                expected_hash: "abc".to_string(),
                actual_hash: Some("def".to_string()),
                success: false,
                retry_count: 1,
                error: None,
            },
            FileVerificationResult {
                path: "file2.txt".to_string(),
                expected_hash: "123".to_string(),
                actual_hash: None,
                success: false,
                retry_count: 2,
                error: Some("File not found".to_string()),
            },
        ];

        let stats = verifier.build_stats(10, &failed, 3);

        assert_eq!(stats.total_files, 10);
        assert_eq!(stats.verified_files, 8);
        assert_eq!(stats.failed_files, 2);
        assert_eq!(stats.retried_files, 3);
    }

    #[test]
    fn test_compute_crc32() {
        let verifier = FileVerifier::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with known content
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        file.flush().unwrap();
        drop(file);

        let hash = verifier.compute_crc32(&file_path).unwrap();

        // CRC32 of "Hello, World!" is known
        assert_eq!(hash.len(), 8); // CRC32 as hex is 8 chars
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_sha256() {
        let verifier = FileVerifier::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with known content
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        file.flush().unwrap();
        drop(file);

        let hash = verifier.compute_sha256(&file_path).unwrap();

        // SHA256 hash is 64 hex characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        // Known SHA256 of "Hello, World!"
        assert_eq!(
            hash,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_verify_single_file_success() {
        let verifier = FileVerifier::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        file.flush().unwrap();
        drop(file);

        // Create expected hash (SHA256 of "Hello, World!")
        let expected = FileHash {
            path: "test.txt".to_string(),
            hash: "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".to_string(),
            algorithm: HashAlgorithm::Sha256,
        };

        let result = verifier.verify_single_file(&file_path, "test.txt", &expected);

        assert!(result.success);
        assert!(result.actual_hash.is_some());
        assert_eq!(result.actual_hash.unwrap(), expected.hash);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_verify_single_file_mismatch() {
        let verifier = FileVerifier::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        file.flush().unwrap();
        drop(file);

        // Create expected hash with wrong value
        let expected = FileHash {
            path: "test.txt".to_string(),
            hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            algorithm: HashAlgorithm::Sha256,
        };

        let result = verifier.verify_single_file(&file_path, "test.txt", &expected);

        assert!(!result.success);
        assert!(result.actual_hash.is_some());
        assert_ne!(result.actual_hash.unwrap(), expected.hash);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_verify_single_file_not_found() {
        let verifier = FileVerifier::new();
        let non_existent = PathBuf::from("/non/existent/file.txt");

        let expected = FileHash {
            path: "file.txt".to_string(),
            hash: "abc123".to_string(),
            algorithm: HashAlgorithm::Sha256,
        };

        let result = verifier.verify_single_file(&non_existent, "file.txt", &expected);

        assert!(!result.success);
        assert!(result.actual_hash.is_none());
        assert!(result.error.is_some());
    }

    #[test]
    fn test_crc32_empty_file() {
        let verifier = FileVerifier::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");

        // Create empty file
        fs::File::create(&file_path).unwrap();

        let hash = verifier.compute_crc32(&file_path).unwrap();

        // CRC32 of empty file is 00000000
        assert_eq!(hash, "00000000");
    }

    #[test]
    fn test_sha256_empty_file() {
        let verifier = FileVerifier::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");

        // Create empty file
        fs::File::create(&file_path).unwrap();

        let hash = verifier.compute_sha256(&file_path).unwrap();

        // SHA256 of empty string is known
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}

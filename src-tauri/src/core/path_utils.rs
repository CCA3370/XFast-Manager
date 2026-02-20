//! Shared path validation utilities
//!
//! Provides helpers for preventing path traversal attacks by canonicalizing
//! paths and verifying they remain within an expected base directory.

use std::io;
use std::path::{Path, PathBuf};

/// Canonicalize `candidate` and verify it is contained within `base`.
///
/// Returns the canonical form of `candidate` on success.
/// Returns an `io::Error` with `PermissionDenied` if the resolved path
/// escapes `base`, or any OS-level canonicalization error.
pub fn validate_child_path(base: &Path, candidate: &Path) -> io::Result<PathBuf> {
    let canonical_base = base.canonicalize()?;
    let canonical_candidate = candidate.canonicalize()?;
    if !canonical_candidate.starts_with(&canonical_base) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "path traversal attempt detected",
        ));
    }
    Ok(canonical_candidate)
}

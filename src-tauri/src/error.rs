use serde::{Deserialize, Serialize};
use std::fmt;

/// Structured error codes for API responses
/// These allow the frontend to distinguish between different error types
/// and display appropriate messages or take specific actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    /// Input validation failed (invalid path, malformed data, etc.)
    ValidationFailed,
    /// Permission denied (file access, admin rights, etc.)
    PermissionDenied,
    /// Resource not found (file, directory, entry doesn't exist)
    NotFound,
    /// Resource already exists (conflict during creation)
    ConflictExists,
    /// Data is corrupted or in unexpected format
    CorruptedData,
    /// Network-related error (download failed, connection issues)
    NetworkError,
    /// Archive-related error (extraction failed, password required)
    ArchiveError,
    /// Password required for encrypted archive
    PasswordRequired,
    /// Incorrect password provided
    IncorrectPassword,
    /// Installation was cancelled by user
    Cancelled,
    /// Disk space insufficient
    InsufficientSpace,
    /// Path traversal attack detected
    SecurityViolation,
    /// Operation timeout
    Timeout,
    /// Internal error (unexpected condition)
    Internal,
}

impl fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiErrorCode::ValidationFailed => write!(f, "validation_failed"),
            ApiErrorCode::PermissionDenied => write!(f, "permission_denied"),
            ApiErrorCode::NotFound => write!(f, "not_found"),
            ApiErrorCode::ConflictExists => write!(f, "conflict_exists"),
            ApiErrorCode::CorruptedData => write!(f, "corrupted_data"),
            ApiErrorCode::NetworkError => write!(f, "network_error"),
            ApiErrorCode::ArchiveError => write!(f, "archive_error"),
            ApiErrorCode::PasswordRequired => write!(f, "password_required"),
            ApiErrorCode::IncorrectPassword => write!(f, "incorrect_password"),
            ApiErrorCode::Cancelled => write!(f, "cancelled"),
            ApiErrorCode::InsufficientSpace => write!(f, "insufficient_space"),
            ApiErrorCode::SecurityViolation => write!(f, "security_violation"),
            ApiErrorCode::Timeout => write!(f, "timeout"),
            ApiErrorCode::Internal => write!(f, "internal"),
        }
    }
}

/// Structured API error with code, message, and optional details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code for programmatic handling
    pub code: ApiErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details (stack trace, field name, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ApiError {
    /// Create a new API error
    pub fn new(code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Create a new API error with details
    pub fn with_details(
        code: ApiErrorCode,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details.into()),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::ValidationFailed, message)
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::NotFound, message)
    }

    /// Create a permission denied error
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::PermissionDenied, message)
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::Internal, message)
    }

    /// Create a security violation error
    pub fn security_violation(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::SecurityViolation, message)
    }

    /// Create a password required error
    pub fn password_required(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::PasswordRequired, message)
    }

    /// Create an incorrect password error
    pub fn incorrect_password(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::IncorrectPassword, message)
    }

    /// Create a conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::ConflictExists, message)
    }

    /// Create a corrupted data error
    pub fn corrupted(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::CorruptedData, message)
    }

    /// Create an archive error
    pub fn archive(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::ArchiveError, message)
    }

    /// Create an insufficient space error
    pub fn insufficient_space(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::InsufficientSpace, message)
    }

    /// Create a cancelled error
    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::Cancelled, message)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(ref details) = self.details {
            write!(f, " ({})", details)?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

/// Convert from std::io::Error to ApiError
impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        let code = match err.kind() {
            std::io::ErrorKind::NotFound => ApiErrorCode::NotFound,
            std::io::ErrorKind::PermissionDenied => ApiErrorCode::PermissionDenied,
            std::io::ErrorKind::AlreadyExists => ApiErrorCode::ConflictExists,
            std::io::ErrorKind::TimedOut => ApiErrorCode::Timeout,
            _ => ApiErrorCode::Internal,
        };
        ApiError::new(code, err.to_string())
    }
}

/// Convert from anyhow::Error to ApiError
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        // Try to extract more specific error information
        let message = err.to_string();

        // Check for common error patterns
        if message.contains("password") || message.contains("encrypted") {
            if message.contains("wrong") || message.contains("incorrect") {
                return ApiError::incorrect_password(message);
            }
            return ApiError::password_required(message);
        }

        if message.contains("not found") || message.contains("does not exist") {
            return ApiError::not_found(message);
        }

        if message.contains("permission") || message.contains("access denied") {
            return ApiError::permission_denied(message);
        }

        if message.contains("traversal") || message.contains("security") {
            return ApiError::security_violation(message);
        }

        ApiError::internal(message)
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = std::result::Result<T, ApiError>;

/// Extension trait to convert ApiResult<T> to Result<T, String> for Tauri commands
/// This maintains backward compatibility while we migrate to ApiError
pub trait ToTauriError<T> {
    /// Convert to Tauri-compatible Result with string error
    fn to_tauri_error(self) -> std::result::Result<T, String>;
}

impl<T> ToTauriError<T> for ApiResult<T> {
    fn to_tauri_error(self) -> std::result::Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ApiError::validation("Invalid path");
        assert_eq!(err.code, ApiErrorCode::ValidationFailed);
        assert_eq!(err.message, "Invalid path");
        assert!(err.details.is_none());
    }

    #[test]
    fn test_error_with_details() {
        let err = ApiError::with_details(
            ApiErrorCode::NotFound,
            "File not found",
            "/path/to/file.txt",
        );
        assert_eq!(err.code, ApiErrorCode::NotFound);
        assert_eq!(err.details, Some("/path/to/file.txt".to_string()));
    }

    #[test]
    fn test_error_display() {
        let err = ApiError::new(ApiErrorCode::Internal, "Something went wrong");
        let display = format!("{}", err);
        assert!(display.contains("internal"));
        assert!(display.contains("Something went wrong"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let api_err: ApiError = io_err.into();
        assert_eq!(api_err.code, ApiErrorCode::NotFound);
    }

    #[test]
    fn test_serialization() {
        let err = ApiError::validation("test error");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("validation_failed"));
        assert!(json.contains("test error"));
    }
}

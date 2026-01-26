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
    /// Database error (SQLite operations)
    DatabaseError,
    /// Migration failed (schema upgrade failed)
    MigrationFailed,
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
            ApiErrorCode::DatabaseError => write!(f, "database_error"),
            ApiErrorCode::MigrationFailed => write!(f, "migration_failed"),
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
    ///
    /// This method is part of the public API for structured error handling.
    /// It allows attaching additional context (e.g., file paths, field names) to errors.
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
    ///
    /// Use when a resource already exists (e.g., file creation conflict).
    /// Part of the public API for structured error handling.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::ConflictExists, message)
    }

    /// Create a corrupted data error
    ///
    /// Use when data is in an unexpected format or corrupted.
    /// Part of the public API for structured error handling.
    pub fn corrupted(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::CorruptedData, message)
    }

    /// Create an archive error
    ///
    /// Use for archive extraction failures or format issues.
    /// Part of the public API for structured error handling.
    pub fn archive(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::ArchiveError, message)
    }

    /// Create an insufficient space error
    ///
    /// Use when there's not enough disk space for an operation.
    /// Part of the public API for structured error handling.
    pub fn insufficient_space(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::InsufficientSpace, message)
    }

    /// Create a cancelled error
    ///
    /// Use when an operation is cancelled by the user.
    /// Part of the public API for structured error handling.
    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::Cancelled, message)
    }

    /// Create a database error
    pub fn database(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::DatabaseError, message)
    }

    /// Create a migration failed error
    pub fn migration_failed(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::MigrationFailed, message)
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
        let message_lower = message.to_lowercase();

        // Check for common error patterns and use appropriate convenience methods

        // Password errors
        if message_lower.contains("password") || message_lower.contains("encrypted") {
            if message_lower.contains("wrong") || message_lower.contains("incorrect") {
                return ApiError::incorrect_password(message);
            }
            return ApiError::password_required(message);
        }

        // Cancellation errors
        if message_lower.contains("cancelled")
            || message_lower.contains("canceled")
            || message_lower.contains("aborted")
        {
            return ApiError::cancelled(message);
        }

        // Archive errors
        if message_lower.contains("archive")
            || message_lower.contains("zip")
            || message_lower.contains("extract")
            || message_lower.contains("decompress")
        {
            return ApiError::archive(message);
        }

        // Disk space errors
        if message_lower.contains("disk space")
            || message_lower.contains("insufficient space")
            || message_lower.contains("no space")
            || message_lower.contains("storage full")
        {
            return ApiError::insufficient_space(message);
        }

        // Corruption errors
        if message_lower.contains("corrupt")
            || message_lower.contains("malformed")
            || message_lower.contains("invalid format")
            || message_lower.contains("unexpected format")
        {
            return ApiError::corrupted(message);
        }

        // Conflict errors
        if message_lower.contains("already exists")
            || message_lower.contains("conflict")
            || message_lower.contains("duplicate")
        {
            return ApiError::conflict(message);
        }

        // Not found errors
        if message_lower.contains("not found") || message_lower.contains("does not exist") {
            return ApiError::not_found(message);
        }

        // Permission errors
        if message_lower.contains("permission") || message_lower.contains("access denied") {
            return ApiError::permission_denied(message);
        }

        // Security errors
        if message_lower.contains("traversal") || message_lower.contains("security") {
            return ApiError::security_violation(message);
        }

        ApiError::internal(message)
    }
}

/// Convert from rusqlite::Error to ApiError
impl From<rusqlite::Error> for ApiError {
    fn from(err: rusqlite::Error) -> Self {
        ApiError::new(ApiErrorCode::DatabaseError, err.to_string())
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = std::result::Result<T, ApiError>;

/// Extension trait to convert ApiResult<T> to Result<T, String> for Tauri commands
///
/// This trait provides backward compatibility for Tauri commands that expect
/// `Result<T, String>` while the internal code uses `ApiResult<T>`.
///
/// # Example
/// ```ignore
/// fn my_command() -> Result<(), String> {
///     do_something().to_tauri_error()
/// }
/// ```
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

    #[test]
    fn test_convenience_methods() {
        // Test all convenience methods to ensure they work correctly
        let conflict = ApiError::conflict("Resource already exists");
        assert_eq!(conflict.code, ApiErrorCode::ConflictExists);

        let corrupted = ApiError::corrupted("Data is corrupted");
        assert_eq!(corrupted.code, ApiErrorCode::CorruptedData);

        let archive = ApiError::archive("Failed to extract archive");
        assert_eq!(archive.code, ApiErrorCode::ArchiveError);

        let space = ApiError::insufficient_space("Not enough disk space");
        assert_eq!(space.code, ApiErrorCode::InsufficientSpace);

        let cancelled = ApiError::cancelled("Operation was cancelled");
        assert_eq!(cancelled.code, ApiErrorCode::Cancelled);

        let db = ApiError::database("Database connection failed");
        assert_eq!(db.code, ApiErrorCode::DatabaseError);

        let migration = ApiError::migration_failed("Migration failed");
        assert_eq!(migration.code, ApiErrorCode::MigrationFailed);
    }

    #[test]
    fn test_api_result_and_to_tauri_error() {
        // Test ApiResult type alias
        fn returns_api_result() -> ApiResult<i32> {
            Ok(42)
        }

        fn returns_api_error() -> ApiResult<i32> {
            Err(ApiError::validation("test"))
        }

        assert_eq!(returns_api_result().unwrap(), 42);
        assert!(returns_api_error().is_err());

        // Test ToTauriError trait
        let ok_result: ApiResult<i32> = Ok(42);
        let tauri_ok = ok_result.to_tauri_error();
        assert_eq!(tauri_ok.unwrap(), 42);

        let err_result: ApiResult<i32> = Err(ApiError::validation("test error"));
        let tauri_err = err_result.to_tauri_error();
        assert!(tauri_err.is_err());
        assert!(tauri_err.unwrap_err().contains("validation_failed"));
    }

    #[test]
    fn test_anyhow_error_conversion() {
        // Test that anyhow errors are converted to appropriate ApiError types

        // Archive error
        let anyhow_err = anyhow::anyhow!("Failed to extract ZIP file");
        let api_err: ApiError = anyhow_err.into();
        assert_eq!(api_err.code, ApiErrorCode::ArchiveError);

        // Cancellation error
        let anyhow_err = anyhow::anyhow!("Operation was cancelled by user");
        let api_err: ApiError = anyhow_err.into();
        assert_eq!(api_err.code, ApiErrorCode::Cancelled);

        // Disk space error
        let anyhow_err = anyhow::anyhow!("Insufficient disk space available");
        let api_err: ApiError = anyhow_err.into();
        assert_eq!(api_err.code, ApiErrorCode::InsufficientSpace);

        // Corruption error
        let anyhow_err = anyhow::anyhow!("Data is corrupted");
        let api_err: ApiError = anyhow_err.into();
        assert_eq!(api_err.code, ApiErrorCode::CorruptedData);

        // Conflict error
        let anyhow_err = anyhow::anyhow!("File already exists");
        let api_err: ApiError = anyhow_err.into();
        assert_eq!(api_err.code, ApiErrorCode::ConflictExists);

        // Not found error
        let anyhow_err = anyhow::anyhow!("File not found");
        let api_err: ApiError = anyhow_err.into();
        assert_eq!(api_err.code, ApiErrorCode::NotFound);

        // Generic error (should be Internal)
        let anyhow_err = anyhow::anyhow!("Some unknown error occurred");
        let api_err: ApiError = anyhow_err.into();
        assert_eq!(api_err.code, ApiErrorCode::Internal);
    }
}

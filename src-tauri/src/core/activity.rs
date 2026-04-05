//! Activity log helper — records user operations into the activity_log table.
//! Logging never causes the caller to fail.

use crate::database::entities::activity_log;
use crate::logger;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::time::{SystemTime, UNIX_EPOCH};

/// Record an activity log entry. Silently swallows errors to avoid disrupting the caller.
pub async fn log_activity(
    conn: &DatabaseConnection,
    operation: &str,
    item_type: &str,
    item_name: &str,
    details: Option<String>,
    success: bool,
) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let entry = activity_log::ActiveModel {
        timestamp: Set(timestamp),
        operation: Set(operation.to_string()),
        item_type: Set(item_type.to_string()),
        item_name: Set(item_name.to_string()),
        details: Set(details),
        success: Set(success),
        ..Default::default()
    };

    if let Err(e) = entry.insert(conn).await {
        logger::log_error(
            &format!("Failed to write activity log: {}", e),
            Some("activity"),
        );
    }
}

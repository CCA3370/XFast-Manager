use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::app_dirs;

/// Update information returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub is_update_available: bool,
    pub release_notes: String,
    pub release_url: String,
    pub published_at: String,
}

/// Remote release API response structure
#[derive(Debug, Deserialize)]
struct RemoteRelease {
    tag_name: String,
    #[allow(dead_code)]
    name: String,
    body: Option<String>,
    #[allow(dead_code)]
    prerelease: bool,
    published_at: String,
    html_url: String,
}

/// Update checker
pub struct UpdateChecker;

impl UpdateChecker {
    /// Create a new update checker
    pub fn new() -> Self {
        Self
    }

    /// Check for updates
    pub async fn check_for_updates(
        &self,
        _manual: bool,
        include_pre_release: bool,
    ) -> Result<UpdateInfo, String> {
        // Get current version
        let current_version = env!("CARGO_PKG_VERSION").to_string();

        // Fetch latest release from proxy API
        let latest_release = self.fetch_latest_release(include_pre_release).await?;

        // Parse version numbers (remove 'v' prefix if present)
        let latest_version = latest_release.tag_name.trim_start_matches('v').to_string();

        // Compare versions
        let is_update_available = self.compare_versions(&current_version, &latest_version)?;

        // Update last check time
        self.update_last_check_time();

        // Build update info
        let update_info = UpdateInfo {
            current_version,
            latest_version: latest_version.clone(),
            is_update_available,
            release_notes: latest_release.body.unwrap_or_default(),
            release_url: latest_release.html_url,
            published_at: latest_release.published_at,
        };

        if is_update_available {
            crate::logger::log_info(
                &format!(
                    "Update available: {} -> {}",
                    update_info.current_version, update_info.latest_version
                ),
                Some("updater"),
            );
        } else {
            crate::logger::log_info("No update available", Some("updater"));
        }

        Ok(update_info)
    }

    fn update_release_api_url(include_pre_release: bool) -> String {
        let base = crate::vercel_api::endpoint_with_override(
            "update-release",
            "XFAST_UPDATE_RELEASE_API_URL",
        );
        crate::vercel_api::append_query_param(
            &base,
            "includePreRelease",
            if include_pre_release { "1" } else { "0" },
        )
    }

    /// Fetch latest release from proxy API
    async fn fetch_latest_release(
        &self,
        include_pre_release: bool,
    ) -> Result<RemoteRelease, String> {
        crate::logger::log_debug(
            &format!(
                "Fetching release metadata (include_pre_release: {})",
                include_pre_release
            ),
            Some("updater"),
            None,
        );

        // Fetch release metadata through the backend HTTP client.
        let client = reqwest::Client::builder()
            .user_agent("XFast Manager")
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let url = Self::update_release_api_url(include_pre_release);
        crate::logger::log_debug(
            &format!("Fetching from proxy: {}", url),
            Some("updater"),
            None,
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch release metadata: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!(
                "Update API returned status: {} ({})",
                status, error_text
            ));
        }

        response
            .json::<RemoteRelease>()
            .await
            .map_err(|e| format!("Failed to parse release metadata: {}", e))
    }

    /// Compare two version strings using semver
    fn compare_versions(&self, current: &str, latest: &str) -> Result<bool, String> {
        let current_ver = semver::Version::parse(current)
            .map_err(|e| format!("Failed to parse current version: {}", e))?;

        let latest_ver = semver::Version::parse(latest)
            .map_err(|e| format!("Failed to parse latest version: {}", e))?;

        Ok(latest_ver > current_ver)
    }

    /// Get last check time from localStorage (via app data directory)
    fn get_last_check_time(&self) -> Option<u64> {
        let cache_file = app_dirs::get_update_cache_path();

        if let Ok(content) = std::fs::read_to_string(cache_file) {
            content.trim().parse().ok()
        } else {
            None
        }
    }

    /// Update last check time
    fn update_last_check_time(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cache_file = app_dirs::get_update_cache_path();
        // Ensure parent directory exists
        if let Some(parent) = cache_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(cache_file, now.to_string());
    }
}

/// Get last check time (for frontend)
pub fn get_last_check_time() -> Option<i64> {
    let checker = UpdateChecker::new();
    checker.get_last_check_time().map(|t| t as i64)
}

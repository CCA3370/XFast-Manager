use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// GitHub Release API response structure
#[derive(Debug, Deserialize)]
struct GitHubRelease {
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
pub struct UpdateChecker {
    repo_owner: String,
    repo_name: String,
    cache_duration: Duration,
}

impl UpdateChecker {
    /// Create a new update checker
    pub fn new() -> Self {
        Self {
            repo_owner: "CCA3370".to_string(),
            repo_name: "XFastInstall-Tauri".to_string(),
            cache_duration: Duration::from_secs(24 * 60 * 60), // 24 hours
        }
    }

    /// Check for updates
    pub async fn check_for_updates(
        &self,
        manual: bool,
        include_pre_release: bool,
    ) -> Result<UpdateInfo, String> {
        // Get current version
        let current_version = env!("CARGO_PKG_VERSION").to_string();

        // Check if we should skip the check (cache)
        if !manual && !self.should_check_update() {
            crate::logger::log_debug(
                "Skipping update check (cache not expired)",
                Some("updater"),
                None,
            );
            return Err("Cache not expired".to_string());
        }

        // Fetch latest release from GitHub
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

    /// Fetch latest release from GitHub API
    async fn fetch_latest_release(
        &self,
        include_pre_release: bool,
    ) -> Result<GitHubRelease, String> {
        crate::logger::log_debug(
            &format!(
                "Fetching releases (include_pre_release: {})",
                include_pre_release
            ),
            Some("updater"),
            None,
        );

        // Use tauri-plugin-http to make the request
        let client = reqwest::Client::builder()
            .user_agent("XFastInstall")
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        if include_pre_release {
            // Get all releases and filter for the latest (including pre-releases)
            let url = format!(
                "https://api.github.com/repos/{}/{}/releases",
                self.repo_owner, self.repo_name
            );

            crate::logger::log_debug(&format!("Fetching from: {}", url), Some("updater"), None);

            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch releases: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("GitHub API returned status: {}", response.status()));
            }

            let releases: Vec<GitHubRelease> = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse releases: {}", e))?;

            releases
                .into_iter()
                .next()
                .ok_or_else(|| "No releases found".to_string())
        } else {
            // Try to get latest stable release first
            let latest_url = format!(
                "https://api.github.com/repos/{}/{}/releases/latest",
                self.repo_owner, self.repo_name
            );

            crate::logger::log_debug(
                &format!("Fetching from: {}", latest_url),
                Some("updater"),
                None,
            );

            let response = client.get(&latest_url).send().await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    // Successfully got latest stable release
                    resp.json()
                        .await
                        .map_err(|e| format!("Failed to parse release: {}", e))
                }
                Ok(resp) if resp.status().as_u16() == 404 => {
                    // No stable release found, try to get all releases and filter non-prerelease
                    crate::logger::log_debug(
                        "No stable release found, fetching all releases",
                        Some("updater"),
                        None,
                    );

                    let all_url = format!(
                        "https://api.github.com/repos/{}/{}/releases",
                        self.repo_owner, self.repo_name
                    );

                    let all_response = client
                        .get(&all_url)
                        .send()
                        .await
                        .map_err(|e| format!("Failed to fetch all releases: {}", e))?;

                    if !all_response.status().is_success() {
                        return Err(format!(
                            "GitHub API returned status: {}",
                            all_response.status()
                        ));
                    }

                    let releases: Vec<GitHubRelease> = all_response
                        .json()
                        .await
                        .map_err(|e| format!("Failed to parse releases: {}", e))?;

                    // Filter for non-prerelease versions
                    let stable_release = releases.into_iter().find(|r| !r.prerelease);

                    match stable_release {
                        Some(release) => Ok(release),
                        None => {
                            // No stable releases found, return a dummy release with current version
                            // This will result in "no update available" message
                            crate::logger::log_debug(
                                "No stable releases found, returning current version",
                                Some("updater"),
                                None,
                            );
                            Ok(GitHubRelease {
                                tag_name: format!("v{}", env!("CARGO_PKG_VERSION")),
                                name: "Current Version".to_string(),
                                body: None,
                                prerelease: false,
                                published_at: "1970-01-01T00:00:00Z".to_string(), // Placeholder date
                                html_url: format!(
                                    "https://github.com/{}/{}/releases",
                                    self.repo_owner, self.repo_name
                                ),
                            })
                        }
                    }
                }
                Ok(resp) => Err(format!("GitHub API returned status: {}", resp.status())),
                Err(e) => Err(format!("Failed to fetch release: {}", e)),
            }
        }
    }

    /// Compare two version strings using semver
    fn compare_versions(&self, current: &str, latest: &str) -> Result<bool, String> {
        let current_ver = semver::Version::parse(current)
            .map_err(|e| format!("Failed to parse current version: {}", e))?;

        let latest_ver = semver::Version::parse(latest)
            .map_err(|e| format!("Failed to parse latest version: {}", e))?;

        Ok(latest_ver > current_ver)
    }

    /// Check if we should perform an update check (based on cache)
    fn should_check_update(&self) -> bool {
        let last_check = self.get_last_check_time();

        match last_check {
            Some(last) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let elapsed = Duration::from_secs(now - last);
                elapsed >= self.cache_duration
            }
            None => true, // Never checked before
        }
    }

    /// Get last check time from localStorage (via app data directory)
    fn get_last_check_time(&self) -> Option<u64> {
        // For simplicity, we'll use a file in the app data directory
        let app_dir = dirs::data_local_dir()?;
        let xfastinstall_dir = app_dir.join("XFastInstall");
        let cache_file = xfastinstall_dir.join("update_check_cache.txt");

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
            .unwrap()
            .as_secs();

        if let Some(app_dir) = dirs::data_local_dir() {
            let xfastinstall_dir = app_dir.join("XFastInstall");
            let _ = std::fs::create_dir_all(&xfastinstall_dir);
            let cache_file = xfastinstall_dir.join("update_check_cache.txt");
            let _ = std::fs::write(cache_file, now.to_string());
        }
    }
}

/// Get last check time (for frontend)
pub fn get_last_check_time() -> Option<i64> {
    let checker = UpdateChecker::new();
    checker.get_last_check_time().map(|t| t as i64)
}

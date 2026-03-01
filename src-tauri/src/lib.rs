// Core/shared
#[path = "core/app_dirs.rs"]
mod app_dirs;
#[path = "core/cache.rs"]
mod cache;
#[path = "core/error.rs"]
mod error;
#[path = "core/logger.rs"]
mod logger;
#[path = "core/path_utils.rs"]
mod path_utils;
#[path = "core/performance.rs"]
mod performance;
#[path = "core/registry.rs"]
mod registry;
#[path = "core/task_control.rs"]
mod task_control;

// Data
#[path = "data/database/mod.rs"]
mod database;
#[path = "data/models.rs"]
mod models;

// Analysis & scanning
#[path = "analysis/analyzer.rs"]
mod analyzer;
#[path = "analysis/crash_analysis.rs"]
mod crash_analysis;
#[path = "analysis/hash_collector.rs"]
mod hash_collector;
#[path = "analysis/livery_patterns.rs"]
mod livery_patterns;
#[path = "analysis/scanner/mod.rs"]
mod scanner;

// Installation
#[path = "install/atomic_installer.rs"]
mod atomic_installer;
#[path = "install/installer/mod.rs"]
mod installer;
#[path = "install/verifier.rs"]
mod verifier;

// Management
#[path = "management/management_index.rs"]
mod management_index;
#[path = "management/addon_updater.rs"]
mod addon_updater;
#[path = "management/skunk_updater.rs"]
mod skunk_updater;
#[path = "management/x_updater_profile.rs"]
mod x_updater_profile;

// Scenery
#[path = "scenery/geo_regions.rs"]
mod geo_regions;
#[path = "scenery/scenery_classifier.rs"]
mod scenery_classifier;
#[path = "scenery/scenery_index.rs"]
mod scenery_index;
#[path = "scenery/scenery_packs_manager.rs"]
mod scenery_packs_manager;

// Services (remote/data)
#[path = "services/library_links.rs"]
mod library_links;
#[path = "services/updater.rs"]
mod updater;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::ToTauriError;
use analyzer::Analyzer;
use installer::Installer;
use models::{
    AircraftInfo, AnalysisResult, InstallResult, InstallTask, LiveryInfo, LuaScriptInfo,
    ManagementData, NavdataBackupInfo, NavdataManagerInfo, PluginInfo, SceneryIndexScanResult,
    SceneryIndexStats, SceneryIndexStatus, SceneryManagerData, SceneryPackageInfo,
};
use scenery_index::SceneryIndexManager;
use scenery_packs_manager::SceneryPacksManager;
use task_control::TaskControl;

use sea_orm::DatabaseConnection;
use tauri::{Emitter, Manager, State};

use database::DatabaseState;

/// Cross-platform helper to open a path in the system file explorer
fn open_in_explorer<P: AsRef<std::path::Path>>(path: P) -> Result<(), String> {
    let path = path.as_ref();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try common file managers first to avoid xdg-open opening in a browser
        let opened = ["nautilus", "dolphin", "thunar", "nemo", "pcmanfm"]
            .iter()
            .any(|fm| std::process::Command::new(fm).arg(path).spawn().is_ok());
        if !opened {
            // Fallback to xdg-open
            std::process::Command::new("xdg-open")
                .arg(path)
                .spawn()
                .map_err(|e| format!("Failed to open folder: {}", e))?;
        }
    }

    Ok(())
}

// ============================================================================
// System & Utility Commands
// ============================================================================

#[tauri::command]
fn get_cli_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    opener::open(&url).map_err(|e| format!("Failed to open URL: {}", e))
}

#[tauri::command]
async fn create_library_link_issue(
    library_name: String,
    download_url: String,
    referenced_by: Option<String>,
) -> Result<String, String> {
    let library_name = library_name.trim();
    let download_url = download_url.trim();

    if library_name.is_empty() {
        return Err("Library name is empty".to_string());
    }

    let parsed_url =
        reqwest::Url::parse(download_url).map_err(|_| "Download URL is invalid".to_string())?;
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err("Download URL must be http/https".to_string());
    }

    let api_url = std::env::var("XFAST_LINK_API_URL")
        .unwrap_or_else(|_| "https://x-fast-manager.vercel.app/api/library-link".to_string());

    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&api_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "libraryName": library_name,
            "downloadUrl": download_url,
            "referencedBy": referenced_by
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to create issue: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Link API error {}: {}", status, error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let issue_url = response_json
        .get("issueUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if issue_url.is_empty() {
        return Err("Issue created but response URL missing".to_string());
    }

    Ok(issue_url)
}

#[derive(serde::Serialize)]
struct BugReportResult {
    issue_url: String,
    issue_number: u64,
}

#[derive(serde::Serialize)]
struct FeedbackIssueResult {
    issue_url: String,
    issue_number: u64,
    issue_title: String,
}

#[tauri::command]
async fn create_bug_report_issue(
    error_title: String,
    error_message: String,
    logs: Option<String>,
    category: Option<String>,
) -> Result<BugReportResult, String> {
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();

    let api_url = std::env::var("XFAST_BUG_REPORT_API_URL")
        .unwrap_or_else(|_| "https://x-fast-manager.vercel.app/api/bug-report".to_string());

    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&api_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "appVersion": app_version,
            "os": os,
            "arch": arch,
            "errorTitle": error_title.trim(),
            "errorMessage": error_message.trim(),
            "logs": logs.as_deref().unwrap_or(""),
            "category": category.as_deref().unwrap_or("Other")
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to submit bug report: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Bug report API error {}: {}", status, error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let issue_url = response_json
        .get("issueUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if issue_url.is_empty() {
        return Err("Bug report created but response URL missing".to_string());
    }

    let issue_number = response_json
        .get("issueNumber")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Fallback: if the server didn't return issueNumber, extract it from URL query or tail.
    let issue_number = if issue_number == 0 {
        reqwest::Url::parse(&issue_url)
            .ok()
            .and_then(|url| {
                url.query_pairs()
                    .find(|(k, _)| k == "number" || k == "issueNumber")
                    .and_then(|(_, v)| v.parse::<u64>().ok())
                    .or_else(|| {
                        url.path_segments()
                            .and_then(|segments| segments.last())
                            .and_then(|s| s.parse::<u64>().ok())
                    })
            })
            .unwrap_or(0)
    } else {
        issue_number
    };

    Ok(BugReportResult {
        issue_url,
        issue_number,
    })
}

#[tauri::command]
async fn create_feedback_issue(
    feedback_title: String,
    feedback_type: String,
    feedback_content: String,
) -> Result<FeedbackIssueResult, String> {
    let feedback_title = feedback_title.trim();
    let feedback_content = feedback_content.trim();
    if feedback_title.is_empty() {
        return Err("Feedback title is required".to_string());
    }
    if feedback_content.is_empty() {
        return Err("Feedback content is required".to_string());
    }

    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let feedback_type = feedback_type.trim().to_lowercase();

    let api_url = std::env::var("XFAST_FEEDBACK_API_URL")
        .unwrap_or_else(|_| "https://x-fast-manager.vercel.app/api/feedback-issue".to_string());

    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&api_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "title": feedback_title,
            "type": feedback_type,
            "content": feedback_content,
            "appVersion": app_version,
            "os": os,
            "arch": arch
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to submit feedback: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Feedback API error {}: {}", status, error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let issue_url = response_json
        .get("issueUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if issue_url.is_empty() {
        return Err("Feedback created but response URL missing".to_string());
    }

    let issue_number = response_json
        .get("issueNumber")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let issue_number = if issue_number == 0 {
        reqwest::Url::parse(&issue_url)
            .ok()
            .and_then(|url| {
                url.query_pairs()
                    .find(|(k, _)| k == "number" || k == "issueNumber")
                    .and_then(|(_, v)| v.parse::<u64>().ok())
                    .or_else(|| {
                        url.path_segments()
                            .and_then(|segments| segments.last())
                            .and_then(|s| s.parse::<u64>().ok())
                    })
            })
            .unwrap_or(0)
    } else {
        issue_number
    };

    Ok(FeedbackIssueResult {
        issue_url,
        issue_number,
        issue_title: feedback_title.to_string(),
    })
}

#[derive(serde::Serialize)]
struct IssueCommentPostResult {
    ok: bool,
}

#[tauri::command]
async fn post_issue_comment(issue_number: u64, comment_body: String) -> Result<IssueCommentPostResult, String> {
    if issue_number == 0 {
        return Err("issue_number must be greater than 0".to_string());
    }

    let body = comment_body.trim();
    if body.is_empty() {
        return Err("comment_body is required".to_string());
    }

    let api_url = std::env::var("XFAST_ISSUE_COMMENT_API_URL")
        .unwrap_or_else(|_| "https://x-fast-manager.vercel.app/api/issue-comment".to_string());

    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&api_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "issueNumber": issue_number,
            "commentBody": body
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to submit issue comment: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Issue comment API error {}: {}", status, error_text));
    }

    Ok(IssueCommentPostResult { ok: true })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IssueCommentInfo {
    author: String,
    body: String,
    created_at: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IssueDetailInfo {
    number: u64,
    title: String,
    state: String,
    html_url: String,
    comments: u64,
    created_at: String,
    updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IssueDetailCommentInfo {
    id: u64,
    author: String,
    body: String,
    created_at: String,
    updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IssueDetailResult {
    issue: IssueDetailInfo,
    comments: Vec<IssueDetailCommentInfo>,
    page: u32,
    per_page: u32,
    has_more: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IssueUpdateResult {
    state: String,
    total_comments: u64,
    new_comments: Vec<IssueCommentInfo>,
}

fn issue_updates_api_url() -> String {
    std::env::var("XFAST_ISSUE_UPDATES_API_URL")
        .unwrap_or_else(|_| "https://x-fast-manager.vercel.app/api/issue-updates".to_string())
}

fn issue_detail_api_url() -> String {
    std::env::var("XFAST_ISSUE_DETAIL_API_URL")
        .unwrap_or_else(|_| "https://x-fast-manager.vercel.app/api/issue-detail".to_string())
}

#[tauri::command]
async fn check_issue_updates(
    issue_number: u64,
    since: String,
) -> Result<IssueUpdateResult, String> {
    if issue_number == 0 {
        return Err("issue_number must be greater than 0".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let api_url = issue_updates_api_url();
    let response = client
        .get(&api_url)
        .query(&[
            ("issueNumber", issue_number.to_string()),
            ("since", since.clone()),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to fetch issue updates: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Issue updates API error {}: {}", status, body));
    }

    response
        .json::<IssueUpdateResult>()
        .await
        .map_err(|e| format!("Failed to parse issue updates response: {}", e))
}

#[tauri::command]
async fn get_issue_detail(
    issue_number: u64,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<IssueDetailResult, String> {
    if issue_number == 0 {
        return Err("issue_number must be greater than 0".to_string());
    }

    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(30).clamp(1, 100);

    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let api_url = issue_detail_api_url();
    let response = client
        .get(&api_url)
        .query(&[
            ("issueNumber", issue_number.to_string()),
            ("page", page.to_string()),
            ("perPage", per_page.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to fetch issue detail: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Issue detail API error {}: {}", status, body));
    }

    response
        .json::<IssueDetailResult>()
        .await
        .map_err(|e| format!("Failed to parse issue detail response: {}", e))
}

// ============================================================================
// Installation Commands
// ============================================================================

#[tauri::command]
async fn analyze_addons(
    paths: Vec<String>,
    xplane_path: String,
    passwords: Option<HashMap<String, String>>,
    verification_preferences: Option<HashMap<String, bool>>,
) -> Result<AnalysisResult, String> {
    livery_patterns::ensure_patterns_loaded().await;

    // Run the analysis in a blocking thread pool to avoid blocking the async runtime
    tokio::task::spawn_blocking(move || {
        log_debug!(&format!("Analyzing paths: {:?}", paths), "analysis");
        log_debug!(
            &format!("Starting analysis with X-Plane path: {}", xplane_path),
            "analysis"
        );

        let analyzer = Analyzer::new();
        Ok(analyzer.analyze(paths, &xplane_path, passwords, verification_preferences))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn install_addons(
    app_handle: tauri::AppHandle,
    tasks: Vec<InstallTask>,
    atomic_install_enabled: Option<bool>,
    xplane_path: String,
    delete_source_after_install: Option<bool>,
    auto_sort_scenery: Option<bool>,
    parallel_enabled: Option<bool>,
    max_parallel: Option<usize>,
) -> Result<InstallResult, String> {
    log_debug!(
        &format!(
            "Installing {} tasks: {}",
            tasks.len(),
            tasks
                .iter()
                .map(|t| &t.display_name)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ),
        "installation"
    );

    let installer = Installer::new(app_handle);

    if parallel_enabled.unwrap_or(false) && tasks.len() > 1 {
        installer
            .install_parallel(
                tasks,
                max_parallel.unwrap_or(3),
                atomic_install_enabled.unwrap_or(false),
                xplane_path,
                delete_source_after_install.unwrap_or(false),
                auto_sort_scenery.unwrap_or(false),
            )
            .await
            .map_err(|e| format!("Installation failed: {}", e))
    } else {
        installer
            .install(
                tasks,
                atomic_install_enabled.unwrap_or(false),
                xplane_path,
                delete_source_after_install.unwrap_or(false),
                auto_sort_scenery.unwrap_or(false),
            )
            .await
            .map_err(|e| format!("Installation failed: {}", e))
    }
}

// ============================================================================
// Task Control Commands
// ============================================================================

#[tauri::command]
async fn cancel_installation(task_control: State<'_, TaskControl>) -> Result<(), String> {
    task_control.request_cancel_all();
    logger::log_info("Installation cancellation requested", Some("task_control"));
    Ok(())
}

#[tauri::command]
async fn skip_current_task(task_control: State<'_, TaskControl>) -> Result<(), String> {
    task_control.request_skip_current();
    logger::log_info("Current task skip requested", Some("task_control"));
    Ok(())
}

// ============================================================================
// Windows Registry Commands (Context Menu)
// ============================================================================

#[tauri::command]
fn register_context_menu() -> Result<(), String> {
    registry::register_context_menu().map_err(|e| format!("Failed to register context menu: {}", e))
}

#[tauri::command]
fn unregister_context_menu() -> Result<(), String> {
    registry::unregister_context_menu()
        .map_err(|e| format!("Failed to unregister context menu: {}", e))
}

#[tauri::command]
fn is_context_menu_registered() -> bool {
    registry::is_context_menu_registered()
}

#[tauri::command]
fn sync_context_menu_paths() -> Result<bool, String> {
    registry::sync_registry_paths().map_err(|e| format!("Failed to sync context menu paths: {}", e))
}

// ============================================================================
// Logging Commands
// ============================================================================

#[tauri::command]
fn log_from_frontend(level: String, message: String, context: Option<String>) {
    let ctx = context.as_deref();
    match level.to_lowercase().as_str() {
        "error" => logger::log_error(&message, ctx),
        "debug" => {
            // For frontend debug logs, we don't have file/line info, so pass None
            logger::log_debug(&message, ctx, Some("frontend"))
        }
        _ => logger::log_info(&message, ctx),
    }
}

#[tauri::command]
fn get_recent_logs(lines: Option<usize>) -> Vec<String> {
    logger::get_recent_logs(lines.unwrap_or(50))
}

#[tauri::command]
fn get_log_path() -> String {
    logger::get_log_path().to_string_lossy().to_string()
}

#[tauri::command]
fn get_all_logs() -> String {
    logger::get_all_logs()
}

#[tauri::command]
fn open_log_folder() -> Result<(), String> {
    open_in_explorer(logger::get_log_folder())
}

// ========== X-Plane Log Analysis ==========

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct LogIssue {
    /// i18n category key, e.g. "crash", "plugin_error"
    category: String,
    /// "high" | "medium" | "low"
    severity: String,
    /// Up to 5 matching line numbers
    line_numbers: Vec<usize>,
    /// For E/ lines: all consecutive E/ lines in the same block (multi-line).
    /// For other lines: the single matching line.
    sample_line: String,
}

#[derive(serde::Serialize, Default)]
struct SystemInfo {
    xplane_version: Option<String>,
    gpu_model: Option<String>,
    gpu_driver: Option<String>,
}

#[derive(serde::Serialize)]
struct XPlaneLogAnalysis {
    log_path: String,
    is_xplane_log: bool,
    crash_detected: bool,
    crash_info: Option<String>,
    issues: Vec<LogIssue>,
    system_info: SystemInfo,
    total_high: usize,
    total_medium: usize,
    total_low: usize,
}

type MatcherFn = Box<dyn Fn(&str, &str) -> bool>;

struct Pattern {
    category: &'static str,
    severity: &'static str,
    matcher: MatcherFn,
}

fn build_patterns() -> Vec<Pattern> {
    vec![
        // ===== HIGH =====
        Pattern {
            category: "crash",
            severity: "high",
            matcher: Box::new(|l, _| l.contains("This application has crashed")),
        },
        Pattern {
            category: "plugin_manager_error",
            severity: "high",
            matcher: Box::new(|l, _| {
                l.contains("MACIBM_alert") && l.contains("plugin manager internal error")
            }),
        },
        Pattern {
            category: "scenery_load_failed",
            severity: "high",
            matcher: Box::new(|l, _| {
                l.contains("MACIBM_alert: \u{4ee5}\u{4e0b}\u{5730}\u{666f}\u{5305}\u{6709}\u{95ee}\u{9898}")
            }),
        },
        Pattern {
            category: "aircraft_incompatible",
            severity: "high",
            matcher: Box::new(|l, _| {
                l.contains("MACIBM_alert: The aircraft") && l.contains("has unusable")
            }),
        },
        Pattern {
            category: "vulkan_device_error",
            severity: "high",
            matcher: Box::new(|l, _| l.contains("MACIBM_alert: Vulkan")),
        },
        Pattern {
            category: "system_alert",
            severity: "high",
            matcher: Box::new(|l, _| l.contains("E/SYS: MACIBM_alert")),
        },
        Pattern {
            category: "plugin_assert",
            severity: "high",
            matcher: Box::new(|l, _| l.contains("E/PLG: Plugin assert:")),
        },
        Pattern {
            category: "plugin_error",
            severity: "high",
            matcher: Box::new(|l, ll| {
                l.contains(" E/PLG:")
                    && (ll.contains("error") || ll.contains("fail") || ll.contains("crash"))
            }),
        },
        Pattern {
            category: "vulkan_gfx_error",
            severity: "high",
            matcher: Box::new(|l, _| l.contains(" E/GFX/VK:") && l.contains("VK_ERROR")),
        },
        Pattern {
            category: "gfx_error",
            severity: "high",
            matcher: Box::new(|l, ll| {
                l.contains(" E/GFX:")
                    && (ll.contains("error") || ll.contains("fail") || ll.contains("crash"))
            }),
        },
        Pattern {
            category: "aircraft_model_error",
            severity: "high",
            matcher: Box::new(|l, _| l.contains(" E/ACF:")),
        },
        Pattern {
            category: "heavy_memory_pressure",
            severity: "high",
            matcher: Box::new(|l, _| {
                l.contains("W/MEM: Entered heavy memory pressure state")
                    || l.contains("I/MEM: Entered heavy memory pressure state")
                    || (l.contains("E/MEM:") && l.contains("Physical memory usage"))
            }),
        },
        Pattern {
            category: "missing_plugin_support",
            severity: "high",
            matcher: Box::new(|_, ll| {
                ll.contains("you are missing a required plugin support file")
            }),
        },
        Pattern {
            category: "out_of_memory",
            severity: "high",
            matcher: Box::new(|l, ll| {
                // English patterns
                ll.contains("out of memory")
                    || ll.contains("memory allocation failed")
                    || ll.contains("cannot allocate")
                    // Chinese patterns
                    || l.contains("内存已满")
                    || l.contains("内存不足")
                    // System fatal assert about memory
                    || (l.contains("E/SYS: THREAD FATAL ASSERT") && l.contains("内存"))
            }),
        },
        Pattern {
            category: "memory_status_critical",
            severity: "high",
            matcher: Box::new(|l, _| {
                l.contains(" E/MEM:")
                    && (l.contains("Memory status information")
                        || l.contains("Physical memory usage")
                        || l.contains("Virtual memory usage"))
            }),
        },
        Pattern {
            category: "memory_access_error",
            severity: "high",
            matcher: Box::new(|_, ll| {
                ll.contains("access violation")
                    || ll.contains("segmentation fault")
                    || ll.contains("sigsegv")
            }),
        },
        // ===== MEDIUM =====
        Pattern {
            category: "dsf_error",
            severity: "medium",
            matcher: Box::new(|l, _| l.contains(" E/DSF:")),
        },
        Pattern {
            category: "scenery_error",
            severity: "medium",
            matcher: Box::new(|l, _| l.contains(" E/SCN:")),
        },
        Pattern {
            category: "network_error",
            severity: "medium",
            matcher: Box::new(|l, _| l.contains(" E/NET:")),
        },
        Pattern {
            category: "weather_error",
            severity: "medium",
            matcher: Box::new(|l, _| l.contains(" E/WXR:")),
        },
        Pattern {
            category: "audio_error",
            severity: "medium",
            matcher: Box::new(|l, ll| {
                l.contains(" E/FMOD:") || (l.contains("FMOD") && ll.contains("error"))
            }),
        },
        Pattern {
            category: "negative_memory_pressure",
            severity: "medium",
            matcher: Box::new(|l, _| l.contains("I/MEM: Entered negative memory pressure state")),
        },
        Pattern {
            category: "severe_texture_downscale",
            severity: "medium",
            matcher: Box::new(|l, _| l.contains("I/TEX: Target scale moved to 0.0625")),
        },
        Pattern {
            category: "runloop_backlog",
            severity: "medium",
            matcher: Box::new(|l, _| l.contains("W/RLP: Runloop is backed up")),
        },
        Pattern {
            category: "nvidia_permission",
            severity: "medium",
            matcher: Box::new(|l, _| {
                l.contains(" E/NVAPI:") || l.contains("NVAPI_INVALID_USER_PRIVILEGE")
            }),
        },
        Pattern {
            category: "third_party_blocked",
            severity: "medium",
            matcher: Box::new(|l, _| {
                l.contains("I/GFX/VK: Disabled") && (l.contains("ReShade") || l.contains("GamePP"))
            }),
        },
        Pattern {
            category: "duplicate_plugin",
            severity: "medium",
            matcher: Box::new(|_, ll| ll.contains("a version of this plugin is already loaded")),
        },
        Pattern {
            category: "ssl_failed",
            severity: "medium",
            matcher: Box::new(|l, ll| {
                l.contains("SSL/TLS connection failed")
                    || (ll.contains("schannel") && ll.contains("failed"))
            }),
        },
        // ===== LOW =====
        Pattern {
            category: "regular_memory_pressure",
            severity: "low",
            matcher: Box::new(|l, _| l.contains("I/MEM: Entered regular memory pressure state")),
        },
        Pattern {
            category: "deprecated_dataref",
            severity: "low",
            matcher: Box::new(|l, _| l.contains("Dataref '") && l.contains("has been replaced")),
        },
        Pattern {
            category: "better_pushback_warning",
            severity: "low",
            matcher: Box::new(|l, _| l.contains("BetterPushback") && l.contains("WARN:")),
        },
        Pattern {
            category: "art_controls_modified",
            severity: "low",
            matcher: Box::new(|l, _| l.contains("(Art controls are modified.)")),
        },
    ]
}

/// Collect all consecutive `E/` or `W/`-prefixed lines that form a single block around `idx`.
/// Lines may have timestamp prefixes like "0:00:06.262 E/SYS: ..."
/// If the matched line itself doesn't contain `E/` or `W/`, return just that line.
/// Caps the block at 30 lines to avoid unreasonably large output.
fn extract_consecutive_e_lines(lines: &[String], idx: usize) -> String {
    let is_error_or_warning = |s: &str| s.contains(" E/") || s.contains(" W/");

    if !is_error_or_warning(&lines[idx]) {
        return lines[idx].clone();
    }

    // Extend upward
    let mut start = idx;
    while start > 0 && is_error_or_warning(&lines[start - 1]) {
        start -= 1;
    }
    // Extend downward
    let mut end = idx;
    while end + 1 < lines.len() && is_error_or_warning(&lines[end + 1]) {
        end += 1;
    }
    // Cap to 30 lines
    let end = end.min(start + 29);
    lines[start..=end].join("\n")
}

/// Like extract_consecutive_e_lines but anchored on the crash marker line.
/// Captures consecutive E/ lines immediately above and below the crash line,
/// so the user sees the full error context surrounding the crash.
fn extract_crash_context(lines: &[String], crash_idx: usize) -> String {
    let is_error_line = |s: &str| s.contains(" E/");

    // Extend upward to capture consecutive E/ lines before the crash marker
    let mut start = crash_idx;
    while start > 0 && is_error_line(&lines[start - 1]) {
        start -= 1;
    }
    // Extend downward to capture consecutive E/ lines after the crash marker
    let mut end = crash_idx;
    while end + 1 < lines.len() && is_error_line(&lines[end + 1]) {
        end += 1;
    }
    // Cap to 30 lines
    let end = end.min(start + 29);
    lines[start..=end].join("\n")
}

fn extract_system_info(lines: &[String]) -> SystemInfo {
    let mut info = SystemInfo::default();

    for line in lines.iter().take(150) {
        // X-Plane version: "Log.txt for X-Plane 12.4.0-..."
        if info.xplane_version.is_none() {
            if let Some(pos) = line.find("Log.txt for X-Plane ") {
                let rest = &line[pos + "Log.txt for X-Plane ".len()..];
                let v: String = rest.split_whitespace().next().unwrap_or("").to_string();
                if !v.is_empty() {
                    info.xplane_version = Some(v);
                }
            } else if let Some(pos) = line.find("Log.txt for ") {
                let rest = &line[pos + "Log.txt for ".len()..];
                let v: String = rest.split_whitespace().next().unwrap_or("").to_string();
                if !v.is_empty() {
                    info.xplane_version = Some(v);
                }
            }
        }

        // GPU model — Vulkan first, then OpenGL
        if info.gpu_model.is_none() {
            for prefix in &["Vulkan Device", "OpenGL Render"] {
                if let Some(pos) = line.find(prefix) {
                    if let Some(colon) = line[pos..].find(':') {
                        let rest = line[pos + colon + 1..].trim();
                        let model = if let Some(p) = rest.rfind(" (") {
                            rest[..p].trim()
                        } else {
                            rest
                        };
                        if !model.is_empty() {
                            info.gpu_model = Some(model.to_string());
                            break;
                        }
                    }
                }
            }
        }

        // GPU driver — Vulkan first, then OpenGL
        if info.gpu_driver.is_none() {
            if let Some(pos) = line.find("Vulkan Driver") {
                if let Some(colon) = line[pos..].find(':') {
                    let driver = line[pos + colon + 1..].trim().to_string();
                    if !driver.is_empty() {
                        info.gpu_driver = Some(driver);
                    }
                }
            } else if let Some(pos) = line.find("OpenGL Version") {
                if let Some(colon) = line[pos..].find(':') {
                    // Format: "4.6.0 NVIDIA 515.76 (2022/07/19)"
                    let rest = line[pos + colon + 1..].trim();
                    let words: Vec<&str> = rest.split_whitespace().collect();
                    if words.len() > 1 {
                        let driver_raw = words[1..].join(" ");
                        let driver = if let Some(p) = driver_raw.rfind(" (") {
                            driver_raw[..p].to_string()
                        } else {
                            driver_raw
                        };
                        if !driver.is_empty() {
                            info.gpu_driver = Some(driver);
                        }
                    }
                }
            }
        }
    }

    info
}

#[tauri::command]
fn analyze_xplane_log(xplane_path: String) -> Result<XPlaneLogAnalysis, String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let log_path = std::path::PathBuf::from(&xplane_path).join("Log.txt");
    let log_path_str = log_path.to_string_lossy().to_string();

    let file = File::open(&log_path).map_err(|e| format!("Cannot open Log.txt: {e}"))?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .map(|l| l.map_err(|e| format!("Read error: {e}")))
        .collect::<Result<Vec<_>, _>>()?;

    // Check if it's an X-Plane log (first 100 lines)
    let xplane_indicators = [
        "X-Plane",
        "X-System folder",
        "Vulkan Device",
        "OpenGL Render",
        "Fetching plugins for",
        "Laminar Research",
    ];
    let is_xplane_log = lines
        .iter()
        .take(100)
        .any(|line| xplane_indicators.iter().any(|ind| line.contains(ind)));

    let system_info = extract_system_info(&lines);

    let patterns = build_patterns();

    // category -> (severity, line_numbers, sample_line)
    let mut issue_map: std::collections::HashMap<&'static str, (&'static str, Vec<usize>, String)> =
        std::collections::HashMap::new();

    let mut crash_detected = false;
    let mut crash_info: Option<String> = None;

    for (idx, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();

        if line.contains("This application has crashed") {
            crash_detected = true;
            if crash_info.is_none() {
                crash_info = Some(extract_crash_context(&lines, idx));
            }
        }

        for pat in &patterns {
            if (pat.matcher)(line, &line_lower) {
                let entry = issue_map.entry(pat.category).or_insert((
                    pat.severity,
                    Vec::new(),
                    String::new(),
                ));
                if entry.1.len() < 5 {
                    entry.1.push(idx + 1);
                    // Keep the largest consecutive E/ block found so far
                    let block = extract_consecutive_e_lines(&lines, idx);
                    if block.len() > entry.2.len() {
                        entry.2 = block;
                    }
                }
            }
        }
    }

    let sev_order = |s: &str| match s {
        "high" => 0u8,
        "medium" => 1,
        _ => 2,
    };
    let mut issues: Vec<LogIssue> = issue_map
        .into_iter()
        .map(|(cat, (sev, nums, sample))| LogIssue {
            category: cat.to_string(),
            severity: sev.to_string(),
            line_numbers: nums,
            sample_line: sample,
        })
        .collect();
    issues.sort_by_key(|i| sev_order(&i.severity));

    let total_high = issues.iter().filter(|i| i.severity == "high").count();
    let total_medium = issues.iter().filter(|i| i.severity == "medium").count();
    let total_low = issues.iter().filter(|i| i.severity == "low").count();

    Ok(XPlaneLogAnalysis {
        log_path: log_path_str,
        is_xplane_log,
        crash_detected,
        crash_info,
        issues,
        system_info,
        total_high,
        total_medium,
        total_low,
    })
}

#[tauri::command]
async fn analyze_crash_report(
    xplane_path: String,
    log_issues: Vec<LogIssue>,
    skip_date_check: bool,
) -> Result<Option<crash_analysis::DeepCrashAnalysis>, String> {
    crash_analysis::analyze_crash_report(&xplane_path, &log_issues, skip_date_check).await
}

// ========== Scenery Folder Commands ==========

fn validate_scenery_folder_name(folder_name: &str) -> error::ApiResult<()> {
    if folder_name.is_empty()
        || folder_name.contains("..")
        || folder_name.contains('/')
        || folder_name.contains('\\')
    {
        return Err(error::ApiError::security_violation(
            "Invalid folder name: path traversal not allowed",
        ));
    }
    Ok(())
}

fn resolve_scenery_entry_path(
    xplane_path: &str,
    folder_name: &str,
) -> error::ApiResult<(PathBuf, PathBuf)> {
    validate_scenery_folder_name(folder_name)?;

    let base_path = PathBuf::from(xplane_path).join("Custom Scenery");
    let candidate = base_path.join(folder_name);
    if candidate.exists() {
        return Ok((candidate, base_path));
    }

    #[cfg(target_os = "windows")]
    {
        let lnk_path = base_path.join(format!("{}.lnk", folder_name));
        if lnk_path.exists() {
            return Ok((lnk_path, base_path));
        }
    }

    Err(error::ApiError::with_details(
        error::ApiErrorCode::NotFound,
        "Scenery folder not found",
        folder_name,
    ))
}

#[tauri::command]
fn open_scenery_folder(xplane_path: String, folder_name: String) -> error::ApiResult<()> {
    let (entry_path, base_path) = resolve_scenery_entry_path(&xplane_path, &folder_name)?;
    let metadata = fs::symlink_metadata(&entry_path)
        .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;

    // If it's a symlink, open the link itself to allow external targets
    if metadata.file_type().is_symlink() {
        return open_in_explorer(&entry_path).map_err(error::ApiError::internal);
    }

    // For regular directories/files, enforce canonical base containment
    let canonical_path = path_utils::validate_child_path(&base_path, &entry_path)
        .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;

    open_in_explorer(&canonical_path).map_err(error::ApiError::internal)
}

#[tauri::command]
async fn delete_scenery_folder(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    folder_name: String,
) -> error::ApiResult<()> {
    let (entry_path, base_path) = resolve_scenery_entry_path(&xplane_path, &folder_name)?;
    let metadata = fs::symlink_metadata(&entry_path)
        .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;

    if metadata.file_type().is_symlink() {
        // Remove the symlink itself without following it
        if let Err(e) = fs::remove_file(&entry_path) {
            // Some platforms treat directory symlinks differently
            if let Err(e2) = fs::remove_dir(&entry_path) {
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    || e2.kind() == std::io::ErrorKind::PermissionDenied
                {
                    return Err(error::ApiError::permission_denied(format!(
                        "Permission denied when deleting: {}",
                        folder_name
                    )));
                }
                return Err(error::ApiError::internal(format!(
                    "Failed to delete scenery link: {} ({}; {})",
                    folder_name, e, e2
                )));
            }
        }
    } else if metadata.is_file() {
        // Handle Windows .lnk shortcuts or other file entries
        fs::remove_file(&entry_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                error::ApiError::permission_denied(format!(
                    "Permission denied when deleting: {}",
                    folder_name
                ))
            } else {
                error::ApiError::internal(format!("Failed to delete scenery file: {}", e))
            }
        })?;
    } else {
        // Security: Use validate_child_path for strict path validation to prevent path traversal attacks
        let canonical_path = path_utils::validate_child_path(&base_path, &entry_path)
            .map_err(|e| error::ApiError::validation(format!("Invalid path: {}", e)))?;

        // Delete the folder using the canonical path for safety
        fs::remove_dir_all(&canonical_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                error::ApiError::permission_denied(format!(
                    "Permission denied when deleting: {}",
                    folder_name
                ))
            } else {
                error::ApiError::internal(format!("Failed to delete scenery folder: {}", e))
            }
        })?;
    }

    // Remove from scenery index if it exists
    if let Err(e) = scenery_index::remove_scenery_entry(&db.get(), &xplane_path, &folder_name).await
    {
        logger::log_error(
            &format!("Failed to remove scenery from index: {}", e),
            Some("scenery"),
        );
    }

    // Update scenery_packs.ini to remove the deleted entry
    let xplane_path = std::path::Path::new(&xplane_path);
    let packs_manager = scenery_packs_manager::SceneryPacksManager::new(xplane_path, db.get());
    if let Err(e) = packs_manager.apply_from_index().await {
        logger::log_error(
            &format!("Failed to update scenery_packs.ini after deletion: {}", e),
            Some("scenery"),
        );
    }

    logger::log_info(
        &format!("Deleted scenery folder: {}", folder_name),
        Some("scenery"),
    );

    Ok(())
}

#[tauri::command]
fn set_log_locale(locale: String) {
    logger::set_locale(&locale);
}

#[tauri::command]
fn set_log_level(level: String) {
    let log_level = match level.to_lowercase().as_str() {
        "debug" => logger::LogLevel::Debug,
        "info" => logger::LogLevel::Info,
        "error" => logger::LogLevel::Error,
        _ => logger::LogLevel::Info, // Default to Info
    };
    logger::set_log_level(log_level);
}

// ========== Path Validation Commands ==========

#[tauri::command]
fn check_path_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[tauri::command]
fn launch_xplane(xplane_path: String, args: Option<Vec<String>>) -> Result<(), String> {
    let path = std::path::Path::new(&xplane_path);
    let extra_args = args.unwrap_or_default();

    #[cfg(target_os = "windows")]
    {
        let exe_path = path.join("X-Plane.exe");

        // Try to launch without elevation first
        let result = std::process::Command::new(&exe_path)
            .args(&extra_args)
            .spawn();

        // If it fails with error 740 (requires elevation), inform user
        match result {
            Ok(_) => {
                logger::log_info("X-Plane launched successfully", Some("app"));
            }
            Err(e) if e.raw_os_error() == Some(740) => {
                // Return a structured error that frontend can handle
                return Err(format!("ELEVATION_REQUIRED:{}", exe_path.display()));
            }
            Err(e) => {
                return Err(format!("Failed to launch X-Plane: {}", e));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS uses 'open' command for .app bundles
        let app_path = path.join("X-Plane.app");
        let mut cmd = std::process::Command::new("open");
        cmd.arg(&app_path);
        if !extra_args.is_empty() {
            cmd.arg("--args");
            cmd.args(&extra_args);
        }
        cmd.spawn()
            .map_err(|e| format!("Failed to launch X-Plane: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Find the first file matching X-Plane-*
        let exe_path = std::fs::read_dir(path)
            .map_err(|e| format!("Failed to read X-Plane directory: {}", e))?
            .flatten()
            .find(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("X-Plane"))
            })
            .map(|entry| entry.path())
            .ok_or_else(|| "X-Plane executable not found (expected X-Plane*)".to_string())?;
        std::process::Command::new(exe_path)
            .args(&extra_args)
            .spawn()
            .map_err(|e| format!("Failed to launch X-Plane: {}", e))?;
    }

    logger::log_info("X-Plane launched", Some("app"));
    Ok(())
}

/// Run a command in a blocking task and check if the process was found
async fn check_process_running(
    command: &'static str,
    args: &'static [&'static str],
    check_output: fn(&std::process::Output) -> bool,
) -> bool {
    let result = tokio::task::spawn_blocking(move || {
        let mut cmd = std::process::Command::new(command);
        cmd.args(args);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd.output()
    })
    .await;

    match result {
        Ok(Ok(output)) => {
            let is_running = check_output(&output);
            logger::log_debug(
                &format!("X-Plane running check: {}", is_running),
                Some("app"),
                None,
            );
            is_running
        }
        Ok(Err(e)) => {
            logger::log_debug(
                &format!("Failed to run {}: {}", command, e),
                Some("app"),
                None,
            );
            false
        }
        Err(e) => {
            logger::log_debug(&format!("Task join error: {}", e), Some("app"), None);
            false
        }
    }
}

#[tauri::command]
async fn is_xplane_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        check_process_running(
            "tasklist",
            &["/FI", "IMAGENAME eq X-Plane.exe", "/NH"],
            |output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains("X-Plane.exe")
            },
        )
        .await
    }

    #[cfg(target_os = "macos")]
    {
        check_process_running("pgrep", &["-x", "X-Plane"], |output| {
            output.status.success()
        })
        .await
    }

    #[cfg(target_os = "linux")]
    {
        // Linux executable is X-Plane-*, use pattern match instead of exact match
        check_process_running("pgrep", &["-f", "X-Plane"], |output| {
            output.status.success()
        })
        .await
    }
}

#[tauri::command]
fn validate_xplane_path(path: String) -> Result<bool, String> {
    let path_obj = std::path::Path::new(&path);

    // Check if path exists
    if !path_obj.exists() {
        return Ok(false);
    }

    // Check if it's a directory
    if !path_obj.is_dir() {
        return Ok(false);
    }

    // Check for X-Plane executable
    if cfg!(target_os = "linux") {
        // Linux: match any file named X-Plane-*
        match std::fs::read_dir(path_obj) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with("X-Plane") {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            }
            Err(_) => Ok(false),
        }
    } else {
        let exe_name = if cfg!(target_os = "windows") {
            "X-Plane.exe"
        } else {
            "X-Plane.app"
        };
        let exe_path = path_obj.join(exe_name);
        Ok(exe_path.exists())
    }
}

// ========== Update Commands ==========

#[tauri::command]
async fn check_for_updates(
    manual: bool,
    include_pre_release: bool,
) -> Result<updater::UpdateInfo, String> {
    let checker = updater::UpdateChecker::new();
    checker.check_for_updates(manual, include_pre_release).await
}

#[tauri::command]
fn get_last_check_time() -> Option<i64> {
    updater::get_last_check_time()
}

// ========== Library Download Links Commands ==========

#[tauri::command]
async fn lookup_library_links(
    library_names: Vec<String>,
) -> Result<std::collections::HashMap<String, Option<String>>, String> {
    Ok(library_links::lookup_library_links_local(library_names).await)
}

#[tauri::command]
async fn lookup_library_links_remote(
    library_names: Vec<String>,
    force_refresh: Option<bool>,
) -> Result<std::collections::HashMap<String, Option<String>>, String> {
    library_links::lookup_library_links_remote(library_names, force_refresh.unwrap_or(false)).await
}

// ========== Scenery Auto-Sorting Commands ==========

#[tauri::command]
async fn get_scenery_classification(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    folder_name: String,
) -> error::ApiResult<SceneryPackageInfo> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let scenery_path = xplane_path.join("Custom Scenery").join(&folder_name);

    if !scenery_path.exists() {
        return Err(error::ApiError::not_found(format!(
            "Scenery folder not found: {}",
            folder_name
        )));
    }

    let index_manager = SceneryIndexManager::new(xplane_path, db);
    index_manager
        .get_or_classify(&scenery_path)
        .await
        .map_err(|e| error::ApiError::internal(format!("Classification failed: {}", e)))
}

#[tauri::command]
async fn sort_scenery_packs(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> Result<bool, String> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let index_manager = SceneryIndexManager::new(xplane_path, db);

    logger::log_info("Resetting scenery index sort order", Some("scenery"));

    let has_changes = index_manager
        .reset_sort_order()
        .await
        .map_err(|e| format!("Failed to reset sort order: {}", e))?;

    logger::log_info(
        "Scenery index sort order reset successfully",
        Some("scenery"),
    );
    Ok(has_changes)
}

#[tauri::command]
async fn rebuild_scenery_index(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> Result<SceneryIndexStats, String> {
    // Rebuild replaces all data, so an incompatible schema can be silently fixed first.
    // Use db.reset() (not reset_schema) so the pool is replaced with a fresh one,
    // clearing any stale sqlx prepared-statement caches.
    if !database::is_schema_compatible(&db.get())
        .await
        .map_err(|e| e.to_string())?
    {
        logger::log_info(
            "Incompatible schema detected before rebuild — resetting",
            Some("database"),
        );
        db.reset().await.map_err(|e| e.to_string())?;
    }

    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let index_manager = SceneryIndexManager::new(xplane_path, db);

    logger::log_info("Rebuilding scenery index", Some("scenery"));

    let index = index_manager
        .rebuild_index()
        .await
        .map_err(|e| format!("Failed to rebuild index: {}", e))?;

    // Compute stats from the in-memory index directly to avoid a second DB read.
    // A SELECT on all columns can fail due to sqlx prepared-statement cache staleness
    // after a DROP + CREATE TABLE schema reset on the same connection pool.
    let mut by_category: HashMap<String, usize> = HashMap::new();
    for info in index.packages.values() {
        let category_name = format!("{:?}", info.category);
        *by_category.entry(category_name).or_insert(0) += 1;
    }
    Ok(SceneryIndexStats {
        total_packages: index.packages.len(),
        by_category,
        last_updated: index.last_updated,
    })
}

/// Reset the scenery database by deleting it entirely
/// This is useful when the database schema version is incompatible
#[tauri::command]
async fn reset_scenery_database() -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        logger::log_info("Resetting scenery database", Some("database"));
        database::delete_database().map_err(|e| format!("Failed to delete database: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Check whether the current database schema is compatible with the entity model.
/// Returns `false` if columns added after the initial release are missing (old database).
#[tauri::command]
async fn check_database_compatibility(db: tauri::State<'_, DatabaseState>) -> Result<bool, String> {
    database::is_schema_compatible(&db.get())
        .await
        .map_err(|e| e.to_string())
}

/// Delete the scenery database and exit the process so the user can relaunch with a clean slate.
#[tauri::command]
async fn reset_and_reinitialize(db: tauri::State<'_, DatabaseState>) -> Result<(), String> {
    db.reset().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_scenery_index_stats(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> Result<SceneryIndexStats, String> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let index_manager = SceneryIndexManager::new(xplane_path, db);

    index_manager
        .get_stats()
        .await
        .map_err(|e| format!("Failed to get stats: {}", e))
}

#[tauri::command]
async fn get_scenery_index_status(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> Result<SceneryIndexStatus, String> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let index_manager = SceneryIndexManager::new(xplane_path, db);

    index_manager
        .index_status()
        .await
        .map_err(|e| format!("Failed to get index status: {}", e))
}

#[tauri::command]
async fn quick_scan_scenery_index(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> Result<SceneryIndexScanResult, String> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let index_manager = SceneryIndexManager::new(xplane_path, db);

    index_manager
        .quick_scan_and_update()
        .await
        .map_err(|e| format!("Failed to quick scan scenery index: {}", e))
}

#[tauri::command]
async fn sync_scenery_packs_with_folder(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> Result<usize, String> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let manager = SceneryPacksManager::new(xplane_path, db);

    manager
        .sync_with_folder()
        .await
        .map_err(|e| format!("Failed to sync scenery packs: {}", e))
}

#[tauri::command]
async fn get_scenery_manager_data(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> Result<SceneryManagerData, String> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let index_manager = SceneryIndexManager::new(xplane_path, db);

    index_manager
        .get_manager_data()
        .await
        .map_err(|e| format!("Failed to get scenery manager data: {}", e))
}

#[tauri::command]
async fn update_scenery_entry(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    folder_name: String,
    enabled: Option<bool>,
    sort_order: Option<u32>,
    category: Option<models::SceneryCategory>,
) -> Result<(), String> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let index_manager = SceneryIndexManager::new(xplane_path, db);

    index_manager
        .update_entry(&folder_name, enabled, sort_order, category)
        .await
        .map_err(|e| format!("Failed to update scenery entry: {}", e))
}

#[tauri::command]
async fn move_scenery_entry(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    folder_name: String,
    new_sort_order: u32,
) -> Result<(), String> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let index_manager = SceneryIndexManager::new(xplane_path, db);

    index_manager
        .move_entry(&folder_name, new_sort_order)
        .await
        .map_err(|e| format!("Failed to move scenery entry: {}", e))
}

#[tauri::command]
async fn apply_scenery_changes(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    entries: Vec<models::SceneryEntryUpdate>,
) -> Result<(), String> {
    let db = db.get();
    let xplane_path = std::path::Path::new(&xplane_path);
    let index_manager = SceneryIndexManager::new(xplane_path, db.clone());

    logger::log_info("Applying scenery changes to index and ini", Some("scenery"));

    // Update index with all entry changes
    index_manager
        .batch_update_entries(&entries)
        .await
        .map_err(|e| format!("Failed to update index: {}", e))?;

    // Apply to ini file
    let packs_manager = SceneryPacksManager::new(xplane_path, db);
    packs_manager
        .apply_from_index()
        .await
        .map_err(|e| format!("Failed to apply scenery changes: {}", e))?;

    logger::log_info("Scenery changes applied successfully", Some("scenery"));
    Ok(())
}

// ========== Management Commands ==========

fn emit_addon_update_status(
    app_handle: &tauri::AppHandle,
    item_type: &str,
    folder_name: &str,
    stage: &str,
    status: &str,
    message: Option<String>,
) {
    let _ = app_handle.emit(
        "addon-update-progress",
        addon_updater::AddonUpdateProgressEvent {
            item_type: item_type.to_string(),
            folder_name: folder_name.to_string(),
            stage: stage.to_string(),
            status: status.to_string(),
            percentage: if status.eq_ignore_ascii_case("completed") {
                100.0
            } else {
                0.0
            },
            processed_units: 0,
            total_units: 0,
            processed_bytes: 0,
            total_bytes: 0,
            speed_bytes_per_sec: 0.0,
            current_file: None,
            message,
        },
    );
}

fn resolve_addon_target_path(
    xplane_path: &str,
    item_type: &str,
    folder_name: &str,
) -> Option<PathBuf> {
    if folder_name.trim().is_empty() || folder_name.contains("..") {
        return None;
    }

    let base = PathBuf::from(xplane_path);
    let normalized = folder_name.replace('\\', "/");
    let relative = PathBuf::from(normalized);
    let target = match item_type {
        "aircraft" | "livery" => base.join("Aircraft").join(relative),
        "plugin" => base.join("Resources").join("plugins").join(relative),
        "scenery" => base.join("Custom Scenery").join(relative),
        _ => return None,
    };

    if !target.exists() {
        return None;
    }

    Some(target)
}

fn is_xupdater_disabled_target(xplane_path: &str, item_type: &str, folder_name: &str) -> bool {
    let Some(target_path) = resolve_addon_target_path(xplane_path, item_type, folder_name) else {
        return false;
    };

    if target_path.join("skunkcrafts_updater.cfg").exists() {
        return false;
    }

    x_updater_profile::find_profile_in_folder(&target_path).is_some()
}

fn blocked_addon_update_error(item_type: &str, folder_name: &str) -> String {
    let mut checksum: u32 = 0x6D2B_79F5;
    for byte in item_type.bytes().chain(folder_name.bytes()) {
        checksum = checksum.rotate_left(5) ^ ((byte as u32).wrapping_mul(0x45D9_F3B));
        checksum = checksum.wrapping_add(0x27D4_EB2D);
    }
    let token = format!("{:08X}", checksum ^ 0xA1C3_0F71);
    format!("resolver handshake mismatch [{}]", token)
}

#[tauri::command]
async fn scan_aircraft(xplane_path: String) -> Result<ManagementData<AircraftInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_aircraft(xplane_path)
            .map_err(|e| format!("Failed to scan aircraft: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn check_aircraft_updates(
    _xplane_path: String,
    mut aircraft: Vec<AircraftInfo>,
) -> Result<Vec<AircraftInfo>, String> {
    management_index::check_aircraft_updates(&mut aircraft).await;
    Ok(aircraft)
}

#[tauri::command]
async fn check_plugins_updates(mut plugins: Vec<PluginInfo>) -> Result<Vec<PluginInfo>, String> {
    management_index::check_plugins_updates(&mut plugins).await;
    Ok(plugins)
}

#[tauri::command]
async fn build_addon_update_plan(
    app_handle: tauri::AppHandle,
    task_control: State<'_, TaskControl>,
    xplane_path: String,
    item_type: String,
    folder_name: String,
    options: addon_updater::AddonUpdateOptions,
) -> Result<addon_updater::AddonUpdatePlan, String> {
    task_control.reset();
    if is_xupdater_disabled_target(&xplane_path, &item_type, &folder_name) {
        let message = blocked_addon_update_error(&item_type, &folder_name);
        emit_addon_update_status(
            &app_handle,
            &item_type,
            &folder_name,
            "scan",
            "failed",
            Some(message.clone()),
        );
        return Err(message);
    }
    let event_handle = app_handle.clone();
    let progress_callback: addon_updater::AddonUpdateProgressCallback =
        Arc::new(move |event| {
            let _ = event_handle.emit("addon-update-progress", event);
        });
    let xplane_path = std::path::Path::new(&xplane_path);
    match addon_updater::build_update_plan(
        xplane_path,
        &item_type,
        &folder_name,
        options,
        Some(task_control.inner().clone()),
        Some(progress_callback),
    )
    .await
    {
        Ok(plan) => Ok(plan),
        Err(e) => {
            emit_addon_update_status(
                &app_handle,
                &item_type,
                &folder_name,
                "scan",
                if e.to_string().to_lowercase().contains("cancelled") {
                    "cancelled"
                } else {
                    "failed"
                },
                Some(e.to_string()),
            );
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn fetch_addon_update_preview(
    app_handle: tauri::AppHandle,
    task_control: State<'_, TaskControl>,
    xplane_path: String,
    item_type: String,
    folder_name: String,
    options: addon_updater::AddonUpdateOptions,
    login: Option<String>,
    license_key: Option<String>,
) -> Result<addon_updater::AddonUpdatePreview, String> {
    task_control.reset();
    if is_xupdater_disabled_target(&xplane_path, &item_type, &folder_name) {
        let message = blocked_addon_update_error(&item_type, &folder_name);
        emit_addon_update_status(
            &app_handle,
            &item_type,
            &folder_name,
            "check",
            "failed",
            Some(message.clone()),
        );
        return Err(message);
    }
    let event_handle = app_handle.clone();
    let progress_callback: addon_updater::AddonUpdateProgressCallback =
        Arc::new(move |event| {
            let _ = event_handle.emit("addon-update-progress", event);
        });
    let xplane_path = std::path::Path::new(&xplane_path);
    match addon_updater::fetch_update_preview(
        xplane_path,
        &item_type,
        &folder_name,
        options,
        login,
        license_key,
        Some(task_control.inner().clone()),
        Some(progress_callback),
    )
    .await
    {
        Ok(preview) => Ok(preview),
        Err(e) => {
            emit_addon_update_status(
                &app_handle,
                &item_type,
                &folder_name,
                "check",
                if e.to_string().to_lowercase().contains("cancelled") {
                    "cancelled"
                } else {
                    "failed"
                },
                Some(e.to_string()),
            );
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn execute_addon_update(
    app_handle: tauri::AppHandle,
    task_control: State<'_, TaskControl>,
    xplane_path: String,
    item_type: String,
    folder_name: String,
    options: addon_updater::AddonUpdateOptions,
) -> Result<addon_updater::AddonUpdateResult, String> {
    task_control.reset();
    if is_xupdater_disabled_target(&xplane_path, &item_type, &folder_name) {
        let message = blocked_addon_update_error(&item_type, &folder_name);
        emit_addon_update_status(
            &app_handle,
            &item_type,
            &folder_name,
            "install",
            "failed",
            Some(message.clone()),
        );
        return Err(message);
    }
    let event_handle = app_handle.clone();
    let progress_callback: addon_updater::AddonUpdateProgressCallback =
        Arc::new(move |event| {
            let _ = event_handle.emit("addon-update-progress", event);
        });
    let xplane_path = std::path::Path::new(&xplane_path);
    match addon_updater::execute_update(
        xplane_path,
        &item_type,
        &folder_name,
        options,
        Some(task_control.inner().clone()),
        Some(progress_callback),
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(e) => {
            emit_addon_update_status(
                &app_handle,
                &item_type,
                &folder_name,
                "install",
                if e.to_string().to_lowercase().contains("cancelled") {
                    "cancelled"
                } else {
                    "failed"
                },
                Some(e.to_string()),
            );
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn set_addon_updater_credentials(
    xplane_path: String,
    item_type: String,
    folder_name: String,
    login: String,
    license_key: String,
) -> Result<(), String> {
    if is_xupdater_disabled_target(&xplane_path, &item_type, &folder_name) {
        return Err(blocked_addon_update_error(&item_type, &folder_name));
    }
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        addon_updater::set_updater_credentials(
            xplane_path,
            &item_type,
            &folder_name,
            &login,
            &license_key,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn get_addon_updater_credentials(
    xplane_path: String,
    item_type: String,
    folder_name: String,
) -> Result<Option<addon_updater::AddonUpdaterCredentials>, String> {
    if is_xupdater_disabled_target(&xplane_path, &item_type, &folder_name) {
        return Err(blocked_addon_update_error(&item_type, &folder_name));
    }
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        addon_updater::get_updater_credentials(xplane_path, &item_type, &folder_name)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn get_addon_update_disk_space(
    xplane_path: String,
    item_type: String,
    folder_name: String,
) -> Result<addon_updater::AddonDiskSpaceInfo, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        addon_updater::get_target_disk_space(xplane_path, &item_type, &folder_name)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn scan_plugins(xplane_path: String) -> Result<ManagementData<PluginInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_plugins(xplane_path)
            .map_err(|e| format!("Failed to scan plugins: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn scan_navdata(xplane_path: String) -> Result<ManagementData<NavdataManagerInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_navdata(xplane_path)
            .map_err(|e| format!("Failed to scan navdata: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn scan_navdata_backups(xplane_path: String) -> Result<Vec<NavdataBackupInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_navdata_backups(xplane_path)
            .map_err(|e| format!("Failed to scan navdata backups: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn restore_navdata_backup(
    xplane_path: String,
    backup_folder_name: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::restore_navdata_backup(xplane_path, &backup_folder_name)
            .map_err(|e| format!("Failed to restore navdata backup: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn toggle_management_item(
    xplane_path: String,
    item_type: String,
    folder_name: String,
) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::toggle_management_item(xplane_path, &item_type, &folder_name)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn delete_management_item(
    xplane_path: String,
    item_type: String,
    folder_name: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::delete_management_item(xplane_path, &item_type, &folder_name)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn open_management_folder(
    xplane_path: String,
    item_type: String,
    folder_name: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::open_management_folder(xplane_path, &item_type, &folder_name)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn get_aircraft_liveries(
    xplane_path: String,
    aircraft_folder: String,
) -> Result<Vec<LiveryInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::get_aircraft_liveries(xplane_path, &aircraft_folder)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn delete_aircraft_livery(
    xplane_path: String,
    aircraft_folder: String,
    livery_folder: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::delete_aircraft_livery(xplane_path, &aircraft_folder, &livery_folder)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn open_livery_folder(
    xplane_path: String,
    aircraft_folder: String,
    livery_folder: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::open_livery_folder(xplane_path, &aircraft_folder, &livery_folder)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn set_cfg_disabled(
    xplane_path: String,
    item_type: String,
    folder_name: String,
    disabled: bool,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::set_cfg_disabled(xplane_path, &item_type, &folder_name, disabled)
            .map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn get_lua_scripts(xplane_path: String) -> Result<Vec<LuaScriptInfo>, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::scan_lua_scripts(xplane_path).map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn toggle_lua_script(xplane_path: String, file_name: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::toggle_lua_script(xplane_path, &file_name).map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[tauri::command]
async fn delete_lua_script(xplane_path: String, file_name: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let xplane_path = std::path::Path::new(&xplane_path);
        management_index::delete_lua_script(xplane_path, &file_name).map_err(error::ApiError::from)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .to_tauri_error()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // When a second instance is launched, this callback is triggered
            // args[0] is the executable path, args[1..] are the actual arguments
            let file_args: Vec<String> = args.iter().skip(1).map(|s| s.to_string()).collect();

            if !file_args.is_empty() {
                logger::log_info(
                    &format!(
                        "{}: {:?}",
                        logger::tr(logger::LogMsg::LaunchedWithArgs),
                        file_args
                    ),
                    Some("app"),
                );
                // Emit event to frontend with the new file paths
                let _ = app.emit("cli-args", file_args);

                // Bring window to front
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window.set_focus() {
                        logger::log_debug(
                            &format!("Failed to focus window: {}", e),
                            Some("app"),
                            Some("lib.rs"),
                        );
                    }
                }
            }
        }))
        .invoke_handler(tauri::generate_handler![
            get_cli_args,
            get_platform,
            get_app_version,
            open_url,
            create_library_link_issue,
            create_bug_report_issue,
            create_feedback_issue,
            post_issue_comment,
            check_issue_updates,
            get_issue_detail,
            analyze_addons,
            install_addons,
            cancel_installation,
            skip_current_task,
            register_context_menu,
            unregister_context_menu,
            is_context_menu_registered,
            sync_context_menu_paths,
            log_from_frontend,
            get_recent_logs,
            get_log_path,
            get_all_logs,
            analyze_xplane_log,
            analyze_crash_report,
            open_log_folder,
            open_scenery_folder,
            delete_scenery_folder,
            set_log_locale,
            set_log_level,
            check_path_exists,
            launch_xplane,
            is_xplane_running,
            validate_xplane_path,
            check_for_updates,
            get_last_check_time,
            // Library download links
            lookup_library_links,
            lookup_library_links_remote,
            // Scenery auto-sorting commands
            get_scenery_classification,
            sort_scenery_packs,
            rebuild_scenery_index,
            reset_scenery_database,
            check_database_compatibility,
            reset_and_reinitialize,
            get_scenery_index_stats,
            get_scenery_index_status,
            quick_scan_scenery_index,
            sync_scenery_packs_with_folder,
            // Scenery manager commands
            get_scenery_manager_data,
            update_scenery_entry,
            move_scenery_entry,
            apply_scenery_changes,
            // Management commands
            scan_aircraft,
            check_aircraft_updates,
            scan_plugins,
            check_plugins_updates,
            build_addon_update_plan,
            fetch_addon_update_preview,
            execute_addon_update,
            set_addon_updater_credentials,
            get_addon_updater_credentials,
            get_addon_update_disk_space,
            scan_navdata,
            scan_navdata_backups,
            restore_navdata_backup,
            toggle_management_item,
            delete_management_item,
            open_management_folder,
            get_aircraft_liveries,
            delete_aircraft_livery,
            open_livery_folder,
            set_cfg_disabled,
            get_lua_scripts,
            toggle_lua_script,
            delete_lua_script
        ])
        .setup(|app| {
            // Initialize TaskControl state
            app.manage(TaskControl::new());

            // Initialize database connection and run migrations once on startup
            let db = tauri::async_runtime::block_on(async {
                let conn = database::open_connection_async().await?;
                database::apply_migrations_async(&conn).await?;
                Ok::<DatabaseConnection, error::ApiError>(conn)
            })
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

            app.manage(DatabaseState::new(db));

            // Log application startup
            logger::log_info(&logger::tr(logger::LogMsg::AppStarted), Some("app"));

            // Handle CLI arguments if present (for first launch)
            let args: Vec<String> = std::env::args().skip(1).collect();
            if !args.is_empty() {
                logger::log_info(
                    &format!(
                        "{}: {:?}",
                        logger::tr(logger::LogMsg::LaunchedWithArgs),
                        args
                    ),
                    Some("app"),
                );
                // Emit event to frontend
                app.emit("cli-args", args.clone()).ok();
            }

            // Fetch latest livery patterns on startup (non-blocking)
            tauri::async_runtime::spawn(async {
                livery_patterns::ensure_patterns_loaded().await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

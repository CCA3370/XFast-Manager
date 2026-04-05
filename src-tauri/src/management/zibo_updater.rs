use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use glob::Pattern;
use regex::Regex;
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::fs::OpenOptions as TokioOpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex as TokioMutex;
use tokio::task::JoinSet;
use uuid::Uuid;
use zip::ZipArchive;

use crate::addon_updater::{AddonUpdateProgressCallback, AddonUpdateProgressEvent};
use crate::logger;
use crate::skunk_updater::{
    SkunkUpdateOptions as AddonUpdateOptions, SkunkUpdatePlan as AddonUpdatePlan,
    SkunkUpdateResult as AddonUpdateResult,
};
use crate::task_control::TaskControl;

pub const ZIBO_PROVIDER: &str = "zibo";
pub const ZIBO_RSS_URL: &str = "https://skymatixva.com/tfiles/feed.xml";
const ZIBO_VERSION_FILE: &str = "version.txt";
const LOG_CTX: &str = "zibo_updater";
const GOOGLE_DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";
const GOOGLE_DRIVE_API_KEY_ENV: &str = "XFAST_ZIBO_GOOGLE_DRIVE_API_KEY";
const ZIBO_MANUAL_FALLBACK_URL: &str =
    "https://forums.x-plane.org/forums/topic/185685-alternative-download-links-zibo-install-guide-training-checklist/";
const ZIBO_PATCH_DOWNLOAD_PROGRESS_MAX: f64 = 90.0;
const ZIBO_PATCH_EXTRACT_PROGRESS_MAX: f64 = 96.0;
const ZIBO_PATCH_COPY_PROGRESS_MAX: f64 = 99.0;
const ZIBO_MAJOR_BACKUP_PROGRESS_MAX: f64 = 8.0;
const ZIBO_MAJOR_RENAME_PROGRESS_MAX: f64 = 10.0;
const ZIBO_MAJOR_FULL_DOWNLOAD_PROGRESS_MAX: f64 = 60.0;
const ZIBO_MAJOR_FULL_EXTRACT_PROGRESS_MAX: f64 = 70.0;
const ZIBO_MAJOR_FULL_COPY_PROGRESS_MAX: f64 = 82.0;
const ZIBO_MAJOR_PATCH_DOWNLOAD_PROGRESS_MAX: f64 = 93.0;
const ZIBO_MAJOR_PATCH_EXTRACT_PROGRESS_MAX: f64 = 96.0;
const ZIBO_MAJOR_PATCH_COPY_PROGRESS_MAX: f64 = 98.0;
const ZIBO_MAJOR_RESTORE_PROGRESS_MAX: f64 = 99.5;
const ZIBO_PROGRESS_EMIT_INTERVAL_MS: u64 = 250;
const ZIBO_SPEED_SAMPLE_INTERVAL_MS: u64 = 1000;
const ZIBO_SPEED_SMOOTHING_ALPHA: f64 = 0.35;
const ZIBO_CHUNKED_DOWNLOAD_MIN_SIZE: u64 = 512 * 1024;
const ZIBO_CHUNK_REQUEST_WINDOW_BYTES: u64 = 16 * 1024 * 1024;
const ZIBO_CHUNKED_TRANSFER_PROGRESS_RATIO: f64 = 0.88;
const ZIBO_CHUNK_RETRY_BASE_DELAY_MS: u64 = 750;
const ZIBO_CHUNK_RETRY_MAX_DELAY_MS: u64 = 8_000;
const ZIBO_CHUNK_MAX_CONSECUTIVE_FAILURES: usize = 5;
const ZIBO_CHUNK_IDLE_TIMEOUT_SECS: u64 = 6;
const ZIBO_CHUNK_REQUEST_SOFT_TIMEOUT_SECS: u64 = 12;
const ZIBO_FILE_IO_BUFFER_BYTES: usize = 1024 * 1024;
const ZIBO_CONFIG_PATTERNS: [&str; 1] = ["*_prefs.txt"];
const GOOGLE_DRIVE_BROWSER_DOWNLOAD_BASE: &str = "https://drive.usercontent.google.com/download";
const GOOGLE_DRIVE_BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Copy)]
struct DriveSource {
    label: &'static str,
    folder_id: &'static str,
    folder_url: &'static str,
}

const ZIBO_DRIVE_SOURCES: [DriveSource; 2] = [
    DriveSource {
        label: "drive-source-a",
        folder_id: "1qo88h_CCQRRrAMhG3EabHSwa4Lzw-WHL",
        folder_url: "https://drive.google.com/drive/folders/1qo88h_CCQRRrAMhG3EabHSwa4Lzw-WHL",
    },
    DriveSource {
        label: "drive-source-b",
        folder_id: "1PkGPZV2J3Fpq8jkvsxgWtXPLMtjlPpGT",
        folder_url: "https://drive.google.com/drive/folders/1PkGPZV2J3Fpq8jkvsxgWtXPLMtjlPpGT",
    },
];

static ZIBO_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)_(\d+)_(\d+)_(full|\d+)\.zip$").expect("valid zibo title regex")
});
static RSS_ITEM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<item>(.*?)</item>").expect("valid rss item regex"));
static DRIVE_CONFIRM_FORM_ACTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<form[^>]+id=["']download-form["'][^>]+action=["']([^"']+)["']"#)
        .expect("valid drive confirm form regex")
});
static DRIVE_CONFIRM_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)href=["']([^"'#>]*(?:confirm|uuid)=[^"'>]*)["']"#)
        .expect("valid drive confirm link regex")
});
static HTML_INPUT_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<input[^>]*>"#).expect("valid input tag regex"));
static LOCAL_VERSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(\d+)\.(\d+)\.(\d+)\s*$").expect("valid local version regex")
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct VersionTriple {
    major: u32,
    minor: u32,
    patch: u32,
}

impl VersionTriple {
    fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    fn major_minor_matches(self, other: Self) -> bool {
        self.major == other.major && self.minor == other.minor
    }

    fn display_string(self) -> String {
        format!("{}.{:02}.{:02}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone)]
pub struct ZiboRelease {
    version: VersionTriple,
    title: String,
}

impl ZiboRelease {
    pub fn version_string(&self) -> String {
        self.version.display_string()
    }
}

#[derive(Debug, Clone)]
enum LocalVersionState {
    Parsed(VersionTriple),
    Missing,
    Invalid(String),
}

#[derive(Debug, Clone)]
enum ZiboInstallMode {
    Patch,
    MajorClean,
}

impl ZiboInstallMode {
    fn as_plan_value(&self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::MajorClean => "major-clean",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZiboManualDownloadReason {
    DriveLimit,
    ReleasePage,
}

impl ZiboManualDownloadReason {
    fn as_plan_value(&self) -> &'static str {
        match self {
            Self::DriveLimit => "drive-limit",
            Self::ReleasePage => "release-page",
        }
    }
}

#[derive(Debug, Clone)]
struct PlannedZiboUpdate {
    install_mode: ZiboInstallMode,
    primary_release: ZiboRelease,
    follow_up_patch: Option<ZiboRelease>,
}

#[derive(Debug, Clone)]
struct ZiboBackupState {
    temp_dir: PathBuf,
    liveries_path: Option<PathBuf>,
    pref_files: Vec<(String, PathBuf)>,
    original_liveries_info: Option<DirectoryInfo>,
    original_pref_sizes: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Copy)]
struct DirectoryInfo {
    file_count: u64,
    total_size: u64,
}

#[derive(Debug, Clone, Copy)]
struct ProgressRange {
    start: f64,
    end: f64,
}

#[derive(Debug, Clone, Copy)]
struct ChunkedDriveDownloadConfig {
    active_connections: usize,
    window_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
enum DriveDownloadMode {
    Single,
    Chunked(ChunkedDriveDownloadConfig),
}

#[derive(Debug, Clone)]
struct DownloadedDriveArchive {
    release: ZiboRelease,
    file: ResolvedDriveFile,
    zip_path: PathBuf,
}

#[derive(Debug, Clone)]
struct ResolvedDriveFile {
    source: DriveSource,
    file_id: String,
    file_name: String,
    total_bytes: u64,
    web_view_link: Option<String>,
}

#[derive(Debug, Clone)]
struct DriveDownloadPiece {
    index: usize,
    start: u64,
    end: u64,
    part_path: PathBuf,
}

impl DriveDownloadPiece {
    fn len(&self) -> u64 {
        self.end.saturating_sub(self.start).saturating_add(1)
    }

    fn range_label(&self) -> String {
        format!("{}-{}", self.start, self.end)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFilesListResponse {
    #[serde(default)]
    files: Vec<DriveFileRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFileRecord {
    id: String,
    name: String,
    size: Option<String>,
    web_view_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DriveApiErrorEnvelope {
    error: DriveApiErrorPayload,
}

#[derive(Debug, Deserialize)]
struct DriveApiErrorPayload {
    #[serde(default)]
    errors: Vec<DriveApiErrorItem>,
}

#[derive(Debug, Deserialize)]
struct DriveApiErrorItem {
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Clone)]
struct ZiboPlanContext {
    local_state: LocalVersionState,
    latest_release: ZiboRelease,
    install_mode: Option<ZiboInstallMode>,
    primary_release: Option<ZiboRelease>,
    follow_up_patch: Option<ZiboRelease>,
    drive_files: Vec<ResolvedDriveFile>,
    preferred_source_url: String,
    manual_download_url: Option<String>,
    manual_download_reason: Option<ZiboManualDownloadReason>,
    warnings: Vec<String>,
    has_update: bool,
    estimated_download_bytes: u64,
}

fn log_info(message: impl Into<String>) {
    logger::log_info(&message.into(), Some(LOG_CTX));
}

fn log_debug(message: impl Into<String>) {
    logger::log_debug(&message.into(), Some(LOG_CTX), None);
}

fn format_binary_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_elapsed(duration: Duration) -> String {
    format!("{:.3}s", duration.as_secs_f64())
}

fn log_timed_step(step: &str, started_at: Instant, detail: impl Into<String>) {
    let detail = detail.into();
    if detail.is_empty() {
        log_debug(format!(
            "Zibo step '{}' completed in {}",
            step,
            format_elapsed(started_at.elapsed())
        ));
    } else {
        log_debug(format!(
            "Zibo step '{}' completed in {} ({})",
            step,
            format_elapsed(started_at.elapsed()),
            detail
        ));
    }
}

fn emit_progress_event(
    callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
    stage: &str,
    status: &str,
    percentage: f64,
    processed_bytes: u64,
    total_bytes: u64,
    speed_bytes_per_sec: f64,
    message: Option<String>,
    current_file: Option<String>,
) {
    let Some(cb) = callback.as_ref() else {
        return;
    };
    cb(AddonUpdateProgressEvent {
        item_type: item_type.to_string(),
        folder_name: folder_name.to_string(),
        stage: stage.to_string(),
        status: status.to_string(),
        percentage: percentage.clamp(0.0, 100.0),
        processed_units: if total_bytes > 0 { processed_bytes } else { 0 },
        total_units: total_bytes,
        processed_bytes,
        total_bytes,
        speed_bytes_per_sec: speed_bytes_per_sec.max(0.0),
        current_file,
        message,
    });
}

fn take_emit_interval_secs(last_emit_at: &mut Instant, force: bool) -> Option<f64> {
    let now = Instant::now();
    let elapsed = now.duration_since(*last_emit_at);
    if force || elapsed >= Duration::from_millis(ZIBO_PROGRESS_EMIT_INTERVAL_MS) {
        *last_emit_at = now;
        Some(elapsed.as_secs_f64())
    } else {
        None
    }
}

fn smooth_speed(previous: f64, next: f64) -> f64 {
    if next <= 0.0 {
        0.0
    } else if previous <= 0.0 {
        next
    } else {
        previous * (1.0 - ZIBO_SPEED_SMOOTHING_ALPHA) + next * ZIBO_SPEED_SMOOTHING_ALPHA
    }
}

fn interpolate_progress(start: f64, end: f64, processed: u64, total: u64) -> f64 {
    if total == 0 {
        return end;
    }
    let ratio = (processed as f64 / total as f64).clamp(0.0, 1.0);
    start + (end - start) * ratio
}

fn emit_file_operation_progress(
    callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
    progress_range: ProgressRange,
    processed_bytes: u64,
    total_bytes: u64,
    message_prefix: &str,
    current_file: Option<&Path>,
) {
    let current_file_label = current_file.map(|path| path.to_string_lossy().replace('\\', "/"));
    let message = if total_bytes > 0 {
        format!(
            "{} ({}/{})",
            message_prefix,
            format_binary_size(processed_bytes.min(total_bytes)),
            format_binary_size(total_bytes)
        )
    } else {
        message_prefix.to_string()
    };

    emit_progress_event(
        callback,
        item_type,
        folder_name,
        "install",
        "in_progress",
        interpolate_progress(
            progress_range.start,
            progress_range.end,
            processed_bytes,
            total_bytes,
        ),
        processed_bytes,
        total_bytes,
        0.0,
        Some(message),
        current_file_label,
    );
}

fn zibo_full_package_download_mode(options: &AddonUpdateOptions) -> DriveDownloadMode {
    if !options.chunked_download_enabled.unwrap_or(true) {
        return DriveDownloadMode::Single;
    }

    let per_file_connections = options.threads_per_task.unwrap_or(6).clamp(1, 32);
    let total_connections = options.total_threads.unwrap_or(32).clamp(1, 64);
    let active_connections = per_file_connections.min(total_connections);

    if active_connections > 1 {
        DriveDownloadMode::Chunked(ChunkedDriveDownloadConfig {
            active_connections,
            window_bytes: ZIBO_CHUNK_REQUEST_WINDOW_BYTES,
        })
    } else {
        DriveDownloadMode::Single
    }
}

fn ensure_not_cancelled(task_control: Option<&TaskControl>, stage: &str) -> Result<()> {
    if task_control.map(|tc| tc.is_cancelled()).unwrap_or(false) {
        return Err(anyhow!(
            "Addon update {} cancelled by user",
            stage.trim().to_lowercase()
        ));
    }
    Ok(())
}

pub fn is_zibo_aircraft(folder_name: &str, acf_file: &str) -> bool {
    let acf_name = acf_file.trim().to_ascii_lowercase();
    let matches_file = matches!(
        acf_name.as_str(),
        "b738.acf" | "b738_4k.acf" | "b738.xfma" | "b738_4k.xfma"
    );
    if !matches_file {
        return false;
    }

    !matches!(
        folder_name.trim().to_ascii_lowercase().as_str(),
        "boeing 737-800" | "boeing b737-800"
    )
}

pub fn is_zibo_target_path(folder_name: &str, target_path: &Path) -> Result<bool> {
    if !target_path.is_dir() {
        return Ok(false);
    }

    for entry in fs::read_dir(target_path)
        .with_context(|| format!("Failed to read '{}'", target_path.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if is_zibo_aircraft(folder_name, name) {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn read_local_version_for_scan(folder: &Path) -> Option<String> {
    match inspect_local_version(folder) {
        LocalVersionState::Parsed(version) => Some(version.display_string()),
        LocalVersionState::Missing | LocalVersionState::Invalid(_) => None,
    }
}

fn inspect_local_version(folder: &Path) -> LocalVersionState {
    let version_path = folder.join(ZIBO_VERSION_FILE);
    let content = match fs::read_to_string(&version_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return LocalVersionState::Missing
        }
        Err(err) => {
            return LocalVersionState::Invalid(format!(
                "Failed to read '{}': {}",
                version_path.display(),
                err
            ))
        }
    };

    let trimmed = content.lines().next().unwrap_or("").trim();
    if trimmed.is_empty() {
        return LocalVersionState::Invalid(format!("'{}' is empty", version_path.display()));
    }

    match parse_local_version(trimmed) {
        Some(version) => LocalVersionState::Parsed(version),
        None => LocalVersionState::Invalid(format!(
            "'{}' does not contain a valid A.B.C version",
            version_path.display()
        )),
    }
}

fn parse_local_version(value: &str) -> Option<VersionTriple> {
    let captures = LOCAL_VERSION_RE.captures(value.trim())?;
    let major = captures.get(1)?.as_str().parse::<u32>().ok()?;
    let minor = captures.get(2)?.as_str().parse::<u32>().ok()?;
    let patch = captures.get(3)?.as_str().parse::<u32>().ok()?;
    Some(VersionTriple::new(major, minor, patch))
}

pub fn should_offer_update(local_version: Option<&str>, remote_version: &str) -> bool {
    let Some(remote) = parse_local_version(remote_version) else {
        return false;
    };
    match local_version.and_then(parse_local_version) {
        Some(local) => remote > local,
        None => true,
    }
}

pub async fn fetch_latest_release(task_control: Option<&TaskControl>) -> Result<ZiboRelease> {
    let releases = fetch_release_catalog(task_control).await?;
    latest_release_from_catalog(&releases)
}

async fn fetch_release_catalog(task_control: Option<&TaskControl>) -> Result<Vec<ZiboRelease>> {
    ensure_not_cancelled(task_control, "check")?;
    let client = build_http_client(15)?;
    let response = client
        .get(ZIBO_RSS_URL)
        .send()
        .await
        .context("Failed to fetch Zibo RSS feed")?
        .error_for_status()
        .context("Zibo RSS feed returned an error status")?;
    let xml = response
        .text()
        .await
        .context("Failed to read Zibo RSS feed body")?;
    ensure_not_cancelled(task_control, "check")?;
    parse_release_catalog(&xml)
}

fn parse_release_catalog(xml: &str) -> Result<Vec<ZiboRelease>> {
    let mut releases = Vec::new();

    for item_match in RSS_ITEM_RE.captures_iter(xml) {
        let Some(item_body) = item_match.get(1) else {
            continue;
        };
        let Some(title) = extract_xml_tag(item_body.as_str(), "title") else {
            continue;
        };
        if let Some(release) = parse_release_entry(&title) {
            releases.push(release);
        }
    }

    if releases.is_empty() {
        return Err(anyhow!("No usable Zibo release was found in the RSS feed"));
    }

    releases.sort_by_key(|release| release.version);
    releases.dedup_by(|left, right| left.title == right.title);
    Ok(releases)
}

fn latest_release_from_catalog(releases: &[ZiboRelease]) -> Result<ZiboRelease> {
    releases
        .iter()
        .max_by_key(|release| release.version)
        .cloned()
        .ok_or_else(|| anyhow!("No usable Zibo release was found in the RSS feed"))
}

fn latest_release_for_branch(
    releases: &[ZiboRelease],
    version: VersionTriple,
) -> Option<ZiboRelease> {
    releases
        .iter()
        .filter(|release| release.version.major_minor_matches(version))
        .max_by_key(|release| release.version)
        .cloned()
}

fn latest_full_release_for_branch(
    releases: &[ZiboRelease],
    version: VersionTriple,
) -> Option<ZiboRelease> {
    releases
        .iter()
        .filter(|release| {
            release.version.major_minor_matches(version) && release.version.patch == 0
        })
        .max_by_key(|release| release.version)
        .cloned()
}

fn plan_zibo_update(
    local_state: &LocalVersionState,
    releases: &[ZiboRelease],
) -> Result<Option<PlannedZiboUpdate>> {
    let latest_release = latest_release_from_catalog(releases)?;

    match local_state {
        LocalVersionState::Parsed(local) if latest_release.version <= *local => Ok(None),
        LocalVersionState::Parsed(local) if latest_release.version.major_minor_matches(*local) => {
            Ok(Some(PlannedZiboUpdate {
                install_mode: ZiboInstallMode::Patch,
                primary_release: latest_release.clone(),
                follow_up_patch: None,
            }))
        }
        LocalVersionState::Parsed(_)
        | LocalVersionState::Missing
        | LocalVersionState::Invalid(_) => {
            let latest_full = latest_full_release_for_branch(releases, latest_release.version)
                .ok_or_else(|| {
                    anyhow!(
                        "Latest Zibo branch {} does not expose a full package in the RSS feed",
                        latest_release.version_string()
                    )
                })?;
            let follow_up_patch = latest_release_for_branch(releases, latest_release.version)
                .filter(|release| release.version > latest_full.version);
            Ok(Some(PlannedZiboUpdate {
                install_mode: ZiboInstallMode::MajorClean,
                primary_release: latest_full,
                follow_up_patch,
            }))
        }
    }
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let rest = xml.get(start..)?;
    let end = rest.find(&close)?;
    Some(xml_entity_decode(rest[..end].trim()))
}

fn parse_release_entry(title: &str) -> Option<ZiboRelease> {
    let captures = ZIBO_TITLE_RE.captures(title.trim())?;
    let major = captures.get(1)?.as_str().parse::<u32>().ok()?;
    let minor = captures.get(2)?.as_str().parse::<u32>().ok()?;
    let patch = match captures.get(3)?.as_str().to_ascii_lowercase().as_str() {
        "full" => 0,
        value => value.parse::<u32>().ok()?,
    };

    Some(ZiboRelease {
        version: VersionTriple::new(major, minor, patch),
        title: title.trim().to_string(),
    })
}

fn xml_entity_decode(input: &str) -> String {
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn extract_html_attr(tag: &str, attr_name: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!("{attr_name}={quote}");
        if let Some(start) = tag.find(&needle) {
            let value_start = start + needle.len();
            let value = tag.get(value_start..)?.split(quote).next()?.trim();
            if !value.is_empty() {
                return Some(xml_entity_decode(value));
            }
        }
    }

    None
}

fn build_drive_browser_download_url(file_id: &str) -> Result<reqwest::Url> {
    let mut url = reqwest::Url::parse(GOOGLE_DRIVE_BROWSER_DOWNLOAD_BASE)
        .context("Invalid Google Drive browser download base URL")?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("id", file_id);
        query.append_pair("export", "download");
        query.append_pair("confirm", "t");
    }
    Ok(url)
}

fn extract_drive_confirm_download_url(
    response_url: &reqwest::Url,
    body: &str,
) -> Option<reqwest::Url> {
    if let Some(captures) = DRIVE_CONFIRM_FORM_ACTION_RE.captures(body) {
        let action = xml_entity_decode(captures.get(1)?.as_str());
        let mut url = reqwest::Url::parse(&action)
            .ok()
            .or_else(|| response_url.join(&action).ok())?;
        let params = HTML_INPUT_TAG_RE
            .find_iter(body)
            .filter_map(|tag_match| {
                let tag = tag_match.as_str();
                let name = extract_html_attr(tag, "name")?;
                let value = extract_html_attr(tag, "value").unwrap_or_default();
                Some((name, value))
            })
            .filter(|(name, _)| {
                matches!(name.as_str(), "id" | "export" | "confirm" | "uuid" | "at")
            })
            .collect::<Vec<_>>();
        if !params.is_empty() {
            url.query_pairs_mut().clear().extend_pairs(params);
        }
        return Some(url);
    }

    DRIVE_CONFIRM_LINK_RE
        .captures(body)
        .and_then(|captures| captures.get(1))
        .and_then(|value| {
            let href = xml_entity_decode(value.as_str());
            reqwest::Url::parse(&href)
                .ok()
                .or_else(|| response_url.join(&href).ok())
        })
}

fn drive_response_is_html(response: &reqwest::Response) -> bool {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase().contains("text/html"))
        .unwrap_or(false)
}

fn build_drive_download_request(
    client: &reqwest::Client,
    url: reqwest::Url,
    range: Option<(u64, u64)>,
) -> reqwest::RequestBuilder {
    let mut request = client
        .get(url)
        .header(reqwest::header::USER_AGENT, GOOGLE_DRIVE_BROWSER_USER_AGENT)
        .header(reqwest::header::ACCEPT, "*/*")
        .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
        .header(reqwest::header::REFERER, "https://drive.google.com/");
    if let Some((start, end)) = range {
        request = request.header(reqwest::header::RANGE, format!("bytes={start}-{end}"));
    }
    request
}

async fn send_drive_download_request(
    client: &reqwest::Client,
    file: &ResolvedDriveFile,
    range: Option<(u64, u64)>,
    operation_label: &str,
) -> Result<reqwest::Response> {
    let mut url = build_drive_browser_download_url(&file.file_id)?;

    for attempt in 1..=3 {
        let response = build_drive_download_request(client, url.clone(), range)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to start Google Drive {} for '{}' from {}",
                    operation_label, file.file_name, file.source.label
                )
            })?;
        let status = response.status();

        if (status.is_success() || status == reqwest::StatusCode::PARTIAL_CONTENT)
            && !drive_response_is_html(&response)
        {
            return Ok(response);
        }

        let response_url = response.url().clone();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::new())
            .replace('\n', " ");

        if attempt < 3 {
            if let Some(confirm_url) = extract_drive_confirm_download_url(&response_url, &body) {
                if confirm_url != url {
                    log_debug(format!(
                        "Resolved Google Drive confirm URL for '{}' from {} during {} attempt {}",
                        file.file_name, file.source.label, operation_label, attempt
                    ));
                    url = confirm_url;
                    continue;
                }
            }
        }

        let reasons = extract_drive_error_reasons(&body);
        let reason_suffix = if reasons.is_empty() {
            String::new()
        } else {
            format!(" [reason={}]", reasons.join(","))
        };
        let details = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim().chars().take(160).collect::<String>())
        };

        return Err(anyhow!(
            "Google Drive {} returned {} for '{}' from {}{}{}",
            operation_label,
            status.as_u16(),
            file.file_name,
            file.source.label,
            reason_suffix,
            details
        ));
    }

    Err(anyhow!(
        "Google Drive {} could not resolve a download URL for '{}' from {}",
        operation_label,
        file.file_name,
        file.source.label
    ))
}

fn build_http_client(timeout_secs: u64) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("XFast-Manager/ZiboUpdater")
        .build()
        .context("Failed to create HTTP client")
}

fn resolve_drive_api_key() -> Option<String> {
    option_env!("XFAST_ZIBO_GOOGLE_DRIVE_API_KEY")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            std::env::var(GOOGLE_DRIVE_API_KEY_ENV)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

fn require_drive_api_key() -> Result<String> {
    resolve_drive_api_key().ok_or_else(|| {
        anyhow!(
            "Missing {} for Zibo Google Drive search/download",
            GOOGLE_DRIVE_API_KEY_ENV
        )
    })
}

fn drive_source_order_for_seed(seed: u128) -> [DriveSource; 2] {
    if seed % 2 == 0 {
        [ZIBO_DRIVE_SOURCES[0], ZIBO_DRIVE_SOURCES[1]]
    } else {
        [ZIBO_DRIVE_SOURCES[1], ZIBO_DRIVE_SOURCES[0]]
    }
}

fn current_drive_source_order() -> [DriveSource; 2] {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    drive_source_order_for_seed(seed)
}

fn escape_drive_query_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn build_drive_search_query(folder_id: &str, archive_name: &str) -> String {
    format!(
        "'{}' in parents and name = '{}' and trashed = false",
        escape_drive_query_literal(folder_id),
        escape_drive_query_literal(archive_name)
    )
}

async fn search_drive_file_in_source(
    client: &reqwest::Client,
    api_key: &str,
    source: DriveSource,
    archive_name: &str,
    task_control: Option<&TaskControl>,
) -> Result<Option<ResolvedDriveFile>> {
    ensure_not_cancelled(task_control, "scan")?;
    let query = build_drive_search_query(source.folder_id, archive_name);
    let response = client
        .get(format!("{}/files", GOOGLE_DRIVE_API_BASE))
        .query(&[
            ("q", query.as_str()),
            ("fields", "files(id,name,size,webViewLink)"),
            ("pageSize", "10"),
            ("supportsAllDrives", "true"),
            ("includeItemsFromAllDrives", "true"),
            ("key", api_key),
        ])
        .send()
        .await
        .with_context(|| {
            format!(
                "Failed to search '{}' in Google Drive source {}",
                archive_name, source.label
            )
        })?
        .error_for_status()
        .with_context(|| {
            format!(
                "Google Drive source {} returned an error while searching '{}'",
                source.label, archive_name
            )
        })?;

    let payload: DriveFilesListResponse = response
        .json()
        .await
        .context("Failed to decode Google Drive file search response")?;
    ensure_not_cancelled(task_control, "scan")?;

    for file in payload.files {
        if file.name != archive_name || !file.name.to_ascii_lowercase().ends_with(".zip") {
            continue;
        }
        let Some(total_bytes) = file
            .size
            .as_deref()
            .and_then(|value| value.parse::<u64>().ok())
        else {
            continue;
        };
        if total_bytes == 0 {
            continue;
        }

        return Ok(Some(ResolvedDriveFile {
            source,
            file_id: file.id,
            file_name: file.name,
            total_bytes,
            web_view_link: file.web_view_link,
        }));
    }

    Ok(None)
}

async fn probe_drive_file_download(
    client: &reqwest::Client,
    api_key: &str,
    file: &ResolvedDriveFile,
    task_control: Option<&TaskControl>,
) -> Result<()> {
    ensure_not_cancelled(task_control, "scan")?;
    let response = client
        .get(format!("{}/files/{}", GOOGLE_DRIVE_API_BASE, file.file_id))
        .query(&[
            ("alt", "media"),
            ("supportsAllDrives", "true"),
            ("key", api_key),
        ])
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()
        .await
        .with_context(|| {
            format!(
                "Failed to probe Google Drive download for '{}' from {}",
                file.file_name, file.source.label
            )
        })?;
    let status = response.status();

    if status.is_success() || status == reqwest::StatusCode::PARTIAL_CONTENT {
        return Ok(());
    }

    let body = response
        .text()
        .await
        .unwrap_or_else(|_| String::new())
        .replace('\n', " ");
    let reasons = extract_drive_error_reasons(&body);
    let body = body.trim();
    let reason_suffix = if reasons.is_empty() {
        String::new()
    } else {
        format!(" [reason={}]", reasons.join(","))
    };
    let details = if body.is_empty() {
        String::new()
    } else {
        format!(": {}", body.chars().take(160).collect::<String>())
    };

    Err(anyhow!(
        "Google Drive probe returned {} for '{}' from {}{}{}",
        status.as_u16(),
        file.file_name,
        file.source.label,
        reason_suffix,
        details
    ))
}

async fn probe_drive_range_support(
    client: &reqwest::Client,
    _api_key: &str,
    file: &ResolvedDriveFile,
    task_control: Option<&TaskControl>,
) -> Result<bool> {
    ensure_not_cancelled(task_control, "install")?;
    let response = send_drive_download_request(client, file, Some((0, 0)), "range probe").await?;
    let status = response.status();
    if status == reqwest::StatusCode::PARTIAL_CONTENT {
        return Ok(true);
    }
    if status.is_success() {
        return Ok(false);
    }
    Err(anyhow!(
        "Google Drive range probe returned {} for '{}' from {}",
        status.as_u16(),
        file.file_name,
        file.source.label
    ))
}

async fn resolve_drive_files_in_source(
    client: &reqwest::Client,
    api_key: &str,
    source: DriveSource,
    releases: &[ZiboRelease],
    task_control: Option<&TaskControl>,
) -> Result<Vec<ResolvedDriveFile>> {
    let mut files = Vec::with_capacity(releases.len());

    for release in releases {
        let file =
            search_drive_file_in_source(client, api_key, source, &release.title, task_control)
                .await?
                .ok_or_else(|| {
                    anyhow!(
                        "'{}' was not found in Google Drive source {}",
                        release.title,
                        source.label
                    )
                })?;
        probe_drive_file_download(client, api_key, &file, task_control).await?;
        files.push(file);
    }

    Ok(files)
}

async fn resolve_drive_files_with_order(
    releases: &[ZiboRelease],
    source_order: [DriveSource; 2],
    task_control: Option<&TaskControl>,
) -> Result<Vec<ResolvedDriveFile>> {
    let api_key = require_drive_api_key()?;
    let client = build_http_client(20)?;
    let mut failures = Vec::new();

    log_info(format!(
        "Resolving Zibo archives [{}] using Google Drive sources {} -> {}",
        releases
            .iter()
            .map(|release| release.title.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        source_order[0].label,
        source_order[1].label
    ));

    for source in source_order {
        match resolve_drive_files_in_source(&client, &api_key, source, releases, task_control).await
        {
            Ok(files) => return Ok(files),
            Err(err) => failures.push(format!("{}: {}", source.label, err)),
        }
    }

    Err(anyhow!(
        "Failed to validate Zibo archives in configured Google Drive sources: {}",
        failures.join(" | ")
    ))
}

async fn emit_chunked_drive_download_progress(
    progress_callback: Option<AddonUpdateProgressCallback>,
    item_type: String,
    folder_name: String,
    file_name: String,
    source_label: String,
    progress_range: ProgressRange,
    downloaded_before: u64,
    combined_total_bytes: u64,
    file_total_bytes: u64,
    downloaded_bytes: Arc<AtomicU64>,
    finished: Arc<AtomicBool>,
) {
    let mut last_emit_at = Instant::now();
    let mut last_speed_sample_at = Instant::now();
    let mut last_speed_sample_bytes = 0u64;
    let mut displayed_speed = 0.0;

    loop {
        let processed = downloaded_bytes
            .load(Ordering::Relaxed)
            .min(file_total_bytes);
        let is_finished = finished.load(Ordering::Relaxed);

        if take_emit_interval_secs(&mut last_emit_at, is_finished).is_some() {
            let speed_elapsed = last_speed_sample_at.elapsed();
            if is_finished || speed_elapsed >= Duration::from_millis(ZIBO_SPEED_SAMPLE_INTERVAL_MS)
            {
                let elapsed_secs = speed_elapsed.as_secs_f64();
                let speed = if elapsed_secs > 0.0 && processed >= last_speed_sample_bytes {
                    (processed - last_speed_sample_bytes) as f64 / elapsed_secs
                } else {
                    0.0
                };
                displayed_speed = smooth_speed(displayed_speed, speed);
                last_speed_sample_at = Instant::now();
                last_speed_sample_bytes = processed;
            }

            emit_progress_event(
                &progress_callback,
                &item_type,
                &folder_name,
                "install",
                "in_progress",
                interpolate_progress(
                    progress_range.start,
                    progress_range.end,
                    processed,
                    file_total_bytes,
                ),
                downloaded_before.saturating_add(processed),
                combined_total_bytes.max(file_total_bytes),
                displayed_speed,
                Some(format!("Downloading {} from {}", file_name, source_label)),
                Some(file_name.clone()),
            );
        }

        if is_finished {
            break;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn download_drive_piece_with_retry(
    client: &reqwest::Client,
    _api_key: &str,
    file: &ResolvedDriveFile,
    piece: &DriveDownloadPiece,
    downloaded_bytes: &Arc<AtomicU64>,
    task_control: Option<TaskControl>,
    worker_id: usize,
) -> Result<()> {
    let piece_started_at = Instant::now();
    let mut output = TokioOpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&piece.part_path)
        .await
        .with_context(|| format!("Failed to create '{}'", piece.part_path.display()))?;
    let mut next_offset = piece.start;
    let mut consecutive_failures = 0usize;
    let mut request_count = 0usize;
    let max_requests = chunk_request_budget(piece.start, piece.end);

    while next_offset <= piece.end {
        ensure_not_cancelled(task_control.as_ref(), "install")?;
        request_count = request_count.saturating_add(1);
        if request_count > max_requests {
            return Err(anyhow!(
                "Failed while downloading '{}' from {}: piece {} exceeded retry budget after {} requests",
                file.file_name,
                file.source.label,
                piece.index,
                max_requests
            ));
        }

        let range_start = next_offset;
        let request_end = chunk_request_end(range_start, piece.end);
        let request_started_at = Instant::now();
        let response = match send_drive_download_request(
            client,
            file,
            Some((range_start, request_end)),
            "range download",
        )
        .await
        {
            Ok(response) => response,
            Err(err) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                if consecutive_failures > ZIBO_CHUNK_MAX_CONSECUTIVE_FAILURES {
                    return Err(err).with_context(|| {
                        format!(
                            "Failed to start range download for '{}' from {}",
                            file.file_name, file.source.label
                        )
                    });
                }
                let delay = chunk_retry_delay(consecutive_failures);
                log_info(format!(
                    "Retrying Zibo piece {} for '{}' from {} after request error at byte {} (worker={} failure {}/{}; retry in {} ms): {}",
                    piece.index,
                    file.file_name,
                    file.source.label,
                    range_start,
                    worker_id,
                    consecutive_failures,
                    ZIBO_CHUNK_MAX_CONSECUTIVE_FAILURES,
                    delay.as_millis(),
                    err
                ));
                tokio::time::sleep(delay).await;
                continue;
            }
        };
        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(anyhow!(
                "Google Drive range download returned {} for '{}' from {}",
                response.status().as_u16(),
                file.file_name,
                file.source.label
            ));
        }

        let mut stream = response.bytes_stream();
        let mut received_in_request = 0u64;
        let mut request_failed = None;

        loop {
            ensure_not_cancelled(task_control.as_ref(), "install")?;
            let chunk = match tokio::time::timeout(
                Duration::from_secs(ZIBO_CHUNK_IDLE_TIMEOUT_SECS),
                stream.next(),
            )
            .await
            {
                Ok(chunk) => chunk,
                Err(_) => {
                    request_failed = Some(anyhow!(
                        "Timed out waiting for Google Drive data for '{}' from {} at byte {} after {}s of inactivity",
                        file.file_name,
                        file.source.label,
                        next_offset,
                        ZIBO_CHUNK_IDLE_TIMEOUT_SECS
                    ));
                    break;
                }
            };

            match chunk {
                Some(Ok(chunk)) => {
                    output.write_all(&chunk).await.with_context(|| {
                        format!(
                            "Failed to write '{}' while downloading piece {}",
                            piece.part_path.display(),
                            piece.index
                        )
                    })?;

                    let written = chunk.len() as u64;
                    next_offset = next_offset.saturating_add(written);
                    received_in_request = received_in_request.saturating_add(written);
                    downloaded_bytes.fetch_add(written, Ordering::Relaxed);

                    if next_offset <= request_end
                        && request_started_at.elapsed()
                            >= Duration::from_secs(ZIBO_CHUNK_REQUEST_SOFT_TIMEOUT_SECS)
                    {
                        request_failed = Some(anyhow!(
                            "Google Drive range request for '{}' from {} exceeded soft timeout after {}s with {} of {} bytes received",
                            file.file_name,
                            file.source.label,
                            ZIBO_CHUNK_REQUEST_SOFT_TIMEOUT_SECS,
                            received_in_request,
                            request_end.saturating_sub(range_start).saturating_add(1)
                        ));
                        break;
                    }
                }
                Some(Err(err)) => {
                    request_failed = Some(anyhow!(err).context(format!(
                        "Failed while downloading '{}' from {}",
                        file.file_name, file.source.label
                    )));
                    break;
                }
                None => break,
            }
        }

        if next_offset > request_end {
            consecutive_failures = 0;
            continue;
        }

        if next_offset > piece.end {
            break;
        }

        let progress_made = received_in_request > 0;
        if received_in_request > 0 {
            consecutive_failures = 0;
        } else {
            consecutive_failures = consecutive_failures.saturating_add(1);
        }

        if consecutive_failures > ZIBO_CHUNK_MAX_CONSECUTIVE_FAILURES {
            if let Some(err) = request_failed {
                return Err(err);
            }
            return Err(anyhow!(
                "Failed while downloading '{}' from {}: piece {} stopped making progress",
                file.file_name,
                file.source.label,
                piece.index
            ));
        }

        let failure_streak = consecutive_failures.max(1);
        let delay = if progress_made {
            Duration::from_millis(150)
        } else {
            chunk_retry_delay(failure_streak)
        };
        let reason = request_failed
            .map(|err| err.to_string())
            .unwrap_or_else(|| {
                format!(
                    "Google Drive ended piece {} early at byte {} of {}",
                    piece.index, next_offset, request_end
                )
            });
        log_info(format!(
            "Retrying Zibo piece {} for '{}' from {} at byte {} (worker={} failure {}/{}; request {}/{}; retry in {} ms): {}",
            piece.index,
            file.file_name,
            file.source.label,
            next_offset,
            worker_id,
            failure_streak,
            ZIBO_CHUNK_MAX_CONSECUTIVE_FAILURES,
            request_count,
            max_requests,
            delay.as_millis(),
            reason
        ));
        tokio::time::sleep(delay).await;
    }

    output
        .flush()
        .await
        .with_context(|| format!("Failed to flush '{}'", piece.part_path.display()))?;

    let actual_size = tokio::fs::metadata(&piece.part_path)
        .await
        .with_context(|| format!("Failed to stat '{}'", piece.part_path.display()))?
        .len();
    let expected_size = piece.len();
    if actual_size != expected_size {
        return Err(anyhow!(
            "Downloaded size mismatch for '{}' piece {}: expected {}, got {}",
            file.file_name,
            piece.index,
            expected_size,
            actual_size
        ));
    }

    log_timed_step(
        &format!("download piece {}", piece.index),
        piece_started_at,
        format!(
            "worker={} range={} bytes={} requests={}",
            worker_id,
            piece.range_label(),
            format_binary_size(piece.len()),
            request_count
        ),
    );

    Ok(())
}

async fn download_drive_file_to_path_chunked(
    _client: &reqwest::Client,
    api_key: &str,
    file: &ResolvedDriveFile,
    output_path: &Path,
    config: ChunkedDriveDownloadConfig,
    task_control: Option<&TaskControl>,
    progress_callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
    progress_range: ProgressRange,
    downloaded_before: u64,
    combined_total_bytes: u64,
) -> Result<()> {
    let download_started_at = Instant::now();
    let (transfer_range, finalize_range) = split_chunked_download_progress_range(progress_range);
    let parts_dir = build_chunked_download_parts_dir(output_path)?;
    fs::create_dir_all(&parts_dir)
        .with_context(|| format!("Failed to create '{}'", parts_dir.display()))?;
    let pieces = build_drive_download_pieces(file.total_bytes, &parts_dir, config.window_bytes);
    if pieces.is_empty() {
        let _ = fs::remove_dir_all(&parts_dir);
        return Err(anyhow!(
            "Google Drive download for '{}' produced no download pieces",
            file.file_name
        ));
    }
    let active_connections = config.active_connections.min(pieces.len().max(1));
    let piece_count = pieces.len();

    emit_progress_event(
        progress_callback,
        item_type,
        folder_name,
        "install",
        "in_progress",
        transfer_range.start,
        downloaded_before,
        combined_total_bytes.max(file.total_bytes),
        0.0,
        Some(format!(
            "Downloading {} from {}",
            file.file_name, file.source.label
        )),
        Some(file.file_name.clone()),
    );

    let downloaded_bytes = Arc::new(AtomicU64::new(0));
    let finished = Arc::new(AtomicBool::new(false));

    let reporter_handle = tokio::spawn(emit_chunked_drive_download_progress(
        progress_callback.clone(),
        item_type.to_string(),
        folder_name.to_string(),
        file.file_name.clone(),
        file.source.label.to_string(),
        transfer_range,
        downloaded_before,
        combined_total_bytes,
        file.total_bytes,
        Arc::clone(&downloaded_bytes),
        Arc::clone(&finished),
    ));

    log_debug(format!(
        "Starting chunked Zibo download archive={} source={} bytes={} activeConnections={} pieceCount={} window={}",
        file.file_name,
        file.source.label,
        format_binary_size(file.total_bytes),
        active_connections,
        piece_count,
        format_binary_size(config.window_bytes)
    ));

    let queue = Arc::new(TokioMutex::new(VecDeque::from(pieces.clone())));
    let result: Result<()> = async {
        let mut join_set = JoinSet::new();

        for worker_id in 0..active_connections {
            let api_key = api_key.to_string();
            let file = file.clone();
            let queue = Arc::clone(&queue);
            let downloaded_bytes = Arc::clone(&downloaded_bytes);
            let task_control = task_control.cloned();

            join_set.spawn(async move {
                let worker_id = worker_id + 1;
                let client = build_chunked_download_client(180)?;
                log_debug(format!(
                    "Zibo chunk worker {} started for '{}'",
                    worker_id, file.file_name
                ));

                loop {
                    ensure_not_cancelled(task_control.as_ref(), "install")?;
                    let next_piece = {
                        let mut queue = queue.lock().await;
                        queue.pop_front()
                    };

                    let Some(piece) = next_piece else {
                        log_debug(format!(
                            "Zibo chunk worker {} completed all assigned work for '{}'",
                            worker_id, file.file_name
                        ));
                        break;
                    };

                    download_drive_piece_with_retry(
                        &client,
                        api_key.as_str(),
                        &file,
                        &piece,
                        &downloaded_bytes,
                        task_control.clone(),
                        worker_id,
                    )
                    .await?;
                }

                Ok::<(), anyhow::Error>(())
            });
        }

        while let Some(next) = join_set.join_next().await {
            match next {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    join_set.abort_all();
                    while join_set.join_next().await.is_some() {}
                    return Err(err);
                }
                Err(err) => {
                    join_set.abort_all();
                    while join_set.join_next().await.is_some() {}
                    return Err(anyhow!("Chunk download task panicked: {}", err));
                }
            }
        }

        Ok(())
    }
    .await;

    finished.store(true, Ordering::Relaxed);
    let _ = reporter_handle.await;

    if let Err(err) = result {
        let _ = fs::remove_dir_all(&parts_dir);
        let _ = fs::remove_file(output_path);
        return Err(err);
    }

    let processed_bytes = downloaded_bytes.load(Ordering::Relaxed);
    if processed_bytes == 0 {
        let _ = fs::remove_dir_all(&parts_dir);
        let _ = fs::remove_file(output_path);
        return Err(anyhow!(
            "Google Drive download for '{}' produced an empty file",
            file.file_name
        ));
    }

    if processed_bytes != file.total_bytes {
        let _ = fs::remove_dir_all(&parts_dir);
        let _ = fs::remove_file(output_path);
        return Err(anyhow!(
            "Downloaded size mismatch for '{}': expected {}, got {}",
            file.file_name,
            file.total_bytes,
            processed_bytes
        ));
    }

    let merged_path = parts_dir.join("merged.tmp");
    let merge_started_at = Instant::now();
    let mut last_merge_emit_at = Instant::now();
    let merge_result = merge_download_pieces_with_progress(
        &pieces,
        &merged_path,
        |merged_bytes, total_bytes, _| {
            ensure_not_cancelled(task_control, "install")?;
            let force_emit = total_bytes == 0 || merged_bytes >= total_bytes;
            if take_emit_interval_secs(&mut last_merge_emit_at, force_emit).is_some() {
                emit_file_operation_progress(
                    progress_callback,
                    item_type,
                    folder_name,
                    finalize_range,
                    merged_bytes,
                    total_bytes,
                    "Finalizing Zibo archive",
                    Some(Path::new(&file.file_name)),
                );
            }
            Ok(())
        },
    );
    if let Err(err) = merge_result {
        let _ = fs::remove_file(&merged_path);
        let _ = fs::remove_dir_all(&parts_dir);
        let _ = fs::remove_file(output_path);
        return Err(err);
    }

    let merged_size = fs::metadata(&merged_path)
        .with_context(|| format!("Failed to stat '{}'", merged_path.display()))?
        .len();
    if merged_size != file.total_bytes {
        let _ = fs::remove_file(&merged_path);
        let _ = fs::remove_dir_all(&parts_dir);
        let _ = fs::remove_file(output_path);
        return Err(anyhow!(
            "Merged size mismatch for '{}': expected {}, got {}",
            file.file_name,
            file.total_bytes,
            merged_size
        ));
    }
    if output_path.exists() {
        let _ = fs::remove_file(output_path);
    }
    fs::rename(&merged_path, output_path).with_context(|| {
        format!(
            "Failed to move merged archive '{}' to '{}'",
            merged_path.display(),
            output_path.display()
        )
    })?;

    log_timed_step(
        "merge archive parts",
        merge_started_at,
        format!(
            "archive={} pieceCount={} bytes={}",
            file.file_name,
            piece_count,
            format_binary_size(file.total_bytes)
        ),
    );

    if let Err(err) = fs::remove_dir_all(&parts_dir) {
        log_info(format!(
            "Zibo chunked download finished but failed to remove parts directory '{}': {}",
            parts_dir.display(),
            err
        ));
    }

    log_timed_step(
        "download archive",
        download_started_at,
        format!(
            "mode=chunked archive={} bytes={} source={} activeConnections={} pieceCount={} window={}",
            file.file_name,
            format_binary_size(file.total_bytes),
            file.source.label,
            active_connections,
            piece_count,
            format_binary_size(config.window_bytes)
        ),
    );

    Ok(())
}

async fn download_drive_file_to_path(
    client: &reqwest::Client,
    api_key: &str,
    file: &ResolvedDriveFile,
    output_path: &Path,
    download_mode: DriveDownloadMode,
    task_control: Option<&TaskControl>,
    progress_callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
    progress_range: ProgressRange,
    downloaded_before: u64,
    combined_total_bytes: u64,
) -> Result<()> {
    ensure_not_cancelled(task_control, "install")?;
    let download_started_at = Instant::now();

    if let DriveDownloadMode::Chunked(config) = download_mode {
        if file.total_bytes >= ZIBO_CHUNKED_DOWNLOAD_MIN_SIZE {
            match probe_drive_range_support(client, api_key, file, task_control).await {
                Ok(true) => {
                    return download_drive_file_to_path_chunked(
                        client,
                        api_key,
                        file,
                        output_path,
                        config,
                        task_control,
                        progress_callback,
                        item_type,
                        folder_name,
                        progress_range,
                        downloaded_before,
                        combined_total_bytes,
                    )
                    .await;
                }
                Ok(false) => {
                    log_info(format!(
                        "Google Drive range download is unavailable for '{}' from {}; falling back to single-connection download",
                        file.file_name, file.source.label
                    ));
                }
                Err(err) => {
                    log_info(format!(
                        "Failed to probe Google Drive range download for '{}' from {} ({}); falling back to single-connection download",
                        file.file_name, file.source.label, err
                    ));
                }
            }
        }
    }

    let response = send_drive_download_request(client, file, None, "download").await?;

    let mut stream = response.bytes_stream();
    let output = fs::File::create(output_path)
        .with_context(|| format!("Failed to create '{}'", output_path.display()))?;
    let mut output = BufWriter::new(output);
    let mut processed_bytes = 0u64;
    let mut last_emit_at = Instant::now();
    let mut last_speed_sample_at = Instant::now();
    let mut last_speed_sample_bytes = 0u64;
    let mut displayed_speed = 0.0;

    while let Some(chunk) = stream.next().await {
        ensure_not_cancelled(task_control, "install")?;
        let chunk = chunk.with_context(|| {
            format!(
                "Failed while downloading '{}' from {}",
                file.file_name, file.source.label
            )
        })?;
        output.write_all(&chunk).with_context(|| {
            format!(
                "Failed to write '{}' while downloading Zibo archive",
                output_path.display()
            )
        })?;
        processed_bytes = processed_bytes.saturating_add(chunk.len() as u64);

        let processed_for_display = processed_bytes.min(file.total_bytes);
        let force_emit = file.total_bytes > 0 && processed_for_display >= file.total_bytes;
        if take_emit_interval_secs(&mut last_emit_at, force_emit).is_some() {
            let speed_elapsed = last_speed_sample_at.elapsed();
            if force_emit || speed_elapsed >= Duration::from_millis(ZIBO_SPEED_SAMPLE_INTERVAL_MS) {
                let elapsed_secs = speed_elapsed.as_secs_f64();
                let speed = if elapsed_secs > 0.0 && processed_bytes >= last_speed_sample_bytes {
                    (processed_bytes - last_speed_sample_bytes) as f64 / elapsed_secs
                } else {
                    0.0
                };
                displayed_speed = smooth_speed(displayed_speed, speed);
                last_speed_sample_at = Instant::now();
                last_speed_sample_bytes = processed_bytes;
            }

            emit_progress_event(
                progress_callback,
                item_type,
                folder_name,
                "install",
                "in_progress",
                interpolate_progress(
                    progress_range.start,
                    progress_range.end,
                    processed_for_display,
                    file.total_bytes,
                ),
                downloaded_before.saturating_add(processed_for_display),
                combined_total_bytes.max(file.total_bytes),
                displayed_speed,
                Some(format!(
                    "Downloading {} from {}",
                    file.file_name, file.source.label
                )),
                Some(file.file_name.clone()),
            );
        }
    }

    output
        .flush()
        .with_context(|| format!("Failed to flush '{}'", output_path.display()))?;

    if processed_bytes == 0 {
        return Err(anyhow!(
            "Google Drive download for '{}' produced an empty file",
            file.file_name
        ));
    }

    if file.total_bytes > 0 && processed_bytes != file.total_bytes {
        return Err(anyhow!(
            "Downloaded size mismatch for '{}': expected {}, got {}",
            file.file_name,
            file.total_bytes,
            processed_bytes
        ));
    }

    log_timed_step(
        "download archive",
        download_started_at,
        format!(
            "mode=single archive={} bytes={} source={}",
            file.file_name,
            format_binary_size(file.total_bytes),
            file.source.label
        ),
    );

    Ok(())
}

fn releases_for_plan(update: &PlannedZiboUpdate) -> Vec<ZiboRelease> {
    let mut releases = vec![update.primary_release.clone()];
    if let Some(patch) = update.follow_up_patch.clone() {
        releases.push(patch);
    }
    releases
}

async fn download_releases_from_drive_with_order(
    releases: &[ZiboRelease],
    source_order: [DriveSource; 2],
    download_root: &Path,
    primary_archive_download_mode: DriveDownloadMode,
    task_control: Option<&TaskControl>,
    progress_callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
    download_progress_ranges: &[ProgressRange],
) -> Result<Vec<DownloadedDriveArchive>> {
    if releases.is_empty() {
        return Ok(Vec::new());
    }
    if download_progress_ranges.len() < releases.len() {
        return Err(anyhow!(
            "Missing download progress ranges for Zibo download plan"
        ));
    }

    let api_key = require_drive_api_key()?;
    let client = build_http_client(180)?;
    let mut failures = Vec::new();
    let combined_total_bytes = resolve_drive_files_with_order(releases, source_order, task_control)
        .await?
        .iter()
        .map(|file| file.total_bytes)
        .sum::<u64>();

    log_info(format!(
        "Downloading Zibo archives [{}] using Google Drive sources {} -> {}",
        releases
            .iter()
            .map(|release| release.title.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        source_order[0].label,
        source_order[1].label
    ));

    for source in source_order {
        ensure_not_cancelled(task_control, "install")?;
        emit_progress_event(
            progress_callback,
            item_type,
            folder_name,
            "install",
            "in_progress",
            0.0,
            0,
            combined_total_bytes,
            0.0,
            Some(format!("Searching Zibo archives in {}", source.label)),
            None,
        );

        let files =
            match resolve_drive_files_in_source(&client, &api_key, source, releases, task_control)
                .await
            {
                Ok(files) => files,
                Err(err) => {
                    failures.push(format!("{}: {}", source.label, err));
                    continue;
                }
            };

        let mut downloaded = Vec::with_capacity(files.len());
        let mut downloaded_before = 0u64;
        let mut failed_download = None;

        for (idx, (release, file)) in releases.iter().cloned().zip(files.into_iter()).enumerate() {
            let zip_path = download_root.join(&file.file_name);
            if zip_path.exists() {
                fs::remove_file(&zip_path)
                    .with_context(|| format!("Failed to reset '{}'", zip_path.display()))?;
            }

            let download_mode = if idx == 0 {
                primary_archive_download_mode
            } else {
                DriveDownloadMode::Single
            };

            match download_drive_file_to_path(
                &client,
                &api_key,
                &file,
                &zip_path,
                download_mode,
                task_control,
                progress_callback,
                item_type,
                folder_name,
                download_progress_ranges
                    .get(idx)
                    .copied()
                    .ok_or_else(|| anyhow!("Missing Zibo download progress range {}", idx))?,
                downloaded_before,
                combined_total_bytes,
            )
            .await
            {
                Ok(()) => {
                    downloaded_before = downloaded_before.saturating_add(file.total_bytes);
                    downloaded.push(DownloadedDriveArchive {
                        release,
                        file,
                        zip_path,
                    });
                }
                Err(err) => {
                    failed_download = Some(err);
                    break;
                }
            }
        }

        if let Some(err) = failed_download {
            for archive in &downloaded {
                let _ = fs::remove_file(&archive.zip_path);
            }
            failures.push(format!("{}: {}", source.label, err));
            continue;
        }

        return Ok(downloaded);
    }

    Err(anyhow!(
        "Failed to download Zibo archives from configured Google Drive sources: {}",
        failures.join(" | ")
    ))
}

fn build_plan_context(
    local_state: &LocalVersionState,
    latest_release: ZiboRelease,
    planned_update: Option<&PlannedZiboUpdate>,
    preferred_manual_download_url: String,
) -> ZiboPlanContext {
    let mut warnings = Vec::new();

    if planned_update.is_some() {
        match local_state {
            LocalVersionState::Missing => warnings.push(
                "Local Zibo version.txt was not found; XFast Manager will use a clean install flow."
                    .to_string(),
            ),
            LocalVersionState::Invalid(err) => warnings.push(format!(
                "Local Zibo version.txt could not be parsed; XFast Manager will use a clean install flow. {}",
                err
            )),
            LocalVersionState::Parsed(_) => {}
        }
    }

    match planned_update {
        None => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            install_mode: None,
            primary_release: None,
            follow_up_patch: None,
            drive_files: Vec::new(),
            preferred_source_url: preferred_manual_download_url,
            manual_download_url: None,
            manual_download_reason: None,
            warnings: Vec::new(),
            has_update: false,
            estimated_download_bytes: 0,
        },
        Some(update) => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            install_mode: Some(update.install_mode.clone()),
            primary_release: Some(update.primary_release.clone()),
            follow_up_patch: update.follow_up_patch.clone(),
            drive_files: Vec::new(),
            preferred_source_url: preferred_manual_download_url,
            manual_download_url: None,
            manual_download_reason: None,
            warnings,
            has_update: true,
            estimated_download_bytes: 0,
        },
    }
}

fn local_version_text(local_state: &LocalVersionState) -> Option<String> {
    match local_state {
        LocalVersionState::Parsed(version) => Some(version.display_string()),
        LocalVersionState::Missing | LocalVersionState::Invalid(_) => None,
    }
}

async fn build_plan_context_from_target(
    target_path: &Path,
    task_control: Option<&TaskControl>,
) -> Result<ZiboPlanContext> {
    let local_state = inspect_local_version(target_path);
    let releases = fetch_release_catalog(task_control).await?;
    let latest_release = latest_release_from_catalog(&releases)?;
    let source_order = current_drive_source_order();
    let planned_update = plan_zibo_update(&local_state, &releases)?;
    let mut context = build_plan_context(
        &local_state,
        latest_release,
        planned_update.as_ref(),
        source_order[0].folder_url.to_string(),
    );

    if let Some(update) = planned_update.as_ref() {
        let releases = releases_for_plan(update);
        match resolve_drive_files_with_order(&releases, source_order, task_control).await {
            Ok(files) => {
                context.estimated_download_bytes =
                    files.iter().map(|file| file.total_bytes).sum::<u64>();
                context.drive_files = files;
            }
            Err(err) => {
                log_info(format!(
                    "Zibo drive probe failed; falling back to manual alternative links: {}",
                    err
                ));
                if drive_probe_failed_only_due_to_missing_files(&err) {
                    context.manual_download_url = Some(ZIBO_MANUAL_FALLBACK_URL.to_string());
                    context.manual_download_reason = Some(ZiboManualDownloadReason::ReleasePage);
                    context.warnings.push(
                        "The expected Zibo archive was not present in the Google Drive mirrors during the lightweight probe yet."
                            .to_string(),
                    );
                } else if drive_probe_contains_quota_exceeded(&err) {
                    context.manual_download_url = Some(ZIBO_MANUAL_FALLBACK_URL.to_string());
                    context.manual_download_reason = Some(ZiboManualDownloadReason::DriveLimit);
                    context.warnings.push(
                        "Google Drive reported that the download quota for the required Zibo package has been exceeded."
                            .to_string(),
                    );
                } else {
                    return Err(err.context("Failed to probe Zibo Google Drive availability"));
                }
            }
        }
    }

    Ok(context)
}

pub async fn build_update_plan(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
    _options: AddonUpdateOptions,
    task_control: Option<TaskControl>,
    progress_callback: Option<AddonUpdateProgressCallback>,
) -> Result<AddonUpdatePlan> {
    let target_path = resolve_zibo_target_path(xplane_path, item_type, folder_name)?;
    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "scan",
        "started",
        0.0,
        0,
        0,
        0.0,
        Some("Checking Zibo metadata".to_string()),
        None,
    );
    ensure_not_cancelled(task_control.as_ref(), "scan")?;

    let context = build_plan_context_from_target(&target_path, task_control.as_ref()).await?;
    let plan = AddonUpdatePlan {
        provider: ZIBO_PROVIDER.to_string(),
        item_type: item_type.to_string(),
        folder_name: folder_name.to_string(),
        local_version: local_version_text(&context.local_state),
        remote_version: Some(context.latest_release.version_string()),
        remote_module: Some(plan_remote_module_url(&context)),
        manual_download_url: context.manual_download_url,
        manual_download_reason: context
            .manual_download_reason
            .map(|reason| reason.as_plan_value().to_string()),
        zibo_install_mode: context
            .install_mode
            .as_ref()
            .map(|mode| mode.as_plan_value().to_string()),
        remote_locked: false,
        has_update: context.has_update,
        estimated_download_bytes: context.estimated_download_bytes,
        add_files: Vec::new(),
        replace_files: Vec::new(),
        delete_files: Vec::new(),
        skip_files: Vec::new(),
        warnings: context.warnings,
        has_beta_config: false,
    };

    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "scan",
        "completed",
        100.0,
        0,
        plan.estimated_download_bytes,
        0.0,
        Some("Zibo update plan ready".to_string()),
        None,
    );

    Ok(plan)
}

pub async fn execute_update(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
    options: AddonUpdateOptions,
    task_control: Option<TaskControl>,
    progress_callback: Option<AddonUpdateProgressCallback>,
) -> Result<AddonUpdateResult> {
    let execute_started_at = Instant::now();
    let target_path = resolve_zibo_target_path(xplane_path, item_type, folder_name)?;
    let context = build_plan_context_from_target(&target_path, task_control.as_ref()).await?;
    let local_version = local_version_text(&context.local_state);
    let remote_version = Some(context.latest_release.version_string());

    if !context.has_update {
        return Ok(AddonUpdateResult {
            provider: ZIBO_PROVIDER.to_string(),
            success: true,
            message: "Already up to date".to_string(),
            item_type: item_type.to_string(),
            folder_name: folder_name.to_string(),
            local_version,
            remote_version,
            updated_files: 0,
            deleted_files: 0,
            skipped_files: 0,
            rollback_used: false,
        });
    }

    if let Some(manual_download_reason) = context.manual_download_reason {
        let manual_download_url = context
            .manual_download_url
            .clone()
            .unwrap_or_else(|| ZIBO_MANUAL_FALLBACK_URL.to_string());
        return Err(match manual_download_reason {
            ZiboManualDownloadReason::DriveLimit => anyhow!(
                "Google Drive has reached its 24-hour download limit for this Zibo package. Use the alternative download links: {}",
                manual_download_url
            ),
            ZiboManualDownloadReason::ReleasePage => anyhow!(
                "The Zibo Google Drive files may not be updated yet. Please retry later or visit the release page for other download options: {}",
                manual_download_url
            ),
        });
    }

    let install_mode = context
        .install_mode
        .clone()
        .ok_or_else(|| anyhow!("Missing Zibo install mode for update execution"))?;
    let planned_update = PlannedZiboUpdate {
        install_mode: install_mode.clone(),
        primary_release: context
            .primary_release
            .clone()
            .ok_or_else(|| anyhow!("Missing Zibo release plan for update execution"))?,
        follow_up_patch: context.follow_up_patch.clone(),
    };
    let total_download_bytes = context.estimated_download_bytes;
    let full_package_download_mode = zibo_full_package_download_mode(&options);

    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "install",
        "started",
        0.0,
        0,
        total_download_bytes,
        0.0,
        Some(match install_mode {
            ZiboInstallMode::Patch => "Preparing Zibo patch update".to_string(),
            ZiboInstallMode::MajorClean => "Preparing Zibo clean install update".to_string(),
        }),
        None,
    );

    let (updated_files, source_label, success_suffix) = match install_mode {
        ZiboInstallMode::Patch => {
            execute_patch_update(
                &planned_update,
                &target_path,
                total_download_bytes,
                task_control.as_ref(),
                &progress_callback,
                item_type,
                folder_name,
            )
            .await?
        }
        ZiboInstallMode::MajorClean => {
            let preserve_liveries = if options.fresh_install {
                options.preserve_liveries
            } else {
                true
            };
            let preserve_config_files = if options.fresh_install {
                options.preserve_config_files
            } else {
                true
            };
            execute_major_clean_update(
                &planned_update,
                &target_path,
                preserve_liveries,
                preserve_config_files,
                full_package_download_mode,
                total_download_bytes,
                task_control.as_ref(),
                &progress_callback,
                item_type,
                folder_name,
            )
            .await?
        }
    };

    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "install",
        "completed",
        100.0,
        total_download_bytes,
        total_download_bytes,
        0.0,
        Some("Zibo update installed".to_string()),
        None,
    );

    log_info(format!(
        "Installed Zibo update '{}' from {} into '{}'",
        context.latest_release.title,
        source_label,
        target_path.display()
    ));

    let mut message = format!(
        "Updated Zibo to {}",
        context.latest_release.version_string()
    );
    if let Some(suffix) = success_suffix {
        message.push(' ');
        message.push_str(&suffix);
    }

    log_timed_step(
        "execute update",
        execute_started_at,
        format!(
            "target={} version={} source={}",
            target_path.display(),
            context.latest_release.version_string(),
            source_label
        ),
    );

    Ok(AddonUpdateResult {
        provider: ZIBO_PROVIDER.to_string(),
        success: true,
        message,
        item_type: item_type.to_string(),
        folder_name: folder_name.to_string(),
        local_version,
        remote_version,
        updated_files,
        deleted_files: 0,
        skipped_files: 0,
        rollback_used: false,
    })
}

fn plan_remote_module_url(context: &ZiboPlanContext) -> String {
    context
        .drive_files
        .first()
        .and_then(|file| file.web_view_link.clone())
        .or_else(|| {
            context
                .drive_files
                .first()
                .map(|file| file.source.folder_url.to_string())
        })
        .or_else(|| context.manual_download_url.clone())
        .unwrap_or_else(|| context.preferred_source_url.clone())
}

fn drive_probe_failed_only_due_to_missing_files(err: &anyhow::Error) -> bool {
    let message = err.to_string();
    message.contains("was not found in Google Drive source")
        && !message.contains("probe returned")
        && !message.contains("Failed to probe Google Drive download")
        && !message.contains("returned an error while searching")
        && !message.contains("Failed to search")
}

fn drive_probe_contains_quota_exceeded(err: &anyhow::Error) -> bool {
    err.to_string().contains("reason=downloadQuotaExceeded")
}

fn extract_drive_error_reasons(body: &str) -> Vec<String> {
    serde_json::from_str::<DriveApiErrorEnvelope>(body)
        .ok()
        .map(|payload| {
            payload
                .error
                .errors
                .into_iter()
                .map(|item| item.reason.trim().to_string())
                .filter(|reason| !reason.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn chunk_retry_delay(consecutive_failures: usize) -> Duration {
    let exponent = consecutive_failures.saturating_sub(1).min(4) as u32;
    let millis = ZIBO_CHUNK_RETRY_BASE_DELAY_MS.saturating_mul(1u64 << exponent);
    Duration::from_millis(millis.min(ZIBO_CHUNK_RETRY_MAX_DELAY_MS))
}

fn chunk_request_end(start: u64, end: u64) -> u64 {
    start
        .saturating_add(ZIBO_CHUNK_REQUEST_WINDOW_BYTES.saturating_sub(1))
        .min(end)
}

fn chunk_request_budget(start: u64, end: u64) -> usize {
    let chunk_bytes = end.saturating_sub(start).saturating_add(1);
    let expected_requests = chunk_bytes
        .saturating_add(ZIBO_CHUNK_REQUEST_WINDOW_BYTES.saturating_sub(1))
        / ZIBO_CHUNK_REQUEST_WINDOW_BYTES.max(1);
    usize::try_from(
        expected_requests
            .saturating_mul(4)
            .saturating_add(ZIBO_CHUNK_MAX_CONSECUTIVE_FAILURES as u64),
    )
    .unwrap_or(usize::MAX)
    .max(ZIBO_CHUNK_MAX_CONSECUTIVE_FAILURES + 1)
}

fn split_chunked_download_progress_range(
    progress_range: ProgressRange,
) -> (ProgressRange, ProgressRange) {
    let span = (progress_range.end - progress_range.start).max(0.0);
    let transfer_end =
        progress_range.start + span * ZIBO_CHUNKED_TRANSFER_PROGRESS_RATIO.clamp(0.0, 1.0);
    (
        ProgressRange {
            start: progress_range.start,
            end: transfer_end,
        },
        ProgressRange {
            start: transfer_end,
            end: progress_range.end,
        },
    )
}

fn build_chunked_download_parts_dir(output_path: &Path) -> Result<PathBuf> {
    let parent = output_path
        .parent()
        .ok_or_else(|| anyhow!("Download output path does not have a parent directory"))?;
    let file_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Download output path does not have a valid file name"))?;

    for _ in 0..8 {
        let candidate = parent.join(format!("{}.parts-{}", file_name, Uuid::new_v4().simple()));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "Failed to allocate a temporary parts directory for '{}'",
        output_path.display()
    ))
}

fn build_drive_download_pieces(
    file_size: u64,
    parts_dir: &Path,
    window_bytes: u64,
) -> Vec<DriveDownloadPiece> {
    if file_size == 0 {
        return Vec::new();
    }

    let window_bytes = window_bytes.max(1);
    let mut pieces = Vec::new();
    let mut start = 0u64;
    let mut index = 0usize;

    while start < file_size {
        let end = start
            .saturating_add(window_bytes.saturating_sub(1))
            .min(file_size.saturating_sub(1));
        pieces.push(DriveDownloadPiece {
            index,
            start,
            end,
            part_path: parts_dir.join(format!("{:05}_{}-{}.part", index, start, end)),
        });
        start = end.saturating_add(1);
        index = index.saturating_add(1);
    }

    pieces
}

fn total_piece_bytes(pieces: &[DriveDownloadPiece]) -> u64 {
    pieces.iter().map(DriveDownloadPiece::len).sum()
}

fn build_chunked_download_client(timeout_secs: u64) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("XFast-Manager/ZiboUpdater")
        .http1_only()
        .build()
        .context("Failed to create chunked download HTTP client")
}

fn merge_download_pieces_with_progress<F>(
    pieces: &[DriveDownloadPiece],
    merged_path: &Path,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(u64, u64, Option<&Path>) -> Result<()>,
{
    let total_bytes = total_piece_bytes(pieces);
    let mut processed_bytes = 0u64;
    let mut output = fs::File::create(merged_path)
        .with_context(|| format!("Failed to create '{}'", merged_path.display()))?;
    let mut buffer = vec![0u8; ZIBO_FILE_IO_BUFFER_BYTES];

    for piece in pieces {
        let mut input = fs::File::open(&piece.part_path)
            .with_context(|| format!("Failed to open '{}'", piece.part_path.display()))?;
        let mut remaining = piece.len();
        while remaining > 0 {
            let read_limit = remaining.min(buffer.len() as u64) as usize;
            let read = input
                .read(&mut buffer[..read_limit])
                .with_context(|| format!("Failed to read '{}'", piece.part_path.display()))?;
            if read == 0 {
                return Err(anyhow!(
                    "Unexpected end of part file '{}'",
                    piece.part_path.display()
                ));
            }
            output
                .write_all(&buffer[..read])
                .with_context(|| format!("Failed to write '{}'", merged_path.display()))?;
            remaining = remaining.saturating_sub(read as u64);
            processed_bytes = processed_bytes.saturating_add(read as u64);
            on_progress(
                processed_bytes,
                total_bytes,
                Some(piece.part_path.as_path()),
            )?;
        }
    }

    output
        .flush()
        .with_context(|| format!("Failed to flush '{}'", merged_path.display()))?;
    on_progress(processed_bytes, total_bytes, None)?;
    Ok(())
}

async fn execute_patch_update(
    planned_update: &PlannedZiboUpdate,
    target_path: &Path,
    total_download_bytes: u64,
    task_control: Option<&TaskControl>,
    progress_callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
) -> Result<(usize, String, Option<String>)> {
    let patch_started_at = Instant::now();
    let workdir = tempfile::tempdir().context("Failed to create Zibo update temp directory")?;
    let downloads_dir = workdir.path().join("downloads");
    let unpacked_dir = workdir.path().join("unpacked");
    fs::create_dir_all(&downloads_dir).context("Failed to create Zibo download directory")?;
    fs::create_dir_all(&unpacked_dir).context("Failed to create Zibo extraction directory")?;

    let download_started_at = Instant::now();
    let archives = download_releases_from_drive_with_order(
        &[planned_update.primary_release.clone()],
        current_drive_source_order(),
        &downloads_dir,
        DriveDownloadMode::Single,
        task_control,
        progress_callback,
        item_type,
        folder_name,
        &[ProgressRange {
            start: 0.0,
            end: ZIBO_PATCH_DOWNLOAD_PROGRESS_MAX,
        }],
    )
    .await?;
    let archive = archives
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Patch download completed without a Zibo archive"))?;
    let source_label = archive.file.source.label.to_string();
    log_timed_step(
        "patch download",
        download_started_at,
        format!(
            "archive={} bytes={} source={}",
            archive.release.title,
            format_binary_size(archive.file.total_bytes),
            source_label
        ),
    );

    let apply_started_at = Instant::now();
    let updated_files = apply_downloaded_archive_to_target(
        &archive,
        &unpacked_dir,
        target_path,
        ProgressRange {
            start: ZIBO_PATCH_DOWNLOAD_PROGRESS_MAX,
            end: ZIBO_PATCH_EXTRACT_PROGRESS_MAX,
        },
        ProgressRange {
            start: ZIBO_PATCH_EXTRACT_PROGRESS_MAX,
            end: ZIBO_PATCH_COPY_PROGRESS_MAX,
        },
        total_download_bytes.max(archive.file.total_bytes),
        task_control,
        progress_callback,
        item_type,
        folder_name,
    )?;
    log_timed_step(
        "patch install workflow",
        apply_started_at,
        format!(
            "archive={} updatedFiles={}",
            archive.release.title, updated_files
        ),
    );
    log_timed_step("patch update total", patch_started_at, String::new());

    Ok((updated_files, source_label, None))
}

async fn execute_major_clean_update(
    planned_update: &PlannedZiboUpdate,
    target_path: &Path,
    preserve_liveries: bool,
    preserve_config_files: bool,
    full_package_download_mode: DriveDownloadMode,
    total_download_bytes: u64,
    task_control: Option<&TaskControl>,
    progress_callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
) -> Result<(usize, String, Option<String>)> {
    let major_started_at = Instant::now();
    emit_progress_event(
        progress_callback,
        item_type,
        folder_name,
        "install",
        "in_progress",
        0.0,
        0,
        total_download_bytes,
        0.0,
        Some("Preparing Zibo clean install backup".to_string()),
        None,
    );

    let backup_started_at = Instant::now();
    let mut last_backup_emit_at = Instant::now();
    let backup = prepare_zibo_backup(
        target_path,
        preserve_liveries,
        preserve_config_files,
        |processed_bytes, total_bytes, processed_files, total_files, current_file| {
            ensure_not_cancelled(task_control, "install")?;
            let force_emit = total_bytes == 0 || processed_bytes >= total_bytes;
            if take_emit_interval_secs(&mut last_backup_emit_at, force_emit).is_some() {
                emit_file_operation_progress(
                    progress_callback,
                    item_type,
                    folder_name,
                    ProgressRange {
                        start: 0.0,
                        end: ZIBO_MAJOR_BACKUP_PROGRESS_MAX,
                    },
                    processed_bytes,
                    total_bytes,
                    &format!(
                        "Backing up Zibo files ({}/{})",
                        processed_files, total_files
                    ),
                    current_file,
                );
            }
            Ok(())
        },
    )?;
    if let Some(backup_state) = backup.as_ref() {
        let (backup_files, backup_bytes) = backup_totals(backup_state);
        log_timed_step(
            "prepare backup",
            backup_started_at,
            format!(
                "files={} bytes={} preserveLiveries={} preserveConfig={}",
                backup_files,
                format_binary_size(backup_bytes),
                preserve_liveries,
                preserve_config_files
            ),
        );
    } else {
        log_timed_step(
            "prepare backup",
            backup_started_at,
            "no files selected for preservation".to_string(),
        );
    }

    let verify_backup_started_at = Instant::now();
    if let Some(backup_state) = backup.as_ref() {
        verify_zibo_backup(backup_state)
            .context("Backup verification failed before clean install")?;
    }
    log_timed_step("verify backup", verify_backup_started_at, String::new());

    emit_progress_event(
        progress_callback,
        item_type,
        folder_name,
        "install",
        "in_progress",
        ZIBO_MAJOR_BACKUP_PROGRESS_MAX,
        0,
        total_download_bytes,
        0.0,
        Some("Zibo backup is ready; preparing clean install".to_string()),
        None,
    );

    let rename_started_at = Instant::now();
    let renamed_old_path = build_renamed_zibo_path(target_path)?;
    fs::rename(target_path, &renamed_old_path).with_context(|| {
        format!(
            "Failed to rename current Zibo folder '{}' to '{}'",
            target_path.display(),
            renamed_old_path.display()
        )
    })?;
    log_timed_step(
        "rename current install",
        rename_started_at,
        format!(
            "from='{}' to='{}'",
            target_path.display(),
            renamed_old_path.display()
        ),
    );

    emit_progress_event(
        progress_callback,
        item_type,
        folder_name,
        "install",
        "in_progress",
        ZIBO_MAJOR_RENAME_PROGRESS_MAX,
        0,
        total_download_bytes,
        0.0,
        Some("Current Zibo installation moved aside".to_string()),
        None,
    );

    let install_result: Result<(usize, String, Option<String>)> = async {
        let workdir = tempfile::tempdir().context("Failed to create Zibo update temp directory")?;
        let downloads_dir = workdir.path().join("downloads");
        let unpacked_dir = workdir.path().join("unpacked");
        fs::create_dir_all(&downloads_dir).context("Failed to create Zibo download directory")?;
        fs::create_dir_all(&unpacked_dir).context("Failed to create Zibo extraction directory")?;

        let releases = releases_for_plan(planned_update);
        let download_ranges = if planned_update.follow_up_patch.is_some() {
            vec![
                ProgressRange {
                    start: ZIBO_MAJOR_RENAME_PROGRESS_MAX,
                    end: ZIBO_MAJOR_FULL_DOWNLOAD_PROGRESS_MAX,
                },
                ProgressRange {
                    start: ZIBO_MAJOR_FULL_COPY_PROGRESS_MAX,
                    end: ZIBO_MAJOR_PATCH_DOWNLOAD_PROGRESS_MAX,
                },
            ]
        } else {
            vec![ProgressRange {
                start: ZIBO_MAJOR_RENAME_PROGRESS_MAX,
                end: 72.0,
            }]
        };

        let download_started_at = Instant::now();
        let archives = download_releases_from_drive_with_order(
            &releases,
            current_drive_source_order(),
            &downloads_dir,
            full_package_download_mode,
            task_control,
            progress_callback,
            item_type,
            folder_name,
            &download_ranges,
        )
        .await?;
        let source_label = archives
            .first()
            .map(|archive| archive.file.source.label.to_string())
            .unwrap_or_else(|| "unknown-source".to_string());
        let archive_bytes = archives
            .iter()
            .map(|archive| archive.file.total_bytes)
            .sum::<u64>();
        log_timed_step(
            "download planned archives",
            download_started_at,
            format!(
                "count={} bytes={} source={}",
                archives.len(),
                format_binary_size(archive_bytes),
                source_label
            ),
        );

        let full_extract_range = if planned_update.follow_up_patch.is_some() {
            ProgressRange {
                start: ZIBO_MAJOR_FULL_DOWNLOAD_PROGRESS_MAX,
                end: ZIBO_MAJOR_FULL_EXTRACT_PROGRESS_MAX,
            }
        } else {
            ProgressRange {
                start: 72.0,
                end: 84.0,
            }
        };
        let full_copy_range = if planned_update.follow_up_patch.is_some() {
            ProgressRange {
                start: ZIBO_MAJOR_FULL_EXTRACT_PROGRESS_MAX,
                end: ZIBO_MAJOR_FULL_COPY_PROGRESS_MAX,
            }
        } else {
            ProgressRange {
                start: 84.0,
                end: 96.0,
            }
        };

        let full_apply_started_at = Instant::now();
        let mut updated_files = apply_downloaded_archive_to_target(
            archives
                .first()
                .ok_or_else(|| anyhow!("Major clean install is missing the full Zibo package"))?,
            &unpacked_dir,
            target_path,
            full_extract_range,
            full_copy_range,
            total_download_bytes,
            task_control,
            progress_callback,
            item_type,
            folder_name,
        )?;
        if let Some(full_archive) = archives.first() {
            log_timed_step(
                "apply full package",
                full_apply_started_at,
                format!(
                    "archive={} updatedFiles={}",
                    full_archive.release.title, updated_files
                ),
            );
        }

        if let Some(patch_archive) = archives.get(1) {
            let patch_apply_started_at = Instant::now();
            updated_files += apply_downloaded_archive_to_target(
                patch_archive,
                &unpacked_dir,
                target_path,
                ProgressRange {
                    start: ZIBO_MAJOR_PATCH_DOWNLOAD_PROGRESS_MAX,
                    end: ZIBO_MAJOR_PATCH_EXTRACT_PROGRESS_MAX,
                },
                ProgressRange {
                    start: ZIBO_MAJOR_PATCH_EXTRACT_PROGRESS_MAX,
                    end: ZIBO_MAJOR_PATCH_COPY_PROGRESS_MAX,
                },
                total_download_bytes,
                task_control,
                progress_callback,
                item_type,
                folder_name,
            )?;
            log_timed_step(
                "apply follow-up patch",
                patch_apply_started_at,
                format!(
                    "archive={} cumulativeUpdatedFiles={}",
                    patch_archive.release.title, updated_files
                ),
            );
        }

        if let Some(backup_state) = backup.as_ref() {
            let restore_started_at = Instant::now();
            let mut last_restore_emit_at = Instant::now();
            restore_zibo_backup(
                backup_state,
                target_path,
                |processed_bytes, total_bytes, processed_files, total_files, current_file| {
                    ensure_not_cancelled(task_control, "install")?;
                    let force_emit = total_bytes == 0 || processed_bytes >= total_bytes;
                    if take_emit_interval_secs(&mut last_restore_emit_at, force_emit).is_some() {
                        emit_file_operation_progress(
                            progress_callback,
                            item_type,
                            folder_name,
                            ProgressRange {
                                start: ZIBO_MAJOR_PATCH_COPY_PROGRESS_MAX,
                                end: ZIBO_MAJOR_RESTORE_PROGRESS_MAX,
                            },
                            processed_bytes,
                            total_bytes,
                            &format!(
                                "Restoring preserved Zibo files ({}/{})",
                                processed_files, total_files
                            ),
                            current_file,
                        );
                    }
                    Ok(())
                },
            )?;
            verify_zibo_restore(backup_state, target_path)
                .context("Restored Zibo backup verification failed")?;
            let (backup_files, backup_bytes) = backup_totals(backup_state);
            log_timed_step(
                "restore preserved files",
                restore_started_at,
                format!(
                    "files={} bytes={}",
                    backup_files,
                    format_binary_size(backup_bytes)
                ),
            );
            let _ = fs::remove_dir_all(&backup_state.temp_dir);
        }

        emit_progress_event(
            progress_callback,
            item_type,
            folder_name,
            "install",
            "in_progress",
            ZIBO_MAJOR_RESTORE_PROGRESS_MAX,
            total_download_bytes,
            total_download_bytes,
            0.0,
            Some("Cleaning up previous Zibo installation".to_string()),
            None,
        );

        let cleanup_started_at = Instant::now();
        let cleanup_warning = match remove_dir_all_robust(&renamed_old_path) {
            Ok(()) => None,
            Err(err) => {
                log_info(format!(
                    "Zibo clean install succeeded but old folder cleanup failed: {}",
                    err
                ));
                Some(format!(
                    "Previous Zibo folder was kept at {}.",
                    renamed_old_path.display()
                ))
            }
        };
        log_timed_step(
            "cleanup previous install",
            cleanup_started_at,
            cleanup_warning.clone().unwrap_or_default(),
        );

        Ok((updated_files, source_label, cleanup_warning))
    }
    .await;

    match install_result {
        Ok(result) => {
            log_timed_step("major clean update total", major_started_at, String::new());
            Ok(result)
        }
        Err(err) => {
            if let Some(backup_state) = backup.as_ref() {
                log_info(format!(
                    "Preserved Zibo backup remains at '{}'",
                    backup_state.temp_dir.display()
                ));
            }

            let rollback_started_at = Instant::now();
            match restore_original_zibo_folder(target_path, &renamed_old_path) {
                Ok(()) => {
                    log_timed_step("rollback original install", rollback_started_at, String::new());
                    Err(anyhow!("{} Original Zibo installation was restored.", err))
                }
                Err(restore_err) => Err(anyhow!(
                    "{} Original Zibo folder remains at '{}' and could not be restored automatically: {}",
                    err,
                    renamed_old_path.display(),
                    restore_err
                )),
            }
        }
    }
}

fn apply_downloaded_archive_to_target(
    archive: &DownloadedDriveArchive,
    unpack_root: &Path,
    target_path: &Path,
    extract_range: ProgressRange,
    copy_range: ProgressRange,
    total_download_bytes: u64,
    task_control: Option<&TaskControl>,
    progress_callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
) -> Result<usize> {
    ensure_not_cancelled(task_control, "install")?;
    let apply_started_at = Instant::now();
    let archive_extract_dir = unpack_root.join(format!("extract_{}", Uuid::new_v4().simple()));
    fs::create_dir_all(&archive_extract_dir)
        .with_context(|| format!("Failed to create '{}'", archive_extract_dir.display()))?;
    let archive_name = archive
        .zip_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("zibo.zip")
        .to_string();

    let extract_started_at = Instant::now();
    let mut last_extract_emit_at = Instant::now();
    unzip_archive_with_progress(
        &archive.zip_path,
        &archive_extract_dir,
        |processed_bytes, total_bytes, processed_entries, total_entries, current_file| {
            ensure_not_cancelled(task_control, "install")?;
            let force_emit = total_bytes == 0 || processed_bytes >= total_bytes;
            if take_emit_interval_secs(&mut last_extract_emit_at, force_emit).is_some() {
                emit_file_operation_progress(
                    progress_callback,
                    item_type,
                    folder_name,
                    extract_range,
                    processed_bytes,
                    total_bytes,
                    &format!(
                        "Extracting Zibo archive ({}/{})",
                        processed_entries, total_entries
                    ),
                    current_file.or(Some(Path::new(&archive_name))),
                );
            }
            Ok(())
        },
    )?;
    let extracted_info = directory_info(&archive_extract_dir).unwrap_or(DirectoryInfo {
        file_count: 0,
        total_size: 0,
    });
    log_timed_step(
        &format!("extract archive {}", archive.release.title),
        extract_started_at,
        format!(
            "entries={} bytes={}",
            extracted_info.file_count,
            format_binary_size(extracted_info.total_size)
        ),
    );

    let extracted_root = find_extracted_zibo_root(&archive_extract_dir)?;
    let copy_started_at = Instant::now();
    let mut last_copy_emit_at = Instant::now();
    let updated_files = copy_dir_contents_overwrite_with_progress(
        &extracted_root,
        target_path,
        |processed_bytes, total_bytes, copied, total, current_file| {
            ensure_not_cancelled(task_control, "install")?;
            let force_emit = total_bytes == 0 || processed_bytes >= total_bytes;
            if take_emit_interval_secs(&mut last_copy_emit_at, force_emit).is_some() {
                emit_file_operation_progress(
                    progress_callback,
                    item_type,
                    folder_name,
                    copy_range,
                    processed_bytes,
                    total_bytes,
                    &format!("Applying Zibo files ({}/{})", copied, total),
                    current_file.or(Some(Path::new(&archive.release.title))),
                );
            }
            Ok(())
        },
    )?;
    let copied_info = directory_info(&extracted_root).unwrap_or(DirectoryInfo {
        file_count: updated_files as u64,
        total_size: 0,
    });
    log_timed_step(
        &format!("apply archive {}", archive.release.title),
        copy_started_at,
        format!(
            "files={} bytes={}",
            updated_files,
            format_binary_size(copied_info.total_size)
        ),
    );
    log_timed_step(
        &format!("archive workflow {}", archive.release.title),
        apply_started_at,
        String::new(),
    );
    let _ = total_download_bytes;
    Ok(updated_files)
}

fn prepare_zibo_backup<F>(
    target_path: &Path,
    preserve_liveries: bool,
    preserve_config_files: bool,
    mut on_progress: F,
) -> Result<Option<ZiboBackupState>>
where
    F: FnMut(u64, u64, u64, u64, Option<&Path>) -> Result<()>,
{
    if !target_path.exists() || (!preserve_liveries && !preserve_config_files) {
        return Ok(None);
    }

    let mut total_files = 0u64;
    let mut total_bytes = 0u64;
    let mut config_candidates: Vec<(String, PathBuf, u64)> = Vec::new();

    let liveries_src = target_path.join("liveries");
    let liveries_info = if preserve_liveries && liveries_src.exists() && liveries_src.is_dir() {
        let info = directory_info(&liveries_src)?;
        total_files = total_files.saturating_add(info.file_count);
        total_bytes = total_bytes.saturating_add(info.total_size);
        Some(info)
    } else {
        None
    };

    if preserve_config_files {
        let patterns = compile_zibo_config_patterns()?;
        for entry in fs::read_dir(target_path)
            .with_context(|| format!("Failed to read '{}'", target_path.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !patterns.iter().any(|pattern| pattern.matches(name)) {
                continue;
            }

            let size = fs::metadata(&path)?.len();
            total_files = total_files.saturating_add(1);
            total_bytes = total_bytes.saturating_add(size);
            config_candidates.push((name.to_string(), path, size));
        }
    }

    if total_files == 0 {
        return Ok(None);
    }

    let temp_dir =
        std::env::temp_dir().join(format!("xfastmanager_zibo_backup_{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).with_context(|| {
        format!(
            "Failed to create Zibo backup directory '{}'",
            temp_dir.display()
        )
    })?;

    let mut backup = ZiboBackupState {
        temp_dir: temp_dir.clone(),
        liveries_path: None,
        pref_files: Vec::new(),
        original_liveries_info: None,
        original_pref_sizes: Vec::new(),
    };
    let mut processed_bytes = 0u64;
    let mut processed_files = 0u64;

    if let Some(info) = liveries_info {
        backup.original_liveries_info = Some(info);
        let liveries_dst = temp_dir.join("liveries");
        copy_dir_contents_overwrite_with_progress(
            &liveries_src,
            &liveries_dst,
            |copied_bytes, _, copied_files, _, current_file| {
                let current_path = current_file.map(|path| PathBuf::from("liveries").join(path));
                on_progress(
                    processed_bytes.saturating_add(copied_bytes),
                    total_bytes,
                    processed_files.saturating_add(copied_files),
                    total_files,
                    current_path.as_deref(),
                )
            },
        )
        .context("Failed to backup Zibo liveries")?;
        processed_bytes = processed_bytes.saturating_add(info.total_size);
        processed_files = processed_files.saturating_add(info.file_count);
        backup.liveries_path = Some(liveries_dst);
    }

    for (name, path, original_size) in config_candidates {
        let backup_path = temp_dir.join(&name);
        copy_file_with_progress(
            &path,
            &backup_path,
            &mut processed_bytes,
            total_bytes,
            &mut processed_files,
            total_files,
            Some(Path::new(&name)),
            &mut on_progress,
        )
        .with_context(|| format!("Failed to backup '{}'", path.display()))?;
        backup.pref_files.push((name.clone(), backup_path));
        backup.original_pref_sizes.push((name, original_size));
    }

    Ok(Some(backup))
}

fn compile_zibo_config_patterns() -> Result<Vec<Pattern>> {
    ZIBO_CONFIG_PATTERNS
        .iter()
        .map(|pattern| {
            Pattern::new(pattern)
                .map_err(|err| anyhow!("Invalid config pattern '{}': {}", pattern, err))
        })
        .collect()
}

fn backup_totals(backup: &ZiboBackupState) -> (u64, u64) {
    let liveries_files = backup
        .original_liveries_info
        .map(|info| info.file_count)
        .unwrap_or(0);
    let liveries_bytes = backup
        .original_liveries_info
        .map(|info| info.total_size)
        .unwrap_or(0);
    let pref_files = backup.original_pref_sizes.len() as u64;
    let pref_bytes = backup
        .original_pref_sizes
        .iter()
        .map(|(_, size)| *size)
        .sum::<u64>();
    (
        liveries_files.saturating_add(pref_files),
        liveries_bytes.saturating_add(pref_bytes),
    )
}

fn verify_zibo_backup(backup: &ZiboBackupState) -> Result<()> {
    if let (Some(liveries_backup_path), Some(original_info)) =
        (&backup.liveries_path, &backup.original_liveries_info)
    {
        if !liveries_backup_path.exists() {
            return Err(anyhow!("Zibo liveries backup folder does not exist"));
        }

        let backup_info = directory_info(liveries_backup_path)?;
        if backup_info.file_count != original_info.file_count {
            return Err(anyhow!(
                "Zibo liveries backup is incomplete: expected {} files, got {}",
                original_info.file_count,
                backup_info.file_count
            ));
        }
        if backup_info.total_size != original_info.total_size {
            return Err(anyhow!(
                "Zibo liveries backup size mismatch: expected {} bytes, got {}",
                original_info.total_size,
                backup_info.total_size
            ));
        }
    }

    for (filename, original_size) in &backup.original_pref_sizes {
        let backup_path = backup.temp_dir.join(filename);
        if !backup_path.exists() {
            return Err(anyhow!("Backup of '{}' does not exist", filename));
        }
        let backup_size = fs::metadata(&backup_path)?.len();
        if backup_size != *original_size {
            return Err(anyhow!(
                "Backup size mismatch for '{}': expected {} bytes, got {}",
                filename,
                original_size,
                backup_size
            ));
        }
    }

    Ok(())
}

fn restore_zibo_backup<F>(
    backup: &ZiboBackupState,
    target_path: &Path,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(u64, u64, u64, u64, Option<&Path>) -> Result<()>,
{
    let liveries_info = if let Some(liveries_backup) = backup.liveries_path.as_ref() {
        Some(directory_info(liveries_backup)?)
    } else {
        None
    };
    let pref_total_bytes = backup
        .original_pref_sizes
        .iter()
        .map(|(_, size)| *size)
        .sum::<u64>();
    let total_bytes = liveries_info
        .map(|info| info.total_size)
        .unwrap_or(0)
        .saturating_add(pref_total_bytes);
    let total_files = liveries_info
        .map(|info| info.file_count)
        .unwrap_or(0)
        .saturating_add(backup.pref_files.len() as u64);
    let mut processed_bytes = 0u64;
    let mut processed_files = 0u64;

    if let Some(liveries_backup) = backup.liveries_path.as_ref() {
        let liveries_target = target_path.join("liveries");
        if liveries_target.exists() {
            merge_directories_skip_existing_with_progress(
                liveries_backup,
                liveries_backup,
                &liveries_target,
                &mut processed_bytes,
                total_bytes,
                &mut processed_files,
                total_files,
                &mut |current_bytes, _, current_files, _, current_file| {
                    let current_path =
                        current_file.map(|path| PathBuf::from("liveries").join(path));
                    on_progress(
                        current_bytes,
                        total_bytes,
                        current_files,
                        total_files,
                        current_path.as_deref(),
                    )
                },
            )?;
        } else {
            copy_dir_contents_overwrite_with_progress(
                liveries_backup,
                &liveries_target,
                |current_bytes, _, current_files, _, current_file| {
                    let current_path =
                        current_file.map(|path| PathBuf::from("liveries").join(path));
                    on_progress(
                        current_bytes,
                        total_bytes,
                        current_files,
                        total_files,
                        current_path.as_deref(),
                    )
                },
            )?;
            if let Some(info) = liveries_info {
                processed_bytes = processed_bytes.saturating_add(info.total_size);
                processed_files = processed_files.saturating_add(info.file_count);
            }
        }
    }

    for (filename, backup_path) in &backup.pref_files {
        let target_file = target_path.join(filename);
        copy_file_with_progress(
            backup_path,
            &target_file,
            &mut processed_bytes,
            total_bytes,
            &mut processed_files,
            total_files,
            Some(Path::new(filename)),
            &mut on_progress,
        )
        .with_context(|| {
            format!(
                "Failed to restore '{}' from '{}'",
                target_file.display(),
                backup_path.display()
            )
        })?;
    }

    Ok(())
}

fn verify_zibo_restore(backup: &ZiboBackupState, target_path: &Path) -> Result<()> {
    for (filename, original_size) in &backup.original_pref_sizes {
        let restored_path = target_path.join(filename);
        if !restored_path.exists() {
            return Err(anyhow!(
                "Restored config file '{}' does not exist",
                filename
            ));
        }

        let restored_size = fs::metadata(&restored_path)?.len();
        if restored_size != *original_size {
            return Err(anyhow!(
                "Restored config file '{}' has the wrong size: expected {} bytes, got {}",
                filename,
                original_size,
                restored_size
            ));
        }
    }

    if backup.liveries_path.is_some() {
        let liveries_target = target_path.join("liveries");
        if !liveries_target.exists() {
            return Err(anyhow!("Restored Zibo liveries folder does not exist"));
        }
    }

    Ok(())
}

fn restore_original_zibo_folder(target_path: &Path, renamed_old_path: &Path) -> Result<()> {
    if !renamed_old_path.exists() {
        return Ok(());
    }

    if target_path.exists() {
        remove_dir_all_robust(target_path)?;
    }

    fs::rename(renamed_old_path, target_path).with_context(|| {
        format!(
            "Failed to restore previous Zibo folder from '{}' to '{}'",
            renamed_old_path.display(),
            target_path.display()
        )
    })
}

fn build_renamed_zibo_path(target_path: &Path) -> Result<PathBuf> {
    let parent = target_path
        .parent()
        .ok_or_else(|| anyhow!("Target Zibo path does not have a parent directory"))?;
    let folder_name = target_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Target Zibo path does not have a valid folder name"))?;

    for _ in 0..8 {
        let candidate = parent.join(format!(
            "{}.xfastmanager-old-{}",
            folder_name,
            Uuid::new_v4().simple()
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "Failed to allocate a temporary renamed Zibo folder beside '{}'",
        target_path.display()
    ))
}

fn directory_info(dir: &Path) -> Result<DirectoryInfo> {
    let mut file_count = 0u64;
    let mut total_size = 0u64;

    for entry in walkdir::WalkDir::new(dir).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() {
            file_count = file_count.saturating_add(1);
            total_size = total_size.saturating_add(entry.metadata()?.len());
        }
    }

    Ok(DirectoryInfo {
        file_count,
        total_size,
    })
}

fn copy_reader_to_file_with_progress<R, F>(
    reader: &mut R,
    destination: &Path,
    file_size: u64,
    processed_bytes: &mut u64,
    total_bytes: u64,
    processed_files: &mut u64,
    total_files: u64,
    current_file: Option<&Path>,
    on_progress: &mut F,
) -> Result<()>
where
    R: Read,
    F: FnMut(u64, u64, u64, u64, Option<&Path>) -> Result<()>,
{
    let mut output = fs::File::create(destination)
        .with_context(|| format!("Failed to create '{}'", destination.display()))?;
    let mut buffer = vec![0u8; ZIBO_FILE_IO_BUFFER_BYTES];
    let mut remaining = file_size;

    while remaining > 0 {
        let read_limit = remaining.min(buffer.len() as u64) as usize;
        let read = reader.read(&mut buffer[..read_limit]).with_context(|| {
            format!("Failed to read source data for '{}'", destination.display())
        })?;
        if read == 0 {
            return Err(anyhow!(
                "Unexpected end of stream while writing '{}'",
                destination.display()
            ));
        }
        output
            .write_all(&buffer[..read])
            .with_context(|| format!("Failed to write '{}'", destination.display()))?;
        *processed_bytes = processed_bytes.saturating_add(read as u64);
        remaining = remaining.saturating_sub(read as u64);
        on_progress(
            *processed_bytes,
            total_bytes,
            *processed_files,
            total_files,
            current_file,
        )?;
    }

    output.flush().ok();
    *processed_files = processed_files.saturating_add(1);
    on_progress(
        *processed_bytes,
        total_bytes,
        *processed_files,
        total_files,
        current_file,
    )?;
    Ok(())
}

fn copy_file_with_progress<F>(
    source: &Path,
    destination: &Path,
    processed_bytes: &mut u64,
    total_bytes: u64,
    processed_files: &mut u64,
    total_files: u64,
    current_file: Option<&Path>,
    on_progress: &mut F,
) -> Result<()>
where
    F: FnMut(u64, u64, u64, u64, Option<&Path>) -> Result<()>,
{
    let file_size = fs::metadata(source)
        .with_context(|| format!("Failed to read metadata for '{}'", source.display()))?
        .len();
    if file_size == 0 {
        let _ = fs::File::create(destination)
            .with_context(|| format!("Failed to create '{}'", destination.display()))?;
        *processed_files = processed_files.saturating_add(1);
        on_progress(
            *processed_bytes,
            total_bytes,
            *processed_files,
            total_files,
            current_file,
        )?;
        return Ok(());
    }

    let mut input =
        fs::File::open(source).with_context(|| format!("Failed to open '{}'", source.display()))?;
    copy_reader_to_file_with_progress(
        &mut input,
        destination,
        file_size,
        processed_bytes,
        total_bytes,
        processed_files,
        total_files,
        current_file,
        on_progress,
    )
}

fn merge_directories_skip_existing_with_progress<F>(
    root_source_dir: &Path,
    source_dir: &Path,
    target_dir: &Path,
    processed_bytes: &mut u64,
    total_bytes: u64,
    processed_files: &mut u64,
    total_files: u64,
    on_progress: &mut F,
) -> Result<()>
where
    F: FnMut(u64, u64, u64, u64, Option<&Path>) -> Result<()>,
{
    if !target_dir.exists() {
        fs::create_dir_all(target_dir)
            .with_context(|| format!("Failed to create '{}'", target_dir.display()))?;
    }

    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("Failed to read '{}'", source_dir.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target_dir.join(entry.file_name());

        if source_path.is_dir() {
            merge_directories_skip_existing_with_progress(
                root_source_dir,
                &source_path,
                &target_path,
                processed_bytes,
                total_bytes,
                processed_files,
                total_files,
                on_progress,
            )?;
            continue;
        }

        let file_size = fs::metadata(&source_path)
            .with_context(|| format!("Failed to read metadata for '{}'", source_path.display()))?
            .len();
        if target_path.exists() {
            *processed_bytes = processed_bytes.saturating_add(file_size);
            *processed_files = processed_files.saturating_add(1);
            on_progress(
                *processed_bytes,
                total_bytes,
                *processed_files,
                total_files,
                source_path.strip_prefix(root_source_dir).ok(),
            )?;
            continue;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create '{}'", parent.display()))?;
        }

        match fs::rename(&source_path, &target_path) {
            Ok(()) => {
                *processed_bytes = processed_bytes.saturating_add(file_size);
                *processed_files = processed_files.saturating_add(1);
                on_progress(
                    *processed_bytes,
                    total_bytes,
                    *processed_files,
                    total_files,
                    source_path.strip_prefix(root_source_dir).ok(),
                )?;
            }
            Err(_) => {
                copy_file_with_progress(
                    &source_path,
                    &target_path,
                    processed_bytes,
                    total_bytes,
                    processed_files,
                    total_files,
                    source_path.strip_prefix(root_source_dir).ok(),
                    on_progress,
                )
                .with_context(|| {
                    format!(
                        "Failed to copy '{}' to '{}'",
                        source_path.display(),
                        target_path.display()
                    )
                })?;
            }
        }
    }

    Ok(())
}

fn remove_readonly_attribute(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("Failed to update permissions for '{}'", path.display()))?;
    }
    Ok(())
}

fn remove_dir_all_robust(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if entry.path().is_file() {
            let _ = remove_readonly_attribute(entry.path());
        }
    }

    const MAX_RETRIES: u32 = 3;
    const INITIAL_DELAY_MS: u64 = 100;

    let mut last_error = None;
    for attempt in 0..=MAX_RETRIES {
        match fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_error = Some(err);
                if attempt < MAX_RETRIES {
                    std::thread::sleep(Duration::from_millis(INITIAL_DELAY_MS * (1 << attempt)));
                }
            }
        }
    }

    Err(anyhow!(
        "Failed to delete directory '{}': {}",
        path.display(),
        last_error.unwrap_or_else(|| std::io::Error::other("unknown directory removal error"))
    ))
}

fn resolve_zibo_target_path(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
) -> Result<PathBuf> {
    if item_type != "aircraft" {
        return Err(anyhow!("Zibo updater only supports aircraft targets"));
    }
    if folder_name.trim().is_empty() {
        return Err(anyhow!("Folder name cannot be empty"));
    }
    if folder_name.contains("..") {
        return Err(anyhow!("Folder name contains invalid traversal segment"));
    }

    let base_path = xplane_path.join("Aircraft");
    let target_path = base_path.join(folder_name.replace('\\', "/"));
    if !target_path.exists() {
        return Err(anyhow!(
            "Target path does not exist: {}",
            target_path.display()
        ));
    }

    crate::path_utils::validate_child_path(&base_path, &target_path)
        .map_err(|err| anyhow!("Invalid target path: {}", err))
}

fn unzip_archive_with_progress<F>(
    archive_path: &Path,
    output_dir: &Path,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(u64, u64, u64, u64, Option<&Path>) -> Result<()>,
{
    let file = fs::File::open(archive_path)
        .with_context(|| format!("Failed to open '{}'", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP '{}'", archive_path.display()))?;
    let total_entries = archive.len() as u64;
    let mut total_bytes = 0u64;
    for idx in 0..archive.len() {
        let entry = archive.by_index(idx).with_context(|| {
            format!(
                "Failed to inspect ZIP entry {} from '{}'",
                idx,
                archive_path.display()
            )
        })?;
        if entry.is_file() {
            total_bytes = total_bytes.saturating_add(entry.size());
        }
    }
    let mut processed_entries = 0u64;
    let mut processed_bytes = 0u64;

    for idx in 0..archive.len() {
        let mut entry = archive.by_index(idx).with_context(|| {
            format!(
                "Failed to read ZIP entry {} from '{}'",
                idx,
                archive_path.display()
            )
        })?;
        let Some(enclosed) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            processed_entries = processed_entries.saturating_add(1);
            on_progress(
                processed_bytes,
                total_bytes,
                processed_entries,
                total_entries,
                None,
            )?;
            continue;
        };
        let destination = output_dir.join(enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&destination)
                .with_context(|| format!("Failed to create '{}'", destination.display()))?;
            processed_entries = processed_entries.saturating_add(1);
            on_progress(
                processed_bytes,
                total_bytes,
                processed_entries,
                total_entries,
                entry.enclosed_name().as_deref(),
            )?;
            continue;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create '{}'", parent.display()))?;
        }

        let entry_name = entry.enclosed_name().map(|path| path.to_path_buf());
        let entry_size = entry.size();
        copy_reader_to_file_with_progress(
            &mut entry,
            &destination,
            entry_size,
            &mut processed_bytes,
            total_bytes,
            &mut processed_entries,
            total_entries,
            entry_name.as_deref(),
            &mut on_progress,
        )
        .with_context(|| format!("Failed to extract '{}'", destination.display()))?;
    }

    Ok(())
}

fn find_extracted_zibo_root(output_dir: &Path) -> Result<PathBuf> {
    let mut matches = Vec::new();

    for entry in walkdir::WalkDir::new(output_dir)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str() else {
            continue;
        };
        if !matches!(
            name.to_ascii_lowercase().as_str(),
            "b738.acf" | "b738_4k.acf"
        ) {
            continue;
        }
        let Some(parent) = entry.path().parent() else {
            continue;
        };
        matches.push(parent.to_path_buf());
    }

    matches.sort();
    matches.dedup();

    match matches.len() {
        0 => Err(anyhow!(
            "Extracted Zibo archive does not contain b738.acf or b738_4k.acf"
        )),
        1 => Ok(matches.remove(0)),
        _ => Err(anyhow!(
            "Extracted Zibo archive contains multiple possible aircraft roots"
        )),
    }
}

fn copy_dir_contents_overwrite_with_progress<F>(
    source_dir: &Path,
    target_dir: &Path,
    mut on_progress: F,
) -> Result<usize>
where
    F: FnMut(u64, u64, u64, u64, Option<&Path>) -> Result<()>,
{
    let info = directory_info(source_dir)?;
    let total_files = info.file_count;
    let total_bytes = info.total_size;
    let mut copied_files = 0usize;
    let mut processed_bytes = 0u64;
    let mut processed_files = 0u64;

    for entry in walkdir::WalkDir::new(source_dir)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(source_dir)
            .with_context(|| format!("Failed to strip prefix for '{}'", path.display()))?;
        if relative.as_os_str().is_empty() {
            continue;
        }

        let destination = target_dir.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination)
                .with_context(|| format!("Failed to create '{}'", destination.display()))?;
            continue;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create '{}'", parent.display()))?;
        }

        copy_file_with_progress(
            path,
            &destination,
            &mut processed_bytes,
            total_bytes,
            &mut processed_files,
            total_files,
            Some(relative),
            &mut on_progress,
        )
        .with_context(|| {
            format!(
                "Failed to copy '{}' to '{}'",
                path.display(),
                destination.display()
            )
        })?;
        copied_files += 1;
    }

    Ok(copied_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zibo_detection_excludes_default_folders() {
        assert!(is_zibo_aircraft("B737-800X", "b738.acf"));
        assert!(is_zibo_aircraft("B737-800X", "b738_4k.acf"));
        assert!(!is_zibo_aircraft("Boeing 737-800", "b738.acf"));
        assert!(!is_zibo_aircraft("Boeing B737-800", "b738_4k.acf"));
        assert!(!is_zibo_aircraft("B737-800X", "a320.acf"));
    }

    #[test]
    fn local_version_parser_is_strict() {
        assert_eq!(
            parse_local_version("4.05.31").map(VersionTriple::display_string),
            Some("4.05.31".to_string())
        );
        assert!(parse_local_version("40531").is_none());
        assert!(parse_local_version("v4.05.31").is_none());
    }

    #[test]
    fn rss_parser_picks_latest_numeric_release() {
        let xml = r#"
        <rss>
          <channel>
            <item>
              <title>B737-800X_XP12_4_05_full.zip</title>
              <link>https://example.com/full.torrent</link>
              <description>Full</description>
            </item>
            <item>
              <title>B738X_XP12_4_05_02.zip</title>
              <link>https://example.com/02.torrent</link>
              <description>Fix</description>
            </item>
            <item>
              <title>B738X_XP12_4_05_31.zip</title>
              <link>https://example.com/31.torrent</link>
              <description>Fix</description>
            </item>
          </channel>
        </rss>
        "#;

        let releases = parse_release_catalog(xml).unwrap();
        let release = latest_release_from_catalog(&releases).unwrap();
        assert_eq!(release.version_string(), "4.05.31");
        assert_eq!(release.title, "B738X_XP12_4_05_31.zip");
    }

    #[test]
    fn plan_zibo_update_uses_clean_install_for_major_mismatch() {
        let local = LocalVersionState::Parsed(VersionTriple::new(4, 4, 18));
        let releases = vec![
            ZiboRelease {
                version: VersionTriple::new(4, 5, 0),
                title: "B737-800X_XP12_4_05_full.zip".to_string(),
            },
            ZiboRelease {
                version: VersionTriple::new(4, 5, 31),
                title: "B738X_XP12_4_05_31.zip".to_string(),
            },
        ];

        let planned = plan_zibo_update(&local, &releases).unwrap().unwrap();
        assert!(matches!(planned.install_mode, ZiboInstallMode::MajorClean));
        assert_eq!(planned.primary_release.version_string(), "4.05.00");
        assert_eq!(
            planned
                .follow_up_patch
                .as_ref()
                .map(ZiboRelease::version_string),
            Some("4.05.31".to_string())
        );
    }

    #[test]
    fn drive_source_order_flips_with_seed() {
        let even_order = drive_source_order_for_seed(0);
        let odd_order = drive_source_order_for_seed(1);

        assert_eq!(even_order[0].folder_id, ZIBO_DRIVE_SOURCES[0].folder_id);
        assert_eq!(odd_order[0].folder_id, ZIBO_DRIVE_SOURCES[1].folder_id);
    }

    #[test]
    fn drive_query_escapes_exact_archive_name() {
        let query = build_drive_search_query("folder'1", "B738X_'special'.zip");
        assert_eq!(
            query,
            "'folder\\'1' in parents and name = 'B738X_\\'special\\'.zip' and trashed = false"
        );
    }

    #[test]
    fn full_package_download_mode_follows_settings() {
        let disabled = AddonUpdateOptions {
            use_beta: false,
            include_liveries: false,
            apply_blacklist: false,
            rollback_on_failure: false,
            parallel_downloads: None,
            channel: None,
            fresh_install: false,
            preserve_liveries: false,
            preserve_config_files: false,
            chunked_download_enabled: Some(false),
            threads_per_task: Some(8),
            total_threads: None,
        };
        assert!(matches!(
            zibo_full_package_download_mode(&disabled),
            DriveDownloadMode::Single
        ));

        let single_thread = AddonUpdateOptions {
            chunked_download_enabled: Some(true),
            threads_per_task: Some(1),
            ..disabled.clone()
        };
        assert!(matches!(
            zibo_full_package_download_mode(&single_thread),
            DriveDownloadMode::Single
        ));

        let multi_thread = AddonUpdateOptions {
            chunked_download_enabled: Some(true),
            threads_per_task: Some(12),
            ..disabled.clone()
        };
        assert!(matches!(
            zibo_full_package_download_mode(&multi_thread),
            DriveDownloadMode::Chunked(ChunkedDriveDownloadConfig {
                active_connections: 12,
                window_bytes: ZIBO_CHUNK_REQUEST_WINDOW_BYTES,
            })
        ));

        let capped_by_total_threads = AddonUpdateOptions {
            total_threads: Some(4),
            ..multi_thread
        };
        assert!(matches!(
            zibo_full_package_download_mode(&capped_by_total_threads),
            DriveDownloadMode::Chunked(ChunkedDriveDownloadConfig {
                active_connections: 4,
                window_bytes: ZIBO_CHUNK_REQUEST_WINDOW_BYTES,
            })
        ));
    }

    #[test]
    fn drive_confirm_form_url_is_parsed() {
        let response_url =
            reqwest::Url::parse("https://drive.google.com/uc?export=download&id=file-123")
                .expect("valid response url");
        let html = r#"
            <html>
              <body>
                <form id="download-form" action="https://drive.usercontent.google.com/download">
                  <input type="hidden" name="id" value="file-123">
                  <input type="hidden" name="export" value="download">
                  <input type="hidden" name="confirm" value="t">
                  <input type="hidden" name="uuid" value="abc123">
                </form>
              </body>
            </html>
        "#;

        let confirm_url =
            extract_drive_confirm_download_url(&response_url, html).expect("confirm url");

        assert_eq!(
            confirm_url.as_str(),
            "https://drive.usercontent.google.com/download?id=file-123&export=download&confirm=t&uuid=abc123"
        );
    }

    #[test]
    fn chunk_requests_are_windowed_and_budgeted() {
        assert_eq!(chunk_request_end(0, 1024), 1024);
        assert_eq!(
            chunk_request_end(0, ZIBO_CHUNK_REQUEST_WINDOW_BYTES * 3),
            ZIBO_CHUNK_REQUEST_WINDOW_BYTES - 1
        );

        let budget = chunk_request_budget(0, ZIBO_CHUNK_REQUEST_WINDOW_BYTES * 3);
        assert!(budget >= 17);
    }
}

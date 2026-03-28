use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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
const ZIBO_DOWNLOAD_PROGRESS_MAX: f64 = 90.0;

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
struct DownloadMetadata {
    total_bytes: u64,
}

#[derive(Debug, Clone)]
struct ResolvedDriveFile {
    source: DriveSource,
    file_id: String,
    file_name: String,
    total_bytes: u64,
    web_view_link: Option<String>,
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

#[derive(Debug, Clone)]
struct ZiboPlanContext {
    local_state: LocalVersionState,
    latest_release: ZiboRelease,
    download_metadata: Option<DownloadMetadata>,
    drive_file: Option<ResolvedDriveFile>,
    preferred_source_url: String,
    manual_download_url: Option<String>,
    warnings: Vec<String>,
    has_update: bool,
}

fn log_info(message: impl Into<String>) {
    logger::log_info(&message.into(), Some(LOG_CTX));
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
    parse_latest_release(&xml)
}

fn parse_latest_release(xml: &str) -> Result<ZiboRelease> {
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

    releases
        .into_iter()
        .max_by_key(|release| release.version)
        .ok_or_else(|| anyhow!("No usable Zibo release was found in the RSS feed"))
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

async fn resolve_drive_file_metadata_with_order(
    release: &ZiboRelease,
    source_order: [DriveSource; 2],
    task_control: Option<&TaskControl>,
) -> Result<ResolvedDriveFile> {
    let api_key = require_drive_api_key()?;
    let client = build_http_client(20)?;
    let mut failures = Vec::new();

    log_info(format!(
        "Resolving Zibo archive '{}' using Google Drive sources {} -> {}",
        release.title, source_order[0].label, source_order[1].label
    ));

    for source in source_order {
        match search_drive_file_in_source(&client, &api_key, source, &release.title, task_control)
            .await
        {
            Ok(Some(file)) => return Ok(file),
            Ok(None) => {
                failures.push(format!(
                    "{}: '{}' not found in {}",
                    source.label, release.title, source.folder_url
                ));
            }
            Err(err) => failures.push(format!("{}: {}", source.label, err)),
        }
    }

    Err(anyhow!(
        "Failed to locate Zibo archive '{}' in configured Google Drive sources: {}",
        release.title,
        failures.join(" | ")
    ))
}

async fn download_drive_file_to_path(
    client: &reqwest::Client,
    api_key: &str,
    file: &ResolvedDriveFile,
    output_path: &Path,
    task_control: Option<&TaskControl>,
    progress_callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
) -> Result<()> {
    ensure_not_cancelled(task_control, "install")?;

    let response = client
        .get(format!("{}/files/{}", GOOGLE_DRIVE_API_BASE, file.file_id))
        .query(&[
            ("alt", "media"),
            ("supportsAllDrives", "true"),
            ("key", api_key),
        ])
        .send()
        .await
        .with_context(|| {
            format!(
                "Failed to start Google Drive download for '{}' from {}",
                file.file_name, file.source.label
            )
        })?
        .error_for_status()
        .with_context(|| {
            format!(
                "Google Drive download returned an error for '{}' from {}",
                file.file_name, file.source.label
            )
        })?;

    let mut stream = response.bytes_stream();
    let mut output = fs::File::create(output_path)
        .with_context(|| format!("Failed to create '{}'", output_path.display()))?;
    let mut processed_bytes = 0u64;
    let mut last_sample_bytes = 0u64;
    let mut last_sample_at = Instant::now();

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

        let elapsed = last_sample_at.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 && processed_bytes >= last_sample_bytes {
            (processed_bytes - last_sample_bytes) as f64 / elapsed
        } else {
            0.0
        };

        emit_progress_event(
            progress_callback,
            item_type,
            folder_name,
            "install",
            "running",
            if file.total_bytes > 0 {
                (processed_bytes.min(file.total_bytes) as f64 / file.total_bytes as f64)
                    * ZIBO_DOWNLOAD_PROGRESS_MAX
            } else {
                0.0
            },
            processed_bytes.min(file.total_bytes),
            file.total_bytes,
            speed,
            Some(format!(
                "Downloading {} from {}",
                file.file_name, file.source.label
            )),
            Some(file.file_name.clone()),
        );

        last_sample_bytes = processed_bytes;
        last_sample_at = Instant::now();
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

    Ok(())
}

async fn download_release_from_drive_with_order(
    release: &ZiboRelease,
    source_order: [DriveSource; 2],
    download_root: &Path,
    task_control: Option<&TaskControl>,
    progress_callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
) -> Result<(ResolvedDriveFile, PathBuf)> {
    let api_key = require_drive_api_key()?;
    let client = build_http_client(180)?;
    let mut failures = Vec::new();

    log_info(format!(
        "Downloading Zibo archive '{}' using Google Drive sources {} -> {}",
        release.title, source_order[0].label, source_order[1].label
    ));

    for source in source_order {
        ensure_not_cancelled(task_control, "install")?;
        emit_progress_event(
            progress_callback,
            item_type,
            folder_name,
            "install",
            "running",
            0.0,
            0,
            0,
            0.0,
            Some(format!("Searching Zibo archive in {}", source.label)),
            None,
        );

        let file = match search_drive_file_in_source(
            &client,
            &api_key,
            source,
            &release.title,
            task_control,
        )
        .await
        {
            Ok(Some(file)) => file,
            Ok(None) => {
                failures.push(format!(
                    "{}: '{}' not found in {}",
                    source.label, release.title, source.folder_url
                ));
                continue;
            }
            Err(err) => {
                failures.push(format!("{}: {}", source.label, err));
                continue;
            }
        };

        let zip_path = download_root.join(&file.file_name);
        if zip_path.exists() {
            fs::remove_file(&zip_path)
                .with_context(|| format!("Failed to reset '{}'", zip_path.display()))?;
        }

        match download_drive_file_to_path(
            &client,
            &api_key,
            &file,
            &zip_path,
            task_control,
            progress_callback,
            item_type,
            folder_name,
        )
        .await
        {
            Ok(()) => return Ok((file, zip_path)),
            Err(err) => {
                let _ = fs::remove_file(&zip_path);
                failures.push(format!("{}: {}", source.label, err));
            }
        }
    }

    Err(anyhow!(
        "Failed to download Zibo archive '{}' from configured Google Drive sources: {}",
        release.title,
        failures.join(" | ")
    ))
}

fn build_plan_context(
    local_state: &LocalVersionState,
    latest_release: ZiboRelease,
    preferred_manual_download_url: String,
) -> ZiboPlanContext {
    match local_state {
        LocalVersionState::Parsed(local) if latest_release.version <= *local => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            download_metadata: None,
            drive_file: None,
            preferred_source_url: preferred_manual_download_url,
            manual_download_url: None,
            warnings: Vec::new(),
            has_update: false,
        },
        LocalVersionState::Parsed(local) if latest_release.version.major_minor_matches(*local) => {
            ZiboPlanContext {
                local_state: local_state.clone(),
                latest_release,
                download_metadata: None,
                drive_file: None,
                preferred_source_url: preferred_manual_download_url,
                manual_download_url: None,
                warnings: Vec::new(),
                has_update: true,
            }
        }
        LocalVersionState::Parsed(_) => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            download_metadata: None,
            drive_file: None,
            preferred_source_url: preferred_manual_download_url.clone(),
            manual_download_url: Some(preferred_manual_download_url),
            warnings: vec![
                "Local Zibo major/minor version does not match the latest feed version; manual major download is required."
                    .to_string(),
            ],
            has_update: true,
        },
        LocalVersionState::Missing => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            download_metadata: None,
            drive_file: None,
            preferred_source_url: preferred_manual_download_url.clone(),
            manual_download_url: Some(preferred_manual_download_url),
            warnings: vec![
                "Local Zibo version.txt was not found; manual major download is required."
                    .to_string(),
            ],
            has_update: true,
        },
        LocalVersionState::Invalid(err) => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            download_metadata: None,
            drive_file: None,
            preferred_source_url: preferred_manual_download_url.clone(),
            manual_download_url: Some(preferred_manual_download_url),
            warnings: vec![
                format!(
                    "Local Zibo version.txt could not be parsed; manual major download is required. {}",
                    err
                ),
            ],
            has_update: true,
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
    let latest_release = fetch_latest_release(task_control).await?;
    let source_order = current_drive_source_order();
    let mut context = build_plan_context(
        &local_state,
        latest_release,
        source_order[0].folder_url.to_string(),
    );

    if context.has_update && context.manual_download_url.is_none() {
        let drive_file = resolve_drive_file_metadata_with_order(
            &context.latest_release,
            source_order,
            task_control,
        )
        .await?;
        context.download_metadata = Some(DownloadMetadata {
            total_bytes: drive_file.total_bytes,
        });
        context.drive_file = Some(drive_file);
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
        remote_module: Some(
            context
                .drive_file
                .as_ref()
                .and_then(|file| file.web_view_link.clone())
                .or_else(|| {
                    context
                        .drive_file
                        .as_ref()
                        .map(|file| file.source.folder_url.to_string())
                })
                .or_else(|| context.manual_download_url.clone())
                .unwrap_or_else(|| context.preferred_source_url.clone()),
        ),
        manual_download_url: context.manual_download_url,
        remote_locked: false,
        has_update: context.has_update,
        estimated_download_bytes: context
            .download_metadata
            .as_ref()
            .map(|metadata| metadata.total_bytes)
            .unwrap_or(0),
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
    _options: AddonUpdateOptions,
    task_control: Option<TaskControl>,
    progress_callback: Option<AddonUpdateProgressCallback>,
) -> Result<AddonUpdateResult> {
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

    if let Some(manual_download_url) = context.manual_download_url.as_ref() {
        return Err(anyhow!(
            "Latest Zibo release requires a manual major download from {}",
            manual_download_url
        ));
    }

    let planned_file = context
        .drive_file
        .clone()
        .ok_or_else(|| anyhow!("Missing Zibo Google Drive file metadata"))?;
    let download_metadata = context
        .download_metadata
        .clone()
        .ok_or_else(|| anyhow!("Missing Zibo download metadata"))?;
    let workdir = tempfile::tempdir().context("Failed to create Zibo update temp directory")?;
    let downloads_dir = workdir.path().join("downloads");
    let unpacked_dir = workdir.path().join("unpacked");
    fs::create_dir_all(&downloads_dir).context("Failed to create Zibo download directory")?;
    fs::create_dir_all(&unpacked_dir).context("Failed to create Zibo extraction directory")?;

    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "install",
        "started",
        0.0,
        0,
        download_metadata.total_bytes,
        0.0,
        Some("Resolving Zibo archive via Google Drive".to_string()),
        Some(planned_file.file_name.clone()),
    );

    let (downloaded_file, zip_path) = download_release_from_drive_with_order(
        &context.latest_release,
        current_drive_source_order(),
        &downloads_dir,
        task_control.as_ref(),
        &progress_callback,
        item_type,
        folder_name,
    )
    .await?;

    ensure_not_cancelled(task_control.as_ref(), "install")?;
    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "install",
        "running",
        92.0,
        downloaded_file.total_bytes,
        downloaded_file.total_bytes,
        0.0,
        Some("Extracting Zibo archive".to_string()),
        Some(
            zip_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("zibo.zip")
                .to_string(),
        ),
    );

    unzip_archive(&zip_path, &unpacked_dir)?;
    let extracted_root = find_extracted_zibo_root(&unpacked_dir)?;
    let updated_files = copy_dir_contents_overwrite(&extracted_root, &target_path)?;

    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "install",
        "completed",
        100.0,
        downloaded_file.total_bytes,
        downloaded_file.total_bytes,
        0.0,
        Some("Zibo update installed".to_string()),
        None,
    );

    log_info(format!(
        "Installed Zibo update '{}' from {} into '{}'",
        context.latest_release.title,
        downloaded_file.source.label,
        target_path.display()
    ));

    Ok(AddonUpdateResult {
        provider: ZIBO_PROVIDER.to_string(),
        success: true,
        message: format!(
            "Updated Zibo to {}",
            context.latest_release.version_string()
        ),
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

fn unzip_archive(archive_path: &Path, output_dir: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)
        .with_context(|| format!("Failed to open '{}'", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP '{}'", archive_path.display()))?;

    for idx in 0..archive.len() {
        let mut entry = archive.by_index(idx).with_context(|| {
            format!(
                "Failed to read ZIP entry {} from '{}'",
                idx,
                archive_path.display()
            )
        })?;
        let Some(enclosed) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            continue;
        };
        let destination = output_dir.join(enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&destination)
                .with_context(|| format!("Failed to create '{}'", destination.display()))?;
            continue;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create '{}'", parent.display()))?;
        }

        let mut output = fs::File::create(&destination)
            .with_context(|| format!("Failed to create '{}'", destination.display()))?;
        std::io::copy(&mut entry, &mut output)
            .with_context(|| format!("Failed to extract '{}'", destination.display()))?;
        output.flush().ok();
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

fn copy_dir_contents_overwrite(source_dir: &Path, target_dir: &Path) -> Result<usize> {
    let mut copied_files = 0usize;

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

        fs::copy(path, &destination).with_context(|| {
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

        let release = parse_latest_release(xml).unwrap();
        assert_eq!(release.version_string(), "4.05.31");
        assert_eq!(release.title, "B738X_XP12_4_05_31.zip");
    }

    #[test]
    fn build_plan_context_requires_manual_on_major_mismatch() {
        let local = LocalVersionState::Parsed(VersionTriple::new(4, 4, 18));
        let latest = ZiboRelease {
            version: VersionTriple::new(4, 5, 31),
            title: "B738X_XP12_4_05_31.zip".to_string(),
        };

        let context =
            build_plan_context(&local, latest, ZIBO_DRIVE_SOURCES[0].folder_url.to_string());
        assert!(context.has_update);
        assert_eq!(
            context.manual_download_url.as_deref(),
            Some(ZIBO_DRIVE_SOURCES[0].folder_url)
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
}

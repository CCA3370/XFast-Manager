use anyhow::{anyhow, Context, Result};
use librqbit::{AddTorrent, AddTorrentOptions, Session, SessionOptions, TorrentStats};
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::time::sleep;
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
pub const ZIBO_MANUAL_DOWNLOAD_URL: &str =
    "https://drive.google.com/drive/folders/1RHz4PQqWNGGpVG9GaHr84kuGs8LM2xyK";
const ZIBO_VERSION_FILE: &str = "version.txt";
const LOG_CTX: &str = "zibo_updater";
const ZIBO_NO_PEER_TIMEOUT_SECS: u64 = 45;
const ZIBO_STALLED_TIMEOUT_SECS: u64 = 180;
const ZIBO_TORRENT_LOG_INTERVAL_SECS: u64 = 5;

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
    torrent_url: String,
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
struct TorrentMetadata {
    file_name: String,
    total_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct PeerSummary {
    queued: usize,
    connecting: usize,
    live: usize,
    seen: usize,
}

impl PeerSummary {
    fn from_stats(stats: &TorrentStats) -> Self {
        let Some(live_stats) = stats.live.as_ref() else {
            return Self::default();
        };
        let peer_stats = &live_stats.snapshot.peer_stats;
        Self {
            queued: peer_stats.queued,
            connecting: peer_stats.connecting,
            live: peer_stats.live,
            seen: peer_stats.seen,
        }
    }

    fn has_activity(self) -> bool {
        self.queued > 0 || self.connecting > 0 || self.live > 0 || self.seen > 0
    }

    fn display_string(self) -> String {
        format!(
            "seen {}, connecting {}, live {}",
            self.seen, self.connecting, self.live
        )
    }
}

#[derive(Debug, Clone)]
struct ZiboPlanContext {
    local_state: LocalVersionState,
    latest_release: ZiboRelease,
    torrent_metadata: Option<TorrentMetadata>,
    manual_download_url: Option<String>,
    warnings: Vec<String>,
    has_update: bool,
}

#[derive(Debug, Clone)]
enum BencodeValue {
    Int(i64),
    Bytes(Vec<u8>),
    List,
    Dict(BTreeMap<String, BencodeValue>),
}

impl BencodeValue {
    fn as_dict(&self) -> Option<&BTreeMap<String, BencodeValue>> {
        match self {
            Self::Dict(value) => Some(value),
            _ => None,
        }
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(value) => Some(value.as_slice()),
            _ => None,
        }
    }

    fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }
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
        let title = extract_xml_tag(item_body.as_str(), "title");
        let link = extract_xml_tag(item_body.as_str(), "link");

        let (Some(title), Some(link)) = (title, link) else {
            continue;
        };
        if let Some(release) = parse_release_entry(&title, &link) {
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

fn parse_release_entry(title: &str, link: &str) -> Option<ZiboRelease> {
    let captures = ZIBO_TITLE_RE.captures(title.trim())?;
    let major = captures.get(1)?.as_str().parse::<u32>().ok()?;
    let minor = captures.get(2)?.as_str().parse::<u32>().ok()?;
    let patch = match captures.get(3)?.as_str().to_ascii_lowercase().as_str() {
        "full" => 0,
        value => value.parse::<u32>().ok()?,
    };
    let torrent_url = reqwest::Url::parse(link.trim()).ok()?.to_string();

    Some(ZiboRelease {
        version: VersionTriple::new(major, minor, patch),
        title: title.trim().to_string(),
        torrent_url,
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
        .build()
        .context("Failed to create HTTP client")
}

async fn fetch_torrent_metadata(
    release: &ZiboRelease,
    task_control: Option<&TaskControl>,
) -> Result<TorrentMetadata> {
    let bytes = fetch_torrent_file_bytes(release, task_control).await?;
    parse_torrent_metadata(bytes.as_slice())
}

async fn fetch_torrent_file_bytes(
    release: &ZiboRelease,
    task_control: Option<&TaskControl>,
) -> Result<Vec<u8>> {
    ensure_not_cancelled(task_control, "scan")?;
    let client = build_http_client(20)?;
    let bytes = client
        .get(&release.torrent_url)
        .send()
        .await
        .with_context(|| format!("Failed to download torrent '{}'", release.torrent_url))?
        .error_for_status()
        .with_context(|| format!("Torrent URL returned error: {}", release.torrent_url))?
        .bytes()
        .await
        .context("Failed to read torrent file bytes")?;
    ensure_not_cancelled(task_control, "scan")?;
    Ok(bytes.to_vec())
}

fn parse_torrent_metadata(bytes: &[u8]) -> Result<TorrentMetadata> {
    let value = parse_bencode(bytes)?;
    let root = value
        .as_dict()
        .ok_or_else(|| anyhow!("Torrent file root is not a dictionary"))?;
    let info = root
        .get("info")
        .and_then(BencodeValue::as_dict)
        .ok_or_else(|| anyhow!("Torrent file is missing an 'info' dictionary"))?;

    if info.contains_key("files") {
        return Err(anyhow!(
            "Zibo torrent contains multiple files; automatic install expects a single ZIP payload"
        ));
    }

    let file_name = info
        .get("name")
        .and_then(BencodeValue::as_bytes)
        .map(|value| String::from_utf8_lossy(value).to_string())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("Torrent file is missing the payload name"))?;

    if !file_name.to_ascii_lowercase().ends_with(".zip") {
        return Err(anyhow!(
            "Zibo torrent payload is not a ZIP archive: {}",
            file_name
        ));
    }

    let length = info
        .get("length")
        .and_then(BencodeValue::as_int)
        .ok_or_else(|| anyhow!("Torrent file is missing the payload length"))?;
    if length <= 0 {
        return Err(anyhow!("Torrent payload length is invalid: {}", length));
    }

    Ok(TorrentMetadata {
        file_name,
        total_bytes: length as u64,
    })
}

fn parse_bencode(input: &[u8]) -> Result<BencodeValue> {
    let (value, consumed) = parse_bencode_at(input, 0)?;
    if consumed != input.len() {
        return Err(anyhow!("Torrent metadata contains trailing bytes"));
    }
    Ok(value)
}

fn parse_bencode_at(input: &[u8], pos: usize) -> Result<(BencodeValue, usize)> {
    let Some(current) = input.get(pos).copied() else {
        return Err(anyhow!("Unexpected end of bencode input"));
    };

    match current {
        b'i' => parse_bencode_int(input, pos),
        b'l' => parse_bencode_list(input, pos),
        b'd' => parse_bencode_dict(input, pos),
        b'0'..=b'9' => parse_bencode_bytes(input, pos),
        _ => Err(anyhow!("Invalid bencode token at byte {}", pos)),
    }
}

fn parse_bencode_int(input: &[u8], pos: usize) -> Result<(BencodeValue, usize)> {
    let start = pos + 1;
    let end = input[start..]
        .iter()
        .position(|byte| *byte == b'e')
        .map(|offset| start + offset)
        .ok_or_else(|| anyhow!("Unterminated bencode integer"))?;
    let value = std::str::from_utf8(&input[start..end])
        .context("Bencode integer is not valid UTF-8")?
        .parse::<i64>()
        .context("Bencode integer is not valid")?;
    Ok((BencodeValue::Int(value), end + 1))
}

fn parse_bencode_bytes(input: &[u8], pos: usize) -> Result<(BencodeValue, usize)> {
    let colon = input[pos..]
        .iter()
        .position(|byte| *byte == b':')
        .map(|offset| pos + offset)
        .ok_or_else(|| anyhow!("Bencode string is missing ':'"))?;
    let len = std::str::from_utf8(&input[pos..colon])
        .context("Bencode string length is not valid UTF-8")?
        .parse::<usize>()
        .context("Bencode string length is invalid")?;
    let start = colon + 1;
    let end = start
        .checked_add(len)
        .ok_or_else(|| anyhow!("Bencode string length overflow"))?;
    let slice = input
        .get(start..end)
        .ok_or_else(|| anyhow!("Bencode string exceeds input length"))?;
    Ok((BencodeValue::Bytes(slice.to_vec()), end))
}

fn parse_bencode_list(input: &[u8], pos: usize) -> Result<(BencodeValue, usize)> {
    let mut cursor = pos + 1;

    while input.get(cursor).copied() != Some(b'e') {
        let (_value, next) = parse_bencode_at(input, cursor)?;
        cursor = next;
    }

    Ok((BencodeValue::List, cursor + 1))
}

fn parse_bencode_dict(input: &[u8], pos: usize) -> Result<(BencodeValue, usize)> {
    let mut values = BTreeMap::new();
    let mut cursor = pos + 1;

    while input.get(cursor).copied() != Some(b'e') {
        let (key, next) = parse_bencode_bytes(input, cursor)?;
        let key = String::from_utf8_lossy(
            key.as_bytes()
                .ok_or_else(|| anyhow!("Bencode dictionary key is not bytes"))?,
        )
        .to_string();
        let (value, next_value) = parse_bencode_at(input, next)?;
        values.insert(key, value);
        cursor = next_value;
    }

    Ok((BencodeValue::Dict(values), cursor + 1))
}

fn build_plan_context(
    local_state: &LocalVersionState,
    latest_release: ZiboRelease,
) -> ZiboPlanContext {
    match local_state {
        LocalVersionState::Parsed(local) if latest_release.version <= *local => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            torrent_metadata: None,
            manual_download_url: None,
            warnings: Vec::new(),
            has_update: false,
        },
        LocalVersionState::Parsed(local) if latest_release.version.major_minor_matches(*local) => {
            ZiboPlanContext {
                local_state: local_state.clone(),
                latest_release,
                torrent_metadata: None,
                manual_download_url: None,
                warnings: Vec::new(),
                has_update: true,
            }
        }
        LocalVersionState::Parsed(_) => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            torrent_metadata: None,
            manual_download_url: Some(ZIBO_MANUAL_DOWNLOAD_URL.to_string()),
            warnings: vec![
                "Local Zibo major/minor version does not match the latest feed version; manual major download is required."
                    .to_string(),
            ],
            has_update: true,
        },
        LocalVersionState::Missing => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            torrent_metadata: None,
            manual_download_url: Some(ZIBO_MANUAL_DOWNLOAD_URL.to_string()),
            warnings: vec![
                "Local Zibo version.txt was not found; manual major download is required."
                    .to_string(),
            ],
            has_update: true,
        },
        LocalVersionState::Invalid(err) => ZiboPlanContext {
            local_state: local_state.clone(),
            latest_release,
            torrent_metadata: None,
            manual_download_url: Some(ZIBO_MANUAL_DOWNLOAD_URL.to_string()),
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
    let mut context = build_plan_context(&local_state, latest_release);

    if context.has_update && context.manual_download_url.is_none() {
        context.torrent_metadata =
            Some(fetch_torrent_metadata(&context.latest_release, task_control).await?);
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
                .manual_download_url
                .clone()
                .unwrap_or_else(|| context.latest_release.torrent_url.clone()),
        ),
        manual_download_url: context.manual_download_url,
        remote_locked: false,
        has_update: context.has_update,
        estimated_download_bytes: context
            .torrent_metadata
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

    if context.manual_download_url.is_some() {
        return Err(anyhow!(
            "Latest Zibo release requires a manual major download from {}",
            ZIBO_MANUAL_DOWNLOAD_URL
        ));
    }

    let torrent_metadata = context
        .torrent_metadata
        .clone()
        .ok_or_else(|| anyhow!("Missing Zibo torrent metadata"))?;
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
        torrent_metadata.total_bytes,
        0.0,
        Some("Downloading Zibo update".to_string()),
        Some(torrent_metadata.file_name.clone()),
    );

    let zip_path = download_torrent_payload(
        &context.latest_release,
        &torrent_metadata,
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
        torrent_metadata.total_bytes,
        torrent_metadata.total_bytes,
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
        torrent_metadata.total_bytes,
        torrent_metadata.total_bytes,
        0.0,
        Some("Zibo update installed".to_string()),
        None,
    );

    log_info(format!(
        "Installed Zibo update '{}' into '{}'",
        context.latest_release.title,
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

async fn download_torrent_payload(
    release: &ZiboRelease,
    metadata: &TorrentMetadata,
    download_root: &Path,
    task_control: Option<&TaskControl>,
    progress_callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
) -> Result<PathBuf> {
    let torrent_bytes = fetch_torrent_file_bytes(release, task_control).await?;
    let (session, dht_enabled) = create_torrent_session(download_root).await?;
    let handle = session
        .add_torrent(
            AddTorrent::from_bytes(torrent_bytes),
            Some(AddTorrentOptions {
                paused: false,
                overwrite: true,
                output_folder: Some(download_root.to_string_lossy().to_string()),
                ..Default::default()
            }),
        )
        .await
        .with_context(|| format!("Failed to add torrent '{}'", release.torrent_url))?
        .into_handle()
        .ok_or_else(|| anyhow!("BitTorrent session returned a list-only torrent"))?;

    let mut wait_future = std::pin::pin!(handle.wait_until_completed());
    let download_result: Result<()> = async {
        let mut last_sample_bytes = 0u64;
        let mut last_sample_at = Instant::now();
        let mut last_progress_at = Instant::now();
        let mut last_status_log_at = Instant::now() - Duration::from_secs(ZIBO_TORRENT_LOG_INTERVAL_SECS);

        loop {
            ensure_not_cancelled(task_control, "install")?;
            let stats = handle.stats();
            if let Some(error) = stats.error.as_ref() {
                return Err(anyhow!("BitTorrent download failed: {}", error));
            }

            let processed_bytes = stats.progress_bytes.min(metadata.total_bytes);
            let peer_summary = PeerSummary::from_stats(&stats);
            if processed_bytes > last_sample_bytes {
                last_progress_at = Instant::now();
            }

            let elapsed = last_sample_at.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 && processed_bytes >= last_sample_bytes {
                (processed_bytes - last_sample_bytes) as f64 / elapsed
            } else {
                0.0
            };

            let progress_message = if processed_bytes == 0 {
                format!("Searching Zibo peers ({})", peer_summary.display_string())
            } else {
                format!("Downloading {} ({})", release.title, peer_summary.display_string())
            };

            emit_progress_event(
                progress_callback,
                item_type,
                folder_name,
                "install",
                "running",
                if metadata.total_bytes > 0 {
                    (processed_bytes as f64 / metadata.total_bytes as f64) * 90.0
                } else {
                    0.0
                },
                processed_bytes,
                metadata.total_bytes,
                speed,
                Some(progress_message),
                Some(metadata.file_name.clone()),
            );

            if last_status_log_at.elapsed() >= Duration::from_secs(ZIBO_TORRENT_LOG_INTERVAL_SECS) {
                let live_speed = stats
                    .live
                    .as_ref()
                    .map(|live| live.download_speed.to_string())
                    .unwrap_or_else(|| "0.00 MiB/s".to_string());
                log_info(format!(
                    "Zibo torrent status: state={}, progress={}/{}, speed={}, dht_enabled={}, peers={}",
                    stats.state,
                    processed_bytes,
                    metadata.total_bytes,
                    live_speed,
                    dht_enabled,
                    peer_summary.display_string()
                ));
                last_status_log_at = Instant::now();
            }

            if stats.finished {
                break;
            }

            if processed_bytes == 0
                && !peer_summary.has_activity()
                && last_progress_at.elapsed() >= Duration::from_secs(ZIBO_NO_PEER_TIMEOUT_SECS)
            {
                return Err(anyhow!(
                    "No reachable peers were found for the Zibo torrent after {} seconds. Automatic update could not start.{} Torrent: {}",
                    ZIBO_NO_PEER_TIMEOUT_SECS,
                    if dht_enabled {
                        ""
                    } else {
                        " The BitTorrent session had to fall back to tracker-only mode because DHT initialization failed."
                    },
                    release.torrent_url
                ));
            }

            if processed_bytes < metadata.total_bytes
                && last_progress_at.elapsed() >= Duration::from_secs(ZIBO_STALLED_TIMEOUT_SECS)
            {
                return Err(anyhow!(
                    "The Zibo torrent stalled for more than {} seconds. Peer status: {}.{} Torrent: {}",
                    ZIBO_STALLED_TIMEOUT_SECS,
                    peer_summary.display_string(),
                    if dht_enabled {
                        ""
                    } else {
                        " The BitTorrent session had to fall back to tracker-only mode because DHT initialization failed."
                    },
                    release.torrent_url
                ));
            }

            last_sample_bytes = processed_bytes;
            last_sample_at = Instant::now();

            tokio::select! {
                result = &mut wait_future => {
                    result.context("BitTorrent download failed")?;
                    break;
                }
                _ = sleep(Duration::from_millis(500)) => {}
            }
        }

        Ok(())
    }
    .await;

    session.stop().await;
    download_result?;

    let zip_path = download_root.join(&metadata.file_name);
    if !zip_path.is_file() {
        return Err(anyhow!(
            "BitTorrent download completed but '{}' was not found",
            zip_path.display()
        ));
    }

    Ok(zip_path)
}

async fn create_torrent_session(download_root: &Path) -> Result<(Arc<Session>, bool)> {
    let dht_enabled_opts = SessionOptions {
        disable_dht: false,
        disable_dht_persistence: true,
        persistence: None,
        listen_port_range: None,
        enable_upnp_port_forwarding: false,
        ..Default::default()
    };

    match Session::new_with_opts(download_root.to_path_buf(), dht_enabled_opts).await {
        Ok(session) => Ok((session, true)),
        Err(primary_err) => {
            log_info(format!(
                "Failed to create DHT-enabled BitTorrent session, retrying without DHT: {primary_err:#}"
            ));
            let fallback_opts = SessionOptions {
                disable_dht: true,
                disable_dht_persistence: true,
                persistence: None,
                listen_port_range: None,
                enable_upnp_port_forwarding: false,
                ..Default::default()
            };
            let session = Session::new_with_opts(download_root.to_path_buf(), fallback_opts)
                .await
                .context("Failed to create BitTorrent session")?;
            Ok((session, false))
        }
    }
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
        assert_eq!(release.torrent_url, "https://example.com/31.torrent");
    }

    #[test]
    fn build_plan_context_requires_manual_on_major_mismatch() {
        let local = LocalVersionState::Parsed(VersionTriple::new(4, 4, 18));
        let latest = ZiboRelease {
            version: VersionTriple::new(4, 5, 31),
            title: "B738X_XP12_4_05_31.zip".to_string(),
            torrent_url: "https://example.com/31.torrent".to_string(),
        };

        let context = build_plan_context(&local, latest);
        assert!(context.has_update);
        assert_eq!(
            context.manual_download_url.as_deref(),
            Some(ZIBO_MANUAL_DOWNLOAD_URL)
        );
    }

    #[test]
    fn bencode_metadata_parser_reads_single_file_torrent() {
        let torrent = b"d8:announce14:http://tracker4:infod6:lengthi42e4:name12:testfile.zipee";
        let metadata = parse_torrent_metadata(torrent).unwrap();
        assert_eq!(metadata.file_name, "testfile.zip");
        assert_eq!(metadata.total_bytes, 42);
    }
}

use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::error::{ApiError, ApiErrorCode};

macro_rules! csl_debug {
    ($($arg:tt)*) => {
        if crate::logger::is_debug_enabled() {
            crate::log_debug!(&format!($($arg)*), "csl");
        }
    };
}

const DEFAULT_SERVER_BASE_URL: &str = "http://x-csl.ru";
const CSL_CANONICAL_REL: &str = "Resources/plugins/IVAO_CSL/CSL";
const CSL_API_BASE_PATH: &str = "package";
const ALTITUDE_API_BASE_PATH: &str = "package/ALTITUDE";
const CSL_INDEX_PATH: &str = "package/x-csl-indexes.idx";
const ALTITUDE_INDEX_PATH: &str = "package/ALTITUDE/files.idx";
const MAX_CSL_PARALLEL_DOWNLOADS: usize = 12;
const CSL_SCAN_COMPARE_CONCURRENCY_LIMIT: usize = 8;
const CSL_RESCAN_COMPARE_CONCURRENCY_LIMIT: usize = 4;
const DESCRIPTION_FETCH_CONCURRENCY: usize = 6;
const DESCRIPTION_FETCH_ATTEMPTS: u32 = 3;
const CSL_LINK_SYNC_EVENT: &str = "csl-link-sync-progress";
const CSL_LINK_SYNC_STATE_DIR: &str = ".xfast-csl-sync";

/// Cached index with TTL, keyed by server URL
static INDEX_CACHE: std::sync::LazyLock<Mutex<HashMap<String, (std::time::Instant, String)>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

const INDEX_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(300); // 5 minutes

/// Cached package descriptions, keyed by "{server}::{package_name}".
static DESC_CACHE: std::sync::LazyLock<Mutex<HashMap<String, String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Cached local MD5 hashes: file_path → (size, mtime_secs, md5).
/// If size+mtime still match, the cached hash is reused (avoids re-reading the file).
static MD5_CACHE: std::sync::LazyLock<std::sync::Mutex<HashMap<PathBuf, (u64, i64, String)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

#[derive(Clone, Debug)]
struct RequestContext {
    operation: String,
    operation_id: String,
    next_http_request_seq: Arc<AtomicU64>,
}

impl RequestContext {
    fn new(operation: &str, request_id: Option<String>) -> Self {
        Self {
            operation: operation.to_string(),
            operation_id: normalize_request_id(request_id.as_deref(), operation),
            next_http_request_seq: Arc::new(AtomicU64::new(0)),
        }
    }

    fn operation(&self) -> &str {
        &self.operation
    }

    fn operation_id(&self) -> &str {
        &self.operation_id
    }

    fn next_http_request_id(&self) -> String {
        let seq = self.next_http_request_seq.fetch_add(1, Ordering::Relaxed) + 1;
        format!("{}-{:04}", self.operation_id, seq)
    }
}

fn sanitize_request_id(raw: &str) -> Option<String> {
    let sanitized: String = raw
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
        .take(96)
        .collect();

    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

fn normalize_request_id(request_id: Option<&str>, operation: &str) -> String {
    request_id.and_then(sanitize_request_id).unwrap_or_else(|| {
        format!(
            "{}-{}",
            operation.replace('_', "-"),
            Uuid::new_v4().simple()
        )
    })
}

fn with_request_tracking_headers(
    builder: reqwest::RequestBuilder,
    request_ctx: &RequestContext,
    request_id: &str,
) -> reqwest::RequestBuilder {
    builder
        .header("X-Request-Id", request_id)
        .header("X-XFast-Operation-Id", request_ctx.operation_id())
        .header("X-XFast-Operation", request_ctx.operation())
}

fn build_http_client(timeout: std::time::Duration) -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .timeout(timeout)
        .user_agent("XFast Manager")
        .build()
        .map_err(|e| {
            ApiError::new(
                ApiErrorCode::NetworkError,
                format!("HTTP client error: {}", e),
            )
        })
}

#[derive(Default)]
pub struct CslDownloadControl {
    tasks: Arc<StdMutex<HashMap<String, Arc<AtomicBool>>>>,
}

pub struct CslDownloadRegistration {
    key: String,
    cancel_flag: Arc<AtomicBool>,
    tasks: Arc<StdMutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl CslDownloadRegistration {
    fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel_flag)
    }
}

impl Drop for CslDownloadRegistration {
    fn drop(&mut self) {
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.remove(&self.key);
        }
    }
}

impl CslDownloadControl {
    pub fn new() -> Self {
        Self::default()
    }

    fn register(&self, key: String) -> Result<CslDownloadRegistration, ApiError> {
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|_| ApiError::internal("Failed to lock CSL download registry"))?;

        if tasks.contains_key(&key) {
            return Err(ApiError::conflict(format!(
                "Download already running for {}",
                key
            )));
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        tasks.insert(key.clone(), Arc::clone(&cancel_flag));

        Ok(CslDownloadRegistration {
            key,
            cancel_flag,
            tasks: Arc::clone(&self.tasks),
        })
    }

    fn cancel(&self, key: &str) {
        if let Ok(tasks) = self.tasks.lock() {
            if let Some(flag) = tasks.get(key) {
                flag.store(true, Ordering::SeqCst);
            }
        }
    }
}

fn install_task_key(source: &str, package_name: &str) -> String {
    format!("{}:{}", source, package_name)
}

fn clamp_parallel_downloads(parallel_downloads: Option<usize>) -> usize {
    parallel_downloads
        .unwrap_or(1)
        .clamp(1, MAX_CSL_PARALLEL_DOWNLOADS)
}

fn ensure_download_not_cancelled(
    cancel_flag: &AtomicBool,
    package_name: &str,
) -> Result<(), ApiError> {
    if cancel_flag.load(Ordering::SeqCst) {
        return Err(ApiError::cancelled(format!(
            "Download cancelled for {}",
            package_name
        )));
    }

    Ok(())
}

fn temp_download_path(local_path: &Path) -> PathBuf {
    let file_name = local_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "download.tmp".to_string());

    local_path.with_file_name(format!("{}.xfastpart", file_name))
}

fn resolve_server_base_url(server_base_url: Option<&str>) -> String {
    server_base_url
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .unwrap_or(DEFAULT_SERVER_BASE_URL)
        .trim_end_matches('/')
        .to_string()
}

fn resolve_csl_api_base(server_base_url: Option<&str>) -> String {
    format!(
        "{}/{}",
        resolve_server_base_url(server_base_url),
        CSL_API_BASE_PATH
    )
}

fn resolve_altitude_api_base(server_base_url: Option<&str>) -> String {
    format!(
        "{}/{}",
        resolve_server_base_url(server_base_url),
        ALTITUDE_API_BASE_PATH
    )
}

fn description_cache_key(server: &str, package_name: &str) -> String {
    format!("{}::{}", server, package_name)
}

// ============================================================================
// Data Structures
// ============================================================================

/// Parsed entry from x-csl-indexes.idx
#[derive(Debug, Clone)]
struct CslIndexEntry {
    entry_type: u8, // 10=file, 11=package header, 15=directory
    path: String,
    size_bytes: u64,
    md5_hash: Option<String>,
    date: String,
    time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CslPackageInfo {
    pub name: String,
    pub total_size_bytes: u64,
    pub file_count: usize,
    pub description: String,
    pub status: String, // "not_installed" | "needs_update" | "up_to_date"
    pub files_to_update: usize,
    pub update_size_bytes: u64,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CslPath {
    pub path: String,
    pub source: String, // "auto" | "custom"
    pub plugin_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CslScanResult {
    pub packages: Vec<CslPackageInfo>,
    pub paths: Vec<CslPath>,
    pub server_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CslProgressEvent {
    pub package_name: String,
    pub current_file: usize,
    pub total_files: usize,
    pub current_file_name: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CslLinkSyncProgressEvent {
    pub request_id: String,
    pub phase: String,
    pub current_target_path: String,
    pub current_package_name: String,
    pub current_file_name: String,
    pub processed_files: usize,
    pub total_files: usize,
    pub completed_targets: usize,
    pub total_targets: usize,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncTargetMode {
    HardLinks,
    DirectoryLinkFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkSyncMode {
    MissingOnly,
    Reconcile,
}

#[derive(Debug, Clone)]
struct SyncTarget {
    base: PathBuf,
    mode: SyncTargetMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CslHardLinkManifest {
    version: u8,
    package_name: String,
    files: Vec<String>,
}

struct LinkSyncProgressTracker {
    app_handle: Option<AppHandle>,
    request_id: String,
    processed_files: usize,
    total_files: usize,
    completed_targets: usize,
    total_targets: usize,
    current_target_path: String,
    current_package_name: String,
    current_file_name: String,
}

impl LinkSyncProgressTracker {
    fn new(
        app_handle: Option<AppHandle>,
        request_id: String,
        total_files: usize,
        total_targets: usize,
    ) -> Self {
        Self {
            app_handle,
            request_id,
            processed_files: 0,
            total_files,
            completed_targets: 0,
            total_targets,
            current_target_path: String::new(),
            current_package_name: String::new(),
            current_file_name: String::new(),
        }
    }

    fn emit(&self, phase: &str, message: Option<String>) {
        if let Some(app_handle) = &self.app_handle {
            let _ = app_handle.emit(
                CSL_LINK_SYNC_EVENT,
                CslLinkSyncProgressEvent {
                    request_id: self.request_id.clone(),
                    phase: phase.to_string(),
                    current_target_path: self.current_target_path.clone(),
                    current_package_name: self.current_package_name.clone(),
                    current_file_name: self.current_file_name.clone(),
                    processed_files: self.processed_files,
                    total_files: self.total_files,
                    completed_targets: self.completed_targets,
                    total_targets: self.total_targets,
                    message,
                },
            );
        }
    }

    fn emit_preparing(&self) {
        self.emit("preparing", None);
    }

    fn begin_target(&mut self, target_base: &Path, message: Option<String>) {
        self.current_target_path = target_base.to_string_lossy().to_string();
        self.current_package_name.clear();
        self.current_file_name.clear();
        self.emit("syncing", message);
    }

    fn begin_package(&mut self, package_name: &str) {
        self.current_package_name = package_name.to_string();
        self.current_file_name.clear();
    }

    fn advance_file(&mut self, file_name: &str) {
        self.current_file_name = file_name.to_string();
        self.processed_files = self.processed_files.saturating_add(1);
        self.emit("syncing", None);
    }

    fn advance_by(&mut self, count: usize, file_name: &str, message: Option<String>) {
        self.current_file_name = file_name.to_string();
        self.processed_files = self.processed_files.saturating_add(count);
        self.emit("syncing", message);
    }

    fn finish_target(&mut self) {
        self.completed_targets = self.completed_targets.saturating_add(1);
        self.current_package_name.clear();
        self.current_file_name.clear();
        self.emit("syncing", None);
    }

    fn complete(&mut self) {
        self.processed_files = self.total_files;
        self.current_package_name.clear();
        self.current_file_name.clear();
        self.emit("completed", None);
    }

    fn fail(&self, message: String) {
        self.emit("failed", Some(message));
    }
}

// ============================================================================
// Index Parsing
// ============================================================================

fn parse_index(content: &str) -> Vec<CslIndexEntry> {
    let lines: Vec<&str> = content.lines().collect();
    let mut entries = Vec::with_capacity(lines.len());

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Fast path: check first char before splitting
        let first_byte = line.as_bytes()[0];
        if first_byte == b'0' && line.starts_with("0 ") {
            continue; // Header line
        }

        let parts: Vec<&str> = line.splitn(7, '%').collect();
        if parts.len() < 6 {
            continue;
        }

        let entry_type = match parts[0].parse::<u8>() {
            Ok(t) if t == 10 || t == 11 || t == 15 => t,
            _ => continue,
        };

        let path = parts[1].to_string();
        let size_bytes = parts[2].parse::<u64>().unwrap_or(0);

        let md5_hash = if entry_type == 10 {
            let hash = parts[3];
            if hash == "Reserve" || hash.is_empty() {
                None
            } else {
                Some(hash.to_string())
            }
        } else {
            None
        };

        entries.push(CslIndexEntry {
            entry_type,
            path,
            size_bytes,
            md5_hash,
            date: parts[4].to_string(),
            time: parts[5].to_string(),
        });
    }

    entries
}

/// Grouped package data (owned, Send-safe)
struct PackageData {
    name: String,
    header_size: u64,
    header_date: String,
    header_time: String,
    files: Vec<FileEntry>,
}

#[derive(Clone)]
struct FileEntry {
    path: String,
    size_bytes: u64,
    md5_hash: Option<String>,
}

fn group_into_packages(entries: &[CslIndexEntry]) -> Vec<PackageData> {
    let mut map: HashMap<String, PackageData> = HashMap::new();

    for entry in entries {
        match entry.entry_type {
            11 => {
                let pkg = map
                    .entry(entry.path.clone())
                    .or_insert_with(|| PackageData {
                        name: entry.path.clone(),
                        header_size: 0,
                        header_date: String::new(),
                        header_time: String::new(),
                        files: Vec::new(),
                    });
                pkg.header_size = entry.size_bytes;
                pkg.header_date = entry.date.clone();
                pkg.header_time = entry.time.clone();
            }
            10 => {
                if let Some(pkg_name) = entry.path.split('/').next() {
                    let pkg = map
                        .entry(pkg_name.to_string())
                        .or_insert_with(|| PackageData {
                            name: pkg_name.to_string(),
                            header_size: 0,
                            header_date: String::new(),
                            header_time: String::new(),
                            files: Vec::new(),
                        });
                    pkg.files.push(FileEntry {
                        path: entry.path.clone(),
                        size_bytes: entry.size_bytes,
                        md5_hash: entry.md5_hash.clone(),
                    });
                }
            }
            _ => {}
        }
    }

    let mut pkgs: Vec<PackageData> = map.into_values().collect();
    pkgs.sort_by(|a, b| a.name.cmp(&b.name));
    pkgs
}

// ============================================================================
// Path Detection
// ============================================================================

const CSL_PLUGIN_PATHS: &[(&str, &str)] = &[
    ("Resources/plugins/IVAO_CSL/CSL", "IVAO CSL"),
    ("Resources/plugins/xPilot/Resources/CSL", "xPilot"),
    ("Resources/plugins/swift-X-Plane/CSL", "swift"),
    ("Resources/plugins/XSquawkBox/Resources/CSL", "XSquawkBox"),
    ("Resources/plugins/LiveTraffic/CSL", "LiveTraffic"),
];

pub fn detect_csl_paths(xplane_path: &Path) -> Vec<CslPath> {
    let mut paths = Vec::new();
    for (rel_path, plugin_name) in CSL_PLUGIN_PATHS {
        let full_path = xplane_path.join(rel_path);
        if full_path.exists() {
            paths.push(CslPath {
                path: full_path.to_string_lossy().to_string(),
                source: "auto".to_string(),
                plugin_name: Some(plugin_name.to_string()),
            });
        }
    }
    paths
}

// ============================================================================
// Optimized Local Comparison
// ============================================================================

/// Compute MD5 with 64KB buffer
fn compute_file_md5(path: &Path) -> Result<String, std::io::Error> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut context = md5::Context::new();
    let mut buffer = [0u8; 65536]; // 64KB buffer
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.consume(&buffer[..count]);
    }
    Ok(format!("{:x}", context.compute()))
}

/// Get the mtime of a file as seconds-since-epoch (platform-portable i64).
fn mtime_secs(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Compute MD5, but reuse a cached hash when file size + mtime are unchanged.
fn compute_file_md5_cached(
    path: &Path,
    meta: &std::fs::Metadata,
) -> Result<String, std::io::Error> {
    let size = meta.len();
    let mtime = mtime_secs(meta);

    // Check cache (std::sync::Mutex — fine inside spawn_blocking)
    {
        let cache = MD5_CACHE.lock().unwrap();
        if let Some((cached_size, cached_mtime, ref cached_hash)) = cache.get(path) {
            if *cached_size == size && *cached_mtime == mtime {
                return Ok(cached_hash.clone());
            }
        }
    }

    let hash = compute_file_md5(path)?;

    // Store in cache
    {
        let mut cache = MD5_CACHE.lock().unwrap();
        cache.insert(path.to_path_buf(), (size, mtime, hash.clone()));
    }

    Ok(hash)
}

fn comparison_parallelism(max_limit: usize) -> usize {
    std::thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(4)
        .clamp(1, max_limit.max(1))
}

/// Enumerate package directories once per scan instead of probing every package name.
fn collect_local_package_dirs(local_paths: &[String]) -> HashMap<String, PathBuf> {
    let mut package_dirs = HashMap::new();

    for base_path in local_paths {
        let entries = match std::fs::read_dir(base_path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let Some(folder_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            package_dirs.entry(folder_name.to_string()).or_insert(path);
        }
    }

    package_dirs
}

fn package_total_size(pkg: &PackageData) -> u64 {
    if pkg.header_size > 0 {
        pkg.header_size
    } else {
        pkg.files.iter().map(|f| f.size_bytes).sum()
    }
}

fn package_last_updated(pkg: &PackageData) -> String {
    if !pkg.header_date.is_empty() {
        format!("{} {}", pkg.header_date, pkg.header_time)
    } else {
        String::new()
    }
}

fn build_csl_package_info(
    pkg: &PackageData,
    status: &str,
    files_to_update: usize,
    update_size_bytes: u64,
) -> CslPackageInfo {
    CslPackageInfo {
        name: pkg.name.clone(),
        total_size_bytes: package_total_size(pkg),
        file_count: pkg.files.len(),
        description: String::new(),
        status: status.to_string(),
        files_to_update,
        update_size_bytes,
        last_updated: package_last_updated(pkg),
    }
}

/// Compare a single package against local files without hashing.
/// Returns `checking` when size-based comparison passes and a hash pass is still needed.
fn compare_package_quick(pkg: &PackageData, local_dir: &Path) -> (String, usize, u64) {
    compare_package(pkg, local_dir, false)
}

/// Compare a single package against local files with MD5 verification when available.
fn compare_package_exact(pkg: &PackageData, local_dir: &Path) -> (String, usize, u64) {
    compare_package(pkg, local_dir, true)
}

fn compare_package(
    pkg: &PackageData,
    local_dir: &Path,
    verify_hashes: bool,
) -> (String, usize, u64) {
    let prefix = format!("{}/", pkg.name);
    let mut files_to_update = 0usize;
    let mut update_size: u64 = 0;

    for file in &pkg.files {
        let rel_path = file.path.strip_prefix(&prefix).unwrap_or(&file.path);
        let local_file = local_dir.join(rel_path);

        // Single metadata() call — covers both exists check and size check
        match std::fs::metadata(&local_file) {
            Ok(meta) => {
                if meta.len() != file.size_bytes {
                    files_to_update += 1;
                    update_size += file.size_bytes;
                    continue;
                }
                if verify_hashes {
                    // Size matches → compute MD5 only if server provides hash
                    if let Some(ref server_hash) = file.md5_hash {
                        match compute_file_md5_cached(&local_file, &meta) {
                            Ok(local_hash) if local_hash != *server_hash => {
                                files_to_update += 1;
                                update_size += file.size_bytes;
                            }
                            Err(_) => {
                                files_to_update += 1;
                                update_size += file.size_bytes;
                            }
                            _ => {} // hash matches
                        }
                    }
                }
            }
            Err(_) => {
                // File doesn't exist or not accessible
                files_to_update += 1;
                update_size += file.size_bytes;
            }
        }
    }

    let status = if files_to_update == 0 {
        if verify_hashes {
            "up_to_date"
        } else {
            "checking"
        }
    } else {
        "needs_update"
    };
    (status.to_string(), files_to_update, update_size)
}

// ============================================================================
// Network Operations
// ============================================================================

async fn fetch_remote_index(
    server: &str,
    index_path: &str,
    request_ctx: &RequestContext,
) -> Result<String, ApiError> {
    let url = format!("{}/{}", server, index_path);

    // Check cache first (keyed by full URL)
    {
        let cache = INDEX_CACHE.lock().await;
        if let Some((fetched_at, ref content)) = cache.get(&url) {
            if fetched_at.elapsed() < INDEX_CACHE_TTL {
                csl_debug!(
                    "[{}] Index cache hit url={} age_secs={} bytes={}",
                    request_ctx.operation_id(),
                    url,
                    fetched_at.elapsed().as_secs(),
                    content.len()
                );
                return Ok(content.clone());
            }
        }
    }
    csl_debug!(
        "[{}] Index cache miss url={} ttl_secs={}",
        request_ctx.operation_id(),
        url,
        INDEX_CACHE_TTL.as_secs()
    );

    let client = build_http_client(std::time::Duration::from_secs(30))?;
    let request_id = request_ctx.next_http_request_id();

    csl_debug!(
        "[{}] HTTP GET start request_id={} url={} purpose=index_fetch",
        request_ctx.operation_id(),
        request_id,
        url
    );

    let resp = with_request_tracking_headers(client.get(&url), request_ctx, &request_id)
        .send()
        .await
        .map_err(|e| {
            csl_debug!(
                "[{}] HTTP GET failed request_id={} url={} purpose=index_fetch error={}",
                request_ctx.operation_id(),
                request_id,
                url,
                e
            );
            ApiError::new(
                ApiErrorCode::NetworkError,
                format!("Failed to fetch index: {}", e),
            )
        })?;

    let status = resp.status();
    if !status.is_success() {
        csl_debug!(
            "[{}] HTTP GET non-success request_id={} url={} purpose=index_fetch status={}",
            request_ctx.operation_id(),
            request_id,
            url,
            status
        );
        return Err(ApiError::new(
            ApiErrorCode::NetworkError,
            format!("Server returned status {}", status),
        ));
    }

    let bytes = resp.bytes().await.map_err(|e| {
        csl_debug!(
            "[{}] HTTP GET read failed request_id={} url={} purpose=index_fetch error={}",
            request_ctx.operation_id(),
            request_id,
            url,
            e
        );
        ApiError::new(
            ApiErrorCode::NetworkError,
            format!("Failed to read response: {}", e),
        )
    })?;

    csl_debug!(
        "[{}] HTTP GET success request_id={} url={} purpose=index_fetch status={} bytes={}",
        request_ctx.operation_id(),
        request_id,
        url,
        status,
        bytes.len()
    );

    let content = String::from_utf8_lossy(&bytes).to_string();

    // Update cache
    {
        let mut cache = INDEX_CACHE.lock().await;
        cache.insert(url.clone(), (std::time::Instant::now(), content.clone()));
    }

    csl_debug!(
        "[{}] Index cache updated url={} bytes={}",
        request_ctx.operation_id(),
        url,
        content.len()
    );

    Ok(content)
}

/// Download a single file with exponential backoff retry (max 5 attempts).
/// Delays: 1s, 2s, 4s, 8s, 16s.
async fn download_file(
    client: &reqwest::Client,
    server: &str,
    remote_path: &str,
    local_path: &Path,
    cancel_flag: &AtomicBool,
    package_name: &str,
    request_ctx: &RequestContext,
) -> Result<u64, ApiError> {
    let url = format!("{}/{}", server, remote_path);

    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ApiError::new(
                ApiErrorCode::Internal,
                format!("Failed to create directory: {}", e),
            )
        })?;
    }

    let mut last_err = None;
    let tmp_path = temp_download_path(local_path);

    for attempt in 0u32..5 {
        ensure_download_not_cancelled(cancel_flag, package_name)?;

        if attempt > 0 {
            let delay = std::time::Duration::from_secs(1 << (attempt - 1)); // 1, 2, 4, 8
            tokio::time::sleep(delay).await;
            ensure_download_not_cancelled(cancel_flag, package_name)?;
        }

        let request_id = request_ctx.next_http_request_id();
        csl_debug!(
            "[{}] HTTP GET start request_id={} url={} purpose=file_download package={} local_path={} attempt={}",
            request_ctx.operation_id(),
            request_id,
            url,
            package_name,
            local_path.display(),
            attempt + 1
        );

        let resp = match with_request_tracking_headers(client.get(&url), request_ctx, &request_id)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                csl_debug!(
                    "[{}] HTTP GET failed request_id={} url={} purpose=file_download package={} attempt={} error={}",
                    request_ctx.operation_id(),
                    request_id,
                    url,
                    package_name,
                    attempt + 1,
                    e
                );
                last_err = Some(format!("Download failed (attempt {}): {}", attempt + 1, e));
                continue;
            }
        };

        let status = resp.status();
        if !status.is_success() {
            csl_debug!(
                "[{}] HTTP GET non-success request_id={} url={} purpose=file_download package={} attempt={} status={}",
                request_ctx.operation_id(),
                request_id,
                url,
                package_name,
                attempt + 1,
                status
            );
            last_err = Some(format!(
                "Download failed with status {} (attempt {})",
                status,
                attempt + 1,
            ));
            continue;
        }

        let mut file = match std::fs::File::create(&tmp_path) {
            Ok(file) => file,
            Err(e) => {
                last_err = Some(format!(
                    "Failed to create file (attempt {}): {}",
                    attempt + 1,
                    e
                ));
                continue;
            }
        };

        let mut stream = resp.bytes_stream();
        let mut size = 0u64;
        let mut failed = false;

        while let Some(chunk) = stream.next().await {
            if let Err(err) = ensure_download_not_cancelled(cancel_flag, package_name) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(err);
            }

            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(e) => {
                    csl_debug!(
                        "[{}] HTTP GET stream failed request_id={} url={} purpose=file_download package={} attempt={} error={}",
                        request_ctx.operation_id(),
                        request_id,
                        url,
                        package_name,
                        attempt + 1,
                        e
                    );
                    last_err = Some(format!(
                        "Failed to read download (attempt {}): {}",
                        attempt + 1,
                        e,
                    ));
                    failed = true;
                    break;
                }
            };

            if let Err(e) = file.write_all(&chunk) {
                csl_debug!(
                    "[{}] Local file write failed request_id={} package={} tmp_path={} attempt={} error={}",
                    request_ctx.operation_id(),
                    request_id,
                    package_name,
                    tmp_path.display(),
                    attempt + 1,
                    e
                );
                last_err = Some(format!(
                    "Failed to write file (attempt {}): {}",
                    attempt + 1,
                    e,
                ));
                failed = true;
                break;
            }

            size += chunk.len() as u64;
        }

        if failed {
            let _ = std::fs::remove_file(&tmp_path);
            continue;
        }

        if let Err(err) = ensure_download_not_cancelled(cancel_flag, package_name) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(err);
        }

        file.flush().map_err(|e| {
            ApiError::new(
                ApiErrorCode::Internal,
                format!("Failed to flush file: {}", e),
            )
        })?;

        if local_path.exists() {
            std::fs::remove_file(local_path).map_err(|e| {
                ApiError::new(
                    ApiErrorCode::Internal,
                    format!("Failed to replace existing file: {}", e),
                )
            })?;
        }

        std::fs::rename(&tmp_path, local_path).map_err(|e| {
            ApiError::new(
                ApiErrorCode::Internal,
                format!("Failed to finalize download: {}", e),
            )
        })?;

        csl_debug!(
            "[{}] HTTP GET success request_id={} url={} purpose=file_download package={} status={} bytes={} local_path={}",
            request_ctx.operation_id(),
            request_id,
            url,
            package_name,
            status,
            size,
            local_path.display()
        );

        return Ok(size);
    }

    let _ = std::fs::remove_file(&tmp_path);

    Err(ApiError::new(
        ApiErrorCode::NetworkError,
        last_err.unwrap_or_else(|| "Download failed after 5 attempts".to_string()),
    ))
}

/// Fetch x-csl-info.info for each package, returning a map of package_name → first line (model name).
/// Uses DESC_CACHE to skip re-fetching known descriptions. Failures are silently skipped.
async fn fetch_package_descriptions(
    server: &str,
    package_names: Vec<String>,
    request_ctx: &RequestContext,
) -> HashMap<String, String> {
    // 1. Read cache — collect already-known descriptions and the list of unknowns
    let (mut result, to_fetch) = {
        let cache = DESC_CACHE.lock().await;
        let mut known = HashMap::new();
        let mut unknown = Vec::new();
        for name in &package_names {
            let cache_key = description_cache_key(server, name);
            if let Some(desc) = cache.get(&cache_key) {
                known.insert(name.clone(), desc.clone());
            } else {
                unknown.push(name.clone());
            }
        }
        (known, unknown)
    };

    csl_debug!(
        "[{}] Description fetch start total_packages={} cached={} remote_fetches={}",
        request_ctx.operation_id(),
        package_names.len(),
        result.len(),
        to_fetch.len()
    );

    if to_fetch.is_empty() {
        csl_debug!(
            "[{}] Description fetch completed from cache only total_packages={}",
            request_ctx.operation_id(),
            result.len()
        );
        return result;
    }

    // 2. Fetch only uncached descriptions
    let client = match build_http_client(std::time::Duration::from_secs(15)) {
        Ok(c) => c,
        Err(err) => {
            csl_debug!(
                "[{}] Description fetch aborted: failed to build HTTP client error={}",
                request_ctx.operation_id(),
                err
            );
            return result;
        }
    };

    let server_owned = server.to_string();
    let request_ctx_for_fetch = request_ctx.clone();
    let fetched: Vec<Option<(String, String)>> =
        stream::iter(to_fetch.into_iter().map(move |name| {
            let client = client.clone();
            let request_ctx = request_ctx_for_fetch.clone();
            let url = format!("{}/{}/x-csl-info.info", server_owned, name);
            async move {
                for attempt in 0..DESCRIPTION_FETCH_ATTEMPTS {
                    let request_id = request_ctx.next_http_request_id();
                    csl_debug!(
                        "[{}] HTTP GET start request_id={} url={} purpose=description_fetch package={} attempt={}",
                        request_ctx.operation_id(),
                        request_id,
                        url,
                        name,
                        attempt + 1
                    );

                    let resp = match with_request_tracking_headers(
                        client.get(&url),
                        &request_ctx,
                        &request_id,
                    )
                    .send()
                    .await
                    {
                        Ok(resp) => resp,
                        Err(err) => {
                            csl_debug!(
                                "[{}] HTTP GET failed request_id={} url={} purpose=description_fetch package={} attempt={} error={}",
                                request_ctx.operation_id(),
                                request_id,
                                url,
                                name,
                                attempt + 1,
                                err
                            );
                            if attempt + 1 < DESCRIPTION_FETCH_ATTEMPTS {
                                tokio::time::sleep(std::time::Duration::from_millis(
                                    300 * (attempt + 1) as u64,
                                ))
                                .await;
                                continue;
                            }
                            return None;
                        }
                    };

                    let status = resp.status();
                    if !status.is_success() {
                        csl_debug!(
                            "[{}] HTTP GET non-success request_id={} url={} purpose=description_fetch package={} attempt={} status={}",
                            request_ctx.operation_id(),
                            request_id,
                            url,
                            name,
                            attempt + 1,
                            status
                        );
                        if attempt + 1 < DESCRIPTION_FETCH_ATTEMPTS {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                300 * (attempt + 1) as u64,
                            ))
                            .await;
                            continue;
                        }
                        return None;
                    }

                    let text = match resp.text().await {
                        Ok(text) => {
                            csl_debug!(
                                "[{}] HTTP GET success request_id={} url={} purpose=description_fetch package={} status={} bytes={}",
                                request_ctx.operation_id(),
                                request_id,
                                url,
                                name,
                                status,
                                text.len()
                            );
                            text
                        }
                        Err(err) => {
                            csl_debug!(
                                "[{}] HTTP GET read failed request_id={} url={} purpose=description_fetch package={} attempt={} error={}",
                                request_ctx.operation_id(),
                                request_id,
                                url,
                                name,
                                attempt + 1,
                                err
                            );
                            if attempt + 1 < DESCRIPTION_FETCH_ATTEMPTS {
                                tokio::time::sleep(std::time::Duration::from_millis(
                                    300 * (attempt + 1) as u64,
                                ))
                                .await;
                                continue;
                            }
                            return None;
                        }
                    };

                    if let Some(first_line) =
                        text.lines().map(str::trim).find(|line| !line.is_empty())
                    {
                        csl_debug!(
                            "[{}] Description resolved package={} request_id={} value={}",
                            request_ctx.operation_id(),
                            name,
                            request_id,
                            first_line
                        );
                        return Some((name, first_line.to_string()));
                    }

                    csl_debug!(
                        "[{}] Description file empty package={} request_id={}",
                        request_ctx.operation_id(),
                        name,
                        request_id
                    );
                    return None;
                }

                None
            }
        }))
        .buffer_unordered(DESCRIPTION_FETCH_CONCURRENCY)
        .collect()
        .await;

    // 3. Write newly-fetched entries into cache and result
    let new_entries: Vec<(String, String)> = fetched.into_iter().flatten().collect();
    if !new_entries.is_empty() {
        let mut cache = DESC_CACHE.lock().await;
        for (name, desc) in &new_entries {
            cache.insert(description_cache_key(server, name), desc.clone());
        }
    }
    for (name, desc) in new_entries {
        result.insert(name, desc);
    }

    csl_debug!(
        "[{}] Description fetch completed resolved_packages={}",
        request_ctx.operation_id(),
        result.len()
    );

    result
}

// ============================================================================
// Internal (shared) logic
// ============================================================================

async fn scan_packages_internal(
    server: &str,
    paths: Vec<CslPath>,
    local_path_strings: Vec<String>,
    request_ctx: &RequestContext,
) -> Result<CslScanResult, String> {
    csl_debug!(
        "[{}] Scan internal start server={} visible_paths={} compare_paths={}",
        request_ctx.operation_id(),
        server,
        paths.len(),
        local_path_strings.len()
    );

    // Fetch remote index (single HTTP request)
    let index_content = fetch_remote_index(server, "x-csl-indexes.idx", request_ctx)
        .await
        .map_err(|e| e.to_string())?;

    let server_version = index_content.lines().next().unwrap_or("").to_string();

    // Parse and group — pure CPU, fast
    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    csl_debug!(
        "[{}] Scan index parsed server_version={} entries={} packages={}",
        request_ctx.operation_id(),
        server_version,
        entries.len(),
        pkg_data_list.len()
    );

    let package_dirs = collect_local_package_dirs(&local_path_strings);
    let compare_concurrency = comparison_parallelism(CSL_SCAN_COMPARE_CONCURRENCY_LIMIT);

    csl_debug!(
        "[{}] Scan compare prepared package_dirs={} compare_concurrency={}",
        request_ctx.operation_id(),
        package_dirs.len(),
        compare_concurrency
    );

    let compare_results: Vec<Result<CslPackageInfo, String>> =
        stream::iter(pkg_data_list.into_iter().map(|pkg| {
            let local_dir = package_dirs.get(&pkg.name).cloned();

            async move {
                if let Some(dir) = local_dir {
                    tokio::task::spawn_blocking(move || {
                        let (status, files_to_update, update_size) =
                            compare_package_quick(&pkg, &dir);
                        build_csl_package_info(&pkg, &status, files_to_update, update_size)
                    })
                    .await
                    .map_err(|e| format!("Package comparison failed: {}", e))
                } else {
                    Ok(build_csl_package_info(
                        &pkg,
                        "not_installed",
                        pkg.files.len(),
                        pkg.files.iter().map(|f| f.size_bytes).sum(),
                    ))
                }
            }
        }))
        .buffer_unordered(compare_concurrency)
        .collect()
        .await;

    let mut packages = Vec::with_capacity(compare_results.len());
    for result in compare_results {
        packages.push(result?);
    }

    // Sort by name
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    csl_debug!(
        "[{}] Scan internal completed packages={} server_version={}",
        request_ctx.operation_id(),
        packages.len(),
        server_version
    );

    Ok(CslScanResult {
        packages,
        paths,
        server_version,
    })
}

async fn install_package_internal(
    server: &str,
    event_name: &str,
    package_name: String,
    target_path: String,
    parallel_downloads: Option<usize>,
    app_handle: AppHandle,
    cancel_flag: Arc<AtomicBool>,
    request_ctx: &RequestContext,
) -> Result<(), String> {
    csl_debug!(
        "[{}] Install internal start package={} server={} target_path={} requested_parallel_downloads={:?}",
        request_ctx.operation_id(),
        package_name,
        server,
        target_path,
        parallel_downloads
    );

    let index_content = fetch_remote_index(server, "x-csl-indexes.idx", request_ctx)
        .await
        .map_err(|e| e.to_string())?;

    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    let pkg = pkg_data_list
        .iter()
        .find(|p| p.name == package_name)
        .ok_or_else(|| format!("Package {} not found in index", package_name))?;

    let target_pkg_dir = Path::new(&target_path).join(&package_name);
    let client =
        build_http_client(std::time::Duration::from_secs(60)).map_err(|e| e.to_string())?;

    csl_debug!(
        "[{}] Install package resolved package={} files_in_index={} target_dir={}",
        request_ctx.operation_id(),
        package_name,
        pkg.files.len(),
        target_pkg_dir.display()
    );

    // Determine which files need downloading (size check + MD5)
    let prefix = format!("{}/", package_name);
    let mut files_to_download: Vec<&FileEntry> = Vec::new();

    for file in &pkg.files {
        let rel_path = file.path.strip_prefix(&prefix).unwrap_or(&file.path);
        let local_file = target_pkg_dir.join(rel_path);

        let needs_download = if !local_file.exists() {
            true
        } else if let Ok(meta) = std::fs::metadata(&local_file) {
            if meta.len() != file.size_bytes {
                true
            } else if let Some(ref server_hash) = file.md5_hash {
                match compute_file_md5_cached(&local_file, &meta) {
                    Ok(local_hash) => local_hash != *server_hash,
                    Err(_) => true,
                }
            } else {
                false
            }
        } else {
            true
        };

        if needs_download {
            files_to_download.push(file);
        }
    }

    let download_total = files_to_download.len();
    let download_total_bytes: u64 = files_to_download.iter().map(|f| f.size_bytes).sum();

    let concurrency = clamp_parallel_downloads(parallel_downloads);

    csl_debug!(
        "[{}] Install plan package={} files_in_index={} files_to_download={} total_bytes={} concurrency={}",
        request_ctx.operation_id(),
        package_name,
        pkg.files.len(),
        download_total,
        download_total_bytes,
        concurrency
    );

    if concurrency <= 1 {
        // Sequential download (original behavior)
        let mut bytes_downloaded: u64 = 0;

        for (i, file) in files_to_download.iter().enumerate() {
            ensure_download_not_cancelled(cancel_flag.as_ref(), &package_name)
                .map_err(|e| e.to_string())?;

            let rel_path = file.path.strip_prefix(&prefix).unwrap_or(&file.path);
            let local_path = target_pkg_dir.join(rel_path);

            let _ = app_handle.emit(
                event_name,
                CslProgressEvent {
                    package_name: package_name.clone(),
                    current_file: i + 1,
                    total_files: download_total,
                    current_file_name: rel_path.to_string(),
                    bytes_downloaded,
                    total_bytes: download_total_bytes,
                },
            );

            let downloaded = download_file(
                &client,
                server,
                &file.path,
                &local_path,
                cancel_flag.as_ref(),
                &package_name,
                request_ctx,
            )
            .await
            .map_err(|e| e.to_string())?;

            bytes_downloaded += downloaded;
            csl_debug!(
                "[{}] Install file completed package={} file={} downloaded_bytes={} cumulative_bytes={}",
                request_ctx.operation_id(),
                package_name,
                rel_path,
                downloaded,
                bytes_downloaded
            );
        }
    } else {
        // Parallel download — collect owned data to avoid lifetime issues
        let owned_files: Vec<String> = files_to_download.iter().map(|f| f.path.clone()).collect();
        let completed = Arc::new(AtomicU64::new(0));
        let bytes_downloaded = Arc::new(AtomicU64::new(0));
        let event_name_owned = event_name.to_string();

        let results: Vec<Result<(), String>> =
            stream::iter(owned_files.into_iter().map(|file_path| {
                let client = client.clone();
                let server = server.to_string();
                let prefix = prefix.clone();
                let target_pkg_dir = target_pkg_dir.clone();
                let app_handle = app_handle.clone();
                let package_name = package_name.clone();
                let cancel_flag = cancel_flag.clone();
                let completed = completed.clone();
                let bytes_downloaded = bytes_downloaded.clone();
                let event_name = event_name_owned.clone();
                let request_ctx = request_ctx.clone();

                async move {
                    ensure_download_not_cancelled(cancel_flag.as_ref(), &package_name)
                        .map_err(|e| e.to_string())?;

                    let rel_path = file_path.strip_prefix(&prefix).unwrap_or(&file_path);
                    let local_path = target_pkg_dir.join(rel_path);

                    let downloaded = download_file(
                        &client,
                        &server,
                        &file_path,
                        &local_path,
                        cancel_flag.as_ref(),
                        &package_name,
                        &request_ctx,
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    let total_dl =
                        bytes_downloaded.fetch_add(downloaded, Ordering::Relaxed) + downloaded;

                    let _ = app_handle.emit(
                        &event_name,
                        CslProgressEvent {
                            package_name: package_name.clone(),
                            current_file: done as usize,
                            total_files: download_total,
                            current_file_name: rel_path.to_string(),
                            bytes_downloaded: total_dl,
                            total_bytes: download_total_bytes,
                        },
                    );

                    csl_debug!(
                        "[{}] Install file completed package={} file={} downloaded_bytes={} completed_files={} cumulative_bytes={}",
                        request_ctx.operation_id(),
                        package_name,
                        rel_path,
                        downloaded,
                        done,
                        total_dl
                    );

                    Ok(())
                }
            }))
            .buffer_unordered(concurrency)
            .collect()
            .await;

        // Check for any errors
        for result in results {
            result?;
        }
    }

    let _ = app_handle.emit(
        event_name,
        CslProgressEvent {
            package_name: package_name.clone(),
            current_file: download_total,
            total_files: download_total,
            current_file_name: "Complete".to_string(),
            bytes_downloaded: download_total_bytes,
            total_bytes: download_total_bytes,
        },
    );

    csl_debug!(
        "[{}] Install internal completed package={} files_downloaded={} total_bytes={}",
        request_ctx.operation_id(),
        package_name,
        download_total,
        download_total_bytes
    );

    Ok(())
}

fn uninstall_package_internal(package_name: &str, paths: &[String]) -> Result<(), String> {
    for base_path in paths {
        let pkg_dir = Path::new(base_path).join(package_name);
        if pkg_dir.exists() && pkg_dir.is_dir() {
            std::fs::remove_dir_all(&pkg_dir)
                .map_err(|e| format!("Failed to remove {}: {}", pkg_dir.display(), e))?;
            return Ok(());
        }
    }

    Err(format!(
        "Package {} not found in any CSL path",
        package_name
    ))
}

fn collect_scan_paths(xplane_path: &str, custom_paths: &[String]) -> (Vec<CslPath>, Vec<String>) {
    let xplane = Path::new(xplane_path);
    let mut paths = detect_csl_paths(xplane);
    for cp in custom_paths {
        if !paths.iter().any(|p| p.path == *cp) {
            paths.push(CslPath {
                path: cp.clone(),
                source: "custom".to_string(),
                plugin_name: None,
            });
        }
    }
    let local_path_strings: Vec<String> = paths.iter().map(|p| p.path.clone()).collect();
    (paths, local_path_strings)
}

// ============================================================================
// CSL Sync Helpers
// ============================================================================

#[cfg(windows)]
fn create_directory_link(target: &Path, link_path: &Path) -> Result<(), String> {
    junction::create(target, link_path).map_err(|e| {
        format!(
            "Failed to create junction {} -> {}: {}",
            link_path.display(),
            target.display(),
            e
        )
    })
}

#[cfg(unix)]
fn create_directory_link(target: &Path, link_path: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link_path).map_err(|e| {
        format!(
            "Failed to create symlink {} -> {}: {}",
            link_path.display(),
            target.display(),
            e
        )
    })
}

#[cfg(windows)]
fn is_link(path: &Path) -> bool {
    junction::exists(path).unwrap_or(false)
}

#[cfg(unix)]
fn is_link(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(windows)]
fn remove_directory_link(link_path: &Path) -> Result<(), String> {
    junction::delete(link_path)
        .map_err(|e| format!("Failed to remove junction {}: {}", link_path.display(), e))
}

#[cfg(unix)]
fn remove_directory_link(link_path: &Path) -> Result<(), String> {
    std::fs::remove_file(link_path)
        .map_err(|e| format!("Failed to remove symlink {}: {}", link_path.display(), e))
}

#[cfg(windows)]
fn read_directory_link_target(link_path: &Path) -> Result<PathBuf, String> {
    junction::get_target(link_path).map_err(|e| {
        format!(
            "Failed to inspect junction target {}: {}",
            link_path.display(),
            e
        )
    })
}

#[cfg(unix)]
fn read_directory_link_target(link_path: &Path) -> Result<PathBuf, String> {
    std::fs::read_link(link_path).map_err(|e| {
        format!(
            "Failed to inspect symlink target {}: {}",
            link_path.display(),
            e
        )
    })
}

fn normalize_path_for_comparison(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn directory_link_points_to(link_path: &Path, expected_target: &Path) -> Result<bool, String> {
    let actual_target = read_directory_link_target(link_path)?;
    let resolved_target = if actual_target.is_absolute() {
        actual_target
    } else {
        link_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(actual_target)
    };

    Ok(normalize_path_for_comparison(&resolved_target)
        == normalize_path_for_comparison(expected_target))
}

#[cfg(windows)]
fn volume_key(path: &Path) -> Option<String> {
    use std::path::Component;

    match path.components().next() {
        Some(Component::Prefix(prefix)) => {
            Some(prefix.as_os_str().to_string_lossy().to_ascii_lowercase())
        }
        _ => None,
    }
}

#[cfg(windows)]
fn supports_hard_links(canonical_base: &Path, target_base: &Path) -> bool {
    volume_key(canonical_base) == volume_key(target_base)
}

#[cfg(unix)]
fn existing_device_id(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;

    for ancestor in path.ancestors() {
        if let Ok(metadata) = std::fs::metadata(ancestor) {
            return Some(metadata.dev());
        }
    }

    None
}

#[cfg(unix)]
fn supports_hard_links(canonical_base: &Path, target_base: &Path) -> bool {
    match (
        existing_device_id(canonical_base),
        existing_device_id(target_base),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn is_existing_or_creatable_sync_target(path: &Path) -> bool {
    if path.is_dir() || is_link(path) {
        return true;
    }

    path.parent()
        .map(|parent| parent.is_dir() || is_link(parent))
        .unwrap_or(false)
}

fn collect_sync_targets(xplane_path: &Path, custom_paths: &[String]) -> Vec<SyncTarget> {
    let canonical = xplane_path.join(CSL_CANONICAL_REL);
    let mut targets = Vec::new();

    for (rel_path, _) in CSL_PLUGIN_PATHS {
        let full = xplane_path.join(rel_path);
        if full == canonical || !is_existing_or_creatable_sync_target(&full) {
            continue;
        }

        targets.push(SyncTarget {
            mode: if supports_hard_links(&canonical, &full) {
                SyncTargetMode::HardLinks
            } else {
                SyncTargetMode::DirectoryLinkFallback
            },
            base: full,
        });
    }

    for custom_path in custom_paths {
        let path = PathBuf::from(custom_path);
        if path == canonical || !is_existing_or_creatable_sync_target(&path) {
            continue;
        }
        if targets.iter().any(|target| target.base == path) {
            continue;
        }

        targets.push(SyncTarget {
            mode: if supports_hard_links(&canonical, &path) {
                SyncTargetMode::HardLinks
            } else {
                SyncTargetMode::DirectoryLinkFallback
            },
            base: path,
        });
    }

    targets
}

fn directory_is_empty(path: &Path) -> Result<bool, String> {
    let mut entries = std::fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory {}: {}", path.display(), e))?;
    Ok(entries.next().is_none())
}

fn remove_link_or_empty_directory(
    link_path: &Path,
    allow_empty_directory: bool,
) -> Result<bool, String> {
    if is_link(link_path) {
        remove_directory_link(link_path)?;

        if link_path.exists() && link_path.is_dir() && directory_is_empty(link_path)? {
            std::fs::remove_dir(link_path).map_err(|e| {
                format!(
                    "Failed to remove directory shell left behind after deleting link {}: {}",
                    link_path.display(),
                    e
                )
            })?;
        }

        return Ok(true);
    }

    if allow_empty_directory && link_path.is_dir() && directory_is_empty(link_path)? {
        std::fs::remove_dir(link_path).map_err(|e| {
            format!(
                "Failed to remove empty directory {}: {}",
                link_path.display(),
                e
            )
        })?;
        return Ok(true);
    }

    Ok(false)
}

fn normalize_rel_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn hardlink_state_dir(base: &Path) -> PathBuf {
    base.join(CSL_LINK_SYNC_STATE_DIR)
}

fn sanitize_manifest_file_name(package_name: &str) -> String {
    package_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn hardlink_manifest_path(base: &Path, package_name: &str) -> PathBuf {
    hardlink_state_dir(base).join(format!(
        "{}.json",
        sanitize_manifest_file_name(package_name)
    ))
}

fn read_hardlink_manifest(
    base: &Path,
    package_name: &str,
) -> Result<Option<CslHardLinkManifest>, String> {
    let manifest_path = hardlink_manifest_path(base, package_name);
    if !manifest_path.exists() {
        return Ok(None);
    }

    let bytes = std::fs::read(&manifest_path).map_err(|e| {
        format!(
            "Failed to read sync manifest {}: {}",
            manifest_path.display(),
            e
        )
    })?;
    let manifest: CslHardLinkManifest = serde_json::from_slice(&bytes).map_err(|e| {
        format!(
            "Failed to parse sync manifest {}: {}",
            manifest_path.display(),
            e
        )
    })?;

    Ok(Some(manifest))
}

fn remove_hardlink_manifest(base: &Path, package_name: &str) -> Result<(), String> {
    let manifest_path = hardlink_manifest_path(base, package_name);
    if manifest_path.exists() {
        std::fs::remove_file(&manifest_path).map_err(|e| {
            format!(
                "Failed to remove sync manifest {}: {}",
                manifest_path.display(),
                e
            )
        })?;
    }

    let state_dir = hardlink_state_dir(base);
    if state_dir.is_dir() && directory_is_empty(&state_dir)? {
        std::fs::remove_dir(&state_dir).map_err(|e| {
            format!(
                "Failed to remove empty sync state dir {}: {}",
                state_dir.display(),
                e
            )
        })?;
    }

    Ok(())
}

fn write_hardlink_manifest(
    base: &Path,
    package_name: &str,
    files: &[String],
) -> Result<(), String> {
    if files.is_empty() {
        return remove_hardlink_manifest(base, package_name);
    }

    let state_dir = hardlink_state_dir(base);
    std::fs::create_dir_all(&state_dir).map_err(|e| {
        format!(
            "Failed to create sync state dir {}: {}",
            state_dir.display(),
            e
        )
    })?;

    let manifest = CslHardLinkManifest {
        version: 1,
        package_name: package_name.to_string(),
        files: files.to_vec(),
    };

    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| {
        format!(
            "Failed to serialize sync manifest for {}: {}",
            package_name, e
        )
    })?;
    let manifest_path = hardlink_manifest_path(base, package_name);
    std::fs::write(&manifest_path, bytes).map_err(|e| {
        format!(
            "Failed to write sync manifest {}: {}",
            manifest_path.display(),
            e
        )
    })
}

fn list_manifest_packages(base: &Path) -> Result<Vec<String>, String> {
    let state_dir = hardlink_state_dir(base);
    if !state_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut packages = Vec::new();
    for entry in std::fs::read_dir(&state_dir).map_err(|e| {
        format!(
            "Failed to read sync state dir {}: {}",
            state_dir.display(),
            e
        )
    })? {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read sync state entry in {}: {}",
                state_dir.display(),
                e
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let manifest: CslHardLinkManifest = match serde_json::from_slice(&bytes) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        if !manifest.package_name.trim().is_empty() {
            packages.push(manifest.package_name);
        }
    }

    packages.sort();
    packages.dedup();
    Ok(packages)
}

fn collect_package_files(package_dir: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(package_dir) {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to walk package directory {}: {}",
                package_dir.display(),
                e
            )
        })?;
        if !entry.file_type().is_file() {
            continue;
        }

        let rel_path = entry.path().strip_prefix(package_dir).map_err(|e| {
            format!(
                "Failed to strip package path prefix {} from {}: {}",
                package_dir.display(),
                entry.path().display(),
                e
            )
        })?;
        files.push(normalize_rel_path(rel_path));
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn remove_file_if_exists(path: &Path) -> Result<bool, String> {
    match path.symlink_metadata() {
        Ok(metadata) => {
            if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
                return Err(format!(
                    "Cannot replace {} because a directory exists where a file is expected",
                    path.display()
                ));
            }

            std::fs::remove_file(path)
                .map_err(|e| format!("Failed to remove file {}: {}", path.display(), e))?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("Failed to inspect {}: {}", path.display(), e)),
    }
}

fn files_match_canonical(source_path: &Path, target_path: &Path) -> Result<bool, String> {
    let source_meta = std::fs::metadata(source_path).map_err(|e| {
        format!(
            "Failed to inspect canonical file {}: {}",
            source_path.display(),
            e
        )
    })?;
    let target_meta = std::fs::metadata(target_path).map_err(|e| {
        format!(
            "Failed to inspect target file {}: {}",
            target_path.display(),
            e
        )
    })?;

    if !source_meta.is_file() || !target_meta.is_file() {
        return Ok(false);
    }

    if source_meta.len() != target_meta.len() {
        return Ok(false);
    }

    let source_hash = compute_file_md5_cached(source_path, &source_meta).map_err(|e| {
        format!(
            "Failed to hash canonical file {}: {}",
            source_path.display(),
            e
        )
    })?;
    let target_hash = compute_file_md5_cached(target_path, &target_meta).map_err(|e| {
        format!(
            "Failed to hash target file {}: {}",
            target_path.display(),
            e
        )
    })?;

    Ok(source_hash == target_hash)
}

fn files_match_canonical_fast(source_path: &Path, target_path: &Path) -> Result<bool, String> {
    let source_meta = std::fs::metadata(source_path).map_err(|e| {
        format!(
            "Failed to inspect canonical file {}: {}",
            source_path.display(),
            e
        )
    })?;
    let target_meta = std::fs::metadata(target_path).map_err(|e| {
        format!(
            "Failed to inspect target file {}: {}",
            target_path.display(),
            e
        )
    })?;

    if !source_meta.is_file() || !target_meta.is_file() {
        return Ok(false);
    }

    Ok(source_meta.len() == target_meta.len()
        && mtime_secs(&source_meta) == mtime_secs(&target_meta))
}

fn remove_empty_parent_dirs(start_path: &Path, stop_at: &Path) -> Result<(), String> {
    let mut current = start_path.parent();

    while let Some(dir) = current {
        if dir == stop_at || !dir.starts_with(stop_at) {
            break;
        }

        if !dir.is_dir() {
            break;
        }

        if directory_is_empty(dir)? {
            std::fs::remove_dir(dir).map_err(|e| {
                format!("Failed to remove empty directory {}: {}", dir.display(), e)
            })?;
            current = dir.parent();
        } else {
            break;
        }
    }

    Ok(())
}

fn cleanup_hardlink_managed_files(
    base: &Path,
    package_name: &str,
    warnings: &mut Vec<String>,
) -> usize {
    let manifest = match read_hardlink_manifest(base, package_name) {
        Ok(manifest) => manifest,
        Err(err) => {
            warnings.push(err);
            None
        }
    };

    let Some(manifest) = manifest else {
        return 0;
    };

    let package_root = base.join(package_name);
    let managed_files: std::collections::BTreeSet<String> = manifest.files.into_iter().collect();
    let mut removed = 0usize;

    for rel_path in managed_files {
        let target_file = package_root.join(&rel_path);
        if let Err(err) = remove_file_if_exists(&target_file) {
            warnings.push(err);
        }
        if let Err(err) = remove_empty_parent_dirs(&target_file, &package_root) {
            warnings.push(err);
        }
        removed += 1;
    }

    if package_root.is_dir() {
        match directory_is_empty(&package_root) {
            Ok(true) => {
                if let Err(err) = std::fs::remove_dir(&package_root) {
                    warnings.push(format!(
                        "Failed to remove empty managed package directory {}: {}",
                        package_root.display(),
                        err
                    ));
                }
            }
            Ok(false) => {}
            Err(err) => warnings.push(err),
        }
    }

    if let Err(err) = remove_hardlink_manifest(base, package_name) {
        warnings.push(err);
    }

    removed
}

fn cleanup_target_package(
    base: &Path,
    package_name: &str,
    allow_empty_directory: bool,
    warnings: &mut Vec<String>,
) {
    let package_path = base.join(package_name);

    if let Err(err) = remove_link_or_empty_directory(&package_path, allow_empty_directory) {
        warnings.push(err);
    }

    cleanup_hardlink_managed_files(base, package_name, warnings);

    if allow_empty_directory && package_path.is_dir() {
        match directory_is_empty(&package_path) {
            Ok(true) => {
                if let Err(err) = std::fs::remove_dir(&package_path) {
                    warnings.push(format!(
                        "Failed to remove empty package directory {}: {}",
                        package_path.display(),
                        err
                    ));
                }
            }
            Ok(false) => {}
            Err(err) => warnings.push(err),
        }
    }
}

fn sync_directory_link_package(
    base: &Path,
    package_name: &str,
    canonical_pkg_dir: &Path,
    mode: LinkSyncMode,
    warnings: &mut Vec<String>,
) {
    let package_path = base.join(package_name);

    if is_link(&package_path) {
        match directory_link_points_to(&package_path, canonical_pkg_dir) {
            Ok(true) => return,
            Ok(false) => {
                if let Err(err) = remove_directory_link(&package_path) {
                    warnings.push(err);
                    return;
                }
            }
            Err(err) => {
                warnings.push(err);
                if let Err(remove_err) = remove_directory_link(&package_path) {
                    warnings.push(remove_err);
                    return;
                }
            }
        }
    } else if package_path.exists() {
        if package_path.is_dir() {
            match directory_is_empty(&package_path) {
                Ok(true) => {
                    if let Err(err) = std::fs::remove_dir(&package_path) {
                        warnings.push(format!(
                            "Failed to remove empty directory {} before creating directory link: {}",
                            package_path.display(),
                            err
                        ));
                        return;
                    }
                }
                Ok(false) => {
                    if mode == LinkSyncMode::Reconcile {
                        warnings.push(format!(
                            "Skipped {}: non-empty real directory exists (directory-link fallback cannot replace it)",
                            package_path.display()
                        ));
                    }
                    return;
                }
                Err(err) => {
                    warnings.push(err);
                    return;
                }
            }
        } else {
            warnings.push(format!(
                "Skipped {}: file exists and cannot be replaced with a directory link",
                package_path.display()
            ));
            return;
        }
    }

    cleanup_hardlink_managed_files(base, package_name, warnings);

    if let Some(parent) = package_path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            warnings.push(format!(
                "Failed to create parent directory {}: {}",
                parent.display(),
                err
            ));
            return;
        }
    }

    if let Err(err) = create_directory_link(canonical_pkg_dir, &package_path) {
        warnings.push(err);
    }
}

fn sync_hardlink_package(
    base: &Path,
    package_name: &str,
    canonical_pkg_dir: &Path,
    current_files: &[String],
    mode: LinkSyncMode,
    tracker: &mut LinkSyncProgressTracker,
    warnings: &mut Vec<String>,
) {
    let package_path = base.join(package_name);

    if let Err(err) = std::fs::create_dir_all(base) {
        warnings.push(format!(
            "Failed to create target CSL directory {}: {}",
            base.display(),
            err
        ));
        if mode == LinkSyncMode::Reconcile {
            tracker.advance_by(current_files.len().max(1), package_name, None);
        }
        return;
    }

    if is_link(&package_path) {
        if mode == LinkSyncMode::MissingOnly {
            match directory_link_points_to(&package_path, canonical_pkg_dir) {
                Ok(true) => return,
                Ok(false) | Err(_) => {
                    if let Err(err) = remove_link_or_empty_directory(&package_path, true) {
                        warnings.push(err);
                        return;
                    }
                }
            }
        } else if let Err(err) = remove_link_or_empty_directory(&package_path, true) {
            warnings.push(err);
            tracker.advance_by(current_files.len().max(1), package_name, None);
            return;
        }
    } else if package_path.exists() && !package_path.is_dir() {
        warnings.push(format!(
            "Skipped {}: file exists and cannot be replaced with managed hard links",
            package_path.display()
        ));
        if mode == LinkSyncMode::Reconcile {
            tracker.advance_by(current_files.len().max(1), package_name, None);
        }
        return;
    }

    if let Err(err) = std::fs::create_dir_all(&package_path) {
        warnings.push(format!(
            "Failed to create package directory {}: {}",
            package_path.display(),
            err
        ));
        if mode == LinkSyncMode::Reconcile {
            tracker.advance_by(current_files.len().max(1), package_name, None);
        }
        return;
    }

    let previous_manifest = match read_hardlink_manifest(base, package_name) {
        Ok(manifest) => manifest,
        Err(err) => {
            warnings.push(err);
            None
        }
    };
    let previous_files: std::collections::HashSet<String> = previous_manifest
        .as_ref()
        .map(|manifest| manifest.files.iter().cloned().collect())
        .unwrap_or_default();
    let current_file_set: std::collections::HashSet<String> =
        current_files.iter().cloned().collect();

    if mode == LinkSyncMode::Reconcile {
        for stale_rel_path in previous_files.difference(&current_file_set) {
            let stale_target = package_path.join(stale_rel_path);
            if let Err(err) = remove_file_if_exists(&stale_target) {
                warnings.push(err);
            }
            if let Err(err) = remove_empty_parent_dirs(&stale_target, &package_path) {
                warnings.push(err);
            }
        }
    }

    let mut next_manifest_files: std::collections::BTreeSet<String> =
        if mode == LinkSyncMode::MissingOnly {
            previous_files.iter().cloned().collect()
        } else {
            std::collections::BTreeSet::new()
        };

    for rel_path in current_files {
        let source_path = canonical_pkg_dir.join(rel_path);
        let target_path = package_path.join(rel_path);
        let was_managed = previous_files.contains(rel_path);

        let target_exists = match target_path.symlink_metadata() {
            Ok(_) => true,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
            Err(err) => {
                warnings.push(format!(
                    "Failed to inspect target file {}: {}",
                    target_path.display(),
                    err
                ));
                tracker.advance_file(rel_path);
                continue;
            }
        };

        if target_exists {
            if mode == LinkSyncMode::MissingOnly {
                match files_match_canonical_fast(&source_path, &target_path) {
                    Ok(true) => {
                        next_manifest_files.insert(rel_path.clone());
                        continue;
                    }
                    Ok(false) if was_managed => {
                        if let Err(err) = remove_file_if_exists(&target_path) {
                            warnings.push(err);
                            continue;
                        }
                    }
                    Ok(false) => {
                        warnings.push(format!(
                            "Skipped {}: user file already exists and will be preserved",
                            target_path.display()
                        ));
                        continue;
                    }
                    Err(err) if was_managed => {
                        warnings.push(err);
                        if let Err(remove_err) = remove_file_if_exists(&target_path) {
                            warnings.push(remove_err);
                            continue;
                        }
                    }
                    Err(err) => {
                        warnings.push(err);
                        continue;
                    }
                }
            } else {
                match files_match_canonical(&source_path, &target_path) {
                    Ok(true) => {
                        next_manifest_files.insert(rel_path.clone());
                        tracker.advance_file(rel_path);
                        continue;
                    }
                    Ok(false) if was_managed => {
                        if let Err(err) = remove_file_if_exists(&target_path) {
                            warnings.push(err);
                            tracker.advance_file(rel_path);
                            continue;
                        }
                    }
                    Ok(false) => {
                        warnings.push(format!(
                            "Skipped {}: user file already exists and will be preserved",
                            target_path.display()
                        ));
                        tracker.advance_file(rel_path);
                        continue;
                    }
                    Err(err) if was_managed => {
                        warnings.push(err);
                        if let Err(remove_err) = remove_file_if_exists(&target_path) {
                            warnings.push(remove_err);
                            tracker.advance_file(rel_path);
                            continue;
                        }
                    }
                    Err(err) => {
                        warnings.push(err);
                        tracker.advance_file(rel_path);
                        continue;
                    }
                }
            }
        }

        if let Some(parent) = target_path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                warnings.push(format!(
                    "Failed to create parent directory {}: {}",
                    parent.display(),
                    err
                ));
                tracker.advance_file(rel_path);
                continue;
            }
        }

        match std::fs::hard_link(&source_path, &target_path) {
            Ok(()) => {
                next_manifest_files.insert(rel_path.clone());
            }
            Err(err) => warnings.push(format!(
                "Failed to create hard link {} -> {}: {}",
                target_path.display(),
                source_path.display(),
                err
            )),
        }

        tracker.advance_file(rel_path);
    }

    let next_manifest_files: Vec<String> = next_manifest_files.into_iter().collect();
    if let Err(err) = write_hardlink_manifest(base, package_name, &next_manifest_files) {
        warnings.push(err);
    }

    if package_path.is_dir() {
        match directory_is_empty(&package_path) {
            Ok(true) => {
                if let Err(err) = std::fs::remove_dir(&package_path) {
                    warnings.push(format!(
                        "Failed to remove empty package directory {}: {}",
                        package_path.display(),
                        err
                    ));
                }
            }
            Ok(false) => {}
            Err(err) => warnings.push(err),
        }
    }
}

fn collect_cleanup_package_names(
    cleanup_targets: &[PathBuf],
    canonical_packages: &[String],
) -> Result<Vec<String>, String> {
    let mut package_names: std::collections::BTreeSet<String> =
        canonical_packages.iter().cloned().collect();

    for target in cleanup_targets {
        for package_name in list_manifest_packages(target)? {
            package_names.insert(package_name);
        }

        let entries = match std::fs::read_dir(target) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries {
            let entry = entry.map_err(|e| {
                format!(
                    "Failed to read cleanup target entry {}: {}",
                    target.display(),
                    e
                )
            })?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name == CSL_LINK_SYNC_STATE_DIR {
                continue;
            }

            if is_link(&path) || (path.is_dir() && directory_is_empty(&path).unwrap_or(false)) {
                package_names.insert(name.to_string());
            }
        }
    }

    Ok(package_names.into_iter().collect())
}

fn list_installed_canonical_packages(canonical_base: &Path) -> Result<Vec<String>, String> {
    if !canonical_base.exists() {
        return Ok(Vec::new());
    }

    let mut packages = Vec::new();
    let entries = std::fs::read_dir(canonical_base).map_err(|e| {
        format!(
            "Failed to read canonical CSL dir {}: {}",
            canonical_base.display(),
            e
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read CSL directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                packages.push(name.to_string());
            }
        }
    }

    packages.sort();
    packages.dedup();
    Ok(packages)
}

fn count_files_under(path: &Path) -> usize {
    if !path.exists() {
        return 0;
    }

    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .count()
}

fn estimate_missing_only_directory_link_units(
    target_base: &Path,
    package_name: &str,
    canonical_pkg_dir: &Path,
) -> usize {
    let package_path = target_base.join(package_name);

    if is_link(&package_path) {
        return match directory_link_points_to(&package_path, canonical_pkg_dir) {
            Ok(true) => 0,
            Ok(false) | Err(_) => 1,
        };
    }

    if !package_path.exists() {
        return 1;
    }

    if package_path.is_dir() {
        return match directory_is_empty(&package_path) {
            Ok(true) => 1,
            Ok(false) | Err(_) => 0,
        };
    }

    0
}

fn estimate_missing_only_hardlink_units(
    target_base: &Path,
    package_name: &str,
    canonical_pkg_dir: &Path,
    current_files: &[String],
) -> usize {
    let package_path = target_base.join(package_name);

    if is_link(&package_path) {
        return match directory_link_points_to(&package_path, canonical_pkg_dir) {
            Ok(true) => 0,
            Ok(false) | Err(_) => current_files.len().max(1),
        };
    }

    let previous_files: std::collections::HashSet<String> =
        read_hardlink_manifest(target_base, package_name)
            .ok()
            .flatten()
            .map(|manifest| manifest.files.into_iter().collect())
            .unwrap_or_default();

    let mut units = 0usize;

    for rel_path in current_files {
        let source_path = canonical_pkg_dir.join(rel_path);
        let target_path = package_path.join(rel_path);
        let was_managed = previous_files.contains(rel_path);

        match target_path.symlink_metadata() {
            Ok(metadata) => {
                if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
                    if was_managed {
                        units += 1;
                    }
                    continue;
                }

                match files_match_canonical_fast(&source_path, &target_path) {
                    Ok(true) => {}
                    Ok(false) => {
                        if was_managed {
                            units += 1;
                        }
                    }
                    Err(_) => {
                        if was_managed {
                            units += 1;
                        }
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                units += 1;
            }
            Err(_) => {
                if was_managed {
                    units += 1;
                }
            }
        }
    }

    units
}

fn estimate_target_package_units(
    target_base: &Path,
    package_name: &str,
    canonical_file_count: Option<usize>,
) -> usize {
    if let Ok(Some(manifest)) = read_hardlink_manifest(target_base, package_name) {
        if !manifest.files.is_empty() {
            return manifest.files.len();
        }
    }

    if let Some(count) = canonical_file_count {
        if count > 0 {
            return count;
        }
    }

    let package_path = target_base.join(package_name);
    if is_link(&package_path) {
        return canonical_file_count.unwrap_or(1).max(1);
    }

    let discovered_files = count_files_under(&package_path);
    if discovered_files > 0 {
        discovered_files
    } else if package_path.exists() {
        1
    } else {
        0
    }
}

fn sync_package_links_internal(
    app_handle: Option<AppHandle>,
    request_ctx: &RequestContext,
    xplane_path: &str,
    custom_paths: &[String],
    package_names: Option<&[String]>,
    target_paths: Option<&[String]>,
    cleanup_paths: Option<&[String]>,
) -> Result<Vec<String>, String> {
    let xplane = Path::new(xplane_path);
    let canonical_base = xplane.join(CSL_CANONICAL_REL);
    let sync_mode = if package_names.is_some() {
        LinkSyncMode::Reconcile
    } else {
        LinkSyncMode::MissingOnly
    };
    let mut active_targets = collect_sync_targets(xplane, custom_paths);

    if let Some(target_paths) = target_paths {
        let allowed_targets: std::collections::HashSet<PathBuf> = target_paths
            .iter()
            .map(|path| PathBuf::from(path.trim()))
            .filter(|path| !path.as_os_str().is_empty())
            .collect();
        active_targets.retain(|target| allowed_targets.contains(&target.base));
    }

    let packages = match package_names {
        Some(names) => {
            let mut names = names.to_vec();
            names.sort();
            names.dedup();
            names
        }
        None => {
            if !canonical_base.exists() {
                Vec::new()
            } else {
                list_installed_canonical_packages(&canonical_base)?
            }
        }
    };

    let mut canonical_files_by_package: HashMap<String, Vec<String>> = HashMap::new();
    for package_name in &packages {
        let canonical_pkg_dir = canonical_base.join(package_name);
        if canonical_pkg_dir.is_dir() {
            canonical_files_by_package.insert(
                package_name.clone(),
                collect_package_files(&canonical_pkg_dir)?,
            );
        }
    }

    let cleanup_targets: Vec<PathBuf> = cleanup_paths
        .unwrap_or(&[])
        .iter()
        .map(PathBuf::from)
        .filter(|path| *path != canonical_base)
        .collect();
    let cleanup_package_names = if cleanup_targets.is_empty() {
        Vec::new()
    } else if package_names.is_some() {
        packages.clone()
    } else {
        collect_cleanup_package_names(&cleanup_targets, &packages)?
    };

    let total_targets = active_targets.len() + cleanup_targets.len();
    let mut total_files = 0usize;

    for target in &cleanup_targets {
        for package_name in &cleanup_package_names {
            total_files += estimate_target_package_units(
                target,
                package_name,
                canonical_files_by_package.get(package_name).map(Vec::len),
            );
        }
    }

    for target in &active_targets {
        for package_name in &packages {
            if let Some(files) = canonical_files_by_package.get(package_name) {
                let canonical_pkg_dir = canonical_base.join(package_name);
                total_files += match sync_mode {
                    LinkSyncMode::MissingOnly => match target.mode {
                        SyncTargetMode::HardLinks => estimate_missing_only_hardlink_units(
                            &target.base,
                            package_name,
                            &canonical_pkg_dir,
                            files,
                        ),
                        SyncTargetMode::DirectoryLinkFallback => {
                            estimate_missing_only_directory_link_units(
                                &target.base,
                                package_name,
                                &canonical_pkg_dir,
                            )
                        }
                    },
                    LinkSyncMode::Reconcile => files.len().max(1),
                };
            } else if package_names.is_some() {
                total_files += estimate_target_package_units(&target.base, package_name, None);
            }
        }
    }

    let mut tracker = LinkSyncProgressTracker::new(
        app_handle,
        request_ctx.operation_id().to_string(),
        total_files,
        total_targets,
    );
    tracker.emit_preparing();

    let sync_result: Result<Vec<String>, String> = (|| {
        let mut warnings = Vec::new();

        for target in &cleanup_targets {
            tracker.begin_target(target, None);
            for package_name in &cleanup_package_names {
                tracker.begin_package(package_name);
                let units = estimate_target_package_units(
                    target,
                    package_name,
                    canonical_files_by_package.get(package_name).map(Vec::len),
                );
                cleanup_target_package(target, package_name, true, &mut warnings);
                if units > 0 {
                    tracker.advance_by(units, package_name, None);
                }
            }
            tracker.finish_target();
        }

        for target in &active_targets {
            let fallback_message = if target.mode == SyncTargetMode::DirectoryLinkFallback {
                Some(format!(
                    "Path {} is on a different volume. Falling back to directory links.",
                    target.base.display()
                ))
            } else {
                None
            };
            tracker.begin_target(&target.base, fallback_message);

            for package_name in &packages {
                tracker.begin_package(package_name);
                if let Some(current_files) = canonical_files_by_package.get(package_name) {
                    let canonical_pkg_dir = canonical_base.join(package_name);
                    match target.mode {
                        SyncTargetMode::HardLinks => sync_hardlink_package(
                            &target.base,
                            package_name,
                            &canonical_pkg_dir,
                            current_files,
                            sync_mode,
                            &mut tracker,
                            &mut warnings,
                        ),
                        SyncTargetMode::DirectoryLinkFallback => {
                            let units = if sync_mode == LinkSyncMode::MissingOnly {
                                estimate_missing_only_directory_link_units(
                                    &target.base,
                                    package_name,
                                    &canonical_pkg_dir,
                                )
                            } else {
                                current_files.len().max(1)
                            };
                            sync_directory_link_package(
                                &target.base,
                                package_name,
                                &canonical_pkg_dir,
                                sync_mode,
                                &mut warnings,
                            );
                            if units > 0 {
                                tracker.advance_by(units, package_name, None);
                            }
                        }
                    }
                } else if package_names.is_some() {
                    let units = estimate_target_package_units(&target.base, package_name, None);
                    cleanup_target_package(&target.base, package_name, false, &mut warnings);
                    if units > 0 {
                        tracker.advance_by(units, package_name, None);
                    }
                }
            }

            tracker.finish_target();
        }

        Ok(warnings)
    })();

    match &sync_result {
        Ok(_) => tracker.complete(),
        Err(err) => tracker.fail(err.clone()),
    }

    sync_result
}

// ============================================================================
// Tauri Commands — CSL
// ============================================================================

#[tauri::command]
pub async fn csl_fetch_package_descriptions(
    package_names: Vec<String>,
    server_base_url: Option<String>,
    request_id: Option<String>,
) -> Result<HashMap<String, String>, String> {
    let request_ctx = RequestContext::new("csl_fetch_package_descriptions", request_id);

    if package_names.is_empty() {
        csl_debug!(
            "[{}] Description fetch skipped because package list is empty",
            request_ctx.operation_id()
        );
        return Ok(HashMap::new());
    }

    let api_base = resolve_csl_api_base(server_base_url.as_deref());
    csl_debug!(
        "[{}] Description fetch command start packages={} server={}",
        request_ctx.operation_id(),
        package_names.len(),
        api_base
    );

    let result = fetch_package_descriptions(&api_base, package_names, &request_ctx).await;

    csl_debug!(
        "[{}] Description fetch command completed resolved_packages={}",
        request_ctx.operation_id(),
        result.len()
    );

    Ok(result)
}

/// Scan packages: fetch remote index, compare with local, return results.
///
/// Optimizations vs. naive approach:
/// 1. No per-package HTTP requests for descriptions — uses package name instead
/// 2. First pass is size-only and returns `checking` for packages awaiting exact hash verification
/// 3. Package directory discovery is done once per scan instead of probing every package name
/// 4. Comparison concurrency is capped to keep first-open scans responsive
#[tauri::command]
pub async fn csl_scan_packages(
    xplane_path: String,
    custom_paths: Vec<String>,
    server_base_url: Option<String>,
    request_id: Option<String>,
) -> Result<CslScanResult, String> {
    let request_ctx = RequestContext::new("csl_scan_packages", request_id);
    let (paths, _) = collect_scan_paths(&xplane_path, &custom_paths);

    // Scan only from canonical path for local comparison
    let xplane = Path::new(&xplane_path);
    let canonical = xplane.join(CSL_CANONICAL_REL);
    let scan_paths = if canonical.exists() {
        vec![canonical.to_string_lossy().to_string()]
    } else {
        vec![]
    };

    let api_base = resolve_csl_api_base(server_base_url.as_deref());
    csl_debug!(
        "[{}] CSL scan command start xplane_path={} custom_paths={} detected_paths={} scan_paths={} server={}",
        request_ctx.operation_id(),
        xplane_path,
        custom_paths.len(),
        paths.len(),
        scan_paths.len(),
        api_base
    );

    let result = scan_packages_internal(&api_base, paths, scan_paths, &request_ctx).await;

    match &result {
        Ok(scan_result) => csl_debug!(
            "[{}] CSL scan command completed packages={} paths={} server_version={}",
            request_ctx.operation_id(),
            scan_result.packages.len(),
            scan_result.paths.len(),
            scan_result.server_version
        ),
        Err(err) => csl_debug!(
            "[{}] CSL scan command failed error={}",
            request_ctx.operation_id(),
            err
        ),
    }

    result
}

/// Rescan specific packages only (uses cached index).
/// Much faster than a full scan — skips description fetches and only compares
/// the requested packages against the canonical CSL directory.
#[tauri::command]
pub async fn csl_rescan_packages(
    xplane_path: String,
    package_names: Vec<String>,
    server_base_url: Option<String>,
    request_id: Option<String>,
) -> Result<Vec<CslPackageInfo>, String> {
    let request_ctx = RequestContext::new("csl_rescan_packages", request_id);
    let xplane = Path::new(&xplane_path);
    let canonical = xplane.join(CSL_CANONICAL_REL);
    let scan_paths = vec![canonical.to_string_lossy().to_string()];
    let server_base_url = resolve_server_base_url(server_base_url.as_deref());

    csl_debug!(
        "[{}] CSL rescan command start xplane_path={} package_count={} server={}",
        request_ctx.operation_id(),
        xplane_path,
        package_names.len(),
        server_base_url
    );

    // Fetch index (hits cache if recent scan happened)
    let index_content = fetch_remote_index(&server_base_url, CSL_INDEX_PATH, &request_ctx)
        .await
        .map_err(|e| e.to_string())?;

    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    // Filter to only requested packages
    let target_set: std::collections::HashSet<&str> =
        package_names.iter().map(|s| s.as_str()).collect();

    let package_dirs = collect_local_package_dirs(&scan_paths);
    let compare_concurrency = comparison_parallelism(CSL_RESCAN_COMPARE_CONCURRENCY_LIMIT);

    csl_debug!(
        "[{}] CSL rescan compare prepared package_dirs={} compare_concurrency={}",
        request_ctx.operation_id(),
        package_dirs.len(),
        compare_concurrency
    );

    let compare_results: Vec<Result<CslPackageInfo, String>> = stream::iter(
        pkg_data_list
            .into_iter()
            .filter_map(|pkg| {
                if target_set.contains(pkg.name.as_str()) {
                    Some(pkg)
                } else {
                    None
                }
            })
            .map(|pkg| {
                let local_dir = package_dirs.get(&pkg.name).cloned();

                async move {
                    if let Some(dir) = local_dir {
                        tokio::task::spawn_blocking(move || {
                            let (status, files_to_update, update_size) =
                                compare_package_exact(&pkg, &dir);
                            build_csl_package_info(&pkg, &status, files_to_update, update_size)
                        })
                        .await
                        .map_err(|e| format!("Package comparison failed: {}", e))
                    } else {
                        Ok(build_csl_package_info(
                            &pkg,
                            "not_installed",
                            pkg.files.len(),
                            pkg.files.iter().map(|f| f.size_bytes).sum(),
                        ))
                    }
                }
            }),
    )
    .buffer_unordered(compare_concurrency)
    .collect()
    .await;

    let mut results = Vec::with_capacity(compare_results.len());
    for result in compare_results {
        results.push(result?);
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));

    csl_debug!(
        "[{}] CSL rescan command completed packages={}",
        request_ctx.operation_id(),
        results.len()
    );

    Ok(results)
}

/// Install or update a specific CSL package.
/// Downloads to the canonical path (Resources/plugins/IVAO_CSL/CSL) and then
/// syncs the package to all other detected CSL directories.
#[tauri::command]
pub async fn csl_install_package(
    package_name: String,
    xplane_path: String,
    custom_paths: Vec<String>,
    parallel_downloads: Option<usize>,
    server_base_url: Option<String>,
    request_id: Option<String>,
    app_handle: AppHandle,
    download_control: State<'_, CslDownloadControl>,
) -> Result<(), String> {
    let request_ctx = RequestContext::new("csl_install_package", request_id);
    let task_key = install_task_key("csl", &package_name);
    let _registration = download_control
        .register(task_key)
        .map_err(|e| e.to_string())?;
    let cancel_flag = _registration.cancel_flag();

    let xplane = Path::new(&xplane_path);
    let canonical_base = xplane.join(CSL_CANONICAL_REL);
    let api_base = resolve_csl_api_base(server_base_url.as_deref());
    csl_debug!(
        "[{}] CSL install command start package={} xplane_path={} custom_paths={} target={} server={} requested_parallel_downloads={:?}",
        request_ctx.operation_id(),
        package_name,
        xplane_path,
        custom_paths.len(),
        canonical_base.display(),
        api_base,
        parallel_downloads
    );

    std::fs::create_dir_all(&canonical_base)
        .map_err(|e| format!("Failed to create canonical CSL dir: {}", e))?;

    let canonical_base_str = canonical_base.to_string_lossy().to_string();
    let install_result = install_package_internal(
        &api_base,
        "csl-progress",
        package_name.clone(),
        canonical_base_str,
        parallel_downloads,
        app_handle.clone(),
        cancel_flag,
        &request_ctx,
    )
    .await;

    if let Err(err) = install_result {
        csl_debug!(
            "[{}] CSL install command failed during download package={} error={}",
            request_ctx.operation_id(),
            package_name,
            err
        );
        return Err(err);
    }

    let warnings = sync_package_links_internal(
        Some(app_handle.clone()),
        &request_ctx,
        &xplane_path,
        &custom_paths,
        Some(std::slice::from_ref(&package_name)),
        None,
        None,
    )?;

    csl_debug!(
        "[{}] CSL install command completed package={} link_warnings={}",
        request_ctx.operation_id(),
        package_name,
        warnings.len()
    );

    Ok(())
}

#[tauri::command]
pub async fn csl_cancel_install(
    source: String,
    package_name: String,
    request_id: Option<String>,
    download_control: State<'_, CslDownloadControl>,
) -> Result<(), String> {
    let request_ctx = RequestContext::new("csl_cancel_install", request_id);
    let normalized_source = match source.as_str() {
        "altitude" => "altitude",
        _ => "csl",
    };

    csl_debug!(
        "[{}] Cancel install command start source={} package={}",
        request_ctx.operation_id(),
        normalized_source,
        package_name
    );
    download_control.cancel(&install_task_key(normalized_source, &package_name));
    csl_debug!(
        "[{}] Cancel install command completed source={} package={}",
        request_ctx.operation_id(),
        normalized_source,
        package_name
    );
    Ok(())
}

/// Uninstall a CSL package.
/// Removes links from other plugin directories first, then deletes the canonical copy.
#[tauri::command]
pub async fn csl_uninstall_package(
    package_name: String,
    xplane_path: String,
    custom_paths: Vec<String>,
    request_id: Option<String>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let request_ctx = RequestContext::new("csl_uninstall_package", request_id);
    let xplane = Path::new(&xplane_path);
    let active_targets = collect_sync_targets(xplane, &custom_paths);

    csl_debug!(
        "[{}] CSL uninstall command start package={} xplane_path={} custom_paths={} active_targets={}",
        request_ctx.operation_id(),
        package_name,
        xplane_path,
        custom_paths.len(),
        active_targets.len()
    );

    // Remove the canonical copy
    let canonical_pkg_dir = xplane.join(CSL_CANONICAL_REL).join(&package_name);
    if canonical_pkg_dir.exists() && canonical_pkg_dir.is_dir() {
        std::fs::remove_dir_all(&canonical_pkg_dir)
            .map_err(|e| format!("Failed to remove {}: {}", canonical_pkg_dir.display(), e))?;

        let warnings = sync_package_links_internal(
            Some(app_handle.clone()),
            &request_ctx,
            &xplane_path,
            &custom_paths,
            Some(std::slice::from_ref(&package_name)),
            None,
            None,
        )?;
        csl_debug!(
            "[{}] CSL uninstall command completed package={} removed_path={} sync_warnings={}",
            request_ctx.operation_id(),
            package_name,
            canonical_pkg_dir.display(),
            warnings.len()
        );
        return Ok(());
    }

    let warnings = sync_package_links_internal(
        Some(app_handle.clone()),
        &request_ctx,
        &xplane_path,
        &custom_paths,
        Some(std::slice::from_ref(&package_name)),
        None,
        None,
    )?;
    if !warnings.is_empty() {
        csl_debug!(
            "[{}] CSL uninstall post-sync warnings package={} warnings={}",
            request_ctx.operation_id(),
            package_name,
            warnings.len()
        );
    }

    // Fallback: try old paths for backward compatibility
    let (_, all_paths) = collect_scan_paths(&xplane_path, &custom_paths);
    let result = uninstall_package_internal(&package_name, &all_paths);

    match &result {
        Ok(()) => csl_debug!(
            "[{}] CSL uninstall fallback completed package={} searched_paths={} sync_warnings={}",
            request_ctx.operation_id(),
            package_name,
            all_paths.len(),
            warnings.len()
        ),
        Err(err) => csl_debug!(
            "[{}] CSL uninstall fallback failed package={} error={} sync_warnings={}",
            request_ctx.operation_id(),
            package_name,
            err,
            warnings.len()
        ),
    }

    result
}

/// Detect CSL paths from known plugin directories
#[tauri::command]
pub async fn csl_detect_paths(xplane_path: String) -> Result<Vec<CslPath>, String> {
    Ok(detect_csl_paths(Path::new(&xplane_path)))
}

#[tauri::command]
pub async fn csl_sync_links(
    xplane_path: String,
    custom_paths: Vec<String>,
    package_names: Option<Vec<String>>,
    target_paths: Option<Vec<String>>,
    cleanup_paths: Option<Vec<String>>,
    request_id: Option<String>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let request_ctx = RequestContext::new("csl_sync_links", request_id);
    csl_debug!(
        "[{}] CSL link sync command start xplane_path={} custom_paths={} package_count={} target_paths={} cleanup_paths={}",
        request_ctx.operation_id(),
        xplane_path,
        custom_paths.len(),
        package_names.as_ref().map(|names| names.len()).unwrap_or(0),
        target_paths.as_ref().map(|paths| paths.len()).unwrap_or(0),
        cleanup_paths.as_ref().map(|paths| paths.len()).unwrap_or(0)
    );

    let result = sync_package_links_internal(
        Some(app_handle),
        &request_ctx,
        &xplane_path,
        &custom_paths,
        package_names.as_deref(),
        target_paths.as_deref(),
        cleanup_paths.as_deref(),
    );

    match &result {
        Ok(warnings) => {
            csl_debug!(
                "[{}] CSL link sync command completed warnings={}",
                request_ctx.operation_id(),
                warnings.len()
            );
            for warning in warnings.iter().take(10) {
                csl_debug!(
                    "[{}] CSL link sync warning: {}",
                    request_ctx.operation_id(),
                    warning
                );
            }
        }
        Err(err) => csl_debug!(
            "[{}] CSL link sync command failed error={}",
            request_ctx.operation_id(),
            err
        ),
    }

    result.map(|_| ())
}

// ============================================================================
// ALTITUDE — folder mapping & helpers
// ============================================================================

/// Folder mappings from client-config.ini [folders] section.
/// Maps remote prefix (e.g. "PilotUI") to local relative path.
const ALTITUDE_FOLDER_MAPPINGS: &[(&str, &str)] = &[
    ("PilotUI", "Resources/plugins/ivao_pilot/PilotUI/data"),
    ("Resources", "Resources/plugins/IVAO_CSL"),
];

/// Default local directory for ALTITUDE files that don't match any folder mapping.
const ALTITUDE_DEFAULT_LOCAL_DIR: &str = "Resources/plugins/IVAO_CSL/CSL";

/// Whether an ALTITUDE file entry is metadata (not downloadable content).
/// These exist in the index but may not be served by the CDN.
fn is_altitude_metadata(file_path: &str) -> bool {
    let file_rel = file_path.strip_prefix("ALTITUDE/").unwrap_or(file_path);
    file_rel == "x-csl-info.info" || file_rel.ends_with(".idx")
}

/// Resolve a file's local path using the ALTITUDE folder mapping.
/// `file_rel` is the path after stripping the `ALTITUDE/` prefix,
/// e.g. "PilotUI/mtlList.xml" or "x-csl-info.info".
fn altitude_resolve_local_path(xplane: &Path, file_rel: &str) -> PathBuf {
    for (prefix, local_dir) in ALTITUDE_FOLDER_MAPPINGS {
        if let Some(rest) = file_rel
            .strip_prefix(prefix)
            .and_then(|r| r.strip_prefix('/'))
        {
            return xplane.join(local_dir).join(rest);
        }
    }
    // No mapping matched — use default CSL dir
    xplane
        .join(ALTITUDE_DEFAULT_LOCAL_DIR)
        .join("ALTITUDE")
        .join(file_rel)
}

/// Compare ALTITUDE files against their mapped local paths.
fn compare_altitude_files(files: &[FileEntry], xplane: &Path) -> (String, usize, u64) {
    let pkg_prefix = "ALTITUDE/";
    let mut files_to_update = 0usize;
    let mut update_size: u64 = 0;
    let mut content_file_count = 0usize;

    for file in files {
        if is_altitude_metadata(&file.path) {
            continue;
        }
        content_file_count += 1;

        let file_rel = file.path.strip_prefix(pkg_prefix).unwrap_or(&file.path);
        let local_file = altitude_resolve_local_path(xplane, file_rel);

        match std::fs::metadata(&local_file) {
            Ok(meta) => {
                if meta.len() != file.size_bytes {
                    files_to_update += 1;
                    update_size += file.size_bytes;
                    continue;
                }
                if let Some(ref server_hash) = file.md5_hash {
                    match compute_file_md5_cached(&local_file, &meta) {
                        Ok(local_hash) if local_hash != *server_hash => {
                            files_to_update += 1;
                            update_size += file.size_bytes;
                        }
                        Err(_) => {
                            files_to_update += 1;
                            update_size += file.size_bytes;
                        }
                        _ => {}
                    }
                }
            }
            Err(_) => {
                files_to_update += 1;
                update_size += file.size_bytes;
            }
        }
    }

    let status = if files_to_update == 0 {
        "up_to_date"
    } else if files_to_update == content_file_count {
        "not_installed"
    } else {
        "needs_update"
    };
    (status.to_string(), files_to_update, update_size)
}

// ============================================================================
// Tauri Commands — ALTITUDE
// ============================================================================

/// Scan ALTITUDE supplementary package
#[tauri::command]
pub async fn altitude_scan_packages(
    xplane_path: String,
    server_base_url: Option<String>,
    request_id: Option<String>,
) -> Result<CslScanResult, String> {
    let request_ctx = RequestContext::new("altitude_scan_packages", request_id);
    let xplane = PathBuf::from(&xplane_path);
    let server_base_url = resolve_server_base_url(server_base_url.as_deref());

    csl_debug!(
        "[{}] ALTITUDE scan command start xplane_path={} server={}",
        request_ctx.operation_id(),
        xplane_path,
        server_base_url
    );

    let index_content = fetch_remote_index(&server_base_url, ALTITUDE_INDEX_PATH, &request_ctx)
        .await
        .map_err(|e| e.to_string())?;

    let server_version = index_content.lines().next().unwrap_or("").to_string();
    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    let mut packages = Vec::new();
    for pkg in &pkg_data_list {
        let content_files: Vec<&FileEntry> = pkg
            .files
            .iter()
            .filter(|f| !is_altitude_metadata(&f.path))
            .collect();

        let (status, files_to_update, update_size) = compare_altitude_files(&pkg.files, &xplane);

        let total_size = if pkg.header_size > 0 {
            pkg.header_size
        } else {
            content_files.iter().map(|f| f.size_bytes).sum()
        };

        let last_updated = if !pkg.header_date.is_empty() {
            format!("{} {}", pkg.header_date, pkg.header_time)
        } else {
            String::new()
        };

        packages.push(CslPackageInfo {
            name: pkg.name.clone(),
            total_size_bytes: total_size,
            file_count: content_files.len(),
            description: "IVAO Altitude resources".to_string(),
            status,
            files_to_update,
            update_size_bytes: update_size,
            last_updated,
        });
    }

    let result = CslScanResult {
        packages,
        paths: vec![],
        server_version,
    };

    csl_debug!(
        "[{}] ALTITUDE scan command completed packages={} server_version={}",
        request_ctx.operation_id(),
        result.packages.len(),
        result.server_version
    );

    Ok(result)
}

/// Install or update the ALTITUDE supplementary package
#[tauri::command]
pub async fn altitude_install_package(
    xplane_path: String,
    parallel_downloads: Option<usize>,
    server_base_url: Option<String>,
    request_id: Option<String>,
    app_handle: AppHandle,
    download_control: State<'_, CslDownloadControl>,
) -> Result<(), String> {
    let request_ctx = RequestContext::new("altitude_install_package", request_id);
    let xplane = PathBuf::from(&xplane_path);
    let package_name = "ALTITUDE".to_string();
    let resolved_server_base_url = resolve_server_base_url(server_base_url.as_deref());
    let api_base = resolve_altitude_api_base(server_base_url.as_deref());
    let task_key = install_task_key("altitude", &package_name);
    let _registration = download_control
        .register(task_key)
        .map_err(|e| e.to_string())?;
    let cancel_flag = _registration.cancel_flag();

    csl_debug!(
        "[{}] ALTITUDE install command start xplane_path={} server={} api_base={} requested_parallel_downloads={:?}",
        request_ctx.operation_id(),
        xplane_path,
        resolved_server_base_url,
        api_base,
        parallel_downloads
    );

    let index_content =
        fetch_remote_index(&resolved_server_base_url, ALTITUDE_INDEX_PATH, &request_ctx)
            .await
            .map_err(|e| e.to_string())?;

    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    let pkg = pkg_data_list
        .iter()
        .find(|p| p.name == package_name)
        .ok_or("ALTITUDE package not found in index")?;

    let client =
        build_http_client(std::time::Duration::from_secs(60)).map_err(|e| e.to_string())?;

    let pkg_prefix = "ALTITUDE/";

    // Determine which files need downloading
    let mut files_to_download: Vec<(&FileEntry, PathBuf)> = Vec::new();

    for file in &pkg.files {
        if is_altitude_metadata(&file.path) {
            continue;
        }

        let file_rel = file.path.strip_prefix(pkg_prefix).unwrap_or(&file.path);
        let local_file = altitude_resolve_local_path(&xplane, file_rel);

        let needs_download = match std::fs::metadata(&local_file) {
            Ok(meta) => {
                if meta.len() != file.size_bytes {
                    true
                } else if let Some(ref server_hash) = file.md5_hash {
                    match compute_file_md5_cached(&local_file, &meta) {
                        Ok(local_hash) => local_hash != *server_hash,
                        Err(_) => true,
                    }
                } else {
                    false
                }
            }
            Err(_) => true,
        };

        if needs_download {
            files_to_download.push((file, local_file));
        }
    }

    let download_total = files_to_download.len();
    let download_total_bytes: u64 = files_to_download.iter().map(|(f, _)| f.size_bytes).sum();
    let concurrency = clamp_parallel_downloads(parallel_downloads);

    csl_debug!(
        "[{}] ALTITUDE install plan files_in_index={} files_to_download={} total_bytes={} concurrency={}",
        request_ctx.operation_id(),
        pkg.files.len(),
        download_total,
        download_total_bytes,
        concurrency
    );

    if concurrency <= 1 {
        let mut bytes_downloaded: u64 = 0;
        for (i, (file, local_path)) in files_to_download.iter().enumerate() {
            ensure_download_not_cancelled(cancel_flag.as_ref(), &package_name)
                .map_err(|e| e.to_string())?;

            let display_name = file.path.strip_prefix(pkg_prefix).unwrap_or(&file.path);
            let _ = app_handle.emit(
                "altitude-progress",
                CslProgressEvent {
                    package_name: package_name.clone(),
                    current_file: i + 1,
                    total_files: download_total,
                    current_file_name: display_name.to_string(),
                    bytes_downloaded,
                    total_bytes: download_total_bytes,
                },
            );
            let downloaded = download_file(
                &client,
                &api_base,
                &file.path,
                local_path,
                cancel_flag.as_ref(),
                &package_name,
                &request_ctx,
            )
            .await
            .map_err(|e| e.to_string())?;
            bytes_downloaded += downloaded;
            csl_debug!(
                "[{}] ALTITUDE file completed file={} downloaded_bytes={} cumulative_bytes={}",
                request_ctx.operation_id(),
                display_name,
                downloaded,
                bytes_downloaded
            );
        }
    } else {
        let owned: Vec<(String, PathBuf)> = files_to_download
            .iter()
            .map(|(f, lp)| (f.path.clone(), lp.clone()))
            .collect();
        let completed = Arc::new(AtomicU64::new(0));
        let bytes_downloaded = Arc::new(AtomicU64::new(0));
        let pkg_prefix_owned = pkg_prefix.to_string();

        let results: Vec<Result<(), String>> =
            stream::iter(owned.into_iter().map(|(remote_path, local_path)| {
                let client = client.clone();
                let app_handle = app_handle.clone();
                let package_name = package_name.clone();
                let api_base = api_base.clone();
                let cancel_flag = cancel_flag.clone();
                let completed = completed.clone();
                let bytes_downloaded = bytes_downloaded.clone();
                let pkg_prefix = pkg_prefix_owned.clone();
                let request_ctx = request_ctx.clone();

                async move {
                    ensure_download_not_cancelled(cancel_flag.as_ref(), &package_name)
                        .map_err(|e| e.to_string())?;

                    let downloaded = download_file(
                        &client,
                        &api_base,
                        &remote_path,
                        &local_path,
                        cancel_flag.as_ref(),
                        &package_name,
                        &request_ctx,
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    let total_dl =
                        bytes_downloaded.fetch_add(downloaded, Ordering::Relaxed) + downloaded;
                    let display_name = remote_path
                        .strip_prefix(&pkg_prefix)
                        .unwrap_or(&remote_path);

                    let _ = app_handle.emit(
                        "altitude-progress",
                        CslProgressEvent {
                            package_name: package_name.clone(),
                            current_file: done as usize,
                            total_files: download_total,
                            current_file_name: display_name.to_string(),
                            bytes_downloaded: total_dl,
                            total_bytes: download_total_bytes,
                        },
                    );
                    csl_debug!(
                        "[{}] ALTITUDE file completed file={} downloaded_bytes={} completed_files={} cumulative_bytes={}",
                        request_ctx.operation_id(),
                        display_name,
                        downloaded,
                        done,
                        total_dl
                    );
                    Ok(())
                }
            }))
            .buffer_unordered(concurrency)
            .collect()
            .await;

        for result in results {
            result?;
        }
    }

    let _ = app_handle.emit(
        "altitude-progress",
        CslProgressEvent {
            package_name: package_name.clone(),
            current_file: download_total,
            total_files: download_total,
            current_file_name: "Complete".to_string(),
            bytes_downloaded: download_total_bytes,
            total_bytes: download_total_bytes,
        },
    );

    csl_debug!(
        "[{}] ALTITUDE install command completed files_downloaded={} total_bytes={}",
        request_ctx.operation_id(),
        download_total,
        download_total_bytes
    );

    Ok(())
}

/// Uninstall the ALTITUDE supplementary package
#[tauri::command]
pub async fn altitude_uninstall_package(
    xplane_path: String,
    server_base_url: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    let request_ctx = RequestContext::new("altitude_uninstall_package", request_id);
    let xplane = Path::new(&xplane_path);
    let server_base_url = resolve_server_base_url(server_base_url.as_deref());

    csl_debug!(
        "[{}] ALTITUDE uninstall command start xplane_path={} server={}",
        request_ctx.operation_id(),
        xplane_path,
        server_base_url
    );

    let index_content = fetch_remote_index(&server_base_url, ALTITUDE_INDEX_PATH, &request_ctx)
        .await
        .map_err(|e| e.to_string())?;

    let entries = parse_index(&index_content);
    let pkg_prefix = "ALTITUDE/";
    let mut removed = 0;

    // Remove individual files at their mapped locations
    for entry in &entries {
        if entry.entry_type != 10 {
            continue;
        }
        let file_rel = entry.path.strip_prefix(pkg_prefix).unwrap_or(&entry.path);
        let local_file = altitude_resolve_local_path(xplane, file_rel);
        if local_file.exists() {
            let _ = std::fs::remove_file(&local_file);
            removed += 1;
        }
    }

    if removed == 0 {
        csl_debug!(
            "[{}] ALTITUDE uninstall command failed because no local files were removed",
            request_ctx.operation_id()
        );
        return Err("ALTITUDE package not found locally".to_string());
    }

    csl_debug!(
        "[{}] ALTITUDE uninstall command completed removed_files={}",
        request_ctx.operation_id(),
        removed
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_package_data(name: &str, files: Vec<(&str, u64, Option<&str>)>) -> PackageData {
        PackageData {
            name: name.to_string(),
            header_size: 0,
            header_date: String::new(),
            header_time: String::new(),
            files: files
                .into_iter()
                .map(|(path, size_bytes, md5_hash)| FileEntry {
                    path: path.to_string(),
                    size_bytes,
                    md5_hash: md5_hash.map(str::to_string),
                })
                .collect(),
        }
    }

    fn write_test_file(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn clamp_parallel_downloads_limits_to_twelve() {
        assert_eq!(clamp_parallel_downloads(None), 1);
        assert_eq!(clamp_parallel_downloads(Some(0)), 1);
        assert_eq!(clamp_parallel_downloads(Some(8)), 8);
        assert_eq!(clamp_parallel_downloads(Some(12)), 12);
        assert_eq!(clamp_parallel_downloads(Some(64)), 12);
    }

    #[test]
    fn install_task_key_includes_source() {
        assert_eq!(install_task_key("csl", "B738"), "csl:B738");
        assert_eq!(
            install_task_key("altitude", "ALTITUDE"),
            "altitude:ALTITUDE"
        );
    }

    #[test]
    fn resolve_server_urls_trim_and_default() {
        assert_eq!(resolve_server_base_url(None), DEFAULT_SERVER_BASE_URL);
        assert_eq!(
            resolve_server_base_url(Some("https://example.com///")),
            "https://example.com"
        );
        assert_eq!(
            resolve_csl_api_base(Some("https://example.com/")),
            "https://example.com/package"
        );
    }

    #[test]
    fn compare_package_quick_marks_size_matched_package_as_checking() {
        let temp_dir = tempfile::tempdir().unwrap();
        let package_dir = temp_dir.path().join("B738");
        write_test_file(&package_dir.join("model.obj"), b"abcd");

        let pkg = make_test_package_data("B738", vec![("B738/model.obj", 4, Some("mismatch"))]);
        let (status, files_to_update, update_size) = compare_package_quick(&pkg, &package_dir);

        assert_eq!(status, "checking");
        assert_eq!(files_to_update, 0);
        assert_eq!(update_size, 0);
    }

    #[test]
    fn compare_package_quick_detects_size_mismatch_without_hashing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let package_dir = temp_dir.path().join("B738");
        write_test_file(&package_dir.join("model.obj"), b"abcd");

        let pkg = make_test_package_data("B738", vec![("B738/model.obj", 5, Some("mismatch"))]);
        let (status, files_to_update, update_size) = compare_package_quick(&pkg, &package_dir);

        assert_eq!(status, "needs_update");
        assert_eq!(files_to_update, 1);
        assert_eq!(update_size, 5);
    }

    #[test]
    fn compare_package_exact_detects_hash_mismatch_after_quick_pass() {
        let temp_dir = tempfile::tempdir().unwrap();
        let package_dir = temp_dir.path().join("B738");
        write_test_file(&package_dir.join("model.obj"), b"abcd");

        let pkg = make_test_package_data("B738", vec![("B738/model.obj", 4, Some("mismatch"))]);
        let (status, files_to_update, update_size) = compare_package_exact(&pkg, &package_dir);

        assert_eq!(status, "needs_update");
        assert_eq!(files_to_update, 1);
        assert_eq!(update_size, 4);
    }

    #[test]
    fn compare_package_exact_marks_hash_matched_package_as_up_to_date() {
        let temp_dir = tempfile::tempdir().unwrap();
        let package_dir = temp_dir.path().join("B738");
        write_test_file(&package_dir.join("model.obj"), b"abcd");

        let expected_hash = format!("{:x}", md5::compute(b"abcd"));
        let pkg = make_test_package_data("B738", vec![("B738/model.obj", 4, Some(&expected_hash))]);
        let (status, files_to_update, update_size) = compare_package_exact(&pkg, &package_dir);

        assert_eq!(status, "up_to_date");
        assert_eq!(files_to_update, 0);
        assert_eq!(update_size, 0);
    }

    #[test]
    fn collect_local_package_dirs_enumerates_directories_once() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path().join("CSL");
        std::fs::create_dir_all(base_path.join("B738")).unwrap();
        std::fs::create_dir_all(base_path.join("A320")).unwrap();
        write_test_file(&base_path.join("readme.txt"), b"ignore");

        let dirs = collect_local_package_dirs(&[base_path.to_string_lossy().to_string()]);

        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs.get("B738"), Some(&base_path.join("B738")));
        assert_eq!(dirs.get("A320"), Some(&base_path.join("A320")));
        assert!(!dirs.contains_key("readme.txt"));
    }

    #[test]
    fn sync_hardlink_package_creates_managed_hardlinks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let canonical_pkg_dir = temp_dir.path().join("canonical").join("A19N");
        let target_base = temp_dir.path().join("xpilot");
        let current_files = vec![
            "xsb_aircraft.txt".to_string(),
            "objects/model.obj".to_string(),
        ];
        let mut tracker = LinkSyncProgressTracker::new(None, "test-sync".to_string(), 2, 1);
        let mut warnings = Vec::new();

        write_test_file(&canonical_pkg_dir.join("xsb_aircraft.txt"), b"hello");
        write_test_file(&canonical_pkg_dir.join("objects/model.obj"), b"world");

        sync_hardlink_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &current_files,
            LinkSyncMode::Reconcile,
            &mut tracker,
            &mut warnings,
        );

        assert!(warnings.is_empty());
        let linked_file = target_base.join("A19N/xsb_aircraft.txt");
        assert!(linked_file.is_file());
        std::fs::write(canonical_pkg_dir.join("xsb_aircraft.txt"), b"updated").unwrap();
        assert_eq!(std::fs::read(&linked_file).unwrap(), b"updated");

        let manifest = read_hardlink_manifest(&target_base, "A19N")
            .unwrap()
            .expect("manifest should exist");
        assert_eq!(manifest.files.len(), 2);
    }

    #[test]
    fn sync_hardlink_package_preserves_existing_user_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let canonical_pkg_dir = temp_dir.path().join("canonical").join("A19N");
        let target_base = temp_dir.path().join("xpilot");
        let existing_file = target_base.join("A19N/xsb_aircraft.txt");
        let current_files = vec!["xsb_aircraft.txt".to_string()];
        let mut tracker = LinkSyncProgressTracker::new(None, "test-sync".to_string(), 1, 1);
        let mut warnings = Vec::new();

        write_test_file(&canonical_pkg_dir.join("xsb_aircraft.txt"), b"hello");
        write_test_file(&existing_file, b"user-data");

        sync_hardlink_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &current_files,
            LinkSyncMode::MissingOnly,
            &mut tracker,
            &mut warnings,
        );

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("user file already exists"));
        assert_eq!(std::fs::read(&existing_file).unwrap(), b"user-data");
        assert!(read_hardlink_manifest(&target_base, "A19N")
            .unwrap()
            .is_none());
    }

    #[test]
    fn cleanup_target_package_removes_manifest_managed_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let canonical_pkg_dir = temp_dir.path().join("canonical").join("A19N");
        let target_base = temp_dir.path().join("xpilot");
        let current_files = vec![
            "xsb_aircraft.txt".to_string(),
            "objects/model.obj".to_string(),
        ];
        let mut tracker = LinkSyncProgressTracker::new(None, "test-sync".to_string(), 2, 1);
        let mut warnings = Vec::new();

        write_test_file(&canonical_pkg_dir.join("xsb_aircraft.txt"), b"hello");
        write_test_file(&canonical_pkg_dir.join("objects/model.obj"), b"world");
        sync_hardlink_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &current_files,
            LinkSyncMode::Reconcile,
            &mut tracker,
            &mut warnings,
        );
        assert!(warnings.is_empty());

        cleanup_target_package(&target_base, "A19N", true, &mut warnings);

        assert!(warnings.is_empty());
        assert!(!target_base.join("A19N/xsb_aircraft.txt").exists());
        assert!(!target_base.join("A19N").exists());
        assert!(read_hardlink_manifest(&target_base, "A19N")
            .unwrap()
            .is_none());
    }

    #[test]
    fn sync_hardlink_package_removes_stale_managed_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let canonical_pkg_dir = temp_dir.path().join("canonical").join("A19N");
        let target_base = temp_dir.path().join("xpilot");
        let initial_files = vec!["xsb_aircraft.txt".to_string(), "old.obj".to_string()];
        let mut tracker = LinkSyncProgressTracker::new(None, "test-sync".to_string(), 2, 1);
        let mut warnings = Vec::new();

        write_test_file(&canonical_pkg_dir.join("xsb_aircraft.txt"), b"hello");
        write_test_file(&canonical_pkg_dir.join("old.obj"), b"old");
        sync_hardlink_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &initial_files,
            LinkSyncMode::Reconcile,
            &mut tracker,
            &mut warnings,
        );
        assert!(warnings.is_empty());

        std::fs::remove_file(canonical_pkg_dir.join("old.obj")).unwrap();
        let next_files = vec!["xsb_aircraft.txt".to_string()];
        let mut tracker = LinkSyncProgressTracker::new(None, "test-sync-2".to_string(), 1, 1);
        sync_hardlink_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &next_files,
            LinkSyncMode::Reconcile,
            &mut tracker,
            &mut warnings,
        );

        assert!(warnings.is_empty());
        assert!(!target_base.join("A19N/old.obj").exists());
        let manifest = read_hardlink_manifest(&target_base, "A19N")
            .unwrap()
            .expect("manifest should still exist");
        assert_eq!(manifest.files, vec!["xsb_aircraft.txt".to_string()]);
    }

    #[test]
    fn sync_hardlink_package_missing_only_keeps_stale_managed_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let canonical_pkg_dir = temp_dir.path().join("canonical").join("A19N");
        let target_base = temp_dir.path().join("xpilot");
        let initial_files = vec!["xsb_aircraft.txt".to_string(), "old.obj".to_string()];
        let mut tracker = LinkSyncProgressTracker::new(None, "test-sync".to_string(), 2, 1);
        let mut warnings = Vec::new();

        write_test_file(&canonical_pkg_dir.join("xsb_aircraft.txt"), b"hello");
        write_test_file(&canonical_pkg_dir.join("old.obj"), b"old");
        sync_hardlink_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &initial_files,
            LinkSyncMode::Reconcile,
            &mut tracker,
            &mut warnings,
        );
        assert!(warnings.is_empty());

        std::fs::remove_file(canonical_pkg_dir.join("old.obj")).unwrap();
        let next_files = vec!["xsb_aircraft.txt".to_string()];
        let mut tracker = LinkSyncProgressTracker::new(None, "test-sync-2".to_string(), 1, 1);
        sync_hardlink_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &next_files,
            LinkSyncMode::MissingOnly,
            &mut tracker,
            &mut warnings,
        );

        assert!(warnings.is_empty());
        assert!(target_base.join("A19N/old.obj").exists());
        let manifest = read_hardlink_manifest(&target_base, "A19N")
            .unwrap()
            .expect("manifest should still exist");
        assert_eq!(
            manifest.files,
            vec!["old.obj".to_string(), "xsb_aircraft.txt".to_string()]
        );
    }

    #[test]
    fn sync_hardlink_package_missing_only_skips_healthy_managed_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let canonical_pkg_dir = temp_dir.path().join("canonical").join("A19N");
        let target_base = temp_dir.path().join("xpilot");
        let current_files = vec![
            "xsb_aircraft.txt".to_string(),
            "objects/model.obj".to_string(),
        ];
        let mut tracker = LinkSyncProgressTracker::new(None, "test-sync".to_string(), 2, 1);
        let mut warnings = Vec::new();

        write_test_file(&canonical_pkg_dir.join("xsb_aircraft.txt"), b"hello");
        write_test_file(&canonical_pkg_dir.join("objects/model.obj"), b"world");
        sync_hardlink_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &current_files,
            LinkSyncMode::Reconcile,
            &mut tracker,
            &mut warnings,
        );
        assert!(warnings.is_empty());

        let planned_units = estimate_missing_only_hardlink_units(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &current_files,
        );
        assert_eq!(planned_units, 0);

        let mut tracker = LinkSyncProgressTracker::new(None, "test-sync-2".to_string(), 0, 1);
        sync_hardlink_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            &current_files,
            LinkSyncMode::MissingOnly,
            &mut tracker,
            &mut warnings,
        );

        assert!(warnings.is_empty());
        assert_eq!(tracker.processed_files, 0);
    }

    #[test]
    fn sync_directory_link_package_replaces_empty_real_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let canonical_pkg_dir = temp_dir.path().join("canonical").join("A19N");
        let target_base = temp_dir.path().join("xpilot");
        let existing_dir = target_base.join("A19N");
        let mut warnings = Vec::new();

        write_test_file(&canonical_pkg_dir.join("xsb_aircraft.txt"), b"hello");
        std::fs::create_dir_all(&existing_dir).unwrap();

        sync_directory_link_package(
            &target_base,
            "A19N",
            &canonical_pkg_dir,
            LinkSyncMode::Reconcile,
            &mut warnings,
        );

        assert!(warnings.is_empty());
        assert!(is_link(&existing_dir));
    }

    #[test]
    fn sync_package_links_creates_file_links_when_parent_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let xplane_path = temp_dir.path();
        let canonical_pkg_dir = xplane_path.join(CSL_CANONICAL_REL).join("A19N");
        let xpilot_resources_dir = xplane_path.join("Resources/plugins/xPilot/Resources/CSL");
        let linked_file = xpilot_resources_dir.join("A19N/xsb_aircraft.txt");

        write_test_file(&canonical_pkg_dir.join("xsb_aircraft.txt"), b"hello");
        std::fs::create_dir_all(&xpilot_resources_dir).unwrap();
        let request_ctx =
            RequestContext::new("test_sync_links", Some("test-sync-links".to_string()));

        let warnings = sync_package_links_internal(
            None,
            &request_ctx,
            &xplane_path.to_string_lossy(),
            &[],
            None,
            None,
            None,
        )
        .unwrap();

        assert!(warnings.is_empty());
        assert!(linked_file.is_file());
        std::fs::write(canonical_pkg_dir.join("xsb_aircraft.txt"), b"updated").unwrap();
        assert_eq!(std::fs::read(&linked_file).unwrap(), b"updated");
    }

    #[test]
    fn sync_package_links_target_paths_only_sync_requested_targets() {
        let temp_dir = tempfile::tempdir().unwrap();
        let xplane_path = temp_dir.path();
        let canonical_pkg_dir = xplane_path.join(CSL_CANONICAL_REL).join("A19N");
        let auto_target = xplane_path.join("Resources/plugins/xPilot/Resources/CSL");
        let custom_target = temp_dir.path().join("custom-csl");
        let custom_target_str = custom_target.to_string_lossy().to_string();

        write_test_file(&canonical_pkg_dir.join("xsb_aircraft.txt"), b"hello");
        std::fs::create_dir_all(&auto_target).unwrap();
        std::fs::create_dir_all(&custom_target).unwrap();
        let request_ctx = RequestContext::new(
            "test_sync_links",
            Some("test-sync-links-target".to_string()),
        );
        let no_targets: Vec<String> = Vec::new();

        let warnings = sync_package_links_internal(
            None,
            &request_ctx,
            &xplane_path.to_string_lossy(),
            std::slice::from_ref(&custom_target_str),
            None,
            Some(&no_targets),
            None,
        )
        .unwrap();
        assert!(warnings.is_empty());
        assert!(!auto_target.join("A19N/xsb_aircraft.txt").exists());
        assert!(!custom_target.join("A19N/xsb_aircraft.txt").exists());

        let target_filter = vec![custom_target_str.clone()];
        let warnings = sync_package_links_internal(
            None,
            &request_ctx,
            &xplane_path.to_string_lossy(),
            std::slice::from_ref(&custom_target_str),
            None,
            Some(&target_filter),
            None,
        )
        .unwrap();

        assert!(warnings.is_empty());
        assert!(!auto_target.join("A19N/xsb_aircraft.txt").exists());
        assert!(custom_target.join("A19N/xsb_aircraft.txt").is_file());
    }
}

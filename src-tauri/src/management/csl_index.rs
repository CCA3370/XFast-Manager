use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::error::{ApiError, ApiErrorCode};

const DEFAULT_SERVER_BASE_URL: &str = "http://x-csl.ru";
const CSL_CANONICAL_REL: &str = "Resources/plugins/IVAO_CSL/CSL";
const CSL_API_BASE_PATH: &str = "package";
const CSL_INDEX_PATH: &str = "package/x-csl-indexes.idx";
const ALTITUDE_INDEX_PATH: &str = "package/ALTITUDE/files.idx";
const MAX_CSL_PARALLEL_DOWNLOADS: usize = 12;
const DESCRIPTION_FETCH_CONCURRENCY: usize = 6;
const DESCRIPTION_FETCH_ATTEMPTS: u32 = 3;

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

/// Find package directory across local paths
fn find_local_package_dir(package_name: &str, local_paths: &[String]) -> Option<PathBuf> {
    for base_path in local_paths {
        let pkg_dir = Path::new(base_path).join(package_name);
        if pkg_dir.is_dir() {
            return Some(pkg_dir);
        }
    }
    None
}

/// Compare a single package against local files.
/// Optimization: size check first, MD5 only when size matches.
/// Uses MD5_CACHE to skip re-hashing unchanged files.
fn compare_package_fast(pkg: &PackageData, local_dir: &Path) -> (String, usize, u64) {
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
            Err(_) => {
                // File doesn't exist or not accessible
                files_to_update += 1;
                update_size += file.size_bytes;
            }
        }
    }

    let status = if files_to_update == 0 {
        "up_to_date"
    } else {
        "needs_update"
    };
    (status.to_string(), files_to_update, update_size)
}

// ============================================================================
// Network Operations
// ============================================================================

async fn fetch_remote_index(server: &str, index_path: &str) -> Result<String, ApiError> {
    let url = format!("{}/{}", server, index_path);

    // Check cache first (keyed by full URL)
    {
        let cache = INDEX_CACHE.lock().await;
        if let Some((fetched_at, ref content)) = cache.get(&url) {
            if fetched_at.elapsed() < INDEX_CACHE_TTL {
                return Ok(content.clone());
            }
        }
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("XFast Manager")
        .build()
        .map_err(|e| {
            ApiError::new(
                ApiErrorCode::NetworkError,
                format!("HTTP client error: {}", e),
            )
        })?;

    let resp = client.get(&url).send().await.map_err(|e| {
        ApiError::new(
            ApiErrorCode::NetworkError,
            format!("Failed to fetch index: {}", e),
        )
    })?;

    if !resp.status().is_success() {
        return Err(ApiError::new(
            ApiErrorCode::NetworkError,
            format!("Server returned status {}", resp.status()),
        ));
    }

    let bytes = resp.bytes().await.map_err(|e| {
        ApiError::new(
            ApiErrorCode::NetworkError,
            format!("Failed to read response: {}", e),
        )
    })?;

    let content = String::from_utf8_lossy(&bytes).to_string();

    // Update cache
    {
        let mut cache = INDEX_CACHE.lock().await;
        cache.insert(url, (std::time::Instant::now(), content.clone()));
    }

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

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(format!("Download failed (attempt {}): {}", attempt + 1, e));
                continue;
            }
        };

        if !resp.status().is_success() {
            last_err = Some(format!(
                "Download failed with status {} (attempt {})",
                resp.status(),
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

    if to_fetch.is_empty() {
        return result;
    }

    // 2. Fetch only uncached descriptions
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("XFast Manager")
        .build()
    {
        Ok(c) => c,
        Err(_) => return result,
    };

    let server_owned = server.to_string();
    let fetched: Vec<Option<(String, String)>> =
        stream::iter(to_fetch.into_iter().map(move |name| {
            let client = client.clone();
            let url = format!("{}/{}/x-csl-info.info", server_owned, name);
            async move {
                for attempt in 0..DESCRIPTION_FETCH_ATTEMPTS {
                    let resp = match client.get(&url).send().await {
                        Ok(resp) => resp,
                        Err(_) => {
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

                    if !resp.status().is_success() {
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
                        Ok(text) => text,
                        Err(_) => {
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
                        return Some((name, first_line.to_string()));
                    }

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

    result
}

// ============================================================================
// Internal (shared) logic
// ============================================================================

async fn scan_packages_internal(
    server: &str,
    paths: Vec<CslPath>,
    local_path_strings: Vec<String>,
) -> Result<CslScanResult, String> {
    // Fetch remote index (single HTTP request)
    let index_content = fetch_remote_index(server, "x-csl-indexes.idx")
        .await
        .map_err(|e| e.to_string())?;

    let server_version = index_content.lines().next().unwrap_or("").to_string();

    // Parse and group — pure CPU, fast
    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    let package_names: Vec<String> = pkg_data_list.iter().map(|p| p.name.clone()).collect();

    // Run description fetch and local comparison IN PARALLEL
    let desc_future = fetch_package_descriptions(server, package_names);

    let compare_future = {
        let local_paths = local_path_strings;
        async move {
            let mut installed_results: Vec<tokio::task::JoinHandle<CslPackageInfo>> = Vec::new();
            let mut not_installed_results: Vec<CslPackageInfo> = Vec::new();

            for pkg in pkg_data_list {
                let local_dir = find_local_package_dir(&pkg.name, &local_paths);

                if let Some(dir) = local_dir {
                    let dir = dir.clone();
                    installed_results.push(tokio::task::spawn_blocking(move || {
                        let (status, files_to_update, update_size) =
                            compare_package_fast(&pkg, &dir);

                        let total_size = if pkg.header_size > 0 {
                            pkg.header_size
                        } else {
                            pkg.files.iter().map(|f| f.size_bytes).sum()
                        };

                        let last_updated = if !pkg.header_date.is_empty() {
                            format!("{} {}", pkg.header_date, pkg.header_time)
                        } else {
                            String::new()
                        };

                        CslPackageInfo {
                            name: pkg.name.clone(),
                            total_size_bytes: total_size,
                            file_count: pkg.files.len(),
                            description: String::new(), // filled after join
                            status,
                            files_to_update,
                            update_size_bytes: update_size,
                            last_updated,
                        }
                    }));
                } else {
                    let total_size: u64 = if pkg.header_size > 0 {
                        pkg.header_size
                    } else {
                        pkg.files.iter().map(|f| f.size_bytes).sum()
                    };
                    let update_size: u64 = pkg.files.iter().map(|f| f.size_bytes).sum();

                    let last_updated = if !pkg.header_date.is_empty() {
                        format!("{} {}", pkg.header_date, pkg.header_time)
                    } else {
                        String::new()
                    };

                    not_installed_results.push(CslPackageInfo {
                        name: pkg.name.clone(),
                        total_size_bytes: total_size,
                        file_count: pkg.files.len(),
                        description: String::new(),
                        status: "not_installed".to_string(),
                        files_to_update: pkg.files.len(),
                        update_size_bytes: update_size,
                        last_updated,
                    });
                }
            }

            // Collect blocking task results
            let mut all = not_installed_results;
            for handle in installed_results {
                match handle.await {
                    Ok(info) => all.push(info),
                    Err(e) => return Err(format!("Package comparison failed: {}", e)),
                }
            }
            Ok(all)
        }
    };

    let (descriptions, compare_result) = tokio::join!(desc_future, compare_future);
    let mut packages = compare_result?;

    // Fill descriptions from the (now-resolved) map
    for pkg in &mut packages {
        if pkg.description.is_empty() {
            pkg.description = descriptions
                .get(&pkg.name)
                .cloned()
                .unwrap_or_else(|| pkg.name.clone());
        }
    }

    // Sort by name
    packages.sort_by(|a, b| a.name.cmp(&b.name));

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
) -> Result<(), String> {
    let index_content = fetch_remote_index(server, "x-csl-indexes.idx")
        .await
        .map_err(|e| e.to_string())?;

    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    let pkg = pkg_data_list
        .iter()
        .find(|p| p.name == package_name)
        .ok_or_else(|| format!("Package {} not found in index", package_name))?;

    let target_pkg_dir = Path::new(&target_path).join(&package_name);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("XFast Manager")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

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
            )
            .await
            .map_err(|e| e.to_string())?;

            bytes_downloaded += downloaded;
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
// Directory Link Helpers (Junction on Windows, Symlink on Unix)
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
        .map(|m| m.file_type().is_symlink())
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

/// Collect all detected CSL directory paths, excluding the canonical path itself.
fn collect_link_targets(xplane_path: &Path, custom_paths: &[String]) -> Vec<PathBuf> {
    let canonical = xplane_path.join(CSL_CANONICAL_REL);
    let mut targets = Vec::new();

    for (rel_path, _) in CSL_PLUGIN_PATHS {
        let full = xplane_path.join(rel_path);
        if full == canonical {
            continue;
        }
        if full.exists() {
            targets.push(full);
        }
    }

    for cp in custom_paths {
        let p = PathBuf::from(cp);
        if p == canonical {
            continue;
        }
        if !targets.contains(&p) {
            targets.push(p);
        }
    }

    targets
}

/// Create links from each link_target/{package_name} → canonical_pkg_dir.
/// Skips if the target is a real directory (not a link) to avoid overwriting user data.
fn create_package_links(
    package_name: &str,
    canonical_pkg_dir: &Path,
    link_targets: &[PathBuf],
) -> Vec<String> {
    let mut warnings = Vec::new();

    for base in link_targets {
        let link_path = base.join(package_name);

        if link_path.exists() || is_link(&link_path) {
            if is_link(&link_path) {
                // Remove existing link and recreate
                if let Err(e) = remove_directory_link(&link_path) {
                    warnings.push(e);
                    continue;
                }
            } else {
                // Real directory — skip to avoid data loss
                warnings.push(format!(
                    "Skipped {}: real directory exists (not a link)",
                    link_path.display()
                ));
                continue;
            }
        }

        // Ensure parent exists
        if let Some(parent) = link_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Err(e) = create_directory_link(canonical_pkg_dir, &link_path) {
            warnings.push(e);
        }
    }

    warnings
}

/// Remove links for a package from all link targets.
fn remove_package_links(package_name: &str, link_targets: &[PathBuf]) -> Vec<String> {
    let mut warnings = Vec::new();

    for base in link_targets {
        let link_path = base.join(package_name);
        if is_link(&link_path) {
            if let Err(e) = remove_directory_link(&link_path) {
                warnings.push(e);
            }
        }
    }

    warnings
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

fn sync_package_links_internal(
    xplane_path: &str,
    custom_paths: &[String],
    package_names: Option<&[String]>,
    cleanup_paths: Option<&[String]>,
) -> Result<(), String> {
    let xplane = Path::new(xplane_path);
    let canonical_base = xplane.join(CSL_CANONICAL_REL);
    if !canonical_base.exists() {
        return Ok(());
    }

    let active_targets = collect_link_targets(xplane, custom_paths);
    let packages = match package_names {
        Some(names) => {
            let mut names = names.to_vec();
            names.sort();
            names.dedup();
            names
        }
        None => list_installed_canonical_packages(&canonical_base)?,
    };

    let cleanup_targets: Vec<PathBuf> = cleanup_paths
        .unwrap_or(&[])
        .iter()
        .map(PathBuf::from)
        .filter(|path| *path != canonical_base)
        .collect();

    for package_name in packages {
        let canonical_pkg_dir = canonical_base.join(&package_name);

        if !cleanup_targets.is_empty() {
            let _warnings = remove_package_links(&package_name, &cleanup_targets);
        }

        if canonical_pkg_dir.is_dir() {
            let _warnings =
                create_package_links(&package_name, &canonical_pkg_dir, &active_targets);
        } else if package_names.is_some() {
            let _warnings = remove_package_links(&package_name, &active_targets);
        }
    }

    Ok(())
}

// ============================================================================
// Tauri Commands — CSL
// ============================================================================

/// Scan packages: fetch remote index, compare with local, return results.
///
/// Optimizations vs. naive approach:
/// 1. No per-package HTTP requests for descriptions — uses package name instead
/// 2. File size check before MD5 — skips expensive hash for mismatched sizes
/// 3. All package comparisons run in parallel on the blocking thread pool
/// 4. 64KB read buffer for MD5 (8x larger)
#[tauri::command]
pub async fn csl_scan_packages(
    xplane_path: String,
    custom_paths: Vec<String>,
    server_base_url: Option<String>,
) -> Result<CslScanResult, String> {
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
    scan_packages_internal(&api_base, paths, scan_paths).await
}

/// Rescan specific packages only (uses cached index).
/// Much faster than a full scan — skips description fetches and only compares
/// the requested packages against the canonical CSL directory.
#[tauri::command]
pub async fn csl_rescan_packages(
    xplane_path: String,
    package_names: Vec<String>,
    server_base_url: Option<String>,
) -> Result<Vec<CslPackageInfo>, String> {
    let xplane = Path::new(&xplane_path);
    let canonical = xplane.join(CSL_CANONICAL_REL);
    let scan_paths = vec![canonical.to_string_lossy().to_string()];
    let server_base_url = resolve_server_base_url(server_base_url.as_deref());

    // Fetch index (hits cache if recent scan happened)
    let index_content = fetch_remote_index(&server_base_url, CSL_INDEX_PATH)
        .await
        .map_err(|e| e.to_string())?;

    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    // Filter to only requested packages
    let target_set: std::collections::HashSet<&str> =
        package_names.iter().map(|s| s.as_str()).collect();

    let mut results = Vec::new();
    let mut handles = Vec::new();

    for pkg in pkg_data_list {
        if !target_set.contains(pkg.name.as_str()) {
            continue;
        }

        let local_dir = find_local_package_dir(&pkg.name, &scan_paths);

        if let Some(dir) = local_dir {
            let dir = dir.clone();
            handles.push(tokio::task::spawn_blocking(move || {
                let (status, files_to_update, update_size) = compare_package_fast(&pkg, &dir);

                let total_size = if pkg.header_size > 0 {
                    pkg.header_size
                } else {
                    pkg.files.iter().map(|f| f.size_bytes).sum()
                };

                let last_updated = if !pkg.header_date.is_empty() {
                    format!("{} {}", pkg.header_date, pkg.header_time)
                } else {
                    String::new()
                };

                CslPackageInfo {
                    name: pkg.name.clone(),
                    total_size_bytes: total_size,
                    file_count: pkg.files.len(),
                    description: String::new(), // caller preserves original description
                    status,
                    files_to_update,
                    update_size_bytes: update_size,
                    last_updated,
                }
            }));
        } else {
            // Not installed
            let total_size: u64 = if pkg.header_size > 0 {
                pkg.header_size
            } else {
                pkg.files.iter().map(|f| f.size_bytes).sum()
            };
            let update_size: u64 = pkg.files.iter().map(|f| f.size_bytes).sum();

            let last_updated = if !pkg.header_date.is_empty() {
                format!("{} {}", pkg.header_date, pkg.header_time)
            } else {
                String::new()
            };

            results.push(CslPackageInfo {
                name: pkg.name.clone(),
                total_size_bytes: total_size,
                file_count: pkg.files.len(),
                description: String::new(),
                status: "not_installed".to_string(),
                files_to_update: pkg.files.len(),
                update_size_bytes: update_size,
                last_updated,
            });
        }
    }

    for handle in handles {
        match handle.await {
            Ok(info) => results.push(info),
            Err(e) => {
                return Err(format!("Package comparison failed: {}", e));
            }
        }
    }

    Ok(results)
}

/// Install or update a specific CSL package.
/// Downloads to the canonical path (Resources/plugins/IVAO_CSL/CSL) and creates
/// junction/symlink in other detected CSL directories.
#[tauri::command]
pub async fn csl_install_package(
    package_name: String,
    xplane_path: String,
    custom_paths: Vec<String>,
    parallel_downloads: Option<usize>,
    server_base_url: Option<String>,
    app_handle: AppHandle,
    download_control: State<'_, CslDownloadControl>,
) -> Result<(), String> {
    let task_key = install_task_key("csl", &package_name);
    let _registration = download_control
        .register(task_key)
        .map_err(|e| e.to_string())?;
    let cancel_flag = _registration.cancel_flag();

    let xplane = Path::new(&xplane_path);
    let canonical_base = xplane.join(CSL_CANONICAL_REL);
    let api_base = resolve_csl_api_base(server_base_url.as_deref());
    std::fs::create_dir_all(&canonical_base)
        .map_err(|e| format!("Failed to create canonical CSL dir: {}", e))?;

    let canonical_base_str = canonical_base.to_string_lossy().to_string();
    install_package_internal(
        &api_base,
        "csl-progress",
        package_name.clone(),
        canonical_base_str,
        parallel_downloads,
        app_handle,
        cancel_flag,
    )
    .await?;

    // Create links in other plugin CSL directories
    let link_targets = collect_link_targets(xplane, &custom_paths);
    let canonical_pkg_dir = canonical_base.join(&package_name);
    let _warnings = create_package_links(&package_name, &canonical_pkg_dir, &link_targets);

    Ok(())
}

#[tauri::command]
pub async fn csl_cancel_install(
    source: String,
    package_name: String,
    download_control: State<'_, CslDownloadControl>,
) -> Result<(), String> {
    let normalized_source = match source.as_str() {
        "altitude" => "altitude",
        _ => "csl",
    };

    download_control.cancel(&install_task_key(normalized_source, &package_name));
    Ok(())
}

/// Uninstall a CSL package.
/// Removes links from other plugin directories first, then deletes the canonical copy.
#[tauri::command]
pub async fn csl_uninstall_package(
    package_name: String,
    xplane_path: String,
    custom_paths: Vec<String>,
) -> Result<(), String> {
    let xplane = Path::new(&xplane_path);
    let link_targets = collect_link_targets(xplane, &custom_paths);

    // Remove links first
    let _warnings = remove_package_links(&package_name, &link_targets);

    // Remove the canonical copy
    let canonical_pkg_dir = xplane.join(CSL_CANONICAL_REL).join(&package_name);
    if canonical_pkg_dir.exists() && canonical_pkg_dir.is_dir() {
        std::fs::remove_dir_all(&canonical_pkg_dir)
            .map_err(|e| format!("Failed to remove {}: {}", canonical_pkg_dir.display(), e))?;
        return Ok(());
    }

    // Fallback: try old paths for backward compatibility
    let (_, all_paths) = collect_scan_paths(&xplane_path, &custom_paths);
    uninstall_package_internal(&package_name, &all_paths)
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
    cleanup_paths: Option<Vec<String>>,
) -> Result<(), String> {
    sync_package_links_internal(
        &xplane_path,
        &custom_paths,
        package_names.as_deref(),
        cleanup_paths.as_deref(),
    )
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
) -> Result<CslScanResult, String> {
    let xplane = PathBuf::from(&xplane_path);
    let server_base_url = resolve_server_base_url(server_base_url.as_deref());

    let index_content = fetch_remote_index(&server_base_url, ALTITUDE_INDEX_PATH)
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

    Ok(CslScanResult {
        packages,
        paths: vec![],
        server_version,
    })
}

/// Install or update the ALTITUDE supplementary package
#[tauri::command]
pub async fn altitude_install_package(
    xplane_path: String,
    parallel_downloads: Option<usize>,
    server_base_url: Option<String>,
    app_handle: AppHandle,
    download_control: State<'_, CslDownloadControl>,
) -> Result<(), String> {
    let xplane = PathBuf::from(&xplane_path);
    let package_name = "ALTITUDE".to_string();
    let resolved_server_base_url = resolve_server_base_url(server_base_url.as_deref());
    let api_base = resolve_csl_api_base(server_base_url.as_deref());
    let task_key = install_task_key("altitude", &package_name);
    let _registration = download_control
        .register(task_key)
        .map_err(|e| e.to_string())?;
    let cancel_flag = _registration.cancel_flag();

    let index_content = fetch_remote_index(&resolved_server_base_url, ALTITUDE_INDEX_PATH)
        .await
        .map_err(|e| e.to_string())?;

    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    let pkg = pkg_data_list
        .iter()
        .find(|p| p.name == package_name)
        .ok_or("ALTITUDE package not found in index")?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("XFast Manager")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

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
            )
            .await
            .map_err(|e| e.to_string())?;
            bytes_downloaded += downloaded;
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

    Ok(())
}

/// Uninstall the ALTITUDE supplementary package
#[tauri::command]
pub async fn altitude_uninstall_package(
    xplane_path: String,
    server_base_url: Option<String>,
) -> Result<(), String> {
    let xplane = Path::new(&xplane_path);
    let server_base_url = resolve_server_base_url(server_base_url.as_deref());

    let index_content = fetch_remote_index(&server_base_url, ALTITUDE_INDEX_PATH)
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
        return Err("ALTITUDE package not found locally".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

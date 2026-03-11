use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::Mutex;

use crate::error::{ApiError, ApiErrorCode};

const CSL_SERVER: &str = "http://csl.x-air.ru/package";

/// Cached index with TTL
static INDEX_CACHE: std::sync::LazyLock<Mutex<Option<(std::time::Instant, String)>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

const INDEX_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(300); // 5 minutes

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
                let pkg = map.entry(entry.path.clone()).or_insert_with(|| PackageData {
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
/// Uses single metadata() call instead of exists() + metadata().
fn compare_package_fast(
    pkg: &PackageData,
    local_dir: &Path,
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
                // Size matches → compute MD5 only if server provides hash
                if let Some(ref server_hash) = file.md5_hash {
                    match compute_file_md5(&local_file) {
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

async fn fetch_remote_index(server: &str) -> Result<String, ApiError> {
    // Check cache first
    {
        let cache = INDEX_CACHE.lock().await;
        if let Some((fetched_at, ref content)) = *cache {
            if fetched_at.elapsed() < INDEX_CACHE_TTL {
                return Ok(content.clone());
            }
        }
    }

    let url = format!("{}/x-csl-indexes.idx", server);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| {
            ApiError::new(ApiErrorCode::NetworkError, format!("HTTP client error: {}", e))
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
        *cache = Some((std::time::Instant::now(), content.clone()));
    }

    Ok(content)
}

async fn download_file(
    client: &reqwest::Client,
    server: &str,
    remote_path: &str,
    local_path: &Path,
) -> Result<u64, ApiError> {
    let url = format!("{}/{}", server, remote_path);

    let resp = client.get(&url).send().await.map_err(|e| {
        ApiError::new(ApiErrorCode::NetworkError, format!("Download failed: {}", e))
    })?;

    if !resp.status().is_success() {
        return Err(ApiError::new(
            ApiErrorCode::NetworkError,
            format!("Download failed with status {}", resp.status()),
        ));
    }

    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ApiError::new(
                ApiErrorCode::Internal,
                format!("Failed to create directory: {}", e),
            )
        })?;
    }

    let bytes = resp.bytes().await.map_err(|e| {
        ApiError::new(
            ApiErrorCode::NetworkError,
            format!("Failed to read download: {}", e),
        )
    })?;

    let size = bytes.len() as u64;
    std::fs::write(local_path, &bytes).map_err(|e| {
        ApiError::new(
            ApiErrorCode::Internal,
            format!("Failed to write file: {}", e),
        )
    })?;

    Ok(size)
}

// ============================================================================
// Tauri Commands
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
) -> Result<CslScanResult, String> {
    let xplane = Path::new(&xplane_path);

    // Detect local CSL paths
    let mut paths = detect_csl_paths(xplane);
    for cp in &custom_paths {
        if !paths.iter().any(|p| p.path == *cp) {
            paths.push(CslPath {
                path: cp.clone(),
                source: "custom".to_string(),
                plugin_name: None,
            });
        }
    }

    let local_path_strings: Vec<String> = paths.iter().map(|p| p.path.clone()).collect();

    // Fetch remote index (single HTTP request)
    let index_content = fetch_remote_index(CSL_SERVER)
        .await
        .map_err(|e| e.to_string())?;

    let server_version = index_content
        .lines()
        .next()
        .unwrap_or("")
        .to_string();

    // Parse and group — pure CPU, fast
    let entries = parse_index(&index_content);
    let pkg_data_list = group_into_packages(&entries);

    // Compare all packages — pre-filter to skip spawn_blocking for not-installed
    let local_paths_arc = std::sync::Arc::new(local_path_strings);
    let mut packages = Vec::with_capacity(pkg_data_list.len());
    let mut handles = Vec::new();

    for pkg in pkg_data_list {
        // Fast path: check if package directory exists in any local path
        let local_dir = find_local_package_dir(&pkg.name, &local_paths_arc);

        if let Some(dir) = local_dir {
            // Package exists locally — spawn blocking for MD5/size comparison
            let dir = dir.clone();
            handles.push(tokio::task::spawn_blocking(move || {
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
                    description: pkg.name,
                    status,
                    files_to_update,
                    update_size_bytes: update_size,
                    last_updated,
                }
            }));
        } else {
            // Not installed — no I/O needed, create result directly
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

            packages.push(CslPackageInfo {
                name: pkg.name.clone(),
                total_size_bytes: total_size,
                file_count: pkg.files.len(),
                description: pkg.name,
                status: "not_installed".to_string(),
                files_to_update: pkg.files.len(),
                update_size_bytes: update_size,
                last_updated,
            });
        }
    }

    // Collect blocking task results
    for handle in handles {
        match handle.await {
            Ok(info) => packages.push(info),
            Err(e) => {
                return Err(format!("Package comparison failed: {}", e));
            }
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

/// Install or update a specific CSL package
#[tauri::command]
pub async fn csl_install_package(
    package_name: String,
    target_path: String,
    parallel_downloads: Option<usize>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let server = CSL_SERVER;

    let index_content = fetch_remote_index(server)
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
                match compute_file_md5(&local_file) {
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

    let concurrency = parallel_downloads.unwrap_or(1).clamp(1, 10);

    if concurrency <= 1 {
        // Sequential download (original behavior)
        let mut bytes_downloaded: u64 = 0;

        for (i, file) in files_to_download.iter().enumerate() {
            let rel_path = file.path.strip_prefix(&prefix).unwrap_or(&file.path);
            let local_path = target_pkg_dir.join(rel_path);

            let _ = app_handle.emit(
                "csl-progress",
                CslProgressEvent {
                    package_name: package_name.clone(),
                    current_file: i + 1,
                    total_files: download_total,
                    current_file_name: rel_path.to_string(),
                    bytes_downloaded,
                    total_bytes: download_total_bytes,
                },
            );

            let downloaded = download_file(&client, server, &file.path, &local_path)
                .await
                .map_err(|e| e.to_string())?;

            bytes_downloaded += downloaded;
        }
    } else {
        // Parallel download — collect owned data to avoid lifetime issues
        let owned_files: Vec<String> = files_to_download.iter().map(|f| f.path.clone()).collect();
        let completed = Arc::new(AtomicU64::new(0));
        let bytes_downloaded = Arc::new(AtomicU64::new(0));

        let results: Vec<Result<(), String>> = stream::iter(
            owned_files.into_iter().map(|file_path| {
                let client = client.clone();
                let server = server.to_string();
                let prefix = prefix.clone();
                let target_pkg_dir = target_pkg_dir.clone();
                let app_handle = app_handle.clone();
                let package_name = package_name.clone();
                let completed = completed.clone();
                let bytes_downloaded = bytes_downloaded.clone();

                async move {
                    let rel_path = file_path
                        .strip_prefix(&prefix)
                        .unwrap_or(&file_path);
                    let local_path = target_pkg_dir.join(rel_path);

                    let downloaded =
                        download_file(&client, &server, &file_path, &local_path)
                            .await
                            .map_err(|e| e.to_string())?;

                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    let total_dl =
                        bytes_downloaded.fetch_add(downloaded, Ordering::Relaxed) + downloaded;

                    let _ = app_handle.emit(
                        "csl-progress",
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
            }),
        )
        .buffer_unordered(concurrency)
        .collect()
        .await;

        // Check for any errors
        for result in results {
            result?;
        }
    }

    let _ = app_handle.emit(
        "csl-progress",
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

/// Uninstall a CSL package
#[tauri::command]
pub async fn csl_uninstall_package(
    package_name: String,
    paths: Vec<String>,
) -> Result<(), String> {
    for base_path in &paths {
        let pkg_dir = Path::new(base_path).join(&package_name);
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

/// Detect CSL paths from known plugin directories
#[tauri::command]
pub async fn csl_detect_paths(xplane_path: String) -> Result<Vec<CslPath>, String> {
    Ok(detect_csl_paths(Path::new(&xplane_path)))
}

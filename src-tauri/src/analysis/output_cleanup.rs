//! Output folder cleanup support.
//!
//! The cleanup catalog is embedded for offline use and can be replaced at
//! runtime by the remote JSON served through the Vercel proxy.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{LazyLock, Mutex, RwLock};
use std::time::Duration;
use walkdir::WalkDir;

use crate::logger;

#[path = "output_isolation.rs"]
mod isolation;
pub use isolation::OutputCleanupStateChange;

const UNKNOWN_PREFIX: &str = "unknown:";
const PROTECTED_OUTPUT_DIRS: &[&str] = &["preferences"];
const ALLOWED_LEVELS: &[&str] = &["recommended", "cleanable", "cautious"];
static OUTPUT_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputCleanupCatalog {
    pub version: u32,
    pub updated: String,
    pub items: Vec<OutputCleanupCatalogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputCleanupCatalogItem {
    pub id: String,
    pub relative_path: String,
    pub display_name: String,
    pub description: String,
    pub level: String,
    #[serde(default)]
    pub default_selected: bool,
    #[serde(default)]
    pub warning_key: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub show_when_missing: bool,
}

#[derive(Debug, Clone)]
struct LoadedCatalog {
    source: String,
    catalog: OutputCleanupCatalog,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputCleanupConfigInfo {
    pub version: u32,
    pub updated: String,
    pub source: String,
    pub item_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputCleanupReport {
    pub version: u32,
    pub updated: String,
    pub source: String,
    pub total_bytes: u64,
    pub total_files: usize,
    pub items: Vec<OutputCleanupItem>,
    disabled_items: Vec<isolation::DisabledOutputItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputCleanupItem {
    pub id: String,
    pub relative_path: String,
    pub folder_name: String,
    pub display_name: String,
    pub description: String,
    pub level: String,
    pub default_selected: bool,
    pub warning_key: Option<String>,
    pub size_bytes: u64,
    pub file_count: usize,
    pub exists: bool,
    pub recognized: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputCleanupTarget {
    pub id: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputCleanupResult {
    pub total_deleted_bytes: u64,
    pub total_deleted_files: usize,
    pub items: Vec<OutputCleanupItemResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputCleanupItemResult {
    pub id: String,
    pub relative_path: String,
    pub deleted_bytes: u64,
    pub deleted_files: usize,
    pub failed_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputCleanupSubmissionResult {
    pub issue_url: String,
    pub issue_number: u64,
    pub issue_title: String,
}

static CLEANUP_CATALOG: LazyLock<RwLock<LoadedCatalog>> =
    LazyLock::new(|| RwLock::new(load_embedded_catalog()));

fn load_embedded_catalog() -> LoadedCatalog {
    let embedded_json = include_str!("../../../data/output_cleanup_items.json");
    match serde_json::from_str::<OutputCleanupCatalog>(embedded_json) {
        Ok(mut catalog) => match normalize_catalog(&mut catalog) {
            Ok(()) => LoadedCatalog {
                source: "embedded".to_string(),
                catalog,
            },
            Err(e) => {
                logger::log_info(
                    &format!(
                        "Failed to validate embedded data/output_cleanup_items.json: {}, using empty fallback",
                        e
                    ),
                    Some("output_cleanup"),
                );
                LoadedCatalog {
                    source: "embedded".to_string(),
                    catalog: OutputCleanupCatalog {
                        version: 0,
                        updated: String::new(),
                        items: Vec::new(),
                    },
                }
            }
        },
        Err(e) => {
            logger::log_info(
                &format!(
                    "Failed to parse embedded data/output_cleanup_items.json: {}, using empty fallback",
                    e
                ),
                Some("output_cleanup"),
            );
            LoadedCatalog {
                source: "embedded".to_string(),
                catalog: OutputCleanupCatalog {
                    version: 0,
                    updated: String::new(),
                    items: Vec::new(),
                },
            }
        }
    }
}

fn normalize_catalog(catalog: &mut OutputCleanupCatalog) -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut paths = HashSet::new();

    catalog.items.retain(|item| {
        !item.id.trim().is_empty()
            && !item.relative_path.trim().is_empty()
            && !item.display_name.trim().is_empty()
    });

    for item in &mut catalog.items {
        item.id = normalize_id(&item.id);
        item.relative_path = normalize_relative_path(&item.relative_path)?;
        item.level = item.level.trim().to_lowercase();

        if !ALLOWED_LEVELS.contains(&item.level.as_str()) {
            item.level = "cleanable".to_string();
        }
        if item.warning_key.as_deref() == Some("addon_data") {
            item.level = "cautious".to_string();
            item.default_selected = false;
        }

        if !ids.insert(item.id.clone()) {
            return Err(format!("Duplicate output cleanup item id: {}", item.id));
        }

        let path_key = item.relative_path.to_lowercase();
        if !paths.insert(path_key) {
            return Err(format!(
                "Duplicate output cleanup relative path: {}",
                item.relative_path
            ));
        }

        if !is_output_relative_path(&item.relative_path) {
            return Err(format!(
                "Cleanup path must be under Output: {}",
                item.relative_path
            ));
        }

        if is_protected_output_relative_path(&item.relative_path) {
            return Err(format!("Cleanup path is protected: {}", item.relative_path));
        }
    }

    Ok(())
}

fn normalize_id(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn normalize_relative_path(raw: &str) -> Result<String, String> {
    let normalized = raw.replace('\\', "/");
    if normalized.is_empty() {
        return Err("Relative path is empty".to_string());
    }

    let path = Path::new(&normalized);
    if path.is_absolute() {
        return Err(format!("Cleanup path must be relative: {}", raw));
    }

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let text = part.to_string_lossy();
                if text.trim().is_empty()
                    || (cfg!(windows) && (text.ends_with(['.', ' ']) || text.contains(':')))
                {
                    return Err(format!("Invalid cleanup path: {}", raw));
                }
                parts.push(text.into_owned());
            }
            Component::CurDir => {}
            _ => return Err(format!("Cleanup path cannot contain parent/root: {}", raw)),
        }
    }

    if parts.is_empty() {
        return Err(format!("Invalid cleanup path: {}", raw));
    }

    Ok(parts.join("/"))
}

fn is_output_relative_path(relative_path: &str) -> bool {
    let mut parts = relative_path.split('/');
    matches!(parts.next(), Some(first) if first.eq_ignore_ascii_case("Output"))
        && parts.next().is_some()
}

fn is_protected_output_relative_path(relative_path: &str) -> bool {
    let mut parts = relative_path.split('/');
    let first = parts.next();
    let second = parts.next();

    if !matches!(first, Some(value) if value.eq_ignore_ascii_case("Output")) {
        return false;
    }

    matches!(
        second,
        Some(value)
            if PROTECTED_OUTPUT_DIRS
                .iter()
                .any(|protected| value.eq_ignore_ascii_case(protected))
    )
}

fn direct_output_child_relative_path(folder_name: &str) -> Result<String, String> {
    if folder_name.contains(['/', '\\']) {
        return Err("Output cleanup only supports direct children".into());
    }
    let normalized = normalize_relative_path(&format!("Output/{}", folder_name))?;
    let part_count = normalized.split('/').count();
    if part_count != 2 {
        return Err(format!(
            "Output cleanup only supports direct children: {}",
            folder_name
        ));
    }
    if is_protected_output_relative_path(&normalized) {
        return Err(format!("Output folder is protected: {}", folder_name));
    }
    Ok(normalized)
}

fn config_info(loaded: &LoadedCatalog) -> OutputCleanupConfigInfo {
    OutputCleanupConfigInfo {
        version: loaded.catalog.version,
        updated: loaded.catalog.updated.clone(),
        source: loaded.source.clone(),
        item_count: loaded.catalog.items.len(),
    }
}

fn current_catalog() -> LoadedCatalog {
    CLEANUP_CATALOG
        .read()
        .expect("output cleanup catalog lock poisoned during read")
        .clone()
}

async fn fetch_remote_catalog() -> Result<OutputCleanupCatalog, String> {
    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let remote_url = crate::vercel_api::endpoint_with_override(
        "output-cleanup-items-data",
        "XFAST_OUTPUT_CLEANUP_ITEMS_API_URL",
    );
    let response = client
        .get(&remote_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP status: {}", response.status()));
    }

    let mut catalog: OutputCleanupCatalog = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    normalize_catalog(&mut catalog)?;
    Ok(catalog)
}

pub async fn refresh_remote_catalog() -> Result<OutputCleanupConfigInfo, String> {
    let catalog = fetch_remote_catalog().await?;
    let mut guard = CLEANUP_CATALOG
        .write()
        .expect("output cleanup catalog lock poisoned during write");
    if catalog.version < guard.catalog.version {
        return Ok(config_info(&guard));
    }
    *guard = LoadedCatalog {
        source: "remote".to_string(),
        catalog,
    };

    logger::log_info(
        &format!(
            "Fetched {} output cleanup items from remote",
            guard.catalog.items.len()
        ),
        Some("output_cleanup"),
    );

    Ok(config_info(&guard))
}

pub fn reset_to_embedded_catalog() -> OutputCleanupConfigInfo {
    let mut guard = CLEANUP_CATALOG
        .write()
        .expect("output cleanup catalog lock poisoned during embedded reset");
    *guard = load_embedded_catalog();
    config_info(&guard)
}

fn folder_size(path: &Path) -> (u64, usize) {
    let mut total = 0u64;
    let mut count = 0usize;

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| entry.metadata().is_ok_and(|meta| !is_link(&meta)))
        .filter_map(|entry| entry.ok())
    {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            count += 1;
        }
    }

    (total, count)
}

fn path_from_relative(xplane_path: &Path, relative_path: &str) -> PathBuf {
    let mut path = xplane_path.join("Output");
    for part in relative_path.split('/').skip(1) {
        path.push(part);
    }
    path
}

fn folder_name_from_relative(relative_path: &str) -> String {
    relative_path
        .split('/')
        .next_back()
        .unwrap_or(relative_path)
        .to_string()
}

pub fn scan_output_cleanup_items(xplane_path: &str) -> Result<OutputCleanupReport, String> {
    let _guard = OUTPUT_LOCK.lock().map_err(|e| e.to_string())?;
    let loaded = current_catalog();
    let xplane = Path::new(xplane_path);
    let output_dir = xplane.join("Output");
    let mut items = Vec::new();
    let mut recognized_paths = HashSet::new();

    for config_item in &loaded.catalog.items {
        let Ok(target) = validate_target_path(xplane, &config_item.relative_path) else {
            continue;
        };
        let exists = target.exists();
        if !exists && !config_item.show_when_missing {
            continue;
        }

        let (size_bytes, file_count) = folder_size(&target);
        recognized_paths.insert(config_item.relative_path.to_lowercase());
        items.push(OutputCleanupItem {
            id: config_item.id.clone(),
            relative_path: config_item.relative_path.clone(),
            folder_name: folder_name_from_relative(&config_item.relative_path),
            display_name: config_item.display_name.clone(),
            description: config_item.description.clone(),
            level: config_item.level.clone(),
            default_selected: config_item.default_selected,
            warning_key: config_item.warning_key.clone(),
            size_bytes,
            file_count,
            exists,
            recognized: true,
        });
    }

    if fs::symlink_metadata(&output_dir).is_ok_and(|meta| meta.is_dir() && !is_link(&meta)) {
        if let Ok(entries) = fs::read_dir(&output_dir) {
            for entry in entries.filter_map(|entry| entry.ok()) {
                let path = entry.path();
                if !fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_dir() && !is_link(&meta)) {
                    continue;
                }

                let name = entry.file_name();
                let Some(folder_name) = name.to_str() else {
                    continue;
                };
                let relative_path = match direct_output_child_relative_path(folder_name) {
                    Ok(path) => path,
                    Err(_) => continue,
                };
                if recognized_paths.contains(&relative_path.to_lowercase()) {
                    continue;
                }

                let (size_bytes, file_count) = folder_size(&path);
                items.push(OutputCleanupItem {
                    id: format!("{}{}", UNKNOWN_PREFIX, folder_name),
                    relative_path,
                    folder_name: folder_name.to_string(),
                    display_name: folder_name.to_string(),
                    description: String::new(),
                    level: "unknown".to_string(),
                    default_selected: false,
                    warning_key: None,
                    size_bytes,
                    file_count,
                    exists: true,
                    recognized: false,
                });
            }
        }
    }

    items.sort_by(|a, b| {
        let rank = |level: &str| match level {
            "recommended" => 0,
            "cleanable" => 1,
            "cautious" => 2,
            "unknown" => 3,
            _ => 4,
        };
        rank(&a.level)
            .cmp(&rank(&b.level))
            .then_with(|| b.size_bytes.cmp(&a.size_bytes))
            .then_with(|| a.display_name.cmp(&b.display_name))
    });

    let total_bytes = items.iter().map(|item| item.size_bytes).sum();
    let total_files = items.iter().map(|item| item.file_count).sum();

    Ok(OutputCleanupReport {
        version: loaded.catalog.version,
        updated: loaded.catalog.updated,
        source: loaded.source,
        total_bytes,
        total_files,
        items,
        disabled_items: isolation::scan(xplane),
    })
}

fn catalog_by_id(catalog: &OutputCleanupCatalog) -> HashMap<String, OutputCleanupCatalogItem> {
    catalog
        .items
        .iter()
        .cloned()
        .map(|item| (item.id.clone(), item))
        .collect()
}

fn resolve_target(
    catalog: &OutputCleanupCatalog,
    target: &OutputCleanupTarget,
) -> Result<String, String> {
    let id = normalize_id(&target.id);
    let by_id = catalog_by_id(catalog);

    if let Some(config_item) = by_id.get(&id) {
        if normalize_relative_path(&target.relative_path)? != config_item.relative_path {
            return Err("Output item changed. Refresh the cleanup list.".into());
        }
        return Ok(config_item.relative_path.clone());
    }

    let Some(folder_name) = target.id.strip_prefix(UNKNOWN_PREFIX) else {
        return Err(format!("Unknown output cleanup item id: {}", target.id));
    };

    let relative_path = normalize_relative_path(&target.relative_path)?;
    if !is_output_relative_path(&relative_path) {
        return Err(format!(
            "Unknown cleanup path must be under Output: {}",
            relative_path
        ));
    }
    if relative_path.split('/').count() != 2 {
        return Err(format!(
            "Unknown cleanup path must be an Output direct child: {}",
            relative_path
        ));
    }
    if is_protected_output_relative_path(&relative_path) {
        return Err(format!("Output folder is protected: {}", relative_path));
    }
    if direct_output_child_relative_path(folder_name)? != relative_path {
        return Err("Output item changed. Refresh the cleanup list.".into());
    }

    let relative_key = relative_path.to_lowercase();
    if catalog
        .items
        .iter()
        .any(|item| item.relative_path.to_lowercase() == relative_key)
    {
        return Err(format!(
            "Path is recognized but item id is invalid: {}",
            relative_path
        ));
    }

    Ok(relative_path)
}

fn validate_target_path(xplane: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative_path = normalize_relative_path(relative_path)?;
    if !is_output_relative_path(&relative_path) || is_protected_output_relative_path(&relative_path)
    {
        return Err(format!(
            "Output path is protected or invalid: {relative_path}"
        ));
    }
    let target = path_from_relative(xplane, &relative_path);
    for path in target.ancestors().take_while(|path| *path != xplane) {
        match fs::symlink_metadata(path) {
            Ok(meta) if is_link(&meta) => {
                return Err(format!("Output paths cannot use links: {}", path.display()));
            }
            Ok(meta) if path != target && !meta.is_dir() => {
                return Err(format!("Expected a directory at {}", path.display()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {error}", path.display())),
        }
    }
    #[cfg(windows)]
    if target.exists() {
        let output = xplane
            .join("Output")
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let resolved = target.canonicalize().map_err(|e| e.to_string())?;
        let tail = resolved.strip_prefix(output).map_err(|e| e.to_string())?;
        let actual_path = format!("Output/{}", tail.display().to_string().replace('\\', "/"));
        if is_protected_output_relative_path(&actual_path) {
            return Err(format!("Output path is protected: {relative_path}"));
        }
    }
    Ok(target)
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn remove_child(path: &Path) -> (u64, usize, Option<String>) {
    let (bytes, files) = folder_size(path);
    let result = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && is_link(&metadata) => fs::remove_dir(path),
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) => return (0, 0, Some(format!("{}: {}", path.display(), e))),
    };

    match result {
        Ok(_) => (bytes, files, None),
        Err(e) => (0, 0, Some(format!("{}: {}", path.display(), e))),
    }
}

fn clean_one_target(
    xplane: &Path,
    target: &OutputCleanupTarget,
    relative_path: String,
) -> Result<OutputCleanupItemResult, String> {
    let target_path = validate_target_path(xplane, &relative_path)?;
    let mut result = OutputCleanupItemResult {
        id: target.id.clone(),
        relative_path,
        deleted_bytes: 0,
        deleted_files: 0,
        failed_paths: Vec::new(),
    };

    if !target_path.exists() {
        return Ok(result);
    }

    if target_path.is_file() {
        let (bytes, files, error) = remove_child(&target_path);
        result.deleted_bytes += bytes;
        result.deleted_files += files;
        if let Some(error) = error {
            result.failed_paths.push(error);
        }
        return Ok(result);
    }

    let entries = fs::read_dir(&target_path).map_err(|e| {
        format!(
            "Failed to read cleanup target {}: {}",
            target_path.display(),
            e
        )
    })?;

    for entry in entries.filter_map(|entry| entry.ok()) {
        let (bytes, files, error) = remove_child(&entry.path());
        result.deleted_bytes += bytes;
        result.deleted_files += files;
        if let Some(error) = error {
            result.failed_paths.push(error);
        }
    }

    Ok(result)
}

pub fn clean_output_items(
    xplane_path: &str,
    targets: Vec<OutputCleanupTarget>,
) -> Result<OutputCleanupResult, String> {
    let _guard = OUTPUT_LOCK.lock().map_err(|e| e.to_string())?;
    if targets.is_empty() {
        return Err("No cleanup items selected".to_string());
    }

    let loaded = current_catalog();
    let xplane = Path::new(xplane_path);
    let mut seen_paths = HashSet::new();
    let mut selected = Vec::new();
    for target in targets {
        let relative_path = resolve_target(&loaded.catalog, &target)?;
        let path_key = relative_path.to_lowercase();
        if !seen_paths.insert(path_key) {
            continue;
        }

        validate_target_path(xplane, &relative_path)?;
        selected.push((target, relative_path));
    }
    let item_results = selected
        .into_iter()
        .map(|(target, relative_path)| clean_one_target(xplane, &target, relative_path))
        .collect::<Result<Vec<_>, _>>()?;

    let total_deleted_bytes = item_results.iter().map(|item| item.deleted_bytes).sum();
    let total_deleted_files = item_results.iter().map(|item| item.deleted_files).sum();

    Ok(OutputCleanupResult {
        total_deleted_bytes,
        total_deleted_files,
        items: item_results,
    })
}

pub fn change_output_cleanup_item_state(
    xplane_path: &str,
    change: OutputCleanupStateChange,
) -> Result<(), String> {
    let _guard = OUTPUT_LOCK.lock().map_err(|e| e.to_string())?;
    let xplane = Path::new(xplane_path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    match change {
        OutputCleanupStateChange::Disable { target } => {
            let relative = resolve_target(&current_catalog().catalog, &target)?;
            isolation::disable(&xplane, &relative)
        }
        OutputCleanupStateChange::Restore { id } => isolation::restore(&xplane, &id),
    }
}

pub async fn submit_unknown_output_cleanup_item(
    folder_name: String,
    relative_path: String,
    description: String,
    expected_level: String,
    size_bytes: u64,
    file_count: usize,
) -> Result<OutputCleanupSubmissionResult, String> {
    let folder_name = folder_name.trim();
    let description = description.trim();
    let expected_level = expected_level.trim().to_lowercase();
    let relative_path = normalize_relative_path(&relative_path)?;

    if folder_name.is_empty() {
        return Err("Folder name is required".to_string());
    }
    if description.is_empty() {
        return Err("Description is required".to_string());
    }
    if !ALLOWED_LEVELS.contains(&expected_level.as_str()) {
        return Err("Expected level is invalid".to_string());
    }
    if relative_path.split('/').count() != 2 || !is_output_relative_path(&relative_path) {
        return Err("Only direct Output child folders can be submitted".to_string());
    }
    if is_protected_output_relative_path(&relative_path) {
        return Err("Protected Output folders cannot be submitted".to_string());
    }

    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let api_url = crate::vercel_api::endpoint_with_override(
        "output-cleanup-item-submission",
        "XFAST_OUTPUT_CLEANUP_SUBMISSION_API_URL",
    );

    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&api_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "folderName": folder_name,
            "relativePath": relative_path,
            "description": description,
            "expectedLevel": expected_level,
            "sizeBytes": size_bytes,
            "fileCount": file_count,
            "appVersion": app_version,
            "os": os,
            "arch": arch
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to submit cleanup item: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Cleanup item API error {}: {}", status, error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let issue_url = response_json
        .get("issueUrl")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    if issue_url.is_empty() {
        return Err("Issue created but response URL missing".to_string());
    }

    let issue_number = response_json
        .get("issueNumber")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let issue_title = response_json
        .get("issueTitle")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();

    Ok(OutputCleanupSubmissionResult {
        issue_url,
        issue_number,
        issue_title,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(path: &Path, bytes: usize) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, vec![b'x'; bytes]).unwrap();
    }

    #[test]
    fn scan_excludes_preferences_and_adds_unknown_items() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("Output");
        write_file(&output.join("preferences").join("prefs.txt"), 10);
        write_file(&output.join("MysteryCache").join("cache.bin"), 20);

        let report = scan_output_cleanup_items(&temp.path().display().to_string()).unwrap();

        assert!(!report.items.iter().any(|item| item
            .relative_path
            .eq_ignore_ascii_case("Output/preferences")));
        let unknown = report
            .items
            .iter()
            .find(|item| item.relative_path == "Output/MysteryCache")
            .unwrap();
        assert!(!unknown.recognized);
        assert_eq!(unknown.level, "unknown");
        assert_eq!(unknown.size_bytes, 20);
    }

    #[test]
    fn clean_unknown_direct_child_keeps_top_level_directory() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("Output").join("MysteryCache");
        write_file(&target.join("nested").join("cache.bin"), 12);

        let result = clean_output_items(
            &temp.path().display().to_string(),
            vec![OutputCleanupTarget {
                id: "unknown:MysteryCache".to_string(),
                relative_path: "Output/MysteryCache".to_string(),
            }],
        )
        .unwrap();

        assert_eq!(result.total_deleted_bytes, 12);
        assert_eq!(result.total_deleted_files, 1);
        assert!(target.exists());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    }

    #[test]
    fn clean_rejects_protected_unknown_folder() {
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp
                .path()
                .join("Output")
                .join("preferences")
                .join("prefs.txt"),
            12,
        );

        let error = clean_output_items(
            &temp.path().display().to_string(),
            vec![OutputCleanupTarget {
                id: "unknown:preferences".to_string(),
                relative_path: "Output/preferences".to_string(),
            }],
        )
        .unwrap_err();

        assert!(error.contains("protected"));
        assert!(temp
            .path()
            .join("Output")
            .join("preferences")
            .join("prefs.txt")
            .exists());
    }

    #[test]
    fn disabled_output_is_excluded_from_cleanup_and_restores_both_versions() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_str().unwrap();
        let target = temp.path().join("Output/CL650/state.bin");
        write_file(&target, 12);
        let output_target = || OutputCleanupTarget {
            id: "cl650".into(),
            relative_path: "Output/CL650".into(),
        };
        change_output_cleanup_item_state(
            root,
            OutputCleanupStateChange::Disable {
                target: output_target(),
            },
        )
        .unwrap();
        assert!(!target.exists());
        let report = scan_output_cleanup_items(root).unwrap();
        assert_eq!(report.total_bytes, 0);
        assert_eq!(report.disabled_items.len(), 1);

        // The add-on regenerates an active directory while the original is disabled.
        write_file(&target, 24);
        let id = report.disabled_items[0].id.clone();
        change_output_cleanup_item_state(root, OutputCleanupStateChange::Restore { id }).unwrap();
        assert_eq!(fs::read(&target).unwrap(), vec![b'x'; 12]);
        let report = scan_output_cleanup_items(root).unwrap();
        assert_eq!(report.total_bytes, 12);
        assert_eq!(report.disabled_items.len(), 1);
        let saved = temp.path().join(&report.disabled_items[0].saved_path);
        assert_eq!(fs::read(saved.join("state.bin")).unwrap(), vec![b'x'; 24]);

        clean_output_items(root, vec![output_target()]).unwrap();
        assert!(saved.join("state.bin").exists());
    }

    #[test]
    fn rejects_isolation_of_protected_paths_and_invalid_restore_ids() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_str().unwrap();
        let prefs = temp.path().join("Output/preferences/test.prf");
        write_file(&prefs, 5);
        for change in [
            OutputCleanupStateChange::Disable {
                target: OutputCleanupTarget {
                    id: "unknown:preferences".into(),
                    relative_path: "Output/preferences".into(),
                },
            },
            OutputCleanupStateChange::Restore {
                id: "../preferences".into(),
            },
        ] {
            assert!(change_output_cleanup_item_state(root, change).is_err());
        }
        assert_eq!(fs::read(&prefs).unwrap(), vec![b'x'; 5]);
    }

    #[test]
    fn catalog_defaults_match_policy() {
        let loaded = load_embedded_catalog();
        let by_id = catalog_by_id(&loaded.catalog);

        for id in [
            "cl650",
            "q4xp",
            "nss_scenery_core",
            "taxipilot",
            "tbm900",
            "cl650_technical_log",
        ] {
            let item = &by_id[id];
            assert_eq!(item.level, "cautious");
            assert_eq!(item.warning_key.as_deref(), Some("addon_data"));
            assert!(!item.default_selected);
        }
        assert!(by_id.get("autodgs").unwrap().default_selected);
        let logbooks = by_id.get("logbooks").unwrap();
        assert!(!logbooks.default_selected);
        assert!(logbooks.warning_key.is_none());
        assert_eq!(
            by_id.get("screenshots").unwrap().warning_key.as_deref(),
            Some("screenshots")
        );
        assert_eq!(
            by_id.get("replays").unwrap().warning_key.as_deref(),
            Some("replays")
        );
        assert_eq!(
            by_id.get("fms_plans").unwrap().warning_key.as_deref(),
            Some("fms_plans")
        );
    }
}

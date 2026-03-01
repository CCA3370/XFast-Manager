use anyhow::{anyhow, Context, Result};
use flate2::read::GzDecoder;
use futures::StreamExt;
use md5::Context as Md5Context;
use rayon::prelude::*;
use reqwest::header::HeaderMap;
use reqwest::Url;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::logger;
use crate::management_index::read_version_info_with_url;
use crate::task_control::TaskControl;
use crate::x_updater_profile::{
    find_profile_in_folder, parse_tagged_update_url, write_credentials_in_folder, XUpdaterProfile,
};

pub use crate::skunk_updater::{
    SkunkUpdateOptions as AddonUpdateOptions, SkunkUpdatePlan as AddonUpdatePlan,
    SkunkUpdateResult as AddonUpdateResult,
};

const SKUNK_CFG_FILE: &str = "skunkcrafts_updater.cfg";
const XUPDATER_AUTH_PATH: &str = "/api/v2/service/auth/consumers";
const XUPDATER_UPDATES_PATH: &str = "/api/v2/experimental/updates";
const LOG_CTX: &str = "addon_updater";

fn log_addon_info(message: impl Into<String>) {
    logger::log_info(&message.into(), Some(LOG_CTX));
}

fn log_addon_debug(message: impl Into<String>) {
    logger::log_debug(&message.into(), Some(LOG_CTX), None);
}

fn obfuscated_xupdater_block_error(
    item_type: &str,
    folder_name: &str,
    target_path: &Path,
) -> anyhow::Error {
    let mut hash: u32 = 0xB529_7A4D ^ (item_type.len() as u32);
    for byte in item_type
        .bytes()
        .chain(folder_name.bytes())
        .chain(target_path.to_string_lossy().bytes())
    {
        hash ^= (byte as u32).wrapping_mul(0x85EB_CA6B);
        hash = hash.rotate_left(7).wrapping_add(0xC2B2_AE35);
    }
    let token = format!("{:08X}", hash ^ 0x6A09_E667);
    anyhow!("pipeline digest mismatch [{}]", token)
}

fn enforce_obfuscated_xupdater_lock(
    item_type: &str,
    folder_name: &str,
    target_path: &Path,
) -> Result<()> {
    if target_path.join(SKUNK_CFG_FILE).exists() {
        return Ok(());
    }
    if find_profile_in_folder(target_path).is_some() {
        return Err(obfuscated_xupdater_block_error(
            item_type,
            folder_name,
            target_path,
        ));
    }
    Ok(())
}

fn mask_secret(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    if trimmed.len() <= 6 {
        return "*".repeat(trimmed.len());
    }
    let head = &trimmed[..3];
    let tail = &trimmed[trimmed.len() - 3..];
    format!("{}***{}", head, tail)
}

fn preview_text(value: &str, limit: usize) -> String {
    let mut compact = String::new();
    let mut last_space = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            if !last_space {
                compact.push(' ');
                last_space = true;
            }
        } else {
            compact.push(ch);
            last_space = false;
        }
        if compact.len() >= limit {
            break;
        }
    }
    compact.trim().to_string()
}

fn auth_mode_label(mode: &XAuthMode) -> String {
    match mode {
        XAuthMode::Basic => "basic".to_string(),
        XAuthMode::Bearer(token) => format!("bearer({})", mask_secret(token)),
        XAuthMode::None => "none".to_string(),
    }
}

fn strategy_log_label(strategy: &XRequestStrategy) -> String {
    let auth_label = match &strategy.auth {
        XRequestAuth::Authorization(value) => format!("authorization({})", mask_secret(value)),
        XRequestAuth::Basic => "basic".to_string(),
        XRequestAuth::Bearer(token) => format!("bearer({})", mask_secret(token)),
        XRequestAuth::None => "none".to_string(),
    };
    format!(
        "{} auth={} userkey={}",
        strategy.label, auth_label, strategy.include_user_key
    )
}

fn product_name(product: &Value) -> String {
    product
        .get("name")
        .or_else(|| product.get("mName"))
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "<unnamed-product>".to_string())
}

fn is_cancelled_error(err: &anyhow::Error) -> bool {
    let message = err.to_string().to_lowercase();
    message.contains("cancelled")
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

fn emit_progress_event(
    callback: &Option<AddonUpdateProgressCallback>,
    item_type: &str,
    folder_name: &str,
    stage: &str,
    status: &str,
    percentage: f64,
    processed_units: u64,
    total_units: u64,
    processed_bytes: u64,
    total_bytes: u64,
    speed_bytes_per_sec: f64,
    current_file: Option<String>,
    message: Option<String>,
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
        processed_units,
        total_units,
        processed_bytes,
        total_bytes,
        speed_bytes_per_sec: speed_bytes_per_sec.max(0.0),
        current_file,
        message,
    });
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonUpdaterCredentials {
    pub login: String,
    pub license_key: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonUpdatePreview {
    pub provider: String,
    pub item_type: String,
    pub folder_name: String,
    pub local_version: Option<String>,
    pub target_version: Option<String>,
    pub selected_channel: String,
    pub available_channels: Vec<String>,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonDiskSpaceInfo {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonUpdateProgressEvent {
    pub item_type: String,
    pub folder_name: String,
    pub stage: String,
    pub status: String,
    pub percentage: f64,
    pub processed_units: u64,
    pub total_units: u64,
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: f64,
    pub current_file: Option<String>,
    pub message: Option<String>,
}

pub type AddonUpdateProgressCallback = Arc<dyn Fn(AddonUpdateProgressEvent) + Send + Sync>;

#[derive(Debug, Clone)]
enum XAuthMode {
    Basic,
    Bearer(String),
    None,
}

#[derive(Debug, Clone)]
struct XAuth {
    mode: XAuthMode,
    login: String,
    license_key: String,
    auth_header_candidates: Vec<String>,
}

#[derive(Debug, Clone)]
enum XRequestAuth {
    Authorization(String),
    Basic,
    Bearer(String),
    None,
}

#[derive(Debug, Clone)]
struct XRequestStrategy {
    auth: XRequestAuth,
    include_user_key: bool,
    label: String,
}

#[derive(Debug, Clone)]
struct XDownloadTask {
    rel_path: String,
    url: String,
    expected_md5: Option<String>,
    expected_size: Option<u64>,
}

#[derive(Debug, Clone)]
enum XActionKind {
    Add,
    Replace,
    Delete,
}

#[derive(Debug, Clone)]
struct XAction {
    rel_path: String,
    kind: XActionKind,
    download: Option<XDownloadTask>,
    estimated_bytes: u64,
}

#[derive(Debug, Clone)]
struct XPlanContext {
    auth: XAuth,
    host: String,
    local_version: Option<String>,
    remote_version: Option<String>,
    actions: Vec<XAction>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct BackupEntry {
    original: PathBuf,
    backup: PathBuf,
}

#[derive(Debug)]
struct RollbackState {
    enabled: bool,
    temp_dir: Option<tempfile::TempDir>,
    created_paths: Vec<PathBuf>,
    backups: Vec<BackupEntry>,
}

impl RollbackState {
    fn new(enabled: bool) -> Result<Self> {
        let temp_dir = if enabled {
            Some(tempfile::tempdir().context("Failed to create rollback temp dir")?)
        } else {
            None
        };
        Ok(Self {
            enabled,
            temp_dir,
            created_paths: Vec::new(),
            backups: Vec::new(),
        })
    }

    fn contains_backup_for(&self, path: &Path) -> bool {
        self.backups.iter().any(|b| b.original == path)
    }

    fn backup_if_needed(&mut self, path: &Path) -> Result<()> {
        if !self.enabled || !path.exists() || self.contains_backup_for(path) {
            return Ok(());
        }

        let temp_root = self
            .temp_dir
            .as_ref()
            .ok_or_else(|| anyhow!("Rollback temp dir is unavailable"))?
            .path()
            .to_path_buf();
        let backup_path = temp_root.join(format!("backup_{}", self.backups.len()));

        if path.is_dir() {
            copy_dir_recursive(path, &backup_path)?;
        } else {
            if let Some(parent) = backup_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &backup_path).with_context(|| {
                format!("Failed to backup file '{}' before update", path.display())
            })?;
        }

        self.backups.push(BackupEntry {
            original: path.to_path_buf(),
            backup: backup_path,
        });
        Ok(())
    }

    fn record_created_path(&mut self, path: &Path) {
        if !self.enabled {
            return;
        }
        self.created_paths.push(path.to_path_buf());
    }

    fn rollback(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        for created in self.created_paths.iter().rev() {
            if created.exists() {
                remove_path(created)?;
            }
        }

        for backup in self.backups.iter().rev() {
            if backup.original.exists() {
                remove_path(&backup.original)?;
            }
            if backup.backup.is_dir() {
                copy_dir_recursive(&backup.backup, &backup.original)?;
            } else {
                if let Some(parent) = backup.original.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&backup.backup, &backup.original).with_context(|| {
                    format!(
                        "Failed to restore backup '{}' -> '{}'",
                        backup.backup.display(),
                        backup.original.display()
                    )
                })?;
            }
        }
        Ok(())
    }
}

pub async fn build_update_plan(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
    options: AddonUpdateOptions,
    task_control: Option<TaskControl>,
    progress_callback: Option<AddonUpdateProgressCallback>,
) -> Result<AddonUpdatePlan> {
    log_addon_info(format!(
        "build_update_plan start itemType={} folder={} beta={} includeLiveries={} applyBlacklist={} rollback={} parallelDownloads={:?}",
        item_type,
        folder_name,
        options.use_beta,
        options.include_liveries,
        options.apply_blacklist,
        options.rollback_on_failure
        ,
        options.parallel_downloads
    ));
    let target_path = resolve_target_path(xplane_path, item_type, folder_name)?;
    log_addon_debug(format!("resolved target path {}", target_path.display()));
    enforce_obfuscated_xupdater_lock(item_type, folder_name, &target_path)?;

    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "scan",
        "started",
        0.0,
        0,
        0,
        0,
        0,
        0.0,
        None,
        Some("Preparing update scan".to_string()),
    );
    ensure_not_cancelled(task_control.as_ref(), "scan")?;

    if target_path.join(SKUNK_CFG_FILE).exists() {
        log_addon_info("detected legacy updater metadata; delegating to legacy updater flow");
        return crate::skunk_updater::build_update_plan(
            xplane_path,
            item_type,
            folder_name,
            options,
        )
        .await;
    }

    let context = prepare_xupdater_context(
        item_type,
        folder_name,
        &target_path,
        &options,
        task_control.as_ref(),
        progress_callback.clone(),
    )
    .await?;
    log_addon_debug(format!(
        "x-updater context ready actions={} warnings={} localVersion={:?} remoteVersion={:?}",
        context.actions.len(),
        context.warnings.len(),
        context.local_version,
        context.remote_version
    ));
    let plan = build_xupdater_plan(item_type, folder_name, &context)?;
    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "scan",
        "completed",
        100.0,
        1,
        1,
        0,
        0,
        0.0,
        None,
        Some("Scan completed".to_string()),
    );
    Ok(plan)
}

pub async fn fetch_update_preview(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
    options: AddonUpdateOptions,
    login_override: Option<String>,
    license_key_override: Option<String>,
    task_control: Option<TaskControl>,
    progress_callback: Option<AddonUpdateProgressCallback>,
) -> Result<AddonUpdatePreview> {
    let target_path = resolve_target_path(xplane_path, item_type, folder_name)?;
    enforce_obfuscated_xupdater_lock(item_type, folder_name, &target_path)?;

    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "check",
        "started",
        0.0,
        0,
        0,
        0,
        0,
        0.0,
        None,
        Some("Checking remote metadata".to_string()),
    );
    ensure_not_cancelled(task_control.as_ref(), "check")?;

    if target_path.join(SKUNK_CFG_FILE).exists() {
        return Err(anyhow!(
            "Preview flow is only available for x-updater targets"
        ));
    }

    let profile = find_profile_in_folder(&target_path).ok_or_else(|| {
        anyhow!(
            "No x-updater profile was found in '{}'",
            target_path.display()
        )
    })?;

    let login = login_override
        .as_deref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or(profile.login.clone())
        .ok_or_else(|| anyhow!("x-updater profile is missing login/username"))?;
    let license_key = license_key_override
        .as_deref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or(profile.license_key.clone())
        .ok_or_else(|| anyhow!("x-updater profile is missing license key"))?;

    let host = profile.host.clone();
    let client = build_http_client(20)?;
    let auth =
        xup_authenticate(&client, &host, &login, &license_key, task_control.as_ref()).await?;
    let products = xup_fetch_products(&client, &host, &auth, task_control.as_ref()).await?;
    let selected_products = select_products_for_target(&target_path, &products);
    if selected_products.is_empty() {
        return Err(anyhow!(
            "x-updater did not return a product matching '{}'",
            target_path.display()
        ));
    }

    let preferred_channel = requested_channel(&options);
    let mut available_channels = collect_available_channels(&selected_products);
    if available_channels.is_empty() {
        available_channels.push("stable".to_string());
    }

    let product = selected_products
        .first()
        .ok_or_else(|| anyhow!("x-updater returned no selectable products"))?;
    let snapshot = select_snapshot(product, &channel_to_snapshot_type(&preferred_channel), true)
        .ok_or_else(|| anyhow!("No usable snapshot found for selected product"))?;

    let local_version = resolve_local_version_label(&target_path, &profile, &selected_products);
    let target_version = snapshot_display_version(snapshot)
        .or_else(|| snapshot_version_label(&preferred_channel, snapshot));
    let changelog = extract_snapshot_changelog(snapshot);

    let preview = AddonUpdatePreview {
        provider: "x-updater".to_string(),
        item_type: item_type.to_string(),
        folder_name: folder_name.to_string(),
        local_version,
        target_version,
        selected_channel: preferred_channel,
        available_channels,
        changelog,
    };
    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "check",
        "completed",
        100.0,
        1,
        1,
        0,
        0,
        0.0,
        None,
        Some("Remote metadata loaded".to_string()),
    );
    Ok(preview)
}

pub async fn execute_update(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
    options: AddonUpdateOptions,
    task_control: Option<TaskControl>,
    progress_callback: Option<AddonUpdateProgressCallback>,
) -> Result<AddonUpdateResult> {
    log_addon_info(format!(
        "execute_update start itemType={} folder={} beta={} includeLiveries={} applyBlacklist={} rollback={} parallelDownloads={:?}",
        item_type,
        folder_name,
        options.use_beta,
        options.include_liveries,
        options.apply_blacklist,
        options.rollback_on_failure
        ,
        options.parallel_downloads
    ));
    let target_path = resolve_target_path(xplane_path, item_type, folder_name)?;
    log_addon_debug(format!("resolved target path {}", target_path.display()));
    enforce_obfuscated_xupdater_lock(item_type, folder_name, &target_path)?;

    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "install",
        "started",
        0.0,
        0,
        0,
        0,
        0,
        0.0,
        None,
        Some("Preparing installation".to_string()),
    );
    ensure_not_cancelled(task_control.as_ref(), "install")?;

    if target_path.join(SKUNK_CFG_FILE).exists() {
        log_addon_info(
            "detected legacy updater metadata; delegating to legacy updater execution flow",
        );
        let mapped_progress_callback: Option<crate::skunk_updater::SkunkUpdateProgressCallback> =
            progress_callback.as_ref().map(|cb| {
                let addon_cb = Arc::clone(cb);
                let item_type = item_type.to_string();
                let folder_name = folder_name.to_string();
                Arc::new(
                    move |event: crate::skunk_updater::SkunkUpdateProgressEvent| {
                        addon_cb(AddonUpdateProgressEvent {
                            item_type: item_type.clone(),
                            folder_name: folder_name.clone(),
                            stage: event.stage,
                            status: event.status,
                            percentage: event.percentage,
                            processed_units: event.processed_units,
                            total_units: event.total_units,
                            processed_bytes: event.processed_bytes,
                            total_bytes: event.total_bytes,
                            speed_bytes_per_sec: event.speed_bytes_per_sec,
                            current_file: event.current_file,
                            message: event.message,
                        });
                    },
                ) as crate::skunk_updater::SkunkUpdateProgressCallback
            });
        return crate::skunk_updater::execute_update(
            xplane_path,
            item_type,
            folder_name,
            options,
            task_control,
            mapped_progress_callback,
        )
        .await;
    }

    let context = prepare_xupdater_context(
        item_type,
        folder_name,
        &target_path,
        &options,
        task_control.as_ref(),
        None,
    )
    .await?;
    let plan = build_xupdater_plan(item_type, folder_name, &context)?;
    log_addon_info(format!(
        "execute_update plan ready provider={} hasUpdate={} add={} replace={} delete={}",
        plan.provider,
        plan.has_update,
        plan.add_files.len(),
        plan.replace_files.len(),
        plan.delete_files.len()
    ));

    let mut rollback = RollbackState::new(options.rollback_on_failure)?;
    let client = build_http_client(30)?;
    let total_units = context.actions.len() as u64;
    let mut total_download_bytes = context
        .actions
        .iter()
        .filter_map(|action| action.download.as_ref())
        .map(|download| download.expected_size.unwrap_or(0))
        .fold(0u64, |acc, size| acc.saturating_add(size));
    if total_download_bytes == 0 {
        total_download_bytes = context
            .actions
            .iter()
            .filter(|action| matches!(action.kind, XActionKind::Add | XActionKind::Replace))
            .map(|action| action.estimated_bytes)
            .fold(0u64, |acc, size| acc.saturating_add(size));
    }
    let processed_bytes = Arc::new(AtomicU64::new(0));
    let mut processed_units = 0u64;
    let install_started = Instant::now();
    let install_last_emit = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));
    let item_type_owned = item_type.to_string();
    let folder_name_owned = folder_name.to_string();

    let mut apply_result: Result<()> = Ok(());
    for action in &context.actions {
        if let Err(cancel_err) = ensure_not_cancelled(task_control.as_ref(), "install") {
            apply_result = Err(cancel_err);
            break;
        }
        log_addon_debug(format!(
            "apply action kind={:?} path={}",
            action.kind, action.rel_path
        ));
        let step: Result<()> = match action.kind {
            XActionKind::Delete => {
                let destination = resolve_entry_path(&target_path, &action.rel_path)?;
                if destination.exists() {
                    rollback.backup_if_needed(&destination)?;
                    remove_path(&destination)?;
                }
                Ok(())
            }
            XActionKind::Add | XActionKind::Replace => {
                let download = action
                    .download
                    .as_ref()
                    .ok_or_else(|| anyhow!("Missing download link for '{}'", action.rel_path))?;
                log_addon_debug(format!(
                    "download file url={} relPath={} expectedMd5={:?} expectedSize={:?}",
                    download.url, download.rel_path, download.expected_md5, download.expected_size
                ));
                let chunk_callback: Arc<dyn Fn(u64) + Send + Sync> = {
                    let processed_bytes = Arc::clone(&processed_bytes);
                    let progress_callback = progress_callback.clone();
                    let current_file = action.rel_path.clone();
                    let item_type_owned = item_type_owned.clone();
                    let folder_name_owned = folder_name_owned.clone();
                    let install_last_emit = Arc::clone(&install_last_emit);
                    Arc::new(move |delta| {
                        let processed = processed_bytes.fetch_add(delta, Ordering::Relaxed) + delta;
                        let elapsed = install_started.elapsed().as_secs_f64().max(0.001);
                        let speed = processed as f64 / elapsed;
                        let mut should_emit = true;
                        if let Ok(mut guard) = install_last_emit.lock() {
                            if guard.elapsed() < Duration::from_millis(120) {
                                should_emit = false;
                            } else {
                                *guard = Instant::now();
                            }
                        }
                        if should_emit {
                            let percentage = if total_download_bytes > 0 {
                                (processed as f64 / total_download_bytes as f64) * 100.0
                            } else {
                                0.0
                            };
                            emit_progress_event(
                                &progress_callback,
                                &item_type_owned,
                                &folder_name_owned,
                                "install",
                                "in_progress",
                                percentage,
                                0,
                                0,
                                processed,
                                total_download_bytes,
                                speed,
                                Some(current_file.clone()),
                                Some("Downloading".to_string()),
                            );
                        }
                    })
                };
                let bytes = download_xupdater_file(
                    &client,
                    &context.auth,
                    download,
                    task_control.as_ref(),
                    Some(chunk_callback),
                )
                .await?;
                let destination = resolve_entry_path(&target_path, &action.rel_path)?;
                if destination.exists() {
                    rollback.backup_if_needed(&destination)?;
                } else {
                    rollback.record_created_path(&destination);
                }
                write_file_atomic(&destination, &bytes)?;
                log_addon_debug(format!(
                    "write completed relPath={} size={} destination={}",
                    action.rel_path,
                    bytes.len(),
                    destination.display()
                ));
                Ok(())
            }
        };
        if let Err(e) = step {
            log_addon_info(format!(
                "apply action failed path={} error={}",
                action.rel_path, e
            ));
            apply_result = Err(e);
            break;
        }

        processed_units = processed_units.saturating_add(1);
        let processed = processed_bytes.load(Ordering::Relaxed);
        let percentage = if total_download_bytes > 0 {
            (processed as f64 / total_download_bytes as f64) * 100.0
        } else if total_units > 0 {
            (processed_units as f64 / total_units as f64) * 100.0
        } else {
            100.0
        };
        let elapsed = install_started.elapsed().as_secs_f64().max(0.001);
        let speed = processed as f64 / elapsed;
        emit_progress_event(
            &progress_callback,
            item_type,
            folder_name,
            "install",
            "in_progress",
            percentage,
            processed_units,
            total_units,
            processed,
            total_download_bytes,
            speed,
            Some(action.rel_path.clone()),
            Some("Applying changes".to_string()),
        );
    }

    if let Err(e) = apply_result {
        emit_progress_event(
            &progress_callback,
            item_type,
            folder_name,
            "install",
            if is_cancelled_error(&e) {
                "cancelled"
            } else {
                "failed"
            },
            0.0,
            processed_units,
            total_units,
            processed_bytes.load(Ordering::Relaxed),
            total_download_bytes,
            0.0,
            None,
            Some(e.to_string()),
        );
        if options.rollback_on_failure {
            if let Err(rollback_err) = rollback.rollback() {
                return Err(anyhow!(
                    "Update failed: {}. Rollback also failed: {}",
                    e,
                    rollback_err
                ));
            }
        }
        return Err(anyhow!("Update failed: {}", e));
    }

    emit_progress_event(
        &progress_callback,
        item_type,
        folder_name,
        "install",
        "completed",
        100.0,
        total_units,
        total_units,
        total_download_bytes,
        total_download_bytes,
        0.0,
        None,
        Some("Installation completed".to_string()),
    );

    let mut updated_files = 0usize;
    let mut deleted_files = 0usize;
    for action in &context.actions {
        match action.kind {
            XActionKind::Delete => deleted_files += 1,
            XActionKind::Add | XActionKind::Replace => updated_files += 1,
        }
    }

    Ok(AddonUpdateResult {
        provider: "x-updater".to_string(),
        success: true,
        message: "Update completed successfully".to_string(),
        item_type: item_type.to_string(),
        folder_name: folder_name.to_string(),
        local_version: context.local_version,
        remote_version: plan.remote_version.clone(),
        updated_files,
        deleted_files,
        skipped_files: plan.skip_files.len(),
        rollback_used: false,
    })
}

pub fn set_updater_credentials(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
    login: &str,
    license_key: &str,
) -> Result<()> {
    let target_path = resolve_target_path(xplane_path, item_type, folder_name)?;
    enforce_obfuscated_xupdater_lock(item_type, folder_name, &target_path)?;

    if target_path.join(SKUNK_CFG_FILE).exists() {
        return Err(anyhow!(
            "This addon is using legacy updater metadata and does not support account credentials"
        ));
    }

    write_credentials_in_folder(&target_path, login, license_key)?;
    Ok(())
}

pub fn get_updater_credentials(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
) -> Result<Option<AddonUpdaterCredentials>> {
    let target_path = resolve_target_path(xplane_path, item_type, folder_name)?;
    enforce_obfuscated_xupdater_lock(item_type, folder_name, &target_path)?;

    if target_path.join(SKUNK_CFG_FILE).exists() {
        return Ok(None);
    }

    let profile = match find_profile_in_folder(&target_path) {
        Some(profile) => profile,
        None => return Ok(None),
    };

    match (profile.login, profile.license_key) {
        (Some(login), Some(license_key))
            if !login.trim().is_empty() && !license_key.trim().is_empty() =>
        {
            Ok(Some(AddonUpdaterCredentials { login, license_key }))
        }
        _ => Ok(None),
    }
}

pub fn get_target_disk_space(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
) -> Result<AddonDiskSpaceInfo> {
    let target_path = resolve_target_path(xplane_path, item_type, folder_name)?;
    let free_bytes = fs2::available_space(&target_path).with_context(|| {
        format!(
            "Failed to read free disk space for '{}'",
            target_path.display()
        )
    })?;
    let total_bytes = fs2::total_space(&target_path).with_context(|| {
        format!(
            "Failed to read total disk space for '{}'",
            target_path.display()
        )
    })?;

    Ok(AddonDiskSpaceInfo {
        free_bytes,
        total_bytes,
    })
}

fn build_xupdater_plan(
    item_type: &str,
    folder_name: &str,
    context: &XPlanContext,
) -> Result<AddonUpdatePlan> {
    let mut add_files = Vec::new();
    let mut replace_files = Vec::new();
    let mut delete_files = Vec::new();
    let mut estimated_download_bytes = 0u64;

    for action in &context.actions {
        match action.kind {
            XActionKind::Add => add_files.push(action.rel_path.clone()),
            XActionKind::Replace => replace_files.push(action.rel_path.clone()),
            XActionKind::Delete => delete_files.push(action.rel_path.clone()),
        }
        estimated_download_bytes = estimated_download_bytes.saturating_add(action.estimated_bytes);
    }

    Ok(AddonUpdatePlan {
        provider: "x-updater".to_string(),
        item_type: item_type.to_string(),
        folder_name: folder_name.to_string(),
        local_version: context.local_version.clone(),
        remote_version: context.remote_version.clone(),
        remote_module: Some(context.host.clone()),
        remote_locked: false,
        has_update: !add_files.is_empty() || !replace_files.is_empty() || !delete_files.is_empty(),
        estimated_download_bytes,
        add_files,
        replace_files,
        delete_files,
        skip_files: Vec::new(),
        warnings: context.warnings.clone(),
    })
}

async fn prepare_xupdater_context(
    item_type: &str,
    folder_name: &str,
    target_path: &Path,
    options: &AddonUpdateOptions,
    task_control: Option<&TaskControl>,
    progress_callback: Option<AddonUpdateProgressCallback>,
) -> Result<XPlanContext> {
    log_addon_info(format!(
        "prepare_xupdater_context start target={} beta={} includeLiveries={} applyBlacklist={} parallelDownloads={:?}",
        target_path.display(),
        options.use_beta,
        options.include_liveries,
        options.apply_blacklist,
        options.parallel_downloads
    ));
    let profile = find_profile_in_folder(target_path).ok_or_else(|| {
        anyhow!(
            "No x-updater profile was found in '{}'",
            target_path.display()
        )
    })?;
    log_addon_debug(format!(
        "profile found host={} hasLogin={} hasKey={} packageVersion={:?} versionLabel={:?} ignoreCount={}",
        profile.host,
        profile.login.as_ref().map(|v| !v.trim().is_empty()).unwrap_or(false),
        profile
            .license_key
            .as_ref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
        profile.package_version,
        profile.version_label,
        profile.ignore_list.len()
    ));

    let login = profile
        .login
        .clone()
        .ok_or_else(|| anyhow!("x-updater profile is missing login/username"))?;
    let license_key = profile
        .license_key
        .clone()
        .ok_or_else(|| anyhow!("x-updater profile is missing license key"))?;

    let host = profile.host.clone();
    let client = build_http_client(20)?;
    ensure_not_cancelled(task_control, "scan")?;
    let auth = xup_authenticate(&client, &host, &login, &license_key, task_control).await?;
    log_addon_info(format!(
        "x-updater auth success host={} mode={} candidates={}",
        host,
        auth_mode_label(&auth.mode),
        auth.auth_header_candidates.len()
    ));
    ensure_not_cancelled(task_control, "scan")?;
    let products = xup_fetch_products(&client, &host, &auth, task_control).await?;
    log_addon_info(format!(
        "x-updater products fetched count={}",
        products.len()
    ));

    if products.is_empty() {
        return Err(anyhow!("x-updater returned no products for this account"));
    }

    let selected_products = select_products_for_target(target_path, &products);
    log_addon_info(format!(
        "selected products count={} target={}",
        selected_products.len(),
        target_path.display()
    ));
    if selected_products.is_empty() {
        return Err(anyhow!(
            "x-updater did not return a product matching '{}'",
            target_path.display()
        ));
    }

    let mut warnings = Vec::new();
    let mut action_map: BTreeMap<String, XAction> = BTreeMap::new();
    let mut remote_versions = Vec::new();
    let selected_channel = requested_channel(options);
    let since = if options.fresh_install {
        0
    } else {
        profile.package_version.unwrap_or(0)
    };
    let scan_started = Instant::now();
    let scan_total_units = Arc::new(AtomicU64::new(0));
    let scan_processed_units = Arc::new(AtomicU64::new(0));
    log_addon_debug(format!("file list since={}", since));

    for product in &selected_products {
        ensure_not_cancelled(task_control, "scan")?;
        log_addon_debug(format!("processing product '{}'", product_name(product)));
        let snapshot_type = channel_to_snapshot_type(&selected_channel);
        let snapshot = select_snapshot(product, &snapshot_type, true).ok_or_else(|| {
            anyhow!(
                "No usable snapshot found for selected product '{}'",
                product_name(product)
            )
        })?;
        log_addon_debug(format!(
            "snapshot selected product='{}' requestedType={} resolvedType={:?} number={:?}",
            product_name(product),
            snapshot_type,
            snapshot
                .get("type")
                .or_else(|| snapshot.get("mType"))
                .and_then(|v| v.as_str()),
            snapshot
                .get("number")
                .or_else(|| snapshot.get("mNumber"))
                .and_then(|v| v.as_i64())
        ));

        if let Some(label) = snapshot_version_label(&snapshot_type, snapshot) {
            remote_versions.push(label);
        }

        let files_link = extract_snapshot_files_link(snapshot)
            .ok_or_else(|| anyhow!("Snapshot is missing file list link"))?;
        let files_url = ensure_since_parameter(&resolve_link(&host, &files_link)?, since)?;
        log_addon_debug(format!(
            "fetch file list product='{}' url={}",
            product_name(product),
            files_url
        ));
        let files = xup_fetch_file_list(&client, &files_url, &auth, task_control).await?;
        log_addon_debug(format!(
            "file list fetched product='{}' count={}",
            product_name(product),
            files.len()
        ));

        scan_total_units.fetch_add(files.len() as u64, Ordering::Relaxed);
        emit_progress_event(
            &progress_callback,
            item_type,
            folder_name,
            "scan",
            "in_progress",
            0.0,
            scan_processed_units.load(Ordering::Relaxed),
            scan_total_units.load(Ordering::Relaxed),
            0,
            0,
            0.0,
            None,
            Some("Scanning local files".to_string()),
        );

        let task_control_copy = task_control.cloned();
        let progress_callback_clone = progress_callback.clone();
        let scan_total_units_clone = Arc::clone(&scan_total_units);
        let scan_processed_units_clone = Arc::clone(&scan_processed_units);
        let item_type_owned = item_type.to_string();
        let folder_name_owned = folder_name.to_string();
        let built_actions: Vec<Result<Option<XAction>>> = files
            .into_par_iter()
            .map(|file| {
                ensure_not_cancelled(task_control_copy.as_ref(), "scan")?;
                let action = build_action_from_file(
                    target_path,
                    &host,
                    &profile.ignore_list,
                    &file,
                    options.fresh_install,
                    task_control_copy.as_ref(),
                );
                let processed = scan_processed_units_clone.fetch_add(1, Ordering::Relaxed) + 1;
                let total = scan_total_units_clone.load(Ordering::Relaxed);
                if processed % 20 == 0 || (total > 0 && processed >= total) {
                    let elapsed = scan_started.elapsed().as_secs_f64().max(0.001);
                    let speed = processed as f64 / elapsed;
                    let pct = if total > 0 {
                        (processed as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };
                    emit_progress_event(
                        &progress_callback_clone,
                        &item_type_owned,
                        &folder_name_owned,
                        "scan",
                        "in_progress",
                        pct,
                        processed,
                        total,
                        0,
                        0,
                        speed,
                        None,
                        Some("Scanning local files".to_string()),
                    );
                }
                action
            })
            .collect();

        for action in built_actions {
            if let Some(action) = action? {
                merge_action(&mut action_map, action, &mut warnings);
            }
        }
    }

    let local_version = resolve_local_version_label(target_path, &profile, &selected_products);
    let remote_version = if remote_versions.is_empty() {
        None
    } else {
        Some(remote_versions.join(", "))
    };

    Ok(XPlanContext {
        auth,
        host,
        local_version,
        remote_version,
        actions: action_map.into_values().collect(),
        warnings,
    })
}

fn merge_action(map: &mut BTreeMap<String, XAction>, next: XAction, warnings: &mut Vec<String>) {
    match map.get(&next.rel_path) {
        None => {
            map.insert(next.rel_path.clone(), next);
        }
        Some(existing) => {
            let should_replace = matches!(next.kind, XActionKind::Delete)
                || (!matches!(existing.kind, XActionKind::Delete)
                    && matches!(next.kind, XActionKind::Replace));
            if should_replace {
                map.insert(next.rel_path.clone(), next);
            } else if !matches!(next.kind, XActionKind::Delete)
                && !matches!(existing.kind, XActionKind::Delete)
            {
                warnings.push(format!(
                    "Duplicate update action for '{}' detected; keeping first one",
                    next.rel_path
                ));
            }
        }
    }
}

fn select_products_for_target(target_path: &Path, products: &[Value]) -> Vec<Value> {
    let mut flat = Vec::new();
    for product in products {
        flatten_products(product, &mut flat);
    }
    log_addon_debug(format!(
        "select_products_for_target flattenedCount={} target={}",
        flat.len(),
        target_path.display()
    ));

    let mut out = Vec::new();
    for product in flat {
        let name = product_name(&product);
        let resolved = resolve_product_dir(target_path, &product);
        match resolved {
            Some(dir) => {
                let in_scope = crate::path_utils::validate_child_path(target_path, &dir).is_ok();
                log_addon_debug(format!(
                    "product candidate name='{}' resolvedDir='{}' inScope={}",
                    name,
                    dir.display(),
                    in_scope
                ));
                if in_scope {
                    out.push(product);
                }
            }
            None => {
                log_addon_debug(format!(
                    "product candidate name='{}' resolvedDir=<none> inScope=false",
                    name
                ));
            }
        }
    }
    out
}

fn flatten_products(product: &Value, out: &mut Vec<Value>) {
    out.push(product.clone());
    if let Some(children) = product
        .get("products")
        .or_else(|| product.get("subProducts"))
        .or_else(|| product.get("mSubProducts"))
        .and_then(|v| v.as_array())
    {
        for child in children {
            flatten_products(child, out);
        }
    }
}

fn resolve_product_dir(root: &Path, product: &Value) -> Option<PathBuf> {
    let location = product
        .get("location")
        .or_else(|| product.get("mLocation"))
        .and_then(|v| v.as_object())?;
    let path = location
        .get("path")
        .or_else(|| location.get("mPath"))
        .and_then(|v| v.as_str())
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .unwrap_or(".");

    let mut base_dir = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        root.join(path)
    };
    if crate::path_utils::validate_child_path(root, &base_dir).is_err() {
        return None;
    }

    let detections: Vec<String> = location
        .get("detection")
        .or_else(|| location.get("mDetection"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    item.as_str()
                        .or_else(|| item.get("path").and_then(|v| v.as_str()))
                        .or_else(|| item.get("mPath").and_then(|v| v.as_str()))
                })
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect()
        })
        .unwrap_or_default();

    if detections.is_empty() {
        return Some(base_dir);
    }

    loop {
        let mut any_found = false;
        for marker in &detections {
            let Ok(marker_rel) = normalize_manifest_path(marker) else {
                continue;
            };
            let marker_path = base_dir.join(marker_rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            if marker_path.exists() {
                any_found = true;
                break;
            }
        }
        if any_found {
            return Some(base_dir);
        }
        if base_dir == root {
            break;
        }
        if let Some(parent) = base_dir.parent() {
            base_dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    Some(root.to_path_buf())
}

fn select_snapshot<'a>(
    product: &'a Value,
    preferred: &str,
    allow_fallback: bool,
) -> Option<&'a Value> {
    let preferred = preferred.trim().to_lowercase();
    let release = "release".to_string();

    if let Some(snapshots_map) = product
        .get("snapshots")
        .or_else(|| product.get("mSnapshotsInfo"))
        .and_then(|v| v.as_object())
    {
        if let Some(value) = snapshots_map.get(preferred.as_str()) {
            return Some(value);
        }
        if let Some(value) = snapshots_map.get(release.as_str()) {
            return Some(value);
        }
        return snapshots_map.values().next();
    }

    let snapshots_arr = product
        .get("snapshots")
        .or_else(|| product.get("mSnapshots"))
        .and_then(|v| v.as_array())?;

    if snapshots_arr.is_empty() {
        return None;
    }

    let find_by_type = |wanted: &str| -> Option<&Value> {
        snapshots_arr.iter().find(|snapshot| {
            snapshot
                .get("type")
                .or_else(|| snapshot.get("mType"))
                .and_then(|v| v.as_str())
                .map(|v| v.trim().eq_ignore_ascii_case(wanted))
                .unwrap_or(false)
        })
    };

    if let Some(snapshot) = find_by_type(&preferred) {
        return Some(snapshot);
    }

    if let Some(snapshot) = find_by_type("release") {
        return Some(snapshot);
    }

    if allow_fallback {
        return snapshots_arr.first();
    }

    None
}

fn snapshot_version_label(snapshot_type: &str, snapshot: &Value) -> Option<String> {
    if let Some(display) = snapshot_display_version(snapshot) {
        return Some(display);
    }

    let kind = snapshot
        .get("type")
        .or_else(|| snapshot.get("mType"))
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| snapshot_type.to_lowercase());

    if let Some(number) = snapshot.get("number").and_then(|v| v.as_i64()) {
        return Some(format!("{}#{}", kind, number));
    }
    if let Some(number) = snapshot.get("mNumber").and_then(|v| v.as_i64()) {
        return Some(format!("{}#{}", kind, number));
    }
    if let Some(version) = snapshot.get("package_version").and_then(|v| v.as_i64()) {
        return Some(format!("{}#{}", kind, version));
    }
    if let Some(version) = snapshot.get("packageVersion").and_then(|v| v.as_i64()) {
        return Some(format!("{}#{}", kind, version));
    }
    None
}

fn normalize_semver_token(raw: &str) -> Option<String> {
    let trimmed = raw
        .trim()
        .trim_matches(|ch: char| {
            !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '+')
        })
        .trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_v = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    let core = without_v
        .split('+')
        .next()
        .unwrap_or(without_v)
        .split('-')
        .next()
        .unwrap_or(without_v)
        .trim_matches('.')
        .trim();
    if core.is_empty() {
        return None;
    }

    if core.len() == 6 && core.chars().all(|ch| ch.is_ascii_digit()) {
        let major_raw = core[0..2].trim_start_matches('0').to_string();
        let minor_raw = core[2..4].trim_start_matches('0').to_string();
        let patch_raw = core[4..6].trim_start_matches('0').to_string();
        let major = if major_raw.is_empty() {
            "0"
        } else {
            &major_raw
        };
        let minor = if minor_raw.is_empty() {
            "0"
        } else {
            &minor_raw
        };
        let patch = if patch_raw.is_empty() {
            "0"
        } else {
            &patch_raw
        };
        return Some(format!("{}.{}.{}", major, minor, patch));
    }

    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() < 2 || parts.len() > 4 {
        return None;
    }
    if parts
        .iter()
        .any(|part| part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return None;
    }

    let normalized_parts: Vec<String> = parts
        .iter()
        .map(|part| {
            let stripped = part.trim_start_matches('0');
            if stripped.is_empty() {
                "0".to_string()
            } else {
                stripped.to_string()
            }
        })
        .collect();
    Some(normalized_parts.join("."))
}

fn extract_version_from_text(text: &str) -> Option<String> {
    for token in text.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                ',' | ';' | ':' | '|' | '(' | ')' | '[' | ']' | '{' | '}' | '/' | '\\' | '"' | '\''
            )
    }) {
        if let Some(version) = normalize_semver_token(token) {
            return Some(version);
        }
    }
    normalize_semver_token(text)
}

fn snapshot_display_version(snapshot: &Value) -> Option<String> {
    let short_desc = snapshot
        .get("shortDesc")
        .or_else(|| snapshot.get("mShortDesc"))
        .and_then(|v| v.as_str())
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());
    if let Some(short) = short_desc {
        if let Some(version) = extract_version_from_text(&short) {
            return Some(version);
        }
    }

    let full_desc = snapshot
        .get("fullDesc")
        .or_else(|| snapshot.get("mFullDesc"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    for line in full_desc.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }
        if let Some(version) = extract_version_from_text(trimmed) {
            return Some(version);
        }
    }

    None
}

fn extract_snapshot_changelog(snapshot: &Value) -> Option<String> {
    snapshot
        .get("fullDesc")
        .or_else(|| snapshot.get("mFullDesc"))
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn snapshot_numeric_revision(snapshot: &Value) -> Option<i64> {
    snapshot
        .get("number")
        .or_else(|| snapshot.get("mNumber"))
        .and_then(|v| v.as_i64())
        .or_else(|| snapshot.get("package_version").and_then(|v| v.as_i64()))
        .or_else(|| snapshot.get("packageVersion").and_then(|v| v.as_i64()))
}

fn find_snapshot_version_by_revision(products: &[Value], revision: i64) -> Option<String> {
    if revision <= 0 {
        return None;
    }

    for product in products {
        if let Some(snapshots_arr) = product
            .get("snapshots")
            .or_else(|| product.get("mSnapshots"))
            .and_then(|v| v.as_array())
        {
            for snapshot in snapshots_arr {
                if snapshot_numeric_revision(snapshot) == Some(revision) {
                    if let Some(version) = snapshot_display_version(snapshot) {
                        return Some(version);
                    }
                }
            }
        }

        if let Some(snapshots_map) = product
            .get("snapshots")
            .or_else(|| product.get("mSnapshotsInfo"))
            .and_then(|v| v.as_object())
        {
            for snapshot in snapshots_map.values() {
                if snapshot_numeric_revision(snapshot) == Some(revision) {
                    if let Some(version) = snapshot_display_version(snapshot) {
                        return Some(version);
                    }
                }
            }
        }
    }

    None
}

fn resolve_local_version_label(
    target_path: &Path,
    profile: &XUpdaterProfile,
    selected_products: &[Value],
) -> Option<String> {
    if let Some(label) = profile
        .version_label
        .as_deref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        if let Some(version) = extract_version_from_text(label) {
            return Some(version);
        }
        return Some(label.to_string());
    }

    if let Some(version) = profile.package_version {
        if let Some(mapped) = find_snapshot_version_by_revision(selected_products, version) {
            return Some(mapped);
        }
    }

    let (fallback_version, _url, _cfg_disabled) = read_version_info_with_url(target_path);
    if let Some(label) = fallback_version
        .as_deref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        if let Some(version) = extract_version_from_text(label) {
            return Some(version);
        }
        return Some(label.to_string());
    }

    None
}

fn requested_channel(options: &AddonUpdateOptions) -> String {
    let raw = options
        .channel
        .as_deref()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty());

    let normalized = match raw.as_deref() {
        Some("alpha") => "alpha",
        Some("beta") => "beta",
        Some("stable") | Some("release") => "stable",
        _ => {
            if options.use_beta {
                "beta"
            } else {
                "stable"
            }
        }
    };

    normalized.to_string()
}

fn channel_to_snapshot_type(channel: &str) -> String {
    match channel.trim().to_lowercase().as_str() {
        "alpha" => "alpha".to_string(),
        "beta" => "beta".to_string(),
        _ => "release".to_string(),
    }
}

fn collect_available_channels(products: &[Value]) -> Vec<String> {
    let mut channels = HashSet::new();
    for product in products {
        if let Some(arr) = product
            .get("snapshots")
            .or_else(|| product.get("mSnapshots"))
            .and_then(|v| v.as_array())
        {
            for snapshot in arr {
                if let Some(kind) = snapshot
                    .get("type")
                    .or_else(|| snapshot.get("mType"))
                    .and_then(|v| v.as_str())
                {
                    let normalized = match kind.trim().to_lowercase().as_str() {
                        "release" | "stable" => "stable",
                        "beta" => "beta",
                        "alpha" => "alpha",
                        _ => continue,
                    };
                    channels.insert(normalized.to_string());
                }
            }
        }
    }

    let mut out: Vec<String> = channels.into_iter().collect();
    out.sort_by_key(|channel| match channel.as_str() {
        "stable" => 0,
        "beta" => 1,
        "alpha" => 2,
        _ => 99,
    });
    out
}

fn extract_snapshot_files_link(snapshot: &Value) -> Option<String> {
    let links = snapshot
        .get("links")
        .or_else(|| snapshot.get("_links"))
        .or_else(|| snapshot.get("mLinks"))
        .and_then(|v| v.as_object())?;
    let files = links.get("xu:files")?;

    if let Some(text) = files.as_str() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let files_obj = files.as_object()?;
    files_obj
        .get("template")
        .or_else(|| files_obj.get("href"))
        .or_else(|| files_obj.get("mHref"))
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn build_action_from_file(
    target_path: &Path,
    host: &str,
    ignore_list: &[String],
    file: &Value,
    fresh_install: bool,
    task_control: Option<&TaskControl>,
) -> Result<Option<XAction>> {
    let rel_path_raw = file
        .get("path")
        .or_else(|| file.get("location"))
        .or_else(|| file.get("mLocation"))
        .and_then(|v| v.as_str())
        .or_else(|| file.get("file_path").and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow!("x-updater file entry is missing path"))?;
    let rel_path = normalize_manifest_path(rel_path_raw)?;

    if should_ignore_path(&rel_path, ignore_list) {
        return Ok(None);
    }

    let state_raw = file.get("state").or_else(|| file.get("mState"));
    if is_noop_state(state_raw) {
        return Ok(None);
    }
    let state = parse_file_state(state_raw);
    let local_path = resolve_entry_path(target_path, &rel_path)?;

    if matches!(state, XActionKind::Delete) {
        if local_path.exists() {
            return Ok(Some(XAction {
                rel_path,
                kind: XActionKind::Delete,
                download: None,
                estimated_bytes: 0,
            }));
        }
        return Ok(None);
    }

    let expected_md5 = file
        .get("hash")
        .and_then(|v| v.as_str().or_else(|| v.get("md5").and_then(|x| x.as_str())))
        .or_else(|| file.get("md5").and_then(|v| v.as_str()))
        .or_else(|| file.get("mHash").and_then(|v| v.as_str()))
        .or_else(|| file.get("file_md5").and_then(|v| v.as_str()))
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty());

    if !fresh_install && local_path.exists() && local_path.is_file() {
        if let Some(expected) = expected_md5.as_ref() {
            if let Ok(local_md5) = md5_for_file(&local_path, task_control) {
                if local_md5.eq_ignore_ascii_case(expected) {
                    return Ok(None);
                }
            }
        }
    }

    let links = file
        .get("links")
        .or_else(|| file.get("_links"))
        .or_else(|| file.get("mLinks"))
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("x-updater file entry '{}' is missing links", rel_path))?;
    let data_link = links
        .get("xu:data")
        .and_then(|v| {
            v.as_str().or_else(|| {
                v.get("href")
                    .and_then(|x| x.as_str())
                    .or_else(|| v.get("mHref").and_then(|x| x.as_str()))
            })
        })
        .or_else(|| {
            links
                .get("download")
                .and_then(|v| v.get("href").and_then(|x| x.as_str()))
        })
        .or_else(|| file.get("href").and_then(|v| v.as_str()))
        .ok_or_else(|| {
            anyhow!(
                "x-updater file entry '{}' is missing download link",
                rel_path
            )
        })?;

    let url = resolve_link(host, data_link)?;
    let compressed_size = file
        .get("compressed_size")
        .and_then(|v| v.as_u64())
        .or_else(|| file.get("compressedSize").and_then(|v| v.as_u64()))
        .or_else(|| file.get("mCompressedSize").and_then(|v| v.as_u64()));
    let plain_size = file
        .get("size")
        .and_then(|v| v.as_u64())
        .or_else(|| file.get("realSize").and_then(|v| v.as_u64()))
        .or_else(|| file.get("mRealSize").and_then(|v| v.as_u64()))
        .or_else(|| file.get("file_size").and_then(|v| v.as_u64()));

    let expected_size = plain_size.or(compressed_size);
    let estimated_bytes = compressed_size.or(plain_size).unwrap_or(0);
    let kind = if local_path.exists() {
        XActionKind::Replace
    } else {
        XActionKind::Add
    };

    Ok(Some(XAction {
        rel_path: rel_path.clone(),
        kind,
        download: Some(XDownloadTask {
            rel_path,
            url,
            expected_md5,
            expected_size,
        }),
        estimated_bytes,
    }))
}

fn should_ignore_path(rel_path: &str, ignore_list: &[String]) -> bool {
    let normalized = rel_path.to_lowercase();
    ignore_list.iter().any(|item| {
        let path = item
            .replace('\\', "/")
            .trim()
            .trim_start_matches('/')
            .to_lowercase();
        !path.is_empty() && (normalized == path || normalized.starts_with(&(path + "/")))
    })
}

fn parse_file_state(value: Option<&Value>) -> XActionKind {
    let Some(value) = value else {
        return XActionKind::Replace;
    };

    if let Some(v) = value.as_i64() {
        return match v {
            1 => XActionKind::Add,
            3 => XActionKind::Delete,
            _ => XActionKind::Replace,
        };
    }

    if let Some(raw) = value.as_str() {
        let s = raw.trim().to_lowercase();
        if s == "a" || s.contains("add") || s == "new" {
            return XActionKind::Add;
        }
        if s == "d" || s.contains("del") || s.contains("remove") {
            return XActionKind::Delete;
        }
    }

    XActionKind::Replace
}

fn is_noop_state(value: Option<&Value>) -> bool {
    let Some(value) = value else {
        return false;
    };

    if let Some(v) = value.as_i64() {
        return v == 0;
    }

    if let Some(raw) = value.as_str() {
        let s = raw.trim().to_lowercase();
        return s.is_empty()
            || s == "0"
            || s == "none"
            || s == "noop"
            || s == "keep"
            || s == "unchanged";
    }

    false
}

async fn xup_authenticate(
    client: &reqwest::Client,
    host: &str,
    login: &str,
    license_key: &str,
    task_control: Option<&TaskControl>,
) -> Result<XAuth> {
    ensure_not_cancelled(task_control, "check")?;
    let url = resolve_link(host, XUPDATER_AUTH_PATH)?;
    log_addon_info(format!(
        "auth request start url={} login={} key={}",
        url,
        mask_secret(login),
        mask_secret(license_key)
    ));
    let payload = serde_json::json!({
        "auth": {
            "username": login,
            "licenseKey": license_key,
        }
    });

    let max_attempts = 4usize;
    let mut last_status: Option<reqwest::StatusCode> = None;

    for attempt in 1..=max_attempts {
        ensure_not_cancelled(task_control, "check")?;
        log_addon_debug(format!(
            "auth attempt {}/{} POST {} headers=[Authorization(Basic),UserName,Key]",
            attempt, max_attempts, url
        ));
        let response = client
            .post(&url)
            .basic_auth(login, Some(license_key))
            .header("UserName", login)
            .header("Key", license_key)
            .json(&payload)
            .send()
            .await
            .context("Failed to authenticate with x-updater")?;

        let status = response.status();
        last_status = Some(status);

        let headers = response.headers().clone();
        let raw_body = response
            .bytes()
            .await
            .context("Failed to read x-updater auth response")?;
        let body_preview = preview_text(&String::from_utf8_lossy(&raw_body), 900);
        log_addon_debug(format!(
            "auth response attempt={} status={} body='{}'",
            attempt, status, body_preview
        ));

        if status.is_success() && status != reqwest::StatusCode::ACCEPTED {
            let mode = parse_xup_auth_mode(&headers, &raw_body);
            let auth_header_candidates =
                collect_xup_auth_candidates(host, &headers, &raw_body, &mode);
            log_addon_info(format!(
                "auth completed attempt={} status={} mode={} candidateCount={}",
                attempt,
                status,
                auth_mode_label(&mode),
                auth_header_candidates.len()
            ));
            return Ok(XAuth {
                mode,
                login: login.to_string(),
                license_key: license_key.to_string(),
                auth_header_candidates,
            });
        }

        if status == reqwest::StatusCode::ACCEPTED
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            if attempt < max_attempts {
                let retry_after_secs = headers
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.trim().parse::<u64>().ok())
                    .map(|v| v.clamp(1, 10))
                    .unwrap_or(1);
                log_addon_debug(format!(
                    "auth pending/rate limited status={} retryAfter={}s",
                    status, retry_after_secs
                ));
                tokio::time::sleep(std::time::Duration::from_secs(retry_after_secs)).await;
                continue;
            }
        } else if status.is_client_error() || status.is_server_error() {
            let body_text = String::from_utf8_lossy(&raw_body);
            log_addon_info(format!(
                "auth failed status={} body='{}'",
                status,
                preview_text(&body_text, 1200)
            ));
            return Err(anyhow!(
                "x-updater authentication failed: HTTP {} ({})",
                status,
                body_text.trim()
            ));
        }

        // Fallback: successful but unusual code/path; continue trying.
    }

    Err(anyhow!(
        "x-updater authentication did not complete after {} attempts (last status: {})",
        max_attempts,
        last_status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ))
}

fn parse_xup_auth_mode(headers: &HeaderMap, raw_body: &[u8]) -> XAuthMode {
    if let Some(token) = extract_bearer_from_headers(headers) {
        return XAuthMode::Bearer(token);
    }

    if raw_body.is_empty() {
        return XAuthMode::Basic;
    }

    if let Ok(body) = serde_json::from_slice::<Value>(raw_body) {
        return parse_xup_auth_mode_from_json(&body);
    }

    if let Ok(text) = std::str::from_utf8(raw_body) {
        if let Some(token) = extract_bearer_from_text(text) {
            return XAuthMode::Bearer(token);
        }
    }

    // Some updater servers return plain text/empty auth acknowledgements.
    // Treat successful HTTP auth as valid and continue in Basic mode.
    XAuthMode::Basic
}

fn parse_xup_auth_mode_from_json(body: &Value) -> XAuthMode {
    let kind = body
        .get("auth")
        .and_then(|v| v.get("type").and_then(|x| x.as_str()))
        .or_else(|| body.get("type").and_then(|x| x.as_str()))
        .map(|v| v.trim().to_lowercase());

    if let Some(kind) = kind {
        match kind.as_str() {
            "none" => return XAuthMode::None,
            "bearer" => {
                if let Some(token) = body
                    .get("auth")
                    .and_then(|v| v.get("token").and_then(|x| x.as_str()))
                    .or_else(|| body.get("token").and_then(|x| x.as_str()))
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                {
                    return XAuthMode::Bearer(token.to_string());
                }
            }
            _ => return XAuthMode::Basic,
        }
    }

    if let Some(token) = body
        .get("auth")
        .and_then(|v| v.get("token").and_then(|x| x.as_str()))
        .or_else(|| body.get("token").and_then(|x| x.as_str()))
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        return XAuthMode::Bearer(token.to_string());
    }

    XAuthMode::Basic
}

fn collect_xup_auth_candidates(
    host: &str,
    headers: &HeaderMap,
    raw_body: &[u8],
    mode: &XAuthMode,
) -> Vec<String> {
    let mut raw_candidates: Vec<String> = Vec::new();

    for key in [
        "authorization",
        "location",
        "x-auth-token",
        "x-bearer-token",
        "bearer-token",
        "token",
    ] {
        if let Some(value) = headers.get(key).and_then(|v| v.to_str().ok()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                raw_candidates.push(trimmed.to_string());
            }
        }
    }

    if let Ok(body) = serde_json::from_slice::<Value>(raw_body) {
        append_json_auth_candidates(&mut raw_candidates, &body);
    }

    if let Ok(body_text) = std::str::from_utf8(raw_body) {
        let candidate = body_text.trim();
        if !candidate.is_empty()
            && candidate.len() <= 1024
            && !candidate.starts_with('{')
            && !candidate.starts_with('[')
            && !candidate.starts_with('<')
        {
            raw_candidates.push(candidate.to_string());
        }
    }

    if let XAuthMode::Bearer(token) = mode {
        let token = token.trim();
        if !token.is_empty() {
            raw_candidates.push(token.to_string());
            raw_candidates.push(format!("Bearer {}", token));
            raw_candidates.push(format!("Token {}", token));
        }
    }

    let mut expanded: Vec<String> = Vec::new();
    for raw in raw_candidates {
        expanded.extend(expand_xup_auth_candidate(host, &raw));
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for candidate in expanded {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_string();
        if seen.insert(key.clone()) {
            out.push(key);
        }
    }
    log_addon_debug(format!(
        "auth candidates collected count={} values={:?}",
        out.len(),
        out.iter().map(|v| mask_secret(v)).collect::<Vec<String>>()
    ));
    out
}

fn append_json_auth_candidates(out: &mut Vec<String>, body: &Value) {
    let mut push_string = |value: Option<&Value>| {
        if let Some(text) = value.and_then(|v| v.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    };

    for key in ["token", "access_token", "authorization", "location"] {
        push_string(body.get(key));
    }

    if let Some(auth) = body.get("auth") {
        for key in ["token", "access_token", "authorization", "location"] {
            push_string(auth.get(key));
        }
    }
}

fn expand_xup_auth_candidate(host: &str, input: &str) -> Vec<String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Vec::new();
    }

    let mut out = vec![raw.to_string()];

    if raw.starts_with("http://") || raw.starts_with("https://") {
        if let Ok(url) = Url::parse(raw) {
            let path = url.path().trim();
            if !path.is_empty() && path != "/" {
                out.push(path.to_string());
            }

            if let Some(last_segment) = url.path_segments().and_then(|segments| {
                segments
                    .filter(|segment| !segment.trim().is_empty())
                    .next_back()
            }) {
                let token = last_segment.trim();
                if !token.is_empty() {
                    out.push(token.to_string());
                    out.push(format!("Bearer {}", token));
                    out.push(format!("Token {}", token));
                }
            }
        }
    } else if raw.starts_with('/') {
        if let Ok(absolute_url) = resolve_link(host, raw) {
            out.push(absolute_url);
        }
        if let Some(last_segment) = raw
            .split('/')
            .filter(|segment| !segment.trim().is_empty())
            .next_back()
        {
            let token = last_segment.trim();
            if !token.is_empty() {
                out.push(token.to_string());
                out.push(format!("Bearer {}", token));
                out.push(format!("Token {}", token));
            }
        }
    } else {
        let lower = raw.to_ascii_lowercase();
        let has_prefix = lower.starts_with("bearer ")
            || lower.starts_with("basic ")
            || lower.starts_with("token ");
        if !has_prefix {
            out.push(format!("Bearer {}", raw));
            out.push(format!("Token {}", raw));
        }
    }

    out
}

fn extract_bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    const HEADER_KEYS: [&str; 5] = [
        "authorization",
        "x-auth-token",
        "x-bearer-token",
        "bearer-token",
        "token",
    ];

    for key in HEADER_KEYS {
        let Some(value) = headers.get(key) else {
            continue;
        };
        let Ok(text_raw) = value.to_str() else {
            continue;
        };
        let text = text_raw.trim();
        if text.is_empty() {
            continue;
        }
        if let Some(token) = parse_bearer_token(text) {
            return Some(token);
        }
        if key != "authorization" {
            return Some(text.to_string());
        }
    }
    None
}

fn extract_bearer_from_text(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(token) = parse_bearer_token(line.trim()) {
            return Some(token);
        }
    }

    parse_bearer_token(text.trim())
}

fn parse_bearer_token(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }

    if let Some(rest) = raw.strip_prefix("Bearer ") {
        let token = rest.trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }

    if let Some(rest) = raw.strip_prefix("bearer ") {
        let token = rest.trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }

    if let Some(rest) = raw.strip_prefix("Token ") {
        let token = rest.trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }

    if let Some(rest) = raw.strip_prefix("token ") {
        let token = rest.trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }

    if let Ok(url) = Url::parse(raw) {
        if let Some(seg) = url.path_segments().and_then(|mut s| s.next_back()) {
            let token = seg.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }

    if raw.starts_with('/') {
        if let Some(seg) = raw.split('/').filter(|s| !s.trim().is_empty()).next_back() {
            let token = seg.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }

    None
}

async fn xup_fetch_products(
    client: &reqwest::Client,
    host: &str,
    auth: &XAuth,
    task_control: Option<&TaskControl>,
) -> Result<Vec<Value>> {
    ensure_not_cancelled(task_control, "scan")?;
    let base_url = resolve_link(host, XUPDATER_UPDATES_PATH)?;
    let candidates = [
        base_url.clone(),
        format!(
            "{}?all=true&empty=true&with-meta=true&check=all&secure=true",
            base_url
        ),
    ];

    let mut errors = Vec::new();
    for url in candidates {
        ensure_not_cancelled(task_control, "scan")?;
        log_addon_info(format!("fetch products start url={}", url));
        match xup_request_json(client, &url, auth, task_control).await {
            Ok(body) => {
                log_addon_debug(format!(
                    "fetch products raw payload type={} url={} preview='{}'",
                    if body.is_array() {
                        "array"
                    } else if body.is_object() {
                        "object"
                    } else {
                        "other"
                    },
                    url,
                    preview_text(&body.to_string(), 800)
                ));
                if let Some(arr) = body.as_array() {
                    if !arr.is_empty() {
                        log_addon_info(format!(
                            "fetch products success url={} count={}",
                            url,
                            arr.len()
                        ));
                        return Ok(arr.clone());
                    }
                }
                if let Some(arr) = body.get("products").and_then(|v| v.as_array()) {
                    if !arr.is_empty() {
                        log_addon_info(format!(
                            "fetch products success url={} count={}",
                            url,
                            arr.len()
                        ));
                        return Ok(arr.clone());
                    }
                }
                if let Some(arr) = body.get("mProductsInfo").and_then(|v| v.as_array()) {
                    if !arr.is_empty() {
                        log_addon_info(format!(
                            "fetch products success url={} count={}",
                            url,
                            arr.len()
                        ));
                        return Ok(arr.clone());
                    }
                }
                errors.push(format!("{} -> empty products payload", url));
            }
            Err(e) => errors.push(format!("{} -> {}", url, e)),
        }
    }

    if errors.is_empty() {
        return Err(anyhow!("x-updater returned no products"));
    }

    Err(anyhow!(
        "x-updater returned no products. Attempts: {}",
        errors.join(" | ")
    ))
}

async fn xup_fetch_file_list(
    client: &reqwest::Client,
    url: &str,
    auth: &XAuth,
    task_control: Option<&TaskControl>,
) -> Result<Vec<Value>> {
    ensure_not_cancelled(task_control, "scan")?;
    log_addon_info(format!("fetch file list start url={}", url));
    let body = xup_request_json(client, url, auth, task_control).await?;
    if let Some(arr) = body.as_array() {
        log_addon_info(format!(
            "fetch file list success shape=array count={}",
            arr.len()
        ));
        return Ok(arr.clone());
    }
    if let Some(arr) = body.get("body").and_then(|v| v.as_array()) {
        log_addon_info(format!(
            "fetch file list success shape=body count={}",
            arr.len()
        ));
        return Ok(arr.clone());
    }
    if let Some(arr) = body.get("files").and_then(|v| v.as_array()) {
        log_addon_info(format!(
            "fetch file list success shape=files count={}",
            arr.len()
        ));
        return Ok(arr.clone());
    }
    if let Some(arr) = body.get("items").and_then(|v| v.as_array()) {
        log_addon_info(format!(
            "fetch file list success shape=items count={}",
            arr.len()
        ));
        return Ok(arr.clone());
    }
    if let Some(arr) = body.get("mFiles").and_then(|v| v.as_array()) {
        log_addon_info(format!(
            "fetch file list success shape=mFiles count={}",
            arr.len()
        ));
        return Ok(arr.clone());
    }
    log_addon_debug(format!(
        "fetch file list unexpected payload preview='{}'",
        preview_text(&body.to_string(), 800)
    ));
    Ok(Vec::new())
}

fn build_xup_request_strategies(auth: &XAuth) -> Vec<XRequestStrategy> {
    let mut strategies = Vec::new();

    let mut push = |auth_mode: XRequestAuth, include_user_key: bool, label: &str| {
        strategies.push(XRequestStrategy {
            auth: auth_mode,
            include_user_key,
            label: label.to_string(),
        });
    };

    for candidate in &auth.auth_header_candidates {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            continue;
        }
        push(
            XRequestAuth::Authorization(trimmed.to_string()),
            false,
            "authorization-only",
        );
        push(
            XRequestAuth::Authorization(trimmed.to_string()),
            true,
            "authorization+userkey",
        );
    }

    match &auth.mode {
        XAuthMode::Bearer(token) => {
            let token_trimmed = token.trim().to_string();
            if !token_trimmed.is_empty() {
                push(
                    XRequestAuth::Bearer(token_trimmed.clone()),
                    true,
                    "bearer+userkey",
                );
                push(XRequestAuth::Bearer(token_trimmed), false, "bearer-only");
            }
            push(XRequestAuth::Basic, true, "basic+userkey");
            push(XRequestAuth::Basic, false, "basic-only");
            push(XRequestAuth::None, true, "userkey-only");
        }
        XAuthMode::Basic => {
            push(XRequestAuth::Basic, true, "basic+userkey");
            push(XRequestAuth::Basic, false, "basic-only");
            push(XRequestAuth::None, true, "userkey-only");
        }
        XAuthMode::None => {
            push(XRequestAuth::None, true, "userkey-only");
            push(XRequestAuth::Basic, true, "basic+userkey");
            push(XRequestAuth::Basic, false, "basic-only");
        }
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for strategy in strategies {
        let key = match &strategy.auth {
            XRequestAuth::Authorization(value) => {
                format!("authorization:{}|{}", value, strategy.include_user_key)
            }
            XRequestAuth::Basic => format!("basic|{}", strategy.include_user_key),
            XRequestAuth::Bearer(token) => {
                format!("bearer:{}|{}", token, strategy.include_user_key)
            }
            XRequestAuth::None => format!("none|{}", strategy.include_user_key),
        };
        if seen.insert(key) {
            out.push(strategy);
        }
    }

    out
}

fn apply_xup_request_strategy(
    request: reqwest::RequestBuilder,
    auth: &XAuth,
    strategy: &XRequestStrategy,
) -> reqwest::RequestBuilder {
    let mut request = request;

    if strategy.include_user_key {
        request = request
            .header("UserName", auth.login.as_str())
            .header("Key", auth.license_key.as_str());
    }

    match &strategy.auth {
        XRequestAuth::Authorization(value) => request.header("Authorization", value),
        XRequestAuth::Basic => request.basic_auth(&auth.login, Some(&auth.license_key)),
        XRequestAuth::Bearer(token) => request.bearer_auth(token),
        XRequestAuth::None => request,
    }
}

async fn xup_request_json(
    client: &reqwest::Client,
    url: &str,
    auth: &XAuth,
    task_control: Option<&TaskControl>,
) -> Result<Value> {
    ensure_not_cancelled(task_control, "scan")?;
    let strategies = build_xup_request_strategies(auth);
    let mut attempts = Vec::new();
    let mut last_error: Option<anyhow::Error> = None;
    log_addon_debug(format!(
        "request_json start url={} strategyCount={}",
        url,
        strategies.len()
    ));

    for strategy in strategies {
        ensure_not_cancelled(task_control, "scan")?;
        let request = client.get(url).header("Accept", "application/json");
        let request = apply_xup_request_strategy(request, auth, &strategy);
        log_addon_debug(format!(
            "request_json send url={} strategy={}",
            url,
            strategy_log_label(&strategy)
        ));
        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                attempts.push(format!("{}:ERR", strategy.label));
                let err_text = err.to_string();
                last_error = Some(anyhow!(err));
                log_addon_debug(format!(
                    "request_json transport error url={} strategy={} error={}",
                    url,
                    strategy_log_label(&strategy),
                    err_text
                ));
                continue;
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            let snippet = compact_error_snippet(&body_text);
            attempts.push(format!("{}:{}:{}", strategy.label, status, snippet));
            log_addon_debug(format!(
                "request_json non-success url={} strategy={} status={} body='{}'",
                url,
                strategy_log_label(&strategy),
                status,
                preview_text(&body_text, 1200)
            ));
            last_error = Some(anyhow!(
                "x-updater request failed: {} -> HTTP {}{}",
                url,
                status,
                if snippet.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", snippet)
                }
            ));
            continue;
        }

        let raw = match response.bytes().await {
            Ok(raw) => raw,
            Err(err) => {
                attempts.push(format!("{}:READERR", strategy.label));
                let err_text = err.to_string();
                last_error = Some(anyhow!(err));
                log_addon_debug(format!(
                    "request_json read error url={} strategy={} error={}",
                    url,
                    strategy_log_label(&strategy),
                    err_text
                ));
                continue;
            }
        };
        log_addon_debug(format!(
            "request_json success status=200 url={} strategy={} bytes={} body='{}'",
            url,
            strategy_log_label(&strategy),
            raw.len(),
            preview_text(&String::from_utf8_lossy(&raw), 1200)
        ));

        match serde_json::from_slice::<Value>(&raw) {
            Ok(value) => {
                log_addon_debug(format!(
                    "request_json parsed ok url={} strategy={} jsonType={}",
                    url,
                    strategy_log_label(&strategy),
                    if value.is_array() {
                        "array"
                    } else if value.is_object() {
                        "object"
                    } else {
                        "other"
                    }
                ));
                return Ok(value);
            }
            Err(err) => {
                attempts.push(format!("{}:BADJSON", strategy.label));
                log_addon_debug(format!(
                    "request_json parse error url={} strategy={} error={}",
                    url,
                    strategy_log_label(&strategy),
                    err
                ));
                last_error = Some(anyhow!(
                    "Failed to parse x-updater response JSON: {} ({})",
                    url,
                    err
                ));
            }
        }
    }

    if attempts.is_empty() {
        return Err(last_error
            .unwrap_or_else(|| anyhow!("x-updater request failed without attempts: {}", url)));
    }

    Err(anyhow!(
        "{}. Attempts: {}",
        last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| format!("x-updater request failed: {}", url)),
        attempts.join(", ")
    ))
}

fn compact_error_snippet(input: &str) -> String {
    let mut out = String::new();
    let mut last_was_space = false;
    for ch in input.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
            continue;
        }
        last_was_space = false;
        out.push(ch);
        if out.len() >= 180 {
            break;
        }
    }
    out.trim().to_string()
}

async fn download_xupdater_file(
    client: &reqwest::Client,
    auth: &XAuth,
    task: &XDownloadTask,
    task_control: Option<&TaskControl>,
    chunk_callback: Option<Arc<dyn Fn(u64) + Send + Sync>>,
) -> Result<Vec<u8>> {
    ensure_not_cancelled(task_control, "install")?;
    let strategies = build_xup_request_strategies(auth);
    let mut attempts = Vec::new();
    let mut last_error: Option<anyhow::Error> = None;
    log_addon_info(format!(
        "download start url={} relPath={} strategyCount={}",
        task.url,
        task.rel_path,
        strategies.len()
    ));

    for strategy in strategies {
        ensure_not_cancelled(task_control, "install")?;
        let request = client
            .get(&task.url)
            .header("Accept", "application/octet-stream");
        let request = apply_xup_request_strategy(request, auth, &strategy);
        log_addon_debug(format!(
            "download send url={} relPath={} strategy={}",
            task.url,
            task.rel_path,
            strategy_log_label(&strategy)
        ));

        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                attempts.push(format!("{}:ERR", strategy.label));
                log_addon_debug(format!(
                    "download transport error url={} strategy={} error={}",
                    task.url,
                    strategy_log_label(&strategy),
                    err
                ));
                last_error = Some(anyhow!("Failed to download '{}': {}", task.url, err));
                continue;
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            let snippet = compact_error_snippet(&body_text);
            attempts.push(format!("{}:{}:{}", strategy.label, status, snippet));
            log_addon_debug(format!(
                "download non-success url={} strategy={} status={} body='{}'",
                task.url,
                strategy_log_label(&strategy),
                status,
                preview_text(&body_text, 1200)
            ));
            last_error = Some(anyhow!(
                "Download failed for '{}': HTTP {}{}",
                task.url,
                status,
                if snippet.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", snippet)
                }
            ));
            continue;
        }

        let mut raw = Vec::new();
        let mut stream = response.bytes_stream();
        let mut stream_failed = None;
        while let Some(next) = stream.next().await {
            if let Err(err) = ensure_not_cancelled(task_control, "install") {
                return Err(err);
            }
            match next {
                Ok(chunk) => {
                    if !chunk.is_empty() {
                        raw.extend_from_slice(&chunk);
                        if let Some(cb) = chunk_callback.as_ref() {
                            cb(chunk.len() as u64);
                        }
                    }
                }
                Err(err) => {
                    stream_failed = Some(err);
                    break;
                }
            }
        }

        if let Some(err) = stream_failed {
            attempts.push(format!("{}:READERR", strategy.label));
            log_addon_debug(format!(
                "download read error url={} strategy={} error={}",
                task.url,
                strategy_log_label(&strategy),
                err
            ));
            last_error = Some(anyhow!(
                "Failed to read response for '{}': {}",
                task.url,
                err
            ));
            continue;
        }

        match validate_download_payload(task, &raw) {
            Ok(data) => {
                log_addon_info(format!(
                    "download success relPath={} strategy={} rawBytes={} finalBytes={}",
                    task.rel_path,
                    strategy_log_label(&strategy),
                    raw.len(),
                    data.len()
                ));
                return Ok(data);
            }
            Err(err) => {
                attempts.push(format!("{}:VERIFYERR", strategy.label));
                log_addon_debug(format!(
                    "download verify error relPath={} strategy={} error={}",
                    task.rel_path,
                    strategy_log_label(&strategy),
                    err
                ));
                last_error = Some(err);
            }
        }
    }

    if attempts.is_empty() {
        return Err(last_error.unwrap_or_else(|| anyhow!("Download failed for '{}'", task.url)));
    }

    Err(anyhow!(
        "{}. Attempts: {}",
        last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| format!("Download failed for '{}'", task.url)),
        attempts.join(", ")
    ))
}

fn validate_download_payload(task: &XDownloadTask, raw: &[u8]) -> Result<Vec<u8>> {
    let raw_size = raw.len() as u64;
    let mut data = raw.to_vec();

    if let Some(expected) = task.expected_md5.as_ref() {
        let raw_md5 = md5_for_bytes(raw);
        if !raw_md5.eq_ignore_ascii_case(expected) {
            if let Ok(decoded) = try_gzip_decode(raw) {
                let decoded_md5 = md5_for_bytes(&decoded);
                if decoded_md5.eq_ignore_ascii_case(expected) {
                    data = decoded;
                } else {
                    return Err(anyhow!(
                        "MD5 mismatch for '{}': expected {}, got {}",
                        task.rel_path,
                        expected,
                        raw_md5
                    ));
                }
            } else {
                return Err(anyhow!(
                    "MD5 mismatch for '{}': expected {}, got {}",
                    task.rel_path,
                    expected,
                    raw_md5
                ));
            }
        }
    }

    if let Some(expected_size) = task.expected_size {
        let actual = data.len() as u64;
        if expected_size > 0 && actual != expected_size && raw_size != expected_size {
            return Err(anyhow!(
                "Size mismatch for '{}': expected {}, got {}",
                task.rel_path,
                expected_size,
                actual
            ));
        }
    }

    Ok(data)
}

fn try_gzip_decode(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .context("Failed to decode gzip stream")?;
    Ok(out)
}

fn md5_for_file(path: &Path, task_control: Option<&TaskControl>) -> Result<String> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("Failed to open '{}' for MD5", path.display()))?;
    let mut hasher = Md5Context::new();
    let mut buffer = [0u8; 8192];
    loop {
        ensure_not_cancelled(task_control, "scan")?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.consume(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.compute()))
}

fn md5_for_bytes(data: &[u8]) -> String {
    format!("{:x}", md5::compute(data))
}

fn resolve_target_path(xplane_path: &Path, item_type: &str, folder_name: &str) -> Result<PathBuf> {
    if folder_name.trim().is_empty() {
        return Err(anyhow!("Folder name cannot be empty"));
    }
    if folder_name.contains("..") {
        return Err(anyhow!("Folder name contains invalid traversal segment"));
    }

    let normalized_folder = folder_name.replace('\\', "/");

    let base_path = match item_type {
        "aircraft" | "livery" => xplane_path.join("Aircraft"),
        "plugin" => xplane_path.join("Resources").join("plugins"),
        "scenery" => xplane_path.join("Custom Scenery"),
        other => return Err(anyhow!("Unsupported item type '{}'", other)),
    };

    let target_path = base_path.join(normalized_folder);
    if !target_path.exists() {
        return Err(anyhow!(
            "Target path does not exist: {}",
            target_path.display()
        ));
    }

    crate::path_utils::validate_child_path(&base_path, &target_path)
        .map_err(|e| anyhow!("Invalid target path: {}", e))
}

fn normalize_manifest_path(path: &str) -> Result<String> {
    let mut normalized = path.trim().replace('\\', "/");
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    normalized = normalized.trim_start_matches('/').to_string();
    if normalized.is_empty() {
        return Err(anyhow!("Path is empty"));
    }

    let mut parts: Vec<String> = Vec::new();
    for comp in normalized.split('/') {
        let seg = comp.trim();
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return Err(anyhow!("Path traversal is not allowed: {}", path));
        }
        if seg.contains(':') {
            return Err(anyhow!("Absolute paths are not allowed: {}", path));
        }
        parts.push(seg.to_string());
    }

    if parts.is_empty() {
        return Err(anyhow!("Path is empty"));
    }

    Ok(parts.join("/"))
}

fn resolve_entry_path(target_root: &Path, rel_path: &str) -> Result<PathBuf> {
    let normalized = normalize_manifest_path(rel_path)?;
    Ok(target_root.join(normalized.replace('/', std::path::MAIN_SEPARATOR_STR)))
}

fn write_file_atomic(destination: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let file_name = destination
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("Invalid destination file name: {}", destination.display()))?;
    let temp_name = format!(".xfm_tmp_{}_{}", std::process::id(), file_name);
    let temp_path = destination.with_file_name(temp_name);

    fs::write(&temp_path, data)
        .with_context(|| format!("Failed to write temporary file '{}'", temp_path.display()))?;
    fs::rename(&temp_path, destination).with_context(|| {
        format!(
            "Failed to replace '{}' with '{}'",
            destination.display(),
            temp_path.display()
        )
    })?;
    Ok(())
}

fn remove_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove directory '{}'", path.display()))?;
    } else {
        fs::remove_file(path).with_context(|| format!("Failed to remove '{}'", path.display()))?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Err(anyhow!(
            "Source directory does not exist: {}",
            src.display()
        ));
    }

    fs::create_dir_all(dst)
        .with_context(|| format!("Failed to create directory '{}'", dst.display()))?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "Failed to copy '{}' -> '{}'",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn build_http_client(timeout_secs: u64) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .user_agent("XFast Manager")
        .build()
        .context("Failed to build HTTP client")
}

fn resolve_link(base_host: &str, href: &str) -> Result<String> {
    let href = href.trim();
    if href.is_empty() {
        return Err(anyhow!("Invalid empty API link"));
    }

    // Support protocol-relative links like //update.x-plane.org/api/...
    if href.starts_with("//") {
        return Ok(format!("https:{}", href));
    }

    // Absolute URL with scheme.
    if href.starts_with("http://") || href.starts_with("https://") {
        return Ok(href.to_string());
    }

    // Host/path URL without scheme (x-updater often returns this shape).
    // Example: update.x-plane.org/api/v2/experimental/updates/...
    if let Ok(host_path_re) = regex::Regex::new(r"^[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/.+$") {
        if host_path_re.is_match(href) {
            return Ok(format!("https://{}", href));
        }
    }

    if let Some(host) = parse_tagged_update_url(base_host) {
        let base = Url::parse(&host).with_context(|| format!("Invalid host '{}'", host))?;
        return Ok(base.join(href)?.to_string());
    }

    let base = Url::parse(base_host).with_context(|| format!("Invalid host '{}'", base_host))?;
    Ok(base.join(href)?.to_string())
}

fn ensure_since_parameter(url: &str, since: i64) -> Result<String> {
    if url.contains("{since}") {
        return Ok(url.replace("{since}", &since.to_string()));
    }

    let mut parsed = Url::parse(url).with_context(|| format!("Invalid URL '{}'", url))?;
    let has_since = parsed.query_pairs().any(|(k, _)| k == "since");
    if !has_since {
        parsed
            .query_pairs_mut()
            .append_pair("since", &since.to_string());
    }
    Ok(parsed.to_string())
}

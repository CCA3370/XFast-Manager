use anyhow::{anyhow, Context, Result};
use crc32fast::Hasher;
use dashmap::DashMap;
use futures::stream::{self, StreamExt};
use glob::Pattern;
use rayon::prelude::*;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Semaphore;
use walkdir::WalkDir;

use crate::task_control::TaskControl;

#[path = "skunk_updater/channels.rs"]
mod channels;

pub(crate) use channels::local_config_path;
use channels::{is_protected_path, public_channels, read_local_config, same_depot, LocalConfig};

const LOCAL_CFG_FILE: &str = "skunkcrafts_updater.cfg";
const LOCAL_IGNORE_FILE: &str = "skunkcrafts_updater_ignore.txt";
const REMOTE_CFG_FILE: &str = "skunkcrafts_updater.cfg";
const REMOTE_MANIFEST_FILE: &str = "skunkcrafts_updater_manifest.txt";
const REMOTE_WHITELIST_FILE: &str = "skunkcrafts_updater_whitelist.txt";
const REMOTE_IGNORELIST_FILE: &str = "skunkcrafts_updater_ignorelist.txt";
const REMOTE_ONCELIST_FILE: &str = "skunkcrafts_updater_oncelist.txt";
const REMOTE_SIZESLIST_FILE: &str = "skunkcrafts_updater_sizeslist.txt";
const REMOTE_BLACKLIST_FILE: &str = "skunkcrafts_updater_blacklist.txt";
const CHANNEL_CHANGED: &str =
    "SkunkCrafts update channel changed. Review the new plan before updating.";
const LOCAL_CRC_CACHE_TTL: Duration = Duration::from_secs(300);
const LOCAL_CRC_CACHE_MAX_SIZE: usize = 20_000;
const CHUNKED_DOWNLOAD_MIN_SIZE: u64 = 512 * 1024;
const CONTENT_DOWNLOAD_MAX_ATTEMPTS: usize = 5;
const CONTENT_DOWNLOAD_RETRY_BASE_DELAY_MS: u64 = 350;
const CONTENT_DOWNLOAD_RETRY_MAX_DELAY_MS: u64 = 2_000;
const CONTENT_DOWNLOAD_CANCEL_POLL_MS: u64 = 100;
const CONTENT_DOWNLOAD_PREALLOCATE_MAX_BYTES: u64 = 8 * 1024 * 1024;
const CONTENT_DOWNLOAD_LOG_CONTEXT: &str = "addon_updater";

#[derive(Debug, Clone)]
struct LocalCrcCacheEntry {
    crc32: u32,
    len: u64,
    modified: Option<SystemTime>,
    cached_at: SystemTime,
}

static LOCAL_CRC_CACHE: LazyLock<DashMap<String, LocalCrcCacheEntry>> = LazyLock::new(DashMap::new);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkunkUpdateOptions {
    #[serde(default)]
    pub use_beta: Option<bool>,
    #[serde(default)]
    pub expected_skunk_module: Option<String>,
    pub include_liveries: bool,
    pub apply_blacklist: bool,
    pub rollback_on_failure: bool,
    #[serde(default)]
    pub parallel_downloads: Option<usize>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub fresh_install: bool,
    #[serde(default)]
    pub preserve_liveries: bool,
    #[serde(default)]
    pub preserve_config_files: bool,
    #[serde(default)]
    pub chunked_download_enabled: Option<bool>,
    #[serde(default)]
    pub threads_per_task: Option<usize>,
    #[serde(default)]
    pub total_threads: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkunkUpdatePlan {
    #[serde(default)]
    pub provider: String,
    pub item_type: String,
    pub folder_name: String,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
    pub remote_module: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_download_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_download_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zibo_install_mode: Option<String>,
    pub remote_locked: bool,
    pub has_update: bool,
    pub estimated_download_bytes: u64,
    pub add_files: Vec<String>,
    pub replace_files: Vec<String>,
    pub delete_files: Vec<String>,
    pub skip_files: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub has_beta_config: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_channel: Option<String>,
    #[serde(default)]
    pub legacy_beta_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkunkUpdateResult {
    #[serde(default)]
    pub provider: String,
    pub success: bool,
    pub message: String,
    pub item_type: String,
    pub folder_name: String,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
    pub updated_files: usize,
    pub deleted_files: usize,
    pub skipped_files: usize,
    pub rollback_used: bool,
}

#[derive(Debug, Clone)]
pub struct SkunkUpdateProgressEvent {
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

pub type SkunkUpdateProgressCallback = Arc<dyn Fn(SkunkUpdateProgressEvent) + Send + Sync>;

fn emit_progress_event(
    callback: &Option<SkunkUpdateProgressCallback>,
    event: SkunkUpdateProgressEvent,
) {
    let Some(cb) = callback.as_ref() else {
        return;
    };
    cb(event);
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

fn is_cancelled_error(err: &anyhow::Error) -> bool {
    err.to_string().to_lowercase().contains("cancelled")
}

#[derive(Debug, Clone)]
struct RemoteConfig {
    version: String,
    locked: bool,
    zone: Option<String>,
    module: Option<String>,
    name: Option<String>,
    main_name: Option<String>,
    public_channels: BTreeMap<String, String>,
    protocol: u32,
}

#[derive(Debug, Clone)]
struct WhitelistEntry {
    path: String,
    crc32: i64,
}

#[derive(Debug, Clone, Default)]
struct RemoteManifest {
    whitelist: Vec<WhitelistEntry>,
    ignorelist: HashSet<String>,
    oncelist: HashSet<String>,
    sizes: HashMap<String, u64>,
    blacklist: HashSet<String>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct RemoteDepot {
    url: String,
    channel: Option<String>,
    config: RemoteConfig,
}

#[derive(Debug)]
struct PreparedUpdate {
    local: LocalConfig,
    remote: RemoteConfig,
    manifest: RemoteManifest,
    module_url: String,
    active_channel: Option<String>,
    target_path: PathBuf,
}

#[derive(Debug, Clone)]
struct LocalRelativeEntry {
    rel_path: String,
    is_dir: bool,
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
    mut options: SkunkUpdateOptions,
) -> Result<SkunkUpdatePlan> {
    options.parallel_downloads = None;
    let target_path = resolve_target_path(xplane_path, item_type, folder_name)?;
    let prepared = prepare_update_context(&target_path, options.use_beta, None).await?;
    build_plan_internal(&prepared, item_type, folder_name, &options)
}

pub async fn execute_update(
    xplane_path: &Path,
    item_type: &str,
    folder_name: &str,
    options: SkunkUpdateOptions,
    task_control: Option<TaskControl>,
    progress_callback: Option<SkunkUpdateProgressCallback>,
) -> Result<SkunkUpdateResult> {
    let target_path = resolve_target_path(xplane_path, item_type, folder_name)?;
    let prepared = prepare_update_context(
        &target_path,
        options.use_beta,
        options.expected_skunk_module.as_deref(),
    )
    .await?;
    let plan = build_plan_internal(&prepared, item_type, folder_name, &options)?;
    let install_started = Instant::now();

    emit_progress_event(
        &progress_callback,
        SkunkUpdateProgressEvent {
            stage: "install".to_string(),
            status: "started".to_string(),
            percentage: 0.0,
            processed_units: 0,
            total_units: 0,
            processed_bytes: 0,
            total_bytes: plan.estimated_download_bytes,
            speed_bytes_per_sec: 0.0,
            current_file: None,
            message: Some("Preparing installation".to_string()),
        },
    );
    ensure_not_cancelled(task_control.as_ref(), "install")?;

    if plan.remote_locked {
        return Err(anyhow!(
            "Remote repository is locked and cannot be updated currently"
        ));
    }

    if plan.add_files.is_empty() && plan.replace_files.is_empty() && plan.delete_files.is_empty() {
        ensure_local_config_current(&prepared, options.use_beta)?;
        if plan.remote_version.is_some() {
            update_local_cfg_fields(&prepared.local.cfg_path, &prepared.remote)?;
        }
        emit_progress_event(
            &progress_callback,
            SkunkUpdateProgressEvent {
                stage: "install".to_string(),
                status: "completed".to_string(),
                percentage: 100.0,
                processed_units: 0,
                total_units: 0,
                processed_bytes: 0,
                total_bytes: 0,
                speed_bytes_per_sec: 0.0,
                current_file: None,
                message: Some("No file changes required".to_string()),
            },
        );
        return Ok(SkunkUpdateResult {
            provider: "manifest".to_string(),
            success: true,
            message: "No file changes required".to_string(),
            item_type: item_type.to_string(),
            folder_name: folder_name.to_string(),
            local_version: prepared.local.version.clone(),
            remote_version: plan.remote_version.clone(),
            updated_files: 0,
            deleted_files: 0,
            skipped_files: plan.skip_files.len(),
            rollback_used: false,
        });
    }

    let mut whitelist_crc: HashMap<String, i64> = HashMap::new();
    for item in &prepared.manifest.whitelist {
        whitelist_crc.insert(item.path.clone(), item.crc32);
    }

    let mut download_targets: Vec<String> = Vec::new();
    download_targets.extend(plan.add_files.clone());
    download_targets.extend(plan.replace_files.clone());

    let total_download_units = download_targets.len() as u64;
    let total_download_bytes = plan.estimated_download_bytes;
    let download_phase_weight = if total_download_units > 0 { 90.0 } else { 0.0 };
    let processed_download_bytes = Arc::new(AtomicU64::new(0));
    let processed_download_units = Arc::new(AtomicU64::new(0));
    let last_emit = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));

    let chunk_progress_callback: Option<Arc<dyn Fn(String, u64) + Send + Sync>> =
        progress_callback.as_ref().map(|_| {
            let progress_callback = progress_callback.clone();
            let processed_download_bytes = Arc::clone(&processed_download_bytes);
            let processed_download_units = Arc::clone(&processed_download_units);
            let last_emit = Arc::clone(&last_emit);
            Arc::new(move |rel_path: String, chunk_size: u64| {
                let processed =
                    processed_download_bytes.fetch_add(chunk_size, Ordering::Relaxed) + chunk_size;
                let processed_units = processed_download_units.load(Ordering::Relaxed);
                let mut should_emit = true;
                if let Ok(mut guard) = last_emit.lock() {
                    if guard.elapsed() < Duration::from_millis(120) {
                        should_emit = false;
                    } else {
                        *guard = Instant::now();
                    }
                }
                if !should_emit {
                    return;
                }

                let ratio = if total_download_bytes > 0 {
                    processed as f64 / total_download_bytes as f64
                } else if total_download_units > 0 {
                    processed_units as f64 / total_download_units as f64
                } else {
                    1.0
                };
                let percentage = (ratio * download_phase_weight).clamp(0.0, download_phase_weight);
                let elapsed = install_started.elapsed().as_secs_f64().max(0.001);
                let speed = processed as f64 / elapsed;
                emit_progress_event(
                    &progress_callback,
                    SkunkUpdateProgressEvent {
                        stage: "install".to_string(),
                        status: "in_progress".to_string(),
                        percentage,
                        processed_units,
                        total_units: total_download_units,
                        processed_bytes: processed,
                        total_bytes: total_download_bytes,
                        speed_bytes_per_sec: speed,
                        current_file: Some(rel_path),
                        message: Some("Downloading".to_string()),
                    },
                );
            }) as Arc<dyn Fn(String, u64) + Send + Sync>
        });

    let file_completed_callback: Option<Arc<dyn Fn(String) + Send + Sync>> =
        progress_callback.as_ref().map(|_| {
            let progress_callback = progress_callback.clone();
            let processed_download_bytes = Arc::clone(&processed_download_bytes);
            let processed_download_units = Arc::clone(&processed_download_units);
            Arc::new(move |rel_path: String| {
                let processed_units = processed_download_units.fetch_add(1, Ordering::Relaxed) + 1;
                let processed = processed_download_bytes.load(Ordering::Relaxed);
                let ratio = if total_download_bytes > 0 {
                    processed as f64 / total_download_bytes as f64
                } else if total_download_units > 0 {
                    processed_units as f64 / total_download_units as f64
                } else {
                    1.0
                };
                let percentage = (ratio * download_phase_weight).clamp(0.0, download_phase_weight);
                let elapsed = install_started.elapsed().as_secs_f64().max(0.001);
                let speed = processed as f64 / elapsed;
                emit_progress_event(
                    &progress_callback,
                    SkunkUpdateProgressEvent {
                        stage: "install".to_string(),
                        status: "in_progress".to_string(),
                        percentage,
                        processed_units,
                        total_units: total_download_units,
                        processed_bytes: processed,
                        total_bytes: total_download_bytes,
                        speed_bytes_per_sec: speed,
                        current_file: Some(rel_path),
                        message: Some("Downloading".to_string()),
                    },
                );
            }) as Arc<dyn Fn(String) + Send + Sync>
        });

    let parallel = options.parallel_downloads.unwrap_or(4).clamp(1, 8);
    let chunked_enabled = options.chunked_download_enabled.unwrap_or(true);
    let threads_per_task = options.threads_per_task.unwrap_or(6).clamp(1, 32);
    let total_threads = options.total_threads.unwrap_or(32).clamp(1, 64);
    let downloaded = match download_files(
        &prepared.module_url,
        &download_targets,
        ContentDownloadBatchConfig {
            expected_crc: &whitelist_crc,
            parallel_downloads: parallel,
            task_control: task_control.clone(),
            chunk_progress_callback,
            file_completed_callback,
            file_sizes: &prepared.manifest.sizes,
            chunked_enabled,
            threads_per_task,
            total_threads,
        },
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            let status = if is_cancelled_error(&e) {
                "cancelled"
            } else {
                "failed"
            };
            emit_progress_event(
                &progress_callback,
                SkunkUpdateProgressEvent {
                    stage: "install".to_string(),
                    status: status.to_string(),
                    percentage: 0.0,
                    processed_units: processed_download_units.load(Ordering::Relaxed),
                    total_units: total_download_units,
                    processed_bytes: processed_download_bytes.load(Ordering::Relaxed),
                    total_bytes: total_download_bytes,
                    speed_bytes_per_sec: 0.0,
                    current_file: None,
                    message: Some(e.to_string()),
                },
            );
            return Err(e);
        }
    };

    ensure_local_config_current(&prepared, options.use_beta)?;
    let mut rollback = RollbackState::new(options.rollback_on_failure)?;
    let total_apply_units =
        (plan.replace_files.len() + plan.add_files.len() + plan.delete_files.len()) as u64;
    let apply_base_percentage = download_phase_weight;
    let apply_span = 100.0 - apply_base_percentage;
    let mut processed_apply_units = 0u64;

    let apply_result: Result<()> = (|| {
        for rel_path in &plan.replace_files {
            ensure_not_cancelled(task_control.as_ref(), "install")?;
            let destination = resolve_entry_path(&prepared.target_path, rel_path)?;
            let existed = destination.exists();

            if existed {
                rollback.backup_if_needed(&destination)?;
            } else {
                rollback.record_created_path(&destination);
            }

            let bytes = downloaded
                .get(rel_path)
                .ok_or_else(|| anyhow!("Missing downloaded data for '{}'", rel_path))?;
            write_file_atomic(&destination, bytes)?;

            processed_apply_units = processed_apply_units.saturating_add(1);
            let processed_bytes = processed_download_bytes.load(Ordering::Relaxed);
            let ratio = if total_apply_units > 0 {
                processed_apply_units as f64 / total_apply_units as f64
            } else {
                1.0
            };
            let percentage = (apply_base_percentage + ratio * apply_span).clamp(0.0, 100.0);
            let elapsed = install_started.elapsed().as_secs_f64().max(0.001);
            let speed = processed_bytes as f64 / elapsed;
            emit_progress_event(
                &progress_callback,
                SkunkUpdateProgressEvent {
                    stage: "install".to_string(),
                    status: "in_progress".to_string(),
                    percentage,
                    processed_units: processed_apply_units,
                    total_units: total_apply_units,
                    processed_bytes,
                    total_bytes: total_download_bytes,
                    speed_bytes_per_sec: speed,
                    current_file: Some(rel_path.clone()),
                    message: Some("Applying changes".to_string()),
                },
            );
        }

        for rel_path in &plan.add_files {
            ensure_not_cancelled(task_control.as_ref(), "install")?;
            let destination = resolve_entry_path(&prepared.target_path, rel_path)?;
            let existed = destination.exists();

            if existed {
                rollback.backup_if_needed(&destination)?;
            } else {
                rollback.record_created_path(&destination);
            }

            let bytes = downloaded
                .get(rel_path)
                .ok_or_else(|| anyhow!("Missing downloaded data for '{}'", rel_path))?;
            write_file_atomic(&destination, bytes)?;

            processed_apply_units = processed_apply_units.saturating_add(1);
            let processed_bytes = processed_download_bytes.load(Ordering::Relaxed);
            let ratio = if total_apply_units > 0 {
                processed_apply_units as f64 / total_apply_units as f64
            } else {
                1.0
            };
            let percentage = (apply_base_percentage + ratio * apply_span).clamp(0.0, 100.0);
            let elapsed = install_started.elapsed().as_secs_f64().max(0.001);
            let speed = processed_bytes as f64 / elapsed;
            emit_progress_event(
                &progress_callback,
                SkunkUpdateProgressEvent {
                    stage: "install".to_string(),
                    status: "in_progress".to_string(),
                    percentage,
                    processed_units: processed_apply_units,
                    total_units: total_apply_units,
                    processed_bytes,
                    total_bytes: total_download_bytes,
                    speed_bytes_per_sec: speed,
                    current_file: Some(rel_path.clone()),
                    message: Some("Applying changes".to_string()),
                },
            );
        }

        for rel_path in &plan.delete_files {
            ensure_not_cancelled(task_control.as_ref(), "install")?;
            let destination = resolve_entry_path(&prepared.target_path, rel_path)?;
            if destination.exists() {
                rollback.backup_if_needed(&destination)?;
                remove_path(&destination)?;
            }

            processed_apply_units = processed_apply_units.saturating_add(1);
            let processed_bytes = processed_download_bytes.load(Ordering::Relaxed);
            let ratio = if total_apply_units > 0 {
                processed_apply_units as f64 / total_apply_units as f64
            } else {
                1.0
            };
            let percentage = (apply_base_percentage + ratio * apply_span).clamp(0.0, 100.0);
            let elapsed = install_started.elapsed().as_secs_f64().max(0.001);
            let speed = processed_bytes as f64 / elapsed;
            emit_progress_event(
                &progress_callback,
                SkunkUpdateProgressEvent {
                    stage: "install".to_string(),
                    status: "in_progress".to_string(),
                    percentage,
                    processed_units: processed_apply_units,
                    total_units: total_apply_units,
                    processed_bytes,
                    total_bytes: total_download_bytes,
                    speed_bytes_per_sec: speed,
                    current_file: Some(rel_path.clone()),
                    message: Some("Applying changes".to_string()),
                },
            );
        }

        if plan.remote_version.is_some() {
            update_local_cfg_fields(&prepared.local.cfg_path, &prepared.remote)?;
        }

        Ok(())
    })();

    if let Err(e) = apply_result {
        if options.rollback_on_failure {
            let rollback_result = rollback.rollback();
            if let Err(rollback_err) = rollback_result {
                return Err(anyhow!(
                    "Update failed: {}. Rollback also failed: {}",
                    e,
                    rollback_err
                ));
            }
        }
        let status = if is_cancelled_error(&e) {
            "cancelled"
        } else {
            "failed"
        };
        emit_progress_event(
            &progress_callback,
            SkunkUpdateProgressEvent {
                stage: "install".to_string(),
                status: status.to_string(),
                percentage: 0.0,
                processed_units: processed_apply_units,
                total_units: total_apply_units,
                processed_bytes: processed_download_bytes.load(Ordering::Relaxed),
                total_bytes: total_download_bytes,
                speed_bytes_per_sec: 0.0,
                current_file: None,
                message: Some(e.to_string()),
            },
        );
        return Err(anyhow!("Update failed: {}", e));
    }

    emit_progress_event(
        &progress_callback,
        SkunkUpdateProgressEvent {
            stage: "install".to_string(),
            status: "completed".to_string(),
            percentage: 100.0,
            processed_units: total_apply_units,
            total_units: total_apply_units,
            processed_bytes: total_download_bytes,
            total_bytes: total_download_bytes,
            speed_bytes_per_sec: 0.0,
            current_file: None,
            message: Some("Installation completed".to_string()),
        },
    );

    Ok(SkunkUpdateResult {
        provider: "manifest".to_string(),
        success: true,
        message: "Update completed successfully".to_string(),
        item_type: item_type.to_string(),
        folder_name: folder_name.to_string(),
        local_version: prepared.local.version,
        remote_version: plan.remote_version.clone(),
        updated_files: plan.add_files.len() + plan.replace_files.len(),
        deleted_files: plan.delete_files.len(),
        skipped_files: plan.skip_files.len(),
        rollback_used: false,
    })
}

fn build_plan_internal(
    prepared: &PreparedUpdate,
    item_type: &str,
    folder_name: &str,
    options: &SkunkUpdateOptions,
) -> Result<SkunkUpdatePlan> {
    let mut add_files: Vec<String> = Vec::new();
    let mut replace_files: Vec<String> = Vec::new();
    let mut delete_files: Vec<String> = Vec::new();
    let mut skip_files: Vec<String> = Vec::new();
    let mut crc_candidates: Vec<(String, PathBuf, i64)> = Vec::new();
    let mut warnings = prepared.manifest.warnings.clone();

    let mut whitelist_paths: HashSet<String> = HashSet::new();
    for entry in &prepared.manifest.whitelist {
        whitelist_paths.insert(entry.path.clone());
    }

    let include_liveries = options.include_liveries && prepared.local.liveries;
    let force_all = options.fresh_install;

    for entry in &prepared.manifest.whitelist {
        let rel_path = &entry.path;

        if !include_liveries && is_livery_path(rel_path) {
            skip_files.push(rel_path.clone());
            continue;
        }

        if is_protected_path(rel_path) || prepared.manifest.ignorelist.contains(rel_path) {
            skip_files.push(rel_path.clone());
            continue;
        }

        let local_path = resolve_entry_path(&prepared.target_path, rel_path)?;
        if entry.crc32 == -1 || prepared.manifest.oncelist.contains(rel_path) {
            if local_path.exists() {
                skip_files.push(rel_path.clone());
            } else {
                add_files.push(rel_path.clone());
            }
            continue;
        }
        if force_all && local_path.is_file() {
            replace_files.push(rel_path.clone());
            continue;
        }

        if !local_path.exists() {
            add_files.push(rel_path.clone());
            continue;
        }

        if local_path.is_dir() {
            warnings.push(format!(
                "Remote entry '{}' points to a directory locally; skipping",
                rel_path
            ));
            skip_files.push(rel_path.clone());
            continue;
        }

        crc_candidates.push((rel_path.clone(), local_path, entry.crc32));
    }

    let crc_results: Result<Vec<(String, bool)>> = crc_candidates
        .par_iter()
        .map(|(rel_path, local_path, expected_crc)| {
            let local_crc = compute_file_crc32_cached(local_path)? as i64;
            Ok::<(String, bool), anyhow::Error>((rel_path.clone(), local_crc != *expected_crc))
        })
        .collect();

    for (rel_path, needs_replace) in crc_results? {
        if needs_replace {
            replace_files.push(rel_path);
        } else {
            skip_files.push(rel_path);
        }
    }

    if options.apply_blacklist {
        delete_files = expand_blacklist_delete_files(
            &prepared.target_path,
            &prepared.manifest.blacklist,
            &whitelist_paths,
            &prepared.manifest.ignorelist,
            include_liveries,
            &mut warnings,
        )?;
    }

    let mut estimated_download_bytes: u64 = 0;
    for rel_path in add_files.iter().chain(replace_files.iter()) {
        if let Some(size) = prepared.manifest.sizes.get(rel_path) {
            estimated_download_bytes = estimated_download_bytes.saturating_add(*size);
        }
    }

    let local_version = prepared.local.version.clone();
    let remote_version = Some(prepared.remote.version.clone());
    let has_update = !prepared.remote.locked
        && (remote_version.as_deref() != local_version.as_deref()
            || !add_files.is_empty()
            || !replace_files.is_empty()
            || !delete_files.is_empty());

    if prepared.remote.locked {
        warnings.push("Remote repository is locked".to_string());
    }

    Ok(SkunkUpdatePlan {
        provider: "manifest".to_string(),
        item_type: item_type.to_string(),
        folder_name: folder_name.to_string(),
        local_version,
        remote_version,
        remote_module: Some(prepared.module_url.clone()),
        manual_download_url: None,
        manual_download_reason: None,
        zibo_install_mode: None,
        remote_locked: prepared.remote.locked,
        has_update,
        estimated_download_bytes,
        add_files,
        replace_files,
        delete_files,
        skip_files,
        warnings,
        has_beta_config: prepared.local.has_legacy_beta,
        active_channel: prepared.active_channel.clone(),
        legacy_beta_enabled: prepared.local.has_legacy_beta && prepared.local.selected.is_some(),
    })
}

fn expand_blacklist_delete_files(
    target_root: &Path,
    blacklist: &HashSet<String>,
    whitelist_paths: &HashSet<String>,
    ignored_paths: &HashSet<String>,
    include_liveries: bool,
    warnings: &mut Vec<String>,
) -> Result<Vec<String>> {
    if blacklist.is_empty() {
        return Ok(Vec::new());
    }

    let local_entries = collect_local_relative_entries(target_root)?;
    if local_entries.is_empty() {
        return Ok(Vec::new());
    }

    let mut delete_candidates: HashSet<String> = HashSet::new();
    let mut matched_blacklist_dirs: HashSet<String> = HashSet::new();

    for blacklist_entry in blacklist {
        let pattern = match Pattern::new(blacklist_entry) {
            Ok(pattern) => pattern,
            Err(e) => {
                warnings.push(format!(
                    "Skipped invalid blacklist pattern '{}': {}",
                    blacklist_entry, e
                ));
                continue;
            }
        };

        for entry in &local_entries {
            if pattern.matches(&entry.rel_path) {
                delete_candidates.insert(entry.rel_path.clone());
                if entry.is_dir {
                    matched_blacklist_dirs.insert(entry.rel_path.clone());
                }
            }
        }
    }

    if !matched_blacklist_dirs.is_empty() {
        for dir_path in &matched_blacklist_dirs {
            let prefix = format!("{}/", dir_path);
            for entry in &local_entries {
                if entry.rel_path.starts_with(&prefix) {
                    delete_candidates.insert(entry.rel_path.clone());
                }
            }
        }
    }

    let mut retained_ancestors = HashSet::new();
    let mut delete_entries = Vec::new();
    for entry in local_entries {
        let path = &entry.rel_path;
        let candidate = delete_candidates.contains(path);
        let whitelisted = whitelist_paths.contains(path);
        if candidate && whitelisted {
            warnings.push(format!(
                "Blacklist entry '{path}' is also in whitelist; skipped"
            ));
        }
        if !candidate
            || whitelisted
            || is_protected_path(path)
            || ignored_paths.contains(path)
            || (!include_liveries && is_livery_path(path))
        {
            let mut parent = path.as_str();
            while let Some((ancestor, _)) = parent.rsplit_once('/') {
                retained_ancestors.insert(ancestor.to_owned());
                parent = ancestor;
            }
        } else {
            delete_entries.push(entry);
        }
    }
    delete_entries.retain(|entry| !entry.is_dir || !retained_ancestors.contains(&entry.rel_path));

    delete_entries.sort_by(|a, b| {
        let a_depth = a.rel_path.matches('/').count();
        let b_depth = b.rel_path.matches('/').count();
        b_depth
            .cmp(&a_depth)
            .then_with(|| a.is_dir.cmp(&b.is_dir))
            .then_with(|| a.rel_path.cmp(&b.rel_path))
    });

    Ok(delete_entries
        .into_iter()
        .map(|entry| entry.rel_path)
        .collect())
}

pub(crate) fn update_client(timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .user_agent("Mozilla/5.0 (compatible; XFast Manager)")
        .build()
        .context("Failed to build HTTP client")
}

#[derive(Debug)]
pub(crate) struct CheckedVersion {
    pub(crate) installed: Option<String>,
    pub(crate) remote: String,
    pub(crate) locked: bool,
}

pub(crate) async fn check_local_version(
    client: &reqwest::Client,
    target_path: &Path,
    use_beta: Option<bool>,
) -> Result<Option<CheckedVersion>> {
    let local = read_local_config(target_path, use_beta)?;
    Ok(resolve_remote_depot(client, &local)
        .await?
        .map(|depot| CheckedVersion {
            installed: local.version,
            remote: depot.config.version,
            locked: depot.config.locked,
        }))
}

pub(crate) async fn check_depot_version(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Option<(String, bool)>> {
    Ok(fetch_remote_config(client, base_url)
        .await?
        .map(|config| (config.version, config.locked)))
}

async fn prepare_update_context(
    target_path: &Path,
    use_beta: Option<bool>,
    expected_module: Option<&str>,
) -> Result<PreparedUpdate> {
    let local = read_local_config(target_path, use_beta)?;
    let client = update_client(Duration::from_secs(20))?;
    let depot = resolve_remote_depot(&client, &local)
        .await?
        .context("SkunkCrafts Main depot configuration was not found")?;
    if expected_module.is_some_and(|expected| !same_depot(expected, &depot.url)) {
        return Err(anyhow!(CHANNEL_CHANGED));
    }
    let mut manifest = if depot.config.locked {
        RemoteManifest::default()
    } else {
        fetch_remote_manifest(&client, &depot.url, &depot.config).await?
    };
    match fs::read_to_string(target_path.join(LOCAL_IGNORE_FILE)) {
        Ok(content) => {
            manifest
                .ignorelist
                .extend(parse_path_set(&content, &mut manifest.warnings));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("Failed to read SkunkCrafts local ignore list"),
    }

    Ok(PreparedUpdate {
        local,
        remote: depot.config,
        manifest,
        module_url: depot.url,
        active_channel: depot.channel,
        target_path: target_path.to_path_buf(),
    })
}

fn ensure_local_config_current(prepared: &PreparedUpdate, use_beta: Option<bool>) -> Result<()> {
    if read_local_config(&prepared.target_path, use_beta)? != prepared.local {
        return Err(anyhow!(CHANNEL_CHANGED));
    }
    Ok(())
}

async fn resolve_remote_depot(
    client: &reqwest::Client,
    local: &LocalConfig,
) -> Result<Option<RemoteDepot>> {
    if let Some(channel) = &local.selected {
        let mut url = channel.url.clone();
        let mut config = fetch_remote_config(client, &url).await?.with_context(|| {
            format!("SkunkCrafts channel '{}' depot was not found", channel.name)
        })?;
        // A selected public depot can advertise its own move, independently of Main.
        if channel.public && !config.locked {
            if let Some(next) = config.public_channels.get(&channel.name) {
                if !same_depot(next, &url) && !same_depot(next, &local.main_url) {
                    let next_config =
                        fetch_remote_config(client, next).await?.with_context(|| {
                            format!(
                                "SkunkCrafts channel '{}' moved to a depot that was not found",
                                channel.name
                            )
                        })?;
                    url = next.clone();
                    config = next_config;
                }
            }
        }
        let label = local.main_name.is_some().then(|| channel.name.clone());
        return Ok(Some(RemoteDepot {
            url,
            channel: label,
            config,
        }));
    }

    if let Some(config) = fetch_remote_config(client, &local.main_url).await? {
        let channel = config.main_name.clone().or_else(|| {
            local
                .main_name
                .clone()
                .or_else(|| (!config.public_channels.is_empty()).then(|| "Main".to_owned()))
        });
        return Ok(Some(RemoteDepot {
            url: local.main_url.clone(),
            channel,
            config,
        }));
    }

    // The publishing guide permits fallback to the first public channel only
    // when Main returns 404. Private selections and other errors stay on their depot.
    if !local.has_legacy_beta {
        if let Some((channel, url)) = local.public_channels.first_key_value() {
            let config = fetch_remote_config(client, url).await?.with_context(|| {
                format!("SkunkCrafts public channel '{channel}' depot was not found")
            })?;
            return Ok(Some(RemoteDepot {
                url: url.clone(),
                channel: Some(channel.clone()),
                config,
            }));
        }
    }
    Ok(None)
}

async fn fetch_remote_config(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Option<RemoteConfig>> {
    let cfg_url = join_url(base_url, REMOTE_CFG_FILE)?;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string();
    let Some(content) = fetch_text_optional(client, &cfg_url, ("t", &timestamp)).await? else {
        return Ok(None);
    };

    let mut map = parse_cfg_lines(&content);
    let public_channels = public_channels(&map);
    let version = map
        .remove("version")
        .filter(|v| !v.is_empty())
        .context("Remote config is missing 'version|'")?;
    let zone = map.remove("zone").filter(|v| !v.is_empty());
    let module = map.remove("module").filter(|v| !v.is_empty());
    let name = map.remove("name").filter(|v| !v.is_empty());
    let main_name = map.remove("main_name").filter(|v| !v.is_empty());
    let protocol = map
        .get("protocol")
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.parse::<u32>()
                .context("Invalid SkunkCrafts depot protocol")
        })
        .transpose()?
        .unwrap_or(1);
    let locked = map
        .get("locked")
        .and_then(|v| parse_bool(v))
        .unwrap_or(false);

    if let Some(module_value) = module.as_deref() {
        if module_value.contains("://") {
            ensure_http_or_https(module_value)?;
        } else if zone.is_some() {
            let _ = resolve_module_url(zone.as_deref(), module_value)?;
        }
    }

    Ok(Some(RemoteConfig {
        version,
        locked,
        zone,
        module,
        name,
        main_name,
        public_channels,
        protocol,
    }))
}

async fn fetch_remote_manifest(
    client: &reqwest::Client,
    base_url: &str,
    config: &RemoteConfig,
) -> Result<RemoteManifest> {
    let version = ("v", config.version.as_str());
    let combined = config.protocol >= 2;
    let file_list = if combined {
        REMOTE_MANIFEST_FILE
    } else {
        REMOTE_WHITELIST_FILE
    };
    let url = join_url(base_url, file_list)?;
    let text = fetch_text_optional(client, &url, version)
        .await?
        .with_context(|| format!("Required SkunkCrafts file '{file_list}' was not found"))?;
    let mut warnings = Vec::new();
    let (whitelist, mut sizes) = parse_file_list(&text, combined, &mut warnings)?;
    let mut ignorelist = HashSet::new();
    let mut oncelist = HashSet::new();

    if !combined {
        let url = join_url(base_url, REMOTE_IGNORELIST_FILE)?;
        if let Some(text) = fetch_text_optional(client, &url, version).await? {
            ignorelist = parse_path_set(&text, &mut warnings);
        }
        let url = join_url(base_url, REMOTE_ONCELIST_FILE)?;
        if let Some(text) = fetch_text_optional(client, &url, version).await? {
            oncelist = parse_path_set(&text, &mut warnings);
        }
        let url = join_url(base_url, REMOTE_SIZESLIST_FILE)?;
        if let Some(text) = fetch_text_optional(client, &url, version).await? {
            sizes = parse_sizes_list(&text, &mut warnings);
        }
    }

    let url = join_url(base_url, REMOTE_BLACKLIST_FILE)?;
    let blacklist = fetch_text_optional(client, &url, version)
        .await?
        .map(|text| parse_path_set(&text, &mut warnings))
        .unwrap_or_default();

    Ok(RemoteManifest {
        whitelist,
        ignorelist,
        oncelist,
        sizes,
        blacklist,
        warnings,
    })
}

/// Probe whether the server supports HTTP Range requests.
/// Returns `Some(content_length)` if supported, `None` otherwise.
async fn probe_range_support(client: &reqwest::Client, url: &str) -> Result<Option<u64>> {
    let resp = client
        .head(url)
        .send()
        .await
        .with_context(|| format!("HEAD request failed for '{}'", url))?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let accepts_ranges = resp
        .headers()
        .get("accept-ranges")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase())
        .unwrap_or_default();

    if accepts_ranges.contains("bytes") {
        let content_length = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        Ok(content_length)
    } else {
        Ok(None)
    }
}

fn content_download_status_is_retryable(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status == reqwest::StatusCode::TOO_EARLY
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn content_download_retry_delay(failed_attempt: usize) -> Duration {
    let exponent = failed_attempt.saturating_sub(1).min(6) as u32;
    let millis = CONTENT_DOWNLOAD_RETRY_BASE_DELAY_MS.saturating_mul(1u64 << exponent);
    Duration::from_millis(millis.min(CONTENT_DOWNLOAD_RETRY_MAX_DELAY_MS))
}

async fn sleep_before_content_download_retry(
    delay: Duration,
    task_control: Option<&TaskControl>,
) -> Result<()> {
    if task_control.is_none() {
        tokio::time::sleep(delay).await;
        return Ok(());
    }

    let deadline = Instant::now() + delay;
    loop {
        ensure_not_cancelled(task_control, "install")?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        tokio::time::sleep(remaining.min(Duration::from_millis(CONTENT_DOWNLOAD_CANCEL_POLL_MS)))
            .await;
    }
}

async fn schedule_content_download_retry(
    operation: &str,
    url: &str,
    failed_attempt: usize,
    error: &anyhow::Error,
    task_control: Option<&TaskControl>,
) -> Result<bool> {
    if failed_attempt >= CONTENT_DOWNLOAD_MAX_ATTEMPTS {
        return Ok(false);
    }

    let delay = content_download_retry_delay(failed_attempt);
    crate::logger::log_info(
        &format!(
            "Retrying content download {} from '{}' after attempt {}/{} in {} ms: {}",
            operation,
            url,
            failed_attempt,
            CONTENT_DOWNLOAD_MAX_ATTEMPTS,
            delay.as_millis(),
            error
        ),
        Some(CONTENT_DOWNLOAD_LOG_CONTEXT),
    );
    sleep_before_content_download_retry(delay, task_control).await?;
    Ok(true)
}

async fn read_content_download_response(
    response: reqwest::Response,
    expected_bytes: Option<u64>,
    task_control: Option<&TaskControl>,
    chunk_progress_callback: &Option<Arc<dyn Fn(String, u64) + Send + Sync>>,
    rel_path: &str,
    operation: &str,
    url: &str,
    progress_high_water: &mut u64,
) -> Result<Vec<u8>> {
    let capacity = expected_bytes
        .and_then(|size| usize::try_from(size.min(CONTENT_DOWNLOAD_PREALLOCATE_MAX_BYTES)).ok())
        .unwrap_or(0);
    let mut data = Vec::with_capacity(capacity);
    let mut stream = response.bytes_stream();
    while let Some(next_chunk) = stream.next().await {
        ensure_not_cancelled(task_control, "install")?;
        let chunk = next_chunk
            .with_context(|| format!("Failed to stream {} for '{}'", operation.trim(), url))?;
        data.extend_from_slice(&chunk);

        let received = data.len() as u64;
        let reportable_received = expected_bytes
            .map(|expected| received.min(expected))
            .unwrap_or(received);
        if reportable_received > *progress_high_water {
            let delta = reportable_received - *progress_high_water;
            *progress_high_water = reportable_received;
            if let Some(callback) = chunk_progress_callback.as_ref() {
                callback(rel_path.to_string(), delta);
            }
        }
    }

    if let Some(expected) = expected_bytes {
        let actual = data.len() as u64;
        if actual != expected {
            return Err(anyhow!(
                "Downloaded size mismatch for {} from '{}': expected {}, got {}",
                operation,
                url,
                expected,
                actual
            ));
        }
    }

    Ok(data)
}

#[derive(Clone, Copy)]
struct ContentDownloadContext<'a> {
    client: &'a reqwest::Client,
    url: &'a str,
    expected_file_bytes: Option<u64>,
    semaphore: &'a Semaphore,
    task_control: Option<&'a TaskControl>,
    progress_callback: &'a Option<Arc<dyn Fn(String, u64) + Send + Sync>>,
    rel_path: &'a str,
}

async fn download_file_single_with_retry(context: ContentDownloadContext<'_>) -> Result<Vec<u8>> {
    let mut progress_high_water = 0u64;

    for attempt in 1..=CONTENT_DOWNLOAD_MAX_ATTEMPTS {
        ensure_not_cancelled(context.task_control, "install")?;
        let permit = context
            .semaphore
            .acquire()
            .await
            .map_err(|_| anyhow!("Semaphore closed"))?;
        let response = match context.client.get(context.url).send().await {
            Ok(response) => response,
            Err(request_error) => {
                drop(permit);
                let error = anyhow!("request failed: {}", request_error);
                if schedule_content_download_retry(
                    &format!("for '{}'", context.rel_path),
                    context.url,
                    attempt,
                    &error,
                    context.task_control,
                )
                .await?
                {
                    continue;
                }
                return Err(error.context(format!(
                    "Failed to download '{}' after {} attempts",
                    context.url, attempt
                )));
            }
        };

        let status = response.status();
        if !status.is_success() {
            drop(permit);
            let error = anyhow!("HTTP {}", status);
            if content_download_status_is_retryable(status)
                && schedule_content_download_retry(
                    &format!("for '{}'", context.rel_path),
                    context.url,
                    attempt,
                    &error,
                    context.task_control,
                )
                .await?
            {
                continue;
            }
            if content_download_status_is_retryable(status) {
                return Err(error.context(format!(
                    "Failed to download '{}' after {} attempts",
                    context.url, attempt
                )));
            }
            return Err(anyhow!(
                "Download failed for '{}': HTTP {}",
                context.url,
                status
            ));
        }

        let result = read_content_download_response(
            response,
            context.expected_file_bytes,
            context.task_control,
            context.progress_callback,
            context.rel_path,
            "response body",
            context.url,
            &mut progress_high_water,
        )
        .await;
        drop(permit);

        match result {
            Ok(data) => return Ok(data),
            Err(error) if is_cancelled_error(&error) => return Err(error),
            Err(error) => {
                if schedule_content_download_retry(
                    &format!("for '{}'", context.rel_path),
                    context.url,
                    attempt,
                    &error,
                    context.task_control,
                )
                .await?
                {
                    continue;
                }
                return Err(error.context(format!(
                    "Failed to download '{}' after {} attempts",
                    context.url, attempt
                )));
            }
        }
    }

    unreachable!("content download retry loop always returns")
}

async fn download_chunk_with_retry(
    context: ContentDownloadContext<'_>,
    index: usize,
    start: u64,
    end: u64,
) -> Result<(usize, Vec<u8>, bool)> {
    let expected_chunk_bytes = end.saturating_sub(start).saturating_add(1);
    let mut progress_high_water = 0u64;

    for attempt in 1..=CONTENT_DOWNLOAD_MAX_ATTEMPTS {
        ensure_not_cancelled(context.task_control, "install")?;
        let permit = context
            .semaphore
            .acquire()
            .await
            .map_err(|_| anyhow!("Semaphore closed"))?;
        let range_header = format!("bytes={}-{}", start, end);
        let response = match context
            .client
            .get(context.url)
            .header("Range", &range_header)
            .send()
            .await
        {
            Ok(response) => response,
            Err(request_error) => {
                drop(permit);
                let error = anyhow!("request failed: {}", request_error);
                if schedule_content_download_retry(
                    &format!("chunk {}", index),
                    context.url,
                    attempt,
                    &error,
                    context.task_control,
                )
                .await?
                {
                    continue;
                }
                return Err(error.context(format!(
                    "Failed to download chunk {} of '{}' after {} attempts",
                    index, context.url, attempt
                )));
            }
        };

        let status = response.status();
        if status == reqwest::StatusCode::OK && index > 0 {
            return Ok((index, Vec::new(), true));
        }

        if status != reqwest::StatusCode::PARTIAL_CONTENT && status != reqwest::StatusCode::OK {
            drop(permit);
            let error = anyhow!("HTTP {}", status);
            if content_download_status_is_retryable(status)
                && schedule_content_download_retry(
                    &format!("chunk {}", index),
                    context.url,
                    attempt,
                    &error,
                    context.task_control,
                )
                .await?
            {
                continue;
            }
            if content_download_status_is_retryable(status) {
                return Err(error.context(format!(
                    "Failed to download chunk {} of '{}' after {} attempts",
                    index, context.url, attempt
                )));
            }
            return Err(anyhow!(
                "Download failed for '{}' chunk {}: HTTP {}",
                context.url,
                index,
                status
            ));
        }

        let is_full_body_fallback = status == reqwest::StatusCode::OK;
        let expected_bytes = if is_full_body_fallback {
            context
                .expected_file_bytes
                .ok_or_else(|| anyhow!("Missing expected file size for chunked download"))?
        } else {
            expected_chunk_bytes
        };
        let operation = if is_full_body_fallback {
            "response body".to_string()
        } else {
            format!("chunk {}", index)
        };
        let result = read_content_download_response(
            response,
            Some(expected_bytes),
            context.task_control,
            context.progress_callback,
            context.rel_path,
            &operation,
            context.url,
            &mut progress_high_water,
        )
        .await;
        drop(permit);

        match result {
            Ok(data) => return Ok((index, data, is_full_body_fallback)),
            Err(error) if is_cancelled_error(&error) => return Err(error),
            Err(error) => {
                if schedule_content_download_retry(
                    &format!("chunk {}", index),
                    context.url,
                    attempt,
                    &error,
                    context.task_control,
                )
                .await?
                {
                    continue;
                }
                return Err(error.context(format!(
                    "Failed to download chunk {} of '{}' after {} attempts",
                    index, context.url, attempt
                )));
            }
        }
    }

    unreachable!("content chunk retry loop always returns")
}

/// Download a single file using multiple concurrent HTTP Range connections.
/// Each chunk acquires a permit from the semaphore before downloading.
async fn download_file_chunked(
    client: &reqwest::Client,
    url: &str,
    file_size: u64,
    num_chunks: usize,
    semaphore: &Arc<Semaphore>,
    task_control: &Option<TaskControl>,
    chunk_progress_callback: &Option<Arc<dyn Fn(String, u64) + Send + Sync>>,
    rel_path: &str,
) -> Result<Vec<u8>> {
    if file_size == 0 {
        return Err(anyhow!("Cannot split an empty download into chunks"));
    }
    if num_chunks == 0 || num_chunks as u64 > file_size {
        return Err(anyhow!(
            "Invalid chunk count {} for a {} byte download",
            num_chunks,
            file_size
        ));
    }

    let chunk_size = file_size / num_chunks as u64;
    let mut ranges: Vec<(u64, u64)> = Vec::with_capacity(num_chunks);
    for i in 0..num_chunks {
        let start = i as u64 * chunk_size;
        let end = if i == num_chunks - 1 {
            file_size - 1
        } else {
            (i as u64 + 1) * chunk_size - 1
        };
        ranges.push((start, end));
    }

    let mut handles = Vec::with_capacity(num_chunks);
    for (idx, (start, end)) in ranges.iter().enumerate() {
        let client = client.clone();
        let url = url.to_string();
        let semaphore = Arc::clone(semaphore);
        let task_control = task_control.clone();
        let chunk_progress_callback = chunk_progress_callback.clone();
        let rel_path = rel_path.to_string();
        let start = *start;
        let end = *end;

        let handle = tokio::spawn(async move {
            download_chunk_with_retry(
                ContentDownloadContext {
                    client: &client,
                    url: &url,
                    expected_file_bytes: Some(file_size),
                    semaphore: &semaphore,
                    task_control: task_control.as_ref(),
                    progress_callback: &chunk_progress_callback,
                    rel_path: &rel_path,
                },
                idx,
                start,
                end,
            )
            .await
        });

        handles.push(handle);
    }

    let mut chunks: Vec<(usize, Vec<u8>)> = Vec::with_capacity(num_chunks);
    let mut full_body_fallback = false;

    let mut first_error = None;
    for handle in handles {
        match handle.await {
            Ok(Ok((idx, data, is_fallback))) => {
                if is_fallback {
                    full_body_fallback = true;
                }
                chunks.push((idx, data));
            }
            Ok(Err(error)) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(anyhow!("Chunk download task panicked: {}", error));
                }
            }
        }
    }

    if let Some(error) = first_error {
        return Err(error);
    }

    if full_body_fallback {
        // Find the first chunk's data (which has the complete body)
        chunks.sort_by_key(|(idx, _)| *idx);
        if let Some((_, data)) = chunks
            .into_iter()
            .find(|(idx, data)| *idx == 0 && !data.is_empty())
        {
            return Ok(data);
        }
        return Err(anyhow!("Range fallback failed: no data from first chunk"));
    }

    // Concatenate chunks in order
    chunks.sort_by_key(|(idx, _)| *idx);
    let total_size = chunks.iter().map(|(_, d)| d.len()).sum();
    let mut result = Vec::with_capacity(total_size);
    for (_, data) in chunks {
        result.extend_from_slice(&data);
    }

    Ok(result)
}

struct ContentDownloadBatchConfig<'a> {
    expected_crc: &'a HashMap<String, i64>,
    parallel_downloads: usize,
    task_control: Option<TaskControl>,
    chunk_progress_callback: Option<Arc<dyn Fn(String, u64) + Send + Sync>>,
    file_completed_callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
    file_sizes: &'a HashMap<String, u64>,
    chunked_enabled: bool,
    threads_per_task: usize,
    total_threads: usize,
}

async fn download_files(
    base_url: &str,
    paths: &[String],
    config: ContentDownloadBatchConfig<'_>,
) -> Result<HashMap<String, Vec<u8>>> {
    if paths.is_empty() {
        return Ok(HashMap::new());
    }

    let ContentDownloadBatchConfig {
        expected_crc,
        parallel_downloads,
        task_control,
        chunk_progress_callback,
        file_completed_callback,
        file_sizes,
        chunked_enabled,
        threads_per_task,
        total_threads,
    } = config;

    let client = update_client(Duration::from_secs(30))?;

    let base = base_url.trim_end_matches('/').to_string();
    let expected_crc = Arc::new(expected_crc.clone());

    // Probe Range support once using first file URL
    let range_supported = if chunked_enabled && threads_per_task > 1 {
        if let Some(first_path) = paths.first() {
            let probe_url = join_url(&base, first_path)?;
            matches!(
                probe_range_support(&client, probe_url.as_str()).await,
                Ok(Some(_))
            )
        } else {
            false
        }
    } else {
        false
    };

    let semaphore = Arc::new(Semaphore::new(total_threads));
    let file_sizes = Arc::new(file_sizes.clone());

    let stream = stream::iter(paths.iter().cloned()).map(|rel_path| {
        let client = client.clone();
        let base = base.clone();
        let expected_crc = Arc::clone(&expected_crc);
        let task_control = task_control.clone();
        let chunk_progress_callback = chunk_progress_callback.clone();
        let file_completed_callback = file_completed_callback.clone();
        let semaphore = Arc::clone(&semaphore);
        let file_sizes = Arc::clone(&file_sizes);
        async move {
            ensure_not_cancelled(task_control.as_ref(), "install")?;
            let url = join_url(&base, &rel_path)?;

            let file_size = file_sizes.get(&rel_path).copied().unwrap_or(0);
            let use_chunked = chunked_enabled
                && range_supported
                && threads_per_task > 1
                && file_size >= CHUNKED_DOWNLOAD_MIN_SIZE;

            let data = if use_chunked {
                download_file_chunked(
                    &client,
                    url.as_str(),
                    file_size,
                    threads_per_task,
                    &semaphore,
                    &task_control,
                    &chunk_progress_callback,
                    &rel_path,
                )
                .await?
            } else {
                download_file_single_with_retry(ContentDownloadContext {
                    client: &client,
                    url: url.as_str(),
                    expected_file_bytes: (file_size > 0).then_some(file_size),
                    semaphore: &semaphore,
                    task_control: task_control.as_ref(),
                    progress_callback: &chunk_progress_callback,
                    rel_path: &rel_path,
                })
                .await?
            };

            if let Some(expected) = expected_crc.get(&rel_path) {
                if *expected >= 0 {
                    let actual_crc = crc32fast::hash(&data) as i64;
                    if actual_crc != *expected {
                        return Err(anyhow!(
                            "CRC mismatch for '{}': expected {}, got {}",
                            rel_path,
                            expected,
                            actual_crc
                        ));
                    }
                }
            }

            if let Some(cb) = file_completed_callback.as_ref() {
                cb(rel_path.clone());
            }

            Ok::<(String, Vec<u8>), anyhow::Error>((rel_path, data))
        }
    });

    let mut result: HashMap<String, Vec<u8>> = HashMap::new();
    let mut buffered = stream.buffer_unordered(parallel_downloads);
    while let Some(item) = buffered.next().await {
        let (path, data) = item?;
        result.insert(path, data);
    }

    Ok(result)
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

fn collect_local_relative_entries(target_root: &Path) -> Result<Vec<LocalRelativeEntry>> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(target_root).min_depth(1) {
        let entry = entry.with_context(|| {
            format!(
                "Failed to enumerate local addon content under '{}'",
                target_root.display()
            )
        })?;

        let rel_path = normalize_local_relative_path(target_root, entry.path())?;
        entries.push(LocalRelativeEntry {
            rel_path,
            is_dir: entry.file_type().is_dir(),
        });
    }

    Ok(entries)
}

fn normalize_local_relative_path(target_root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(target_root).with_context(|| {
        format!(
            "Failed to strip addon root '{}' from '{}'",
            target_root.display(),
            path.display()
        )
    })?;

    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            std::path::Component::Normal(segment) => {
                parts.push(segment.to_string_lossy().to_string());
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err(anyhow!("Local addon path escapes root: {}", path.display()));
            }
            _ => {}
        }
    }

    if parts.is_empty() {
        return Err(anyhow!("Local addon path is empty: {}", path.display()));
    }

    Ok(parts.join("/"))
}

fn update_local_cfg_fields(cfg_path: &Path, remote: &RemoteConfig) -> Result<()> {
    let content = fs::read_to_string(cfg_path)
        .with_context(|| format!("Failed to read cfg '{}'", cfg_path.display()))?;

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    upsert_cfg_field(&mut lines, "version", remote.version.trim());
    if let Some(zone) = remote.zone.as_deref() {
        upsert_cfg_field(&mut lines, "zone", zone.trim());
    }
    if let Some(module) = remote.module.as_deref() {
        upsert_cfg_field(&mut lines, "module", module.trim());
    }
    if let Some(name) = remote.name.as_deref() {
        upsert_cfg_field(&mut lines, "name", name.trim());
    }
    write_file_atomic(cfg_path, lines.join("\n").as_bytes())
        .with_context(|| format!("Failed to write cfg '{}'", cfg_path.display()))?;
    Ok(())
}

fn upsert_cfg_field(lines: &mut Vec<String>, key: &str, value: &str) {
    let key_prefix = format!("{}|", key.to_lowercase());
    for line in lines.iter_mut() {
        if line.trim().to_lowercase().starts_with(&key_prefix) {
            *line = format!("{}|{}", key, value);
            return;
        }
    }
    lines.push(format!("{}|{}", key, value));
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

fn is_livery_path(rel_path: &str) -> bool {
    let lower = rel_path.to_lowercase();
    lower.starts_with("liveries/") || lower.contains("/liveries/")
}

fn resolve_module_url(zone: Option<&str>, module: &str) -> Result<String> {
    let module_value = module.trim();
    if module_value.is_empty() {
        return Err(anyhow!("Missing 'module|' value"));
    }

    if ensure_http_or_https(module_value).is_ok() {
        return Ok(module_value.to_string());
    }

    let zone_value = zone
        .map(|z| z.trim())
        .filter(|z| !z.is_empty())
        .ok_or_else(|| {
            anyhow!(
                "Module '{}' is not an absolute URL and no valid 'zone|' was provided",
                module_value
            )
        })?;
    if ensure_http_or_https(zone_value).is_ok() {
        return join_url(zone_value, module_value);
    }

    // Older SkunkCrafts depots use a CDN zone name instead of a full URL.
    if zone_value.eq_ignore_ascii_case("custom")
        || zone_value.len() > 59
        || zone_value.starts_with('-')
        || zone_value.ends_with('-')
        || !zone_value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-')
    {
        return Err(anyhow!(
            "Invalid SkunkCrafts zone '{zone_value}' for a relative module"
        ));
    }
    join_url(
        &format!("http://{}-ccd.kxcdn.com/", zone_value.to_ascii_lowercase()),
        module_value,
    )
}

fn compute_file_crc32(path: &Path) -> Result<u32> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("Failed to open file '{}' for CRC32", path.display()))?;
    let mut hasher = Hasher::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize())
}

fn evict_local_crc_cache_if_needed() {
    let expired_keys: Vec<String> = LOCAL_CRC_CACHE
        .iter()
        .filter_map(|entry| {
            if let Ok(elapsed) = entry.value().cached_at.elapsed() {
                if elapsed >= LOCAL_CRC_CACHE_TTL {
                    return Some(entry.key().clone());
                }
            }
            None
        })
        .collect();
    for key in expired_keys {
        LOCAL_CRC_CACHE.remove(&key);
    }

    if LOCAL_CRC_CACHE.len() >= LOCAL_CRC_CACHE_MAX_SIZE {
        let remove_count = std::cmp::max(LOCAL_CRC_CACHE_MAX_SIZE / 10, 1000);
        let keys: Vec<String> = LOCAL_CRC_CACHE
            .iter()
            .take(remove_count)
            .map(|entry| entry.key().clone())
            .collect();
        for key in keys {
            LOCAL_CRC_CACHE.remove(&key);
        }
    }
}

fn get_cached_file_crc32(path: &Path, len: u64, modified: Option<SystemTime>) -> Option<u32> {
    let key = path.to_string_lossy().to_string();
    if let Some(entry) = LOCAL_CRC_CACHE.get(&key) {
        let cache = entry.value();
        let is_valid = cache.len == len
            && cache.modified == modified
            && cache
                .cached_at
                .elapsed()
                .map(|elapsed| elapsed < LOCAL_CRC_CACHE_TTL)
                .unwrap_or(false);
        if is_valid {
            return Some(cache.crc32);
        }
        drop(entry);
        LOCAL_CRC_CACHE.remove(&key);
    }
    None
}

fn cache_file_crc32(path: &Path, len: u64, modified: Option<SystemTime>, crc32: u32) {
    evict_local_crc_cache_if_needed();
    LOCAL_CRC_CACHE.insert(
        path.to_string_lossy().to_string(),
        LocalCrcCacheEntry {
            crc32,
            len,
            modified,
            cached_at: SystemTime::now(),
        },
    );
}

fn compute_file_crc32_cached(path: &Path) -> Result<u32> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to read file metadata '{}'", path.display()))?;
    let len = metadata.len();
    let modified = metadata.modified().ok();
    if let Some(cached) = get_cached_file_crc32(path, len, modified) {
        return Ok(cached);
    }
    let crc32 = compute_file_crc32(path)?;
    cache_file_crc32(path, len, modified, crc32);
    Ok(crc32)
}

fn parse_cfg_lines(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('|') {
            let k = k.trim();
            let lower = k.to_ascii_lowercase();
            let key = if let Some(name) = lower.strip_prefix("channel.") {
                format!("channel.{}", &k[k.len() - name.len()..])
            } else {
                lower
            };
            let value = v.trim().to_string();
            if !key.is_empty() {
                map.insert(key, value);
            }
        }
    }
    map
}

fn parse_file_list(
    content: &str,
    combined: bool,
    warnings: &mut Vec<String>,
) -> Result<(Vec<WhitelistEntry>, HashMap<String, u64>)> {
    let mut files = BTreeMap::new();
    let mut sizes = HashMap::new();
    let mut rows = 0;
    for line in content.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        rows += 1;
        let Some((raw_path, metadata)) = line.split_once('|') else {
            warnings.push(format!(
                "Skipped malformed SkunkCrafts file list entry '{line}'"
            ));
            continue;
        };
        let path = match normalize_manifest_path(raw_path) {
            Ok(path) => path,
            Err(error) => {
                warnings.push(format!(
                    "Skipped unsafe file list path '{raw_path}': {error}"
                ));
                continue;
            }
        };
        let (raw_crc, size) = if combined {
            let Some((crc, size)) = metadata.split_once('|') else {
                warnings.push(format!("Skipped manifest entry '{path}' with no size"));
                continue;
            };
            let Ok(size) = size.trim().parse::<u64>() else {
                warnings.push(format!("Skipped manifest entry '{path}' with invalid size"));
                continue;
            };
            (crc, Some(size))
        } else {
            (metadata, None)
        };
        let Ok(crc32) = raw_crc.trim().parse::<i64>() else {
            warnings.push(format!("Skipped file list entry '{path}' with invalid CRC"));
            continue;
        };
        if crc32 != -1 && u32::try_from(crc32).is_err() {
            warnings.push(format!(
                "Skipped file list entry '{path}' with unsupported CRC"
            ));
            continue;
        }
        if let Some(size) = size {
            sizes.insert(path.clone(), size);
        }
        files.insert(path, crc32);
    }
    if rows != 0 && files.is_empty() {
        return Err(anyhow!("SkunkCrafts file list contains no valid entries"));
    }
    let whitelist = files
        .into_iter()
        .map(|(path, crc32)| WhitelistEntry { path, crc32 })
        .collect();
    Ok((whitelist, sizes))
}

fn parse_path_set(content: &str, warnings: &mut Vec<String>) -> HashSet<String> {
    let mut set = HashSet::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let path_raw = line.split('|').next().unwrap_or(line);
        match normalize_manifest_path(path_raw) {
            Ok(path) => {
                set.insert(path);
            }
            Err(e) => {
                warnings.push(format!(
                    "Skipped unsafe list path '{}': {}",
                    path_raw.trim(),
                    e
                ));
            }
        }
    }
    set
}

fn parse_sizes_list(content: &str, warnings: &mut Vec<String>) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (path_raw, size_raw) = match line.split_once('|') {
            Some(parts) => parts,
            None => {
                warnings.push(format!("Skipped malformed sizeslist line '{}'", line));
                continue;
            }
        };
        let path = match normalize_manifest_path(path_raw) {
            Ok(v) => v,
            Err(e) => {
                warnings.push(format!(
                    "Skipped unsafe sizeslist path '{}': {}",
                    path_raw.trim(),
                    e
                ));
                continue;
            }
        };
        let size = match size_raw.trim().parse::<u64>() {
            Ok(v) => v,
            Err(_) => {
                warnings.push(format!(
                    "Skipped sizeslist path '{}' due to invalid size '{}'",
                    path,
                    size_raw.trim()
                ));
                continue;
            }
        };
        map.insert(path, size);
    }
    map
}

fn ensure_http_or_https(url: &str) -> Result<()> {
    let parsed = Url::parse(url).with_context(|| format!("Invalid module URL '{}'", url))?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        _ => Err(anyhow!("Only HTTP(S) module URLs are supported")),
    }
}

async fn fetch_text_optional(
    client: &reqwest::Client,
    url: &str,
    cache_parameter: (&str, &str),
) -> Result<Option<String>> {
    use reqwest::StatusCode;

    let mut response = client
        .get(url)
        .query(&[cache_parameter])
        .send()
        .await
        .with_context(|| format!("Failed to request '{url}'"))?;
    if matches!(
        response.status(),
        StatusCode::NOT_FOUND | StatusCode::FORBIDDEN
    ) {
        // Some depots reject query parameters.
        response = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to request '{url}'"))?;
    }
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(anyhow!(
            "Request to '{url}' failed with HTTP {}",
            response.status()
        ));
    }

    let text = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from '{url}'"))?;
    let start = text.trim_start_matches('\u{feff}').trim_start();
    let prefix: String = start
        .chars()
        .take(16)
        .flat_map(char::to_lowercase)
        .collect();
    if prefix.starts_with("<!doctype html") || prefix.starts_with("<html") {
        return Err(anyhow!("SkunkCrafts depot returned a web page for '{url}'"));
    }
    Ok(Some(text))
}

fn join_url(base_url: &str, path: &str) -> Result<String> {
    ensure_http_or_https(base_url)?;
    let mut base = Url::parse(base_url).with_context(|| format!("Invalid URL '{base_url}'"))?;
    let normalized = normalize_manifest_path(path)?;
    base.path_segments_mut()
        .map_err(|_| anyhow!("URL cannot be used for path segments"))?
        .pop_if_empty()
        .extend(normalized.split('/'));
    base.set_fragment(None);
    Ok(base.to_string())
}

fn parse_bool(value: &str) -> Option<bool> {
    let v = value.trim().to_lowercase();
    match v.as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::thread::{self, JoinHandle};

    struct TestResponse {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    type TestHttpHandler = dyn Fn(&str) -> Option<TestResponse> + Send + Sync + 'static;

    struct TestHttpServer {
        base_url: String,
        address: SocketAddr,
        shutdown: Arc<AtomicBool>,
        handle: Option<JoinHandle<()>>,
    }

    impl TestHttpServer {
        fn start(handler: Arc<TestHttpHandler>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind test HTTP server");
            let address = listener.local_addr().expect("read test server address");
            listener
                .set_nonblocking(true)
                .expect("set test server nonblocking");
            let shutdown = Arc::new(AtomicBool::new(false));
            let shutdown_for_thread = Arc::clone(&shutdown);
            let handle = thread::spawn(move || {
                while !shutdown_for_thread.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            if shutdown_for_thread.load(Ordering::SeqCst) {
                                break;
                            }
                            // The listening socket is non-blocking so the server can poll
                            // the shutdown flag. On BSD-derived platforms such as macOS,
                            // accepted sockets may retain that mode, which makes the first
                            // request read race with the client and fail with WouldBlock.
                            // Handle each accepted connection in blocking mode instead.
                            stream
                                .set_nonblocking(false)
                                .expect("set test client blocking");
                            let request = read_http_request(&mut stream);
                            if let Some(response) = handler(&request) {
                                write_http_response(&mut stream, response);
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(error) => panic!("test HTTP server accept failed: {error}"),
                    }
                }
            });

            Self {
                base_url: format!("http://{address}"),
                address,
                shutdown,
                handle: Some(handle),
            }
        }
    }

    impl Drop for TestHttpServer {
        fn drop(&mut self) {
            self.shutdown.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(self.address);
            if let Some(handle) = self.handle.take() {
                handle.join().expect("join test HTTP server");
            }
        }
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set test request timeout");
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        loop {
            let read = stream.read(&mut buffer).expect("read test HTTP request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8(request).expect("test HTTP request is UTF-8")
    }

    fn write_http_response(stream: &mut TcpStream, response: TestResponse) {
        let reason = match response.status {
            200 => "OK",
            206 => "Partial Content",
            404 => "Not Found",
            503 => "Service Unavailable",
            _ => "Test Response",
        };
        let mut head = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            response.status,
            reason,
            response.body.len()
        );
        for (name, value) in response.headers {
            head.push_str(&format!("{}: {}\r\n", name, value));
        }
        head.push_str("\r\n");
        stream
            .write_all(head.as_bytes())
            .expect("write test response headers");
        stream
            .write_all(&response.body)
            .expect("write test response body");
    }

    fn request_header(request: &str, name: &str) -> Option<String> {
        request.lines().find_map(|line| {
            let (header_name, value) = line.split_once(':')?;
            header_name
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    }

    fn test_response(status: u16, body: impl Into<Vec<u8>>) -> TestResponse {
        TestResponse {
            status,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    fn update_options() -> SkunkUpdateOptions {
        serde_json::from_value(serde_json::json!({
            "includeLiveries": true,
            "applyBlacklist": true,
            "rollbackOnFailure": true,
            "chunkedDownloadEnabled": false,
        }))
        .unwrap()
    }

    fn write_fixture(path: &Path, relative_path: &str, text: &str) {
        let path = path.join(relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn remote_version(check: CheckedVersion) -> (String, bool) {
        (check.remote, check.locked)
    }

    #[tokio::test]
    async fn migrated_v3_beta_keeps_its_depot_and_can_still_update_and_repair() {
        let beta_revision = Arc::new(AtomicUsize::new(1));
        let current_revision = Arc::clone(&beta_revision);
        let server = TestHttpServer::start(Arc::new(move |request| {
            let url = request.split_whitespace().nth(1).unwrap();
            let (path, query) = url.split_once('?').unwrap_or((url, ""));
            let revision = current_revision.load(Ordering::SeqCst);
            let beta = format!("beta build {revision}");
            let response = match path {
                "/main/skunkcrafts_updater.cfg" => test_response(200, "version|2.8.22"),
                "/beta/skunkcrafts_updater.cfg" => {
                    assert!(query.starts_with("t="));
                    test_response(200, format!("version|2.8.23.{revision}"))
                }
                "/beta/skunkcrafts_updater_whitelist.txt" => {
                    assert_eq!(query, format!("v=2.8.23.{revision}"));
                    test_response(
                        200,
                        format!("plugin.xpl|{}\n", crc32fast::hash(beta.as_bytes())),
                    )
                }
                "/beta/plugin.xpl" => test_response(200, beta),
                _ => test_response(404, ""),
            };
            Some(response)
        }));
        let xp = tempfile::tempdir().unwrap();
        let path = xp.path().join("Resources/plugins/FlyWithLua");
        let main = format!("module|{}/main\nversion|2.8.23.1\n", server.base_url);
        let legacy_file = "skunkcrafts_updater_beta.cfg";
        write_fixture(&path, LOCAL_CFG_FILE, &main);
        write_fixture(
            &path,
            legacy_file,
            &format!("module|{}/beta\n", server.base_url),
        );
        write_fixture(&path, "plugin.xpl", "beta build 1");
        let client = update_client(Duration::from_secs(5)).unwrap();
        let check_version = |use_beta| check_local_version(&client, &path, use_beta);
        assert_eq!(
            check_version(None).await.unwrap().map(remote_version),
            Some(("2.8.23.1".into(), false))
        );
        assert_eq!(
            check_version(Some(false))
                .await
                .unwrap()
                .map(remote_version),
            Some(("2.8.22".into(), false))
        );
        write_fixture(
            &path,
            legacy_file,
            &format!("module|{}/beta\nenabled|false\n", server.base_url),
        );
        assert_eq!(
            check_version(None).await.unwrap().map(remote_version),
            Some(("2.8.22".into(), false))
        );
        assert_eq!(
            check_version(Some(true)).await.unwrap().map(remote_version),
            Some(("2.8.23.1".into(), false))
        );

        // Reproduce the official v4 import, including a sanitized on-disk channel filename.
        fs::remove_file(path.join(legacy_file)).unwrap();
        let selected = "channel|Beta candidate / test?\n";
        write_fixture(&path, "skunkcrafts/channel.cfg", selected);
        write_fixture(
            &path,
            "skunkcrafts/channels/Beta_candidate___test_.cfg",
            &format!("{selected}module|{}/beta\nzone|custom\n", server.base_url),
        );
        let mut options = update_options();
        options.use_beta = Some(false);
        let preview = build_update_plan(xp.path(), "plugin", "FlyWithLua", options.clone())
            .await
            .unwrap();
        assert_eq!(
            preview.active_channel.as_deref(),
            Some("Beta candidate / test?")
        );
        assert_eq!(preview.remote_version.as_deref(), Some("2.8.23.1"));
        assert!(!preview.has_update);
        assert!(!preview.has_beta_config);

        beta_revision.store(2, Ordering::SeqCst);
        let preview = build_update_plan(xp.path(), "plugin", "FlyWithLua", options.clone())
            .await
            .unwrap();
        assert_eq!(preview.replace_files, ["plugin.xpl"]);
        options.expected_skunk_module = preview.remote_module;
        let result = execute_update(
            xp.path(),
            "plugin",
            "FlyWithLua",
            options.clone(),
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.remote_version.as_deref(), Some("2.8.23.2"));
        assert_eq!(
            fs::read_to_string(path.join("plugin.xpl")).unwrap(),
            "beta build 2"
        );
        assert_eq!(
            fs::read_to_string(path.join("skunkcrafts/channel.cfg")).unwrap(),
            selected
        );
        let cfg = fs::read_to_string(path.join(LOCAL_CFG_FILE)).unwrap();
        assert!(cfg.contains("version|2.8.23.2"));
        assert!(cfg.contains(&format!("module|{}/main", server.base_url)));

        // An equal-version repair also uses the beta manifest.
        write_fixture(&path, "plugin.xpl", "damaged");
        let repair = build_update_plan(xp.path(), "plugin", "FlyWithLua", options)
            .await
            .unwrap();
        assert_eq!(repair.local_version, repair.remote_version);
        assert_eq!(repair.replace_files, ["plugin.xpl"]);
    }

    #[test]
    fn early_v4_configs_and_explicit_main_survive_mixed_layouts() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path();
        let main = "module|https://example.test/main\nversion|1.0\n";
        let inline = format!(
            "{main}channel|RC\nchannel.RC.module|https://example.test/rc\nchannel.RC.zone|custom\n"
        );
        write_fixture(path, "skunkcrafts/updater.cfg", &inline);
        let local = read_local_config(path, Some(false)).unwrap();
        assert!(local.cfg_path.ends_with("skunkcrafts/updater.cfg"));
        assert_eq!(local.selected.unwrap().url, "https://example.test/rc");

        write_fixture(path, LOCAL_CFG_FILE, &inline);
        write_fixture(
            path,
            "skunkcrafts_updater_beta.cfg",
            "module|https://example.test/stale\n",
        );
        write_fixture(path, "skunkcrafts/channel.cfg", "channel|stable\n");
        let local = read_local_config(path, Some(true)).unwrap();
        assert!(local.cfg_path.ends_with(LOCAL_CFG_FILE));
        assert!(local.selected.is_none());
        assert!(!local.has_legacy_beta);

        write_fixture(path, "skunkcrafts/channel.cfg", "channel|removed\n");
        write_fixture(
            path,
            "skunkcrafts/channels/removed.cfg.removed",
            "channel|removed\nmodule|https://example.test/removed\n",
        );
        assert!(read_local_config(path, None)
            .unwrap_err()
            .to_string()
            .contains("no longer available"));
    }

    #[test]
    fn historical_zone_and_encoded_depot_paths_resolve() {
        assert_eq!(
            resolve_module_url(Some("aerobask"), "products/Aircraft").unwrap(),
            "http://aerobask-ccd.kxcdn.com/products/Aircraft"
        );
        assert!(resolve_module_url(Some("custom"), "relative").is_err());
        assert!(resolve_module_url(Some("invalid.zone"), "relative").is_err());
        assert_eq!(
            join_url(
                "https://example.test/My%20folder/?token=example",
                "file name.txt"
            )
            .unwrap(),
            "https://example.test/My%20folder/file%20name.txt?token=example"
        );
    }

    #[tokio::test]
    async fn native_v4_public_fallback_requires_main_404_and_follows_named_selection() {
        let main_status = Arc::new(AtomicUsize::new(404));
        let status = Arc::clone(&main_status);
        let server = TestHttpServer::start(Arc::new(move |request| {
            let url = request.split_whitespace().nth(1).unwrap();
            let (path, query) = url.split_once('?').unwrap_or((url, ""));
            let response = match path {
                "/main/skunkcrafts_updater.cfg" => {
                    let status = status.load(Ordering::SeqCst) as u16;
                    test_response(status, "version|1.0")
                }
                "/a/skunkcrafts_updater.cfg" if !query.is_empty() => test_response(403, ""),
                "/a/skunkcrafts_updater.cfg" => test_response(200, "version|2.0"),
                "/z/skunkcrafts_updater.cfg" => test_response(200, "version|3.0"),
                _ => test_response(404, ""),
            };
            Some(response)
        }));
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path();
        let cfg = format!(
            "module|{url}/main\nversion|1.0\nchannel.Z beta|{url}/z\nchannel.A beta|{url}/a\n",
            url = server.base_url
        );
        write_fixture(path, LOCAL_CFG_FILE, &cfg);
        let client = update_client(Duration::from_secs(5)).unwrap();
        assert_eq!(
            check_local_version(&client, path, None)
                .await
                .unwrap()
                .map(remote_version),
            Some(("2.0".into(), false))
        );
        let legacy_beta = "skunkcrafts_updater_beta.cfg";
        write_fixture(path, legacy_beta, &format!("module|{}/z", server.base_url));
        assert!(check_local_version(&client, path, Some(false))
            .await
            .unwrap()
            .is_none());
        fs::remove_file(path.join(legacy_beta)).unwrap();

        main_status.store(503, Ordering::SeqCst);
        assert!(check_local_version(&client, path, None)
            .await
            .unwrap_err()
            .to_string()
            .contains("503"));
        write_fixture(path, "skunkcrafts/channel.cfg", "channel|Z beta\n");
        assert_eq!(
            check_local_version(&client, path, None)
                .await
                .unwrap()
                .map(remote_version),
            Some(("3.0".into(), false))
        );
    }

    #[tokio::test]
    async fn combined_manifest_keeps_updater_state_and_local_ignores_in_updates() {
        let content = b"new files";
        let server =
            TestHttpServer::start(Arc::new(move |request| {
                let url = request.split_whitespace().nth(1).unwrap();
                let (path, query) = url.split_once('?').unwrap_or((url, ""));
                let response = match path {
                "/skunkcrafts_updater.cfg" => test_response(200, "version|2.0\nprotocol|2"),
                "/skunkcrafts_updater_manifest.txt" => {
                    assert_eq!(query, "v=2.0");
                    test_response(200, format!(
                        "files/data.bin|{}|{}\nprefs.ini|-1|100\nskunkcrafts/channel.cfg|0|0\n",
                        crc32fast::hash(content), content.len()
                    ))
                }
                "/skunkcrafts_updater_blacklist.txt" => test_response(200,
                    "skunkcrafts\nskunkcrafts_updater.cfg\nuser\nskunkcrafts_updater_ignore.txt\n"),
                "/files/data.bin" => test_response(200, content.to_vec()),
                _ => test_response(404, ""),
            };
                Some(response)
            }));
        let xp = tempfile::tempdir().unwrap();
        let path = xp.path().join("Custom Scenery/Package");
        write_fixture(
            &path,
            LOCAL_CFG_FILE,
            &format!("module|{}\nversion|1.0", server.base_url),
        );
        write_fixture(&path, LOCAL_IGNORE_FILE, "user/keep.ini\n");
        write_fixture(&path, "user/keep.ini", "user file");
        write_fixture(&path, "user/remove.ini", "retired");
        write_fixture(&path, "prefs.ini", "modified preferences");
        write_fixture(&path, "skunkcrafts/channel.cfg", "channel|stable\n");
        let mut options = update_options();
        options.fresh_install = true;
        let plan = build_update_plan(xp.path(), "scenery", "Package", options.clone())
            .await
            .unwrap();
        assert_eq!(plan.add_files, ["files/data.bin"]);
        assert_eq!(plan.delete_files, ["user/remove.ini"]);
        assert_eq!(plan.estimated_download_bytes, content.len() as u64);

        options.expected_skunk_module = Some(format!("{}/other-channel", server.base_url));
        let error = execute_update(xp.path(), "scenery", "Package", options.clone(), None, None)
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), CHANNEL_CHANGED);
        assert!(!path.join("files/data.bin").exists());
        assert!(path.join("user/remove.ini").exists());

        options.expected_skunk_module = plan.remote_module;
        execute_update(xp.path(), "scenery", "Package", options, None, None)
            .await
            .unwrap();
        assert_eq!(fs::read(path.join("files/data.bin")).unwrap(), content);
        assert_eq!(
            fs::read_to_string(path.join("prefs.ini")).unwrap(),
            "modified preferences"
        );
        assert_eq!(
            fs::read_to_string(path.join("user/keep.ini")).unwrap(),
            "user file"
        );
        assert!(path.join("skunkcrafts/channel.cfg").exists());
        assert!(path.join(LOCAL_IGNORE_FILE).exists());
        assert!(!path.join("user/remove.ini").exists());
    }

    #[test]
    fn content_download_retry_policy_only_retries_temporary_http_failures() {
        for status in [408, 425, 429, 500, 502, 503, 504] {
            assert!(
                content_download_status_is_retryable(
                    reqwest::StatusCode::from_u16(status).expect("valid HTTP status")
                ),
                "HTTP {status} should be retried"
            );
        }

        for status in [400, 401, 403, 404, 416] {
            assert!(
                !content_download_status_is_retryable(
                    reqwest::StatusCode::from_u16(status).expect("valid HTTP status")
                ),
                "HTTP {status} should fail without retrying"
            );
        }
    }

    #[test]
    fn content_download_retry_delay_uses_a_bounded_exponential_backoff() {
        assert_eq!(content_download_retry_delay(1), Duration::from_millis(350));
        assert_eq!(content_download_retry_delay(2), Duration::from_millis(700));
        assert_eq!(
            content_download_retry_delay(3),
            Duration::from_millis(1_400)
        );
        assert_eq!(
            content_download_retry_delay(4),
            Duration::from_millis(2_000)
        );
        assert_eq!(
            content_download_retry_delay(10),
            Duration::from_millis(2_000)
        );
    }

    #[tokio::test]
    async fn chunked_download_retries_a_transient_connection_failure() {
        let payload = Arc::new(b"abcdefghijkl".to_vec());
        let failed_range_attempts = Arc::new(AtomicUsize::new(0));
        let payload_for_server = Arc::clone(&payload);
        let attempts_for_server = Arc::clone(&failed_range_attempts);
        let server = TestHttpServer::start(Arc::new(move |request| {
            let range = request_header(request, "range").expect("range request header");
            if range == "bytes=4-7" && attempts_for_server.fetch_add(1, Ordering::SeqCst) == 0 {
                return None;
            }

            let bounds = range
                .strip_prefix("bytes=")
                .and_then(|value| value.split_once('-'))
                .expect("valid byte range");
            let start = bounds.0.parse::<usize>().expect("valid range start");
            let end = bounds.1.parse::<usize>().expect("valid range end");
            Some(TestResponse {
                status: 206,
                headers: vec![(
                    "Content-Range".to_string(),
                    format!("bytes {start}-{end}/{}", payload_for_server.len()),
                )],
                body: payload_for_server[start..=end].to_vec(),
            })
        }));

        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("build test HTTP client");
        let semaphore = Arc::new(Semaphore::new(3));
        let processed = Arc::new(AtomicU64::new(0));
        let processed_for_callback = Arc::clone(&processed);
        let progress_callback: Option<Arc<dyn Fn(String, u64) + Send + Sync>> =
            Some(Arc::new(move |_, bytes| {
                processed_for_callback.fetch_add(bytes, Ordering::SeqCst);
            }));

        let downloaded = download_file_chunked(
            &client,
            &format!("{}/content.bin", server.base_url),
            payload.len() as u64,
            3,
            &semaphore,
            &None,
            &progress_callback,
            "content.bin",
        )
        .await
        .expect("transient chunk failure should recover");

        assert_eq!(downloaded, *payload);
        assert_eq!(failed_range_attempts.load(Ordering::SeqCst), 2);
        assert_eq!(processed.load(Ordering::SeqCst), payload.len() as u64);
    }

    #[tokio::test]
    async fn single_download_retries_a_temporary_server_failure() {
        let payload = Arc::new(b"temporary failure recovered".to_vec());
        let attempts = Arc::new(AtomicUsize::new(0));
        let payload_for_server = Arc::clone(&payload);
        let attempts_for_server = Arc::clone(&attempts);
        let server = TestHttpServer::start(Arc::new(move |_| {
            let attempt = attempts_for_server.fetch_add(1, Ordering::SeqCst) + 1;
            if attempt == 1 {
                return Some(TestResponse {
                    status: 503,
                    headers: Vec::new(),
                    body: Vec::new(),
                });
            }
            Some(TestResponse {
                status: 200,
                headers: Vec::new(),
                body: payload_for_server.as_ref().clone(),
            })
        }));

        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("build test HTTP client");
        let semaphore = Arc::new(Semaphore::new(1));
        let processed = Arc::new(AtomicU64::new(0));
        let processed_for_callback = Arc::clone(&processed);
        let progress_callback: Option<Arc<dyn Fn(String, u64) + Send + Sync>> =
            Some(Arc::new(move |_, bytes| {
                processed_for_callback.fetch_add(bytes, Ordering::SeqCst);
            }));

        let url = format!("{}/content.bin", server.base_url);
        let downloaded = download_file_single_with_retry(ContentDownloadContext {
            client: &client,
            url: &url,
            expected_file_bytes: Some(payload.len() as u64),
            semaphore: &semaphore,
            task_control: None,
            progress_callback: &progress_callback,
            rel_path: "content.bin",
        })
        .await
        .expect("temporary server failure should recover");

        assert_eq!(downloaded, *payload);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(processed.load(Ordering::SeqCst), payload.len() as u64);
    }

    #[tokio::test]
    async fn single_download_does_not_retry_a_permanent_client_failure() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_server = Arc::clone(&attempts);
        let server = TestHttpServer::start(Arc::new(move |_| {
            attempts_for_server.fetch_add(1, Ordering::SeqCst);
            Some(TestResponse {
                status: 404,
                headers: Vec::new(),
                body: Vec::new(),
            })
        }));

        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("build test HTTP client");
        let url = format!("{}/missing.bin", server.base_url);
        let semaphore = Arc::new(Semaphore::new(1));
        let progress_callback = None;
        let error = download_file_single_with_retry(ContentDownloadContext {
            client: &client,
            url: &url,
            expected_file_bytes: None,
            semaphore: &semaphore,
            task_control: None,
            progress_callback: &progress_callback,
            rel_path: "missing.bin",
        })
        .await
        .expect_err("permanent client failure should be returned");

        assert!(error.to_string().contains("HTTP 404"));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}

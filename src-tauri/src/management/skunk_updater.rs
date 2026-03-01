use anyhow::{anyhow, Context, Result};
use crc32fast::Hasher;
use futures::stream::{self, StreamExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::task_control::TaskControl;

const LOCAL_CFG_FILE: &str = "skunkcrafts_updater.cfg";
const LOCAL_BETA_CFG_FILE: &str = "skunkcraft_updater_beta.cfg";
const REMOTE_CONFIG_FILE: &str = "skunkcrafts_updater_config.txt";
const REMOTE_CFG_FALLBACK_FILE: &str = "skunkcrafts_updater.cfg";
const REMOTE_WHITELIST_FILE: &str = "skunkcrafts_updater_whitelist.txt";
const REMOTE_IGNORELIST_FILE: &str = "skunkcrafts_updater_ignorelist.txt";
const REMOTE_ONCELIST_FILE: &str = "skunkcrafts_updater_oncelist.txt";
const REMOTE_SIZESLIST_FILE: &str = "skunkcrafts_updater_sizeslist.txt";
const REMOTE_BLACKLIST_FILE: &str = "skunkcrafts_updater_blacklist.txt";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkunkUpdateOptions {
    pub use_beta: bool,
    pub include_liveries: bool,
    pub apply_blacklist: bool,
    pub rollback_on_failure: bool,
    #[serde(default)]
    pub parallel_downloads: Option<usize>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub fresh_install: bool,
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
    pub remote_locked: bool,
    pub has_update: bool,
    pub estimated_download_bytes: u64,
    pub add_files: Vec<String>,
    pub replace_files: Vec<String>,
    pub delete_files: Vec<String>,
    pub skip_files: Vec<String>,
    pub warnings: Vec<String>,
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
struct LocalConfig {
    cfg_path: PathBuf,
    module: String,
    version: Option<String>,
    liveries: bool,
    beta_module: Option<String>,
}

#[derive(Debug, Clone)]
struct RemoteConfig {
    version: String,
    locked: bool,
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

#[derive(Debug, Clone)]
struct PreparedUpdate {
    local: LocalConfig,
    remote: RemoteConfig,
    manifest: RemoteManifest,
    module_url: String,
    target_path: PathBuf,
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
    let prepared = prepare_update_context(&target_path, options.use_beta).await?;
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
    let prepared = prepare_update_context(&target_path, options.use_beta).await?;
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
        if let Some(remote_version) = plan.remote_version.as_ref() {
            update_local_cfg_version(&prepared.local.cfg_path, remote_version)?;
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
    let downloaded = match download_files(
        &prepared.module_url,
        &download_targets,
        &whitelist_crc,
        &prepared.manifest.sizes,
        parallel,
        task_control.clone(),
        chunk_progress_callback,
        file_completed_callback,
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

        if let Some(remote_version) = plan.remote_version.as_ref() {
            update_local_cfg_version(&prepared.local.cfg_path, remote_version)?;
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
    let mut warnings = prepared.manifest.warnings.clone();

    let mut whitelist_paths: HashSet<String> = HashSet::new();
    for entry in &prepared.manifest.whitelist {
        whitelist_paths.insert(entry.path.clone());
    }

    let include_liveries = options.include_liveries && prepared.local.liveries;

    for entry in &prepared.manifest.whitelist {
        let rel_path = &entry.path;

        if !include_liveries && is_livery_path(rel_path) {
            skip_files.push(rel_path.clone());
            continue;
        }

        if prepared.manifest.ignorelist.contains(rel_path) {
            skip_files.push(rel_path.clone());
            continue;
        }

        let local_path = resolve_entry_path(&prepared.target_path, rel_path)?;
        let should_copy_once = entry.crc32 < 0 || prepared.manifest.oncelist.contains(rel_path);
        if should_copy_once {
            if local_path.exists() {
                skip_files.push(rel_path.clone());
            } else {
                add_files.push(rel_path.clone());
            }
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

        let local_crc = compute_file_crc32(&local_path)? as i64;
        if local_crc != entry.crc32 {
            replace_files.push(rel_path.clone());
        } else {
            skip_files.push(rel_path.clone());
        }
    }

    if options.apply_blacklist {
        for rel_path in &prepared.manifest.blacklist {
            if whitelist_paths.contains(rel_path) {
                warnings.push(format!(
                    "Blacklist entry '{}' is also in whitelist; skipped for safety",
                    rel_path
                ));
                continue;
            }
            if !include_liveries && is_livery_path(rel_path) {
                continue;
            }
            let local_path = resolve_entry_path(&prepared.target_path, rel_path)?;
            if local_path.exists() {
                delete_files.push(rel_path.clone());
            }
        }
    }

    let mut estimated_download_bytes: u64 = 0;
    for rel_path in add_files.iter().chain(replace_files.iter()) {
        if let Some(size) = prepared.manifest.sizes.get(rel_path) {
            estimated_download_bytes = estimated_download_bytes.saturating_add(*size);
        }
    }

    let local_version = prepared.local.version.clone();
    let remote_version = Some(prepared.remote.version.clone());
    let has_update = remote_version.as_deref() != local_version.as_deref();

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
        remote_locked: prepared.remote.locked,
        has_update,
        estimated_download_bytes,
        add_files,
        replace_files,
        delete_files,
        skip_files,
        warnings,
    })
}

async fn prepare_update_context(target_path: &Path, use_beta: bool) -> Result<PreparedUpdate> {
    let local = read_local_config(target_path)?;
    let module_url = select_module_url(&local, use_beta);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("XFast Manager")
        .build()
        .context("Failed to build HTTP client")?;

    let remote = fetch_remote_config(&client, &module_url).await?;
    let manifest = fetch_remote_manifest(&client, &module_url).await?;

    Ok(PreparedUpdate {
        local,
        remote,
        manifest,
        module_url,
        target_path: target_path.to_path_buf(),
    })
}

fn read_local_config(target_path: &Path) -> Result<LocalConfig> {
    let cfg_path = target_path.join(LOCAL_CFG_FILE);
    if !cfg_path.exists() {
        return Err(anyhow!(
            "Missing '{}' in '{}'",
            LOCAL_CFG_FILE,
            target_path.display()
        ));
    }

    let cfg_content = fs::read_to_string(&cfg_path)
        .with_context(|| format!("Failed to read {}", cfg_path.display()))?;
    let cfg_map = parse_cfg_lines(&cfg_content);

    let module = cfg_map
        .get("module")
        .cloned()
        .ok_or_else(|| anyhow!("Missing 'module|' in {}", cfg_path.display()))?;
    ensure_http_or_https(&module)?;

    let version = cfg_map.get("version").cloned();
    let liveries = cfg_map
        .get("liveries")
        .map(|v| parse_bool(v).unwrap_or(true))
        .unwrap_or(true);

    let beta_cfg_path = target_path.join(LOCAL_BETA_CFG_FILE);
    let beta_module = if beta_cfg_path.exists() {
        let beta_content = fs::read_to_string(&beta_cfg_path)
            .with_context(|| format!("Failed to read {}", beta_cfg_path.display()))?;
        let beta_map = parse_cfg_lines(&beta_content);
        beta_map
            .get("module")
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    } else {
        None
    };

    if let Some(beta) = beta_module.as_ref() {
        ensure_http_or_https(beta)?;
    }

    Ok(LocalConfig {
        cfg_path,
        module: module.trim().to_string(),
        version: version
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
        liveries,
        beta_module,
    })
}

fn select_module_url(local: &LocalConfig, use_beta: bool) -> String {
    if use_beta {
        if let Some(beta) = local.beta_module.as_ref() {
            return beta.clone();
        }
    }
    local.module.clone()
}

async fn fetch_remote_config(client: &reqwest::Client, base_url: &str) -> Result<RemoteConfig> {
    let primary_url = join_url(base_url, REMOTE_CONFIG_FILE)?;
    let fallback_url = join_url(base_url, REMOTE_CFG_FALLBACK_FILE)?;

    let primary = fetch_text_optional(client, &primary_url).await?;
    let content = if let Some(text) = primary {
        text
    } else {
        fetch_text_required(client, &fallback_url).await?
    };

    let map = parse_cfg_lines(&content);
    let version = map
        .get("version")
        .cloned()
        .ok_or_else(|| anyhow!("Remote config is missing 'version|'"))?;
    let locked = map
        .get("locked")
        .map(|v| parse_bool(v).unwrap_or(false))
        .unwrap_or(false);

    Ok(RemoteConfig {
        version: version.trim().to_string(),
        locked,
    })
}

async fn fetch_remote_manifest(client: &reqwest::Client, base_url: &str) -> Result<RemoteManifest> {
    let whitelist_url = join_url(base_url, REMOTE_WHITELIST_FILE)?;
    let whitelist_text = fetch_text_required(client, &whitelist_url).await?;
    let mut warnings = Vec::new();
    let whitelist = parse_whitelist(&whitelist_text, &mut warnings)?;
    if whitelist.is_empty() {
        return Err(anyhow!("Remote whitelist is empty"));
    }

    let ignorelist = {
        let url = join_url(base_url, REMOTE_IGNORELIST_FILE)?;
        if let Some(text) = fetch_text_optional(client, &url).await? {
            parse_path_set(&text, &mut warnings)
        } else {
            HashSet::new()
        }
    };
    let oncelist = {
        let url = join_url(base_url, REMOTE_ONCELIST_FILE)?;
        if let Some(text) = fetch_text_optional(client, &url).await? {
            parse_path_set(&text, &mut warnings)
        } else {
            HashSet::new()
        }
    };
    let sizes = {
        let url = join_url(base_url, REMOTE_SIZESLIST_FILE)?;
        if let Some(text) = fetch_text_optional(client, &url).await? {
            parse_sizes_list(&text, &mut warnings)
        } else {
            HashMap::new()
        }
    };
    let blacklist = {
        let url = join_url(base_url, REMOTE_BLACKLIST_FILE)?;
        if let Some(text) = fetch_text_optional(client, &url).await? {
            parse_path_set(&text, &mut warnings)
        } else {
            HashSet::new()
        }
    };

    Ok(RemoteManifest {
        whitelist,
        ignorelist,
        oncelist,
        sizes,
        blacklist,
        warnings,
    })
}

async fn download_files(
    base_url: &str,
    paths: &[String],
    expected_crc: &HashMap<String, i64>,
    expected_sizes: &HashMap<String, u64>,
    parallel_downloads: usize,
    task_control: Option<TaskControl>,
    chunk_progress_callback: Option<Arc<dyn Fn(String, u64) + Send + Sync>>,
    file_completed_callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
) -> Result<HashMap<String, Vec<u8>>> {
    if paths.is_empty() {
        return Ok(HashMap::new());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("XFast Manager")
        .build()
        .context("Failed to build HTTP client")?;

    let base = base_url.trim_end_matches('/').to_string();
    let expected_crc = Arc::new(expected_crc.clone());
    let expected_sizes = Arc::new(expected_sizes.clone());

    let stream = stream::iter(paths.iter().cloned()).map(|rel_path| {
        let client = client.clone();
        let base = base.clone();
        let expected_crc = Arc::clone(&expected_crc);
        let expected_sizes = Arc::clone(&expected_sizes);
        let task_control = task_control.clone();
        let chunk_progress_callback = chunk_progress_callback.clone();
        let file_completed_callback = file_completed_callback.clone();
        async move {
            ensure_not_cancelled(task_control.as_ref(), "install")?;
            let url = join_url(&base, &rel_path)?;
            let response = client
                .get(url.clone())
                .send()
                .await
                .with_context(|| format!("Failed to download '{}'", url))?;

            if !response.status().is_success() {
                return Err(anyhow!(
                    "Download failed for '{}': HTTP {}",
                    url,
                    response.status()
                ));
            }

            let mut data = Vec::new();
            let mut stream = response.bytes_stream();
            while let Some(next_chunk) = stream.next().await {
                ensure_not_cancelled(task_control.as_ref(), "install")?;
                let chunk = next_chunk
                    .with_context(|| format!("Failed to stream response body for '{}'", url))?;
                data.extend_from_slice(&chunk);
                if let Some(cb) = chunk_progress_callback.as_ref() {
                    cb(rel_path.clone(), chunk.len() as u64);
                }
            }

            if let Some(expected) = expected_sizes.get(&rel_path) {
                let actual = data.len() as u64;
                if actual != *expected {
                    return Err(anyhow!(
                        "Size mismatch for '{}': expected {}, got {}",
                        rel_path,
                        expected,
                        actual
                    ));
                }
            }

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

fn update_local_cfg_version(cfg_path: &Path, version: &str) -> Result<()> {
    let content = fs::read_to_string(cfg_path)
        .with_context(|| format!("Failed to read cfg '{}'", cfg_path.display()))?;

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut replaced = false;
    for line in &mut lines {
        if line.trim().to_lowercase().starts_with("version|") {
            *line = format!("version|{}", version.trim());
            replaced = true;
            break;
        }
    }
    if !replaced {
        lines.push(format!("version|{}", version.trim()));
    }
    fs::write(cfg_path, lines.join("\n"))
        .with_context(|| format!("Failed to write cfg '{}'", cfg_path.display()))?;
    Ok(())
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

fn compute_file_crc32(path: &Path) -> Result<u32> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("Failed to open file '{}' for CRC32", path.display()))?;
    let mut hasher = Hasher::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize())
}

fn parse_cfg_lines(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('|') {
            let key = k.trim().to_lowercase();
            let value = v.trim().to_string();
            if !key.is_empty() {
                map.insert(key, value);
            }
        }
    }
    map
}

fn parse_whitelist(content: &str, warnings: &mut Vec<String>) -> Result<Vec<WhitelistEntry>> {
    let mut map: HashMap<String, i64> = HashMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (path_raw, crc_raw) = match line.split_once('|') {
            Some(parts) => parts,
            None => {
                warnings.push(format!("Skipped malformed whitelist line '{}'", line));
                continue;
            }
        };
        let path = match normalize_manifest_path(path_raw) {
            Ok(v) => v,
            Err(e) => {
                warnings.push(format!(
                    "Skipped unsafe whitelist path '{}': {}",
                    path_raw.trim(),
                    e
                ));
                continue;
            }
        };
        let crc32 = match crc_raw.trim().parse::<i64>() {
            Ok(v) => v,
            Err(_) => {
                warnings.push(format!(
                    "Skipped whitelist path '{}' due to invalid CRC '{}'",
                    path,
                    crc_raw.trim()
                ));
                continue;
            }
        };
        map.insert(path, crc32);
    }

    Ok(map
        .into_iter()
        .map(|(path, crc32)| WhitelistEntry { path, crc32 })
        .collect())
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

async fn fetch_text_optional(client: &reqwest::Client, url: &str) -> Result<Option<String>> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to request '{}'", url))?;

    if response.status().as_u16() == 404 {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(anyhow!(
            "Request to '{}' failed with HTTP {}",
            url,
            response.status()
        ));
    }

    let text = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from '{}'", url))?;
    Ok(Some(text))
}

async fn fetch_text_required(client: &reqwest::Client, url: &str) -> Result<String> {
    fetch_text_optional(client, url)
        .await?
        .ok_or_else(|| anyhow!("Required remote file '{}' was not found", url))
}

fn join_url(base_url: &str, path: &str) -> Result<String> {
    ensure_http_or_https(base_url)?;
    let mut base = Url::parse(base_url).with_context(|| format!("Invalid URL '{}'", base_url))?;

    let mut new_segments: Vec<String> = Vec::new();
    if let Some(existing) = base.path_segments() {
        for seg in existing {
            if !seg.is_empty() {
                new_segments.push(seg.to_string());
            }
        }
    }

    let normalized = normalize_manifest_path(path)?;
    for seg in normalized.split('/') {
        new_segments.push(seg.to_string());
    }

    {
        let mut path_segments = base
            .path_segments_mut()
            .map_err(|_| anyhow!("URL cannot be used for path segments"))?;
        path_segments.clear();
        for seg in &new_segments {
            path_segments.push(seg);
        }
    }

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

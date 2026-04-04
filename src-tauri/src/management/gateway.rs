use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use futures::stream::{self, StreamExt};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri::{AppHandle, State};
use tempfile::tempdir;

use crate::activity;
use crate::analyzer::Analyzer;
use crate::database::entities::gateway_installs;
use crate::database::DatabaseState;
use crate::error::{ApiError, ApiErrorCode, ApiResult};
use crate::installer::Installer;
use crate::livery_patterns;
use crate::logger;
use crate::models::{AddonType, InstallTask};
use crate::path_utils;
use crate::scenery_classifier::classify_scenery;
use crate::scenery_index::SceneryIndexManager;
use crate::scenery_packs_manager::SceneryPacksManager;

const GATEWAY_API_BASE: &str = "https://gateway.x-plane.com/apiv1";
const AIRPORT_DIRECTORY_CACHE_TTL: Duration = Duration::from_secs(30 * 60);
const UPDATE_CHECK_CONCURRENCY: usize = 4;
const EXTERNAL_AIRPORT_CONFLICT_DETAIL: &str = "gateway_external_airport_conflict";

static GATEWAY_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("XFast-Manager Gateway/1.0")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
        .expect("gateway client")
});

static AIRPORT_DIRECTORY_CACHE: LazyLock<RwLock<Option<GatewayAirportDirectoryCache>>> =
    LazyLock::new(|| RwLock::new(None));

#[derive(Debug, Clone)]
struct GatewayAirportDirectoryCache {
    fetched_at: SystemTime,
    airports: Vec<GatewayAirportSearchResult>,
}

#[derive(Debug, Clone)]
struct GatewayAirportSummaryData {
    icao: String,
    airport_name: Option<String>,
    scenery_count: Option<i64>,
    recommended_scenery_id: Option<i64>,
    recommended_artist: Option<String>,
    recommended_accepted_at: Option<String>,
}

#[derive(Debug, Clone)]
struct GatewaySceneryInstallPayload {
    scenery_id: i64,
    icao: Option<String>,
    airport_name: Option<String>,
    status: Option<String>,
    artist: Option<String>,
    approved_date: Option<String>,
    comment: Option<String>,
    features: Vec<String>,
    master_zip_blob: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayInstallRequest {
    pub xplane_path: String,
    pub icao: String,
    pub scenery_id: i64,
    pub auto_sort_scenery: Option<bool>,
    pub ignore_external_conflict: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayAirportSearchResult {
    pub icao: String,
    pub airport_name: Option<String>,
    pub scenery_count: Option<i64>,
    pub recommended_scenery_id: Option<i64>,
    pub recommended_artist: Option<String>,
    pub recommended_accepted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayScenerySummary {
    pub scenery_id: i64,
    pub artist: Option<String>,
    pub status: Option<String>,
    pub approved_date: Option<String>,
    pub comment: Option<String>,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayAirportDetail {
    pub icao: String,
    pub airport_name: Option<String>,
    pub scenery_count: Option<i64>,
    pub recommended_scenery_id: Option<i64>,
    pub recommended_artist: Option<String>,
    pub recommended_accepted_at: Option<String>,
    pub sceneries: Vec<GatewayScenerySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySceneryDetail {
    pub scenery_id: i64,
    pub icao: Option<String>,
    pub airport_name: Option<String>,
    pub status: Option<String>,
    pub artist: Option<String>,
    pub approved_date: Option<String>,
    pub comment: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayInstalledAirport {
    pub id: i64,
    pub airport_icao: String,
    pub airport_name: String,
    pub scenery_id: i64,
    pub folder_name: String,
    pub artist: Option<String>,
    pub approved_date: Option<String>,
    pub installed_at: i64,
    pub update_available: Option<bool>,
    pub latest_scenery_id: Option<i64>,
    pub latest_artist: Option<String>,
    pub latest_approved_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayInstallWarning {
    pub kind: String,
    pub message: String,
}

#[tauri::command]
pub async fn gateway_search_airports(
    query: String,
    limit: Option<usize>,
) -> ApiResult<Vec<GatewayAirportSearchResult>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let directory = fetch_airport_directory().await?;
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let query_lower = query.to_ascii_lowercase();

    let mut matches: Vec<(usize, GatewayAirportSearchResult)> = directory
        .into_iter()
        .filter_map(|airport| {
            airport_match_score(&airport, &query_lower).map(|score| (score, airport))
        })
        .collect();

    matches.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.icao.cmp(&b.1.icao))
            .then_with(|| a.1.airport_name.cmp(&b.1.airport_name))
    });

    Ok(matches
        .into_iter()
        .take(limit)
        .map(|(_, airport)| airport)
        .collect())
}

#[tauri::command]
pub async fn gateway_get_airport(icao: String) -> ApiResult<GatewayAirportDetail> {
    let icao = normalize_icao(&icao)?;
    let payload = fetch_gateway_airport_payload(&icao).await?;
    parse_gateway_airport_detail(&payload, &icao).ok_or_else(|| {
        ApiError::corrupted(format!(
            "Gateway airport response for {} is missing expected fields",
            icao
        ))
    })
}

#[tauri::command]
pub async fn gateway_get_scenery(scenery_id: i64) -> ApiResult<GatewaySceneryDetail> {
    let payload = fetch_gateway_scenery_payload(scenery_id).await?;
    let detail = parse_gateway_scenery_detail(&payload, scenery_id).ok_or_else(|| {
        ApiError::corrupted(format!(
            "Gateway scenery response for {} is missing expected fields",
            scenery_id
        ))
    })?;
    Ok(sanitize_scenery_detail(detail))
}

#[tauri::command]
pub async fn gateway_list_installed(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> ApiResult<Vec<GatewayInstalledAirport>> {
    let xplane_root = validate_xplane_root(&xplane_path)?;
    let xplane_key = normalize_xplane_key(&xplane_root);
    list_installed_internal(&db.get(), &xplane_root, &xplane_key).await
}

#[tauri::command]
pub async fn gateway_check_updates(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> ApiResult<Vec<GatewayInstalledAirport>> {
    let xplane_root = validate_xplane_root(&xplane_path)?;
    let xplane_key = normalize_xplane_key(&xplane_root);
    let installed = list_installed_internal(&db.get(), &xplane_root, &xplane_key).await?;
    if installed.is_empty() {
        return Ok(installed);
    }

    let results: Vec<(GatewayInstalledAirport, bool)> = stream::iter(installed.into_iter())
        .map(|installed| async move {
            match fetch_gateway_airport_payload(&installed.airport_icao).await {
                Ok(payload) => {
                    let summary = parse_gateway_airport_summary(&payload, &installed.airport_icao);
                    let mut next = installed.clone();
                    if let Some(summary) = summary {
                        next.latest_scenery_id = summary.recommended_scenery_id;
                        next.latest_artist = summary.recommended_artist;
                        next.latest_approved_date = summary.recommended_accepted_at;
                        next.update_available = summary
                            .recommended_scenery_id
                            .map(|latest| latest != next.scenery_id);
                        (next, true)
                    } else {
                        next.update_available = None;
                        (next, false)
                    }
                }
                Err(error) => {
                    logger::log_error(
                        &format!(
                            "Failed to check Gateway updates for {}: {}",
                            installed.airport_icao, error
                        ),
                        Some("gateway"),
                    );
                    let mut next = installed.clone();
                    next.update_available = None;
                    (next, false)
                }
            }
        })
        .buffer_unordered(UPDATE_CHECK_CONCURRENCY)
        .collect()
        .await;

    let success_count = results.iter().filter(|(_, success)| *success).count();
    if success_count == 0 {
        return Err(ApiError::new(
            ApiErrorCode::NetworkError,
            "Failed to reach X-Plane Gateway for update checks",
        ));
    }

    Ok(results.into_iter().map(|(installed, _)| installed).collect())
}

fn validate_xplane_root(xplane_path: &str) -> ApiResult<PathBuf> {
    let trimmed = xplane_path.trim();
    if trimmed.is_empty() {
        return Err(ApiError::validation("X-Plane path is required"));
    }

    let path = PathBuf::from(trimmed);
    if !path.exists() {
        return Err(ApiError::not_found("Configured X-Plane path does not exist"));
    }
    if !path.is_dir() {
        return Err(ApiError::validation("Configured X-Plane path must be a directory"));
    }
    Ok(path)
}

fn normalize_xplane_key(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        text.to_ascii_lowercase()
    } else {
        text
    }
}

fn normalize_icao(icao: &str) -> ApiResult<String> {
    let normalized = icao.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return Err(ApiError::validation("ICAO is required"));
    }
    Ok(normalized)
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

async fn fetch_airport_directory() -> ApiResult<Vec<GatewayAirportSearchResult>> {
    if let Some(cached) = AIRPORT_DIRECTORY_CACHE
        .read()
        .ok()
        .and_then(|cache| cache.clone())
    {
        if cached
            .fetched_at
            .elapsed()
            .unwrap_or_default()
            .lt(&AIRPORT_DIRECTORY_CACHE_TTL)
        {
            return Ok(cached.airports);
        }
    }

    let payload = fetch_gateway_json(&format!("{}/airports", GATEWAY_API_BASE)).await?;
    let mut airports = extract_airport_directory_entries(&payload);
    airports.sort_by(|a, b| a.icao.cmp(&b.icao));

    if let Ok(mut cache) = AIRPORT_DIRECTORY_CACHE.write() {
        *cache = Some(GatewayAirportDirectoryCache {
            fetched_at: SystemTime::now(),
            airports: airports.clone(),
        });
    }

    Ok(airports)
}

async fn fetch_gateway_airport_payload(icao: &str) -> ApiResult<Value> {
    fetch_gateway_json(&format!("{}/airport/{}", GATEWAY_API_BASE, icao)).await
}

async fn fetch_gateway_scenery_payload(scenery_id: i64) -> ApiResult<Value> {
    if scenery_id <= 0 {
        return Err(ApiError::validation("Scenery ID must be a positive integer"));
    }
    fetch_gateway_json(&format!("{}/scenery/{}", GATEWAY_API_BASE, scenery_id)).await
}

async fn fetch_gateway_json(url: &str) -> ApiResult<Value> {
    let response = GATEWAY_HTTP_CLIENT
        .get(url)
        .send()
        .await
        .map_err(|error| ApiError::new(ApiErrorCode::NetworkError, error.to_string()))?;

    if !response.status().is_success() {
        return Err(ApiError::new(
            ApiErrorCode::NetworkError,
            format!("Gateway API request failed with status {}", response.status()),
        ));
    }

    response
        .json::<Value>()
        .await
        .map_err(|error| ApiError::corrupted(format!("Failed to parse Gateway response: {}", error)))
}

fn extract_airport_directory_entries(payload: &Value) -> Vec<GatewayAirportSearchResult> {
    if let Some(array) = payload.as_array() {
        return array
            .iter()
            .filter_map(|value| parse_gateway_airport_summary(value, ""))
            .map(summary_to_search_result)
            .collect();
    }

    let Some(root) = payload.as_object() else {
        return Vec::new();
    };

    pick_array(root, &["airports", "Airports", "items", "results", "data"])
        .into_iter()
        .flatten()
        .filter_map(|value| parse_gateway_airport_summary(value, ""))
        .map(summary_to_search_result)
        .collect()
}

fn airport_match_score(airport: &GatewayAirportSearchResult, query: &str) -> Option<usize> {
    let icao = airport.icao.to_ascii_lowercase();
    let name = airport
        .airport_name
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if icao == query {
        return Some(0);
    }
    if icao.starts_with(query) {
        return Some(1);
    }
    if name.starts_with(query) {
        return Some(2);
    }
    if name.contains(query) {
        return Some(3);
    }

    let all_tokens_match = query
        .split_whitespace()
        .all(|token| !token.is_empty() && (icao.contains(token) || name.contains(token)));
    if all_tokens_match {
        return Some(4);
    }

    None
}

async fn gateway_install_scenery_impl(
    app_handle: AppHandle,
    conn: DatabaseConnection,
    request: GatewayInstallRequest,
) -> ApiResult<GatewayInstalledAirport> {
    let GatewayInstallRequest {
        xplane_path,
        icao,
        scenery_id,
        auto_sort_scenery,
        ignore_external_conflict,
    } = request;
    let xplane_root = validate_xplane_root(&xplane_path)?;
    let xplane_key = normalize_xplane_key(&xplane_root);
    let airport_icao = normalize_icao(&icao)?;
    let skip_external_conflict_check = ignore_external_conflict.unwrap_or(false);

    logger::log_info(
        &format!(
            "Gateway install requested for {} scenery {} (skip_external_conflict_check={})",
            airport_icao, scenery_id, skip_external_conflict_check
        ),
        Some("gateway"),
    );

    let should_auto_sort = auto_sort_scenery.unwrap_or(false);

    let installed_before = find_install_by_airport(&conn, &xplane_key, &airport_icao).await?;

    let airport_payload = fetch_gateway_airport_payload(&airport_icao).await?;
    let airport_detail =
        parse_gateway_airport_detail(&airport_payload, &airport_icao).ok_or_else(|| {
            ApiError::corrupted(format!(
                "Gateway airport response for {} is missing expected fields",
                airport_icao
            ))
        })?;

    if !skip_external_conflict_check {
        ensure_no_external_airport_conflict(&conn, &xplane_root, &airport_icao).await?;
    }

    let scenery_payload = fetch_gateway_scenery_payload(scenery_id).await?;
    let scenery_detail =
        parse_gateway_scenery_detail(&scenery_payload, scenery_id).ok_or_else(|| {
            ApiError::corrupted(format!(
                "Gateway scenery response for {} is missing expected fields",
                scenery_id
            ))
        })?;

    let archive_bytes = decode_master_zip_blob(&scenery_detail)?;
    let temp_dir = tempdir().map_err(ApiError::from)?;
    let temp_archive_path = temp_dir
        .path()
        .join(format!("gateway_{}_{}.zip", airport_icao, scenery_id));
    fs::write(&temp_archive_path, archive_bytes).map_err(ApiError::from)?;

    livery_patterns::ensure_patterns_loaded().await;

    let xplane_path_for_analysis = xplane_root.to_string_lossy().to_string();
    let temp_archive_for_analysis = temp_archive_path.to_string_lossy().to_string();
    let analysis_result = tokio::task::spawn_blocking(move || {
        let analyzer = Analyzer::new();
        analyzer.analyze(
            vec![temp_archive_for_analysis],
            &xplane_path_for_analysis,
            None,
            None,
        )
    })
    .await
    .map_err(|error| ApiError::internal(format!("Gateway analysis task failed: {}", error)))?;

    let mut tasks = analysis_result.tasks;
    let mut task = extract_gateway_install_task(&mut tasks, &analysis_result.errors, &airport_icao)?;
    let folder_name = extract_folder_name_from_task(&task)?;
    let target_path = PathBuf::from(&task.target_path);

    if target_path.exists()
        && installed_before
            .as_ref()
            .map(|record| !record.folder_name.eq_ignore_ascii_case(&folder_name))
            .unwrap_or(true)
    {
        return Err(ApiError::conflict(format!(
            "Gateway target folder already exists: {}",
            folder_name
        )));
    }

    if let Some(existing) = &installed_before {
        if existing.scenery_id == scenery_id && target_path.exists() {
            return Ok(model_to_installed_airport(existing.clone()));
        }

        if existing.folder_name.eq_ignore_ascii_case(&folder_name) {
            task.should_overwrite = true;
        }
    }

    task.id = format!("gateway-{}-{}", airport_icao, scenery_id);
    task.display_name = format!("Gateway {} #{}", airport_icao, scenery_id);

    let install_result = Installer::new(app_handle)
        .install(
            vec![task.clone()],
            false,
            xplane_root.to_string_lossy().to_string(),
            false,
            should_auto_sort,
            Vec::new(),
        )
        .await
        .map_err(|error| ApiError::internal(format!("Gateway installation failed: {}", error)))?;

    let task_result = install_result.task_results.first().cloned();
    if install_result.failed_tasks > 0 || task_result.as_ref().is_some_and(|result| !result.success) {
        let message = task_result
            .and_then(|result| result.error_message)
            .unwrap_or_else(|| "Gateway installation failed".to_string());
        activity::log_activity(
            &conn,
            if installed_before.is_some() { "update" } else { "install" },
            "gateway",
            &airport_icao,
            Some(message.clone()),
            false,
        )
        .await;
        return Err(ApiError::archive(message));
    }

    if let Some(existing) = &installed_before {
        if !existing.folder_name.eq_ignore_ascii_case(&folder_name) {
            if let Err(error) =
                remove_managed_scenery_folder(&conn, &xplane_root, &existing.folder_name).await
            {
                logger::log_error(
                    &format!(
                        "Installed new Gateway scenery for {} but failed to remove previous folder {}: {}",
                        airport_icao, existing.folder_name, error
                    ),
                    Some("gateway"),
                );
            }
        }
    }

    if !should_auto_sort {
        maybe_update_scenery_index_after_install(&conn, &xplane_root, &folder_name).await;
    }

    let installed_model =
        upsert_install_record(&conn, &xplane_key, &airport_detail, &scenery_detail, &folder_name)
            .await?;

    activity::log_activity(
        &conn,
        if installed_before.is_some() { "update" } else { "install" },
        "gateway",
        &airport_icao,
        Some(format!("scenery {} -> {}", scenery_id, folder_name)),
        true,
    )
    .await;

    Ok(model_to_installed_airport(installed_model))
}

#[tauri::command]
pub async fn gateway_install_scenery(
    app_handle: AppHandle,
    db: State<'_, DatabaseState>,
    request: Option<GatewayInstallRequest>,
    xplane_path: Option<String>,
    icao: Option<String>,
    scenery_id: Option<i64>,
    auto_sort_scenery: Option<bool>,
    ignore_external_conflict: Option<bool>,
) -> ApiResult<GatewayInstalledAirport> {
    let request = request.unwrap_or(GatewayInstallRequest {
        xplane_path: xplane_path
            .ok_or_else(|| ApiError::validation("xplanePath is required"))?,
        icao: icao.ok_or_else(|| ApiError::validation("icao is required"))?,
        scenery_id: scenery_id.ok_or_else(|| ApiError::validation("sceneryId is required"))?,
        auto_sort_scenery,
        ignore_external_conflict,
    });

    gateway_install_scenery_impl(app_handle, db.get(), request).await
}

#[tauri::command]
pub async fn gateway_force_install_scenery(
    app_handle: AppHandle,
    db: State<'_, DatabaseState>,
    xplane_path: String,
    icao: String,
    scenery_id: i64,
    auto_sort_scenery: Option<bool>,
) -> ApiResult<GatewayInstalledAirport> {
    gateway_install_scenery_impl(
        app_handle,
        db.get(),
        GatewayInstallRequest {
            xplane_path,
            icao,
            scenery_id,
            auto_sort_scenery,
            ignore_external_conflict: Some(true),
        },
    )
    .await
}

#[tauri::command]
pub async fn gateway_uninstall_airport(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    airport_icao: String,
) -> ApiResult<()> {
    let xplane_root = validate_xplane_root(&xplane_path)?;
    let xplane_key = normalize_xplane_key(&xplane_root);
    let airport_icao = normalize_icao(&airport_icao)?;
    let conn = db.get();

    let Some(record) = find_install_by_airport(&conn, &xplane_key, &airport_icao).await? else {
        return Err(ApiError::not_found(format!(
            "No Gateway install record found for {}",
            airport_icao
        )));
    };

    remove_managed_scenery_folder(&conn, &xplane_root, &record.folder_name).await?;
    gateway_installs::Entity::delete_by_id(record.id)
        .exec(&conn)
        .await
        .map_err(ApiError::from)?;

    activity::log_activity(
        &conn,
        "uninstall",
        "gateway",
        &airport_icao,
        Some(format!("removed {}", record.folder_name)),
        true,
    )
    .await;

    Ok(())
}

fn parse_gateway_airport_summary(
    payload: &Value,
    fallback_icao: &str,
) -> Option<GatewayAirportSummaryData> {
    let root = payload.as_object()?;
    let airport = pick_gateway_airport_object(root);
    let scenery_list = pick_gateway_scenery_list(root, airport);
    let metadata = pick_gateway_metadata(root, airport);
    let airport_code = pick_gateway_airport_code(airport)
    .or_else(|| pick_gateway_airport_code(root))
    .unwrap_or_else(|| fallback_icao.to_string())
    .trim()
    .to_ascii_uppercase();

    if airport_code.is_empty() {
        return None;
    }

    let airport_name = pick_gateway_airport_name(airport).or_else(|| pick_gateway_airport_name(root));

    let root_recommended = pick_object(root, &["recommendedScenery", "RecommendedScenery"]);
    let airport_recommended = pick_object(airport, &["recommendedScenery", "RecommendedScenery"]);

    let recommended_scenery_id = normalize_gateway_id(pick_i64(
        airport,
        &[
            "RecommendedSceneryId",
            "recommendedSceneryId",
            "recommended_scenery_id",
        ],
    )
    .or_else(|| {
        pick_i64(
            root,
            &[
                "RecommendedSceneryId",
                "recommendedSceneryId",
                "recommended_scenery_id",
            ],
        )
    })
    .or_else(|| {
        root_recommended.and_then(|value| pick_i64(value, &["id", "sceneryId", "SceneryId"]))
    })
    .or_else(|| {
        airport_recommended.and_then(|value| pick_i64(value, &["id", "sceneryId", "SceneryId"]))
    }));

    let recommended = recommended_scenery_id.and_then(|target_id| {
        scenery_list.iter().find_map(|entry| {
            let record = entry.as_object()?;
            let entry_scenery_id = normalize_gateway_id(pick_i64(
                record,
                &["sceneryId", "SceneryId", "id"],
            ))?;
            (entry_scenery_id == target_id).then_some(record)
        })
    });

    let scenery_count = pick_i64(
        airport,
        &[
            "SubmissionCount",
            "ApprovedSceneryCount",
            "AcceptedSceneryCount",
            "sceneryCount",
            "SceneryCount",
            "totalSceneries",
        ],
    )
        .or_else(|| {
            pick_i64(
                root,
                &[
                    "SubmissionCount",
                    "ApprovedSceneryCount",
                    "AcceptedSceneryCount",
                    "sceneryCount",
                    "SceneryCount",
                    "totalSceneries",
                ],
            )
        })
        .or_else(|| (!scenery_list.is_empty()).then_some(scenery_list.len() as i64));

    let recommended_artist = recommended
        .and_then(pick_gateway_artist)
        .or_else(|| metadata.and_then(pick_gateway_artist));

    let recommended_accepted_at = recommended
        .and_then(pick_gateway_summary_date)
        .or_else(|| metadata.and_then(pick_gateway_summary_date));

    if recommended_scenery_id.is_none()
        && scenery_count.is_none()
        && airport_name.is_none()
        && recommended_artist.is_none()
        && recommended_accepted_at.is_none()
    {
        return None;
    }

    Some(GatewayAirportSummaryData {
        icao: airport_code,
        airport_name,
        scenery_count,
        recommended_scenery_id,
        recommended_artist,
        recommended_accepted_at,
    })
}

fn parse_gateway_airport_detail(payload: &Value, fallback_icao: &str) -> Option<GatewayAirportDetail> {
    let summary = parse_gateway_airport_summary(payload, fallback_icao)?;
    let root = payload.as_object()?;
    let airport = pick_gateway_airport_object(root);
    let scenery_list = pick_gateway_scenery_list(root, airport);

    let mut sceneries: Vec<GatewayScenerySummary> = scenery_list
        .iter()
        .filter_map(|entry| {
            let record = entry.as_object()?;
            let scenery_id = normalize_gateway_id(pick_i64(
                record,
                &["sceneryId", "SceneryId", "id"],
            ))?;

            Some(GatewayScenerySummary {
                scenery_id,
                artist: pick_gateway_artist(record),
                status: pick_gateway_status(record),
                approved_date: pick_gateway_approved_date(record),
                comment: pick_gateway_comment(record),
                recommended: summary
                    .recommended_scenery_id
                    .map(|value| value == scenery_id)
                    .unwrap_or(false),
            })
        })
        .collect();

    sceneries.sort_by(|a, b| {
        b.recommended
            .cmp(&a.recommended)
            .then_with(|| b.approved_date.cmp(&a.approved_date))
            .then_with(|| b.scenery_id.cmp(&a.scenery_id))
    });

    Some(GatewayAirportDetail {
        icao: summary.icao,
        airport_name: summary.airport_name,
        scenery_count: summary.scenery_count.or_else(|| Some(sceneries.len() as i64)),
        recommended_scenery_id: summary.recommended_scenery_id,
        recommended_artist: summary.recommended_artist,
        recommended_accepted_at: summary.recommended_accepted_at,
        sceneries,
    })
}

fn parse_gateway_scenery_detail(
    payload: &Value,
    fallback_scenery_id: i64,
) -> Option<GatewaySceneryInstallPayload> {
    let root = payload.as_object()?;
    let detail = pick_object(root, &["scenery", "Scenery", "data"]).unwrap_or(root);
    let airport =
        pick_object(detail, &["airport", "Airport"]).or_else(|| pick_object(root, &["airport", "Airport"]));
    let scenery_id = normalize_gateway_id(pick_i64(
        detail,
        &["sceneryId", "SceneryId", "id"],
    ))
    .unwrap_or(fallback_scenery_id);

    let artist = pick_gateway_artist(detail);
    let status = pick_gateway_status(detail);
    let approved_date = pick_gateway_approved_date(detail);
    let comment = pick_gateway_comment(detail);
    let features = parse_gateway_feature_labels(detail);

    let icao = airport
        .and_then(pick_gateway_airport_code)
        .or_else(|| pick_gateway_airport_code(detail))
        .or_else(|| pick_gateway_airport_code(root));
    let airport_name = airport
        .and_then(pick_gateway_airport_name)
        .or_else(|| pick_gateway_airport_name(detail))
        .or_else(|| pick_gateway_airport_name(root));
    let master_zip_blob = pick_string(
        detail,
        &["masterZipBlob", "MasterZipBlob", "master_zip_blob", "blob"],
    )
    .or_else(|| {
        pick_object(detail, &["sceneryFiles", "SceneryFiles"])
            .and_then(|files| pick_string(files, &["masterZipBlob", "MasterZipBlob"]))
    });

    if !status.is_some()
        && !artist.is_some()
        && !approved_date.is_some()
        && !comment.is_some()
        && features.is_empty()
        && scenery_id <= 0
    {
        return None;
    }

    Some(GatewaySceneryInstallPayload {
        scenery_id,
        icao,
        airport_name,
        status,
        artist,
        approved_date,
        comment,
        features,
        master_zip_blob,
    })
}

fn sanitize_scenery_detail(detail: GatewaySceneryInstallPayload) -> GatewaySceneryDetail {
    GatewaySceneryDetail {
        scenery_id: detail.scenery_id,
        icao: detail.icao,
        airport_name: detail.airport_name,
        status: detail.status,
        artist: detail.artist,
        approved_date: detail.approved_date,
        comment: detail.comment,
        features: detail.features,
    }
}

fn decode_master_zip_blob(detail: &GatewaySceneryInstallPayload) -> ApiResult<Vec<u8>> {
    let blob = detail
        .master_zip_blob
        .as_deref()
        .ok_or_else(|| ApiError::corrupted("Gateway scenery is missing masterZipBlob"))?;

    let compact_blob: String = blob.chars().filter(|value| !value.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(compact_blob.as_bytes())
        .map_err(|error| {
            ApiError::corrupted(format!("Failed to decode Gateway scenery archive: {}", error))
        })
}

fn extract_gateway_install_task(
    tasks: &mut Vec<InstallTask>,
    errors: &[String],
    airport_icao: &str,
) -> ApiResult<InstallTask> {
    if let Some(task) = tasks
        .drain(..)
        .find(|task| matches!(task.addon_type, AddonType::Scenery | AddonType::SceneryLibrary))
    {
        return Ok(task);
    }

    if !errors.is_empty() {
        return Err(ApiError::corrupted(errors.join("\n")));
    }

    Err(ApiError::corrupted(format!(
        "Gateway archive for {} did not produce an installable scenery task",
        airport_icao
    )))
}

fn extract_folder_name_from_task(task: &InstallTask) -> ApiResult<String> {
    Path::new(&task.target_path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
        .ok_or_else(|| ApiError::internal("Gateway install target folder is invalid"))
}

async fn find_install_by_airport(
    conn: &DatabaseConnection,
    xplane_key: &str,
    airport_icao: &str,
) -> ApiResult<Option<gateway_installs::Model>> {
    gateway_installs::Entity::find()
        .filter(gateway_installs::Column::XplanePath.eq(xplane_key))
        .filter(gateway_installs::Column::AirportIcao.eq(airport_icao))
        .one(conn)
        .await
        .map_err(ApiError::from)
}

async fn list_installed_internal(
    conn: &DatabaseConnection,
    xplane_root: &Path,
    xplane_key: &str,
) -> ApiResult<Vec<GatewayInstalledAirport>> {
    let models = gateway_installs::Entity::find()
        .filter(gateway_installs::Column::XplanePath.eq(xplane_key))
        .order_by_asc(gateway_installs::Column::AirportIcao)
        .all(conn)
        .await
        .map_err(ApiError::from)?;

    let custom_scenery_path = xplane_root.join("Custom Scenery");
    let mut stale_ids = Vec::new();
    let mut installed = Vec::new();

    for model in models {
        let folder_exists = custom_scenery_path.join(&model.folder_name).exists();
        if folder_exists {
            installed.push(model_to_installed_airport(model));
        } else {
            stale_ids.push(model.id);
        }
    }

    if !stale_ids.is_empty() {
        gateway_installs::Entity::delete_many()
            .filter(gateway_installs::Column::Id.is_in(stale_ids))
            .exec(conn)
            .await
            .map_err(ApiError::from)?;
    }

    Ok(installed)
}

#[tauri::command]
pub async fn gateway_check_install_warning(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    icao: String,
) -> ApiResult<Option<GatewayInstallWarning>> {
    let xplane_root = validate_xplane_root(&xplane_path)?;
    let airport_icao = normalize_icao(&icao)?;
    let conn = db.get();

    let warning = find_external_airport_conflict_message(&conn, &xplane_root, &airport_icao)
        .await?
        .map(|message| GatewayInstallWarning {
            kind: "external_airport_conflict".to_string(),
            message,
        });

    if let Some(warning) = &warning {
        logger::log_info(
            &format!(
                "Gateway install warning detected for {} (kind={})",
                airport_icao, warning.kind
            ),
            Some("gateway"),
        );
    }

    Ok(warning)
}

async fn upsert_install_record(
    conn: &DatabaseConnection,
    xplane_key: &str,
    airport_detail: &GatewayAirportDetail,
    scenery_detail: &GatewaySceneryInstallPayload,
    folder_name: &str,
) -> ApiResult<gateway_installs::Model> {
    let now = now_epoch_seconds();
    let airport_name = airport_detail
        .airport_name
        .clone()
        .or_else(|| scenery_detail.airport_name.clone())
        .unwrap_or_else(|| airport_detail.icao.clone());

    if let Some(existing) = find_install_by_airport(conn, xplane_key, &airport_detail.icao).await? {
        let mut active: gateway_installs::ActiveModel = existing.clone().into();
        active.airport_name = Set(airport_name);
        active.scenery_id = Set(scenery_detail.scenery_id);
        active.folder_name = Set(folder_name.to_string());
        active.artist = Set(scenery_detail.artist.clone());
        active.approved_date = Set(scenery_detail.approved_date.clone());
        active.installed_at = Set(now);
        return active.update(conn).await.map_err(ApiError::from);
    }

    gateway_installs::ActiveModel {
        xplane_path: Set(xplane_key.to_string()),
        airport_icao: Set(airport_detail.icao.clone()),
        airport_name: Set(airport_name),
        scenery_id: Set(scenery_detail.scenery_id),
        folder_name: Set(folder_name.to_string()),
        artist: Set(scenery_detail.artist.clone()),
        approved_date: Set(scenery_detail.approved_date.clone()),
        installed_at: Set(now),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(ApiError::from)
}

fn model_to_installed_airport(model: gateway_installs::Model) -> GatewayInstalledAirport {
    GatewayInstalledAirport {
        id: model.id,
        airport_icao: model.airport_icao,
        airport_name: model.airport_name,
        scenery_id: model.scenery_id,
        folder_name: model.folder_name,
        artist: model.artist,
        approved_date: model.approved_date,
        installed_at: model.installed_at,
        update_available: None,
        latest_scenery_id: None,
        latest_artist: None,
        latest_approved_date: None,
    }
}

async fn ensure_no_external_airport_conflict(
    conn: &DatabaseConnection,
    xplane_root: &Path,
    airport_icao: &str,
) -> ApiResult<()> {
    if let Some(message) = find_external_airport_conflict_message(conn, xplane_root, airport_icao).await? {
        return Err(ApiError::with_details(
            ApiErrorCode::ConflictExists,
            message,
            EXTERNAL_AIRPORT_CONFLICT_DETAIL,
        ));
    }

    Ok(())
}

async fn find_external_airport_conflict_message(
    conn: &DatabaseConnection,
    xplane_root: &Path,
    airport_icao: &str,
) -> ApiResult<Option<String>> {
    if let Some(folder_name) =
        find_conflicting_airport_from_index(conn, xplane_root, airport_icao).await?
    {
        return Ok(Some(format!(
            "Custom Scenery already contains a non-Gateway airport for {}: {}",
            airport_icao, folder_name
        )));
    }

    if let Some(folder_name) =
        find_conflicting_airport_from_folder_scan(conn, xplane_root, airport_icao).await?
    {
        return Ok(Some(format!(
            "Custom Scenery already contains a non-Gateway airport for {}: {}",
            airport_icao, folder_name
        )));
    }

    Ok(None)
}

async fn find_conflicting_airport_from_index(
    conn: &DatabaseConnection,
    xplane_root: &Path,
    airport_icao: &str,
) -> ApiResult<Option<String>> {
    let xplane_key = normalize_xplane_key(xplane_root);
    let managed_folders: HashSet<String> = gateway_installs::Entity::find()
        .filter(gateway_installs::Column::XplanePath.eq(xplane_key))
        .all(conn)
        .await
        .map_err(ApiError::from)?
        .into_iter()
        .map(|record| record.folder_name.to_ascii_lowercase())
        .collect();

    let index_manager = SceneryIndexManager::new(xplane_root, conn.clone());
    if !index_manager.has_index().await.unwrap_or(false) {
        return Ok(None);
    }

    let index = index_manager
        .load_index()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(index.packages.values().find_map(|package| {
        let airport_id = package.airport_id.as_deref()?;
        if !airport_id.eq_ignore_ascii_case(airport_icao) {
            return None;
        }
        if package.category == crate::models::SceneryCategory::DefaultAirport {
            return None;
        }
        if managed_folders.contains(&package.folder_name.to_ascii_lowercase()) {
            return None;
        }
        Some(package.folder_name.clone())
    }))
}

async fn find_conflicting_airport_from_folder_scan(
    conn: &DatabaseConnection,
    xplane_root: &Path,
    airport_icao: &str,
) -> ApiResult<Option<String>> {
    let xplane_key = normalize_xplane_key(xplane_root);
    let managed_folders: HashSet<String> = gateway_installs::Entity::find()
        .filter(gateway_installs::Column::XplanePath.eq(xplane_key))
        .all(conn)
        .await
        .map_err(ApiError::from)?
        .into_iter()
        .map(|record| record.folder_name.to_ascii_lowercase())
        .collect();

    let custom_scenery_path = xplane_root.join("Custom Scenery");
    if !custom_scenery_path.exists() {
        return Ok(None);
    }

    for entry in fs::read_dir(&custom_scenery_path).map_err(ApiError::from)? {
        let entry = entry.map_err(ApiError::from)?;
        let path = entry.path();
        let folder_name = entry.file_name().to_string_lossy().to_string();
        let folder_key = folder_name.to_ascii_lowercase();
        if managed_folders.contains(&folder_key) || !path.is_dir() || folder_key.contains("global airports") {
            continue;
        }

        let info = match classify_scenery(&path, xplane_root) {
            Ok(info) => info,
            Err(_) => continue,
        };
        if info.category == crate::models::SceneryCategory::DefaultAirport {
            continue;
        }
        if info
            .airport_id
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case(airport_icao))
            .unwrap_or(false)
        {
            return Ok(Some(folder_name));
        }
    }

    Ok(None)
}

async fn maybe_update_scenery_index_after_install(
    conn: &DatabaseConnection,
    xplane_root: &Path,
    folder_name: &str,
) {
    let index_manager = SceneryIndexManager::new(xplane_root, conn.clone());
    if !index_manager.has_index().await.unwrap_or(false) {
        return;
    }

    let folder_path = xplane_root.join("Custom Scenery").join(folder_name);
    if let Err(error) = index_manager.get_or_classify(&folder_path).await {
        logger::log_error(
            &format!(
                "Failed to update scenery index after Gateway install for {}: {}",
                folder_name, error
            ),
            Some("gateway"),
        );
    }
}

async fn remove_managed_scenery_folder(
    conn: &DatabaseConnection,
    xplane_root: &Path,
    folder_name: &str,
) -> ApiResult<()> {
    let custom_scenery_path = xplane_root.join("Custom Scenery");
    let entry_path = custom_scenery_path.join(folder_name);

    if entry_path.exists() {
        let metadata = fs::symlink_metadata(&entry_path).map_err(ApiError::from)?;
        if metadata.file_type().is_symlink() {
            fs::remove_file(&entry_path)
                .or_else(|_| fs::remove_dir(&entry_path))
                .map_err(ApiError::from)?;
        } else if metadata.is_file() {
            fs::remove_file(&entry_path).map_err(ApiError::from)?;
        } else {
            let canonical_path =
                path_utils::validate_child_path(&custom_scenery_path, &entry_path).map_err(|error| {
                    ApiError::validation(format!("Invalid scenery path: {}", error))
                })?;
            fs::remove_dir_all(&canonical_path).map_err(ApiError::from)?;
        }
    }

    if let Err(error) =
        crate::scenery_index::remove_scenery_entry(conn, &xplane_root.to_string_lossy(), folder_name)
            .await
    {
        logger::log_error(
            &format!(
                "Failed to remove Gateway scenery {} from index: {}",
                folder_name, error
            ),
            Some("gateway"),
        );
    }

    let packs_manager = SceneryPacksManager::new(xplane_root, conn.clone());
    if let Err(error) = packs_manager.apply_from_index().await {
        logger::log_error(
            &format!(
                "Failed to update scenery_packs.ini after removing Gateway scenery {}: {}",
                folder_name, error
            ),
            Some("gateway"),
        );
    }

    Ok(())
}

fn summary_to_search_result(summary: GatewayAirportSummaryData) -> GatewayAirportSearchResult {
    GatewayAirportSearchResult {
        icao: summary.icao,
        airport_name: summary.airport_name,
        scenery_count: summary.scenery_count,
        recommended_scenery_id: summary.recommended_scenery_id,
        recommended_artist: summary.recommended_artist,
        recommended_accepted_at: summary.recommended_accepted_at,
    }
}

fn pick_gateway_airport_object<'a>(root: &'a Map<String, Value>) -> &'a Map<String, Value> {
    pick_object(root, &["airport", "Airport", "data"]).unwrap_or(root)
}

fn pick_gateway_scenery_list(root: &Map<String, Value>, airport: &Map<String, Value>) -> Vec<Value> {
    pick_array(root, &["scenery", "Sceneries", "sceneries", "results", "items"])
        .or_else(|| pick_array(airport, &["scenery", "Sceneries", "sceneries", "results", "items"]))
        .cloned()
        .unwrap_or_default()
}

fn pick_gateway_metadata<'a>(
    root: &'a Map<String, Value>,
    airport: &'a Map<String, Value>,
) -> Option<&'a Map<String, Value>> {
    pick_object(root, &["metadata", "Metadata"])
        .or_else(|| pick_object(airport, &["metadata", "Metadata"]))
}

fn normalize_gateway_id(value: Option<i64>) -> Option<i64> {
    value.filter(|id| *id > 0)
}

fn pick_gateway_airport_code(record: &Map<String, Value>) -> Option<String> {
    pick_string(
        record,
        &["AirportCode", "airportCode", "icao", "ICAO", "code", "ident"],
    )
}

fn pick_gateway_airport_name(record: &Map<String, Value>) -> Option<String> {
    pick_string(
        record,
        &["AirportName", "airportName", "aptName", "name", "Name"],
    )
}

fn pick_gateway_artist(record: &Map<String, Value>) -> Option<String> {
    pick_string(
        record,
        &[
            "userName",
            "username",
            "artist",
            "artistName",
            "author",
            "authorName",
            "submittedBy",
        ],
    )
    .or_else(|| {
        pick_object(record, &["user", "User"])
            .and_then(|user| pick_string(user, &["name", "username", "displayName", "userName"]))
    })
}

fn pick_gateway_summary_date(record: &Map<String, Value>) -> Option<String> {
    pick_string(
        record,
        &[
            "dateAccepted",
            "dateApproved",
            "acceptedAt",
            "approvedDate",
            "approvalDate",
            "approvedAt",
            "date",
            "updatedAt",
        ],
    )
}

fn pick_gateway_approved_date(record: &Map<String, Value>) -> Option<String> {
    pick_string(
        record,
        &[
            "dateApproved",
            "dateAccepted",
            "approvedDate",
            "approvalDate",
            "acceptedAt",
            "updatedAt",
        ],
    )
}

fn pick_gateway_status(record: &Map<String, Value>) -> Option<String> {
    pick_string(
        record,
        &[
            "status",
            "gatewayStatus",
            "submissionStatus",
            "approvalStatus",
            "state",
        ],
    )
    .or_else(|| {
        if pick_string(record, &["dateDeclined"]).is_some() {
            Some("Declined".to_string())
        } else if pick_string(record, &["dateApproved"]).is_some() {
            Some("Approved".to_string())
        } else if pick_string(record, &["dateAccepted"]).is_some() {
            Some("Accepted".to_string())
        } else {
            None
        }
    })
}

fn pick_gateway_comment(record: &Map<String, Value>) -> Option<String> {
    let mut comments = Vec::new();
    if let Some(comment) = pick_string(record, &["artistComments"]) {
        comments.push(comment);
    }
    if let Some(comment) = pick_string(record, &["moderatorComments"]) {
        comments.push(comment);
    }
    if comments.is_empty() {
        if let Some(comment) = pick_string(
            record,
            &["comments", "comment", "description", "notes"],
        ) {
            comments.push(comment);
        }
    }
    comments.dedup();
    (!comments.is_empty()).then(|| comments.join("\n\n"))
}

fn parse_gateway_feature_labels(record: &Map<String, Value>) -> Vec<String> {
    let mut labels = Vec::new();

    if let Some(airport_type) = pick_string(record, &["type", "Type"]) {
        labels.push(airport_type);
    }

    if let Some(raw_features) = pick_string(record, &["features", "featureFlags"]) {
        for token in raw_features.split(',').map(str::trim).filter(|token| !token.is_empty()) {
            if token.chars().all(|ch| ch.is_ascii_digit()) {
                let feature_id = token.parse::<i64>().ok();
                labels.push(
                    feature_id
                        .and_then(gateway_feature_name)
                        .unwrap_or_else(|| format!("Feature {}", token)),
                );
            } else {
                labels.push(token.to_string());
            }
        }
    }

    let runway_count =
        pick_i64(record, &["runwayCount", "runwaysCount"]).or_else(|| pick_array_len(record, &["runways", "Runways"]));
    let gate_count = pick_i64(record, &["gateCount", "gatesCount", "startupCount"])
        .or_else(|| pick_array_len(record, &["gates", "startupLocations", "ramps"]));
    let taxiway_count =
        pick_i64(record, &["taxiwayCount", "taxiwaysCount"]).or_else(|| {
            pick_array_len(record, &["taxiways", "taxiwayEdges"])
        });

    if let Some(runway_count) = runway_count {
        labels.push(format!("RWY {}", runway_count));
    }
    if let Some(gate_count) = gate_count {
        labels.push(format!("Gates {}", gate_count));
    }
    if let Some(taxiway_count) = taxiway_count {
        labels.push(format!("Taxiway {}", taxiway_count));
    }

    if let Some(tags) = pick_array(record, &["tags"]) {
        for tag in tags.iter().take(5) {
            if let Some(tag) = tag.as_str().map(str::trim).filter(|text| !text.is_empty()) {
                labels.push(tag.to_string());
            }
        }
    }

    let mut deduped = Vec::new();
    for label in labels {
        if !deduped.contains(&label) {
            deduped.push(label);
        }
    }
    deduped
}

fn gateway_feature_name(feature_id: i64) -> Option<String> {
    let name = match feature_id {
        1 => "Has ATC Flow",
        2 => "Has Taxi Route",
        5 => "Has Log.txt Issue",
        6 => "LR Internal Use",
        8 => "Has Ground Routes",
        11 => "Runway Numbering/Length Fix",
        18 => "Runway Numbering Fix",
        20 => "Floating Runway",
        29 => "Ground Routes Certified",
        35 => "Misused Draped Sign Polygons",
        38 => "Runway in Water",
        40 => "Runway Unusable (XP11)",
        42 => "Low Res Terrain Polygons (XP11)",
        43 => "Fix Fragmented Road Network XP11",
        47 => "Overlap - Wrong location",
        51 => "Boat Injection",
        52 => "Tunnel Injection",
        55 => "Parking Lot Injection",
        57 => "Embankment Injection",
        58 => "Pier Injection",
        59 => "Runway is Stepped (XP11)",
        62 => "Custom Runway Markings",
        64 => "Runway Misaligned",
        65 => "Facade Injection",
        67 => "Ground Markings Injection",
        70 => "Jetway Kit Injection",
        71 => "Challenged by Artist",
        75 => "Structure(s) do not match imagery",
        78 => "Has orphaned taxiway",
        79 => "Misused Terrain Polygons",
        80 => "Road network duplication",
        83 => "XP12 pre-opening",
        84 => "Better than a newer submission.",
        86 => "Has Road Network",
        87 => "Temporary Terrain Polygon(s)",
        88 => "Runway Unusable (XP12)",
        89 => "Runway is Stepped (XP12)",
        90 => "Roads made with polygons",
        92 => "Review in Sim",
        93 => "Fix Fragmented Road Network XP12",
        94 => "Oversized Terrain Polygon(s)",
        95 => "Contains Flatten Polygon",
        _ => return None,
    };
    Some(name.to_string())
}

fn pick_string(record: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        record.get(*key).and_then(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(|text| text.to_string())
        })
    })
}

fn pick_i64(record: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        let value = record.get(*key)?;
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
            .or_else(|| value.as_f64().map(|number| number as i64))
            .or_else(|| {
                value
                    .as_str()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .and_then(|text| text.parse::<i64>().ok())
            })
    })
}

fn pick_object<'a>(record: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Map<String, Value>> {
    keys.iter()
        .find_map(|key| record.get(*key).and_then(Value::as_object))
}

fn pick_array<'a>(record: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Vec<Value>> {
    keys.iter()
        .find_map(|key| record.get(*key).and_then(Value::as_array))
}

fn pick_array_len(record: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
    pick_array(record, keys).map(|items| items.len() as i64)
}

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use futures::stream::{self, StreamExt};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use semver::Version;
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
const RELEASE_DIRECTORY_CACHE_TTL: Duration = Duration::from_secs(30 * 60);
const RELEASE_SCENERY_CACHE_TTL: Duration = Duration::from_secs(30 * 60);
const UPDATE_CHECK_CONCURRENCY: usize = 4;
const EXTERNAL_AIRPORT_CONFLICT_DETAIL: &str = "gateway_external_airport_conflict";
const GATEWAY_REQUEST_ATTEMPTS: usize = 3;

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
static RELEASE_DIRECTORY_CACHE: LazyLock<RwLock<Option<GatewayReleaseDirectoryCache>>> =
    LazyLock::new(|| RwLock::new(None));
static RELEASE_SCENERY_CACHE: LazyLock<RwLock<HashMap<String, GatewayReleaseSceneryCache>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone)]
struct GatewayAirportDirectoryCache {
    fetched_at: SystemTime,
    airports: Vec<GatewayAirportSearchResult>,
}

#[derive(Debug, Clone)]
struct GatewayReleaseDirectoryCache {
    fetched_at: SystemTime,
    releases: Vec<GatewayReleaseInfo>,
}

#[derive(Debug, Clone)]
struct GatewayReleaseSceneryCache {
    fetched_at: SystemTime,
    scenery_packs: HashSet<i64>,
}

#[derive(Debug, Clone)]
struct GatewayReleaseInfo {
    version: String,
    date: String,
    parsed_version: Version,
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

#[derive(Debug, Clone)]
struct GatewayCurrentReleaseSceneryData {
    scenery_id: Option<i64>,
    artist: Option<String>,
    approved_date: Option<String>,
}

#[derive(Debug, Clone)]
struct GatewayReleaseComparison {
    ahead_of_current_xplane: bool,
    current_xplane_scenery: GatewayCurrentReleaseSceneryData,
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
pub struct GatewayReleaseContext {
    pub detected_version_raw: Option<String>,
    pub matched_release_version: Option<String>,
    pub matched_release_date: Option<String>,
    pub comparison_available: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ahead_of_current_xplane: Option<bool>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ahead_of_current_xplane: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_release_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_scenery_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_artist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_approved_date: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ahead_of_current_xplane: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_release_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_scenery_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_artist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_xplane_approved_date: Option<String>,
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
    release_version: Option<String>,
) -> ApiResult<Vec<GatewayAirportSearchResult>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let directory = fetch_airport_directory().await?;
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let query_lower = query.to_ascii_lowercase();
    let release_scenery = fetch_release_scenery_set(release_version.as_deref()).await;

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
        .map(|(_, airport)| enrich_search_result_for_release(airport, release_scenery.as_ref()))
        .collect())
}

#[tauri::command]
pub async fn gateway_get_airport(
    icao: String,
    release_version: Option<String>,
) -> ApiResult<GatewayAirportDetail> {
    let icao = normalize_icao(&icao)?;
    let payload = fetch_gateway_airport_payload(&icao).await?;
    let detail = parse_gateway_airport_detail(&payload, &icao).ok_or_else(|| {
        ApiError::corrupted(format!(
            "Gateway airport response for {} is missing expected fields",
            icao
        ))
    })?;
    let release_info = fetch_release_info(release_version.as_deref()).await;
    let release_scenery = fetch_release_scenery_set(release_version.as_deref()).await;
    Ok(enrich_airport_detail_for_release(
        detail,
        release_info.as_ref(),
        release_scenery.as_ref(),
    ))
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
    _release_version: Option<String>,
) -> ApiResult<Vec<GatewayInstalledAirport>> {
    let xplane_root = validate_xplane_root(&xplane_path)?;
    let xplane_key = normalize_xplane_key(&xplane_root);
    list_installed_internal(&db.get(), &xplane_root, &xplane_key).await
}

#[tauri::command]
pub async fn gateway_check_updates(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    release_version: Option<String>,
) -> ApiResult<Vec<GatewayInstalledAirport>> {
    let xplane_root = validate_xplane_root(&xplane_path)?;
    let xplane_key = normalize_xplane_key(&xplane_root);
    let release_info = fetch_release_info(release_version.as_deref()).await;
    let release_scenery =
        std::sync::Arc::new(fetch_release_scenery_set(release_version.as_deref()).await);
    let installed = list_installed_internal(&db.get(), &xplane_root, &xplane_key).await?;
    if installed.is_empty() {
        return Ok(installed);
    }

    let results: Vec<(GatewayInstalledAirport, bool)> = stream::iter(installed)
        .map(|installed| {
            let release_info = release_info.clone();
            let release_scenery = release_scenery.clone();
            async move {
                match fetch_gateway_airport_payload(&installed.airport_icao).await {
                    Ok(payload) => {
                        let summary =
                            parse_gateway_airport_summary(&payload, &installed.airport_icao);
                        let detail =
                            parse_gateway_airport_detail(&payload, &installed.airport_icao);
                        let mut next = installed.clone();
                        if let Some(summary) = summary {
                            next.latest_scenery_id = summary.recommended_scenery_id;
                            next.latest_artist = summary.recommended_artist;
                            next.latest_approved_date = summary.recommended_accepted_at;
                            next.update_available = summary
                                .recommended_scenery_id
                                .map(|latest| latest != next.scenery_id);
                            if let Some(detail) = detail {
                                let comparison = compute_release_comparison(
                                    &detail,
                                    release_scenery.as_ref().as_ref(),
                                );
                                apply_release_comparison_to_installed(
                                    &mut next,
                                    release_info.as_ref(),
                                    comparison.as_ref(),
                                );
                            }
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

    Ok(results
        .into_iter()
        .map(|(installed, _)| installed)
        .collect())
}

#[tauri::command]
pub async fn gateway_resolve_release_context(
    xplane_path: String,
) -> ApiResult<GatewayReleaseContext> {
    let xplane_root = validate_xplane_root(&xplane_path)?;
    resolve_release_context(&xplane_root).await
}

async fn resolve_release_context(xplane_root: &Path) -> ApiResult<GatewayReleaseContext> {
    let detected_version_raw = detect_xplane_version(xplane_root);
    let Some(detected_version_raw) = detected_version_raw else {
        return Ok(GatewayReleaseContext {
            detected_version_raw: None,
            matched_release_version: None,
            matched_release_date: None,
            comparison_available: false,
        });
    };

    let Some(parsed_version) = parse_xplane_version_for_compare(&detected_version_raw) else {
        return Ok(GatewayReleaseContext {
            detected_version_raw: Some(detected_version_raw),
            matched_release_version: None,
            matched_release_date: None,
            comparison_available: false,
        });
    };

    let Some(releases) = fetch_release_directory().await else {
        return Ok(GatewayReleaseContext {
            detected_version_raw: Some(detected_version_raw),
            matched_release_version: None,
            matched_release_date: None,
            comparison_available: false,
        });
    };

    let Some(release) = find_matching_release(&parsed_version, &releases) else {
        return Ok(GatewayReleaseContext {
            detected_version_raw: Some(detected_version_raw),
            matched_release_version: None,
            matched_release_date: None,
            comparison_available: false,
        });
    };

    Ok(GatewayReleaseContext {
        detected_version_raw: Some(detected_version_raw),
        matched_release_version: Some(release.version.clone()),
        matched_release_date: Some(release.date.clone()),
        comparison_available: true,
    })
}

async fn fetch_release_directory() -> Option<Vec<GatewayReleaseInfo>> {
    if let Some(cached) = RELEASE_DIRECTORY_CACHE
        .read()
        .ok()
        .and_then(|cache| cache.clone())
    {
        if cached
            .fetched_at
            .elapsed()
            .unwrap_or_default()
            .lt(&RELEASE_DIRECTORY_CACHE_TTL)
        {
            return Some(cached.releases);
        }
    }

    let payload = match fetch_gateway_json(&format!("{}/releases", GATEWAY_API_BASE)).await {
        Ok(payload) => payload,
        Err(error) => {
            logger::log_error(
                &format!("Failed to fetch Gateway release directory: {}", error),
                Some("gateway"),
            );
            return None;
        }
    };

    let releases = parse_gateway_release_directory(&payload);
    if releases.is_empty() {
        logger::log_error(
            "Gateway release directory returned no usable versions",
            Some("gateway"),
        );
        return None;
    }

    if let Ok(mut cache) = RELEASE_DIRECTORY_CACHE.write() {
        *cache = Some(GatewayReleaseDirectoryCache {
            fetched_at: SystemTime::now(),
            releases: releases.clone(),
        });
    }

    Some(releases)
}

async fn fetch_release_info(version: Option<&str>) -> Option<GatewayReleaseInfo> {
    let version = version.map(str::trim).filter(|value| !value.is_empty())?;
    let releases = fetch_release_directory().await?;
    releases
        .into_iter()
        .find(|release| release.version == version)
}

async fn fetch_release_scenery_set(version: Option<&str>) -> Option<HashSet<i64>> {
    let version = version.map(str::trim).filter(|value| !value.is_empty())?;

    if let Some(cached) = RELEASE_SCENERY_CACHE
        .read()
        .ok()
        .and_then(|cache| cache.get(version).cloned())
    {
        if cached
            .fetched_at
            .elapsed()
            .unwrap_or_default()
            .lt(&RELEASE_SCENERY_CACHE_TTL)
        {
            return Some(cached.scenery_packs);
        }
    }

    let payload =
        match fetch_gateway_json(&format!("{}/release/{}", GATEWAY_API_BASE, version)).await {
            Ok(payload) => payload,
            Err(error) => {
                logger::log_error(
                    &format!(
                        "Failed to fetch Gateway release scenery packs for {}: {}",
                        version, error
                    ),
                    Some("gateway"),
                );
                return None;
            }
        };

    let scenery_packs = parse_release_scenery_set(&payload);
    if scenery_packs.is_empty() {
        logger::log_info(
            &format!(
                "Gateway release {} returned an empty scenery pack set or unrecognized shape",
                version
            ),
            Some("gateway"),
        );
    }

    if let Ok(mut cache) = RELEASE_SCENERY_CACHE.write() {
        cache.insert(
            version.to_string(),
            GatewayReleaseSceneryCache {
                fetched_at: SystemTime::now(),
                scenery_packs: scenery_packs.clone(),
            },
        );
    }

    Some(scenery_packs)
}

fn parse_gateway_release_directory(payload: &Value) -> Vec<GatewayReleaseInfo> {
    let Some(entries) = payload.as_array() else {
        return Vec::new();
    };

    let mut releases: Vec<GatewayReleaseInfo> = entries
        .iter()
        .filter_map(|entry| {
            let record = entry.as_object()?;
            let version = pick_string(record, &["Version", "version"])?
                .trim()
                .to_string();
            let date = pick_string(record, &["Date", "date"])?.trim().to_string();
            let parsed_version = parse_xplane_version_for_compare(&version)?;
            Some(GatewayReleaseInfo {
                version,
                date,
                parsed_version,
            })
        })
        .collect();

    releases.sort_by(|a, b| a.parsed_version.cmp(&b.parsed_version));
    releases
}

fn parse_release_scenery_set(payload: &Value) -> HashSet<i64> {
    let Some(root) = payload.as_object() else {
        return HashSet::new();
    };

    pick_array(root, &["SceneryPacks", "sceneryPacks"])
        .into_iter()
        .flatten()
        .filter_map(|value| {
            value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|id| i64::try_from(id).ok()))
        })
        .filter(|id| *id > 0)
        .collect()
}

fn find_matching_release<'a>(
    detected_version: &Version,
    releases: &'a [GatewayReleaseInfo],
) -> Option<&'a GatewayReleaseInfo> {
    releases
        .iter()
        .filter(|release| release.parsed_version <= *detected_version)
        .max_by(|left, right| left.parsed_version.cmp(&right.parsed_version))
}

fn parse_xplane_version_for_compare(raw: &str) -> Option<Version> {
    let normalized = normalize_xplane_version_token(raw)?;
    Version::parse(&normalized).ok()
}

fn normalize_xplane_version_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() {
        return None;
    }

    let first_token = trimmed.split_whitespace().next().unwrap_or(trimmed);
    let mut current_segment = String::new();
    let mut segments: Vec<String> = Vec::new();

    for ch in first_token.chars() {
        if ch.is_ascii_digit() {
            current_segment.push(ch);
            continue;
        }

        if ch == '.' {
            if current_segment.is_empty() {
                break;
            }
            segments.push(std::mem::take(&mut current_segment));
            if segments.len() >= 4 {
                break;
            }
            continue;
        }

        if !current_segment.is_empty() {
            break;
        }
    }

    if !current_segment.is_empty() {
        segments.push(current_segment);
    }

    if segments.is_empty() {
        return None;
    }

    let mut normalized: Vec<String> = segments
        .into_iter()
        .take(3)
        .filter_map(|segment| segment.parse::<u64>().ok().map(|value| value.to_string()))
        .collect();

    if normalized.is_empty() {
        return None;
    }

    while normalized.len() < 3 {
        normalized.push("0".to_string());
    }

    Some(normalized.join("."))
}

fn detect_xplane_version(xplane_root: &Path) -> Option<String> {
    #[cfg(windows)]
    if let Some(version) = detect_xplane_version_from_windows_binary(xplane_root) {
        return Some(version);
    }

    #[cfg(target_os = "macos")]
    if let Some(version) = detect_xplane_version_from_macos_bundle(xplane_root) {
        return Some(version);
    }

    detect_xplane_version_from_log(xplane_root)
}

fn detect_xplane_version_from_log(xplane_root: &Path) -> Option<String> {
    let log_path = xplane_root.join("Log.txt");
    let content = fs::read_to_string(log_path).ok()?;
    let first_line = content.lines().next()?.trim();

    first_line
        .find("Log.txt for X-Plane ")
        .map(|pos| {
            first_line[pos + "Log.txt for X-Plane ".len()..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .filter(|value| !value.is_empty())
        .or_else(|| {
            first_line
                .find("Log.txt for ")
                .map(|pos| {
                    first_line[pos + "Log.txt for ".len()..]
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string()
                })
                .filter(|value| !value.is_empty())
        })
}

#[cfg(target_os = "macos")]
fn detect_xplane_version_from_macos_bundle(xplane_root: &Path) -> Option<String> {
    let plist_path = xplane_root.join("X-Plane.app/Contents/Info.plist");
    let plist = plist::Value::from_file(&plist_path).ok()?;
    let dict = plist.as_dictionary()?;

    dict.get("CFBundleShortVersionString")
        .and_then(plist::Value::as_string)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            dict.get("CFBundleVersion")
                .and_then(plist::Value::as_string)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

#[cfg(windows)]
fn detect_xplane_version_from_windows_binary(xplane_root: &Path) -> Option<String> {
    let exe_path = xplane_root.join("X-Plane.exe");
    if !exe_path.exists() {
        return None;
    }

    read_windows_file_version(&exe_path)
}

#[cfg(windows)]
fn read_windows_file_version(path: &Path) -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;

    use winapi::shared::minwindef::{DWORD, LPVOID, UINT};
    use winapi::um::winver::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};

    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut handle: DWORD = 0;

    unsafe {
        let size = GetFileVersionInfoSizeW(path_wide.as_ptr(), &mut handle);
        if size == 0 {
            return None;
        }

        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(path_wide.as_ptr(), 0, size, data.as_mut_ptr() as LPVOID) == 0 {
            return None;
        }

        let translation_block: Vec<u16> = OsStr::new("\\VarFileInfo\\Translation")
            .encode_wide()
            .chain(Some(0))
            .collect();
        let mut translation_ptr: LPVOID = null_mut();
        let mut translation_len: UINT = 0;
        let (lang, codepage) = if VerQueryValueW(
            data.as_mut_ptr() as LPVOID,
            translation_block.as_ptr(),
            &mut translation_ptr,
            &mut translation_len,
        ) != 0
            && !translation_ptr.is_null()
            && translation_len >= 4
        {
            let translation = std::slice::from_raw_parts(
                translation_ptr as *const u16,
                translation_len as usize / 2,
            );
            (
                translation.first().copied().unwrap_or(0x0409),
                translation.get(1).copied().unwrap_or(0x04b0),
            )
        } else {
            (0x0409, 0x04b0)
        };

        let product_key = format!(
            "\\StringFileInfo\\{:04x}{:04x}\\ProductVersion",
            lang, codepage
        );
        if let Some(version) = query_windows_version_string(&mut data, &product_key) {
            return Some(version);
        }

        let file_key = format!(
            "\\StringFileInfo\\{:04x}{:04x}\\FileVersion",
            lang, codepage
        );
        if let Some(version) = query_windows_version_string(&mut data, &file_key) {
            return Some(version);
        }
        None
    }
}

#[cfg(windows)]
unsafe fn query_windows_version_string(data: &mut [u8], key: &str) -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use winapi::shared::minwindef::{LPVOID, UINT};
    use winapi::um::winver::VerQueryValueW;

    let key_wide: Vec<u16> = OsStr::new(key).encode_wide().chain(Some(0)).collect();
    let mut value_ptr: LPVOID = std::ptr::null_mut();
    let mut value_len: UINT = 0;

    if VerQueryValueW(
        data.as_mut_ptr() as LPVOID,
        key_wide.as_ptr(),
        &mut value_ptr,
        &mut value_len,
    ) == 0
        || value_ptr.is_null()
        || value_len == 0
    {
        return None;
    }

    let value_slice = std::slice::from_raw_parts(value_ptr as *const u16, value_len as usize);
    let end = value_slice
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(value_slice.len());
    let value = String::from_utf16_lossy(&value_slice[..end])
        .trim()
        .to_string();
    (!value.is_empty()).then_some(value)
}

fn enrich_search_result_for_release(
    mut airport: GatewayAirportSearchResult,
    release_scenery: Option<&HashSet<i64>>,
) -> GatewayAirportSearchResult {
    airport.ahead_of_current_xplane = release_scenery.map(|set| {
        airport
            .recommended_scenery_id
            .map(|recommended| !set.contains(&recommended))
            .unwrap_or(false)
    });
    airport
}

fn enrich_airport_detail_for_release(
    mut detail: GatewayAirportDetail,
    release_info: Option<&GatewayReleaseInfo>,
    release_scenery: Option<&HashSet<i64>>,
) -> GatewayAirportDetail {
    if let Some(release_info) = release_info {
        detail.current_xplane_release_version = Some(release_info.version.clone());
        detail.current_xplane_release_date = Some(release_info.date.clone());
    }

    if let Some(comparison) = compute_release_comparison(&detail, release_scenery) {
        detail.ahead_of_current_xplane = Some(comparison.ahead_of_current_xplane);
        detail.current_xplane_scenery_id = comparison.current_xplane_scenery.scenery_id;
        detail.current_xplane_artist = comparison.current_xplane_scenery.artist;
        detail.current_xplane_approved_date = comparison.current_xplane_scenery.approved_date;
    }

    detail
}

fn compute_release_comparison(
    detail: &GatewayAirportDetail,
    release_scenery: Option<&HashSet<i64>>,
) -> Option<GatewayReleaseComparison> {
    let release_scenery = release_scenery?;
    let current_xplane_scenery = resolve_current_release_scenery(detail, release_scenery);
    let ahead_of_current_xplane = detail
        .recommended_scenery_id
        .map(|recommended_id| {
            current_xplane_scenery
                .scenery_id
                .map(|current_id| current_id != recommended_id)
                .unwrap_or(true)
        })
        .unwrap_or(false);

    Some(GatewayReleaseComparison {
        ahead_of_current_xplane,
        current_xplane_scenery,
    })
}

fn resolve_current_release_scenery(
    detail: &GatewayAirportDetail,
    release_scenery: &HashSet<i64>,
) -> GatewayCurrentReleaseSceneryData {
    detail
        .sceneries
        .iter()
        .filter(|scenery| release_scenery.contains(&scenery.scenery_id))
        .max_by(|left, right| {
            left.approved_date
                .as_deref()
                .cmp(&right.approved_date.as_deref())
                .then_with(|| left.scenery_id.cmp(&right.scenery_id))
        })
        .map(|scenery| GatewayCurrentReleaseSceneryData {
            scenery_id: Some(scenery.scenery_id),
            artist: scenery.artist.clone(),
            approved_date: scenery.approved_date.clone(),
        })
        .unwrap_or(GatewayCurrentReleaseSceneryData {
            scenery_id: None,
            artist: None,
            approved_date: None,
        })
}

fn apply_release_comparison_to_installed(
    installed: &mut GatewayInstalledAirport,
    release_info: Option<&GatewayReleaseInfo>,
    comparison: Option<&GatewayReleaseComparison>,
) {
    if let Some(release_info) = release_info {
        installed.current_xplane_release_version = Some(release_info.version.clone());
        installed.current_xplane_release_date = Some(release_info.date.clone());
    }

    if let Some(comparison) = comparison {
        installed.ahead_of_current_xplane = Some(comparison.ahead_of_current_xplane);
        installed.current_xplane_scenery_id = comparison.current_xplane_scenery.scenery_id;
        installed.current_xplane_artist = comparison.current_xplane_scenery.artist.clone();
        installed.current_xplane_approved_date =
            comparison.current_xplane_scenery.approved_date.clone();
    }
}

fn validate_xplane_root(xplane_path: &str) -> ApiResult<PathBuf> {
    let trimmed = xplane_path.trim();
    if trimmed.is_empty() {
        return Err(ApiError::validation("X-Plane path is required"));
    }

    let path = PathBuf::from(trimmed);
    if !path.exists() {
        return Err(ApiError::not_found(
            "Configured X-Plane path does not exist",
        ));
    }
    if !path.is_dir() {
        return Err(ApiError::validation(
            "Configured X-Plane path must be a directory",
        ));
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
        return Err(ApiError::validation(
            "Scenery ID must be a positive integer",
        ));
    }
    fetch_gateway_json(&format!("{}/scenery/{}", GATEWAY_API_BASE, scenery_id)).await
}

async fn fetch_gateway_json(url: &str) -> ApiResult<Value> {
    let mut last_error: Option<ApiError> = None;

    for attempt in 0..GATEWAY_REQUEST_ATTEMPTS {
        let response = match GATEWAY_HTTP_CLIENT.get(url).send().await {
            Ok(response) => response,
            Err(error) => {
                let should_retry = gateway_transport_error_is_retryable(&error);
                let message = gateway_transport_error_message(&error);
                logger::log_info(
                    &format!(
                        "Gateway API request failed url={} attempt={}/{} retry={} error={}",
                        url,
                        attempt + 1,
                        GATEWAY_REQUEST_ATTEMPTS,
                        should_retry,
                        error
                    ),
                    Some("gateway"),
                );
                last_error = Some(ApiError::new(ApiErrorCode::NetworkError, message));
                if should_retry && attempt + 1 < GATEWAY_REQUEST_ATTEMPTS {
                    tokio::time::sleep(gateway_retry_delay(attempt)).await;
                    continue;
                }
                break;
            }
        };

        let status = response.status();
        if !status.is_success() {
            if gateway_status_is_retryable(status) {
                let message = gateway_status_error_message(status);
                logger::log_info(
                    &format!(
                        "Gateway API returned temporary status url={} attempt={}/{} status={}",
                        url,
                        attempt + 1,
                        GATEWAY_REQUEST_ATTEMPTS,
                        status
                    ),
                    Some("gateway"),
                );
                last_error = Some(ApiError::new(ApiErrorCode::NetworkError, message));
                if attempt + 1 < GATEWAY_REQUEST_ATTEMPTS {
                    tokio::time::sleep(gateway_retry_delay(attempt)).await;
                    continue;
                }
                break;
            }

            return Err(ApiError::new(
                ApiErrorCode::NetworkError,
                format!("Gateway API request failed with status {}", status),
            ));
        }

        return response.json::<Value>().await.map_err(|error| {
            ApiError::corrupted(format!("Failed to parse Gateway response: {}", error))
        });
    }

    Err(last_error.unwrap_or_else(|| {
        ApiError::new(
            ApiErrorCode::NetworkError,
            "Gateway service is temporarily unavailable. Please try again later.",
        )
    }))
}

fn gateway_retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(match attempt {
        0 => 350,
        1 => 900,
        _ => 1500,
    })
}

fn gateway_status_is_retryable(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 429 | 502 | 503 | 504)
}

fn gateway_status_error_message(status: reqwest::StatusCode) -> String {
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return "Gateway service is busy right now. Please wait a moment and try again."
            .to_string();
    }

    "Gateway service is temporarily unavailable. Please try again later.".to_string()
}

fn gateway_transport_error_is_retryable(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request()
}

fn gateway_transport_error_message(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return "Gateway service did not respond in time. Please try again later.".to_string();
    }

    "Gateway service is temporarily unavailable. Please try again later.".to_string()
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
    let mut task =
        extract_gateway_install_task(&mut tasks, &analysis_result.errors, &airport_icao)?;
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
    if install_result.failed_tasks > 0 || task_result.as_ref().is_some_and(|result| !result.success)
    {
        let message = task_result
            .and_then(|result| result.error_message)
            .unwrap_or_else(|| "Gateway installation failed".to_string());
        activity::log_activity(
            &conn,
            if installed_before.is_some() {
                "update"
            } else {
                "install"
            },
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

    let installed_model = upsert_install_record(
        &conn,
        &xplane_key,
        &airport_detail,
        &scenery_detail,
        &folder_name,
    )
    .await?;

    activity::log_activity(
        &conn,
        if installed_before.is_some() {
            "update"
        } else {
            "install"
        },
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
        xplane_path: xplane_path.ok_or_else(|| ApiError::validation("xplanePath is required"))?,
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

    let airport_name =
        pick_gateway_airport_name(airport).or_else(|| pick_gateway_airport_name(root));

    let root_recommended = pick_object(root, &["recommendedScenery", "RecommendedScenery"]);
    let airport_recommended = pick_object(airport, &["recommendedScenery", "RecommendedScenery"]);

    let recommended_scenery_id = normalize_gateway_id(
        pick_i64(
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
        }),
    );

    let recommended = recommended_scenery_id.and_then(|target_id| {
        scenery_list.iter().find_map(|entry| {
            let record = entry.as_object()?;
            let entry_scenery_id =
                normalize_gateway_id(pick_i64(record, &["sceneryId", "SceneryId", "id"]))?;
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

fn parse_gateway_airport_detail(
    payload: &Value,
    fallback_icao: &str,
) -> Option<GatewayAirportDetail> {
    let summary = parse_gateway_airport_summary(payload, fallback_icao)?;
    let root = payload.as_object()?;
    let airport = pick_gateway_airport_object(root);
    let scenery_list = pick_gateway_scenery_list(root, airport);

    let mut sceneries: Vec<GatewayScenerySummary> = scenery_list
        .iter()
        .filter_map(|entry| {
            let record = entry.as_object()?;
            let scenery_id =
                normalize_gateway_id(pick_i64(record, &["sceneryId", "SceneryId", "id"]))?;

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
        scenery_count: summary.scenery_count.or(Some(sceneries.len() as i64)),
        recommended_scenery_id: summary.recommended_scenery_id,
        recommended_artist: summary.recommended_artist,
        recommended_accepted_at: summary.recommended_accepted_at,
        sceneries,
        ahead_of_current_xplane: None,
        current_xplane_release_version: None,
        current_xplane_release_date: None,
        current_xplane_scenery_id: None,
        current_xplane_artist: None,
        current_xplane_approved_date: None,
    })
}

fn parse_gateway_scenery_detail(
    payload: &Value,
    fallback_scenery_id: i64,
) -> Option<GatewaySceneryInstallPayload> {
    let root = payload.as_object()?;
    let detail = pick_object(root, &["scenery", "Scenery", "data"]).unwrap_or(root);
    let airport = pick_object(detail, &["airport", "Airport"])
        .or_else(|| pick_object(root, &["airport", "Airport"]));
    let scenery_id = normalize_gateway_id(pick_i64(detail, &["sceneryId", "SceneryId", "id"]))
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

    let compact_blob: String = blob
        .chars()
        .filter(|value| !value.is_whitespace())
        .collect();
    base64::engine::general_purpose::STANDARD
        .decode(compact_blob.as_bytes())
        .map_err(|error| {
            ApiError::corrupted(format!(
                "Failed to decode Gateway scenery archive: {}",
                error
            ))
        })
}

fn extract_gateway_install_task(
    tasks: &mut Vec<InstallTask>,
    errors: &[String],
    airport_icao: &str,
) -> ApiResult<InstallTask> {
    if let Some(task) = tasks.drain(..).find(|task| {
        matches!(
            task.addon_type,
            AddonType::Scenery | AddonType::SceneryLibrary
        )
    }) {
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
        ahead_of_current_xplane: None,
        current_xplane_release_version: None,
        current_xplane_release_date: None,
        current_xplane_scenery_id: None,
        current_xplane_artist: None,
        current_xplane_approved_date: None,
    }
}

async fn ensure_no_external_airport_conflict(
    conn: &DatabaseConnection,
    xplane_root: &Path,
    airport_icao: &str,
) -> ApiResult<()> {
    if let Some(message) =
        find_external_airport_conflict_message(conn, xplane_root, airport_icao).await?
    {
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
        if managed_folders.contains(&folder_key)
            || !path.is_dir()
            || folder_key.contains("global airports")
        {
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
            let canonical_path = path_utils::validate_child_path(&custom_scenery_path, &entry_path)
                .map_err(|error| {
                    ApiError::validation(format!("Invalid scenery path: {}", error))
                })?;
            fs::remove_dir_all(&canonical_path).map_err(ApiError::from)?;
        }
    }

    if let Err(error) = crate::scenery_index::remove_scenery_entry(
        conn,
        &xplane_root.to_string_lossy(),
        folder_name,
    )
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
        ahead_of_current_xplane: None,
    }
}

fn pick_gateway_airport_object(root: &Map<String, Value>) -> &Map<String, Value> {
    pick_object(root, &["airport", "Airport", "data"]).unwrap_or(root)
}

fn pick_gateway_scenery_list(
    root: &Map<String, Value>,
    airport: &Map<String, Value>,
) -> Vec<Value> {
    pick_array(
        root,
        &["scenery", "Sceneries", "sceneries", "results", "items"],
    )
    .or_else(|| {
        pick_array(
            airport,
            &["scenery", "Sceneries", "sceneries", "results", "items"],
        )
    })
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
        &[
            "AirportCode",
            "airportCode",
            "icao",
            "ICAO",
            "code",
            "ident",
        ],
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
        if let Some(comment) = pick_string(record, &["comments", "comment", "description", "notes"])
        {
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
        for token in raw_features
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
        {
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

    let runway_count = pick_i64(record, &["runwayCount", "runwaysCount"])
        .or_else(|| pick_array_len(record, &["runways", "Runways"]));
    let gate_count = pick_i64(record, &["gateCount", "gatesCount", "startupCount"])
        .or_else(|| pick_array_len(record, &["gates", "startupLocations", "ramps"]));
    let taxiway_count = pick_i64(record, &["taxiwayCount", "taxiwaysCount"])
        .or_else(|| pick_array_len(record, &["taxiways", "taxiwayEdges"]));

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

fn pick_object<'a>(
    record: &'a Map<String, Value>,
    keys: &[&str],
) -> Option<&'a Map<String, Value>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gateway_detail(
        recommended_scenery_id: Option<i64>,
        sceneries: Vec<GatewayScenerySummary>,
    ) -> GatewayAirportDetail {
        GatewayAirportDetail {
            icao: "KSEA".to_string(),
            airport_name: Some("Seattle".to_string()),
            scenery_count: Some(sceneries.len() as i64),
            recommended_scenery_id,
            recommended_artist: None,
            recommended_accepted_at: None,
            sceneries,
            ahead_of_current_xplane: None,
            current_xplane_release_version: None,
            current_xplane_release_date: None,
            current_xplane_scenery_id: None,
            current_xplane_artist: None,
            current_xplane_approved_date: None,
        }
    }

    fn make_scenery(id: i64, approved_date: &str, recommended: bool) -> GatewayScenerySummary {
        GatewayScenerySummary {
            scenery_id: id,
            artist: Some(format!("Artist {}", id)),
            status: Some("Approved".to_string()),
            approved_date: Some(approved_date.to_string()),
            comment: None,
            recommended,
        }
    }

    #[test]
    fn normalizes_xplane_versions_for_release_matching() {
        assert_eq!(
            normalize_xplane_version_token("12.4.1-r1-52b26ed9").as_deref(),
            Some("12.4.1")
        );
        assert_eq!(
            normalize_xplane_version_token("12.00r6").as_deref(),
            Some("12.0.0")
        );
        assert_eq!(
            normalize_xplane_version_token("  v12.1.4 ").as_deref(),
            Some("12.1.4")
        );
    }

    #[test]
    fn release_comparison_marks_gateway_ahead_when_recommended_not_in_release() {
        let detail = make_gateway_detail(
            Some(200),
            vec![
                make_scenery(200, "2026-01-01T00:00:00.000Z", true),
                make_scenery(150, "2025-12-01T00:00:00.000Z", false),
            ],
        );
        let release_scenery = HashSet::from([150]);

        let comparison = compute_release_comparison(&detail, Some(&release_scenery))
            .expect("comparison should be available");

        assert!(comparison.ahead_of_current_xplane);
        assert_eq!(comparison.current_xplane_scenery.scenery_id, Some(150));
    }

    #[test]
    fn release_comparison_marks_up_to_date_when_recommended_in_release() {
        let detail = make_gateway_detail(
            Some(200),
            vec![
                make_scenery(200, "2026-01-01T00:00:00.000Z", true),
                make_scenery(150, "2025-12-01T00:00:00.000Z", false),
            ],
        );
        let release_scenery = HashSet::from([200]);

        let comparison = compute_release_comparison(&detail, Some(&release_scenery))
            .expect("comparison should be available");

        assert!(!comparison.ahead_of_current_xplane);
        assert_eq!(comparison.current_xplane_scenery.scenery_id, Some(200));
    }

    #[test]
    fn gateway_temporary_statuses_are_retryable_without_html_body() {
        assert!(gateway_status_is_retryable(
            reqwest::StatusCode::BAD_GATEWAY
        ));
        assert!(gateway_status_is_retryable(
            reqwest::StatusCode::SERVICE_UNAVAILABLE
        ));
        assert!(gateway_status_is_retryable(
            reqwest::StatusCode::GATEWAY_TIMEOUT
        ));
        assert!(gateway_status_is_retryable(
            reqwest::StatusCode::TOO_MANY_REQUESTS
        ));

        let message = gateway_status_error_message(reqwest::StatusCode::BAD_GATEWAY);
        assert!(message.contains("temporarily unavailable"));
        assert!(!message.to_lowercase().contains("bad gateway"));
        assert!(!message.contains("<html"));
    }
}

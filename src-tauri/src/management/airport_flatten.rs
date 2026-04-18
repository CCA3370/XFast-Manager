use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sea_orm::DatabaseConnection;
use tauri::State;

use crate::activity;
use crate::database::DatabaseState;
use crate::models::{
    AirportFlattenSearchResult, AirportFlattenSourceKind, AirportFlattenTarget, SceneryCategory,
    SceneryPackageInfo, SetAirportFlattenRequest,
};
use crate::scenery_index::SceneryIndexManager;

const FLATTEN_LINE: &str = "1302 flatten 1";

#[derive(Debug, Clone)]
struct AptAirportBlock {
    icao: String,
    name: String,
    flattened: bool,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
struct AptIcaoSummary {
    airport_name: String,
    flattened: bool,
}

fn global_airports_apt_path(xplane_root: &Path) -> PathBuf {
    xplane_root
        .join("Global Scenery")
        .join("Global Airports")
        .join("Earth nav data")
        .join("apt.dat")
}

fn is_airport_header(line: &str) -> bool {
    line.starts_with("1 ") || line.starts_with("16 ") || line.starts_with("17 ")
}

fn is_flatten_line(line: &str) -> bool {
    let parts: Vec<&str> = line.split_whitespace().collect();
    parts.len() == 3 && parts[0] == "1302" && parts[1] == "flatten" && parts[2] == "1"
}

fn parse_airport_header(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let icao = parts[4].trim().to_uppercase();
    if icao.is_empty() {
        return None;
    }

    let name = if parts.len() > 5 {
        parts[5..].join(" ")
    } else {
        icao.clone()
    };

    Some((icao, name))
}

fn split_text_into_lines(text: &str) -> Vec<String> {
    text.lines().map(|line| line.to_string()).collect()
}

fn parse_airport_blocks(lines: &[String]) -> Vec<AptAirportBlock> {
    let mut blocks = Vec::new();
    let mut current: Option<AptAirportBlock> = None;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if is_airport_header(trimmed) {
            if let Some(mut block) = current.take() {
                block.end = index;
                blocks.push(block);
            }

            current = parse_airport_header(trimmed).map(|(icao, name)| AptAirportBlock {
                icao,
                name,
                flattened: false,
                start: index,
                end: lines.len(),
            });
            continue;
        }

        if let Some(block) = current.as_mut() {
            if is_flatten_line(trimmed) {
                block.flattened = true;
            }
        }
    }

    if let Some(mut block) = current.take() {
        block.end = lines.len();
        blocks.push(block);
    }

    blocks
}

fn summarize_apt_text(text: &str) -> Vec<AptAirportBlock> {
    let lines = split_text_into_lines(text);
    parse_airport_blocks(&lines)
}

fn inspect_apt_file_for_icao(apt_path: &Path, icao: &str) -> io::Result<Option<AptIcaoSummary>> {
    let text = fs::read_to_string(apt_path)?;
    let matches: Vec<AptAirportBlock> = summarize_apt_text(&text)
        .into_iter()
        .filter(|block| block.icao.eq_ignore_ascii_case(icao))
        .collect();

    if matches.is_empty() {
        return Ok(None);
    }

    Ok(Some(AptIcaoSummary {
        airport_name: matches[0].name.clone(),
        flattened: matches.iter().all(|block| block.flattened),
    }))
}

fn search_apt_file(apt_path: &Path, query: &str) -> io::Result<Vec<(String, String)>> {
    let text = fs::read_to_string(apt_path)?;
    let query_upper = query.to_uppercase();
    let query_lower = query.to_lowercase();

    Ok(summarize_apt_text(&text)
        .into_iter()
        .filter(|block| {
            block.icao.contains(&query_upper) || block.name.to_lowercase().contains(&query_lower)
        })
        .map(|block| (block.icao, block.name))
        .collect())
}

fn summarize_single_scenery_target(
    scenery_dir: &Path,
    folder_name: &str,
) -> io::Result<Option<AirportFlattenTarget>> {
    let apt_path = scenery_dir.join("Earth nav data").join("apt.dat");
    if !apt_path.is_file() {
        return Ok(None);
    }

    let text = fs::read_to_string(&apt_path)?;
    let airports = summarize_apt_text(&text);
    if airports.len() != 1 {
        return Ok(None);
    }

    let airport = &airports[0];
    Ok(Some(AirportFlattenTarget {
        icao: airport.icao.clone(),
        airport_name: airport.name.clone(),
        source_kind: AirportFlattenSourceKind::Custom,
        source_label: folder_name.to_string(),
        source_path: apt_path.display().to_string(),
        folder_name: Some(folder_name.to_string()),
        flattened: airport.flattened,
    }))
}

fn resolve_scenery_dir(xplane_root: &Path, info: &SceneryPackageInfo) -> PathBuf {
    if let Some(actual_path) = info.actual_path.as_deref() {
        let resolved = PathBuf::from(actual_path);
        if resolved.is_absolute() {
            resolved
        } else {
            xplane_root.join(resolved)
        }
    } else {
        xplane_root.join("Custom Scenery").join(&info.folder_name)
    }
}

pub fn inspect_scenery_flatten_target(
    xplane_root: &Path,
    info: &SceneryPackageInfo,
) -> Option<AirportFlattenTarget> {
    if info.category == SceneryCategory::DefaultAirport
        || !info.has_apt_dat
        || info.folder_name.trim().eq_ignore_ascii_case("Global Airports")
        || info.airport_id.as_deref().unwrap_or("").is_empty()
    {
        return None;
    }

    let scenery_dir = resolve_scenery_dir(xplane_root, info);
    summarize_single_scenery_target(&scenery_dir, &info.folder_name)
        .ok()
        .flatten()
}

fn upsert_flatten_in_block(block: &[String], enabled: bool) -> Vec<String> {
    if block.is_empty() {
        return Vec::new();
    }

    let header = block[0].clone();
    let mut body: Vec<String> = block[1..]
        .iter()
        .filter(|line| !is_flatten_line(line.trim()))
        .cloned()
        .collect();

    if enabled {
        let insert_at = body
            .iter()
            .rposition(|line| line.trim().starts_with("1302 "))
            .map(|index| index + 1)
            .unwrap_or(0);
        body.insert(insert_at, FLATTEN_LINE.to_string());
    }

    let mut updated = Vec::with_capacity(body.len() + 1);
    updated.push(header);
    updated.extend(body);
    updated
}

fn rebuild_text(lines: &[String], original: &str) -> String {
    let line_ending = if original.contains("\r\n") { "\r\n" } else { "\n" };
    let mut rebuilt = lines.join(line_ending);
    if original.ends_with('\n') {
        rebuilt.push_str(line_ending);
    }
    rebuilt
}

fn set_flatten_state_in_text(original: &str, icao: &str, enabled: bool) -> Result<(String, String), String> {
    let mut lines = split_text_into_lines(original);
    let blocks = parse_airport_blocks(&lines);
    let matching_blocks: Vec<AptAirportBlock> = blocks
        .into_iter()
        .filter(|block| block.icao.eq_ignore_ascii_case(icao))
        .collect();

    if matching_blocks.is_empty() {
        return Err(format!("Airport {} was not found in apt.dat", icao));
    }

    let airport_name = matching_blocks[0].name.clone();

    for block in matching_blocks.iter().rev() {
        let replacement = upsert_flatten_in_block(&lines[block.start..block.end], enabled);
        lines.splice(block.start..block.end, replacement);
    }

    Ok((rebuild_text(&lines, original), airport_name))
}

fn backup_apt_file_if_needed(apt_path: &Path) -> io::Result<()> {
    let backup_path = apt_path.with_file_name("apt.dat.xfast.bak");
    if backup_path.exists() {
        return Ok(());
    }

    fs::copy(apt_path, backup_path)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_file_atomically(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winbase::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW};

    fn wide(path: &OsStr) -> Vec<u16> {
        path.encode_wide().chain(std::iter::once(0)).collect()
    }

    let source_wide = wide(source.as_os_str());
    let destination_wide = wide(destination.as_os_str());

    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };

    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn replace_file_atomically(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

fn write_updated_apt_file(apt_path: &Path, contents: &str) -> io::Result<()> {
    let parent = apt_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "apt.dat parent directory missing"))?;
    let temp_path = parent.join(format!(
        "apt.dat.xfast.tmp.{}",
        std::process::id()
    ));

    fs::write(&temp_path, contents.as_bytes())?;

    if let Err(error) = replace_file_atomically(&temp_path, apt_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    Ok(())
}

fn update_apt_flatten_state(apt_path: &Path, icao: &str, enabled: bool) -> Result<String, String> {
    let original = fs::read_to_string(apt_path)
        .map_err(|error| format!("Failed to read {}: {}", apt_path.display(), error))?;
    let (updated, airport_name) = set_flatten_state_in_text(&original, icao, enabled)?;

    if updated != original {
        backup_apt_file_if_needed(apt_path)
            .map_err(|error| format!("Failed to back up {}: {}", apt_path.display(), error))?;
        write_updated_apt_file(apt_path, &updated)
            .map_err(|error| format!("Failed to write {}: {}", apt_path.display(), error))?;
    }

    Ok(airport_name)
}

fn default_target_for_icao(xplane_root: &Path, icao: &str) -> Result<Option<AirportFlattenTarget>, String> {
    let apt_path = global_airports_apt_path(xplane_root);
    if !apt_path.is_file() {
        return Ok(None);
    }

    let Some(summary) = inspect_apt_file_for_icao(&apt_path, icao)
        .map_err(|error| format!("Failed to inspect {}: {}", apt_path.display(), error))?
    else {
        return Ok(None);
    };

    Ok(Some(AirportFlattenTarget {
        icao: icao.to_string(),
        airport_name: summary.airport_name,
        source_kind: AirportFlattenSourceKind::Default,
        source_label: "Global Airports".to_string(),
        source_path: apt_path.display().to_string(),
        folder_name: None,
        flattened: summary.flattened,
    }))
}

fn custom_targets_for_icao(
    xplane_root: &Path,
    packages: &[SceneryPackageInfo],
    icao: &str,
) -> Vec<AirportFlattenTarget> {
    let mut targets = Vec::new();

    for info in packages.iter().filter(|info| {
        info.category != SceneryCategory::DefaultAirport
            && info.has_apt_dat
            && !info.folder_name.trim().eq_ignore_ascii_case("Global Airports")
    }) {
        let scenery_dir = resolve_scenery_dir(xplane_root, info);
        let apt_path = scenery_dir.join("Earth nav data").join("apt.dat");
        if !apt_path.is_file() {
            continue;
        }

        let Some(summary) = inspect_apt_file_for_icao(&apt_path, icao).ok().flatten() else {
            continue;
        };

        targets.push(AirportFlattenTarget {
            icao: icao.to_string(),
            airport_name: summary.airport_name,
            source_kind: AirportFlattenSourceKind::Custom,
            source_label: info.folder_name.clone(),
            source_path: apt_path.display().to_string(),
            folder_name: Some(info.folder_name.clone()),
            flattened: summary.flattened,
        });
    }

    targets.sort_by(|left, right| left.source_label.cmp(&right.source_label));
    targets
}

fn upsert_search_entry(
    rows: &mut std::collections::HashMap<String, AirportFlattenSearchResult>,
    icao: String,
    airport_name: String,
    source_kind: AirportFlattenSourceKind,
) {
    let entry = rows
        .entry(icao.clone())
        .or_insert_with(|| AirportFlattenSearchResult {
            icao,
            airport_name: airport_name.clone(),
            has_default_source: false,
            custom_source_count: 0,
        });

    if entry.airport_name.is_empty() {
        entry.airport_name = airport_name;
    }

    match source_kind {
        AirportFlattenSourceKind::Default => entry.has_default_source = true,
        AirportFlattenSourceKind::Custom => entry.custom_source_count += 1,
    }
}

fn search_score(row: &AirportFlattenSearchResult, query_upper: &str) -> (i32, usize, usize) {
    let name_upper = row.airport_name.to_uppercase();
    let primary = if row.icao == query_upper {
        0
    } else if row.icao.starts_with(query_upper) {
        1
    } else if row.icao.contains(query_upper) {
        2
    } else if name_upper.starts_with(query_upper) {
        3
    } else {
        4
    };

    (primary, row.icao.len(), row.airport_name.len())
}

async fn load_scenery_packages(
    db: DatabaseConnection,
    xplane_root: &Path,
) -> Result<Vec<SceneryPackageInfo>, String> {
    let manager = SceneryIndexManager::new(xplane_root, db);
    let mut index = manager
        .load_index()
        .await
        .map_err(|error| format!("Failed to load scenery index: {}", error))?;

    if index.packages.is_empty() && xplane_root.join("Custom Scenery").is_dir() {
        index = manager
            .rebuild_index()
            .await
            .map_err(|error| format!("Failed to build scenery index: {}", error))?;
    }

    Ok(index.packages.into_values().collect())
}

#[tauri::command]
pub async fn airport_flatten_search_airports(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<AirportFlattenSearchResult>, String> {
    let xplane_root = PathBuf::from(&xplane_path);
    crate::validate_xplane_root_path(&xplane_root)?;

    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let normalized_query_upper = trimmed.to_uppercase();
    let limit = limit.unwrap_or(20).clamp(1, 100);

    let packages = load_scenery_packages(db.get(), &xplane_root).await?;

    let mut rows = std::collections::HashMap::<String, AirportFlattenSearchResult>::new();

    let default_apt = global_airports_apt_path(&xplane_root);
    if default_apt.is_file() {
        for (icao, airport_name) in
            search_apt_file(&default_apt, trimmed).map_err(|error| {
                format!("Failed to search {}: {}", default_apt.display(), error)
            })?
        {
            upsert_search_entry(
                &mut rows,
                icao,
                airport_name,
                AirportFlattenSourceKind::Default,
            );
        }
    }

    for info in packages.iter().filter(|info| {
        info.category != SceneryCategory::DefaultAirport
            && info.has_apt_dat
            && !info.folder_name.trim().eq_ignore_ascii_case("Global Airports")
    }) {
        let apt_path = resolve_scenery_dir(&xplane_root, info)
            .join("Earth nav data")
            .join("apt.dat");
        if !apt_path.is_file() {
            continue;
        }

        let matches = search_apt_file(&apt_path, trimmed).unwrap_or_default();
        let mut per_package = std::collections::HashMap::<String, String>::new();
        for (icao, airport_name) in matches {
            per_package.entry(icao).or_insert(airport_name);
        }

        for (icao, airport_name) in per_package {
            upsert_search_entry(&mut rows, icao, airport_name, AirportFlattenSourceKind::Custom);
        }
    }

    let mut results: Vec<AirportFlattenSearchResult> = rows.into_values().collect();
    results.sort_by(|left, right| {
        search_score(left, &normalized_query_upper)
            .cmp(&search_score(right, &normalized_query_upper))
            .then_with(|| left.icao.cmp(&right.icao))
    });
    results.truncate(limit);

    Ok(results)
}

#[tauri::command]
pub async fn airport_flatten_get_targets(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    icao: String,
) -> Result<Vec<AirportFlattenTarget>, String> {
    let xplane_root = PathBuf::from(&xplane_path);
    crate::validate_xplane_root_path(&xplane_root)?;

    let normalized_icao = icao.trim().to_uppercase();
    if normalized_icao.is_empty() {
        return Err("ICAO is required".to_string());
    }

    let packages = load_scenery_packages(db.get(), &xplane_root).await?;
    let mut targets = Vec::new();

    if let Some(default_target) = default_target_for_icao(&xplane_root, &normalized_icao)? {
        targets.push(default_target);
    }

    targets.extend(custom_targets_for_icao(
        &xplane_root,
        &packages,
        &normalized_icao,
    ));

    Ok(targets)
}

#[tauri::command]
pub async fn scenery_get_flatten_target(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    folder_name: String,
) -> Result<Option<AirportFlattenTarget>, String> {
    let xplane_root = PathBuf::from(&xplane_path);
    crate::validate_xplane_root_path(&xplane_root)?;

    let packages = load_scenery_packages(db.get(), &xplane_root).await?;
    let package = packages
        .into_iter()
        .find(|info| info.folder_name == folder_name)
        .ok_or_else(|| format!("Scenery package not found: {}", folder_name))?;

    Ok(inspect_scenery_flatten_target(&xplane_root, &package))
}

fn resolve_custom_apt_path(
    xplane_root: &Path,
    packages: &[SceneryPackageInfo],
    folder_name: &str,
) -> Result<PathBuf, String> {
    let package = packages
        .iter()
        .find(|info| info.folder_name == folder_name)
        .ok_or_else(|| format!("Scenery package not found: {}", folder_name))?;
    Ok(resolve_scenery_dir(xplane_root, package)
        .join("Earth nav data")
        .join("apt.dat"))
}

#[tauri::command]
pub async fn airport_flatten_set_state(
    db: State<'_, DatabaseState>,
    request: SetAirportFlattenRequest,
) -> Result<AirportFlattenTarget, String> {
    let xplane_root = PathBuf::from(&request.xplane_path);
    crate::validate_xplane_root_path(&xplane_root)?;

    let normalized_icao = request.icao.trim().to_uppercase();
    if normalized_icao.is_empty() {
        return Err("ICAO is required".to_string());
    }

    let packages = load_scenery_packages(db.get(), &xplane_root).await?;
    let operation = if request.enabled { "enable" } else { "disable" };

    let result = (|| -> Result<AirportFlattenTarget, String> {
        let (apt_path, source_kind, source_label, folder_name) = match request.source_kind {
            AirportFlattenSourceKind::Default => (
                global_airports_apt_path(&xplane_root),
                AirportFlattenSourceKind::Default,
                "Global Airports".to_string(),
                None,
            ),
            AirportFlattenSourceKind::Custom => {
                let folder_name = request
                    .folder_name
                    .clone()
                    .ok_or_else(|| "folderName is required for custom scenery flattening".to_string())?;
                (
                    resolve_custom_apt_path(&xplane_root, &packages, &folder_name)?,
                    AirportFlattenSourceKind::Custom,
                    folder_name.clone(),
                    Some(folder_name),
                )
            }
        };

        if !apt_path.is_file() {
            return Err(format!("apt.dat not found: {}", apt_path.display()));
        }

        let airport_name = update_apt_flatten_state(&apt_path, &normalized_icao, request.enabled)?;

        Ok(AirportFlattenTarget {
            icao: normalized_icao.clone(),
            airport_name,
            source_kind,
            source_label,
            source_path: apt_path.display().to_string(),
            folder_name,
            flattened: request.enabled,
        })
    })();

    match &result {
        Ok(target) => {
            activity::log_activity(
                &db.get(),
                operation,
                "airport_flatten",
                &target.icao,
                Some(format!(
                    "sourceKind={:?}; sourceLabel={}; enabled={}",
                    target.source_kind, target.source_label, target.flattened
                )),
                true,
            )
            .await;
        }
        Err(error) => {
            activity::log_activity(
                &db.get(),
                operation,
                "airport_flatten",
                &normalized_icao,
                Some(error.clone()),
                false,
            )
            .await;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_APT: &str = "I\n1100 Generated by test\n1 0 0 0 KSEA Seattle Intl\n1302 datum_lat 47.449\n1302 datum_lon -122.309\n100 45 1 0 0 0 0 0 16L 47.45 -122.31 34R 47.44 -122.30\n1 0 0 0 KPDX Portland Intl\n1302 datum_lat 45.589\n100 45 1 0 0 0 0 0 10L 45.58 -122.59 28R 45.60 -122.58\n";

    #[test]
    fn summarize_apt_text_detects_flatten_metadata() {
        let text = "I\n1100 Generated by test\n1 0 0 0 KSEA Seattle Intl\n1302 flatten 1\n100 45 1 0 0 0 0 0 16L 47.45 -122.31 34R 47.44 -122.30\n";
        let airports = summarize_apt_text(text);

        assert_eq!(airports.len(), 1);
        assert_eq!(airports[0].icao, "KSEA");
        assert!(airports[0].flattened);
    }

    #[test]
    fn enabling_flatten_updates_only_target_airport() {
        let (updated, airport_name) = set_flatten_state_in_text(SAMPLE_APT, "KSEA", true).unwrap();

        assert_eq!(airport_name, "Seattle Intl");
        assert!(updated.contains("1 0 0 0 KSEA Seattle Intl\n1302 datum_lat 47.449\n1302 datum_lon -122.309\n1302 flatten 1\n100 45 1 0 0 0 0 0 16L 47.45 -122.31 34R 47.44 -122.30"));
        assert!(!updated.contains("KPDX Portland Intl\n1302 flatten 1"));
    }

    #[test]
    fn disabling_flatten_preserves_other_metadata() {
        let text = "I\n1100 Generated by test\n1 0 0 0 KSEA Seattle Intl\n1302 datum_lat 47.449\n1302 flatten 1\n1302 datum_lon -122.309\n100 45 1 0 0 0 0 0 16L 47.45 -122.31 34R 47.44 -122.30\n";
        let (updated, _) = set_flatten_state_in_text(text, "KSEA", false).unwrap();

        assert!(updated.contains("1302 datum_lat 47.449"));
        assert!(updated.contains("1302 datum_lon -122.309"));
        assert!(!updated.contains("1302 flatten 1"));
    }

    #[test]
    fn summarize_single_scenery_target_requires_single_airport() {
        let dir = tempfile::tempdir().unwrap();
        let apt_dir = dir.path().join("Earth nav data");
        fs::create_dir_all(&apt_dir).unwrap();
        fs::write(apt_dir.join("apt.dat"), SAMPLE_APT).unwrap();

        let result = summarize_single_scenery_target(dir.path(), "Demo Airport").unwrap();
        assert!(result.is_none());
    }
}

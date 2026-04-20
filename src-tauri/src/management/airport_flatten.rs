use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex as TokioMutex;

use crate::activity;
use crate::app_dirs;
use crate::database::entities::airport_flatten_overrides as override_entity;
use crate::database::DatabaseState;
use crate::logger;
use crate::models::{
    AirportFlattenApplyAllResult, AirportFlattenApplyFailure, AirportFlattenOverride,
    AirportFlattenOverrideStatus, AirportFlattenSearchResult, AirportFlattenSourceKind,
    AirportFlattenTarget, SceneryCategory, SceneryPackageInfo, SetAirportFlattenRequest,
};
use crate::scenery_index::SceneryIndexManager;

const FLATTEN_LINE: &str = "1302 flatten 1";

static PARSED_APT_CACHE: LazyLock<RwLock<HashMap<String, ParsedAptCacheEntry>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static AIRPORT_FLATTEN_INDEX_CACHE: LazyLock<RwLock<HashMap<String, AirportFlattenIndex>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static AIRPORT_FLATTEN_INDEX_BUILD_LOCK: LazyLock<TokioMutex<()>> =
    LazyLock::new(|| TokioMutex::new(()));

const AIRPORT_FLATTEN_INDEX_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FileStamp {
    modified_ms: Option<u64>,
    len: u64,
}

#[derive(Debug, Clone)]
struct ParsedAptCacheEntry {
    stamp: FileStamp,
    airports: Vec<AptAirportBlock>,
}

#[derive(Debug, Clone)]
struct FlattenSourceRef {
    icao: String,
    source_kind: AirportFlattenSourceKind,
    source_label: String,
    source_path: String,
    folder_name: Option<String>,
    airport_name: String,
    flattened: bool,
}

#[derive(Debug, Clone)]
struct AirportFlattenIndex {
    source_files: HashMap<String, SourceFileIndexEntry>,
    search_rows: Vec<AirportFlattenSearchResult>,
    sources_by_icao: HashMap<String, Vec<FlattenSourceRef>>,
    single_custom_sources_by_folder: HashMap<String, FlattenSourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedAirportRecord {
    icao: String,
    airport_name: String,
    flattened: bool,
    byte_start: u64,
    byte_end: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceFileIndexEntry {
    source_kind: AirportFlattenSourceKind,
    source_label: String,
    source_path: String,
    folder_name: Option<String>,
    stamp: FileStamp,
    airports: Vec<CachedAirportRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedAirportFlattenIndex {
    version: u32,
    xplane_root: String,
    source_files: Vec<SourceFileIndexEntry>,
}

#[derive(Debug, Clone)]
struct CandidateSourceFile {
    source_kind: AirportFlattenSourceKind,
    source_label: String,
    source_path: String,
    folder_name: Option<String>,
}

#[derive(Debug, Clone)]
struct AptAirportBlock {
    icao: String,
    name: String,
    flattened: bool,
    start: usize,
    end: usize,
}

fn read_file_stamp(path: &Path) -> io::Result<FileStamp> {
    let metadata = fs::metadata(path)?;
    Ok(FileStamp {
        modified_ms: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64),
        len: metadata.len(),
    })
}

fn read_optional_file_stamp(path: &Path) -> Option<FileStamp> {
    read_file_stamp(path).ok()
}

fn parsed_apt_blocks(path: &Path) -> io::Result<Vec<AptAirportBlock>> {
    let key = path.to_string_lossy().to_string();
    let stamp = read_file_stamp(path)?;

    if let Some(cached) = PARSED_APT_CACHE
        .read()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
    {
        if cached.stamp == stamp {
            return Ok(cached.airports);
        }
    }

    let text = fs::read_to_string(path)?;
    let airports = summarize_apt_text(&text);

    if let Ok(mut cache) = PARSED_APT_CACHE.write() {
        cache.insert(
            key,
            ParsedAptCacheEntry {
                stamp,
                airports: airports.clone(),
            },
        );
    }

    Ok(airports)
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

fn trim_line_bytes(line: &[u8]) -> &[u8] {
    let mut end = line.len();
    while end > 0 && (line[end - 1] == b'\n' || line[end - 1] == b'\r') {
        end -= 1;
    }
    &line[..end]
}

fn scan_apt_file_for_cache(path: &Path) -> io::Result<Vec<CachedAirportRecord>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut airports = Vec::new();
    let mut current: Option<CachedAirportRecord> = None;
    let mut raw_line = Vec::<u8>::new();
    let mut byte_offset = 0u64;

    loop {
        raw_line.clear();
        let bytes_read = reader.read_until(b'\n', &mut raw_line)?;
        if bytes_read == 0 {
            break;
        }

        let trimmed = String::from_utf8_lossy(trim_line_bytes(&raw_line));
        let trimmed = trimmed.trim();

        if is_airport_header(trimmed) {
            if let Some(mut airport) = current.take() {
                airport.byte_end = byte_offset;
                airports.push(airport);
            }

            current = parse_airport_header(trimmed).map(|(icao, name)| CachedAirportRecord {
                icao,
                airport_name: name,
                flattened: false,
                byte_start: byte_offset,
                byte_end: byte_offset,
            });
        } else if let Some(airport) = current.as_mut() {
            if is_flatten_line(trimmed) {
                airport.flattened = true;
            }
        }

        byte_offset += bytes_read as u64;
    }

    if let Some(mut airport) = current.take() {
        airport.byte_end = byte_offset;
        airports.push(airport);
    }

    Ok(airports)
}

fn summarize_single_scenery_target(
    scenery_dir: &Path,
    folder_name: &str,
) -> io::Result<Option<AirportFlattenTarget>> {
    let apt_path = scenery_dir.join("Earth nav data").join("apt.dat");
    if !apt_path.is_file() {
        return Ok(None);
    }

    let airports = parsed_apt_blocks(&apt_path)?;
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
        || info
            .folder_name
            .trim()
            .eq_ignore_ascii_case("Global Airports")
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
    let line_ending = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut rebuilt = lines.join(line_ending);
    if original.ends_with('\n') {
        rebuilt.push_str(line_ending);
    }
    rebuilt
}

fn set_flatten_state_in_text(
    original: &str,
    icao: &str,
    enabled: bool,
) -> Result<(String, String), String> {
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

fn clear_readonly_attribute(path: &Path) -> io::Result<()> {
    let metadata = match fs::metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_file_atomically(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winbase::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};

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
    let parent = apt_path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "apt.dat parent directory missing")
    })?;
    let temp_path = parent.join(format!("apt.dat.xfast.tmp.{}", std::process::id()));

    fs::write(&temp_path, contents.as_bytes())?;
    let _ = clear_readonly_attribute(apt_path);

    if let Err(error) = replace_file_atomically(&temp_path, apt_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    Ok(())
}

fn store_parsed_apt_cache(apt_path: &Path, contents: &str) -> io::Result<()> {
    let key = apt_path.to_string_lossy().to_string();
    let stamp = read_file_stamp(apt_path)?;
    let airports = summarize_apt_text(contents);

    if let Ok(mut cache) = PARSED_APT_CACHE.write() {
        cache.insert(key, ParsedAptCacheEntry { stamp, airports });
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
        store_parsed_apt_cache(apt_path, &updated).map_err(|error| {
            format!(
                "Failed to refresh apt cache for {}: {}",
                apt_path.display(),
                error
            )
        })?;
    }

    Ok(airport_name)
}

fn update_cached_parsed_apt_state(apt_path: &Path, icao: &str, flattened: bool) {
    let key = apt_path.to_string_lossy().to_string();
    let new_stamp = read_optional_file_stamp(apt_path);

    if let Ok(mut cache) = PARSED_APT_CACHE.write() {
        if let Some(entry) = cache.get_mut(&key) {
            for airport in entry.airports.iter_mut() {
                if airport.icao.eq_ignore_ascii_case(icao) {
                    airport.flattened = flattened;
                }
            }
            if let Some(stamp) = new_stamp {
                entry.stamp = stamp;
            }
        }
    }
}

fn rewrite_airport_block_by_offsets(
    apt_path: &Path,
    airport: &CachedAirportRecord,
    enabled: bool,
) -> Result<(String, i64), String> {
    use std::fs::File;
    use std::io::{BufWriter, Read, Seek, SeekFrom, Write};

    let mut source = File::open(apt_path)
        .map_err(|error| format!("Failed to read {}: {}", apt_path.display(), error))?;

    let block_len = airport.byte_end.saturating_sub(airport.byte_start);
    source
        .seek(SeekFrom::Start(airport.byte_start))
        .map_err(|error| format!("Failed to seek {}: {}", apt_path.display(), error))?;

    let mut original_block = vec![0u8; block_len as usize];
    source.read_exact(&mut original_block).map_err(|error| {
        format!(
            "Failed to read airport block from {}: {}",
            apt_path.display(),
            error
        )
    })?;

    let original_block_text = String::from_utf8_lossy(&original_block).into_owned();
    let (updated_block_text, airport_name) =
        set_flatten_state_in_text(&original_block_text, &airport.icao, enabled)?;

    if updated_block_text == original_block_text {
        return Ok((airport_name, 0));
    }

    backup_apt_file_if_needed(apt_path)
        .map_err(|error| format!("Failed to back up {}: {}", apt_path.display(), error))?;

    let parent = apt_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "apt.dat parent directory missing"))
        .map_err(|error| error.to_string())?;
    let temp_path = parent.join(format!("apt.dat.xfast.tmp.{}", std::process::id()));
    let temp_file = File::create(&temp_path)
        .map_err(|error| format!("Failed to create {}: {}", temp_path.display(), error))?;
    let mut writer = BufWriter::new(temp_file);

    source
        .seek(SeekFrom::Start(0))
        .map_err(|error| format!("Failed to seek {}: {}", apt_path.display(), error))?;
    io::copy(
        &mut std::io::Read::by_ref(&mut source).take(airport.byte_start),
        &mut writer,
    )
    .map_err(|error| {
        format!(
            "Failed to copy prelude for {}: {}",
            apt_path.display(),
            error
        )
    })?;

    writer
        .write_all(updated_block_text.as_bytes())
        .map_err(|error| {
            format!(
                "Failed to write updated block for {}: {}",
                apt_path.display(),
                error
            )
        })?;

    source
        .seek(SeekFrom::Start(airport.byte_end))
        .map_err(|error| format!("Failed to seek {}: {}", apt_path.display(), error))?;
    io::copy(&mut source, &mut writer)
        .map_err(|error| format!("Failed to copy tail for {}: {}", apt_path.display(), error))?;
    writer
        .flush()
        .map_err(|error| format!("Failed to flush {}: {}", temp_path.display(), error))?;
    drop(writer);
    drop(source);

    let _ = clear_readonly_attribute(apt_path);

    replace_file_atomically(&temp_path, apt_path)
        .map_err(|error| format!("Failed to replace {}: {}", apt_path.display(), error))?;

    let delta = updated_block_text.as_bytes().len() as i64 - original_block.len() as i64;
    Ok((airport_name, delta))
}

fn build_target_from_source_ref(
    source_ref: &FlattenSourceRef,
    icao: &str,
) -> Result<AirportFlattenTarget, String> {
    Ok(AirportFlattenTarget {
        icao: icao.to_string(),
        airport_name: source_ref.airport_name.clone(),
        source_kind: source_ref.source_kind.clone(),
        source_label: source_ref.source_label.clone(),
        source_path: source_ref.source_path.clone(),
        folder_name: source_ref.folder_name.clone(),
        flattened: source_ref.flattened,
    })
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

fn airport_flatten_cache_key(xplane_root: &Path) -> String {
    xplane_root.to_string_lossy().to_string()
}

fn normalize_xplane_root_key(xplane_root: &Path) -> String {
    let value = xplane_root.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        value.to_lowercase()
    }

    #[cfg(not(target_os = "windows"))]
    {
        value
    }
}

fn airport_flatten_cache_path(xplane_root: &Path) -> PathBuf {
    let normalized = normalize_xplane_root_key(xplane_root);
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    let file_name = format!("airport_flatten_{:016x}.json", hasher.finish());
    app_dirs::get_app_data_dir()
        .join("cache")
        .join("airport_flatten")
        .join(file_name)
}

fn load_persisted_airport_flatten_source_files(
    xplane_root: &Path,
) -> Result<Option<HashMap<String, SourceFileIndexEntry>>, String> {
    let cache_path = airport_flatten_cache_path(xplane_root);
    if !cache_path.is_file() {
        return Ok(None);
    }

    let content = fs::read_to_string(&cache_path)
        .map_err(|error| format!("Failed to read {}: {}", cache_path.display(), error))?;
    let persisted: PersistedAirportFlattenIndex = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(error) => {
            logger::log_info(
                &format!(
                    "Discarding incompatible airport flatten cache {}: {}",
                    cache_path.display(),
                    error
                ),
                Some("airport_flatten"),
            );
            let _ = fs::remove_file(&cache_path);
            return Ok(None);
        }
    };

    if persisted.version != AIRPORT_FLATTEN_INDEX_VERSION
        || persisted.xplane_root != normalize_xplane_root_key(xplane_root)
    {
        return Ok(None);
    }

    Ok(Some(
        persisted
            .source_files
            .into_iter()
            .map(|entry| (entry.source_path.clone(), entry))
            .collect(),
    ))
}

fn persist_airport_flatten_source_files(
    xplane_root: &Path,
    source_files: &HashMap<String, SourceFileIndexEntry>,
) -> Result<(), String> {
    let cache_path = airport_flatten_cache_path(xplane_root);
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {}", parent.display(), error))?;
    }

    let persisted = PersistedAirportFlattenIndex {
        version: AIRPORT_FLATTEN_INDEX_VERSION,
        xplane_root: normalize_xplane_root_key(xplane_root),
        source_files: source_files.values().cloned().collect(),
    };
    let json = serde_json::to_string_pretty(&persisted)
        .map_err(|error| format!("Failed to serialize airport flatten index: {}", error))?;
    let temp_path = cache_path.with_extension("tmp");
    fs::write(&temp_path, json)
        .map_err(|error| format!("Failed to write {}: {}", temp_path.display(), error))?;
    fs::rename(&temp_path, &cache_path)
        .map_err(|error| format!("Failed to replace {}: {}", cache_path.display(), error))?;

    Ok(())
}

fn candidate_source_files(
    xplane_root: &Path,
    packages: &[SceneryPackageInfo],
) -> Vec<CandidateSourceFile> {
    let mut files = Vec::new();
    let default_apt = global_airports_apt_path(xplane_root);
    if default_apt.is_file() {
        files.push(CandidateSourceFile {
            source_kind: AirportFlattenSourceKind::Default,
            source_label: "Global Airports".to_string(),
            source_path: default_apt.display().to_string(),
            folder_name: None,
        });
    }

    for info in packages.iter().filter(|info| {
        info.category != SceneryCategory::DefaultAirport
            && info.has_apt_dat
            && !info
                .folder_name
                .trim()
                .eq_ignore_ascii_case("Global Airports")
    }) {
        let apt_path = resolve_scenery_dir(xplane_root, info)
            .join("Earth nav data")
            .join("apt.dat");
        if !apt_path.is_file() {
            continue;
        }

        files.push(CandidateSourceFile {
            source_kind: AirportFlattenSourceKind::Custom,
            source_label: info.folder_name.clone(),
            source_path: apt_path.display().to_string(),
            folder_name: Some(info.folder_name.clone()),
        });
    }

    files
}

fn compress_airports_for_cache(airports: Vec<CachedAirportRecord>) -> Vec<CachedAirportRecord> {
    let mut unique = HashMap::<String, CachedAirportRecord>::new();

    for airport in airports {
        unique
            .entry(airport.icao.clone())
            .and_modify(|entry| {
                entry.flattened &= airport.flattened;
                entry.byte_start = entry.byte_start.min(airport.byte_start);
                entry.byte_end = entry.byte_end.max(airport.byte_end);
            })
            .or_insert(airport);
    }

    let mut rows: Vec<CachedAirportRecord> = unique.into_values().collect();
    rows.sort_by(|left, right| left.icao.cmp(&right.icao));
    rows
}

fn derive_airport_flatten_index(
    source_files: HashMap<String, SourceFileIndexEntry>,
) -> AirportFlattenIndex {
    let mut search_rows = HashMap::<String, AirportFlattenSearchResult>::new();
    let mut sources_by_icao = HashMap::<String, Vec<FlattenSourceRef>>::new();
    let mut single_custom_sources_by_folder = HashMap::<String, FlattenSourceRef>::new();

    for source_file in source_files.values() {
        let is_single_custom_source = source_file.source_kind == AirportFlattenSourceKind::Custom
            && source_file.airports.len() == 1;

        for airport in &source_file.airports {
            upsert_search_entry(
                &mut search_rows,
                airport.icao.clone(),
                airport.airport_name.clone(),
                source_file.source_kind.clone(),
            );

            let source_ref = FlattenSourceRef {
                icao: airport.icao.clone(),
                source_kind: source_file.source_kind.clone(),
                source_label: source_file.source_label.clone(),
                source_path: source_file.source_path.clone(),
                folder_name: source_file.folder_name.clone(),
                airport_name: airport.airport_name.clone(),
                flattened: airport.flattened,
            };

            if is_single_custom_source {
                if let Some(folder_name) = source_file.folder_name.as_ref() {
                    single_custom_sources_by_folder.insert(folder_name.clone(), source_ref.clone());
                }
            }

            sources_by_icao
                .entry(airport.icao.clone())
                .or_default()
                .push(source_ref);
        }
    }

    for refs in sources_by_icao.values_mut() {
        refs.sort_by(|left, right| left.source_label.cmp(&right.source_label));
    }

    let mut search_rows: Vec<AirportFlattenSearchResult> = search_rows.into_values().collect();
    search_rows.sort_by(|left, right| left.icao.cmp(&right.icao));

    AirportFlattenIndex {
        source_files,
        search_rows,
        sources_by_icao,
        single_custom_sources_by_folder,
    }
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

async fn refresh_airport_flatten_source_files(
    db: DatabaseConnection,
    xplane_root: &Path,
    previous: HashMap<String, SourceFileIndexEntry>,
) -> Result<HashMap<String, SourceFileIndexEntry>, String> {
    let packages = load_scenery_packages(db, xplane_root).await?;
    let candidates = candidate_source_files(xplane_root, &packages);
    let mut refreshed = HashMap::<String, SourceFileIndexEntry>::new();

    for candidate in candidates {
        let path = PathBuf::from(&candidate.source_path);
        let stamp = match read_file_stamp(&path) {
            Ok(value) => value,
            Err(error) => {
                return Err(format!("Failed to stat {}: {}", path.display(), error));
            }
        };

        if let Some(existing) = previous.get(&candidate.source_path) {
            if existing.stamp == stamp
                && existing.source_kind == candidate.source_kind
                && existing.source_label == candidate.source_label
                && existing.folder_name == candidate.folder_name
            {
                refreshed.insert(candidate.source_path.clone(), existing.clone());
                continue;
            }
        }

        let airports = scan_apt_file_for_cache(&path)
            .map_err(|error| format!("Failed to inspect {}: {}", path.display(), error))?;
        if airports.is_empty() {
            continue;
        }

        refreshed.insert(
            candidate.source_path.clone(),
            SourceFileIndexEntry {
                source_kind: candidate.source_kind,
                source_label: candidate.source_label,
                source_path: candidate.source_path,
                folder_name: candidate.folder_name,
                stamp,
                airports: compress_airports_for_cache(airports),
            },
        );
    }

    Ok(refreshed)
}

async fn get_or_build_airport_flatten_index(
    db: DatabaseConnection,
    xplane_root: &Path,
) -> Result<AirportFlattenIndex, String> {
    let key = airport_flatten_cache_key(xplane_root);

    if let Some(cached) = AIRPORT_FLATTEN_INDEX_CACHE
        .read()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
    {
        return Ok(cached);
    }

    let _build_guard = AIRPORT_FLATTEN_INDEX_BUILD_LOCK.lock().await;

    if let Some(cached) = AIRPORT_FLATTEN_INDEX_CACHE
        .read()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
    {
        return Ok(cached);
    }

    let previous_source_files = if let Some(cached) = AIRPORT_FLATTEN_INDEX_CACHE
        .read()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
    {
        cached.source_files
    } else {
        load_persisted_airport_flatten_source_files(xplane_root)?.unwrap_or_default()
    };

    let refreshed_source_files =
        refresh_airport_flatten_source_files(db, xplane_root, previous_source_files).await?;
    let built = derive_airport_flatten_index(refreshed_source_files.clone());

    if let Ok(mut cache) = AIRPORT_FLATTEN_INDEX_CACHE.write() {
        cache.insert(key, built.clone());
    }

    if let Err(error) = persist_airport_flatten_source_files(xplane_root, &refreshed_source_files) {
        logger::log_info(
            &format!("Failed to persist airport flatten index: {}", error),
            Some("airport_flatten"),
        );
    }

    Ok(built)
}

fn update_cached_flatten_state(
    xplane_root: &Path,
    icao: &str,
    source_path: &str,
    flattened: bool,
    airport_name: &str,
    block_delta: i64,
) {
    let key = airport_flatten_cache_key(xplane_root);
    let mut source_files_to_persist: Option<HashMap<String, SourceFileIndexEntry>> = None;

    if let Ok(mut cache) = AIRPORT_FLATTEN_INDEX_CACHE.write() {
        if let Some(entry) = cache.get_mut(&key) {
            let mut persisted_changed = false;

            if let Some(source_refs) = entry.sources_by_icao.get_mut(icao) {
                for source_ref in source_refs.iter_mut() {
                    if source_ref.source_path == source_path {
                        source_ref.flattened = flattened;
                        source_ref.airport_name = airport_name.to_string();
                        persisted_changed = true;
                    }
                }
            }

            if let Some(source_file) = entry.source_files.get_mut(source_path) {
                let mut target_end_before = None::<u64>;
                for airport in source_file.airports.iter_mut() {
                    if airport.icao.eq_ignore_ascii_case(icao) {
                        target_end_before = Some(airport.byte_end);
                        airport.flattened = flattened;
                        airport.airport_name = airport_name.to_string();
                        airport.byte_end = ((airport.byte_end as i64) + block_delta) as u64;
                        persisted_changed = true;
                    }
                }

                if block_delta != 0 {
                    if let Some(old_end) = target_end_before {
                        for airport in source_file.airports.iter_mut() {
                            if airport.byte_start >= old_end {
                                airport.byte_start =
                                    ((airport.byte_start as i64) + block_delta) as u64;
                                airport.byte_end = ((airport.byte_end as i64) + block_delta) as u64;
                            }
                        }
                    }
                }

                if let Some(stamp) = read_optional_file_stamp(Path::new(&source_file.source_path)) {
                    source_file.stamp = stamp;
                }
            }

            if let Some(source_file) = entry.source_files.get(source_path) {
                if let Some(folder_name) = source_file.folder_name.as_deref() {
                    if let Some(source_ref) =
                        entry.single_custom_sources_by_folder.get_mut(folder_name)
                    {
                        source_ref.flattened = flattened;
                        source_ref.airport_name = airport_name.to_string();
                        persisted_changed = true;
                    }
                }
            }

            if let Some(source_refs) = entry.sources_by_icao.get_mut(icao) {
                source_refs.sort_by(|left, right| left.source_label.cmp(&right.source_label));
            }

            if let Some(source_file) = entry.source_files.get_mut(source_path) {
                source_file
                    .airports
                    .sort_by(|left, right| left.byte_start.cmp(&right.byte_start));
            }

            if persisted_changed {
                source_files_to_persist = Some(entry.source_files.clone());
            }
        }
    }

    if let Some(source_files) = source_files_to_persist {
        if let Err(error) = persist_airport_flatten_source_files(xplane_root, &source_files) {
            logger::log_info(
                &format!("Failed to persist airport flatten index: {}", error),
                Some("airport_flatten"),
            );
        }
    }
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

    let index = get_or_build_airport_flatten_index(db.get(), &xplane_root).await?;
    let query_lower = trimmed.to_lowercase();
    let mut results: Vec<AirportFlattenSearchResult> = index
        .search_rows
        .iter()
        .filter(|row| {
            row.icao.contains(&normalized_query_upper)
                || row.airport_name.to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect();
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

    let index = get_or_build_airport_flatten_index(db.get(), &xplane_root).await?;
    let Some(source_refs) = index.sources_by_icao.get(&normalized_icao) else {
        return Ok(Vec::new());
    };

    let mut targets = Vec::with_capacity(source_refs.len());
    for source_ref in source_refs {
        targets.push(build_target_from_source_ref(source_ref, &normalized_icao)?);
    }

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

    let index = get_or_build_airport_flatten_index(db.get(), &xplane_root).await?;
    let Some(source_ref) = index.single_custom_sources_by_folder.get(&folder_name) else {
        return Ok(None);
    };

    Ok(Some(build_target_from_source_ref(
        source_ref,
        &source_ref.icao,
    )?))
}

#[tauri::command]
pub async fn airport_flatten_set_state(
    db: State<'_, DatabaseState>,
    request: SetAirportFlattenRequest,
) -> Result<AirportFlattenTarget, String> {
    let xplane_root = PathBuf::from(&request.xplane_path);
    crate::validate_xplane_root_path(&xplane_root)?;

    let operation = if request.enabled { "enable" } else { "disable" };
    let icao_for_log = request.icao.trim().to_uppercase();
    let conn = db.get();

    let result = apply_flatten_state_inner(&conn, &xplane_root, request).await;

    match &result {
        Ok(target) => {
            activity::log_activity(
                &conn,
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
                &conn,
                operation,
                "airport_flatten",
                &icao_for_log,
                Some(error.clone()),
                false,
            )
            .await;
        }
    }

    result
}

async fn apply_flatten_state_inner(
    db: &DatabaseConnection,
    xplane_root: &Path,
    request: SetAirportFlattenRequest,
) -> Result<AirportFlattenTarget, String> {
    let normalized_icao = request.icao.trim().to_uppercase();
    if normalized_icao.is_empty() {
        return Err("ICAO is required".to_string());
    }

    let cache_key = airport_flatten_cache_key(xplane_root);
    let maybe_index = if request.source_path.is_some() {
        None
    } else {
        Some(get_or_build_airport_flatten_index(db.clone(), xplane_root).await?)
    };

    let source_ref = if let Some(source_path) = request.source_path.clone() {
        let cached_ref = AIRPORT_FLATTEN_INDEX_CACHE
            .read()
            .ok()
            .and_then(|cache| cache.get(&cache_key).cloned())
            .and_then(|index| {
                index
                    .sources_by_icao
                    .get(&normalized_icao)
                    .and_then(|refs| {
                        refs.iter()
                            .find(|source| source.source_path == source_path)
                            .cloned()
                    })
            });

        match cached_ref {
            Some(value) => value,
            None => FlattenSourceRef {
                icao: normalized_icao.clone(),
                source_kind: request.source_kind.clone(),
                source_label: match request.source_kind {
                    AirportFlattenSourceKind::Default => "Global Airports".to_string(),
                    AirportFlattenSourceKind::Custom => {
                        request.folder_name.clone().ok_or_else(|| {
                            "folderName is required for custom scenery flattening".to_string()
                        })?
                    }
                },
                source_path,
                folder_name: request.folder_name.clone(),
                airport_name: normalized_icao.clone(),
                flattened: request.enabled,
            },
        }
    } else {
        let index = maybe_index
            .as_ref()
            .ok_or_else(|| "Airport flatten index is unavailable".to_string())?;
        match request.source_kind {
            AirportFlattenSourceKind::Default => index
                .sources_by_icao
                .get(&normalized_icao)
                .and_then(|refs| {
                    refs.iter()
                        .find(|source| source.source_kind == AirportFlattenSourceKind::Default)
                })
                .cloned()
                .ok_or_else(|| {
                    format!("Default airport source not found for {}", normalized_icao)
                })?,
            AirportFlattenSourceKind::Custom => {
                let folder_name = request.folder_name.clone().ok_or_else(|| {
                    "folderName is required for custom scenery flattening".to_string()
                })?;
                index
                    .single_custom_sources_by_folder
                    .get(&folder_name)
                    .cloned()
                    .ok_or_else(|| format!("Custom airport source not found for {}", folder_name))?
            }
        }
    };

    let apt_path = PathBuf::from(&source_ref.source_path);
    crate::path_utils::validate_child_path(xplane_root, &apt_path)
        .map_err(|error| format!("Invalid airport source path: {}", error))?;

    if !apt_path.is_file() {
        return Err(format!("apt.dat not found: {}", apt_path.display()));
    }

    let target = if source_ref.flattened == request.enabled {
        AirportFlattenTarget {
            icao: normalized_icao.clone(),
            airport_name: source_ref.airport_name.clone(),
            source_kind: source_ref.source_kind.clone(),
            source_label: source_ref.source_label.clone(),
            source_path: source_ref.source_path.clone(),
            folder_name: source_ref.folder_name.clone(),
            flattened: request.enabled,
        }
    } else {
        let cached_airport = AIRPORT_FLATTEN_INDEX_CACHE
            .read()
            .ok()
            .and_then(|cache| cache.get(&cache_key).cloned())
            .and_then(|index| index.source_files.get(&source_ref.source_path).cloned())
            .and_then(|source_file| {
                source_file
                    .airports
                    .into_iter()
                    .find(|airport| airport.icao.eq_ignore_ascii_case(&normalized_icao))
            });

        let (airport_name, block_delta) = if let Some(cached_airport) = cached_airport {
            rewrite_airport_block_by_offsets(&apt_path, &cached_airport, request.enabled)?
        } else {
            (
                update_apt_flatten_state(&apt_path, &normalized_icao, request.enabled)?,
                0,
            )
        };
        update_cached_parsed_apt_state(&apt_path, &normalized_icao, request.enabled);
        update_cached_flatten_state(
            xplane_root,
            &normalized_icao,
            &source_ref.source_path,
            request.enabled,
            &airport_name,
            block_delta,
        );

        AirportFlattenTarget {
            icao: normalized_icao.clone(),
            airport_name,
            source_kind: source_ref.source_kind.clone(),
            source_label: source_ref.source_label.clone(),
            source_path: source_ref.source_path.clone(),
            folder_name: source_ref.folder_name.clone(),
            flattened: request.enabled,
        }
    };

    if let Err(error) = upsert_override(db, xplane_root, &target).await {
        logger::log_info(
            &format!("Failed to persist airport flatten override: {}", error),
            Some("airport_flatten"),
        );
    }

    Ok(target)
}

fn source_kind_to_db_str(kind: &AirportFlattenSourceKind) -> &'static str {
    match kind {
        AirportFlattenSourceKind::Default => "default",
        AirportFlattenSourceKind::Custom => "custom",
    }
}

fn source_kind_from_db_str(value: &str) -> AirportFlattenSourceKind {
    if value.eq_ignore_ascii_case("default") {
        AirportFlattenSourceKind::Default
    } else {
        AirportFlattenSourceKind::Custom
    }
}

async fn upsert_override(
    db: &DatabaseConnection,
    xplane_root: &Path,
    target: &AirportFlattenTarget,
) -> Result<(), sea_orm::DbErr> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);

    let active = override_entity::ActiveModel {
        xplane_path: Set(normalize_xplane_root_key(xplane_root)),
        icao: Set(target.icao.clone()),
        source_path: Set(target.source_path.clone()),
        source_kind: Set(source_kind_to_db_str(&target.source_kind).to_string()),
        source_label: Set(target.source_label.clone()),
        folder_name: Set(target.folder_name.clone()),
        airport_name: Set(target.airport_name.clone()),
        desired_flattened: Set(target.flattened),
        updated_at: Set(now_ms),
        ..Default::default()
    };

    override_entity::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([
                override_entity::Column::XplanePath,
                override_entity::Column::SourcePath,
                override_entity::Column::Icao,
            ])
            .update_columns([
                override_entity::Column::SourceKind,
                override_entity::Column::SourceLabel,
                override_entity::Column::FolderName,
                override_entity::Column::AirportName,
                override_entity::Column::DesiredFlattened,
                override_entity::Column::UpdatedAt,
            ])
            .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

fn classify_override_status(current: Option<bool>, desired: bool) -> AirportFlattenOverrideStatus {
    match current {
        None => AirportFlattenOverrideStatus::SourceMissing,
        Some(value) if value == desired => AirportFlattenOverrideStatus::InSync,
        Some(_) => AirportFlattenOverrideStatus::Drifted,
    }
}

fn status_priority(status: AirportFlattenOverrideStatus) -> u8 {
    match status {
        AirportFlattenOverrideStatus::Drifted => 0,
        AirportFlattenOverrideStatus::SourceMissing => 1,
        AirportFlattenOverrideStatus::InSync => 2,
    }
}

fn current_flattened_for(
    index: &AirportFlattenIndex,
    source_path: &str,
    icao: &str,
) -> Option<bool> {
    index
        .source_files
        .get(source_path)
        .and_then(|source_file| {
            source_file
                .airports
                .iter()
                .find(|airport| airport.icao.eq_ignore_ascii_case(icao))
        })
        .map(|airport| airport.flattened)
}

#[tauri::command]
pub async fn airport_flatten_list_overrides(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> Result<Vec<AirportFlattenOverride>, String> {
    let xplane_root = PathBuf::from(&xplane_path);
    crate::validate_xplane_root_path(&xplane_root)?;

    let conn = db.get();
    let xplane_key = normalize_xplane_root_key(&xplane_root);

    let rows = override_entity::Entity::find()
        .filter(override_entity::Column::XplanePath.eq(xplane_key))
        .all(&conn)
        .await
        .map_err(|error| format!("Failed to load airport flatten overrides: {}", error))?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let index = get_or_build_airport_flatten_index(conn, &xplane_root).await?;

    let mut results: Vec<AirportFlattenOverride> = rows
        .into_iter()
        .map(|row| {
            let current_flattened = current_flattened_for(&index, &row.source_path, &row.icao);
            let status = classify_override_status(current_flattened, row.desired_flattened);
            let source_kind = source_kind_from_db_str(&row.source_kind);
            AirportFlattenOverride {
                icao: row.icao,
                airport_name: row.airport_name,
                source_kind,
                source_label: row.source_label,
                source_path: row.source_path,
                folder_name: row.folder_name,
                desired_flattened: row.desired_flattened,
                current_flattened,
                status,
                updated_at: row.updated_at,
            }
        })
        .collect();

    results.sort_by(|left, right| {
        status_priority(left.status)
            .cmp(&status_priority(right.status))
            .then_with(|| left.icao.cmp(&right.icao))
    });

    Ok(results)
}

#[tauri::command]
pub async fn airport_flatten_clear_override(
    db: State<'_, DatabaseState>,
    xplane_path: String,
    icao: String,
    source_path: String,
) -> Result<(), String> {
    let xplane_root = PathBuf::from(&xplane_path);
    crate::validate_xplane_root_path(&xplane_root)?;

    let normalized_icao = icao.trim().to_uppercase();
    if normalized_icao.is_empty() {
        return Err("ICAO is required".to_string());
    }

    let conn = db.get();
    let xplane_key = normalize_xplane_root_key(&xplane_root);

    override_entity::Entity::delete_many()
        .filter(override_entity::Column::XplanePath.eq(xplane_key))
        .filter(override_entity::Column::Icao.eq(normalized_icao))
        .filter(override_entity::Column::SourcePath.eq(source_path))
        .exec(&conn)
        .await
        .map_err(|error| format!("Failed to delete airport flatten override: {}", error))?;

    Ok(())
}

#[tauri::command]
pub async fn airport_flatten_apply_all_drifted(
    db: State<'_, DatabaseState>,
    xplane_path: String,
) -> Result<AirportFlattenApplyAllResult, String> {
    let xplane_root = PathBuf::from(&xplane_path);
    crate::validate_xplane_root_path(&xplane_root)?;

    let conn = db.get();
    let xplane_key = normalize_xplane_root_key(&xplane_root);

    let rows = override_entity::Entity::find()
        .filter(override_entity::Column::XplanePath.eq(xplane_key))
        .all(&conn)
        .await
        .map_err(|error| format!("Failed to load airport flatten overrides: {}", error))?;

    let mut applied = 0usize;
    let mut skipped = 0usize;
    let mut failed: Vec<AirportFlattenApplyFailure> = Vec::new();

    if rows.is_empty() {
        return Ok(AirportFlattenApplyAllResult {
            applied,
            skipped,
            failed,
        });
    }

    let index = get_or_build_airport_flatten_index(conn.clone(), &xplane_root).await?;

    for row in rows {
        let current = current_flattened_for(&index, &row.source_path, &row.icao);

        let current_flattened = match current {
            Some(value) => value,
            None => {
                skipped += 1;
                continue;
            }
        };

        if current_flattened == row.desired_flattened {
            continue;
        }

        let source_kind = source_kind_from_db_str(&row.source_kind);
        let request = SetAirportFlattenRequest {
            xplane_path: xplane_path.clone(),
            icao: row.icao.clone(),
            source_kind,
            folder_name: row.folder_name.clone(),
            source_path: Some(row.source_path.clone()),
            enabled: row.desired_flattened,
        };

        let operation = if row.desired_flattened {
            "enable"
        } else {
            "disable"
        };

        match apply_flatten_state_inner(&conn, &xplane_root, request).await {
            Ok(target) => {
                applied += 1;
                activity::log_activity(
                    &conn,
                    operation,
                    "airport_flatten",
                    &target.icao,
                    Some(format!(
                        "applyAll; sourceLabel={}; enabled={}",
                        target.source_label, target.flattened
                    )),
                    true,
                )
                .await;
            }
            Err(error) => {
                activity::log_activity(
                    &conn,
                    operation,
                    "airport_flatten",
                    &row.icao,
                    Some(format!("applyAll: {}", error)),
                    false,
                )
                .await;
                failed.push(AirportFlattenApplyFailure {
                    icao: row.icao.clone(),
                    source_path: row.source_path.clone(),
                    error,
                });
            }
        }
    }

    Ok(AirportFlattenApplyAllResult {
        applied,
        skipped,
        failed,
    })
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

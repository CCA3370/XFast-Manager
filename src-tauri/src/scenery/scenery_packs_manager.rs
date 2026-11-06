//! Scenery packs.ini manager module
//!
//! This module writes and sorts the scenery_packs.ini file using the index as source of truth
//! based on scenery classifications.

use crate::database::SceneryQueries;
use crate::logger;
use crate::models::{
    is_global_airports_folder_name, SceneryCategory, SceneryEntryUpdate, SceneryIndex,
    SceneryPackEntry, SceneryPackageInfo, GLOBAL_AIRPORTS_ENTRY_NAME,
};
use crate::scenery_index::SceneryIndexManager;
use anyhow::{anyhow, Result};
use chrono::Local;
use sea_orm::DatabaseConnection;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const INI_HEADER: &str = "I\n1000 Version\nSCENERY\n\n";
const GLOBAL_AIRPORTS_ENABLED_METADATA_KEY: &str = "global_airports_enabled";
const GLOBAL_AIRPORTS_SORT_ORDER_METADATA_KEY: &str = "global_airports_sort_order";
const GLOBAL_AIRPORTS_CATEGORY_METADATA_KEY: &str = "global_airports_category";

/// Normalize a scenery path for scenery_packs.ini
/// Converts backslashes to forward slashes and ensures trailing slash
fn normalize_scenery_path(path: &str) -> String {
    // Convert all backslashes to forward slashes (X-Plane uses forward slashes)
    let normalized = path.replace('\\', "/");
    // Ensure trailing slash
    if normalized.ends_with('/') {
        normalized
    } else {
        format!("{}/", normalized)
    }
}

fn entry_path_for_ini(entry: &SceneryPackEntry) -> String {
    if entry.is_global_airports {
        entry.path.clone()
    } else {
        normalize_scenery_path(&entry.path)
    }
}

fn is_global_airports_package(info: &SceneryPackageInfo) -> bool {
    is_global_airports_folder_name(&info.folder_name)
}

fn infer_xplane_major_version_from_text(text: &str) -> Option<u8> {
    let lower = text.to_ascii_lowercase();
    for marker in ["x-plane 12", "x-plane12", "xp12"] {
        if lower.contains(marker) {
            return Some(12);
        }
    }
    for marker in ["x-plane 11", "x-plane11", "xp11"] {
        if lower.contains(marker) {
            return Some(11);
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn xplane_executable_major_version(exe_path: &Path) -> Option<u8> {
    use std::ffi::OsStr;
    use std::mem;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use winapi::shared::minwindef::{DWORD, LPVOID, UINT};
    use winapi::um::winver::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};

    #[repr(C)]
    struct VsFixedFileInfo {
        signature: DWORD,
        struct_version: DWORD,
        file_version_ms: DWORD,
        file_version_ls: DWORD,
        product_version_ms: DWORD,
        product_version_ls: DWORD,
        file_flags_mask: DWORD,
        file_flags: DWORD,
        file_os: DWORD,
        file_type: DWORD,
        file_subtype: DWORD,
        file_date_ms: DWORD,
        file_date_ls: DWORD,
    }

    let wide_path: Vec<u16> = exe_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut handle: DWORD = 0;
    let size = unsafe { GetFileVersionInfoSizeW(wide_path.as_ptr(), &mut handle) };
    if size == 0 {
        return None;
    }

    let mut buffer = vec![0u8; size as usize];
    let ok =
        unsafe { GetFileVersionInfoW(wide_path.as_ptr(), 0, size, buffer.as_mut_ptr() as LPVOID) };
    if ok == 0 {
        return None;
    }

    let root: Vec<u16> = OsStr::new("\\")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut version_info: LPVOID = ptr::null_mut();
    let mut len: UINT = 0;
    let ok = unsafe {
        VerQueryValueW(
            buffer.as_mut_ptr() as LPVOID,
            root.as_ptr(),
            &mut version_info,
            &mut len,
        )
    };
    if ok == 0 || version_info.is_null() || (len as usize) < mem::size_of::<VsFixedFileInfo>() {
        return None;
    }

    let fixed = unsafe { &*(version_info as *const VsFixedFileInfo) };
    if fixed.signature != 0xfeef04bd {
        return None;
    }

    Some((fixed.product_version_ms >> 16) as u8)
}

#[cfg(not(target_os = "windows"))]
fn xplane_executable_major_version(_exe_path: &Path) -> Option<u8> {
    None
}

fn find_xplane_executable_in_root(path: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let exact = path.join("X-Plane.exe");
        if exact.is_file() {
            return Some(exact);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let app_path = path.join("X-Plane.app");
        if app_path.exists() {
            return Some(app_path);
        }
    }

    let mut candidates: Vec<PathBuf> = fs::read_dir(path)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|entry_path| {
            let is_candidate = if cfg!(target_os = "macos") {
                entry_path.is_dir()
            } else {
                entry_path.is_file()
            };
            is_candidate
                && entry_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("X-Plane"))
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

fn detect_xplane_major_version(xplane_path: &Path) -> Option<u8> {
    if let Some(exe_path) = find_xplane_executable_in_root(xplane_path) {
        if let Some(version) = xplane_executable_major_version(&exe_path) {
            return Some(version);
        }

        if let Some(name) = exe_path.file_name().and_then(|name| name.to_str()) {
            if let Some(version) = infer_xplane_major_version_from_text(name) {
                return Some(version);
            }
        }
    }

    if let Some(name) = xplane_path.file_name().and_then(|name| name.to_str()) {
        if let Some(version) = infer_xplane_major_version_from_text(name) {
            return Some(version);
        }
    }

    fs::read_to_string(xplane_path.join("Log.txt"))
        .ok()
        .and_then(|content| infer_xplane_major_version_from_text(&content))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalAirportsState {
    pub enabled: bool,
    pub sort_order: u32,
    pub category: SceneryCategory,
}

fn sort_visible_entries<T>(entries: &mut [(u32, bool, T)]) {
    entries.sort_by(|(sort_a, global_a, _), (sort_b, global_b, _)| {
        sort_a.cmp(sort_b).then_with(|| global_b.cmp(global_a))
    });
}

fn build_entries_from_sorted_packages(
    packages: &[&SceneryPackageInfo],
    global_airports: &GlobalAirportsState,
) -> Vec<SceneryPackEntry> {
    let mut entries: Vec<(u32, bool, SceneryPackEntry)> = Vec::new();

    for info in packages {
        if is_global_airports_package(info) {
            continue;
        }

        // Skip disabled Unrecognized packages - they should not be written to scenery_packs.ini
        // Enabled Unrecognized packages WILL be written to ini
        if info.category == SceneryCategory::Unrecognized && !info.enabled {
            continue;
        }

        let path = if let Some(actual_path) = &info.actual_path {
            actual_path.clone()
        } else {
            format!("Custom Scenery/{}/", info.folder_name)
        };

        entries.push((
            info.sort_order,
            false,
            SceneryPackEntry {
                enabled: info.enabled,
                path,
                is_global_airports: false,
            },
        ));
    }

    entries.push((
        global_airports.sort_order,
        true,
        SceneryPackEntry {
            enabled: global_airports.enabled,
            path: GLOBAL_AIRPORTS_ENTRY_NAME.to_string(),
            is_global_airports: true,
        },
    ));

    sort_visible_entries(&mut entries);
    entries.into_iter().map(|(_, _, entry)| entry).collect()
}

fn entries_match_expected(
    ini_entries: &[SceneryPackEntry],
    expected_entries: &[SceneryPackEntry],
) -> bool {
    if ini_entries.len() != expected_entries.len() {
        return false;
    }

    ini_entries
        .iter()
        .zip(expected_entries.iter())
        .all(|(ini_entry, expected_entry)| {
            ini_entry.enabled == expected_entry.enabled
                && ini_entry.is_global_airports == expected_entry.is_global_airports
                && entry_path_for_ini(ini_entry) == entry_path_for_ini(expected_entry)
        })
}

fn global_airports_default_priority(is_xplane_12: bool) -> u8 {
    if is_xplane_12 {
        SceneryCategory::RegionalOverlay.priority()
    } else {
        SceneryCategory::DefaultAirport.priority()
    }
}

fn parse_ini_entries(content: &str) -> Vec<SceneryPackEntry> {
    let mut entries = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        let (enabled, path) = if let Some(path) = line.strip_prefix("SCENERY_PACK_DISABLED ") {
            (false, path)
        } else if let Some(path) = line.strip_prefix("SCENERY_PACK ") {
            (true, path)
        } else {
            continue;
        };

        let is_global_airports = path.contains("*GLOBAL_AIRPORTS*");
        entries.push(SceneryPackEntry {
            enabled,
            path: if is_global_airports {
                GLOBAL_AIRPORTS_ENTRY_NAME.to_string()
            } else {
                path.trim().to_string()
            },
            is_global_airports,
        });
    }

    entries
}

fn ini_path_key(xplane_path: &Path, path: &str) -> String {
    if path.contains(GLOBAL_AIRPORTS_ENTRY_NAME) {
        return GLOBAL_AIRPORTS_ENTRY_NAME.to_string();
    }
    let normalized = path.trim().replace('\\', "/");
    let normalized = normalized.trim_start_matches("./").trim_end_matches('/');
    let absolute =
        if Path::new(normalized).is_absolute() || normalized.as_bytes().get(1) == Some(&b':') {
            normalized.to_string()
        } else {
            format!(
                "{}/{}",
                xplane_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .trim_end_matches('/'),
                normalized
            )
        };
    if cfg!(windows) {
        absolute.to_lowercase()
    } else {
        absolute
    }
}

fn package_ini_path(info: &SceneryPackageInfo) -> String {
    info.actual_path
        .clone()
        .unwrap_or_else(|| format!("Custom Scenery/{}/", info.folder_name))
}

/// Import the file's order and switches; never classify-sort its entries.
fn import_ini_order(
    xplane_path: &Path,
    content: &str,
    index: &mut SceneryIndex,
) -> (Option<(bool, u32)>, Vec<SceneryEntryUpdate>) {
    let entries = parse_ini_entries(content);
    let mut states = HashMap::new();
    for (position, entry) in entries.iter().enumerate() {
        states
            .entry(ini_path_key(xplane_path, &entry.path))
            .or_insert((entry.enabled, position as u32));
    }
    let mut packages: Vec<_> = index.packages.values_mut().collect();
    packages.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then_with(|| a.folder_name.cmp(&b.folder_name))
    });
    let mut next = entries.len() as u32;
    let mut changes = Vec::new();
    for info in packages {
        let before = (info.enabled, info.sort_order);
        if let Some(&(enabled, order)) =
            states.get(&ini_path_key(xplane_path, &package_ini_path(info)))
        {
            info.enabled = enabled;
            info.sort_order = order;
        } else {
            info.sort_order = next;
            next += 1;
        }
        if before != (info.enabled, info.sort_order) {
            changes.push(SceneryEntryUpdate {
                folder_name: info.folder_name.clone(),
                enabled: info.enabled,
                sort_order: info.sort_order,
                category: None,
            });
        }
    }
    (states.get(GLOBAL_AIRPORTS_ENTRY_NAME).copied(), changes)
}

/// Patch switches or remove named paths while preserving all other lines byte-for-byte.
fn patch_ini_without_reordering(
    xplane_path: &Path,
    content: &str,
    updates: &[SceneryPackEntry],
    removed: &HashSet<String>,
) -> String {
    let updates_by_path: HashMap<_, _> = updates
        .iter()
        .map(|entry| (ini_path_key(xplane_path, &entry.path), entry))
        .collect();
    let mut seen = HashSet::new();
    let mut output = String::new();
    for line in content.split_inclusive('\n') {
        let parsed = parse_ini_entries(line);
        let Some(entry) = parsed.first() else {
            output.push_str(line);
            continue;
        };
        let key = ini_path_key(xplane_path, &entry.path);
        if removed.contains(&key) {
            continue;
        }
        seen.insert(key.clone());
        if let Some(update) = updates_by_path
            .get(&key)
            .filter(|update| update.enabled != entry.enabled)
        {
            let trimmed = line.trim_start();
            output.push_str(&line[..line.len() - trimmed.len()]);
            let old_prefix = if entry.enabled {
                "SCENERY_PACK "
            } else {
                "SCENERY_PACK_DISABLED "
            };
            output.push_str(if update.enabled {
                "SCENERY_PACK "
            } else {
                "SCENERY_PACK_DISABLED "
            });
            output.push_str(trimmed.strip_prefix(old_prefix).unwrap());
        } else {
            output.push_str(line);
        }
    }
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    if output.is_empty() && content.is_empty() {
        output.push_str(INI_HEADER);
    }
    for entry in updates {
        let key = ini_path_key(xplane_path, &entry.path);
        if removed.contains(&key) || !seen.insert(key) {
            continue;
        }
        if !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(if entry.enabled {
            "SCENERY_PACK "
        } else {
            "SCENERY_PACK_DISABLED "
        });
        output.push_str(&entry_path_for_ini(entry));
        output.push_str(newline);
    }
    output
}

/// Manager for scenery_packs.ini operations
pub struct SceneryPacksManager {
    xplane_path: PathBuf,
    ini_path: PathBuf,
    db: DatabaseConnection,
}

impl SceneryPacksManager {
    /// Create a new manager
    pub fn new(xplane_path: &Path, db: DatabaseConnection) -> Self {
        let ini_path = xplane_path.join("Custom Scenery").join("scenery_packs.ini");
        Self {
            xplane_path: xplane_path.to_path_buf(),
            ini_path,
            db,
        }
    }

    fn default_global_airports_enabled(packages: &[&SceneryPackageInfo]) -> bool {
        packages
            .iter()
            .find(|info| is_global_airports_package(info))
            .map(|info| info.enabled)
            .unwrap_or(true)
    }

    fn default_global_airports_sort_order(
        packages: &[&SceneryPackageInfo],
        is_xplane_12: bool,
    ) -> u32 {
        let mut visible_packages: Vec<_> = packages
            .iter()
            .copied()
            .filter(|info| {
                !is_global_airports_package(info)
                    && !(info.category == SceneryCategory::Unrecognized && !info.enabled)
            })
            .collect();
        visible_packages.sort_by_key(|info| info.sort_order);

        visible_packages
            .iter()
            .position(|info| {
                info.category.priority() >= global_airports_default_priority(is_xplane_12)
            })
            .unwrap_or(visible_packages.len()) as u32
    }

    fn default_global_airports_category() -> SceneryCategory {
        SceneryCategory::DefaultAirport
    }

    pub async fn get_global_airports_state_for_packages(
        &self,
        packages: &[&SceneryPackageInfo],
    ) -> Result<GlobalAirportsState> {
        let is_xplane_12 = matches!(detect_xplane_major_version(&self.xplane_path), Some(12));
        let enabled = match SceneryQueries::get_metadata(
            &self.db,
            GLOBAL_AIRPORTS_ENABLED_METADATA_KEY,
        )
        .await
        {
            Ok(Some(value)) => !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no"
            ),
            Ok(None) => Self::default_global_airports_enabled(packages),
            Err(error) => {
                logger::log_info(
                    &format!("Failed to load Global Airports enabled metadata: {}", error),
                    Some("scenery_packs"),
                );
                Self::default_global_airports_enabled(packages)
            }
        };

        let sort_order =
            match SceneryQueries::get_metadata(&self.db, GLOBAL_AIRPORTS_SORT_ORDER_METADATA_KEY)
                .await
            {
                Ok(Some(value)) => value.trim().parse::<u32>().unwrap_or_else(|_| {
                    Self::default_global_airports_sort_order(packages, is_xplane_12)
                }),
                Ok(None) => Self::default_global_airports_sort_order(packages, is_xplane_12),
                Err(error) => {
                    logger::log_info(
                        &format!("Failed to load Global Airports sort metadata: {}", error),
                        Some("scenery_packs"),
                    );
                    Self::default_global_airports_sort_order(packages, is_xplane_12)
                }
            };

        let category =
            match SceneryQueries::get_metadata(&self.db, GLOBAL_AIRPORTS_CATEGORY_METADATA_KEY)
                .await
            {
                Ok(Some(value)) => match value.trim() {
                    "FixedHighPriority" => SceneryCategory::FixedHighPriority,
                    "Airport" => SceneryCategory::Airport,
                    "DefaultAirport" => SceneryCategory::DefaultAirport,
                    "RegionalOverlay" => SceneryCategory::RegionalOverlay,
                    "Library" => SceneryCategory::Library,
                    "Overlay" => SceneryCategory::Overlay,
                    "AirportMesh" => SceneryCategory::AirportMesh,
                    "Mesh" => SceneryCategory::Mesh,
                    "Other" => SceneryCategory::Other,
                    "Unrecognized" => SceneryCategory::Unrecognized,
                    _ => Self::default_global_airports_category(),
                },
                Ok(None) => Self::default_global_airports_category(),
                Err(error) => {
                    logger::log_info(
                        &format!(
                            "Failed to load Global Airports category metadata: {}",
                            error
                        ),
                        Some("scenery_packs"),
                    );
                    Self::default_global_airports_category()
                }
            };

        Ok(GlobalAirportsState {
            enabled,
            sort_order,
            category,
        })
    }

    pub async fn set_global_airports_enabled(&self, enabled: bool) -> Result<()> {
        SceneryQueries::set_metadata(
            &self.db,
            GLOBAL_AIRPORTS_ENABLED_METADATA_KEY,
            if enabled { "true" } else { "false" },
        )
        .await
        .map_err(|e| anyhow!("{}", e))
    }

    pub async fn set_global_airports_sort_order(&self, sort_order: u32) -> Result<()> {
        SceneryQueries::set_metadata(
            &self.db,
            GLOBAL_AIRPORTS_SORT_ORDER_METADATA_KEY,
            &sort_order.to_string(),
        )
        .await
        .map_err(|e| anyhow!("{}", e))
    }

    pub async fn set_global_airports_category(&self, category: &SceneryCategory) -> Result<()> {
        let value = match category {
            SceneryCategory::FixedHighPriority => "FixedHighPriority",
            SceneryCategory::Airport => "Airport",
            SceneryCategory::DefaultAirport => "DefaultAirport",
            SceneryCategory::RegionalOverlay => "RegionalOverlay",
            SceneryCategory::Library => "Library",
            SceneryCategory::Overlay => "Overlay",
            SceneryCategory::AirportMesh => "AirportMesh",
            SceneryCategory::Mesh => "Mesh",
            SceneryCategory::Other => "Other",
            SceneryCategory::Unrecognized => "Unrecognized",
        };

        SceneryQueries::set_metadata(&self.db, GLOBAL_AIRPORTS_CATEGORY_METADATA_KEY, value)
            .await
            .map_err(|e| anyhow!("{}", e))
    }

    pub async fn reset_global_airports_position_to_default(&self) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());
        let index = index_manager.load_index().await?;
        let packages: Vec<_> = index.packages.values().collect();
        let is_xplane_12 = matches!(detect_xplane_major_version(&self.xplane_path), Some(12));
        let default_sort_order = Self::default_global_airports_sort_order(&packages, is_xplane_12);
        self.set_global_airports_sort_order(default_sort_order)
            .await?;
        self.set_global_airports_category(&SceneryCategory::DefaultAirport)
            .await
    }

    fn write_ini_at_path(ini_path: &Path, entries: &[SceneryPackEntry]) -> Result<()> {
        // Build content in memory first so we can retry with a fallback strategy
        let mut content: Vec<u8> = Vec::new();
        content.extend_from_slice(INI_HEADER.as_bytes());
        for entry in entries {
            let prefix = if entry.enabled {
                "SCENERY_PACK"
            } else {
                "SCENERY_PACK_DISABLED"
            };
            let path = entry_path_for_ini(entry);
            content.extend_from_slice(format!("{} {}\n", prefix, path).as_bytes());
        }
        Self::write_content_at_path(ini_path, &content)
    }

    fn write_content_at_path(ini_path: &Path, content: &[u8]) -> Result<()> {
        Self::ensure_ini_parent_dir(ini_path)?;

        // Clear anything that might block the write (read-only bit, macOS uchg flag)
        prepare_path_for_write(ini_path);

        // Strategy 1: atomic write via temp file + rename (preferred)
        let temp_path = ini_path.with_extension("ini.tmp");
        let atomic_result = (|| -> std::io::Result<()> {
            let mut file = fs::File::create(&temp_path)?;
            file.write_all(&content)?;
            file.flush()?;
            drop(file);
            fs::rename(&temp_path, ini_path)?;
            Ok(())
        })();

        if atomic_result.is_ok() {
            return Ok(());
        }
        // Clean up temp file if rename failed
        let _ = fs::remove_file(&temp_path);

        // Strategy 2: direct overwrite (works when rename is blocked but file itself is writable,
        // e.g. macOS directory permission edge cases or X-Plane holding a rename lock)
        prepare_path_for_write(ini_path);
        if let Err(direct_err) = fs::write(ini_path, &content) {
            let hint = if cfg!(target_os = "macos") {
                " If X-Plane is running, close it and try again. Otherwise: the file may be flagged \
                 immutable (run `chflags nouchg <path>` in Terminal), the app may need Full Disk Access \
                 (System Settings → Privacy & Security → Full Disk Access), or check Custom Scenery > \
                 Get Info > Sharing & Permissions."
            } else {
                " Make sure X-Plane is not running, and that the Custom Scenery folder is writable."
            };
            return Err(anyhow!("{}{}", direct_err, hint));
        }

        Ok(())
    }

    fn ensure_ini_parent_dir(ini_path: &Path) -> Result<()> {
        let parent = ini_path.parent().ok_or_else(|| {
            anyhow!(
                "Invalid ini path: no parent directory ({})",
                ini_path.display()
            )
        })?;

        fs::create_dir_all(parent).map_err(|e| {
            let hint = if e.to_string().contains("failed to create whole tree") {
                " This usually means the configured X-Plane path is invalid (for example drive-relative like 'E:'), or the drive is unavailable."
            } else {
                ""
            };
            anyhow!(
                "Failed to prepare Custom Scenery directory '{}': {}{}",
                parent.display(),
                e,
                hint
            )
        })
    }

    /// Write sorted entries back to scenery_packs.ini
    #[allow(dead_code)]
    pub fn write_ini(&self, entries: &[SceneryPackEntry]) -> Result<()> {
        Self::write_ini_at_path(&self.ini_path, entries)
    }

    fn backup_ini_at_path(ini_path: &Path) -> Result<PathBuf> {
        if !ini_path.exists() {
            return Err(anyhow!("scenery_packs.ini does not exist"));
        }

        let parent_dir = ini_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid ini path: no parent directory"))?;

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("scenery_packs.ini.backup.{}", timestamp);
        let backup_path = parent_dir.join(&backup_name);

        prepare_path_for_write(ini_path);
        fs::rename(ini_path, &backup_path)?;
        logger::log_info(
            &format!("Created backup: {:?}", backup_path),
            Some("scenery_packs"),
        );

        // Clean up old backups, keeping only the 3 most recent
        Self::cleanup_old_backups(parent_dir, 3);

        Ok(backup_path)
    }

    /// Create a backup of scenery_packs.ini
    #[allow(dead_code)]
    pub fn backup_ini(&self) -> Result<PathBuf> {
        Self::backup_ini_at_path(&self.ini_path)
    }

    /// Remove old backup files, keeping only the specified number of most recent backups
    fn cleanup_old_backups(dir: &std::path::Path, keep_count: usize) {
        let mut backups: Vec<_> = match fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("scenery_packs.ini.backup.")
                })
                .collect(),
            Err(_) => return,
        };

        if backups.len() <= keep_count {
            return;
        }

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| {
            let time_a = a.metadata().and_then(|m| m.modified()).ok();
            let time_b = b.metadata().and_then(|m| m.modified()).ok();
            time_b.cmp(&time_a)
        });

        // Remove old backups (skip the first `keep_count`)
        for backup in backups.into_iter().skip(keep_count) {
            if let Err(e) = fs::remove_file(backup.path()) {
                logger::log_info(
                    &format!("Failed to remove old backup {:?}: {}", backup.path(), e),
                    Some("scenery_packs"),
                );
            } else {
                logger::log_info(
                    &format!("Removed old backup: {:?}", backup.path()),
                    Some("scenery_packs"),
                );
            }
        }
    }

    /// Add a new entry to scenery_packs.ini (used after installation)
    #[allow(dead_code)]
    pub async fn add_entry(&self, folder_name: &str, category: &SceneryCategory) -> Result<()> {
        self.add_entry_with_locked_entries(folder_name, category, &[])
            .await
    }

    /// Add a new entry to scenery_packs.ini while preserving locked scenery sort positions.
    pub async fn add_entry_with_locked_entries(
        &self,
        folder_name: &str,
        category: &SceneryCategory,
        locked_folder_names: &[String],
    ) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());

        // If index hasn't been created yet, don't add to index or sort
        // User hasn't built the index, so we shouldn't automatically manage scenery order
        if !index_manager.has_index().await? {
            logger::log_info(
                &format!(
                    "Skipping scenery indexing for '{}': index not yet created",
                    folder_name
                ),
                Some("scenery_packs"),
            );
            return Ok(());
        }

        let folder_path = self.xplane_path.join("Custom Scenery").join(folder_name);

        let info = index_manager.get_or_classify(&folder_path).await?;
        if &info.category != category {
            index_manager
                .update_entry(folder_name, None, None, Some(category.clone()))
                .await?;
        }

        if !crate::scenery_sorting::auto_sort_enabled() {
            index_manager.update_index().await?;
            return self.apply_preserving_order().await;
        }

        let _ = index_manager
            .reset_sort_order_with_locked_entries(locked_folder_names.to_vec())
            .await?;
        self.auto_sort_from_index().await
    }

    /// Ensure all installed scenery is in scenery_packs.ini
    /// Only performs incremental indexing if the index has been created
    pub async fn sync_with_folder(&self) -> Result<usize> {
        let custom_scenery_path = self.xplane_path.join("Custom Scenery");
        if !custom_scenery_path.exists() {
            return Ok(0);
        }

        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());

        // If index hasn't been created yet, don't perform incremental indexing
        // User hasn't built the index, so we shouldn't automatically update it
        if !index_manager.has_index().await? {
            logger::log_info(
                "Skipping sync_with_folder: index not yet created",
                Some("scenery_packs"),
            );
            return Ok(0);
        }

        let before_index = index_manager.load_index().await?;
        let before_keys: std::collections::HashSet<String> =
            before_index.packages.keys().cloned().collect();

        let updated_index = index_manager.update_index().await?;
        let after_keys: std::collections::HashSet<String> =
            updated_index.packages.keys().cloned().collect();

        let added_count = after_keys.difference(&before_keys).count();

        if before_keys != after_keys {
            self.auto_sort_from_index().await?;
        }

        Ok(added_count)
    }

    /// Sort scenery_packs.ini based entirely on index sort_order
    /// This is used by the scenery manager after manual reordering
    pub async fn auto_sort_from_index(&self) -> Result<()> {
        if !crate::scenery_sorting::auto_sort_enabled() {
            return self.apply_preserving_order().await;
        }
        self.write_order_from_index(true).await
    }

    /// Explicit manual ordering is independent of the automatic sorting preference.
    pub async fn apply_order_from_index(&self) -> Result<()> {
        self.write_order_from_index(false).await
    }

    async fn write_order_from_index(&self, automatic: bool) -> Result<()> {
        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());
        let index = index_manager.load_index().await?;

        if index.packages.is_empty() {
            logger::log_info(
                "No scenery packages in index, nothing to sort",
                Some("scenery_packs"),
            );
            return Ok(());
        }

        if automatic && !crate::scenery_sorting::auto_sort_enabled() {
            return self.apply_preserving_order().await;
        }

        // Create backup if ini exists
        if self.ini_path.exists() {
            let ini_path = self.ini_path.clone();
            if let Err(e) = tokio::task::spawn_blocking(move || Self::backup_ini_at_path(&ini_path))
                .await
                .map_err(|e| anyhow!("Blocking task failed: {}", e))?
            {
                logger::log_info(
                    &format!("Failed to create backup: {}", e),
                    Some("scenery_packs"),
                );
            }
        }

        let mut packages: Vec<_> = index.packages.values().collect();
        packages.sort_by_key(|p| p.sort_order);
        let global_airports = self
            .get_global_airports_state_for_packages(&packages)
            .await?;
        let entries = build_entries_from_sorted_packages(&packages, &global_airports);

        // Write sorted entries
        let ini_path = self.ini_path.clone();
        let entries_len = entries.len();
        tokio::task::spawn_blocking(move || Self::write_ini_at_path(&ini_path, &entries))
            .await
            .map_err(|e| anyhow!("Blocking task failed: {}", e))??;

        logger::log_info(
            &format!("Sorted {} scenery entries from index", entries_len),
            Some("scenery_packs"),
        );

        Ok(())
    }

    /// Apply index state (enabled/sort_order) to scenery_packs.ini
    /// This preserves the order from the index and applies enabled states
    pub async fn apply_from_index(&self) -> Result<()> {
        // This is essentially the same as auto_sort_from_index
        // but we call it explicitly to make the intent clear
        self.auto_sort_from_index().await
    }

    pub async fn import_file_state(
        &self,
        index: &mut SceneryIndex,
    ) -> Result<Vec<SceneryEntryUpdate>> {
        let content = match fs::read_to_string(&self.ini_path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        let (global, changes) = import_ini_order(&self.xplane_path, &content, index);
        if let Some((enabled, order)) = global {
            for (key, value) in [
                (
                    GLOBAL_AIRPORTS_ENABLED_METADATA_KEY,
                    if enabled { "1" } else { "0" }.to_string(),
                ),
                (GLOBAL_AIRPORTS_SORT_ORDER_METADATA_KEY, order.to_string()),
            ] {
                let existing = SceneryQueries::get_metadata(&self.db, key)
                    .await
                    .map_err(|error| anyhow!("{error}"))?;
                if existing.as_deref() != Some(value.as_str()) {
                    SceneryQueries::set_metadata(&self.db, key, &value)
                        .await
                        .map_err(|error| anyhow!("{error}"))?;
                }
            }
        }
        Ok(changes)
    }

    pub async fn apply_preserving_order(&self) -> Result<()> {
        let index = SceneryIndexManager::new(&self.xplane_path, self.db.clone())
            .load_index()
            .await?;
        let mut packages: Vec<_> = index.packages.values().collect();
        packages.sort_by(|a, b| {
            a.sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.folder_name.cmp(&b.folder_name))
        });
        let global = self
            .get_global_airports_state_for_packages(&packages)
            .await?;
        let mut updates: Vec<_> = packages
            .into_iter()
            .filter(|info| !is_global_airports_package(info))
            .map(|info| SceneryPackEntry {
                enabled: info.enabled,
                path: package_ini_path(info),
                is_global_airports: false,
            })
            .collect();
        updates.push(SceneryPackEntry {
            enabled: global.enabled,
            path: GLOBAL_AIRPORTS_ENTRY_NAME.to_string(),
            is_global_airports: true,
        });
        self.patch_file_preserving_order(updates, HashSet::new())
            .await
    }

    pub async fn remove_entry_from_ini(&self, folder_name: &str) -> Result<()> {
        let info = SceneryIndexManager::new(&self.xplane_path, self.db.clone())
            .get_package(folder_name)
            .await?;
        let mut removed = HashSet::from([ini_path_key(
            &self.xplane_path,
            &format!("Custom Scenery/{folder_name}/"),
        )]);
        if let Some(info) = info {
            removed.insert(ini_path_key(&self.xplane_path, &package_ini_path(&info)));
        }
        self.patch_file_preserving_order(Vec::new(), removed).await
    }

    async fn patch_file_preserving_order(
        &self,
        updates: Vec<SceneryPackEntry>,
        removed: HashSet<String>,
    ) -> Result<()> {
        let ini_path = self.ini_path.clone();
        let xplane_path = self.xplane_path.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let content = match fs::read_to_string(&ini_path) {
                Ok(content) => content,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(error) => return Err(error.into()),
            };
            if content.is_empty() && updates.is_empty() {
                return Ok(());
            }
            let patched = patch_ini_without_reordering(&xplane_path, &content, &updates, &removed);
            if patched == content {
                return Ok(());
            }
            if ini_path.exists() {
                Self::backup_ini_at_path(&ini_path)?;
            }
            Self::write_content_at_path(&ini_path, patched.as_bytes())
        })
        .await
        .map_err(|error| anyhow!("Blocking task failed: {error}"))?
    }

    /// Check if ini file is in sync with the index
    /// Returns true if ini order/enabled states match the entries generated from the index.
    pub async fn is_synced_with_index(&self) -> Result<bool> {
        // If ini doesn't exist, it's not synced
        if !self.ini_path.exists() {
            return Ok(false);
        }

        let index_manager = SceneryIndexManager::new(&self.xplane_path, self.db.clone());
        let index = index_manager.load_index().await?;

        if index.packages.is_empty() {
            return Ok(true);
        }

        // Read current ini file
        let ini_path = self.ini_path.clone();
        let content = tokio::task::spawn_blocking(move || fs::read_to_string(&ini_path))
            .await
            .map_err(|e| anyhow!("Blocking task failed: {}", e))??;

        let mut packages: Vec<_> = index.packages.values().collect();
        packages.sort_by_key(|p| p.sort_order);
        let global_airports = self
            .get_global_airports_state_for_packages(&packages)
            .await?;
        let expected_entries = build_entries_from_sorted_packages(&packages, &global_airports);
        let ini_entries = parse_ini_entries(&content);

        if !crate::scenery_sorting::auto_sort_enabled() {
            let actual: HashMap<_, _> = ini_entries
                .iter()
                .map(|entry| (ini_path_key(&self.xplane_path, &entry.path), entry.enabled))
                .collect();
            return Ok(expected_entries.iter().all(|entry| {
                actual.get(&ini_path_key(&self.xplane_path, &entry.path)) == Some(&entry.enabled)
            }));
        }

        Ok(entries_match_expected(&ini_entries, &expected_entries))
    }
}

/// Best-effort clearing of attributes that commonly block writes to `scenery_packs.ini`.
/// All failures are silently ignored — this is a pre-write sweetener, not a hard requirement.
///
/// Covers:
/// - The POSIX/Windows read-only bit.
/// - On macOS, the user-immutable flag (`UF_IMMUTABLE`) set via `chflags uchg`, which causes
///   EPERM (os error 1) on rename/write even when the file is otherwise owner-writable.
fn prepare_path_for_write(path: &Path) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };

    #[cfg(target_os = "windows")]
    #[allow(clippy::permissions_set_readonly_false)]
    {
        let mut perms = metadata.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            let _ = fs::set_permissions(path, perms);
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o200);
        let _ = fs::set_permissions(path, permissions);
    }

    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        if let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) {
            // SAFETY: c_path lives for the duration of the call. flags=0 clears all BSD flags;
            // EPERM on failure is expected when the caller doesn't own the file and is ignored.
            unsafe {
                libc::chflags(c_path.as_ptr(), 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn registration(path: &str, enabled: bool) -> SceneryPackEntry {
        SceneryPackEntry {
            enabled,
            path: path.to_string(),
            is_global_airports: path == GLOBAL_AIRPORTS_ENTRY_NAME,
        }
    }

    #[test]
    fn patches_switches_without_reordering_or_removing_unmanaged_lines() {
        let root = Path::new("/sim");
        let before = "I\r\n1000 Version\r\nSCENERY\r\n# hand ordered\r\nSCENERY_PACK Custom Scenery/Z Mesh/\r\nSCENERY_PACK *GLOBAL_AIRPORTS*\r\nSCENERY_PACK_DISABLED Custom Scenery/A Airport/\r\nSCENERY_PACK E:/External/Unknown/\r\n";
        let output = patch_ini_without_reordering(
            root,
            before,
            &[
                registration("Custom Scenery/A Airport/", true),
                registration("Custom Scenery/Z Mesh/", false),
                registration(GLOBAL_AIRPORTS_ENTRY_NAME, false),
            ],
            &HashSet::new(),
        );
        assert_eq!(
            output,
            before
                .replace(
                    "SCENERY_PACK Custom Scenery/Z Mesh/",
                    "SCENERY_PACK_DISABLED Custom Scenery/Z Mesh/"
                )
                .replace(
                    "SCENERY_PACK *GLOBAL_AIRPORTS*",
                    "SCENERY_PACK_DISABLED *GLOBAL_AIRPORTS*"
                )
                .replace(
                    "SCENERY_PACK_DISABLED Custom Scenery/A Airport/",
                    "SCENERY_PACK Custom Scenery/A Airport/"
                )
        );
    }

    #[test]
    fn appends_new_entries_and_matches_external_paths_without_duplicate_registrations() {
        let root = Path::new("/sim");
        let before = "SCENERY_PACK E:\\External\\Pack\\\nSCENERY_PACK Custom Scenery/Z Mesh/";
        let output = patch_ini_without_reordering(
            root,
            before,
            &[
                registration("E:/External/Pack/", true),
                registration("Custom Scenery/New Airport/", true),
            ],
            &HashSet::new(),
        );
        assert_eq!(
            output,
            format!("{before}\nSCENERY_PACK Custom Scenery/New Airport/\n")
        );
    }

    #[test]
    fn deleting_one_path_keeps_comments_and_remaining_order_exactly() {
        let root = Path::new("/sim");
        let before = "# custom\nSCENERY_PACK Custom Scenery/Keep/\nSCENERY_PACK_DISABLED E:/External/Delete/\nSCENERY_PACK *GLOBAL_AIRPORTS*\n";
        let removed = HashSet::from([ini_path_key(root, "E:\\External\\Delete\\")]);
        assert_eq!(
            patch_ini_without_reordering(root, before, &[], &removed),
            before.replace("SCENERY_PACK_DISABLED E:/External/Delete/\n", "")
        );
    }

    #[test]
    fn auto_sort_off_still_allows_explicit_manual_order() {
        let temp = tempfile::tempdir().unwrap();
        let db = crate::database::open_memory_connection().unwrap();
        crate::database::apply_migrations(&db).unwrap();
        let index_manager = SceneryIndexManager::new(temp.path(), db.clone());
        let index = SceneryIndex {
            version: crate::database::CURRENT_SCHEMA_VERSION as u32,
            packages: HashMap::from([
                (
                    "Z Mesh".to_string(),
                    make_package("Z Mesh", SceneryCategory::Mesh, 0, true),
                ),
                (
                    "A Airport".to_string(),
                    make_package("A Airport", SceneryCategory::Airport, 1, true),
                ),
            ]),
            last_updated: SystemTime::UNIX_EPOCH,
        };
        let packs = SceneryPacksManager::new(temp.path(), db);
        fs::create_dir_all(temp.path().join("Custom Scenery")).unwrap();
        let before = "# user order\nSCENERY_PACK Custom Scenery/A Airport/\nSCENERY_PACK *GLOBAL_AIRPORTS*\nSCENERY_PACK Custom Scenery/Z Mesh/\n";
        fs::write(&packs.ini_path, before).unwrap();
        tauri::async_runtime::block_on(async {
            index_manager.save_index(&index).await.unwrap();
            packs.auto_sort_from_index().await.unwrap();
            assert_eq!(fs::read_to_string(&packs.ini_path).unwrap(), before);
            packs.apply_order_from_index().await.unwrap();
            let after = parse_ini_entries(&fs::read_to_string(&packs.ini_path).unwrap());
            let names: Vec<_> = after
                .iter()
                .filter(|entry| !entry.is_global_airports)
                .map(|entry| entry.path.as_str())
                .collect();
            assert_eq!(
                names,
                vec!["Custom Scenery/Z Mesh/", "Custom Scenery/A Airport/"]
            );
        });
    }

    #[cfg(unix)]
    #[test]
    fn prepare_path_for_write_adds_only_owner_write_permission() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("scenery_packs.ini");
        fs::write(&file, "SCENERY_PACK Custom Scenery/Test/\n").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o444)).unwrap();

        prepare_path_for_write(&file);

        assert_eq!(
            fs::metadata(file).unwrap().permissions().mode() & 0o777,
            0o644
        );
    }

    fn global_airports_state(
        enabled: bool,
        sort_order: u32,
        category: SceneryCategory,
    ) -> GlobalAirportsState {
        GlobalAirportsState {
            enabled,
            sort_order,
            category,
        }
    }

    fn make_package(
        folder_name: &str,
        category: SceneryCategory,
        sort_order: u32,
        enabled: bool,
    ) -> SceneryPackageInfo {
        SceneryPackageInfo {
            folder_name: folder_name.to_string(),
            category,
            sub_priority: 0,
            last_modified: SystemTime::UNIX_EPOCH,
            has_apt_dat: false,
            airport_id: None,
            has_dsf: false,
            has_library_txt: false,
            has_textures: false,
            has_objects: false,
            texture_count: 0,
            earth_nav_tile_count: 0,
            indexed_at: SystemTime::UNIX_EPOCH,
            required_libraries: Vec::new(),
            missing_libraries: Vec::new(),
            exported_library_names: Vec::new(),
            enabled,
            sort_order,
            actual_path: None,
            continent: None,
            original_category: None,
        }
    }

    #[test]
    fn test_category_priority_order() {
        // Verify priority order matches design
        assert!(
            SceneryCategory::FixedHighPriority.priority() < SceneryCategory::Airport.priority()
        );
        assert!(SceneryCategory::Airport.priority() < SceneryCategory::DefaultAirport.priority());
        assert!(SceneryCategory::DefaultAirport.priority() < SceneryCategory::Other.priority());
        assert!(SceneryCategory::Other.priority() < SceneryCategory::RegionalOverlay.priority());
        assert!(SceneryCategory::RegionalOverlay.priority() < SceneryCategory::Library.priority());
        assert!(SceneryCategory::Library.priority() < SceneryCategory::Overlay.priority());
        assert!(SceneryCategory::Overlay.priority() < SceneryCategory::AirportMesh.priority());
        assert!(SceneryCategory::AirportMesh.priority() < SceneryCategory::Mesh.priority());
        assert!(SceneryCategory::Mesh.priority() < SceneryCategory::Unrecognized.priority());
    }

    #[test]
    fn writes_global_airports_once_at_default_airport_position() {
        let airport = make_package("KSEA Demo", SceneryCategory::Airport, 0, true);
        let global_airports =
            make_package("Global Airports", SceneryCategory::DefaultAirport, 1, true);
        let library = make_package("MisterX Library", SceneryCategory::Library, 2, true);
        let packages = vec![&airport, &global_airports, &library];

        let entries = build_entries_from_sorted_packages(
            &packages,
            &global_airports_state(true, 1, SceneryCategory::DefaultAirport),
        );

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, "Custom Scenery/KSEA Demo/");
        assert!(entries[1].is_global_airports);
        assert_eq!(entries[1].path, "*GLOBAL_AIRPORTS*");
        assert_eq!(entries[2].path, "Custom Scenery/MisterX Library/");
        assert!(!entries
            .iter()
            .any(|entry| { !entry.is_global_airports && entry.path.contains("Global Airports") }));
    }

    #[test]
    fn ignores_global_airports_package_sort_order_for_special_entry_position() {
        let global_airports =
            make_package("Global Airports", SceneryCategory::DefaultAirport, 0, true);
        let airport = make_package("Airport A", SceneryCategory::Airport, 0, true);
        let default_airport = make_package(
            "X-Plane Airports - EGPR Barra",
            SceneryCategory::DefaultAirport,
            1,
            true,
        );
        let library = make_package("Library A", SceneryCategory::Library, 2, true);
        let packages = vec![&global_airports, &airport, &default_airport, &library];

        let entries = build_entries_from_sorted_packages(
            &packages,
            &global_airports_state(true, 1, SceneryCategory::DefaultAirport),
        );

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].path, "Custom Scenery/Airport A/");
        assert!(entries[1].is_global_airports);
        assert_eq!(
            entries[2].path,
            "Custom Scenery/X-Plane Airports - EGPR Barra/"
        );
        assert_eq!(entries[3].path, "Custom Scenery/Library A/");
    }

    #[test]
    fn treats_misclassified_global_airports_folder_as_special_marker() {
        let airport = make_package("Airport A", SceneryCategory::Airport, 0, true);
        let global_airports = make_package("Global Airports", SceneryCategory::Airport, 1, false);
        let overlay = make_package("Overlay Pack", SceneryCategory::Overlay, 2, true);
        let packages = vec![&airport, &global_airports, &overlay];

        let entries = build_entries_from_sorted_packages(
            &packages,
            &global_airports_state(false, 1, SceneryCategory::DefaultAirport),
        );

        assert_eq!(entries.len(), 3);
        assert!(entries[1].is_global_airports);
        assert!(!entries[1].enabled);
        assert_eq!(entries[2].path, "Custom Scenery/Overlay Pack/");
    }

    #[test]
    fn places_global_airports_before_libraries_for_xplane_11() {
        let airport = make_package("Airport A", SceneryCategory::Airport, 0, true);
        let default_airport = make_package(
            "X-Plane Airports - EGPR Barra",
            SceneryCategory::DefaultAirport,
            1,
            true,
        );
        let library = make_package("Library A", SceneryCategory::Library, 2, true);
        let overlay = make_package("Overlay A", SceneryCategory::Overlay, 3, true);
        let packages = vec![&airport, &default_airport, &library, &overlay];

        assert_eq!(
            SceneryPacksManager::default_global_airports_sort_order(&packages, false),
            1
        );
    }

    #[test]
    fn places_global_airports_before_regional_overlays_for_xplane_12() {
        let airport = make_package("Airport A", SceneryCategory::Airport, 0, true);
        let default_airport = make_package(
            "X-Plane Airports - EGPR Barra",
            SceneryCategory::DefaultAirport,
            1,
            true,
        );
        let regional_overlay = make_package(
            "simHeaven_X-World_Europe-1-vfr",
            SceneryCategory::RegionalOverlay,
            2,
            true,
        );
        let library = make_package("Library A", SceneryCategory::Library, 3, true);
        let overlay = make_package("Overlay A", SceneryCategory::Overlay, 4, true);
        let mesh = make_package("Mesh A", SceneryCategory::Mesh, 5, true);
        let packages = vec![
            &airport,
            &default_airport,
            &regional_overlay,
            &library,
            &overlay,
            &mesh,
        ];

        assert_eq!(
            SceneryPacksManager::default_global_airports_sort_order(&packages, true),
            2
        );
    }

    #[test]
    fn parses_global_airports_and_actual_paths_from_ini() {
        let content = concat!(
            "I\n1000 Version\nSCENERY\n\n",
            "SCENERY_PACK C:\\X-Plane\\Custom Scenery\\Airport A\n",
            "SCENERY_PACK *GLOBAL_AIRPORTS*\n",
            "SCENERY_PACK_DISABLED D:/External/Shortcut Scenery/\n"
        );

        let entries = parse_ini_entries(content);

        assert_eq!(entries.len(), 3);
        assert_eq!(
            entry_path_for_ini(&entries[0]),
            "C:/X-Plane/Custom Scenery/Airport A/"
        );
        assert!(entries[1].is_global_airports);
        assert_eq!(entries[1].path, "*GLOBAL_AIRPORTS*");
        assert!(!entries[2].enabled);
        assert_eq!(
            entry_path_for_ini(&entries[2]),
            "D:/External/Shortcut Scenery/"
        );
    }

    #[test]
    fn detects_global_airports_position_mismatch_in_ini() {
        let expected = vec![
            SceneryPackEntry {
                enabled: true,
                path: "Custom Scenery/Overlay A/".to_string(),
                is_global_airports: false,
            },
            SceneryPackEntry {
                enabled: true,
                path: GLOBAL_AIRPORTS_ENTRY_NAME.to_string(),
                is_global_airports: true,
            },
            SceneryPackEntry {
                enabled: true,
                path: "Custom Scenery/Library A/".to_string(),
                is_global_airports: false,
            },
        ];
        let ini_entries = parse_ini_entries(concat!(
            "I\n1000 Version\nSCENERY\n\n",
            "SCENERY_PACK Custom Scenery/Overlay A/\n",
            "SCENERY_PACK Custom Scenery/Library A/\n",
            "SCENERY_PACK *GLOBAL_AIRPORTS*\n"
        ));

        assert!(!entries_match_expected(&ini_entries, &expected));
    }

    #[test]
    fn honors_custom_global_airports_sort_order() {
        let airport = make_package("Airport A", SceneryCategory::Airport, 0, true);
        let library = make_package("Library A", SceneryCategory::Library, 1, true);
        let overlay = make_package("Overlay A", SceneryCategory::Overlay, 2, true);
        let packages = vec![&airport, &library, &overlay];

        let entries = build_entries_from_sorted_packages(
            &packages,
            &global_airports_state(true, 2, SceneryCategory::Overlay),
        );

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].path, "Custom Scenery/Airport A/");
        assert_eq!(entries[1].path, "Custom Scenery/Library A/");
        assert!(entries[2].is_global_airports);
        assert_eq!(entries[3].path, "Custom Scenery/Overlay A/");
    }
}

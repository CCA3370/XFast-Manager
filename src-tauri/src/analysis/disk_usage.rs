//! Disk usage analysis — scans X-Plane directories and reports folder sizes.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

/// Full report returned by `scan_disk_usage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsageReport {
    pub total_bytes: u64,
    pub categories: Vec<CategoryDiskUsage>,
    pub scan_duration_ms: u64,
}

/// One category (e.g., Aircraft, Plugins).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDiskUsage {
    pub category: String,
    pub total_bytes: u64,
    pub item_count: usize,
    pub items: Vec<ItemDiskUsage>,
}

/// A single addon/folder inside a category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDiskUsage {
    pub folder_name: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub file_count: usize,
    pub item_type: String,
}

/// Detailed scan of a single folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderDiskUsage {
    pub folder_name: String,
    pub total_bytes: u64,
    pub file_count: usize,
    pub largest_files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub size_bytes: u64,
}

// ---- Scanning helpers ---------------------------------------------------

struct CategorySpec {
    label: &'static str,
    sub_dir: &'static str,
    item_type: &'static str,
}

const CATEGORIES: &[CategorySpec] = &[
    CategorySpec {
        label: "Aircraft",
        sub_dir: "Aircraft",
        item_type: "aircraft",
    },
    CategorySpec {
        label: "Plugins",
        sub_dir: "Resources/plugins",
        item_type: "plugin",
    },
    CategorySpec {
        label: "Scenery",
        sub_dir: "Custom Scenery",
        item_type: "scenery",
    },
    CategorySpec {
        label: "Navdata",
        sub_dir: "Custom Data",
        item_type: "navdata",
    },
    CategorySpec {
        label: "Screenshots",
        sub_dir: "Output/screenshots",
        item_type: "screenshot",
    },
];

fn folder_size(path: &Path) -> (u64, usize) {
    let mut total: u64 = 0;
    let mut count: usize = 0;
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            count += 1;
        }
    }
    (total, count)
}

fn scan_category(xplane: &Path, spec: &CategorySpec) -> CategoryDiskUsage {
    let dir = xplane.join(spec.sub_dir);
    let mut items: Vec<ItemDiskUsage> = Vec::new();

    if dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            let sub_dirs: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect();

            let results: Vec<ItemDiskUsage> = sub_dirs
                .par_iter()
                .map(|p| {
                    let (size, fc) = folder_size(p);
                    let name = p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    ItemDiskUsage {
                        folder_name: name.clone(),
                        display_name: name,
                        size_bytes: size,
                        file_count: fc,
                        item_type: spec.item_type.to_string(),
                    }
                })
                .collect();

            items = results;
        }
    }

    // Sort descending by size
    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    let total_bytes: u64 = items.iter().map(|i| i.size_bytes).sum();
    let item_count = items.len();

    CategoryDiskUsage {
        category: spec.label.to_string(),
        total_bytes,
        item_count,
        items,
    }
}

/// Full scan of all known directories.
pub fn scan_disk_usage(xplane_path: &str) -> DiskUsageReport {
    let xplane = Path::new(xplane_path);
    let start = Instant::now();

    let categories: Vec<CategoryDiskUsage> = CATEGORIES
        .par_iter()
        .map(|spec| scan_category(xplane, spec))
        .collect();

    let total_bytes: u64 = categories.iter().map(|c| c.total_bytes).sum();
    let scan_duration_ms = start.elapsed().as_millis() as u64;

    DiskUsageReport {
        total_bytes,
        categories,
        scan_duration_ms,
    }
}

/// Detailed scan of one specific folder within a category.
pub fn scan_folder_disk_usage(
    xplane_path: &str,
    item_type: &str,
    folder_name: &str,
) -> Result<FolderDiskUsage, String> {
    let xplane = Path::new(xplane_path);

    let sub_dir = match item_type {
        "aircraft" => "Aircraft",
        "plugin" => "Resources/plugins",
        "scenery" => "Custom Scenery",
        "navdata" => "Custom Data",
        "screenshot" => "Output/screenshots",
        _ => return Err(format!("Unknown item type: {}", item_type)),
    };

    let folder = xplane.join(sub_dir).join(folder_name);
    if !folder.is_dir() {
        return Err(format!("Folder does not exist: {}", folder.display()));
    }

    let mut total_bytes: u64 = 0;
    let mut file_count: usize = 0;
    let mut files: Vec<FileEntry> = Vec::new();

    for entry in WalkDir::new(&folder).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            total_bytes += size;
            file_count += 1;

            let rel = entry
                .path()
                .strip_prefix(&folder)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();
            files.push(FileEntry {
                path: rel,
                size_bytes: size,
            });
        }
    }

    // Top 20 largest files
    files.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    files.truncate(20);

    Ok(FolderDiskUsage {
        folder_name: folder_name.to_string(),
        total_bytes,
        file_count,
        largest_files: files,
    })
}

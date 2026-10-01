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
            let mut sub_dirs: Vec<PathBuf> = Vec::new();
            let mut root_file_bytes: u64 = 0;
            let mut root_file_count: usize = 0;

            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    sub_dirs.push(path);
                } else if path.is_file() {
                    root_file_bytes += path.metadata().map(|m| m.len()).unwrap_or(0);
                    root_file_count += 1;
                }
            }

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

            // Include files at category root level (e.g. screenshots)
            if root_file_bytes > 0 {
                items.push(ItemDiskUsage {
                    folder_name: String::new(),
                    display_name: format!("({} files)", root_file_count),
                    size_bytes: root_file_bytes,
                    file_count: root_file_count,
                    item_type: spec.item_type.to_string(),
                });
            }
        }
    }

    // Sort descending by size
    items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));

    let total_bytes: u64 = items.iter().map(|i| i.size_bytes).sum();
    let item_count = items.len();

    CategoryDiskUsage {
        category: spec.label.to_string(),
        total_bytes,
        item_count,
        items,
    }
}

// Keep the remainder separate so base scenery, simulator resources, output and
// files at the X-Plane root are included without counting addon categories twice.
fn remaining_files(xplane: &Path) -> impl Iterator<Item = walkdir::DirEntry> + '_ {
    WalkDir::new(xplane)
        .into_iter()
        .filter_entry(move |entry| {
            !CATEGORIES
                .iter()
                .any(|spec| entry.path() == xplane.join(spec.sub_dir))
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
}

fn scan_remaining_category(xplane: &Path) -> CategoryDiskUsage {
    let mut total_bytes = 0;
    let mut file_count = 0;
    for entry in remaining_files(xplane) {
        total_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        file_count += 1;
    }
    CategoryDiskUsage {
        category: "Other".to_string(),
        total_bytes,
        item_count: 1,
        items: vec![ItemDiskUsage {
            folder_name: String::new(),
            display_name: "X-Plane".to_string(),
            size_bytes: total_bytes,
            file_count,
            item_type: "other".to_string(),
        }],
    }
}

/// Full scan of all known directories.
pub fn scan_disk_usage(xplane_path: &str) -> DiskUsageReport {
    let xplane = Path::new(xplane_path);
    let start = Instant::now();

    let mut categories: Vec<CategoryDiskUsage> = CATEGORIES
        .par_iter()
        .map(|spec| scan_category(xplane, spec))
        .collect();

    categories.push(scan_remaining_category(xplane));

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
        "other" if folder_name.is_empty() => "",
        _ => return Err(format!("Unknown item type: {}", item_type)),
    };

    let folder = xplane.join(sub_dir).join(folder_name);
    if !folder.is_dir() {
        return Err(format!("Folder does not exist: {}", folder.display()));
    }

    let mut total_bytes: u64 = 0;
    let mut file_count: usize = 0;
    let mut files: Vec<FileEntry> = Vec::new();

    let entries: Box<dyn Iterator<Item = walkdir::DirEntry> + '_> = if item_type == "other" {
        Box::new(remaining_files(xplane))
    } else {
        Box::new(WalkDir::new(&folder).into_iter().filter_map(Result::ok))
    };
    for entry in entries {
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
    files.sort_by_key(|file| std::cmp::Reverse(file.size_bytes));
    files.truncate(20);

    Ok(FolderDiskUsage {
        folder_name: folder_name.to_string(),
        total_bytes,
        file_count,
        largest_files: files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_counts_base_files_once_and_remainder_detail_agrees() {
        let root = tempfile::tempdir().unwrap();
        let files = [
            ("Aircraft/Test/plane.acf", 11),
            ("Resources/plugins/Test/plugin.xpl", 13),
            ("Custom Scenery/Test/Earth nav data/apt.dat", 17),
            ("Custom Data/nav.dat", 19),
            ("Output/screenshots/test.png", 23),
            ("Global Scenery/Earth nav data/tile.dsf", 29),
            ("Resources/default scenery/test.dat", 31),
            ("Output/preferences/test.prf", 37),
            ("X-Plane-x86_64", 41),
        ];
        for (name, bytes) in files {
            let path = root.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, vec![0; bytes]).unwrap();
        }
        let report = scan_disk_usage(root.path().to_str().unwrap());
        assert_eq!(report.total_bytes, 221);
        let other = report
            .categories
            .iter()
            .find(|c| c.category == "Other")
            .unwrap();
        assert_eq!(other.total_bytes, 138);
        let detail = scan_folder_disk_usage(root.path().to_str().unwrap(), "other", "").unwrap();
        assert_eq!(detail.total_bytes, 138);
        assert_eq!(detail.file_count, 4);
    }
}

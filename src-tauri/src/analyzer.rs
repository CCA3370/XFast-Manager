use anyhow::Result;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::models::{AddonType, AnalysisResult, DetectedItem, InstallTask};
use crate::scanner::Scanner;

pub struct Analyzer {
    scanner: Scanner,
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            scanner: Scanner::new(),
        }
    }

    /// Analyze a list of paths and return installation tasks
    pub fn analyze(&self, paths: Vec<String>, xplane_path: &str) -> AnalysisResult {
        let mut all_detected = Vec::new();
        let mut errors = Vec::new();

        // Scan all paths
        for path_str in paths {
            let path = Path::new(&path_str);
            
            match self.scanner.scan_path(path) {
                Ok(detected) => all_detected.extend(detected),
                Err(e) => errors.push(format!("Failed to scan {}: {}", path_str, e)),
            }
        }

        // Deduplicate items
        let deduplicated = self.deduplicate(all_detected);

        // Convert to install tasks
        let tasks = deduplicated
            .into_iter()
            .map(|item| self.create_install_task(item, xplane_path))
            .collect();

        AnalysisResult { tasks, errors }
    }

    /// Deduplicate detected items based on path hierarchy
    fn deduplicate(&self, items: Vec<DetectedItem>) -> Vec<DetectedItem> {
        let mut result = Vec::new();

        for item in items {
            let item_path = PathBuf::from(&item.path);
            let mut should_add = true;

            // Check if this item is a subdirectory of any existing item
            for existing in &result {
                let existing_path = PathBuf::from(&existing.path);

                // If item is under existing path, skip it
                if item_path.starts_with(&existing_path) {
                    should_add = false;
                    break;
                }

                // If existing is under item path, remove existing and add item
                if existing_path.starts_with(&item_path) {
                    should_add = true;
                    // Mark existing for removal
                }
            }

            if should_add {
                // Remove any items that are subdirectories of this item
                result.retain(|existing| {
                    let existing_path = PathBuf::from(&existing.path);
                    !existing_path.starts_with(&item_path) || existing_path == item_path
                });

                // Check for duplicates
                if !result.iter().any(|e| e.path == item.path) {
                    result.push(item);
                }
            }
        }

        result
    }

    /// Create an install task from a detected item
    fn create_install_task(&self, item: DetectedItem, xplane_path: &str) -> InstallTask {
        let xplane_root = Path::new(xplane_path);
        
        let target_base = match item.addon_type {
            AddonType::Aircraft => xplane_root.join("Aircraft"),
            AddonType::Scenery => xplane_root.join("Custom Scenery"),
            AddonType::Plugin => xplane_root.join("Resources").join("plugins"),
            AddonType::Navdata => {
                // Determine if it's GNS430 or main Custom Data
                if item.display_name.contains("GNS430") {
                    xplane_root.join("Custom Data").join("GNS430")
                } else {
                    xplane_root.join("Custom Data")
                }
            }
        };

        let folder_name = Path::new(&item.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let target_path = target_base.join(folder_name);

        // Check if target already exists
        let conflict_exists = target_path.exists();

        InstallTask {
            id: Uuid::new_v4().to_string(),
            addon_type: item.addon_type,
            source_path: item.path,
            target_path: target_path.to_string_lossy().to_string(),
            display_name: item.display_name,
            conflict_exists: if conflict_exists { Some(true) } else { None },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication() {
        let analyzer = Analyzer::new();

        let items = vec![
            DetectedItem {
                addon_type: AddonType::Aircraft,
                path: "/test/A330".to_string(),
                display_name: "A330".to_string(),
            },
            DetectedItem {
                addon_type: AddonType::Plugin,
                path: "/test/A330/plugins/fms".to_string(),
                display_name: "FMS".to_string(),
            },
        ];

        let result = analyzer.deduplicate(items);

        // Should only have the Aircraft, not the plugin (subdirectory)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addon_type, AddonType::Aircraft);
    }
}

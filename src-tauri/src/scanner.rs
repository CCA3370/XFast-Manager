use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::models::{AddonType, DetectedItem, NavdataCycle};

/// Scans a directory or archive and detects addon types based on markers
pub struct Scanner;

impl Scanner {
    pub fn new() -> Self {
        Scanner
    }

    /// Scan a path (file or directory) and detect all addon types
    pub fn scan_path(&self, path: &Path) -> Result<Vec<DetectedItem>> {
        let mut detected_items = Vec::new();

        if path.is_dir() {
            detected_items.extend(self.scan_directory(path)?);
        } else if path.is_file() {
            // For archives, we need to extract to temp first or scan without extraction
            // For now, we'll handle archives by treating them as potential root containers
            detected_items.extend(self.scan_archive(path)?);
        }

        Ok(detected_items)
    }

    /// Scan a directory recursively
    fn scan_directory(&self, dir: &Path) -> Result<Vec<DetectedItem>> {
        let mut detected = Vec::new();

        // Walk through the directory
        for entry in WalkDir::new(dir).follow_links(false) {
            let entry = entry?;
            let path = entry.path();

            if !entry.file_type().is_file() {
                continue;
            }

            // Check for different addon types based on file markers
            if let Some(item) = self.check_aircraft(path, dir)? {
                detected.push(item);
            } else if let Some(item) = self.check_scenery(path, dir)? {
                detected.push(item);
            } else if let Some(item) = self.check_plugin(path, dir)? {
                detected.push(item);
            } else if let Some(item) = self.check_navdata(path, dir)? {
                detected.push(item);
            }
        }

        Ok(detected)
    }

    /// Scan archive without full extraction
    fn scan_archive(&self, archive_path: &Path) -> Result<Vec<DetectedItem>> {
        let mut detected = Vec::new();
        let extension = archive_path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match extension {
            "zip" => detected.extend(self.scan_zip(archive_path)?),
            _ => {
                // For other formats (7z, rar), we'll need to extract to temp
                // For now, return empty
                eprintln!("Archive format {} not yet fully supported", extension);
            }
        }

        Ok(detected)
    }

    /// Scan a ZIP archive
    fn scan_zip(&self, zip_path: &Path) -> Result<Vec<DetectedItem>> {
        use zip::ZipArchive;
        
        let file = fs::File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut detected = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let file_path = file.name();

            // Check for markers in the archive
            if file_path.ends_with(".acf") {
                if let Some(item) = self.detect_aircraft_in_archive(file_path, zip_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with("library.txt") {
                if let Some(item) = self.detect_scenery_library(file_path, zip_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with(".dsf") {
                if let Some(item) = self.detect_scenery_dsf(file_path, zip_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with(".xpl") {
                if let Some(item) = self.detect_plugin_in_archive(file_path, zip_path)? {
                    detected.push(item);
                }
            } else if file_path.ends_with("cycle.json") {
                // Need to read the file content
                let mut file = archive.by_index(i)?;
                let mut content = String::new();
                use std::io::Read;
                file.read_to_string(&mut content)?;
                
                if let Some(item) = self.detect_navdata_in_archive(file_path, &content, zip_path)? {
                    detected.push(item);
                }
            }
        }

        Ok(detected)
    }

    // Type A: Aircraft Detection
    fn check_aircraft(&self, file_path: &Path, root: &Path) -> Result<Option<DetectedItem>> {
        if file_path.extension().and_then(|s| s.to_str()) != Some("acf") {
            return Ok(None);
        }

        // Get the parent directory of the .acf file
        let parent = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Failed to get parent directory")
        })?;

        // If .acf is in root, use the root folder name
        // Otherwise, use the immediate parent folder
        let install_path = if parent == root {
            root.to_path_buf()
        } else {
            parent.to_path_buf()
        };

        let display_name = install_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Aircraft")
            .to_string();

        Ok(Some(DetectedItem {
            addon_type: AddonType::Aircraft,
            path: install_path.to_string_lossy().to_string(),
            display_name,
        }))
    }

    fn detect_aircraft_in_archive(&self, file_path: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);
        let parent = path.parent();

        let display_name = if let Some(p) = parent {
            if p.as_os_str().is_empty() {
                // .acf is in root, use archive name
                archive_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Aircraft")
                    .to_string()
            } else {
                // Use parent folder name
                p.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Aircraft")
                    .to_string()
            }
        } else {
            archive_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Aircraft")
                .to_string()
        };

        Ok(Some(DetectedItem {
            addon_type: AddonType::Aircraft,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
        }))
    }

    // Type B: Scenery Detection
    fn check_scenery(&self, file_path: &Path, root: &Path) -> Result<Option<DetectedItem>> {
        let file_name = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        
        if file_name == "library.txt" {
            return self.detect_scenery_by_library(file_path);
        }

        if file_path.extension().and_then(|s| s.to_str()) == Some("dsf") {
            return self.detect_scenery_by_dsf(file_path);
        }

        Ok(None)
    }

    fn detect_scenery_by_library(&self, file_path: &Path) -> Result<Option<DetectedItem>> {
        // Install the immediate folder containing library.txt
        let parent = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Failed to get parent directory")
        })?;

        let display_name = parent
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Scenery")
            .to_string();

        Ok(Some(DetectedItem {
            addon_type: AddonType::Scenery,
            path: parent.to_string_lossy().to_string(),
            display_name,
        }))
    }

    fn detect_scenery_by_dsf(&self, file_path: &Path) -> Result<Option<DetectedItem>> {
        // Go UP 2 levels from the .dsf file
        let parent = file_path.parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent());

        if let Some(install_dir) = parent {
            let display_name = install_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Scenery")
                .to_string();

            Ok(Some(DetectedItem {
                addon_type: AddonType::Scenery,
                path: install_dir.to_string_lossy().to_string(),
                display_name,
            }))
        } else {
            Ok(None)
        }
    }

    fn detect_scenery_library(&self, file_path: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);
        let parent = path.parent();

        let display_name = if let Some(p) = parent {
            p.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Scenery")
                .to_string()
        } else {
            "Unknown Scenery".to_string()
        };

        Ok(Some(DetectedItem {
            addon_type: AddonType::Scenery,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
        }))
    }

    fn detect_scenery_dsf(&self, file_path: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);
        // Go up 2 levels
        let parent = path.parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent());

        let display_name = if let Some(p) = parent {
            p.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Scenery")
                .to_string()
        } else {
            "Unknown Scenery".to_string()
        };

        Ok(Some(DetectedItem {
            addon_type: AddonType::Scenery,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
        }))
    }

    // Type C: Plugin Detection
    fn check_plugin(&self, file_path: &Path, _root: &Path) -> Result<Option<DetectedItem>> {
        if file_path.extension().and_then(|s| s.to_str()) != Some("xpl") {
            return Ok(None);
        }

        let parent = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Failed to get parent directory")
        })?;

        let parent_name = parent
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Check if parent is a platform-specific folder
        let install_path = if matches!(
            parent_name,
            "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
        ) {
            // Go up one more level
            parent.parent().unwrap_or(parent).to_path_buf()
        } else {
            parent.to_path_buf()
        };

        let display_name = install_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Plugin")
            .to_string();

        Ok(Some(DetectedItem {
            addon_type: AddonType::Plugin,
            path: install_path.to_string_lossy().to_string(),
            display_name,
        }))
    }

    fn detect_plugin_in_archive(&self, file_path: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
        let path = PathBuf::from(file_path);
        let parent = path.parent();

        let display_name = if let Some(p) = parent {
            let parent_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            
            // Check if parent is platform-specific
            if matches!(
                parent_name,
                "32" | "64" | "win" | "lin" | "mac" | "win_x64" | "mac_x64" | "lin_x64"
            ) {
                // Go up one more level
                if let Some(grandparent) = p.parent() {
                    grandparent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown Plugin")
                        .to_string()
                } else {
                    "Unknown Plugin".to_string()
                }
            } else {
                parent_name.to_string()
            }
        } else {
            "Unknown Plugin".to_string()
        };

        Ok(Some(DetectedItem {
            addon_type: AddonType::Plugin,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
        }))
    }

    // Type D: Navdata Detection
    fn check_navdata(&self, file_path: &Path, _root: &Path) -> Result<Option<DetectedItem>> {
        if file_path.file_name().and_then(|s| s.to_str()) != Some("cycle.json") {
            return Ok(None);
        }

        let content = fs::read_to_string(file_path)
            .context("Failed to read cycle.json")?;

        let cycle: NavdataCycle = serde_json::from_str(&content)
            .context("Failed to parse cycle.json")?;

        let parent = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Failed to get parent directory")
        })?;

        // Determine install path based on name
        let (install_path, display_name) = if cycle.name.contains("X-Plane 12") || cycle.name.contains("X-Plane 11") {
            (parent.to_path_buf(), format!("Navdata: {}", cycle.name))
        } else if cycle.name.contains("X-Plane GNS430") {
            (parent.to_path_buf(), format!("Navdata GNS430: {}", cycle.name))
        } else {
            return Err(anyhow::anyhow!("Unknown Navdata Format: {}", cycle.name));
        };

        Ok(Some(DetectedItem {
            addon_type: AddonType::Navdata,
            path: install_path.to_string_lossy().to_string(),
            display_name,
        }))
    }

    fn detect_navdata_in_archive(&self, file_path: &str, content: &str, archive_path: &Path) -> Result<Option<DetectedItem>> {
        let cycle: NavdataCycle = serde_json::from_str(content)
            .context("Failed to parse cycle.json")?;

        let display_name = if cycle.name.contains("X-Plane 12") || cycle.name.contains("X-Plane 11") {
            format!("Navdata: {}", cycle.name)
        } else if cycle.name.contains("X-Plane GNS430") {
            format!("Navdata GNS430: {}", cycle.name)
        } else {
            return Err(anyhow::anyhow!("Unknown Navdata Format: {}", cycle.name));
        };

        Ok(Some(DetectedItem {
            addon_type: AddonType::Navdata,
            path: archive_path.to_string_lossy().to_string(),
            display_name,
        }))
    }
}

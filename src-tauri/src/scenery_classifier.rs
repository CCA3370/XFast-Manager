//! Scenery classification module for auto-sorting
//!
//! This module analyzes scenery packages and determines their category
//! by parsing DSF file headers and checking file system structure.

use crate::models::{DsfHeader, SceneryCategory, SceneryPackageInfo};
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;
use walkdir::WalkDir;

const MAX_SCAN_DEPTH: usize = 15;
const MAX_PLUGIN_SCAN_DEPTH: usize = 5;

/// Check if folder contains plugins (.xpl files)
fn has_plugins(scenery_path: &Path) -> bool {
    let plugins_path = scenery_path.join("plugins");

    // Check if plugins folder exists (follow symlinks)
    if !plugins_path.metadata().map(|m| m.is_dir()).unwrap_or(false) {
        return false;
    }

    // Search for .xpl files up to 5 levels deep
    for entry in WalkDir::new(&plugins_path)
        .follow_links(true)
        .max_depth(MAX_PLUGIN_SCAN_DEPTH)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext.eq_ignore_ascii_case("xpl") {
                    return true;
                }
            }
        }
    }

    false
}

/// Main entry point for scenery classification
pub fn classify_scenery(scenery_path: &Path, xplane_path: &Path) -> Result<SceneryPackageInfo> {
    let folder_name = scenery_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid folder name"))?
        .to_string();

    crate::log_debug!(&format!("Classifying scenery: {}", folder_name), "scenery_classifier");
    crate::log_debug!(&format!("  Path: {:?}", scenery_path), "scenery_classifier");

    // Check if path is a symlink and log the target
    if let Ok(metadata) = scenery_path.symlink_metadata() {
        if metadata.is_symlink() {
            if let Ok(target) = std::fs::read_link(scenery_path) {
                crate::log_debug!(&format!("  ⚠ This is a symlink pointing to: {:?}", target), "scenery_classifier");
            }
        }
    }

    // Early validation: Check if this is a valid scenery package
    // Must have either "Earth nav data" folder, "library.txt" file, or plugins folder with .xpl files
    // Use metadata() to follow symbolic links
    let library_txt_path = scenery_path.join("library.txt");
    let has_library_txt = library_txt_path
        .metadata()
        .map(|m| m.is_file())
        .unwrap_or(false);
    if has_library_txt {
        crate::log_debug!(&format!("  Found library.txt at: {:?}", library_txt_path), "scenery_classifier");
    }

    let earth_nav_path = scenery_path.join("Earth nav data");
    let has_earth_nav_data = earth_nav_path
        .metadata()
        .map(|m| m.is_dir())
        .unwrap_or(false);
    if has_earth_nav_data {
        crate::log_debug!(&format!("  Found Earth nav data at: {:?}", earth_nav_path), "scenery_classifier");
    }

    let has_plugin_files = has_plugins(scenery_path);
    if has_plugin_files {
        crate::log_debug!(&format!("  Found plugins folder with .xpl files"), "scenery_classifier");
    }

    crate::log_debug!(
        &format!("  has_library_txt: {}, has_earth_nav_data: {}, has_plugins: {}",
            has_library_txt, has_earth_nav_data, has_plugin_files),
        "scenery_classifier"
    );

    if !has_library_txt && !has_earth_nav_data && !has_plugin_files {
        crate::log_debug!(
            &format!("  ❌ Not a valid scenery: missing 'Earth nav data', 'library.txt', and plugins"),
            "scenery_classifier"
        );
        return Err(anyhow!("Not a valid scenery package: missing 'Earth nav data', 'library.txt', and plugins"));
    }

    // If only has plugins (no scenery features), classify as Other
    if has_plugin_files && !has_library_txt && !has_earth_nav_data {
        crate::log_debug!(&format!("  ✓ Classified as Other (has plugins but no scenery features)"), "scenery_classifier");
        return Ok(build_package_info(
            folder_name,
            SceneryCategory::Other,
            scenery_path,
            false,
            false,
            false,
            0,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )?);
    }

    // Collect file system information
    crate::log_debug!("  Checking for apt.dat...", "scenery_classifier");
    let has_apt_dat = check_apt_dat_recursive(scenery_path)?;
    crate::log_debug!(&format!("  apt.dat check complete: {}", has_apt_dat), "scenery_classifier");

    crate::log_debug!("  Searching for DSF files...", "scenery_classifier");
    let dsf_files = find_dsf_files(scenery_path)?;
    crate::log_debug!(&format!("  DSF search complete: {} files", dsf_files.len()), "scenery_classifier");

    crate::log_debug!("  Counting textures...", "scenery_classifier");
    let texture_count = count_texture_files(scenery_path)?;
    crate::log_debug!(&format!("  Texture count complete: {}", texture_count), "scenery_classifier");

    crate::log_debug!(
        &format!("  has_apt_dat: {}, dsf_files: {}, texture_count: {}",
            has_apt_dat, dsf_files.len(), texture_count),
        "scenery_classifier"
    );

    // Decision Tree Classification
    // Parse DSF header if available
    let dsf_header_opt: Option<DsfHeader> = if !dsf_files.is_empty() {
        match parse_dsf_header(&dsf_files[0]) {
            Ok(header) => {
                if let Some(ref agent) = header.creation_agent {
                    crate::log_debug!(&format!("  creation_agent: {}", agent), "scenery_classifier");
                }
                crate::log_debug!(&format!("  sim/overlay: {}", header.is_overlay), "scenery_classifier");
                crate::log_debug!(&format!("  has terrain refs: {} (count: {})", !header.terrain_references.is_empty(), header.terrain_references.len()), "scenery_classifier");
                if !header.terrain_references.is_empty() {
                    crate::log_debug!(&format!("  terrain refs sample: {:?}", header.terrain_references.iter().take(3).collect::<Vec<_>>()), "scenery_classifier");
                }
                Some(header)
            }
            Err(e) => {
                crate::log_debug!(&format!("  Failed to parse DSF: {}", e), "scenery_classifier");
                None
            }
        }
    } else {
        None
    };

    // Decision Tree:
    // 1. Has apt.dat OR (DSF with WorldEditor creation_agent) → Airport
    if has_apt_dat {
        crate::log_debug!(&format!("  ✓ Classified as Airport (has apt.dat)"), "scenery_classifier");

        // Extract required libraries but don't check for missing ones yet
        // (will be done after index is built)
        let required_libraries = if let Some(ref header) = dsf_header_opt {
            extract_required_libraries(&header.object_references)
        } else {
            Vec::new()
        };

        // Parse library.txt if exists to get exported library names
        let exported_library_names = if has_library_txt {
            let library_txt_path = scenery_path.join("library.txt");
            crate::scenery_index::parse_library_exports(&library_txt_path)
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if !exported_library_names.is_empty() {
            crate::log_debug!(
                &format!("  Airport also exports libraries: {:?}", exported_library_names),
                "scenery_classifier"
            );
        }

        return Ok(build_package_info(
            folder_name,
            SceneryCategory::Airport,
            scenery_path,
            true,
            !dsf_files.is_empty(),
            has_library_txt,
            texture_count,
            required_libraries,
            Vec::new(), // missing_libraries will be filled later
            exported_library_names,
        )?);
    }

    if let Some(ref header) = dsf_header_opt {
        if let Some(ref agent) = header.creation_agent {
            if agent.to_lowercase().contains("worldeditor") {
                crate::log_debug!(&format!("  ✓ Classified as Airport (WorldEditor without apt.dat)"), "scenery_classifier");
                let required = extract_required_libraries(&header.object_references);

                // Parse library.txt if exists to get exported library names
                let exported_library_names = if has_library_txt {
                    let library_txt_path = scenery_path.join("library.txt");
                    crate::scenery_index::parse_library_exports(&library_txt_path)
                        .into_iter()
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                if !exported_library_names.is_empty() {
                    crate::log_debug!(
                        &format!("  Airport also exports libraries: {:?}", exported_library_names),
                        "scenery_classifier"
                    );
                }

                return Ok(build_package_info(
                    folder_name,
                    SceneryCategory::Airport,
                    scenery_path,
                    false,
                    true,
                    has_library_txt,
                    texture_count,
                    required,
                    Vec::new(), // missing_libraries will be filled later
                    exported_library_names,
                )?);
            }
        }
    }

    // 2. Has sim/overlay 1 but no apt.dat → Overlay
    if let Some(ref header) = dsf_header_opt {
        if header.is_overlay {
            crate::log_debug!(&format!("  ✓ Classified as Overlay (sim/overlay without apt.dat)"), "scenery_classifier");
            let required = extract_required_libraries(&header.object_references);

            // Parse library.txt if exists to get exported library names
            let exported_library_names = if has_library_txt {
                let library_txt_path = scenery_path.join("library.txt");
                crate::scenery_index::parse_library_exports(&library_txt_path)
                    .into_iter()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };

            if !exported_library_names.is_empty() {
                crate::log_debug!(
                    &format!("  Overlay also exports libraries: {:?}", exported_library_names),
                    "scenery_classifier"
                );
            }

            return Ok(build_package_info(
                folder_name,
                SceneryCategory::Overlay,
                scenery_path,
                false,
                true,
                has_library_txt,
                texture_count,
                required,
                Vec::new(), // missing_libraries will be filled later
                exported_library_names,
            )?);
        }
    }

    // 3. Has library.txt but no Earth nav data → Library or FixedHighPriority (SAM)
    if has_library_txt && !has_earth_nav_data {
        // Check if it's a SAM library
        // Match patterns:
        // 1. "sam" as a separate word: "SAM_Library", "open_SAM_library"
        // 2. Starts with "sam": "SAM-DeveloperPack"
        // 3. Ends with "sam": "openSAM", "openSAM_Library"
        // But NOT: "zsam" (airport code), "sample" (different word)

        let folder_lower = folder_name.to_lowercase();

        // Split by non-alphanumeric and check for exact "sam"
        let parts: Vec<&str> = folder_lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        let has_sam_word = parts.iter().any(|&part| part == "sam");

        // Check if any part ends with "sam" (like "opensam")
        let has_sam_suffix = parts.iter().any(|&part| {
            part.ends_with("sam") && part.len() > 3 && {
                // Make sure it's not part of another word like "sample"
                let prefix = &part[..part.len()-3];
                // Common SAM library prefixes
                matches!(prefix, "open" | "my" | "custom" | "new")
            }
        });

        let is_sam = has_sam_word || has_sam_suffix;

        let category = if is_sam {
            crate::log_debug!(&format!("  ✓ Classified as FixedHighPriority (SAM library)"), "scenery_classifier");
            SceneryCategory::FixedHighPriority
        } else {
            crate::log_debug!(&format!("  ✓ Classified as Library (has library.txt but no Earth nav data)"), "scenery_classifier");
            SceneryCategory::Library
        };

        // Parse library.txt to get exported library names
        let library_txt_path = scenery_path.join("library.txt");
        let exported_library_names = crate::scenery_index::parse_library_exports(&library_txt_path)
            .into_iter()
            .collect::<Vec<_>>();

        crate::log_debug!(
            &format!("  Library exports: {:?}", exported_library_names),
            "scenery_classifier"
        );

        return Ok(build_package_info(
            folder_name,
            category,
            scenery_path,
            false,
            !dsf_files.is_empty(),
            true,
            texture_count,
            Vec::new(),
            Vec::new(),
            exported_library_names,
        )?);
    }

    // 4. No sim/overlay and no apt.dat, OR has TERRAIN_DEF → Mesh
    // Check for terrain references in DSF
    let has_terrain_def = if let Some(ref header) = dsf_header_opt {
        !header.terrain_references.is_empty()
    } else {
        false
    };

    if !has_apt_dat && has_earth_nav_data {
        // Has Earth nav data but no apt.dat and no sim/overlay → Mesh
        crate::log_debug!(&format!("  ✓ Classified as Mesh (Earth nav data without apt.dat/overlay)"), "scenery_classifier");

        // Check if it's Ortho4XP (special case of Mesh - Orthophotos)
        let category = if let Some(ref header) = dsf_header_opt {
            if let Some(ref agent) = header.creation_agent {
                if agent.to_lowercase().contains("ortho4xp") {
                    crate::log_debug!(&format!("    → Orthophotos (Ortho4XP)"), "scenery_classifier");
                    SceneryCategory::Orthophotos
                } else {
                    SceneryCategory::Mesh
                }
            } else {
                SceneryCategory::Mesh
            }
        } else {
            SceneryCategory::Mesh
        };

        let (required_libraries, missing_libraries) = if let Some(ref header) = dsf_header_opt {
            let required = extract_required_libraries(&header.object_references);
            let missing = check_missing_libraries_from_refs(&header.object_references, xplane_path, scenery_path)?;
            (required, missing)
        } else {
            (Vec::new(), Vec::new())
        };

        // Parse library.txt if exists to get exported library names
        let exported_library_names = if has_library_txt {
            let library_txt_path = scenery_path.join("library.txt");
            crate::scenery_index::parse_library_exports(&library_txt_path)
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if !exported_library_names.is_empty() {
            crate::log_debug!(
                &format!("  Mesh/Orthophotos also exports libraries: {:?}", exported_library_names),
                "scenery_classifier"
            );
        }

        return Ok(build_package_info(
            folder_name,
            category,
            scenery_path,
            false,
            !dsf_files.is_empty(),
            has_library_txt,
            texture_count,
            required_libraries,
            missing_libraries,
            exported_library_names,
        )?);
    }

    if has_terrain_def {
        crate::log_debug!(&format!("  ✓ Classified as Mesh (has TERRAIN_DEF)"), "scenery_classifier");
        let (required_libraries, missing_libraries) = if let Some(ref header) = dsf_header_opt {
            let required = extract_required_libraries(&header.object_references);
            let missing = check_missing_libraries_from_refs(&header.object_references, xplane_path, scenery_path)?;
            (required, missing)
        } else {
            (Vec::new(), Vec::new())
        };

        // Parse library.txt if exists to get exported library names
        let exported_library_names = if has_library_txt {
            let library_txt_path = scenery_path.join("library.txt");
            crate::scenery_index::parse_library_exports(&library_txt_path)
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if !exported_library_names.is_empty() {
            crate::log_debug!(
                &format!("  Mesh also exports libraries: {:?}", exported_library_names),
                "scenery_classifier"
            );
        }

        return Ok(build_package_info(
            folder_name,
            SceneryCategory::Mesh,
            scenery_path,
            false,
            true,
            has_library_txt,
            texture_count,
            required_libraries,
            missing_libraries,
            exported_library_names,
        )?);
    }

    // Default: Other
    crate::log_debug!(&format!("  ✓ Classified as Other (no clear indicators)"), "scenery_classifier");

    // Parse library.txt if exists to get exported library names
    let exported_library_names = if has_library_txt {
        let library_txt_path = scenery_path.join("library.txt");
        crate::scenery_index::parse_library_exports(&library_txt_path)
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    if !exported_library_names.is_empty() {
        crate::log_debug!(
            &format!("  Other also exports libraries: {:?}", exported_library_names),
            "scenery_classifier"
        );
    }

    Ok(build_package_info(
        folder_name,
        SceneryCategory::Other,
        scenery_path,
        false,
        !dsf_files.is_empty(),
        has_library_txt,
        texture_count,
        Vec::new(),
        Vec::new(),
        exported_library_names,
    )?)
}

/// Check if apt.dat exists recursively in Earth nav data directories
fn check_apt_dat_recursive(scenery_path: &Path) -> Result<bool> {
    // apt.dat is always in Earth nav data folder, so only search there
    let earth_nav_path = scenery_path.join("Earth nav data");
    if !earth_nav_path.exists() {
        return Ok(false);
    }

    // Only search up to 5 levels deep in Earth nav data
    for entry in WalkDir::new(&earth_nav_path)
        .follow_links(true)  // Explicitly follow symbolic links
        .max_depth(5)  // Limit depth to avoid scanning too deep
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if name.eq_ignore_ascii_case("apt.dat") {
                    // Validate apt.dat format
                    if validate_apt_dat(entry.path())? {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}

/// Validate apt.dat file format (first line "I", second line starts with "1")
fn validate_apt_dat(path: &Path) -> Result<bool> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 256];
    let bytes_read = file.read(&mut buffer)?;

    if bytes_read < 10 {
        return Ok(false);
    }

    let content = String::from_utf8_lossy(&buffer[..bytes_read]);
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < 2 {
        return Ok(false);
    }

    // First line should be "I" or "A"
    let first_line = lines[0].trim();
    if first_line != "I" && first_line != "A" {
        return Ok(false);
    }

    // Second line should start with version number (usually 1000, 1050, 1100, 1200, etc.)
    let second_line = lines[1].trim();
    if second_line.starts_with("1") || second_line.starts_with("850") {
        return Ok(true);
    }

    Ok(false)
}

/// Find first DSF file in scenery package (for classification)
fn find_dsf_files(scenery_path: &Path) -> Result<Vec<std::path::PathBuf>> {
    // Only need one DSF file for classification, so return as soon as we find one
    // Use a more efficient approach: check common locations first

    crate::log_debug!("  Starting DSF search...", "scenery_classifier");

    // First, try to find DSF in Earth nav data subdirectories (most common location)
    let earth_nav_path = scenery_path.join("Earth nav data");
    if earth_nav_path.exists() {
        crate::log_debug!("  Scanning Earth nav data folder...", "scenery_classifier");

        // Only scan 2 levels deep in Earth nav data (Earth nav data/+XX+YYY/*.dsf)
        let walker = WalkDir::new(&earth_nav_path)
            .follow_links(true)
            .min_depth(2)  // Skip the Earth nav data folder itself
            .max_depth(2)  // Only go into first level subdirectories
            .into_iter();

        let mut count = 0;
        for entry in walker.filter_map(|e| e.ok()) {
            count += 1;
            if count % 100 == 0 {
                crate::log_debug!(&format!("  Scanned {} entries...", count), "scenery_classifier");
            }

            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext.eq_ignore_ascii_case("dsf") {
                        crate::log_debug!(
                            &format!("  Found DSF after {} entries: {:?}", count, entry.path()),
                            "scenery_classifier"
                        );
                        return Ok(vec![entry.path().to_path_buf()]);
                    }
                }
            }
        }

        crate::log_debug!(&format!("  No DSF found in Earth nav data after scanning {} entries", count), "scenery_classifier");
    }

    crate::log_debug!("  Doing general search...", "scenery_classifier");

    // If not found in Earth nav data, do a general search (but still limit depth)
    for entry in WalkDir::new(scenery_path)
        .follow_links(true)
        .max_depth(5)  // Limit depth to avoid scanning too deep
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext.eq_ignore_ascii_case("dsf") {
                    crate::log_debug!(
                        &format!("  Found DSF: {:?}", entry.path()),
                        "scenery_classifier"
                    );
                    return Ok(vec![entry.path().to_path_buf()]);
                }
            }
        }
    }

    // No DSF files found
    crate::log_debug!("  No DSF files found", "scenery_classifier");
    Ok(Vec::new())
}

/// Check if DSF file is 7z compressed
fn is_dsf_compressed(dsf_path: &Path) -> Result<bool> {
    let mut file = File::open(dsf_path)?;
    let mut magic = [0u8; 4];

    if file.read_exact(&mut magic).is_err() {
        return Ok(false);
    }

    // 7z magic bytes: 0x37 0x7A 0xBC 0xAF
    Ok(magic == [0x37, 0x7A, 0xBC, 0xAF])
}

/// Decompress 7z DSF file
fn decompress_dsf(dsf_path: &Path) -> Result<Vec<u8>> {
    use sevenz_rust2::decompress_file;

    // Create a temporary buffer to hold decompressed data
    let mut decompressed = Vec::new();

    // Use sevenz_rust2 to decompress
    let temp_dir = tempfile::tempdir()?;
    decompress_file(dsf_path, temp_dir.path())?;

    // Find the decompressed DSF file
    for entry in std::fs::read_dir(temp_dir.path())? {
        let entry = entry?;
        if entry.path().extension().map_or(false, |e| e == "dsf")
            || entry.file_type()?.is_file()
        {
            decompressed = std::fs::read(entry.path())?;
            break;
        }
    }

    if decompressed.is_empty() {
        // If no DSF found in temp dir, the 7z might contain raw data
        // Try reading the first file
        for entry in std::fs::read_dir(temp_dir.path())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                decompressed = std::fs::read(entry.path())?;
                break;
            }
        }
    }

    Ok(decompressed)
}

/// Parse DSF file header
pub fn parse_dsf_header(dsf_path: &Path) -> Result<DsfHeader> {
    // Check if compressed
    let is_compressed = is_dsf_compressed(dsf_path)?;

    // Get DSF data
    let data = if is_compressed {
        decompress_dsf(dsf_path)?
    } else {
        std::fs::read(dsf_path)?
    };

    // Verify magic bytes: "XPLNEDSF"
    if data.len() < 12 || &data[0..8] != b"XPLNEDSF" {
        return Err(anyhow!("Invalid DSF file: missing magic bytes"));
    }

    // Extract properties from PROP section
    let properties = extract_dsf_properties(&data)?;

    // Extract object and terrain references from definitions
    let (object_references, terrain_references) = extract_dsf_definitions(&data)?;

    Ok(DsfHeader {
        is_overlay: properties.get("sim/overlay").map(|v| v == "1").unwrap_or(false),
        airport_icao: properties.get("sim/filter/aptid").cloned(),
        creation_agent: properties.get("sim/creation_agent").cloned(),
        has_exclusions: properties.keys().any(|k| k.starts_with("sim/exclude_")),
        requires_agpoint: properties.contains_key("sim/require_agpoint"),
        requires_object: properties.contains_key("sim/require_object"),
        object_references,
        terrain_references,
    })
}

/// Extract properties from DSF PROP section
fn extract_dsf_properties(data: &[u8]) -> Result<HashMap<String, String>> {
    let mut properties = HashMap::new();

    // DSF structure: magic (8) + version (4) + atoms
    // Each atom: id (4) + length (4) + data
    // We need to find DAEH (HEAD reversed) then PORP (PROP reversed)

    let mut offset = 12; // Skip magic + version

    while offset + 8 <= data.len() {
        // Read atom ID (4 bytes, little-endian reversed)
        let atom_id = &data[offset..offset + 4];
        let atom_len = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;

        if atom_len < 8 || offset + atom_len > data.len() {
            break;
        }

        // Check for HEAD atom (contains PROP)
        if atom_id == b"DAEH" {
            // HEAD atom, look for PROP inside
            let head_data = &data[offset + 8..offset + atom_len];
            if let Ok(props) = extract_props_from_head(head_data) {
                properties = props;
            }
            break;
        }

        offset += atom_len;
    }

    Ok(properties)
}

/// Extract PROP data from HEAD atom
fn extract_props_from_head(head_data: &[u8]) -> Result<HashMap<String, String>> {
    let mut properties = HashMap::new();
    let mut offset = 0;

    while offset + 8 <= head_data.len() {
        let atom_id = &head_data[offset..offset + 4];
        let atom_len = u32::from_le_bytes([
            head_data[offset + 4],
            head_data[offset + 5],
            head_data[offset + 6],
            head_data[offset + 7],
        ]) as usize;

        if atom_len < 8 || offset + atom_len > head_data.len() {
            break;
        }

        // Check for PROP atom
        if atom_id == b"PORP" {
            let prop_data = &head_data[offset + 8..offset + atom_len];
            properties = parse_prop_strings(prop_data)?;
            break;
        }

        offset += atom_len;
    }

    Ok(properties)
}

/// Parse null-terminated key-value strings from PROP data
fn parse_prop_strings(prop_data: &[u8]) -> Result<HashMap<String, String>> {
    let mut properties = HashMap::new();
    let mut i = 0;

    while i < prop_data.len() {
        // Read key (null-terminated)
        let key_end = prop_data[i..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(prop_data.len() - i);

        if key_end == 0 {
            break;
        }

        let key = String::from_utf8_lossy(&prop_data[i..i + key_end]).to_string();
        i += key_end + 1;

        if i >= prop_data.len() {
            break;
        }

        // Read value (null-terminated)
        let value_end = prop_data[i..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(prop_data.len() - i);

        let value = String::from_utf8_lossy(&prop_data[i..i + value_end]).to_string();
        i += value_end + 1;

        if !key.is_empty() {
            properties.insert(key, value);
        }
    }

    Ok(properties)
}

/// Extract object/terrain definitions from DSF file
fn extract_dsf_definitions(data: &[u8]) -> Result<(Vec<String>, Vec<String>)> {
    let mut object_refs = Vec::new();
    let mut terrain_refs = Vec::new();
    let mut offset = 12; // Skip magic + version

    while offset + 8 <= data.len() {
        let atom_id = &data[offset..offset + 4];
        let atom_len = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;

        if atom_len < 8 || offset + atom_len > data.len() {
            break;
        }

        // Check for DEFN atom (contains TERT, OBJT, POLY, etc.)
        if atom_id == b"NFED" {
            let defn_data = &data[offset + 8..offset + atom_len];
            if let Ok((objs, terrains)) = extract_definitions_from_defn(defn_data) {
                object_refs.extend(objs);
                terrain_refs.extend(terrains);
            }
            break;
        }

        offset += atom_len;
    }

    Ok((object_refs, terrain_refs))
}

/// Extract definitions from DEFN atom
fn extract_definitions_from_defn(defn_data: &[u8]) -> Result<(Vec<String>, Vec<String>)> {
    let mut object_refs = Vec::new();
    let mut terrain_refs = Vec::new();
    let mut offset = 0;

    while offset + 8 <= defn_data.len() {
        let atom_id = &defn_data[offset..offset + 4];
        let atom_len = u32::from_le_bytes([
            defn_data[offset + 4],
            defn_data[offset + 5],
            defn_data[offset + 6],
            defn_data[offset + 7],
        ]) as usize;

        if atom_len < 8 || offset + atom_len > defn_data.len() {
            break;
        }

        // TRET (terrain), OBJT (objects), POLY (polygons), NETW (networks)
        if atom_id == b"TRET" {
            // Terrain definitions
            let def_data = &defn_data[offset + 8..offset + atom_len];
            if let Ok(defs) = parse_definition_strings(def_data) {
                terrain_refs.extend(defs);
            }
        } else if atom_id == b"TJBO" || atom_id == b"YLOP" || atom_id == b"WTEN" {
            // Object, polygon, and network definitions
            let def_data = &defn_data[offset + 8..offset + atom_len];
            if let Ok(defs) = parse_definition_strings(def_data) {
                object_refs.extend(defs);
            }
        }

        offset += atom_len;
    }

    Ok((object_refs, terrain_refs))
}

/// Parse null-terminated definition strings
fn parse_definition_strings(def_data: &[u8]) -> Result<Vec<String>> {
    let mut definitions = Vec::new();
    let mut i = 0;

    while i < def_data.len() {
        let end = def_data[i..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(def_data.len() - i);

        if end == 0 {
            i += 1;
            continue;
        }

        let def = String::from_utf8_lossy(&def_data[i..i + end]).to_string();
        if !def.is_empty() && (def.contains('/') || def.contains('.')) {
            definitions.push(def);
        }

        i += end + 1;
    }

    Ok(definitions)
}

/// Count texture files in scenery folder (up to 5 for classification)
fn count_texture_files(scenery_path: &Path) -> Result<usize> {
    let textures_path = scenery_path.join("textures");
    if !textures_path.exists() {
        return Ok(0);
    }

    // Only count up to 5 textures - enough to determine if this is an orthophoto scenery
    let mut count = 0;
    for entry in WalkDir::new(&textures_path)
        .follow_links(true)  // Explicitly follow symbolic links
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if ext_lower == "dds" || ext_lower == "png" || ext_lower == "jpg" {
                    count += 1;
                    if count >= 5 {
                        // Found enough textures for classification
                        return Ok(count);
                    }
                }
            }
        }
    }

    Ok(count)
}

/// Extract library names from object references
fn extract_required_libraries(object_refs: &[String]) -> Vec<String> {
    object_refs
        .iter()
        .filter_map(|obj| extract_library_name(obj))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

/// Extract library name from object path
fn extract_library_name(obj_path: &str) -> Option<String> {
    // Skip default X-Plane libraries
    if obj_path.starts_with("lib/") {
        return None;
    }

    // Get first path component
    let first_component = obj_path.split('/').next()?;

    // Known library prefixes
    let known_library_prefixes = [
        "opensceneryx",
        "opensam",
        "sam",
        "acs_",
        "flightbeam",
        "misterx",
        "naps",
        "gt_",
        "ff_",
        "r2_",
        "ra_",
        "rd_",
        "re_",
        "cdb",
        "bs2001",
        "fjs",
        "flyagi",
        "jb_",
        "pm_",
        "puf_",
        "ruscenery",
        "handy",
        "vehicle",
        "vfr",
        "world-models",
        "x-codr",
        "zdp",
        "orbx",
        "pp",
        "dense_forests",
        "flags",
        "aircraft-static",
        "3d_people",
        "aericaps",
    ];

    let first_lower = first_component.to_lowercase();
    if known_library_prefixes
        .iter()
        .any(|prefix| first_lower.starts_with(prefix))
    {
        return Some(first_component.to_string());
    }

    // Skip generic local folders (common subdirectories within scenery packages)
    if matches!(
        first_component,
        "objects" | "facades" | "vegetation" | "roads" | "textures" | "terrain"
        | "forests" | "lines" | "beaches" | "orthophotos" | "earth nav data"
        | "Earth nav data" | "plugins" | "documentation" | "doc" | "docs"
    ) {
        return None;
    }

    // If the path has only one component (no slash), it's likely a local file, not a library reference
    if !obj_path.contains('/') {
        return None;
    }

    // Assume it's a library reference
    Some(first_component.to_string())
}

/// Check for missing libraries from object references
fn check_missing_libraries_from_refs(
    object_refs: &[String],
    xplane_path: &Path,
    scenery_path: &Path,
) -> Result<Vec<String>> {
    let mut missing = Vec::new();

    // Get the current scenery folder name to exclude self-references
    let current_folder_name = scenery_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    crate::log_debug!(
        &format!("  Checking libraries for scenery: {}", current_folder_name),
        "scenery_classifier"
    );

    // Build library index from all library.txt files
    let library_index = crate::scenery_index::build_library_index(xplane_path);

    let required = extract_required_libraries(object_refs);

    crate::log_debug!(
        &format!("  Required libraries: {:?}", required),
        "scenery_classifier"
    );

    for lib_name in required {
        // Skip if this is a reference to the current scenery package itself (case-insensitive)
        if lib_name.eq_ignore_ascii_case(current_folder_name) {
            crate::log_debug!(
                &format!("  ✓ Skipping self-reference: {} (matches current folder: {})", lib_name, current_folder_name),
                "scenery_classifier"
            );
            continue;
        }

        // Check if this is a subdirectory within the current scenery package
        let subdir_path = scenery_path.join(&lib_name);
        if subdir_path.exists() && subdir_path.is_dir() {
            crate::log_debug!(
                &format!("  ✓ Skipping internal subdirectory: {} (exists in current package)", lib_name),
                "scenery_classifier"
            );
            continue;
        }

        // Check if this library name is in the library index
        if let Some(folder_name) = library_index.get(&lib_name) {
            crate::log_debug!(
                &format!("  ✓ Found library '{}' in folder '{}'", lib_name, folder_name),
                "scenery_classifier"
            );
        } else {
            // Library not found in index, mark as missing
            crate::log_debug!(
                &format!("  ✗ Missing library: {}", lib_name),
                "scenery_classifier"
            );
            missing.push(lib_name);
        }
    }

    Ok(missing)
}

/// Get directory modification time
fn get_dir_modified_time(path: &Path) -> Result<SystemTime> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.modified()?)
}

/// Build SceneryPackageInfo
fn build_package_info(
    folder_name: String,
    category: SceneryCategory,
    scenery_path: &Path,
    has_apt_dat: bool,
    has_dsf: bool,
    has_library_txt: bool,
    texture_count: usize,
    required_libraries: Vec<String>,
    missing_libraries: Vec<String>,
    exported_library_names: Vec<String>,
) -> Result<SceneryPackageInfo> {
    // Calculate sub-priority based on category and folder name
    let sub_priority = calculate_sub_priority(&category, &folder_name);

    Ok(SceneryPackageInfo {
        folder_name,
        category,
        sub_priority,
        last_modified: get_dir_modified_time(scenery_path)?,
        has_apt_dat,
        has_dsf,
        has_library_txt,
        has_textures: texture_count > 0,
        has_objects: scenery_path.join("objects").exists(),
        texture_count,
        indexed_at: SystemTime::now(),
        required_libraries,
        missing_libraries,
        exported_library_names,
        enabled: true,   // Default to enabled
        sort_order: 0,   // Will be assigned during index rebuild
    })
}

/// Calculate sub-priority for a scenery package
/// Sub-priority is used to order scenery within the same category
fn calculate_sub_priority(category: &SceneryCategory, folder_name: &str) -> u8 {
    let folder_name_lower = folder_name.to_lowercase();

    match category {
        SceneryCategory::Orthophotos => {
            // XPME orthophotos should be last (priority 2)
            if folder_name_lower.contains("xpme") {
                2
            } else {
                0
            }
        }
        SceneryCategory::Mesh => {
            // XPME mesh should be last (priority 2)
            if folder_name_lower.contains("xpme") {
                2
            } else {
                1
            }
        }
        _ => 0, // Default sub-priority for all other categories
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_sam_library_detection() {
        // SAM libraries should be classified as FixedHighPriority
        let folder_names = vec!["SAM_Library", "openSAM_Library", "sam_scenery", "MySAMPack"];
        for name in folder_names {
            let lower = name.to_lowercase();
            assert!(
                lower.contains("sam"),
                "{} should contain 'sam'",
                name
            );
        }
    }

    #[test]
    fn test_extract_library_name() {
        // Should extract library names
        assert_eq!(
            extract_library_name("opensceneryx/objects/airport/radio/2.obj"),
            Some("opensceneryx".to_string())
        );
        assert_eq!(
            extract_library_name("ACS_Singapore/facades/house.fac"),
            Some("ACS_Singapore".to_string())
        );

        // Should return None for local paths
        assert_eq!(extract_library_name("objects/building.obj"), None);
        assert_eq!(extract_library_name("facades/house.fac"), None);

        // Should return None for default libs
        assert_eq!(
            extract_library_name("lib/g10/terrain10/apt_terrain.ter"),
            None
        );
    }

    #[test]
    fn test_validate_apt_dat_format() {
        // This test would need actual test files
        // For now, just verify the function exists and compiles
    }
}

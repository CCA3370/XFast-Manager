use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Read;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    SevenZ,
    Rar,
}

impl ArchiveFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::SevenZ => "7z",
            ArchiveFormat::Rar => "rar",
        }
    }

    fn temp_suffix(self) -> &'static str {
        match self {
            ArchiveFormat::Zip => ".zip",
            ArchiveFormat::SevenZ => ".7z",
            ArchiveFormat::Rar => ".rar",
        }
    }
}

pub struct PreparedArchive {
    read_path: PathBuf,
    _temp_file: Option<NamedTempFile>,
}

impl PreparedArchive {
    pub fn read_path(&self) -> &Path {
        &self.read_path
    }
}

fn split_numbered_series(file_name: &str, marker_lower: &str) -> Option<(String, usize, u32)> {
    let lower = file_name.to_ascii_lowercase();
    let pos = lower.rfind(marker_lower)?;
    let digits = &file_name[(pos + marker_lower.len())..];
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let width = digits.len();
    let index = digits.parse::<u32>().ok()?;
    let prefix = file_name[..(pos + marker_lower.len())].to_string();
    Some((prefix, width, index))
}

fn split_rar_part(file_name: &str) -> Option<(String, usize, u32)> {
    let lower = file_name.to_ascii_lowercase();
    if !lower.ends_with(".rar") {
        return None;
    }

    let body_lower = &lower[..(lower.len() - 4)];
    let pos = body_lower.rfind(".part")?;
    let digits_start = pos + ".part".len();
    let digits_end = file_name.len() - 4;
    let digits = &file_name[digits_start..digits_end];
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let width = digits.len();
    let index = digits.parse::<u32>().ok()?;
    let prefix = file_name[..pos].to_string();
    Some((prefix, width, index))
}

fn split_plain_numbered(file_name: &str) -> Option<(String, usize, u32)> {
    let (base, ext) = file_name.rsplit_once('.')?;
    if ext.is_empty() || !ext.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let width = ext.len();
    let index = ext.parse::<u32>().ok()?;
    Some((format!("{}.", base), width, index))
}

fn split_zip_z_volume(file_name: &str) -> Option<(String, usize, u32)> {
    let (base, ext) = file_name.rsplit_once('.')?;
    let ext_lower = ext.to_ascii_lowercase();
    let digits = ext_lower.strip_prefix('z')?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let width = digits.len();
    let index = digits.parse::<u32>().ok()?;
    Some((base.to_string(), width, index))
}

fn split_rar_r_volume(file_name: &str) -> Option<(String, usize, u32)> {
    let (base, ext) = file_name.rsplit_once('.')?;
    let ext_lower = ext.to_ascii_lowercase();
    let digits = ext_lower.strip_prefix('r')?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let width = digits.len();
    let index = digits.parse::<u32>().ok()?;
    Some((base.to_string(), width, index))
}

fn split_volume_error(group_name: &str, missing_name: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "Missing split archive volume for {}: {}",
        group_name,
        missing_name
    )
}

fn find_existing_sibling(parent: &Path, target_name: &str) -> Option<PathBuf> {
    let direct = parent.join(target_name);
    if direct.exists() {
        return Some(direct);
    }

    let target_lower = target_name.to_ascii_lowercase();
    let entries = fs::read_dir(parent).ok()?;
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            if name.to_ascii_lowercase() == target_lower {
                return Some(entry.path());
            }
        }
    }

    None
}

fn archive_entry_key(path: &Path) -> String {
    let key = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        key.to_ascii_lowercase()
    } else {
        key
    }
}

pub fn normalize_archive_input_paths(paths: Vec<String>) -> Vec<String> {
    let mut normalized_paths = Vec::new();
    let mut seen_paths = HashSet::new();

    for path_str in paths {
        let normalized = normalize_archive_entry_path(Path::new(&path_str));
        let key = archive_entry_key(&normalized);
        if seen_paths.insert(key) {
            normalized_paths.push(normalized.to_string_lossy().to_string());
        }
    }

    normalized_paths
}

fn collect_numbered_siblings(parent: &Path, prefix: &str, width: usize) -> BTreeMap<u32, PathBuf> {
    let mut parts = BTreeMap::new();
    let prefix_lower = prefix.to_ascii_lowercase();

    let Ok(entries) = fs::read_dir(parent) else {
        return parts;
    };

    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let name_lower = name.to_ascii_lowercase();
        if !name_lower.starts_with(&prefix_lower) {
            continue;
        }

        let Some(suffix) = name.get(prefix.len()..) else {
            continue;
        };
        if suffix.len() != width || !suffix.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        if let Ok(index) = suffix.parse::<u32>() {
            parts.entry(index).or_insert_with(|| entry.path());
        }
    }

    parts
}

fn collect_numbered_parts_checked(
    parent: &Path,
    prefix: &str,
    width: usize,
    first_index: u32,
    group_name: &str,
) -> Result<Vec<PathBuf>> {
    let existing = collect_numbered_siblings(parent, prefix, width);
    if existing.is_empty() {
        return Ok(Vec::new());
    }

    let max_index = *existing
        .keys()
        .max()
        .expect("non-empty numbered parts map should have max");
    let mut parts = Vec::new();
    for index in first_index..=max_index {
        match existing.get(&index) {
            Some(path) => parts.push(path.clone()),
            None => {
                let missing = format!("{}{:0width$}", prefix, index, width = width);
                return Err(split_volume_error(group_name, &missing));
            }
        }
    }

    Ok(parts)
}

fn collect_letter_numbered_siblings(
    parent: &Path,
    base: &str,
    marker: char,
) -> BTreeMap<u32, (usize, PathBuf)> {
    let mut parts = BTreeMap::new();
    let base_lower = base.to_ascii_lowercase();

    let Ok(entries) = fs::read_dir(parent) else {
        return parts;
    };

    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Some((entry_base, ext)) = name.rsplit_once('.') else {
            continue;
        };
        if entry_base.to_ascii_lowercase() != base_lower {
            continue;
        }

        let ext_lower = ext.to_ascii_lowercase();
        let Some(digits) = ext_lower.strip_prefix(marker) else {
            continue;
        };
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        if let Ok(index) = digits.parse::<u32>() {
            parts
                .entry(index)
                .or_insert_with(|| (digits.len(), entry.path()));
        }
    }

    parts
}

fn collect_letter_numbered_parts_checked(
    parent: &Path,
    base: &str,
    marker: char,
    first_index: u32,
    default_width: usize,
    group_name: &str,
) -> Result<Vec<PathBuf>> {
    let existing = collect_letter_numbered_siblings(parent, base, marker);
    if existing.is_empty() {
        return Ok(Vec::new());
    }

    let width = existing
        .get(&first_index)
        .map(|(width, _)| *width)
        .or_else(|| existing.values().next().map(|(width, _)| *width))
        .unwrap_or(default_width);
    let max_index = *existing
        .keys()
        .max()
        .expect("non-empty lettered parts map should have max");

    let mut parts = Vec::new();
    for index in first_index..=max_index {
        match existing.get(&index) {
            Some((_, path)) => parts.push(path.clone()),
            None => {
                let missing = format!("{}.{}{:0width$}", base, marker, index, width = width);
                return Err(split_volume_error(group_name, &missing));
            }
        }
    }

    Ok(parts)
}

fn detect_archive_format_from_signature(path: &Path) -> Option<ArchiveFormat> {
    if !path.is_file() {
        return None;
    }

    let mut file = fs::File::open(path).ok()?;
    let mut header = [0u8; 8];
    let read = file.read(&mut header).ok()?;
    if read < 4 {
        return None;
    }

    if header.starts_with(b"PK\x03\x04")
        || header.starts_with(b"PK\x05\x06")
        || header.starts_with(b"PK\x07\x08")
    {
        return Some(ArchiveFormat::Zip);
    }

    if read >= 6 && header[..6] == [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C] {
        return Some(ArchiveFormat::SevenZ);
    }

    if (read >= 7 && header[..7] == [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x00])
        || header == [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00]
    {
        return Some(ArchiveFormat::Rar);
    }

    None
}

fn is_split_archive_path(file_name: &str, lower: &str, path: &Path) -> bool {
    split_numbered_series(file_name, ".zip.").is_some()
        || split_numbered_series(file_name, ".7z.").is_some()
        || split_plain_numbered(file_name).is_some()
        || split_zip_z_volume(file_name).is_some()
        || split_rar_r_volume(file_name).is_some()
        || split_rar_part(file_name).is_some()
        || (lower.ends_with(".zip")
            && path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|stem| {
                    find_existing_sibling(
                        path.parent().unwrap_or_else(|| Path::new(".")),
                        &format!("{}.z01", stem),
                    )
                    .is_some()
                })
                .unwrap_or(false))
        || (lower.ends_with(".7z")
            && find_existing_sibling(
                path.parent().unwrap_or_else(|| Path::new(".")),
                &format!("{}{}", file_name, ".001"),
            )
            .is_some())
}

pub fn detect_archive_format(path: &Path) -> Option<ArchiveFormat> {
    let file_name = path.file_name()?.to_str()?;
    let lower = file_name.to_ascii_lowercase();

    if !is_split_archive_path(file_name, &lower, path) {
        if let Some(format) = detect_archive_format_from_signature(path) {
            return Some(format);
        }
    }

    if lower.ends_with(".zip")
        || split_numbered_series(file_name, ".zip.").is_some()
        || split_zip_z_volume(file_name).is_some()
    {
        return Some(ArchiveFormat::Zip);
    }

    if lower.ends_with(".7z") || split_numbered_series(file_name, ".7z.").is_some() {
        return Some(ArchiveFormat::SevenZ);
    }

    if lower.ends_with(".rar")
        || split_rar_part(file_name).is_some()
        || split_rar_r_volume(file_name).is_some()
    {
        return Some(ArchiveFormat::Rar);
    }

    if let Some((prefix, width, index)) = split_plain_numbered(file_name) {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let first_name = format!("{}{:0width$}", prefix, 1, width = width);
        if let Some(first_path) = find_existing_sibling(parent, &first_name) {
            return detect_archive_format_from_signature(&first_path);
        }

        if index > 1 {
            return Some(ArchiveFormat::SevenZ);
        }
    }

    None
}

pub fn normalize_archive_entry_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = match path.file_name().and_then(|s| s.to_str()) {
        Some(v) => v,
        None => return path.to_path_buf(),
    };

    if let Some((prefix, width, _)) = split_numbered_series(file_name, ".zip.") {
        let first_name = format!("{}{:0width$}", prefix, 1, width = width);
        if let Some(first) = find_existing_sibling(parent, &first_name) {
            return first;
        }
    }

    if let Some((prefix, width, _)) = split_numbered_series(file_name, ".7z.") {
        let first_name = format!("{}{:0width$}", prefix, 1, width = width);
        if let Some(first) = find_existing_sibling(parent, &first_name) {
            return first;
        }
    }

    if let Some((base, _, _)) = split_zip_z_volume(file_name) {
        let zip_name = format!("{}.zip", base);
        if let Some(zip_path) = find_existing_sibling(parent, &zip_name) {
            return zip_path;
        }
    }

    if let Some((base, _, _)) = split_rar_r_volume(file_name) {
        let rar_name = format!("{}.rar", base);
        if let Some(rar_path) = find_existing_sibling(parent, &rar_name) {
            return rar_path;
        }
    }

    if let Some((prefix, width, index)) = split_rar_part(file_name) {
        if index != 1 {
            let first_part_name = format!("{}.part{:0width$}.rar", prefix, 1, width = width);
            if let Some(part_path) = find_existing_sibling(parent, &first_part_name) {
                return part_path;
            }
        }

        return path.to_path_buf();
    }

    if let Some((prefix, width, index)) = split_plain_numbered(file_name) {
        if index != 1 {
            let first_name = format!("{}{:0width$}", prefix, 1, width = width);
            if let Some(first_path) = find_existing_sibling(parent, &first_name) {
                if detect_archive_format_from_signature(&first_path).is_some() {
                    return first_path;
                }
            }
        }
    }

    path.to_path_buf()
}

fn collect_zip_split_parts(path: &Path) -> Result<Option<Vec<PathBuf>>> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = match path.file_name().and_then(|s| s.to_str()) {
        Some(v) => v,
        None => return Ok(None),
    };
    let lower = file_name.to_ascii_lowercase();

    if let Some((prefix, width, _)) = split_numbered_series(file_name, ".zip.") {
        let parts = collect_numbered_parts_checked(parent, &prefix, width, 1, file_name)?;
        if parts.is_empty() {
            return Ok(None);
        }
        return Ok(Some(parts));
    }

    if let Some((base, width, _)) = split_zip_z_volume(file_name) {
        let parts = collect_letter_numbered_parts_checked(parent, &base, 'z', 1, width, file_name)?;
        if parts.is_empty() {
            return Ok(None);
        }

        let final_name = format!("{}.zip", base);
        let final_part = find_existing_sibling(parent, &final_name)
            .ok_or_else(|| split_volume_error(file_name, &final_name))?;

        let mut full_parts = parts;
        full_parts.push(final_part);
        return Ok(Some(full_parts));
    }

    if lower.ends_with(".zip") {
        let stem = &file_name[..(file_name.len() - 4)];
        let mut parts = collect_letter_numbered_parts_checked(parent, stem, 'z', 1, 2, file_name)?;

        if parts.is_empty() {
            return Ok(None);
        }

        let final_part = find_existing_sibling(parent, file_name)
            .ok_or_else(|| split_volume_error(file_name, file_name))?;
        parts.push(final_part);
        return Ok(Some(parts));
    }

    Ok(None)
}

fn collect_7z_split_parts(path: &Path) -> Result<Option<Vec<PathBuf>>> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = match path.file_name().and_then(|s| s.to_str()) {
        Some(v) => v,
        None => return Ok(None),
    };

    if let Some((prefix, width, _)) = split_numbered_series(file_name, ".7z.") {
        let parts = collect_numbered_parts_checked(parent, &prefix, width, 1, file_name)?;
        if parts.is_empty() {
            return Ok(None);
        }
        return Ok(Some(parts));
    }

    if file_name.to_ascii_lowercase().ends_with(".7z") {
        let split_prefix = format!("{}.", file_name);
        let parts = collect_numbered_parts_checked(parent, &split_prefix, 3, 1, file_name)?;
        if !parts.is_empty() {
            return Ok(Some(parts));
        }
    }

    Ok(None)
}

fn collect_plain_numbered_split_parts(
    path: &Path,
    format: ArchiveFormat,
) -> Result<Option<Vec<PathBuf>>> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = match path.file_name().and_then(|s| s.to_str()) {
        Some(v) => v,
        None => return Ok(None),
    };

    if split_numbered_series(file_name, ".zip.").is_some()
        || split_numbered_series(file_name, ".7z.").is_some()
    {
        return Ok(None);
    }

    let Some((prefix, width, _)) = split_plain_numbered(file_name) else {
        return Ok(None);
    };

    let first_name = format!("{}{:0width$}", prefix, 1, width = width);
    let first_path = find_existing_sibling(parent, &first_name)
        .ok_or_else(|| split_volume_error(file_name, &first_name))?;

    match detect_archive_format_from_signature(&first_path) {
        Some(detected) if detected == format => {}
        Some(detected) => {
            return Err(anyhow::anyhow!(
                "Split archive volume {} was detected as {}, not {}",
                first_name,
                detected.as_str(),
                format.as_str()
            ));
        }
        None => return Ok(None),
    }

    let parts = collect_numbered_parts_checked(parent, &prefix, width, 1, file_name)?;
    if parts.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parts))
    }
}

fn validate_rar_split_parts(path: &Path) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = match path.file_name().and_then(|s| s.to_str()) {
        Some(v) => v,
        None => return Ok(()),
    };
    let lower = file_name.to_ascii_lowercase();

    if let Some((base, width, _)) = split_rar_r_volume(file_name) {
        let first_name = format!("{}.rar", base);
        find_existing_sibling(parent, &first_name)
            .ok_or_else(|| split_volume_error(file_name, &first_name))?;
        collect_letter_numbered_parts_checked(parent, &base, 'r', 0, width, file_name)?;
        return Ok(());
    }

    if lower.ends_with(".rar") && split_rar_part(file_name).is_none() {
        let base = &file_name[..(file_name.len() - 4)];
        collect_letter_numbered_parts_checked(parent, base, 'r', 0, 2, file_name)?;
        return Ok(());
    }

    if let Some((prefix, width, _)) = split_rar_part(file_name) {
        let expected_first = format!("{}.part{:0width$}.rar", prefix, 1, width = width);
        find_existing_sibling(parent, &expected_first)
            .ok_or_else(|| split_volume_error(file_name, &expected_first))?;

        let existing = collect_rar_part_siblings(parent, &prefix);
        let max_index = existing.keys().max().copied().unwrap_or(1).max(1);
        for index in 1..=max_index {
            if !existing.contains_key(&index) {
                let missing = format!("{}.part{:0width$}.rar", prefix, index, width = width);
                return Err(split_volume_error(file_name, &missing));
            }
        }
    }

    Ok(())
}

fn collect_rar_part_siblings(parent: &Path, prefix: &str) -> BTreeMap<u32, PathBuf> {
    let mut parts = BTreeMap::new();
    let prefix_lower = prefix.to_ascii_lowercase();

    let Ok(entries) = fs::read_dir(parent) else {
        return parts;
    };

    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".rar") {
            continue;
        }
        let body = &name[..(name.len() - 4)];
        let body_lower = body.to_ascii_lowercase();
        let Some(rest) = body_lower.strip_prefix(&format!("{}.part", prefix_lower)) else {
            continue;
        };
        if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(index) = rest.parse::<u32>() {
            parts.entry(index).or_insert_with(|| entry.path());
        }
    }

    parts
}

fn concat_parts_to_temp(parts: &[PathBuf], format: ArchiveFormat) -> Result<PreparedArchive> {
    let mut temp_file = NamedTempFile::with_suffix(format.temp_suffix())
        .context("Failed to create temp archive for split volumes")?;

    {
        let out = temp_file.as_file_mut();
        let mut writer = BufWriter::new(out);
        for part in parts {
            let mut input = fs::File::open(part).with_context(|| {
                format!("Failed to open split archive part: {}", part.display())
            })?;
            std::io::copy(&mut input, &mut writer).with_context(|| {
                format!("Failed to merge split archive part: {}", part.display())
            })?;
        }
        writer.flush()?;
    }

    Ok(PreparedArchive {
        read_path: temp_file.path().to_path_buf(),
        _temp_file: Some(temp_file),
    })
}

pub fn prepare_archive_for_read(path: &Path, format: ArchiveFormat) -> Result<PreparedArchive> {
    let normalized = normalize_archive_entry_path(path);

    let parts = match format {
        ArchiveFormat::Zip => match collect_zip_split_parts(&normalized)? {
            Some(parts) => Some(parts),
            None => collect_plain_numbered_split_parts(&normalized, format)?,
        },
        ArchiveFormat::SevenZ => match collect_7z_split_parts(&normalized)? {
            Some(parts) => Some(parts),
            None => collect_plain_numbered_split_parts(&normalized, format)?,
        },
        ArchiveFormat::Rar => {
            validate_rar_split_parts(&normalized)?;
            if let Some(parts) = collect_plain_numbered_split_parts(&normalized, format)? {
                if let Some(first) = parts.first() {
                    return Ok(PreparedArchive {
                        read_path: first.clone(),
                        _temp_file: None,
                    });
                }
            }
            None
        }
    };

    if let Some(parts) = parts {
        let lower = normalized.to_string_lossy().to_ascii_lowercase();
        let needs_concat = parts.len() > 1
            || (format == ArchiveFormat::Zip && !lower.ends_with(".zip"))
            || (format == ArchiveFormat::SevenZ && !lower.ends_with(".7z"));
        if needs_concat {
            return concat_parts_to_temp(&parts, format);
        }
    }

    Ok(PreparedArchive {
        read_path: normalized,
        _temp_file: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        detect_archive_format, normalize_archive_entry_path, normalize_archive_input_paths,
        prepare_archive_for_read, ArchiveFormat,
    };
    use std::fs;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn detect_archive_format_prefers_signature_for_single_file_archives() {
        let temp = tempdir().expect("failed to create tempdir");
        let archive = temp.path().join("archive.zip");
        fs::write(&archive, [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0x00, 0x04])
            .expect("failed to write archive");

        assert_eq!(detect_archive_format(&archive), Some(ArchiveFormat::SevenZ));
    }

    #[test]
    fn detect_archive_format_keeps_split_zip_detection() {
        let temp = tempdir().expect("failed to create tempdir");
        let zip = temp.path().join("archive.zip");
        let z01 = temp.path().join("archive.z01");
        fs::write(&zip, b"not-a-real-zip").expect("failed to write zip");
        fs::write(&z01, b"split-part").expect("failed to write z01");

        assert_eq!(detect_archive_format(&zip), Some(ArchiveFormat::Zip));
        assert_eq!(detect_archive_format(&z01), Some(ArchiveFormat::Zip));
    }

    #[test]
    fn split_7z_non_first_volume_normalizes_and_reads_contiguous_parts() {
        let temp = tempdir().expect("failed to create tempdir");
        let part1 = temp.path().join("addon.7z.001");
        let part2 = temp.path().join("addon.7z.002");
        fs::write(&part1, [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, b'a', b'b'])
            .expect("failed to write first part");
        fs::write(&part2, b"cd").expect("failed to write second part");

        assert_eq!(normalize_archive_entry_path(&part2), part1);
        assert_eq!(detect_archive_format(&part2), Some(ArchiveFormat::SevenZ));

        let prepared =
            prepare_archive_for_read(&part2, ArchiveFormat::SevenZ).expect("prepare split 7z");
        let mut merged = Vec::new();
        fs::File::open(prepared.read_path())
            .expect("open merged archive")
            .read_to_end(&mut merged)
            .expect("read merged archive");

        assert_eq!(
            merged,
            vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, b'a', b'b', b'c', b'd']
        );
    }

    #[test]
    fn split_archive_inputs_are_deduplicated_to_first_volume() {
        let temp = tempdir().expect("failed to create tempdir");
        let part1 = temp.path().join("addon.7z.001");
        let part2 = temp.path().join("addon.7z.002");
        let part3 = temp.path().join("addon.7z.003");
        fs::write(&part1, [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C])
            .expect("failed to write first part");
        fs::write(&part2, b"part2").expect("failed to write second part");
        fs::write(&part3, b"part3").expect("failed to write third part");

        let normalized = normalize_archive_input_paths(vec![
            part2.to_string_lossy().to_string(),
            part1.to_string_lossy().to_string(),
            part3.to_string_lossy().to_string(),
        ]);

        assert_eq!(normalized, vec![part1.to_string_lossy().to_string()]);
    }

    #[test]
    fn detects_supported_split_archive_name_patterns() {
        let temp = tempdir().expect("failed to create tempdir");

        let zip = temp.path().join("package.zip");
        let z01 = temp.path().join("package.z01");
        fs::write(&zip, b"final").expect("failed to write zip");
        fs::write(&z01, b"part").expect("failed to write z01");
        assert_eq!(detect_archive_format(&z01), Some(ArchiveFormat::Zip));
        assert_eq!(normalize_archive_entry_path(&z01), zip);

        let zip001 = temp.path().join("numbered.zip.001");
        fs::write(&zip001, b"part").expect("failed to write zip.001");
        assert_eq!(detect_archive_format(&zip001), Some(ArchiveFormat::Zip));

        let rar = temp.path().join("legacy.rar");
        let r00 = temp.path().join("legacy.r00");
        fs::write(&rar, b"rar").expect("failed to write rar");
        fs::write(&r00, b"r00").expect("failed to write r00");
        assert_eq!(detect_archive_format(&r00), Some(ArchiveFormat::Rar));
        assert_eq!(normalize_archive_entry_path(&r00), rar);

        let part1 = temp.path().join("modern.part1.rar");
        let part2 = temp.path().join("modern.part2.rar");
        fs::write(&part1, b"rar").expect("failed to write part1 rar");
        fs::write(&part2, b"rar").expect("failed to write part2 rar");
        assert_eq!(detect_archive_format(&part2), Some(ArchiveFormat::Rar));
        assert_eq!(normalize_archive_entry_path(&part2), part1);

        let plain1 = temp.path().join("plain.001");
        let plain2 = temp.path().join("plain.002");
        fs::write(&plain1, [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C])
            .expect("failed to write plain first volume");
        fs::write(&plain2, b"more").expect("failed to write plain second volume");
        assert_eq!(detect_archive_format(&plain2), Some(ArchiveFormat::SevenZ));
        assert_eq!(normalize_archive_entry_path(&plain2), plain1);
    }

    #[test]
    fn missing_middle_volume_returns_specific_error() {
        let temp = tempdir().expect("failed to create tempdir");
        let part1 = temp.path().join("addon.7z.001");
        let part3 = temp.path().join("addon.7z.003");
        fs::write(&part1, [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C])
            .expect("failed to write first part");
        fs::write(&part3, b"part3").expect("failed to write third part");

        let error = match prepare_archive_for_read(&part1, ArchiveFormat::SevenZ) {
            Ok(_) => panic!("missing middle volume should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("addon.7z.002"));
    }

    #[test]
    fn missing_zip_z_volume_returns_specific_error() {
        let temp = tempdir().expect("failed to create tempdir");
        let zip = temp.path().join("package.zip");
        let z02 = temp.path().join("package.z02");
        fs::write(&zip, b"final").expect("failed to write final zip");
        fs::write(&z02, b"part").expect("failed to write z02");

        let error = match prepare_archive_for_read(&zip, ArchiveFormat::Zip) {
            Ok(_) => panic!("missing z01 should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("package.z01"));
    }
}

use anyhow::{Context, Result};
use std::fs;
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

pub fn detect_archive_format(path: &Path) -> Option<ArchiveFormat> {
    let file_name = path.file_name()?.to_str()?;
    let lower = file_name.to_ascii_lowercase();

    if lower.ends_with(".zip")
        || split_numbered_series(file_name, ".zip.").is_some()
        || {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            ext.len() >= 2
                && ext.to_ascii_lowercase().starts_with('z')
                && ext[1..].chars().all(|c| c.is_ascii_digit())
        }
    {
        return Some(ArchiveFormat::Zip);
    }

    if lower.ends_with(".7z") || split_numbered_series(file_name, ".7z.").is_some() {
        return Some(ArchiveFormat::SevenZ);
    }

    if lower.ends_with(".rar")
        || split_rar_part(file_name).is_some()
        || {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            ext.len() >= 2
                && ext.to_ascii_lowercase().starts_with('r')
                && ext[1..].chars().all(|c| c.is_ascii_digit())
        }
    {
        return Some(ArchiveFormat::Rar);
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

    if let Some((base, ext)) = file_name.rsplit_once('.') {
        let ext_lower = ext.to_ascii_lowercase();
        if ext_lower.starts_with('z')
            && ext_lower.len() >= 2
            && ext_lower[1..].chars().all(|c| c.is_ascii_digit())
        {
            let zip_name = format!("{}.zip", base);
            if let Some(zip_path) = find_existing_sibling(parent, &zip_name) {
                return zip_path;
            }
        }

        if ext_lower.starts_with('r')
            && ext_lower.len() >= 2
            && ext_lower[1..].chars().all(|c| c.is_ascii_digit())
        {
            let rar_name = format!("{}.rar", base);
            if let Some(rar_path) = find_existing_sibling(parent, &rar_name) {
                return rar_path;
            }
        }
    }

    if let Some((prefix, width, index)) = split_rar_part(file_name) {
        if index != 1 {
            let first_part_name = format!("{}.part{:0width$}.rar", prefix, 1, width = width);
            if let Some(part_path) = find_existing_sibling(parent, &first_part_name) {
                return part_path;
            }
        }

        let plain_rar = format!("{}.rar", prefix);
        if let Some(rar_path) = find_existing_sibling(parent, &plain_rar) {
            return rar_path;
        }
    }

    path.to_path_buf()
}

fn collect_numbered_parts(parent: &Path, prefix: &str, width: usize) -> Vec<PathBuf> {
    let mut parts = Vec::new();
    let mut index = 1u32;
    loop {
        let part_name = format!("{}{:0width$}", prefix, index, width = width);
        if let Some(part_path) = find_existing_sibling(parent, &part_name) {
            parts.push(part_path);
            index += 1;
            continue;
        }
        break;
    }
    parts
}

fn collect_zip_split_parts(path: &Path) -> Result<Option<Vec<PathBuf>>> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = match path.file_name().and_then(|s| s.to_str()) {
        Some(v) => v,
        None => return Ok(None),
    };
    let lower = file_name.to_ascii_lowercase();

    if let Some((prefix, width, _)) = split_numbered_series(file_name, ".zip.") {
        let parts = collect_numbered_parts(parent, &prefix, width);
        if parts.is_empty() {
            return Ok(None);
        }
        return Ok(Some(parts));
    }

    if lower.ends_with(".zip") {
        let stem = &file_name[..(file_name.len() - 4)];
        let mut parts = Vec::new();
        let mut index = 1u32;
        loop {
            let part_name = format!("{}.z{:02}", stem, index);
            if let Some(part_path) = find_existing_sibling(parent, &part_name) {
                parts.push(part_path);
                index += 1;
                continue;
            }
            break;
        }

        if parts.is_empty() {
            return Ok(None);
        }

        let final_part = find_existing_sibling(parent, file_name).ok_or_else(|| {
            anyhow::anyhow!("Missing final ZIP volume for split archive: {}", path.display())
        })?;
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
        let parts = collect_numbered_parts(parent, &prefix, width);
        if parts.is_empty() {
            return Ok(None);
        }
        return Ok(Some(parts));
    }

    if file_name.to_ascii_lowercase().ends_with(".7z") {
        let split_prefix = format!("{}.", file_name);
        let parts = collect_numbered_parts(parent, &split_prefix, 3);
        if !parts.is_empty() {
            return Ok(Some(parts));
        }
    }

    Ok(None)
}

fn concat_parts_to_temp(parts: &[PathBuf], format: ArchiveFormat) -> Result<PreparedArchive> {
    let mut temp_file = NamedTempFile::with_suffix(format.temp_suffix())
        .context("Failed to create temp archive for split volumes")?;

    {
        let out = temp_file.as_file_mut();
        let mut writer = BufWriter::new(out);
        for part in parts {
            let mut input = fs::File::open(part)
                .with_context(|| format!("Failed to open split archive part: {}", part.display()))?;
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
        ArchiveFormat::Zip => collect_zip_split_parts(&normalized)?,
        ArchiveFormat::SevenZ => collect_7z_split_parts(&normalized)?,
        ArchiveFormat::Rar => None,
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

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff", "ico", "pnm", "ppm", "pbm",
    "pam", "avif", "heif", "heic", "qoi",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "webm", "mkv", "avi", "wmv", "flv", "mpg", "mpeg", "ts", "m2ts",
];

const PREVIEWABLE_VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "m4v", "webm", "avi", "mkv"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotMediaItem {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub path: String,
    pub media_type: String,
    pub ext: String,
    pub size: u64,
    pub modified_at: i64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<f64>,
    pub editable: bool,
    pub previewable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotOperationResult {
    pub output_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveEditedImageRequest {
    pub target_path: Option<String>,
}

pub fn list_screenshot_media(xplane_path: &Path) -> Result<Vec<ScreenshotMediaItem>> {
    let screenshots_dir = screenshot_dir(xplane_path);
    if !screenshots_dir.exists() {
        return Ok(Vec::new());
    }

    let mut items: Vec<ScreenshotMediaItem> = Vec::new();
    for entry in fs::read_dir(&screenshots_dir).with_context(|| {
        format!(
            "Failed to read screenshots directory: {}",
            screenshots_dir.display()
        )
    })? {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !file_type.is_file() {
            continue;
        }

        let file_name = match entry.file_name().into_string() {
            Ok(v) => v,
            Err(_) => continue,
        };

        if file_name.starts_with('.') {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_lowercase();

        let media_type = if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            "image"
        } else if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
            "video"
        } else {
            continue;
        };
        let previewable =
            media_type == "image" || PREVIEWABLE_VIDEO_EXTENSIONS.contains(&ext.as_str());

        let metadata = match fs::metadata(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
            .map(|v| v.as_secs() as i64)
            .unwrap_or_default();

        items.push(ScreenshotMediaItem {
            id: format!("{}-{}", file_name, modified_at),
            name: file_name.clone(),
            file_name,
            path: path.to_string_lossy().to_string(),
            media_type: media_type.to_string(),
            ext,
            size: metadata.len(),
            modified_at,
            width: None,
            height: None,
            duration: None,
            editable: media_type == "image",
            previewable,
        });
    }

    items.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()))
    });
    Ok(items)
}

pub fn delete_screenshot_media(
    xplane_path: &Path,
    file_name: &str,
    prefer_trash: bool,
) -> Result<bool> {
    let source = resolve_media_path(xplane_path, file_name)?;
    if prefer_trash && trash::delete(&source).is_ok() {
        return Ok(true);
    }

    fs::remove_file(&source).with_context(|| format!("Failed to delete {}", source.display()))?;
    Ok(false)
}

pub fn save_screenshot_media_as(
    xplane_path: &Path,
    file_name: &str,
    target_path: &Path,
) -> Result<ScreenshotOperationResult> {
    let source = resolve_media_path(xplane_path, file_name)?;
    let target = normalize_target_path(target_path)?;
    if source == target {
        return Ok(ScreenshotOperationResult {
            output_path: target.to_string_lossy().to_string(),
        });
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create folder {}", parent.display()))?;
    }
    fs::copy(&source, &target).with_context(|| {
        format!(
            "Failed to copy screenshot: {} -> {}",
            source.display(),
            target.display()
        )
    })?;

    Ok(ScreenshotOperationResult {
        output_path: target.to_string_lossy().to_string(),
    })
}

pub fn save_edited_screenshot_image(
    xplane_path: &Path,
    file_name: &str,
    bytes: &[u8],
    request: SaveEditedImageRequest,
) -> Result<ScreenshotOperationResult> {
    if bytes.is_empty() {
        return Err(anyhow!("Edited image bytes are empty"));
    }

    let source = resolve_media_path(xplane_path, file_name)?;
    let target = if let Some(target) = request.target_path {
        normalize_target_path(Path::new(&target))?
    } else {
        source
    };

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create folder {}", parent.display()))?;
    }
    fs::write(&target, bytes)
        .with_context(|| format!("Failed to write edited image {}", target.display()))?;

    Ok(ScreenshotOperationResult {
        output_path: target.to_string_lossy().to_string(),
    })
}

pub fn read_screenshot_media_bytes(xplane_path: &Path, file_name: &str) -> Result<Vec<u8>> {
    let source = resolve_media_path(xplane_path, file_name)?;
    fs::read(&source).with_context(|| format!("Failed to read media bytes {}", source.display()))
}

pub fn build_reddit_share_url(
    xplane_path: &Path,
    file_name: &str,
    title: Option<String>,
    _mode: Option<String>,
) -> Result<String> {
    // Validate that the media file exists
    let _source = resolve_media_path(xplane_path, file_name)?;

    let post_title = title
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("X-Plane Screenshot - {}", file_name));

    let url = reqwest::Url::parse_with_params(
        "https://www.reddit.com/r/Xplane/submit",
        &[
            ("title", post_title.as_str()),
            ("type", "IMAGE"),
        ],
    )
    .context("Failed to build Reddit share URL")?;

    Ok(url.to_string())
}

fn screenshot_dir(xplane_path: &Path) -> PathBuf {
    xplane_path.join("Output").join("screenshots")
}

fn resolve_media_path(xplane_path: &Path, file_name: &str) -> Result<PathBuf> {
    validate_file_name(file_name)?;
    let base = screenshot_dir(xplane_path);
    if !base.exists() {
        return Err(anyhow!("Screenshots folder not found"));
    }

    let candidate = base.join(file_name);
    if !candidate.exists() {
        return Err(anyhow!("Screenshot not found: {}", file_name));
    }

    let canonical = crate::path_utils::validate_child_path(&base, &candidate)
        .map_err(|e| anyhow!("Invalid screenshot path: {}", e))?;

    if !canonical.is_file() {
        return Err(anyhow!("Not a file: {}", file_name));
    }

    Ok(canonical)
}

fn validate_file_name(file_name: &str) -> Result<()> {
    if file_name.is_empty()
        || file_name.contains("..")
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        return Err(anyhow!("Invalid file name"));
    }
    Ok(())
}

fn normalize_target_path(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(anyhow!("Target path is empty"));
    }

    if path.is_dir() {
        return Err(anyhow!("Target path must be a file"));
    }

    Ok(path.to_path_buf())
}

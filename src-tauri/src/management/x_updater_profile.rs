use anyhow::{anyhow, Context, Result};
use reqwest::Url;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const XUPDATER_URL_PREFIX: &str = "x-updater:";
const DEFAULT_XUPDATER_HOST: &str = "https://update.x-plane.org";

const XUPDATER_NATIVE_DIR_NAMES: [&str; 3] = ["x-updater", "x_updater", "xupdater"];

// Native x-updater config names observed in real add-ons.
const XUPDATER_PROFILE_CFG_FILES: [&str; 12] = [
    "x-updater.cnf",
    "x_updater.cnf",
    "xupdater.cnf",
    "x-updater.cfg",
    "x_updater.cfg",
    "xupdater.cfg",
    ".x-updater.cfg",
    ".x_updater.cfg",
    "x-updater.conf",
    "x_updater.conf",
    "x-jet-updater.cfg",
    "xjetupdater.cfg",
];

// Keep legacy/custom JSON support for backward compatibility read/write, but never create new JSON.
const XUPDATER_PROFILE_JSON_FILES: [&str; 8] = [
    "x-updater.json",
    "x_updater.json",
    "xupdater.json",
    ".x-updater.json",
    ".x_updater.json",
    "x-updater-profile.json",
    "x_updater_profile.json",
    "xupdater_profile.json",
];

const XUPDATER_NATIVE_MARKER_FILES: [&str; 7] = [
    "client-configuration",
    "productid",
    "xupdignore",
    "description.txt",
    "x-updater.log",
    "x-updater.log.1",
    "x-updater.log.2",
];

#[derive(Debug, Clone)]
pub struct XUpdaterProfile {
    pub host: String,
    pub login: Option<String>,
    pub license_key: Option<String>,
    pub package_version: Option<i64>,
    pub version_label: Option<String>,
    pub ignore_list: Vec<String>,
}

impl XUpdaterProfile {
    pub fn has_credentials(&self) -> bool {
        self.login.is_some() && self.license_key.is_some()
    }
}

pub fn is_profile_file_name(file_name: &str) -> bool {
    let name = file_name.to_lowercase();
    XUPDATER_PROFILE_CFG_FILES.contains(&name.as_str())
        || XUPDATER_PROFILE_JSON_FILES.contains(&name.as_str())
}

pub fn tag_host_as_update_url(host: &str) -> String {
    format!("{}{}", XUPDATER_URL_PREFIX, normalize_host(Some(host)))
}

pub fn parse_tagged_update_url(update_url: &str) -> Option<String> {
    if !update_url
        .to_lowercase()
        .starts_with(&XUPDATER_URL_PREFIX.to_lowercase())
    {
        return None;
    }
    let host = update_url[XUPDATER_URL_PREFIX.len()..].trim();
    if host.is_empty() {
        return Some(DEFAULT_XUPDATER_HOST.to_string());
    }
    Some(normalize_host(Some(host)))
}

pub fn find_profile_in_folder(folder: &Path) -> Option<XUpdaterProfile> {
    let candidates = collect_profile_candidate_paths(folder);
    let mut fallback_profile: Option<XUpdaterProfile> = None;

    for path in candidates {
        if let Some(profile) = parse_profile_file(&path) {
            if profile.has_credentials() {
                return Some(profile);
            }
            if fallback_profile.is_none() {
                fallback_profile = Some(profile);
            }
        }
    }

    if fallback_profile.is_some() {
        return fallback_profile;
    }

    // Native x-updater may exist without credentials yet (e.g., only productId + client-configuration).
    if has_native_updater_layout(folder) {
        return Some(XUpdaterProfile {
            host: DEFAULT_XUPDATER_HOST.to_string(),
            login: None,
            license_key: None,
            package_version: None,
            version_label: None,
            ignore_list: Vec::new(),
        });
    }

    None
}

pub fn write_credentials_in_folder(folder: &Path, login: &str, license_key: &str) -> Result<()> {
    let trimmed_login = login.trim();
    let trimmed_license_key = license_key.trim();
    if trimmed_login.is_empty() || trimmed_license_key.is_empty() {
        return Err(anyhow!("Missing account or activation key"));
    }

    fs::create_dir_all(folder)
        .with_context(|| format!("Failed to create folder '{}'", folder.display()))?;

    let profile_path = resolve_native_credentials_path(folder)?;
    write_cfg_credentials(&profile_path, trimmed_login, trimmed_license_key)
}

fn resolve_native_credentials_path(folder: &Path) -> Result<PathBuf> {
    if let Some(path) = find_preferred_cfg_path(folder) {
        return Ok(path);
    }

    let native_dir = ensure_native_updater_dir(folder)?;
    Ok(native_dir.join("x-updater.cnf"))
}

fn find_preferred_cfg_path(folder: &Path) -> Option<PathBuf> {
    let candidates = collect_profile_candidate_paths(folder);
    let mut best: Option<(i32, PathBuf)> = None;

    for path in candidates {
        if !is_cfg_file_path(&path) {
            continue;
        }

        let mut score: i32 = 0;
        if is_path_inside_native_dir(folder, &path) {
            score += 100;
        }
        if path
            .file_name()
            .map(|v| v.to_string_lossy().to_lowercase() == "x-updater.cnf")
            .unwrap_or(false)
        {
            score += 20;
        }
        if parse_profile_file(&path)
            .map(|p| p.has_credentials())
            .unwrap_or(false)
        {
            score += 10;
        }

        match &best {
            Some((current, _)) if score <= *current => {}
            _ => best = Some((score, path)),
        }
    }

    best.map(|(_, path)| path)
}

fn ensure_native_updater_dir(folder: &Path) -> Result<PathBuf> {
    if is_native_updater_dir(folder) {
        fs::create_dir_all(folder)
            .with_context(|| format!("Failed to create '{}'", folder.display()))?;
        return Ok(folder.to_path_buf());
    }

    if let Some(dir) = find_native_updater_dirs(folder).into_iter().next() {
        fs::create_dir_all(&dir).with_context(|| format!("Failed to create '{}'", dir.display()))?;
        return Ok(dir);
    }

    let dir = folder.join("x-updater");
    fs::create_dir_all(&dir).with_context(|| format!("Failed to create '{}'", dir.display()))?;
    Ok(dir)
}

fn collect_profile_candidate_paths(folder: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut seen = HashSet::new();

    let mut candidate_dirs: Vec<PathBuf> = Vec::new();

    if is_native_updater_dir(folder) {
        candidate_dirs.push(folder.to_path_buf());
    }
    candidate_dirs.extend(find_native_updater_dirs(folder));
    candidate_dirs.push(folder.to_path_buf());

    for dir in candidate_dirs {
        let read_dir = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in read_dir.flatten() {
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if !file_type.is_file() {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            if !is_profile_file_name(&file_name) {
                continue;
            }
            let path = entry.path();
            let key = path.to_string_lossy().to_lowercase();
            if seen.insert(key) {
                out.push(path);
            }
        }
    }

    out
}

fn find_native_updater_dirs(folder: &Path) -> Vec<PathBuf> {
    let read_dir = match fs::read_dir(folder) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut dirs = Vec::new();
    for entry in read_dir.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !file_type.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_lowercase();
        if XUPDATER_NATIVE_DIR_NAMES.contains(&name.as_str()) {
            dirs.push(entry.path());
        }
    }

    dirs
}

fn is_native_updater_dir(path: &Path) -> bool {
    path.file_name()
        .map(|v| {
            let name = v.to_string_lossy().to_lowercase();
            XUPDATER_NATIVE_DIR_NAMES.contains(&name.as_str())
        })
        .unwrap_or(false)
}

fn is_path_inside_native_dir(folder: &Path, path: &Path) -> bool {
    let native_dirs = find_native_updater_dirs(folder);
    native_dirs.iter().any(|dir| path.starts_with(dir))
}

fn has_native_updater_layout(folder: &Path) -> bool {
    if is_native_updater_dir(folder) {
        return true;
    }

    if !find_native_updater_dirs(folder).is_empty() {
        return true;
    }

    let read_dir = match fs::read_dir(folder) {
        Ok(rd) => rd,
        Err(_) => return false,
    };

    for entry in read_dir.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if is_profile_file_name(&name) || XUPDATER_NATIVE_MARKER_FILES.contains(&name.as_str()) {
            return true;
        }
    }
    false
}

fn is_cfg_file_path(path: &Path) -> bool {
    path.file_name()
        .map(|v| {
            let name = v.to_string_lossy().to_lowercase();
            XUPDATER_PROFILE_CFG_FILES.contains(&name.as_str())
        })
        .unwrap_or(false)
}

fn write_cfg_credentials(path: &Path, login: &str, license_key: &str) -> Result<()> {
    const LOGIN_KEYS: [&str; 4] = ["login", "username", "user", "email"];
    const LICENSE_KEYS: [&str; 6] = [
        "licensekey",
        "license_key",
        "key",
        "license",
        "password",
        "token",
    ];
    const HOST_KEYS: [&str; 7] = [
        "host",
        "server",
        "update_host",
        "updatehost",
        "base_url",
        "baseurl",
        "url",
    ];

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }

    let text = if path.exists() {
        fs::read_to_string(path)
            .with_context(|| format!("Failed to read profile '{}'", path.display()))?
    } else {
        String::new()
    };

    let mut default_sep = '=';
    for raw_line in text.lines() {
        if let Some((_key, _value, sep)) = split_cfg_line(raw_line.trim()) {
            default_sep = sep;
            break;
        }
    }

    let mut output_lines = Vec::new();
    let mut login_written = false;
    let mut key_written = false;
    let mut host_found = false;
    let mut config_version_found = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            output_lines.push(raw_line.to_string());
            continue;
        }

        if let Some((key_raw, _value_raw, sep)) = split_cfg_line(raw_line) {
            let key = key_raw.trim().to_lowercase();
            if LOGIN_KEYS.contains(&key.as_str()) {
                output_lines.push(format!(
                    "{}{}{}",
                    key_raw.trim(),
                    sep,
                    encode_cfg_value(login, sep)
                ));
                login_written = true;
                continue;
            }
            if LICENSE_KEYS.contains(&key.as_str()) {
                output_lines.push(format!(
                    "{}{}{}",
                    key_raw.trim(),
                    sep,
                    encode_cfg_value(license_key, sep)
                ));
                key_written = true;
                continue;
            }
            if HOST_KEYS.contains(&key.as_str()) {
                host_found = true;
            }
            if key == "configversion" {
                config_version_found = true;
            }
        }

        output_lines.push(raw_line.to_string());
    }

    if output_lines.is_empty() {
        output_lines.push("#ClientApp properties".to_string());
    }

    if !host_found {
        output_lines.push(format!(
            "host{}{}",
            default_sep,
            encode_cfg_value(DEFAULT_XUPDATER_HOST, default_sep)
        ));
    }
    if !login_written {
        output_lines.push(format!(
            "login{}{}",
            default_sep,
            encode_cfg_value(login, default_sep)
        ));
    }
    if !key_written {
        output_lines.push(format!(
            "key{}{}",
            default_sep,
            encode_cfg_value(license_key, default_sep)
        ));
    }
    if !config_version_found {
        output_lines.push(format!("configVersion{}3", default_sep));
    }

    let mut out = output_lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }

    fs::write(path, out).with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn encode_cfg_value(value: &str, sep: char) -> String {
    if sep != '=' {
        return value.to_string();
    }

    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ':' => out.push_str("\\:"),
            '=' => out.push_str("\\="),
            _ => out.push(ch),
        }
    }
    out
}

fn split_cfg_line(line: &str) -> Option<(&str, &str, char)> {
    if let Some((k, v)) = line.split_once('|') {
        return Some((k, v, '|'));
    }
    if let Some((k, v)) = line.split_once('=') {
        return Some((k, v, '='));
    }
    None
}

fn parse_profile_file(path: &Path) -> Option<XUpdaterProfile> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    if XUPDATER_PROFILE_JSON_FILES.contains(&name.as_str()) {
        return parse_profile_json(path);
    }
    parse_profile_cfg(path)
}

fn parse_profile_json(path: &Path) -> Option<XUpdaterProfile> {
    let text = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;

    let root = value.as_object()?;
    let body = lookup_object(root, "profile").unwrap_or(root);

    let auth = lookup_object(body, "auth");

    let host = get_string(
        body,
        &[
            "host",
            "server",
            "updateHost",
            "baseUrl",
            "base_url",
            "url",
            "module",
        ],
    );
    let login = get_string(body, &["login", "username", "user", "email"]).or_else(|| {
        auth.and_then(|obj| get_string(obj, &["login", "username", "user", "email"]))
    });
    let license_key = get_string(
        body,
        &[
            "licenseKey",
            "license_key",
            "key",
            "license",
            "password",
            "token",
        ],
    )
    .or_else(|| {
        auth.and_then(|obj| {
            get_string(
                obj,
                &[
                    "licenseKey",
                    "license_key",
                    "key",
                    "license",
                    "password",
                    "token",
                ],
            )
        })
    });

    let package_version = get_i64(
        body,
        &[
            "packageVersion",
            "package_version",
            "package",
            "since",
            "revision",
            "snapshotNum",
            "snapshot_num",
        ],
    )
    .or_else(|| get_i64(body, &["version"]));

    let version_label = get_string(body, &["version"]);

    let ignore_list = get_ignore_list(body, &["ignoreList", "ignore_list", "ignore", "exclude"]);

    Some(XUpdaterProfile {
        host: normalize_host(host.as_deref()),
        login,
        license_key,
        package_version,
        version_label,
        ignore_list,
    })
}

fn parse_profile_cfg(path: &Path) -> Option<XUpdaterProfile> {
    let text = fs::read_to_string(path).ok()?;
    let mut host: Option<String> = None;
    let mut login: Option<String> = None;
    let mut license_key: Option<String> = None;
    let mut package_version: Option<i64> = None;
    let mut version_label: Option<String> = None;
    let mut ignore_list: Vec<String> = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let (key_raw, value_raw) = if let Some((k, v)) = line.split_once('|') {
            (k, v)
        } else if let Some((k, v)) = line.split_once('=') {
            (k, v)
        } else {
            continue;
        };

        let key = key_raw.trim().to_lowercase();
        let value = parse_cfg_value(value_raw);
        if value.is_empty() {
            continue;
        }

        match key.as_str() {
            "host" | "server" | "update_host" | "updatehost" | "base_url" | "baseurl" | "url"
            | "module" => host = Some(value),
            "login" | "username" | "user" | "email" => login = Some(value),
            "licensekey" | "license_key" | "key" | "license" | "password" | "token" => {
                license_key = Some(value)
            }
            "package_version"
            | "packageversion"
            | "package"
            | "since"
            | "revision"
            | "snapshotnum"
            | "snapshot_num" => {
                package_version = parse_i64(&value)
            }
            "version" => {
                version_label = Some(value.clone());
                if package_version.is_none() {
                    package_version = parse_i64(&value);
                }
            }
            "ignore" | "ignore_list" | "ignorelist" | "exclude" | "excludes" => {
                ignore_list.extend(parse_ignore_entries(&value));
            }
            _ => {}
        }
    }

    Some(XUpdaterProfile {
        host: normalize_host(host.as_deref()),
        login,
        license_key,
        package_version,
        version_label,
        ignore_list: dedup_ignore_list(ignore_list),
    })
}

fn parse_cfg_value(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'').trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut escaped = false;
    for ch in trimmed.chars() {
        if escaped {
            match ch {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                _ => out.push(ch),
            }
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        out.push(ch);
    }
    if escaped {
        out.push('\\');
    }
    out.trim().to_string()
}

fn lookup_object<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Option<&'a serde_json::Map<String, Value>> {
    lookup_value(object, key)?.as_object()
}

fn lookup_value<'a>(object: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a Value> {
    if let Some(v) = object.get(key) {
        return Some(v);
    }
    object
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
}

fn get_string(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = lookup_value(object, key) {
            if let Some(s) = value.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

fn get_i64(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<i64> {
    for key in keys {
        if let Some(value) = lookup_value(object, key) {
            if let Some(v) = value.as_i64() {
                return Some(v);
            }
            if let Some(v) = value.as_u64() {
                if v <= i64::MAX as u64 {
                    return Some(v as i64);
                }
            }
            if let Some(s) = value.as_str() {
                if let Some(parsed) = parse_i64(s) {
                    return Some(parsed);
                }
            }
        }
    }
    None
}

fn get_ignore_list(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(value) = lookup_value(object, key) {
            if let Some(array) = value.as_array() {
                let mut values = Vec::new();
                for item in array {
                    if let Some(s) = item.as_str() {
                        values.extend(parse_ignore_entries(s));
                    }
                }
                return dedup_ignore_list(values);
            }
            if let Some(s) = value.as_str() {
                return dedup_ignore_list(parse_ignore_entries(s));
            }
        }
    }
    Vec::new()
}

fn parse_ignore_entries(raw: &str) -> Vec<String> {
    raw.split(&[',', ';', '\n'][..])
        .filter_map(|part| {
            let value = normalize_ignore_path(part);
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        })
        .collect()
}

fn dedup_ignore_list(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut dedup = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            dedup.push(value);
        }
    }
    dedup
}

fn normalize_ignore_path(raw: &str) -> String {
    let mut value = raw.trim().replace('\\', "/");
    while value.starts_with("./") {
        value = value[2..].to_string();
    }
    value.trim_start_matches('/').trim().to_string()
}

fn parse_i64(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(v) = trimmed.parse::<i64>() {
        return Some(v);
    }
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<i64>().ok()
}

fn normalize_host(raw: Option<&str>) -> String {
    let mut host = raw.unwrap_or(DEFAULT_XUPDATER_HOST).trim().to_string();
    if host.is_empty() {
        host = DEFAULT_XUPDATER_HOST.to_string();
    }
    if !host.starts_with("http://") && !host.starts_with("https://") {
        host = format!("https://{}", host);
    }
    if let Ok(mut url) = Url::parse(&host) {
        match url.scheme() {
            "http" | "https" => {}
            _ => {
                return DEFAULT_XUPDATER_HOST.to_string();
            }
        }
        url.set_fragment(None);
        return url.as_str().trim_end_matches('/').to_string();
    }
    DEFAULT_XUPDATER_HOST.to_string()
}

//! Livery detection patterns for various aircraft types
//!
//! This module defines patterns to detect aircraft liveries and map them
//! to their corresponding aircraft types.

use serde::Deserialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use crate::logger;

/// Represents a single detection rule for livery identification
#[derive(Debug, Clone, Deserialize)]
pub struct DetectionRule {
    /// Pattern type: "path" for folder path matching, "file" for filename matching
    pub pattern_type: String,
    /// The pattern to match (folder path or filename glob)
    /// Supports '*' (0+ chars) and '?' (0-1 chars) wildcards
    pub pattern: String,
    /// Lowercased pattern for case-insensitive matching
    #[serde(skip)]
    pub pattern_lower: String,
}

/// Represents a livery pattern definition
#[derive(Debug, Clone, Deserialize)]
pub struct LiveryPattern {
    /// Unique identifier for the aircraft type (e.g., "FF777")
    pub aircraft_type_id: String,
    /// Human-readable name for the aircraft
    pub aircraft_name: String,
    /// Detection rules - any match identifies the livery
    pub detection_rules: Vec<DetectionRule>,
    /// ACF file name patterns that identify this aircraft (without extension)
    /// Supports '*' (0+ chars) and '?' (0-1 chars) wildcards, case-insensitive
    pub acf_identifiers: Vec<String>,
    /// Lowercased ACF identifiers for case-insensitive matching
    #[serde(skip)]
    pub acf_identifiers_lower: Vec<String>,
}

/// Remote JSON schema for livery patterns
#[derive(Debug, Deserialize)]
struct LiveryPatternsData {
    #[allow(dead_code)]
    version: u32,
    #[allow(dead_code)]
    updated: String,
    patterns: Vec<LiveryPattern>,
}

/// Remote URL for the livery patterns JSON file via proxy
const REMOTE_URL: &str = "https://x-fast-manager.vercel.app/api/livery-patterns-data";

/// Loaded livery patterns (embedded by default, remote override when available)
static LIVERY_PATTERNS: LazyLock<RwLock<Vec<LiveryPattern>>> =
    LazyLock::new(|| RwLock::new(load_embedded_patterns()));

/// Ensure we only attempt remote fetch once per startup
static REMOTE_FETCHED: AtomicBool = AtomicBool::new(false);

fn prepare_patterns(mut patterns: Vec<LiveryPattern>) -> Vec<LiveryPattern> {
    for pattern in &mut patterns {
        for rule in &mut pattern.detection_rules {
            rule.pattern_type = rule.pattern_type.trim().to_lowercase();
            rule.pattern_lower = rule.pattern.to_lowercase();
        }
        pattern.acf_identifiers_lower = pattern
            .acf_identifiers
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
    }

    patterns
}

fn load_embedded_patterns() -> Vec<LiveryPattern> {
    let embedded_json = include_str!("../../../data/livery_patterns.json");

    match serde_json::from_str::<LiveryPatternsData>(embedded_json) {
        Ok(data) => prepare_patterns(data.patterns),
        Err(e) => {
            logger::log_info(
                &format!(
                    "Failed to parse embedded data/livery_patterns.json: {}, using empty fallback",
                    e
                ),
                Some("livery_patterns"),
            );
            Vec::new()
        }
    }
}

async fn fetch_remote_patterns() -> Result<Vec<LiveryPattern>, String> {
    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let remote_url =
        std::env::var("XFAST_LIVERY_PATTERNS_API_URL").unwrap_or_else(|_| REMOTE_URL.to_string());
    let response = client
        .get(&remote_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP status: {}", response.status()));
    }

    let data: LiveryPatternsData = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(prepare_patterns(data.patterns))
}

/// Ensure livery patterns are loaded from remote once per startup
pub async fn ensure_patterns_loaded() {
    if REMOTE_FETCHED.swap(true, Ordering::SeqCst) {
        return;
    }

    match fetch_remote_patterns().await {
        Ok(patterns) if !patterns.is_empty() => {
            let mut guard = LIVERY_PATTERNS
                .write()
                .expect("livery patterns lock poisoned during remote fetch");
            *guard = patterns;
            logger::log_info(
                "Fetched livery patterns from remote",
                Some("livery_patterns"),
            );
        }
        Ok(_) => {
            logger::log_info(
                "Remote livery patterns were empty; keeping embedded data",
                Some("livery_patterns"),
            );
        }
        Err(e) => {
            logger::log_info(
                &format!(
                    "Failed to fetch remote livery patterns: {}, using embedded data",
                    e
                ),
                Some("livery_patterns"),
            );
        }
    }
}

/// Check if a path matches a livery pattern
/// Returns (aircraft_type_id, livery_root_path) if matched
pub fn check_livery_pattern(file_path: &str) -> Option<(String, String)> {
    let normalized = file_path.replace('\\', "/");
    let normalized_lower = normalized.to_lowercase();

    let patterns = LIVERY_PATTERNS
        .read()
        .expect("livery patterns lock poisoned during pattern check");
    for pattern in patterns.iter() {
        for rule in pattern.detection_rules.iter() {
            match rule.pattern_type.as_str() {
                "path" => {
                    // Path-based detection: look for folder path pattern
                    if let Some(pos) = normalized_lower.find(&rule.pattern_lower) {
                        let prefix = &normalized[..pos];
                        let livery_root = if prefix.is_empty() {
                            String::new()
                        } else {
                            prefix.trim_end_matches('/').to_string()
                        };
                        return Some((pattern.aircraft_type_id.clone(), livery_root));
                    }
                }
                "file" => {
                    // File-based detection: match filename with glob pattern
                    if let Some(livery_root) =
                        match_file_pattern(&normalized, &normalized_lower, rule)
                    {
                        return Some((pattern.aircraft_type_id.clone(), livery_root));
                    }
                }
                _ => {}
            }
        }
    }

    None
}

/// Match a file pattern and return the livery root if matched
fn match_file_pattern(
    normalized: &str,
    normalized_lower: &str,
    rule: &DetectionRule,
) -> Option<String> {
    let pattern_lower = &rule.pattern_lower;

    // Check if pattern contains a path separator (e.g., "objects/fuselage319*.png")
    if pattern_lower.contains('/') {
        // Split pattern into path and filename parts
        let pattern_parts: Vec<&str> = pattern_lower.split('/').collect();
        let pattern_filename = pattern_parts.last()?;

        // Split the normalized path
        let path_parts: Vec<&str> = normalized_lower.split('/').collect();

        // Find where the pattern path matches in the normalized path
        for i in 0..path_parts.len().saturating_sub(pattern_parts.len() - 1) {
            // Check if the path components match (supporting glob in directory names)
            let mut path_matches = true;
            for (j, pattern_part) in pattern_parts[..pattern_parts.len() - 1].iter().enumerate() {
                if i + j >= path_parts.len() || !matches_glob(pattern_part, path_parts[i + j]) {
                    path_matches = false;
                    break;
                }
            }

            if path_matches {
                // Check if the filename matches (with glob)
                let filename_idx = i + pattern_parts.len() - 1;
                if filename_idx < path_parts.len()
                    && matches_glob(pattern_filename, path_parts[filename_idx])
                {
                    // Found a match! Livery root is everything before the first matched component
                    let original_parts: Vec<&str> = normalized.split('/').collect();
                    let livery_root = if i > 0 {
                        original_parts[..i].join("/")
                    } else {
                        String::new()
                    };

                    return Some(livery_root);
                }
            }
        }
    } else {
        // Pattern is just a filename, possibly with glob
        // Extract the filename from the path
        let file_name = normalized_lower.rsplit('/').next()?;

        if matches_glob(pattern_lower, file_name) {
            // Livery root is the file's parent folder
            let parts: Vec<&str> = normalized.split('/').collect();
            if parts.len() > 1 {
                return Some(parts[..parts.len() - 1].join("/"));
            }
        }
    }

    None
}

/// Glob matching supporting '*' (0+ chars) and '?' (0-1 chars) wildcards
fn matches_glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let plen = p.len();
    let tlen = t.len();

    // dp[i][j] = pattern[0..i] matches text[0..j]
    let mut dp = vec![vec![false; tlen + 1]; plen + 1];
    dp[0][0] = true;

    // Handle leading wildcards matching empty text
    for i in 1..=plen {
        if p[i - 1] == '*' || p[i - 1] == '?' {
            dp[i][0] = dp[i - 1][0];
        }
    }

    for i in 1..=plen {
        for j in 1..=tlen {
            match p[i - 1] {
                '*' => {
                    // '*' matches zero or more characters
                    dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
                }
                '?' => {
                    // '?' matches zero or one character
                    dp[i][j] = dp[i - 1][j] || dp[i - 1][j - 1];
                }
                c => {
                    // Literal character must match exactly
                    dp[i][j] = dp[i - 1][j - 1] && c == t[j - 1];
                }
            }
        }
    }

    dp[plen][tlen]
}

/// Check if an ACF file name matches any known aircraft type
/// Returns the aircraft_type_id if matched
pub fn check_acf_identifier(acf_file_name: &str) -> Option<String> {
    // Remove extension if present
    let name = Path::new(acf_file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(acf_file_name);

    let name_lower = name.to_lowercase();

    let patterns = LIVERY_PATTERNS
        .read()
        .expect("livery patterns lock poisoned during ACF identifier check");
    for pattern in patterns.iter() {
        for identifier in pattern.acf_identifiers_lower.iter() {
            if matches_glob(identifier, &name_lower) {
                return Some(pattern.aircraft_type_id.clone());
            }
        }
    }

    None
}

/// Get the human-readable name for an aircraft type
pub fn get_aircraft_name(aircraft_type_id: &str) -> Option<String> {
    let patterns = LIVERY_PATTERNS
        .read()
        .expect("livery patterns lock poisoned during aircraft name lookup");
    patterns
        .iter()
        .find(|p| p.aircraft_type_id == aircraft_type_id)
        .map(|p| p.aircraft_name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_livery_pattern_ff777() {
        // Test FF777 livery detection
        let result = check_livery_pattern("MyLivery/objects/777/texture.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "FF_B777");
        assert_eq!(root, "MyLivery");

        // Test with backslashes
        let result = check_livery_pattern("MyLivery\\objects\\777\\texture.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "FF_B777");
        assert_eq!(root, "MyLivery");
    }

    #[test]
    fn test_check_livery_pattern_toliss_a319() {
        // Test Toliss A319 icon detection - basic pattern
        let result = check_livery_pattern("MyLivery/a319_icon11.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A319");
        assert_eq!(root, "MyLivery");

        // Test Toliss A319 icon detection - with prefix before icon11
        let result = check_livery_pattern("MyLivery/a319_neo_icon11.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A319");
        assert_eq!(root, "MyLivery");

        // Test Toliss A319 icon detection - with suffix after icon11
        let result = check_livery_pattern("MyLivery/a319_icon11_hd.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A319");
        assert_eq!(root, "MyLivery");

        // Test Toliss A319 fuselage detection (png)
        let result = check_livery_pattern("MyLivery/objects/fuselage319.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A319");
        assert_eq!(root, "MyLivery");

        // Test Toliss A319 fuselage detection (dds)
        let result = check_livery_pattern("MyLivery/objects/fuselage319.dds");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A319");
        assert_eq!(root, "MyLivery");
    }

    #[test]
    fn test_check_livery_pattern_toliss_a320() {
        // Test Toliss A320 icon detection
        let result = check_livery_pattern("MyLivery/a320_icon11.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A320");
        assert_eq!(root, "MyLivery");

        // Test Toliss A320 fuselage detection
        let result = check_livery_pattern("MyLivery/objects/fuselage320.dds");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A320");
        assert_eq!(root, "MyLivery");

        // Test Toliss A320 fuselage with suffix
        let result = check_livery_pattern("MyLivery/objects/fuselage320_neo.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A320");
        assert_eq!(root, "MyLivery");

        // Test Toliss A320 LEAP1A engine detection
        let result = check_livery_pattern("MyLivery/objects/LEAP1A.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A320");
        assert_eq!(root, "MyLivery");

        // Test Toliss A320 LEAP1A engine detection (dds)
        let result = check_livery_pattern("MyLivery/objects/LEAP1A.dds");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "TOLISS_A320");
        assert_eq!(root, "MyLivery");
    }

    #[test]
    fn test_check_livery_pattern_no_match() {
        // Test non-matching path
        let result = check_livery_pattern("SomeFolder/textures/image.png");
        assert!(result.is_none());
    }

    #[test]
    fn test_matches_glob() {
        // Test pattern with multiple wildcards: a319_*icon11*.png
        assert!(matches_glob("a319_*icon11*.png", "a319_icon11.png"));
        assert!(matches_glob("a319_*icon11*.png", "a319_neo_icon11.png"));
        assert!(matches_glob("a319_*icon11*.png", "a319_icon11_hd.png"));
        assert!(matches_glob("a319_*icon11*.png", "a319_neo_icon11_hd.png"));
        assert!(!matches_glob("a319_*icon11*.png", "a320_icon11.png"));
        assert!(!matches_glob("a319_*icon11*.png", "a319_icon11.dds"));
        // Test exact match
        assert!(matches_glob("fuselage319.png", "fuselage319.png"));
        assert!(!matches_glob("fuselage319.png", "fuselage320.png"));
        // Test '?' wildcard (matches 0 or 1 character)
        assert!(matches_glob("test?.png", "test.png")); // 0 chars
        assert!(matches_glob("test?.png", "testA.png")); // 1 char
        assert!(!matches_glob("test?.png", "testAB.png")); // 2 chars - no match
        assert!(matches_glob("a?b", "ab")); // ? matches 0
        assert!(matches_glob("a?b", "axb")); // ? matches 1
        assert!(!matches_glob("a?b", "axxb")); // ? can't match 2
                                               // Test combined * and ?
        assert!(matches_glob("737_*NG", "737_80NG"));
        assert!(matches_glob("737_*NG", "737_9ENG"));
    }

    #[test]
    fn test_check_livery_pattern_levelup_b737() {
        // Test LevelUp B737 with glob directory matching
        let result = check_livery_pattern("MyLivery/objects/737_80NG/737_80NG_fuselage.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "LEVELUP_B737");
        assert_eq!(root, "MyLivery");

        // Test with dds
        let result = check_livery_pattern("MyLivery/objects/737_90NG/737_90NG_wing.dds");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "LEVELUP_B737");
        assert_eq!(root, "MyLivery");
    }

    #[test]
    fn test_check_acf_identifier() {
        // FF_B777
        assert_eq!(
            check_acf_identifier("777-200ER.acf"),
            Some("FF_B777".to_string())
        );
        assert_eq!(
            check_acf_identifier("777-200ER_xp12"),
            Some("FF_B777".to_string())
        );
        assert_eq!(
            check_acf_identifier("777-F_xp12_lo.acf"),
            Some("FF_B777".to_string())
        );
        // Toliss
        assert_eq!(
            check_acf_identifier("a319.acf"),
            Some("TOLISS_A319".to_string())
        );
        assert_eq!(
            check_acf_identifier("a320_StdDef.acf"),
            Some("TOLISS_A320".to_string())
        );
        assert_eq!(
            check_acf_identifier("a321_XP11.acf"),
            Some("TOLISS_A321".to_string())
        );
        assert_eq!(
            check_acf_identifier("A330-900.acf"),
            Some("TOLISS_A339".to_string())
        );
        // Unknown
        assert_eq!(check_acf_identifier("unknown.acf"), None);
    }
}

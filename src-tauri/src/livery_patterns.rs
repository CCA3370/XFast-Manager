//! Livery detection patterns for various aircraft types
//!
//! This module defines patterns to detect aircraft liveries and map them
//! to their corresponding aircraft types.

use std::path::Path;

/// Represents a single detection rule for livery identification
#[derive(Debug, Clone)]
pub struct DetectionRule {
    /// Pattern type: "path" for folder path matching, "file" for filename matching
    pub pattern_type: &'static str,
    /// The pattern to match (folder path or filename glob)
    pub pattern: &'static str,
    /// How many parent levels to go up from the match to find livery root
    /// For "path" type: 0 means parent of matched path component
    /// For "file" type: 0 means the file's parent folder, 1 means grandparent, etc.
    pub parent_levels: usize,
}

/// Represents a livery pattern definition
#[derive(Debug, Clone)]
pub struct LiveryPattern {
    /// Unique identifier for the aircraft type (e.g., "FF777")
    pub aircraft_type_id: &'static str,
    /// Human-readable name for the aircraft
    pub aircraft_name: &'static str,
    /// Detection rules - any match identifies the livery
    pub detection_rules: &'static [DetectionRule],
    /// ACF file names that identify this aircraft (without extension)
    pub acf_identifiers: &'static [&'static str],
}

/// All registered livery patterns
pub static LIVERY_PATTERNS: &[LiveryPattern] = &[
    LiveryPattern {
        aircraft_type_id: "FF_B777",
        aircraft_name: "FlightFactor 777v2",
        detection_rules: &[DetectionRule {
            pattern_type: "path",
            pattern: "objects/777",
            parent_levels: 0,
        }],
        acf_identifiers: &[
            "777-200ER",
            "777-200ER_xp12",
            "777-200ER_xp12_lo",
            "777-200LR",
            "777-200LR_xp12",
            "777-200LR_xp12_lo",
            "777-300ER",
            "777-300ER_xp12",
            "777-300ER_xp12_lo",
            "777-F",
            "777-F_xp12",
            "777-F_xp12_lo",
        ],
    },
    LiveryPattern {
        aircraft_type_id: "TOLISS_A319",
        aircraft_name: "ToLiss A319",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "a319_*icon11*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage319*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage319*.dds",
                parent_levels: 0,
            },
        ],
        acf_identifiers: &[
            "a319", 
            "a319_StdDef", 
            "a319_XP11", 
            "a319_XP11_StdDef"
        ],
    },
    LiveryPattern {
        aircraft_type_id: "TOLISS_A320",
        aircraft_name: "ToLiss A320",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "a320_*icon11*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage320*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage320*.dds",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/LEAP1A.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/LEAP1A.dds",
                parent_levels: 0,
            },
        ],
        acf_identifiers: &[
            "a320", 
            "a320_StdDef", 
            "a320_XP11", 
            "a320_XP11_StdDef"
        ],
    },
    LiveryPattern {
        aircraft_type_id: "TOLISS_A321",
        aircraft_name: "ToLiss A321",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "a321_*icon11*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage321*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage321*.dds",
                parent_levels: 0,
            },
        ],
        acf_identifiers: &[
            "a321", 
            "a321_StdDef", 
            "a321_XP11", 
            "a321_XP11_StdDef"
        ],
    },
    LiveryPattern {
        aircraft_type_id: "TOLISS_A339",
        aircraft_name: "ToLiss A339",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "A330-900_*icon11*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/A339_Engines.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/A339_Engines.dds",
                parent_levels: 0,
            },
        ],
        acf_identifiers: &[
            "A330-900",
            "A330-900_stdDef",
            "A330-900_XP11",
            "A330-900_XP11_stdDef",
        ],
    },
    LiveryPattern {
        aircraft_type_id: "TOLISS_A346",
        aircraft_name: "ToLiss A346",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "A340-600_*icon11*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/EngineA346*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/EngineA346*.dds",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/FuselageA346*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/FuselageA346*.dds",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/StabilizersA346*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/StabilizersA346*.dds",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/WING*A346.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/WING*A346.dds",
                parent_levels: 0,
            },
        ],
        acf_identifiers: &[
            "A340-600",
            "A340-600_stdDef",
            "A340-600_XP11",
            "A340-600_XP11_stdDef",
        ],
    },
    LiveryPattern {
        aircraft_type_id: "ZIBO_B738",
        aircraft_name: "Zibo B738",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "b738_*icon11*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/738*.png",
                parent_levels: 0,
            },
        ],
        acf_identifiers: &[
            "b738",
            "b738_4k",
        ],
    },/*
    LiveryPattern {
        aircraft_type_id: "X-CRAFTS_E170",
        aircraft_name: "X-Crafts E170",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "e170_*icon11*.png",
                parent_levels: 0,
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/e170*.png",
                parent_levels: 0,
            },
        ],
        acf_identifiers: &[
            "e170",
            "e170_4k",
        ],
    },*/
];

/// Check if a path matches a livery pattern
/// Returns (aircraft_type_id, livery_root_path) if matched
pub fn check_livery_pattern(file_path: &str) -> Option<(&'static str, String)> {
    let normalized = file_path.replace('\\', "/");
    let normalized_lower = normalized.to_lowercase();

    for pattern in LIVERY_PATTERNS {
        for rule in pattern.detection_rules {
            match rule.pattern_type {
                "path" => {
                    // Path-based detection: look for folder path pattern
                    let pattern_lower = rule.pattern.to_lowercase();
                    if let Some(pos) = normalized_lower.find(&pattern_lower) {
                        let prefix = &normalized[..pos];
                        let livery_root = if prefix.is_empty() {
                            String::new()
                        } else {
                            prefix.trim_end_matches('/').to_string()
                        };
                        return Some((pattern.aircraft_type_id, livery_root));
                    }
                }
                "file" => {
                    // File-based detection: match filename with glob pattern
                    if let Some(livery_root) =
                        match_file_pattern(&normalized, &normalized_lower, rule)
                    {
                        return Some((pattern.aircraft_type_id, livery_root));
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
    let pattern_lower = rule.pattern.to_lowercase();

    // Check if pattern contains a path separator (e.g., "objects/fuselage319*.png")
    if pattern_lower.contains('/') {
        // Split pattern into path and filename parts
        let pattern_parts: Vec<&str> = pattern_lower.split('/').collect();
        let pattern_filename = pattern_parts.last()?;

        // Split the normalized path
        let path_parts: Vec<&str> = normalized_lower.split('/').collect();

        // Find where the pattern path matches in the normalized path
        for i in 0..path_parts.len().saturating_sub(pattern_parts.len() - 1) {
            // Check if the path components match
            let mut path_matches = true;
            for (j, pattern_part) in pattern_parts[..pattern_parts.len() - 1].iter().enumerate() {
                if i + j >= path_parts.len() || path_parts[i + j] != *pattern_part {
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
                    // Found a match! Calculate livery root
                    let original_parts: Vec<&str> = normalized.split('/').collect();
                    let mut livery_root = if i > 0 {
                        original_parts[..i].join("/")
                    } else {
                        String::new()
                    };

                    // Apply parent_levels
                    for _ in 0..rule.parent_levels {
                        if let Some(last_slash) = livery_root.rfind('/') {
                            livery_root = livery_root[..last_slash].to_string();
                        } else {
                            livery_root = String::new();
                            break;
                        }
                    }

                    return Some(livery_root);
                }
            }
        }
    } else {
        // Pattern is just a filename, possibly with glob
        // Extract the filename from the path
        let file_name = normalized_lower.rsplit('/').next()?;

        if matches_glob(&pattern_lower, file_name) {
            // Find the parent folder(s) based on parent_levels
            let parts: Vec<&str> = normalized.split('/').collect();
            if parts.len() > 1 {
                // parent_levels: 0 = file's parent folder, 1 = grandparent, etc.
                let end_idx = parts.len().saturating_sub(1 + rule.parent_levels);
                if end_idx > 0 {
                    return Some(parts[..end_idx].join("/"));
                } else if end_idx == 0 {
                    return Some(String::new());
                }
            }
        }
    }

    None
}

/// Simple glob matching supporting only '*' wildcard
fn matches_glob(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text;
    }

    let parts: Vec<&str> = pattern.split('*').collect();

    if parts.len() == 2 {
        // Single wildcard: prefix*suffix
        let prefix = parts[0];
        let suffix = parts[1];
        return text.starts_with(prefix)
            && text.ends_with(suffix)
            && text.len() >= prefix.len() + suffix.len();
    }

    // For more complex patterns, do a simple check
    // This handles patterns like "a319_icon*.png"
    let mut remaining = text;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // First part must be at the start
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else if i == parts.len() - 1 {
            // Last part must be at the end
            if !remaining.ends_with(part) {
                return false;
            }
        } else {
            // Middle parts must exist somewhere
            if let Some(pos) = remaining.find(part) {
                remaining = &remaining[pos + part.len()..];
            } else {
                return false;
            }
        }
    }

    true
}

/// Check if an ACF file name matches any known aircraft type
/// Returns the aircraft_type_id if matched
pub fn check_acf_identifier(acf_file_name: &str) -> Option<&'static str> {
    // Remove extension if present
    let name = Path::new(acf_file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(acf_file_name);

    for pattern in LIVERY_PATTERNS {
        for identifier in pattern.acf_identifiers {
            if name.eq_ignore_ascii_case(identifier) {
                return Some(pattern.aircraft_type_id);
            }
        }
    }

    None
}

/// Get the human-readable name for an aircraft type
pub fn get_aircraft_name(aircraft_type_id: &str) -> Option<&'static str> {
    LIVERY_PATTERNS
        .iter()
        .find(|p| p.aircraft_type_id == aircraft_type_id)
        .map(|p| p.aircraft_name)
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
        assert_eq!(aircraft_type, "FF777");
        assert_eq!(root, "MyLivery");

        // Test with backslashes
        let result = check_livery_pattern("MyLivery\\objects\\777\\texture.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "FF777");
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
    }

    #[test]
    fn test_check_acf_identifier() {
        // FF777
        assert_eq!(check_acf_identifier("777-200ER.acf"), Some("FF777"));
        assert_eq!(check_acf_identifier("777-200ER_xp12"), Some("FF777"));
        assert_eq!(check_acf_identifier("777-F_xp12_lo.acf"), Some("FF777"));
        // Toliss
        assert_eq!(check_acf_identifier("a319.acf"), Some("TOLISS_A319"));
        assert_eq!(check_acf_identifier("a320_StdDef.acf"), Some("TOLISS_A320"));
        assert_eq!(check_acf_identifier("a321_XP11.acf"), Some("TOLISS_A321"));
        assert_eq!(check_acf_identifier("A330-900.acf"), Some("TOLISS_A339"));
        // Unknown
        assert_eq!(check_acf_identifier("unknown.acf"), None);
    }
}

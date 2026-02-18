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
    /// Supports '*' (0+ chars) and '?' (0-1 chars) wildcards
    pub pattern: &'static str,
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
    /// ACF file name patterns that identify this aircraft (without extension)
    /// Supports '*' (0+ chars) and '?' (0-1 chars) wildcards, case-insensitive
    pub acf_identifiers: &'static [&'static str],
}

/// All registered livery patterns
pub static LIVERY_PATTERNS: &[LiveryPattern] = &[
    //FlightFactor
    LiveryPattern {
        aircraft_type_id: "FF_B777",
        aircraft_name: "FlightFactor 777v2",
        detection_rules: &[DetectionRule {
            pattern_type: "path",
            pattern: "objects/777",
        }],
        acf_identifiers: &[
            "777-200*",
            "777-300*",
            "777-F*",
        ],
    },
    //ToLiss
    LiveryPattern {
        aircraft_type_id: "TOLISS_A319",
        aircraft_name: "ToLiss A319",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "a319_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage319*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage319*.dds",
            },
        ],
        acf_identifiers: &["a319*"],
    },
    LiveryPattern {
        aircraft_type_id: "TOLISS_A320",
        aircraft_name: "ToLiss A320",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "a320_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage320*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage320*.dds",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/LEAP1A.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/LEAP1A.dds",
            },
        ],
        acf_identifiers: &["a320*"],
    },
    LiveryPattern {
        aircraft_type_id: "TOLISS_A321",
        aircraft_name: "ToLiss A321",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "a321_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage321*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage321*.dds",
            },
        ],
        acf_identifiers: &["a321*"],
    },
    LiveryPattern {
        aircraft_type_id: "TOLISS_A339",
        aircraft_name: "ToLiss A339",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "A330-900_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/A339_Engines.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/A339_Engines.dds",
            },
        ],
        acf_identifiers: &["A330-900*"],
    },
    LiveryPattern {
        aircraft_type_id: "TOLISS_A346",
        aircraft_name: "ToLiss A346",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "A340-600_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/EngineA346*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/EngineA346*.dds",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/FuselageA346*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/FuselageA346*.dds",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/StabilizersA346*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/StabilizersA346*.dds",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/WING?A346.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/WING?A346.dds",
            },
        ],
        acf_identifiers: &["A340-600*"],
    },
    //Free
    LiveryPattern {
        aircraft_type_id: "ZIBO_B738",
        aircraft_name: "Zibo B738",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "b738_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/738*.png",
            },
        ],
        acf_identifiers: &["b738???"],
    },
    LiveryPattern {
        aircraft_type_id: "LEVELUP_B737",
        aircraft_name: "LevelUp B737",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "737_??NG_icon11*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/737_??NG/737_*NG*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/737_??NG/737_*NG*.dds",
            },
        ],
        acf_identifiers: &["737_??NG"],
    },
    //X-Crafts
    LiveryPattern {
        aircraft_type_id: "X-CRAFTS_E170",
        aircraft_name: "X-Crafts E170",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "E170_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "path",
                pattern: "objects/E170",
            },
        ],
        acf_identifiers: &["E170*"],
    },
    LiveryPattern {
        aircraft_type_id: "X-CRAFTS_E175",
        aircraft_name: "X-Crafts E175",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "E175_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "path",
                pattern: "objects/E175",
            },
        ],
        acf_identifiers: &["E175*"],
    },
    LiveryPattern {
        aircraft_type_id: "X-CRAFTS_E190",
        aircraft_name: "X-Crafts E190",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "E190_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "path",
                pattern: "objects/E190",
            },
        ],
        acf_identifiers: &["E190*"],
    },
    LiveryPattern {
        aircraft_type_id: "X-CRAFTS_E195",
        aircraft_name: "X-Crafts E195",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "E195_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "path",
                pattern: "objects/E195",
            },
        ],
        acf_identifiers: &["E195*"],
    },
    LiveryPattern {
        aircraft_type_id: "X-CRAFTS_LINEAGE",
        aircraft_name: "X-Crafts Lineage",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "Lineage_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "path",
                pattern: "objects/LINEAGE",
            },
        ],
        acf_identifiers: &["Lineage*"],
    },
    //Felis
    LiveryPattern {
        aircraft_type_id: "FELIS_B742",
        aircraft_name: "Felis B742",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "B742_*Felis_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/engine_cowls.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/engine_cowls.dds",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage_main.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage_main.dds",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage_main2.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/fuselage_main2.dds",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/metal_parts.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/metal_parts.dds",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/mings_main.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/mings_main.dds",
            },
        ],
        acf_identifiers: &["B742_PW_Felis*"],
    },/*
    LiveryPattern {
        aircraft_type_id: "FELIS_B742F",
        aircraft_name: "Felis B742F",
        detection_rules: &[
            DetectionRule {
                pattern_type: "file",
                pattern: "B742_*Felis_*icon11*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/felis_b742_*.png",
            },
            DetectionRule {
                pattern_type: "file",
                pattern: "objects/felis_b742_*.dds",
            },
        ],
        acf_identifiers: &["B742_PW_Felis*"],
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

        if matches_glob(&pattern_lower, file_name) {
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
pub fn check_acf_identifier(acf_file_name: &str) -> Option<&'static str> {
    // Remove extension if present
    let name = Path::new(acf_file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(acf_file_name);

    let name_lower = name.to_lowercase();

    for pattern in LIVERY_PATTERNS {
        for identifier in pattern.acf_identifiers {
            if matches_glob(&identifier.to_lowercase(), &name_lower) {
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
        assert!(matches_glob("test?.png", "test.png"));   // 0 chars
        assert!(matches_glob("test?.png", "testA.png"));  // 1 char
        assert!(!matches_glob("test?.png", "testAB.png")); // 2 chars - no match
        assert!(matches_glob("a?b", "ab"));   // ? matches 0
        assert!(matches_glob("a?b", "axb"));  // ? matches 1
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
        assert_eq!(check_acf_identifier("777-200ER.acf"), Some("FF_B777"));
        assert_eq!(check_acf_identifier("777-200ER_xp12"), Some("FF_B777"));
        assert_eq!(check_acf_identifier("777-F_xp12_lo.acf"), Some("FF_B777"));
        // Toliss
        assert_eq!(check_acf_identifier("a319.acf"), Some("TOLISS_A319"));
        assert_eq!(check_acf_identifier("a320_StdDef.acf"), Some("TOLISS_A320"));
        assert_eq!(check_acf_identifier("a321_XP11.acf"), Some("TOLISS_A321"));
        assert_eq!(check_acf_identifier("A330-900.acf"), Some("TOLISS_A339"));
        // Unknown
        assert_eq!(check_acf_identifier("unknown.acf"), None);
    }
}

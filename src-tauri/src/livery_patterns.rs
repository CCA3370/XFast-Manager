//! Livery detection patterns for various aircraft types
//!
//! This module defines patterns to detect aircraft liveries and map them
//! to their corresponding aircraft types.

use std::path::Path;

/// Represents a livery pattern definition
#[derive(Debug, Clone)]
pub struct LiveryPattern {
    /// Unique identifier for the aircraft type (e.g., "FF777")
    pub aircraft_type_id: &'static str,
    /// Human-readable name for the aircraft
    pub aircraft_name: &'static str,
    /// Path pattern to detect livery (glob-like, e.g., "**/objects/777/**")
    /// The parent of the matched pattern component is the livery root
    pub detection_pattern: &'static str,
    /// ACF file names that identify this aircraft (without extension)
    pub acf_identifiers: &'static [&'static str],
}

/// All registered livery patterns
pub static LIVERY_PATTERNS: &[LiveryPattern] = &[
    LiveryPattern {
        aircraft_type_id: "FF777",
        aircraft_name: "FlightFactor 777v2",
        detection_pattern: "objects/777",
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
    // Add more patterns here as needed
];

/// Check if a path matches a livery pattern
/// Returns (aircraft_type_id, livery_root_path) if matched
pub fn check_livery_pattern(file_path: &str) -> Option<(&'static str, String)> {
    let normalized = file_path.replace('\\', "/").to_lowercase();

    for pattern in LIVERY_PATTERNS {
        let pattern_lower = pattern.detection_pattern.to_lowercase();

        // Check if the path contains the pattern
        if let Some(pos) = normalized.find(&pattern_lower) {
            // Find the livery root (parent of the pattern match)
            // For "objects/777", we want the folder containing "objects"
            let prefix = &file_path[..pos];
            let livery_root = if prefix.is_empty() {
                // Pattern is at root
                String::new()
            } else {
                // Remove trailing slash
                prefix.trim_end_matches('/').trim_end_matches('\\').to_string()
            };

            return Some((pattern.aircraft_type_id, livery_root));
        }
    }

    None
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
    fn test_check_livery_pattern() {
        // Test FF777 livery detection
        let result = check_livery_pattern("MyLivery/objects/777/texture.png");
        assert!(result.is_some());
        let (aircraft_type, root) = result.unwrap();
        assert_eq!(aircraft_type, "FF777");
        assert_eq!(root, "MyLivery");

        // Test with backslashes
        let result = check_livery_pattern("MyLivery\\objects\\777\\texture.png");
        assert!(result.is_some());

        // Test non-matching path
        let result = check_livery_pattern("SomeFolder/textures/image.png");
        assert!(result.is_none());
    }

    #[test]
    fn test_check_acf_identifier() {
        assert_eq!(check_acf_identifier("777-200ER.acf"), Some("FF777"));
        assert_eq!(check_acf_identifier("777-200ER_xp12"), Some("FF777"));
        assert_eq!(check_acf_identifier("777-F_xp12_lo.acf"), Some("FF777"));
        assert_eq!(check_acf_identifier("A320.acf"), None);
    }
}

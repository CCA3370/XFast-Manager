//! Shared scenery sorting profiles.

/// Return the numeric layer for a SimHeaven X-WORLD/X-* package folder.
///
/// Supported examples include current names such as
/// `simHeaven_X-World_Europe-6-scenery`, legacy names such as
/// `simHeaven_X-Europe-3-network`, and newer packages with more than nine
/// layers. The vegetation library intentionally has no layer number.
pub(crate) fn simheaven_layer_number(folder_name: &str) -> Option<u8> {
    const LEGACY_REGIONS: &[&str] = &[
        "africa",
        "america",
        "americas",
        "antarctica",
        "asia",
        "australia",
        "europe",
        "oceania",
    ];

    let normalized = folder_name.to_ascii_lowercase();
    let tokens: Vec<&str> = normalized
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();

    if tokens.first().copied() != Some("simheaven") {
        return None;
    }

    let is_x_world = tokens
        .windows(2)
        .any(|pair| pair[0] == "x" && pair[1] == "world");
    let is_legacy_x_region = tokens
        .windows(2)
        .any(|pair| pair[0] == "x" && LEGACY_REGIONS.iter().any(|region| pair[1] == *region));

    if !is_x_world && !is_legacy_x_region {
        return None;
    }

    tokens
        .iter()
        .rev()
        .find_map(|token| token.parse::<u8>().ok())
}

#[cfg(test)]
mod tests {
    use super::simheaven_layer_number;

    #[test]
    fn recognizes_current_legacy_and_extended_simheaven_layers() {
        assert_eq!(
            simheaven_layer_number("simHeaven_X-World_Europe-1-vfr"),
            Some(1)
        );
        assert_eq!(
            simheaven_layer_number("simHeaven_X-Europe-8-network"),
            Some(8)
        );
        assert_eq!(
            simheaven_layer_number("simHeaven_X-WORLD-Pro_TEST-13-network"),
            Some(13)
        );
        assert_eq!(
            simheaven_layer_number("simHeaven_X-World_Europe-v3-10-network"),
            Some(10)
        );
    }

    #[test]
    fn does_not_treat_libraries_or_unrelated_names_as_layers() {
        assert_eq!(
            simheaven_layer_number("simHeaven_X-World_Vegetation_Library"),
            None
        );
        assert_eq!(simheaven_layer_number("X-World_Europe-1-vfr"), None);
        assert_eq!(simheaven_layer_number("simHeaven_Library_2"), None);
    }
}

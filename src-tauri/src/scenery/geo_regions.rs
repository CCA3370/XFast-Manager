//! Geographic region lookup for scenery packages
//!
//! Uses Natural Earth Admin 0 Sovereignty data (1:10m scale) for continent
//! detection from integer lat/lon tile coordinates (as found in X-Plane DSF
//! filenames, e.g. `+30+135.dsf` → lat=30, lon=135).
//!
//! The sovereignty dataset includes maritime territories and remote islands,
//! providing better coverage than the standard countries dataset.
//!
//! **Why dilation?**
//! Even at 1:10m scale, some 1°×1° tiles whose centre point falls just
//! offshore or on small islands return "Unknown" even though the tile
//! is clearly part of a country (coastal airports, narrow peninsulas, islands).
//! A nearest-continent dilation pass — run once at startup — spreads each
//! continent label outward by up to `DILATE_PASSES` degrees, filling those gaps
//! without ever "jumping" a continent boundary.
//!
//! To regenerate `geo_continent_map.bin` after a Natural Earth update, run:
//! ```sh
//! node tools/generate_geo_data.cjs \
//!   path/to/ne_10m_admin_0_sovereignty_dir \
//!   src-tauri/src/scenery/geo_continent_map.bin
//! ```

use std::sync::OnceLock;

/// Raw continent map from Natural Earth (64,800 bytes).
///
/// Index: `(lat + 90) * 360 + (lon + 180)`, lat ∈ [-90, 90), lon ∈ [-180, 180).
/// Values: 0=Ocean, 1=Africa, 2=Antarctica, 3=Asia, 4=Europe,
///         5=North America, 6=Oceania, 7=South America.
static RAW_MAP: &[u8] = include_bytes!("geo_continent_map.bin");

/// Dilated continent map (computed once, same shape as RAW_MAP).
static DILATED_MAP: OnceLock<Vec<u8>> = OnceLock::new();

/// Number of dilation iterations.  Each pass extends every continent label
/// by one 1°-cell into adjacent ocean cells.  8 passes ≈ 888 km at the
/// equator, which covers remote islands and archipelagos.
const DILATE_PASSES: usize = 8;

static CONTINENT_NAMES: [&str; 8] = [
    "Unknown",       // 0: Ocean / Unknown
    "Africa",        // 1
    "Antarctica",    // 2
    "Asia",          // 3
    "Europe",        // 4
    "North America", // 5
    "Oceania",       // 6
    "South America", // 7
];

/// Dilate land labels into adjacent ocean cells.
///
/// For every unlabelled (ocean, value 0) cell, collect the continent IDs of
/// its 8 neighbours from the *previous* pass and assign the most frequent
/// non-zero ID.  Ties are broken by the lowest ID (stable ordering).
/// The antimeridian wraps correctly; the poles are clamped.
fn dilate(src: &[u8], passes: usize) -> Vec<u8> {
    let mut map = src.to_vec();

    for _ in 0..passes {
        let prev = map.clone();

        for li in 0i32..180 {
            for lo in 0i32..360 {
                let idx = (li * 360 + lo) as usize;
                if prev[idx] != 0 {
                    continue; // already a land cell — keep it
                }

                let mut freq = [0u32; 8]; // index = continent_id - 1
                for dlat in -1i32..=1 {
                    for dlon in -1i32..=1 {
                        if dlat == 0 && dlon == 0 {
                            continue;
                        }
                        let ni = li + dlat;
                        if !(0..180).contains(&ni) {
                            continue; // beyond the poles
                        }
                        let no = (lo + dlon + 360) % 360;
                        let v = prev[(ni * 360 + no) as usize] as usize;
                        if v > 0 && v <= 7 {
                            freq[v - 1] += 1;
                        }
                    }
                }

                // Pick the most-frequent neighbour continent (0 if all ocean)
                let best = freq
                    .iter()
                    .enumerate()
                    .filter(|(_, &c)| c > 0)
                    .max_by_key(|(_, &c)| c)
                    .map(|(i, _)| i as u8 + 1)
                    .unwrap_or(0);

                map[idx] = best;
            }
        }
    }

    map
}

/// Return the dilated continent map, computing it on first call.
fn continent_map() -> &'static [u8] {
    DILATED_MAP.get_or_init(|| dilate(RAW_MAP, DILATE_PASSES))
}

/// Look up the continent for a 1°×1° DSF tile coordinate.
///
/// `lat` and `lon` are the integer lower-left coordinates of the tile
/// (e.g. `+30+135` → `lat = 30`, `lon = 135`, tile spans 30–31°N 135–136°E).
///
/// Returns a continent name (`"Africa"`, `"Asia"`, …) or `"Unknown"` for
/// deep-ocean tiles and out-of-range inputs.
pub fn lookup_region(lat: i32, lon: i32) -> &'static str {
    if !(-90..90).contains(&lat) || !(-180..180).contains(&lon) {
        return "Unknown";
    }
    let idx = (lat + 90) as usize * 360 + (lon + 180) as usize;
    CONTINENT_NAMES[continent_map()[idx] as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singapore() {
        assert_eq!(lookup_region(1, 103), "Asia");
    }

    #[test]
    fn test_china() {
        assert_eq!(lookup_region(39, 116), "Asia");
    }

    #[test]
    fn test_usa_inland() {
        // Kansas — clearly continental US
        assert_eq!(lookup_region(38, -98), "North America");
    }

    #[test]
    fn test_usa_coastal() {
        // New York / New Jersey coast — needs dilation to resolve
        assert_eq!(lookup_region(40, -74), "North America");
    }

    #[test]
    fn test_uk() {
        assert_eq!(lookup_region(51, 0), "Europe");
    }

    #[test]
    fn test_australia() {
        assert_eq!(lookup_region(-33, 151), "Oceania");
    }

    #[test]
    fn test_hong_kong() {
        assert_eq!(lookup_region(22, 114), "Asia");
    }

    #[test]
    fn test_antarctica() {
        assert_eq!(lookup_region(-80, 0), "Antarctica");
    }

    #[test]
    fn test_brazil() {
        assert_eq!(lookup_region(-15, -47), "South America");
    }

    #[test]
    fn test_okinawa() {
        // Okinawa, Japan (ROAH Naha Airport)
        assert_eq!(lookup_region(26, 127), "Asia");
    }

    #[test]
    fn test_caribbean_islands() {
        // St. Maarten (TNCS Princess Juliana Airport)
        assert_eq!(lookup_region(18, -63), "North America");
    }

    #[test]
    fn test_hawaii() {
        // Hawaii (PHNL Honolulu) - classified as North America in Natural Earth data
        assert_eq!(lookup_region(21, -157), "North America");
    }

    #[test]
    fn test_ocean_not_empty() {
        // Mid-Pacific — should be non-empty string
        assert!(!lookup_region(0, -150).is_empty());
    }

    #[test]
    fn test_out_of_range() {
        assert_eq!(lookup_region(90, 0), "Unknown");
        assert_eq!(lookup_region(-91, 0), "Unknown");
        assert_eq!(lookup_region(0, 180), "Unknown");
        assert_eq!(lookup_region(0, -181), "Unknown");
    }
}

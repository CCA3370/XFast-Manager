//! Geographic region mapping for scenery packages
//!
//! This module provides country and continent lookup based on coordinates.
//! Countries are sorted by area (smallest first) to ensure small countries
//! like Singapore are matched before larger countries like China.

/// Geographic region with bounding box
pub struct GeoRegion {
    pub name: &'static str,
    pub continent: &'static str,
    pub min_lat: i32,
    pub max_lat: i32,
    pub min_lon: i32,
    pub max_lon: i32,
}

impl GeoRegion {
    const fn new(
        name: &'static str,
        continent: &'static str,
        min_lat: i32,
        max_lat: i32,
        min_lon: i32,
        max_lon: i32,
    ) -> Self {
        Self {
            name,
            continent,
            min_lat,
            max_lat,
            min_lon,
            max_lon,
        }
    }

    /// Check if a coordinate falls within this region
    pub fn contains(&self, lat: i32, lon: i32) -> bool {
        lat >= self.min_lat && lat <= self.max_lat && lon >= self.min_lon && lon <= self.max_lon
    }
}

/// Static list of geographic regions sorted by area (smallest first)
/// This ensures small countries are matched before larger ones
pub static GEO_REGIONS: &[GeoRegion] = &[
    // ========== Micro States & Small Countries ==========
    GeoRegion::new("Monaco", "Europe", 43, 44, 7, 8),
    GeoRegion::new("Vatican City", "Europe", 41, 42, 12, 13),
    GeoRegion::new("San Marino", "Europe", 43, 44, 12, 13),
    GeoRegion::new("Liechtenstein", "Europe", 47, 48, 9, 10),
    GeoRegion::new("Malta", "Europe", 35, 37, 14, 15),
    GeoRegion::new("Andorra", "Europe", 42, 43, 1, 2),
    GeoRegion::new("Singapore", "Asia", 1, 2, 103, 104),
    GeoRegion::new("Bahrain", "Asia", 25, 27, 50, 51),
    GeoRegion::new("Maldives", "Asia", -1, 8, 72, 74),
    GeoRegion::new("Luxembourg", "Europe", 49, 51, 5, 7),
    GeoRegion::new("Brunei", "Asia", 4, 6, 114, 116),
    GeoRegion::new("Hong Kong", "Asia", 22, 23, 113, 115),
    GeoRegion::new("Macau", "Asia", 22, 23, 113, 114),

    // ========== Caribbean & Central America (Small) ==========
    GeoRegion::new("Barbados", "North America", 13, 14, -60, -59),
    GeoRegion::new("Antigua and Barbuda", "North America", 16, 18, -62, -61),
    GeoRegion::new("Saint Lucia", "North America", 13, 15, -62, -60),
    GeoRegion::new("Grenada", "North America", 11, 13, -62, -61),
    GeoRegion::new("Saint Vincent", "North America", 12, 14, -62, -61),
    GeoRegion::new("Dominica", "North America", 15, 16, -62, -61),
    GeoRegion::new("Saint Kitts and Nevis", "North America", 17, 18, -63, -62),
    GeoRegion::new("Trinidad and Tobago", "North America", 10, 12, -62, -60),
    GeoRegion::new("Jamaica", "North America", 17, 19, -79, -76),
    GeoRegion::new("Puerto Rico", "North America", 17, 19, -68, -65),
    GeoRegion::new("Bahamas", "North America", 20, 28, -80, -72),
    GeoRegion::new("Cayman Islands", "North America", 19, 20, -82, -79),
    GeoRegion::new("Bermuda", "North America", 32, 33, -65, -64),
    GeoRegion::new("Aruba", "North America", 12, 13, -70, -69),
    GeoRegion::new("Curacao", "North America", 12, 13, -69, -68),
    GeoRegion::new("US Virgin Islands", "North America", 17, 19, -65, -64),
    GeoRegion::new("British Virgin Islands", "North America", 18, 19, -65, -64),
    GeoRegion::new("Turks and Caicos", "North America", 21, 22, -73, -71),

    // ========== Pacific Islands ==========
    GeoRegion::new("Guam", "Oceania", 13, 14, 144, 145),
    GeoRegion::new("Palau", "Oceania", 2, 9, 131, 135),
    GeoRegion::new("Micronesia", "Oceania", 1, 10, 137, 163),
    GeoRegion::new("Marshall Islands", "Oceania", 4, 15, 160, 173),
    GeoRegion::new("Nauru", "Oceania", -1, 0, 166, 167),
    GeoRegion::new("Tuvalu", "Oceania", -10, -5, 176, 180),
    GeoRegion::new("Kiribati", "Oceania", -12, 5, -180, 180),
    GeoRegion::new("Samoa", "Oceania", -15, -13, -173, -171),
    GeoRegion::new("Tonga", "Oceania", -23, -15, -177, -173),
    GeoRegion::new("Vanuatu", "Oceania", -21, -13, 166, 171),
    GeoRegion::new("Fiji", "Oceania", -21, -12, 177, -179),
    GeoRegion::new("Solomon Islands", "Oceania", -13, -5, 155, 170),
    GeoRegion::new("New Caledonia", "Oceania", -23, -19, 163, 169),
    GeoRegion::new("French Polynesia", "Oceania", -28, -7, -155, -134),
    GeoRegion::new("Cook Islands", "Oceania", -22, -8, -166, -157),
    GeoRegion::new("Northern Mariana Islands", "Oceania", 14, 21, 144, 146),
    GeoRegion::new("American Samoa", "Oceania", -15, -14, -171, -169),

    // ========== Small European Countries ==========
    GeoRegion::new("Cyprus", "Europe", 34, 36, 32, 35),
    GeoRegion::new("Slovenia", "Europe", 45, 47, 13, 17),
    GeoRegion::new("North Macedonia", "Europe", 40, 43, 20, 23),
    GeoRegion::new("Albania", "Europe", 39, 43, 19, 21),
    GeoRegion::new("Kosovo", "Europe", 42, 43, 20, 22),
    GeoRegion::new("Montenegro", "Europe", 41, 44, 18, 21),
    GeoRegion::new("Bosnia and Herzegovina", "Europe", 42, 46, 15, 20),
    GeoRegion::new("Croatia", "Europe", 42, 47, 13, 20),
    GeoRegion::new("Estonia", "Europe", 57, 60, 21, 29),
    GeoRegion::new("Latvia", "Europe", 55, 58, 20, 29),
    GeoRegion::new("Lithuania", "Europe", 53, 57, 20, 27),
    GeoRegion::new("Moldova", "Europe", 45, 49, 26, 31),
    GeoRegion::new("Slovakia", "Europe", 47, 50, 16, 23),
    GeoRegion::new("Czech Republic", "Europe", 48, 52, 12, 19),
    GeoRegion::new("Hungary", "Europe", 45, 49, 16, 23),
    GeoRegion::new("Austria", "Europe", 46, 49, 9, 18),
    GeoRegion::new("Switzerland", "Europe", 45, 48, 5, 11),
    GeoRegion::new("Belgium", "Europe", 49, 52, 2, 7),
    GeoRegion::new("Netherlands", "Europe", 50, 54, 3, 8),
    GeoRegion::new("Denmark", "Europe", 54, 58, 8, 16),
    GeoRegion::new("Ireland", "Europe", 51, 56, -11, -5),
    GeoRegion::new("Portugal", "Europe", 36, 43, -10, -6),
    GeoRegion::new("Serbia", "Europe", 42, 47, 18, 23),
    GeoRegion::new("Bulgaria", "Europe", 41, 45, 22, 29),
    GeoRegion::new("Greece", "Europe", 34, 42, 19, 30),

    // ========== Middle East (Small to Medium) ==========
    GeoRegion::new("Qatar", "Asia", 24, 27, 50, 52),
    GeoRegion::new("Kuwait", "Asia", 28, 31, 46, 49),
    GeoRegion::new("United Arab Emirates", "Asia", 22, 27, 51, 57),
    GeoRegion::new("Israel", "Asia", 29, 34, 34, 36),
    GeoRegion::new("Palestine", "Asia", 31, 33, 34, 36),
    GeoRegion::new("Lebanon", "Asia", 33, 35, 35, 37),
    GeoRegion::new("Jordan", "Asia", 29, 34, 34, 40),
    GeoRegion::new("Oman", "Asia", 16, 27, 51, 60),
    GeoRegion::new("Yemen", "Asia", 12, 19, 42, 55),
    GeoRegion::new("Syria", "Asia", 32, 38, 35, 43),
    GeoRegion::new("Iraq", "Asia", 29, 38, 38, 49),

    // ========== Small African Countries ==========
    GeoRegion::new("Gambia", "Africa", 13, 14, -17, -13),
    GeoRegion::new("Cabo Verde", "Africa", 14, 18, -26, -22),
    GeoRegion::new("Sao Tome and Principe", "Africa", 0, 2, 6, 8),
    GeoRegion::new("Comoros", "Africa", -13, -11, 43, 45),
    GeoRegion::new("Mauritius", "Africa", -21, -19, 57, 64),
    GeoRegion::new("Seychelles", "Africa", -10, -4, 46, 56),
    GeoRegion::new("Reunion", "Africa", -22, -20, 55, 56),
    GeoRegion::new("Djibouti", "Africa", 10, 13, 41, 44),
    GeoRegion::new("Equatorial Guinea", "Africa", 0, 4, 5, 12),
    GeoRegion::new("Eswatini", "Africa", -28, -25, 30, 33),
    GeoRegion::new("Lesotho", "Africa", -31, -28, 27, 30),
    GeoRegion::new("Rwanda", "Africa", -3, -1, 28, 31),
    GeoRegion::new("Burundi", "Africa", -5, -2, 28, 31),
    GeoRegion::new("Togo", "Africa", 6, 12, -1, 2),
    GeoRegion::new("Benin", "Africa", 6, 13, 0, 4),
    GeoRegion::new("Sierra Leone", "Africa", 6, 10, -14, -10),
    GeoRegion::new("Liberia", "Africa", 4, 9, -12, -7),
    GeoRegion::new("Guinea-Bissau", "Africa", 10, 13, -17, -13),
    GeoRegion::new("Eritrea", "Africa", 12, 18, 36, 44),
    GeoRegion::new("Malawi", "Africa", -17, -9, 32, 36),
    GeoRegion::new("Tunisia", "Africa", 30, 38, 7, 12),
    GeoRegion::new("Senegal", "Africa", 12, 17, -18, -11),
    GeoRegion::new("Guinea", "Africa", 7, 13, -15, -7),
    GeoRegion::new("Ghana", "Africa", 4, 12, -4, 2),
    GeoRegion::new("Ivory Coast", "Africa", 4, 11, -9, -2),
    GeoRegion::new("Burkina Faso", "Africa", 9, 16, -6, 3),
    GeoRegion::new("Uganda", "Africa", -2, 5, 29, 35),
    GeoRegion::new("Gabon", "Africa", -4, 3, 8, 15),
    GeoRegion::new("Republic of the Congo", "Africa", -6, 4, 11, 19),
    GeoRegion::new("Cameroon", "Africa", 1, 14, 8, 17),
    GeoRegion::new("Zimbabwe", "Africa", -23, -15, 25, 34),
    GeoRegion::new("Zambia", "Africa", -18, -8, 21, 34),
    GeoRegion::new("Botswana", "Africa", -27, -17, 19, 30),
    GeoRegion::new("Namibia", "Africa", -29, -17, 11, 26),
    GeoRegion::new("Mozambique", "Africa", -27, -10, 30, 41),
    GeoRegion::new("Madagascar", "Africa", -26, -11, 43, 51),
    GeoRegion::new("Kenya", "Africa", -5, 5, 33, 42),
    GeoRegion::new("Tanzania", "Africa", -12, -1, 29, 41),
    GeoRegion::new("Somalia", "Africa", -2, 12, 40, 52),
    GeoRegion::new("Ethiopia", "Africa", 3, 15, 32, 48),
    GeoRegion::new("Central African Republic", "Africa", 2, 12, 14, 28),
    GeoRegion::new("South Sudan", "Africa", 3, 13, 23, 36),
    GeoRegion::new("Nigeria", "Africa", 4, 14, 2, 15),
    GeoRegion::new("Morocco", "Africa", 27, 36, -14, -1),
    GeoRegion::new("Angola", "Africa", -18, -4, 11, 25),
    GeoRegion::new("South Africa", "Africa", -35, -22, 16, 33),
    GeoRegion::new("Egypt", "Africa", 22, 32, 24, 37),
    GeoRegion::new("Libya", "Africa", 19, 34, 9, 26),
    GeoRegion::new("Sudan", "Africa", 8, 23, 21, 39),
    GeoRegion::new("Chad", "Africa", 7, 24, 13, 24),
    GeoRegion::new("Niger", "Africa", 11, 24, 0, 16),
    GeoRegion::new("Mali", "Africa", 10, 25, -13, 5),
    GeoRegion::new("Mauritania", "Africa", 14, 28, -18, -4),
    GeoRegion::new("Democratic Republic of the Congo", "Africa", -14, 6, 12, 32),
    GeoRegion::new("Algeria", "Africa", 18, 38, -9, 12),

    // ========== Small Asian Countries ==========
    GeoRegion::new("Bhutan", "Asia", 26, 29, 88, 93),
    GeoRegion::new("Nepal", "Asia", 26, 31, 80, 89),
    GeoRegion::new("Bangladesh", "Asia", 20, 27, 88, 93),
    GeoRegion::new("Sri Lanka", "Asia", 5, 10, 79, 82),
    GeoRegion::new("Taiwan", "Asia", 21, 26, 119, 123),
    GeoRegion::new("South Korea", "Asia", 33, 39, 124, 132),
    GeoRegion::new("North Korea", "Asia", 37, 43, 124, 131),
    GeoRegion::new("Cambodia", "Asia", 9, 15, 102, 108),
    GeoRegion::new("Laos", "Asia", 13, 23, 100, 108),
    GeoRegion::new("Vietnam", "Asia", 8, 24, 102, 110),
    GeoRegion::new("Malaysia", "Asia", 0, 8, 99, 120),
    GeoRegion::new("Philippines", "Asia", 4, 22, 116, 127),
    GeoRegion::new("Thailand", "Asia", 5, 21, 97, 106),
    GeoRegion::new("Myanmar", "Asia", 9, 29, 92, 102),
    GeoRegion::new("Japan", "Asia", 24, 46, 122, 154),
    GeoRegion::new("Pakistan", "Asia", 23, 38, 60, 78),
    GeoRegion::new("Afghanistan", "Asia", 29, 39, 60, 75),
    GeoRegion::new("Uzbekistan", "Asia", 37, 46, 55, 74),
    GeoRegion::new("Turkmenistan", "Asia", 35, 43, 52, 67),
    GeoRegion::new("Tajikistan", "Asia", 36, 42, 67, 76),
    GeoRegion::new("Kyrgyzstan", "Asia", 39, 44, 69, 81),
    GeoRegion::new("Georgia", "Asia", 41, 44, 40, 47),
    GeoRegion::new("Armenia", "Asia", 38, 42, 43, 47),
    GeoRegion::new("Azerbaijan", "Asia", 38, 42, 44, 51),
    GeoRegion::new("Iran", "Asia", 25, 40, 44, 64),
    GeoRegion::new("Saudi Arabia", "Asia", 16, 33, 34, 56),
    GeoRegion::new("Turkey", "Asia", 35, 43, 25, 45),
    GeoRegion::new("Indonesia", "Asia", -11, 6, 95, 141),
    GeoRegion::new("Mongolia", "Asia", 41, 53, 87, 120),
    GeoRegion::new("Kazakhstan", "Asia", 40, 56, 46, 88),
    GeoRegion::new("India", "Asia", 6, 36, 68, 98),

    // ========== Medium European Countries ==========
    GeoRegion::new("Romania", "Europe", 43, 49, 20, 30),
    GeoRegion::new("Poland", "Europe", 49, 55, 14, 25),
    GeoRegion::new("Italy", "Europe", 35, 48, 6, 19),
    GeoRegion::new("United Kingdom", "Europe", 49, 61, -9, 2),
    GeoRegion::new("Germany", "Europe", 47, 56, 5, 16),
    GeoRegion::new("Finland", "Europe", 59, 70, 20, 32),
    GeoRegion::new("Norway", "Europe", 57, 72, 4, 32),
    GeoRegion::new("Sweden", "Europe", 55, 70, 10, 25),
    GeoRegion::new("Spain", "Europe", 35, 44, -10, 5),
    GeoRegion::new("France", "Europe", 41, 52, -5, 10),
    GeoRegion::new("Ukraine", "Europe", 44, 53, 22, 41),
    GeoRegion::new("Belarus", "Europe", 51, 57, 23, 33),
    GeoRegion::new("Iceland", "Europe", 63, 67, -25, -13),

    // ========== North America ==========
    GeoRegion::new("Belize", "North America", 15, 19, -90, -87),
    GeoRegion::new("El Salvador", "North America", 13, 15, -91, -87),
    GeoRegion::new("Costa Rica", "North America", 8, 12, -86, -82),
    GeoRegion::new("Panama", "North America", 7, 10, -83, -77),
    GeoRegion::new("Honduras", "North America", 12, 17, -90, -83),
    GeoRegion::new("Nicaragua", "North America", 10, 15, -88, -82),
    GeoRegion::new("Guatemala", "North America", 13, 18, -93, -88),
    GeoRegion::new("Cuba", "North America", 19, 24, -85, -74),
    GeoRegion::new("Haiti", "North America", 18, 20, -75, -71),
    GeoRegion::new("Dominican Republic", "North America", 17, 20, -72, -68),
    GeoRegion::new("Mexico", "North America", 14, 33, -118, -86),
    GeoRegion::new("Greenland", "North America", 59, 84, -74, -11),
    GeoRegion::new("Canada", "North America", 41, 84, -141, -52),
    GeoRegion::new("United States", "North America", 24, 50, -125, -66),
    GeoRegion::new("Alaska", "North America", 51, 72, -180, -130),
    GeoRegion::new("Hawaii", "North America", 18, 23, -161, -154),

    // ========== South America ==========
    GeoRegion::new("Suriname", "South America", 1, 6, -59, -53),
    GeoRegion::new("Guyana", "South America", 1, 9, -62, -56),
    GeoRegion::new("French Guiana", "South America", 2, 6, -55, -51),
    GeoRegion::new("Uruguay", "South America", -35, -30, -59, -53),
    GeoRegion::new("Paraguay", "South America", -28, -19, -63, -54),
    GeoRegion::new("Ecuador", "South America", -5, 2, -82, -75),
    GeoRegion::new("Chile", "South America", -56, -17, -76, -66),
    GeoRegion::new("Venezuela", "South America", 0, 13, -74, -59),
    GeoRegion::new("Colombia", "South America", -5, 14, -80, -66),
    GeoRegion::new("Bolivia", "South America", -23, -9, -70, -57),
    GeoRegion::new("Peru", "South America", -19, 0, -82, -68),
    GeoRegion::new("Argentina", "South America", -56, -21, -74, -53),
    GeoRegion::new("Brazil", "South America", -34, 6, -74, -34),

    // ========== Oceania (Large) ==========
    GeoRegion::new("Papua New Guinea", "Oceania", -12, 0, 140, 160),
    GeoRegion::new("New Zealand", "Oceania", -48, -34, 166, 179),
    GeoRegion::new("Australia", "Oceania", -45, -10, 112, 154),

    // ========== Large Countries (Last) ==========
    GeoRegion::new("China", "Asia", 18, 54, 73, 135),
    GeoRegion::new("Russia", "Europe", 41, 82, 19, 180),

    // ========== Antarctica ==========
    GeoRegion::new("Antarctica", "Antarctica", -90, -60, -180, 180),
];

/// Continent fallback regions for ocean areas
static CONTINENT_REGIONS: &[GeoRegion] = &[
    GeoRegion::new("Asia", "Asia", -15, 82, 25, 180),
    GeoRegion::new("Europe", "Europe", 34, 82, -25, 65),
    GeoRegion::new("Africa", "Africa", -35, 38, -20, 55),
    GeoRegion::new("North America", "North America", 5, 84, -180, -30),
    GeoRegion::new("South America", "South America", -60, 15, -85, -30),
    GeoRegion::new("Oceania", "Oceania", -50, 0, 100, 180),
    GeoRegion::new("Antarctica", "Antarctica", -90, -60, -180, 180),
];

/// Look up the country and continent for a given coordinate
/// Returns (country_name, continent_name)
/// If no country matches, returns (None, continent_name)
pub fn lookup_region(lat: i32, lon: i32) -> (Option<&'static str>, &'static str) {
    // First try to find a matching country
    for region in GEO_REGIONS {
        if region.contains(lat, lon) {
            return (Some(region.name), region.continent);
        }
    }

    // No country found, try to determine continent
    for region in CONTINENT_REGIONS {
        if region.contains(lat, lon) {
            return (None, region.continent);
        }
    }

    // Default to unknown
    (None, "Unknown")
}

/// Look up region for multiple coordinates and return the most common result
/// This is useful for scenery packages that span multiple tiles
pub fn lookup_region_for_coordinates(coords: &[(i32, i32)]) -> (Option<&'static str>, &'static str) {
    if coords.is_empty() {
        return (None, "Unknown");
    }

    if coords.len() == 1 {
        return lookup_region(coords[0].0, coords[0].1);
    }

    // Calculate the geographic center
    let (center_lat, center_lon) = calculate_center(coords);
    lookup_region(center_lat, center_lon)
}

/// Calculate the geographic center of a set of coordinates
pub fn calculate_center(coords: &[(i32, i32)]) -> (i32, i32) {
    if coords.is_empty() {
        return (0, 0);
    }

    let sum_lat: i64 = coords.iter().map(|(lat, _)| *lat as i64).sum();
    let sum_lon: i64 = coords.iter().map(|(_, lon)| *lon as i64).sum();
    let count = coords.len() as i64;

    ((sum_lat / count) as i32, (sum_lon / count) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singapore() {
        let (country, continent) = lookup_region(1, 103);
        assert_eq!(country, Some("Singapore"));
        assert_eq!(continent, "Asia");
    }

    #[test]
    fn test_china() {
        let (country, continent) = lookup_region(39, 116);
        assert_eq!(country, Some("China"));
        assert_eq!(continent, "Asia");
    }

    #[test]
    fn test_usa() {
        let (country, continent) = lookup_region(40, -74);
        assert_eq!(country, Some("United States"));
        assert_eq!(continent, "North America");
    }

    #[test]
    fn test_uk() {
        let (country, continent) = lookup_region(51, 0);
        assert_eq!(country, Some("United Kingdom"));
        assert_eq!(continent, "Europe");
    }

    #[test]
    fn test_australia() {
        let (country, continent) = lookup_region(-33, 151);
        assert_eq!(country, Some("Australia"));
        assert_eq!(continent, "Oceania");
    }

    #[test]
    fn test_ocean_fallback() {
        // Middle of Pacific Ocean
        let (country, continent) = lookup_region(0, -150);
        assert!(country.is_none());
        // Should fall back to a continent or Unknown
    }

    #[test]
    fn test_calculate_center() {
        let coords = vec![(30, 110), (31, 111), (32, 112)];
        let (lat, lon) = calculate_center(&coords);
        assert_eq!(lat, 31);
        assert_eq!(lon, 111);
    }

    #[test]
    fn test_small_country_priority() {
        // Singapore should be matched before China
        let (country, _) = lookup_region(1, 103);
        assert_eq!(country, Some("Singapore"));
    }

    #[test]
    fn test_hong_kong() {
        let (country, continent) = lookup_region(22, 114);
        assert_eq!(country, Some("Hong Kong"));
        assert_eq!(continent, "Asia");
    }
}

//! Scenery sort strategy refinements
//!
//! Adds two refinements on top of the base category-priority sort performed by
//! `scenery_index::sort_packages_with_special_rules`:
//!
//! 1. **Airport sub-priority** — user-custom airports rank above Aerosoft
//!    airports within the `Airport` category. Implemented by assigning
//!    `sub_priority = 1` to Aerosoft entries; the base comparator already
//!    sorts on `(category.priority(), sub_priority)`, so this is enough to
//!    flip the order.
//!
//! 2. **Ortho re-sequencing** — orthophoto packages (classified as `Mesh` by
//!    the base classifier) are lifted above the `Overlay` block in the final
//!    sorted vec.
//!
//! Both refinements are pure mutations on the package vec, so no internal
//! helpers of `scenery_index` need to be exposed.

use crate::models::{SceneryCategory, SceneryPackageInfo};

/// Pre-sort hook: assign `sub_priority` based on folder-name heuristics for
/// categories whose intra-group ordering is not file-content-driven.
///
/// Currently only the `Airport` category is affected:
/// Aerosoft → `1`, everything else stays at the classifier-assigned default.
///
/// Sub-priorities set by the classifier for other categories (notably XPME
/// inside `Mesh`, which uses `2`) are preserved.
pub(crate) fn assign_sub_priorities(packages: &mut [SceneryPackageInfo]) {
    for pkg in packages.iter_mut() {
        if pkg.category == SceneryCategory::Airport && is_aerosoft_folder(&pkg.folder_name) {
            pkg.sub_priority = 1;
        }
    }
}

/// Post-sort hook: move ortho packages so they sit immediately before the
/// first `Overlay` entry in the already-sorted vec.
///
/// Orthos are detected by folder-name heuristic among `Mesh` packages. If
/// there are no `Overlay` entries, the orthos stay in place. Relative order
/// among orthos and among non-ortho entries is preserved.
pub(crate) fn resequence_orthos_above_overlays(packages: &mut Vec<SceneryPackageInfo>) {
    let (orthos, mut rest): (Vec<SceneryPackageInfo>, Vec<SceneryPackageInfo>) =
        std::mem::take(packages)
            .into_iter()
            .partition(|p| is_ortho_folder(p));

    if orthos.is_empty() {
        *packages = rest;
        return;
    }

    let insert_at = rest
        .iter()
        .position(|p| p.category == SceneryCategory::Overlay)
        .unwrap_or(rest.len());

    let tail = rest.split_off(insert_at);
    rest.extend(orthos);
    rest.extend(tail);
    *packages = rest;
}

fn is_aerosoft_folder(folder_name: &str) -> bool {
    folder_name.to_lowercase().starts_with("aerosoft")
}

fn is_ortho_folder(info: &SceneryPackageInfo) -> bool {
    // Only consider Mesh entries to avoid relocating user-renamed libraries or
    // airports that happen to contain "ortho".
    info.category == SceneryCategory::Mesh
        && crate::scenery_classifier::is_ortho_folder_name(&info.folder_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn make_package(folder_name: &str, category: SceneryCategory) -> SceneryPackageInfo {
        SceneryPackageInfo {
            folder_name: folder_name.to_string(),
            category: category.clone(),
            sub_priority: 0,
            last_modified: SystemTime::UNIX_EPOCH,
            has_apt_dat: false,
            airport_id: None,
            has_dsf: false,
            has_library_txt: false,
            has_textures: false,
            has_objects: false,
            texture_count: 0,
            earth_nav_tile_count: 1,
            indexed_at: SystemTime::UNIX_EPOCH,
            required_libraries: Vec::new(),
            missing_libraries: Vec::new(),
            exported_library_names: Vec::new(),
            enabled: true,
            sort_order: 0,
            actual_path: None,
            continent: None,
            original_category: Some(category),
        }
    }

    /// Mirror the production comparator's primary key
    /// (`compare_packages_for_sorting`): category priority, sub-priority, then
    /// case-insensitive folder name.
    fn baseline_sort(packages: &mut [SceneryPackageInfo]) {
        packages.sort_by(|a, b| {
            (
                a.category.priority(),
                a.sub_priority,
                a.folder_name.to_lowercase(),
            )
                .cmp(&(
                    b.category.priority(),
                    b.sub_priority,
                    b.folder_name.to_lowercase(),
                ))
        });
    }

    #[test]
    fn custom_airport_ranks_above_aerosoft_airport() {
        let mut packages = vec![
            make_package("Aerosoft - LFPG Paris", SceneryCategory::Airport),
            make_package("My Custom KSEA", SceneryCategory::Airport),
        ];

        assign_sub_priorities(&mut packages);
        baseline_sort(&mut packages);

        let names: Vec<&str> = packages.iter().map(|p| p.folder_name.as_str()).collect();
        assert_eq!(names, vec!["My Custom KSEA", "Aerosoft - LFPG Paris"]);
        assert_eq!(packages[0].sub_priority, 0);
        assert_eq!(packages[1].sub_priority, 1);
    }

    #[test]
    fn aerosoft_airport_ranks_above_global_airports() {
        let mut packages = vec![
            make_package("Global Airports", SceneryCategory::DefaultAirport),
            make_package("Aerosoft - LFPG Paris", SceneryCategory::Airport),
        ];

        assign_sub_priorities(&mut packages);
        baseline_sort(&mut packages);

        let names: Vec<&str> = packages.iter().map(|p| p.folder_name.as_str()).collect();
        assert_eq!(names, vec!["Aerosoft - LFPG Paris", "Global Airports"]);
    }

    #[test]
    fn library_with_library_txt_ranks_below_airports_and_above_overlays() {
        let mut lib = make_package("MisterX Library", SceneryCategory::Library);
        lib.has_library_txt = true;

        let mut packages = vec![
            make_package("MyOverlay", SceneryCategory::Overlay),
            lib,
            make_package("My Custom KSEA", SceneryCategory::Airport),
        ];

        assign_sub_priorities(&mut packages);
        baseline_sort(&mut packages);
        resequence_orthos_above_overlays(&mut packages);

        let names: Vec<&str> = packages.iter().map(|p| p.folder_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["My Custom KSEA", "MisterX Library", "MyOverlay"]
        );
        assert_eq!(packages[1].category, SceneryCategory::Library);
        assert!(packages[1].has_library_txt);
    }

    #[test]
    fn ortho_ranks_above_overlay_after_resequence() {
        let mut packages = vec![
            make_package("MyOverlay", SceneryCategory::Overlay),
            make_package("zOrtho4XP_+50+005", SceneryCategory::Mesh),
        ];

        assign_sub_priorities(&mut packages);
        baseline_sort(&mut packages);
        // Before re-sequence: Overlay (priority 5) sits above Mesh (priority 7).
        assert_eq!(packages[0].folder_name, "MyOverlay");

        resequence_orthos_above_overlays(&mut packages);

        let names: Vec<&str> = packages.iter().map(|p| p.folder_name.as_str()).collect();
        assert_eq!(names, vec!["zOrtho4XP_+50+005", "MyOverlay"]);
    }

    #[test]
    fn overlay_ranks_above_regular_mesh() {
        let mut packages = vec![
            make_package("UHD Mesh v4 +50-005", SceneryCategory::Mesh),
            make_package("MyOverlay", SceneryCategory::Overlay),
        ];

        assign_sub_priorities(&mut packages);
        baseline_sort(&mut packages);
        resequence_orthos_above_overlays(&mut packages);

        let names: Vec<&str> = packages.iter().map(|p| p.folder_name.as_str()).collect();
        assert_eq!(names, vec!["MyOverlay", "UHD Mesh v4 +50-005"]);
    }

    #[test]
    fn unrecognized_folders_stay_at_bottom() {
        let mut packages = vec![
            make_package("WeirdMystery", SceneryCategory::Unrecognized),
            make_package("UHD Mesh v4 +50-005", SceneryCategory::Mesh),
            make_package("My Custom KSEA", SceneryCategory::Airport),
            make_package("MyOverlay", SceneryCategory::Overlay),
            make_package("zOrtho4XP_+50+005", SceneryCategory::Mesh),
        ];

        assign_sub_priorities(&mut packages);
        baseline_sort(&mut packages);
        resequence_orthos_above_overlays(&mut packages);

        let last = packages.last().unwrap();
        assert_eq!(last.folder_name, "WeirdMystery");
        assert_eq!(last.category, SceneryCategory::Unrecognized);
    }
}

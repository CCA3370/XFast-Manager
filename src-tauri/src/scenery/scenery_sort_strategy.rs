//! Airport priority refinements within the existing scenery category order.
//!
//! Custom airports override bundled Aerosoft airports. Both stay above Global
//! Airports, while overlays remain above orthophoto/base meshes.

use crate::models::{SceneryCategory, SceneryPackageInfo};

/// Apply the Aerosoft airport sub-priority without changing other categories.
/// Return whether metadata changed so even an already-sorted index is saved.
pub(crate) fn assign_sub_priorities(packages: &mut [SceneryPackageInfo]) -> bool {
    let mut changed = false;
    for pkg in packages {
        if pkg.category == SceneryCategory::Airport
            && is_aerosoft_folder(&pkg.folder_name)
            && pkg.sub_priority != 1
        {
            pkg.sub_priority = 1;
            changed = true;
        }
    }
    changed
}

fn is_aerosoft_folder(folder_name: &str) -> bool {
    let name = folder_name.trim_start().to_lowercase();
    let Some(suffix) = name.strip_prefix("aerosoft") else {
        return false;
    };
    // Match the brand at the start, with a token boundary; community names such
    // as MyAerosoftReplacement or AerosoftReplacement are not bundled airports.
    suffix.is_empty() || suffix.starts_with(|c: char| !c.is_alphanumeric())
}

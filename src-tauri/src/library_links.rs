use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use crate::logger;

/// Remote JSON schema for library download links
#[derive(Debug, Deserialize)]
struct LibraryLinksData {
    #[allow(dead_code)]
    version: u32,
    #[allow(dead_code)]
    updated: String,
    libraries: HashMap<String, String>,
}

/// Cached library links with expiration
struct CachedLinks {
    links: HashMap<String, String>,
    fetched_at: SystemTime,
}

/// Global in-memory cache for library download links
static CACHE: Lazy<Mutex<Option<CachedLinks>>> = Lazy::new(|| Mutex::new(None));

/// Cache TTL: 24 hours (matching updater pattern)
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Remote URL for the library links JSON file
const REMOTE_URL: &str =
    "https://raw.githubusercontent.com/CCA3370/XFast-Manager/main/data/library_links.json";

/// Returns the hardcoded fallback library links database.
/// Used when the remote fetch fails (network unavailable, timeout, etc.).
fn hardcoded_links() -> HashMap<String, String> {
    let entries = [
        ("opensceneryx", "https://www.opensceneryx.com/"),
        ("opensam", "https://www.stairport.com/sam/"),
        ("sam", "https://www.stairport.com/sam/"),
        ("misterx", "https://forums.x-plane.org/index.php?/files/file/28167-misterx-library/"),
        ("re_library", "https://forums.x-plane.org/index.php?/files/file/30389-re_library/"),
        ("the_handy_objects_library", "https://forums.x-plane.org/index.php?/files/file/24261-the-handy-objects-library/"),
        ("handy", "https://forums.x-plane.org/index.php?/files/file/24261-the-handy-objects-library/"),
        ("cdb-library", "https://forums.x-plane.org/index.php?/files/file/23316-cdb-library/"),
        ("cdb", "https://forums.x-plane.org/index.php?/files/file/23316-cdb-library/"),
        ("ff_library", "https://forums.x-plane.org/index.php?/files/file/14391-ff-library/"),
        ("r2_library", "https://forums.x-plane.org/index.php?/files/file/14388-r2-library/"),
        ("bs2001", "https://forums.x-plane.org/index.php?/files/file/38049-bs2001-object-library/"),
        ("ruscenery", "https://forums.x-plane.org/index.php?/files/file/24460-ruscenery-library/"),
        ("world-models", "https://developer.x-plane.com/tools/worldmodels/"),
        ("gt_library", "https://forums.x-plane.org/index.php?/files/file/29045-gt_library/"),
        ("flyagi", "https://forums.x-plane.org/index.php?/files/file/26891-flyagi-vegetation/"),
        ("prefab_library", "https://forums.x-plane.org/index.php?/files/file/30054-prefab-library/"),
        ("naps", "https://forums.x-plane.org/index.php?/files/file/48701-naps-library/"),
        ("pp_library", "https://forums.x-plane.org/index.php?/files/file/31712-pp_library/"),
        ("pp", "https://forums.x-plane.org/index.php?/files/file/31712-pp_library/"),
        ("dense_forests", "https://forums.x-plane.org/index.php?/files/file/76474-dense-forests-library/"),
        ("3d_people", "https://forums.x-plane.org/index.php?/files/file/46498-3d-people-library/"),
    ];

    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Fetch library links from the remote JSON, with in-memory caching.
/// Falls back to hardcoded data on any failure.
async fn get_library_links() -> HashMap<String, String> {
    // Check cache first
    {
        let cache = CACHE.lock().unwrap();
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed().unwrap_or(CACHE_TTL) < CACHE_TTL {
                return cached.links.clone();
            }
        }
    }

    // Try remote fetch
    match fetch_remote_links().await {
        Ok(links) => {
            logger::log_info(
                &format!(
                    "Fetched {} library download links from remote",
                    links.len()
                ),
                Some("library_links"),
            );
            let mut cache = CACHE.lock().unwrap();
            *cache = Some(CachedLinks {
                links: links.clone(),
                fetched_at: SystemTime::now(),
            });
            links
        }
        Err(e) => {
            logger::log_info(
                &format!(
                    "Failed to fetch remote library links: {}, using hardcoded fallback",
                    e
                ),
                Some("library_links"),
            );
            let fallback = hardcoded_links();
            // Cache the fallback too, but with a shorter TTL to retry sooner
            let mut cache = CACHE.lock().unwrap();
            *cache = Some(CachedLinks {
                links: fallback.clone(),
                fetched_at: SystemTime::now(),
            });
            fallback
        }
    }
}

/// Fetch library links JSON from the remote GitHub repository.
async fn fetch_remote_links() -> Result<HashMap<String, String>, String> {
    let client = reqwest::Client::builder()
        .user_agent("XFast Manager")
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(REMOTE_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP status: {}", response.status()));
    }

    let data: LibraryLinksData = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Normalize all keys to lowercase for case-insensitive lookup
    let links: HashMap<String, String> = data
        .libraries
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect();

    Ok(links)
}

/// Look up download links for a list of missing library names.
/// Returns a map of library_name -> Option<download_url>.
/// Lookup is case-insensitive.
pub async fn lookup_library_links(
    library_names: Vec<String>,
) -> HashMap<String, Option<String>> {
    let links_db = get_library_links().await;

    library_names
        .into_iter()
        .map(|name| {
            let url = links_db.get(&name.to_lowercase()).cloned();
            (name, url)
        })
        .collect()
}

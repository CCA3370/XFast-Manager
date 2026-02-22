use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime};

use crate::logger;

/// Remote JSON schema for library download links
#[derive(Debug, Deserialize)]
struct LibraryLinksData {
    // Retained for schema version compatibility checks (future use)
    #[allow(dead_code)]
    version: u32,
    // Retained for cache freshness validation (future use)
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
static CACHE: LazyLock<Mutex<Option<CachedLinks>>> = LazyLock::new(|| Mutex::new(None));

/// Cache TTL: 24 hours (matching updater pattern)
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Remote URL for the library links JSON file
const REMOTE_URL: &str =
    "https://raw.githubusercontent.com/CCA3370/XFast-Manager/dev/data/library_links.json";

fn normalize_library_key(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .trim_matches(|c| c == '/' || c == '\\' || c == '\0');
    let first_component = trimmed.split(&['/', '\\'][..]).next().unwrap_or(trimmed);
    first_component.trim().to_lowercase()
}

fn find_library_url(links_db: &HashMap<String, String>, name: &str) -> Option<String> {
    let normalized = normalize_library_key(name);
    if normalized.is_empty() {
        return None;
    }

    links_db.get(&normalized).cloned().or_else(|| {
        let dashed = normalized.replace('_', "-");
        if dashed != normalized {
            return links_db.get(&dashed).cloned();
        }

        let underscored = normalized.replace('-', "_");
        if underscored != normalized {
            return links_db.get(&underscored).cloned();
        }

        None
    })
}

/// Returns the hardcoded fallback library links database.
/// Used when the remote fetch fails (network unavailable, timeout, etc.).
fn hardcoded_links() -> HashMap<String, String> {
    let embedded_json = include_str!("../../../data/library_links.json");

    match serde_json::from_str::<LibraryLinksData>(embedded_json) {
        Ok(data) => data
            .libraries
            .into_iter()
            .map(|(k, v)| (normalize_library_key(&k), v))
            .filter(|(k, _)| !k.is_empty())
            .collect(),
        Err(e) => {
            logger::log_info(
                &format!(
                    "Failed to parse embedded data/library_links.json: {}, using empty fallback",
                    e
                ),
                Some("library_links"),
            );
            HashMap::new()
        }
    }
}

/// Fetch library links from the remote JSON, with in-memory caching.
/// Returns an error on fetch/parse failure.
async fn get_remote_links(force_refresh: bool) -> Result<HashMap<String, String>, String> {
    // Check cache first
    if !force_refresh {
        let cache = CACHE
            .lock()
            .expect("library links cache lock poisoned during read");
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed().unwrap_or(CACHE_TTL) < CACHE_TTL {
                return Ok(cached.links.clone());
            }
        }
    }

    // Try remote fetch
    match fetch_remote_links().await {
        Ok(links) => {
            logger::log_info(
                &format!("Fetched {} library download links from remote", links.len()),
                Some("library_links"),
            );
            let mut cache = CACHE
                .lock()
                .expect("library links cache lock poisoned during write");
            *cache = Some(CachedLinks {
                links: links.clone(),
                fetched_at: SystemTime::now(),
            });
            Ok(links)
        }
        Err(e) => Err(e),
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
        .map(|(k, v)| (normalize_library_key(&k), v))
        .filter(|(k, _)| !k.is_empty())
        .collect();

    Ok(links)
}

/// Look up download links for a list of missing library names.
/// Returns a map of library_name -> Option<download_url>.
/// Lookup is case-insensitive.
pub async fn lookup_library_links_local(
    library_names: Vec<String>,
) -> HashMap<String, Option<String>> {
    let links_db = hardcoded_links();

    library_names
        .into_iter()
        .map(|name| {
            let url = find_library_url(&links_db, &name);
            (name, url)
        })
        .collect()
}

/// Look up download links using remote JSON source.
/// Uses in-memory cache and returns error when remote is unavailable.
pub async fn lookup_library_links_remote(
    library_names: Vec<String>,
    force_refresh: bool,
) -> Result<HashMap<String, Option<String>>, String> {
    let links_db = get_remote_links(force_refresh).await?;

    Ok(library_names
        .into_iter()
        .map(|name| {
            let url = find_library_url(&links_db, &name);
            (name, url)
        })
        .collect())
}

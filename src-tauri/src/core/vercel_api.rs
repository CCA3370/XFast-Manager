const DEFAULT_VERCEL_API_BASE_URL: &str = "https://x-fast-manager.vercel.app/api";

fn normalized_base_url() -> String {
    let configured = std::env::var("XFAST_VERCEL_API_BASE_URL").ok();
    let base = configured
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_VERCEL_API_BASE_URL);
    let trimmed = base.trim_end_matches('/');

    if trimmed.ends_with("/api") {
        trimmed.to_string()
    } else {
        format!("{}/api", trimmed)
    }
}

fn normalized_route(route: &str) -> &str {
    route.trim().trim_start_matches('/')
}

pub fn endpoint(route: &str) -> String {
    format!("{}/{}", normalized_base_url(), normalized_route(route))
}

pub fn endpoint_with_override(route: &str, override_env: &str) -> String {
    if let Ok(value) = std::env::var(override_env) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    endpoint(route)
}

pub fn append_query_param(url: &str, key: &str, value: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}{key}={value}")
}

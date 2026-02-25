// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workaround for WebKitGTK rendering freeze on certain Linux GPU configurations.
    // The DMA-BUF renderer can cause the entire UI to hang (no animations, no interaction).
    // This must be set BEFORE the WebView is created.
    #[cfg(target_os = "linux")]
    {
        let set_default_env = |key: &str, value: &str| {
            if std::env::var_os(key).is_none() {
                std::env::set_var(key, value);
            }
        };

        set_default_env("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        // Workaround for "Could not create default EGL display: EGL_BAD_PARAMETER" on
        // certain Wayland systems (e.g. EndeavourOS, Arch Linux). WebKit still tries to
        // initialise an EGL display for compositing even when DMA-BUF is disabled; setting
        // this variable prevents that code path from running.
        set_default_env("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

        // Some Wayland + AppImage combinations still fail early with:
        // "Could not create default EGL display: EGL_BAD_PARAMETER".
        // Force GTK to use X11 backend as a fallback, but never override user settings.
        let is_wayland_session = std::env::var("XDG_SESSION_TYPE")
            .map(|session| session.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
            || std::env::var_os("WAYLAND_DISPLAY").is_some();
        let is_appimage = std::env::var_os("APPIMAGE").is_some();

        if is_appimage && is_wayland_session && std::env::var_os("GDK_BACKEND").is_none() {
            std::env::set_var("GDK_BACKEND", "x11");
        }
    }

    xfastmanager_lib::run();
}

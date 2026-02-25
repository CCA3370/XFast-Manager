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

        // Default is native backend (Wayland/X11 decided by GTK).
        // For problematic Linux environments, allow explicit X11 fallback:
        // XFAST_FORCE_X11=1 ./XFast-Manager.AppImage
        let force_x11 = std::env::var("XFAST_FORCE_X11")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if force_x11 && std::env::var_os("GDK_BACKEND").is_none() {
            std::env::set_var("GDK_BACKEND", "x11");
        }
    }

    xfastmanager_lib::run();
}

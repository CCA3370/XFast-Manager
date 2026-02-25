// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workaround for WebKitGTK rendering freeze on certain Linux GPU configurations.
    // The DMA-BUF renderer can cause the entire UI to hang (no animations, no interaction).
    // This must be set BEFORE the WebView is created.
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        // Workaround for "Could not create default EGL display: EGL_BAD_PARAMETER" on
        // certain Wayland systems (e.g. EndeavourOS, Arch Linux). WebKit still tries to
        // initialise an EGL display for compositing even when DMA-BUF is disabled; setting
        // this variable prevents that code path from running.
        if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }

    xfastmanager_lib::run();
}

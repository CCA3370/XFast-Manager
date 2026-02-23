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
    }

    xfastmanager_lib::run();
}

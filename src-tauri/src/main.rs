// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
fn env_var_is_truthy(key: &str) -> bool {
    std::env::var(key)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_force_x11_for_linux_launch_env(
    gdk_backend_already_set: bool,
    force_x11: bool,
    prefer_wayland: bool,
    is_appimage: bool,
    is_wayland_session: bool,
    has_x11_display: bool,
) -> bool {
    if gdk_backend_already_set {
        return false;
    }

    if force_x11 {
        return true;
    }

    if prefer_wayland {
        return false;
    }

    is_appimage && is_wayland_session && has_x11_display
}

#[cfg(target_os = "linux")]
fn should_force_x11_for_linux_launch() -> bool {
    should_force_x11_for_linux_launch_env(
        std::env::var_os("GDK_BACKEND").is_some(),
        env_var_is_truthy("XFAST_FORCE_X11"),
        env_var_is_truthy("XFAST_PREFER_WAYLAND"),
        std::env::var_os("APPIMAGE").is_some(),
        std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE")
                .map(|value| value.eq_ignore_ascii_case("wayland"))
                .unwrap_or(false),
        std::env::var_os("DISPLAY").is_some(),
    )
}

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
        // Additionally, prefer X11 automatically for AppImage launches under Wayland
        // when an X11 display is available. This avoids the Wayland/EGL startup crash
        // reported on some Arch-based systems while keeping local/dev builds native.
        if should_force_x11_for_linux_launch() {
            std::env::set_var("GDK_BACKEND", "x11");
        }
    }

    xfastmanager_lib::run();
}

#[cfg(test)]
mod tests {
    use super::should_force_x11_for_linux_launch_env;

    #[test]
    fn appimage_wayland_with_x11_display_forces_x11() {
        assert!(should_force_x11_for_linux_launch_env(
            false, false, false, true, true, true
        ));
    }

    #[test]
    fn appimage_wayland_without_x11_display_does_not_force_x11() {
        assert!(!should_force_x11_for_linux_launch_env(
            false, false, false, true, true, false
        ));
    }

    #[test]
    fn explicit_wayland_preference_disables_auto_x11() {
        assert!(!should_force_x11_for_linux_launch_env(
            false, false, true, true, true, true
        ));
    }

    #[test]
    fn explicit_force_x11_overrides_non_appimage_launch() {
        assert!(should_force_x11_for_linux_launch_env(
            false, true, false, false, false, false
        ));
    }

    #[test]
    fn existing_gdk_backend_is_respected() {
        assert!(!should_force_x11_for_linux_launch_env(
            true, true, false, true, true, true
        ));
    }
}

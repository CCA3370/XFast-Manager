fn main() {
    if std::env::var("TARGET").is_ok_and(|target| target.ends_with("-windows-msvc")) {
        // Tauri links TaskDialogIndirect from Common Controls v6. Unit test
        // executables also need that dependency to load before the harness runs.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' \
             name='Microsoft.Windows.Common-Controls' version='6.0.0.0' \
             processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
        // Tauri already embeds its own manifest in the app binary (and its
        // binary unit tests). Avoid generating a second manifest for that target.
        println!("cargo:rustc-link-arg-bin=xfastmanager=/MANIFEST:NO");
    }

    tauri_build::build()
}

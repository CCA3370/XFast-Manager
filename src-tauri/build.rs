use std::path::PathBuf;

fn main() {
    // Tauri's application binary receives its Windows manifest from tauri-build,
    // but Cargo's Rust test harness does not. Without Common Controls v6,
    // Windows can fail to load the test executable before any test runs
    // (STATUS_ENTRYPOINT_NOT_FOUND / TaskDialogIndirect).
    //
    // Scope these linker arguments to test binaries only so the packaged app
    // continues to use Tauri's own manifest unchanged.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest = PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR must be set by Cargo"),
        )
        .join("tests")
        .join("windows-test.manifest");

        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }

    tauri_build::build()
}

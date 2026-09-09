//! An application started with no arguments must expose no activation items.
//!
//! This lives in its own integration-test binary because an Avalonia
//! application lifetime is process-wide: only one `App::run` may happen per
//! process.

use avalonia::{App, TextBlock, Window};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn host_path() -> PathBuf {
    if let Ok(path) = std::env::var("AVN_HOST_NATIVE_LIB") {
        return PathBuf::from(path);
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    #[cfg(target_os = "windows")]
    let candidates = [
        "host/bin/Release/net10.0/win-x64/publish/Avalonia.Host.dll",
        "host/bin/Release/net10.0/win-arm64/publish/Avalonia.Host.dll",
        "host/bin/Debug/net10.0/win-x64/publish/Avalonia.Host.dll",
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        "rust/target/dotnet-linux-x64/publish/Avalonia.Host/release_linux-x64/Avalonia.Host.so",
        "rust/target/dotnet-linux-arm64/publish/Avalonia.Host/release_linux-arm64/Avalonia.Host.so",
    ];
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let candidates: [&str; 0] = [];

    candidates
        .into_iter()
        .map(|relative| root.join(relative))
        .find(|path| path.exists())
        .unwrap_or_else(|| {
            panic!(
                "Avalonia.Host native library not found. Publish with \
                 `rust/build.ps1` on Windows or `rust/build.sh` on Linux, or set \
                 AVN_HOST_NATIVE_LIB."
            )
        })
}

// Same host-launch family as desktop_files.rs: requires an interactive desktop session.
#[ignore = "host-side managed exception under headless runners (tracked)"]
#[test]
fn explicitly_empty_startup_arguments_produce_no_activation_items() {
    let ran = Arc::new(AtomicBool::new(false));
    let ran_in_handler = ran.clone();

    App::load(host_path())
        .unwrap()
        .without_startup_arguments()
        .run(move |scope| {
            assert!(scope.startup_arguments()?.is_empty());
            assert!(scope.activation_items()?.is_empty());
            ran_in_handler.store(true, Ordering::SeqCst);

            let window = Window::new()?
                .title("no activation")?
                .content(Some(&TextBlock::new()?.text("no activation")?))?;
            scope.mount(window.clone())?;
            scope.post(move || window.close().expect("close the test window"))?;
            Ok(())
        })
        .unwrap();

    assert!(ran.load(Ordering::SeqCst));
}

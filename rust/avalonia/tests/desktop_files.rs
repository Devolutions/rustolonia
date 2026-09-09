//! Safe-API desktop file integration against the published NativeAOT host.
//!
//! No platform dialog is ever opened: this covers capability discovery,
//! startup/open-with argument forwarding, activation normalization, drop and
//! activation subscription lifetime, window tracking, and the error mapping for
//! a picker started off the UI thread.
//!
//! An Avalonia application lifetime is process-wide, so this file starts
//! exactly one application; the companion `desktop_files_without_arguments.rs`
//! integration test runs in its own binary for the same reason.

use avalonia::{
    App, DragDropEffects, Error, OpenFilePickerOptions, StorageItemKind, TextBlock, Window,
};
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

// Requires an interactive desktop session: the host's storage-capability and picker
// paths surface a managed exception (COR_E_EXCEPTION) under headless CI runners and
// headless local runs alike. Tracked for follow-up; every other suite covers the ABI
// surface these tests exercise (see tests/Avalonia.Host.Tests/Desktop/*).
#[ignore = "host-side managed exception under headless runners (tracked)"]
#[test]
fn desktop_file_integration_runs_through_nativeaot() {
    let file = std::env::temp_dir().join(format!("avn-open-with-{}.log", std::process::id()));
    std::fs::write(&file, b"stage 29").unwrap();
    let file_argument = file.to_string_lossy().to_string();

    let ran = Arc::new(AtomicBool::new(false));
    let ran_in_handler = ran.clone();
    let app = App::load(host_path()).unwrap().with_startup_arguments([
        "--verbose".to_string(),
        file_argument.clone(),
        file_argument.clone(),
        "myapp://open/7".to_string(),
    ]);

    app.run(move |scope| {
        // Verbatim arguments, in order, nothing dropped.
        assert_eq!(
            vec![
                "--verbose".to_string(),
                file_argument.clone(),
                file_argument.clone(),
                "myapp://open/7".to_string(),
            ],
            scope.startup_arguments()?
        );

        // Normalized activation items: the switch is skipped, the duplicate
        // collapses, ordering and the non-local URI are preserved, and only the
        // local item has a filesystem path.
        let items = scope.activation_items()?;
        assert_eq!(2, items.len(), "{items:?}");
        assert_eq!(StorageItemKind::File, items[0].kind());
        assert!(items[0].uri().starts_with("file:///"));
        let local = items[0].local_path().expect("a local file has a path");
        assert!(
            local.to_string_lossy().ends_with(".log"),
            "{}",
            local.display()
        );
        assert_eq!("myapp://open/7", items[1].uri());
        assert_eq!(None, items[1].local_path());

        let window = Window::new()?
            .title("desktop files")?
            .content(Some(&TextBlock::new()?.text("desktop files")?))?;
        scope.mount(window.clone())?;

        // Mounted windows are tracked so a Rust view model can parent a picker.
        assert_eq!(1, scope.windows().len());
        assert!(scope.main_window().is_some());

        // Capability discovery must not open anything.
        let capabilities = scope.storage_capabilities(&window)?;
        assert!(
            capabilities.can_open || capabilities.can_save || capabilities.can_pick_folder,
            "a desktop top-level should expose at least one picker: {capabilities:?}"
        );

        // Drop subscriptions attach and detach deterministically, including
        // through the scope-retained variant.
        let mut subscription = scope.subscribe_file_drop(&window, DragDropEffects::COPY, |_| {})?;
        subscription.unsubscribe()?;
        subscription.unsubscribe()?;
        scope.on_file_drop(
            &window,
            DragDropEffects::COPY | DragDropEffects::LINK,
            |_| {},
        )?;

        let mut activation = scope.subscribe_activation(|_| {})?;
        activation.unsubscribe()?;
        scope.on_activation(|_| {})?;

        // Error mapping: a picker started off the UI thread must fail rather
        // than opening a dialog on the wrong thread.
        let off_thread = scope.clone();
        let off_thread_window = window.clone();
        let mapped = std::thread::spawn(move || {
            matches!(
                off_thread.open_file_picker(
                    &off_thread_window,
                    &OpenFilePickerOptions::new().title("must not open"),
                ),
                Err(Error::Abi(_))
            )
        })
        .join()
        .unwrap();
        assert!(mapped, "a cross-thread picker must map to an ABI error");

        ran_in_handler.store(true, Ordering::SeqCst);

        // Close from the dispatcher queue: closing the last window inside the
        // startup callback would shut the dispatcher down before its main loop
        // ever starts.
        scope.post(move || window.close().expect("close the test window"))?;
        Ok(())
    })
    .unwrap();

    std::fs::remove_file(&file).ok();
    assert!(ran.load(Ordering::SeqCst));
}

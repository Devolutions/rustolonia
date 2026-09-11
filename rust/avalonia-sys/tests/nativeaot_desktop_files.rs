//! Raw ABI conformance for the stage 29 desktop file integration capability.
//!
//! These run against the published NativeAOT host and prove that every vtable
//! slot of the new, separately versioned interfaces lines up between the
//! managed `[GeneratedComInterface]` declarations and the handwritten Rust
//! bindings. Nothing here opens a platform dialog.

use avalonia_sys::{
    activation_handler, app_handler, file_drop_handler, storage_completion, ComPtr, Error, Host,
    IAvnApplication3, IAvnControl, IAvnFilePickerOptions, IAvnStorageCompletion, IAvnWindow,
    AVN_E_FIXTURE, E_INVALIDARG, E_POINTER,
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
    for relative in candidates {
        let path = root.join(relative);
        if path.exists() {
            return path;
        }
    }
    panic!(
        "Avalonia.Host native library not found. Run `rust/build.ps1` on \
         Windows or `rust/build.sh` on Linux, or set AVN_HOST_NATIVE_LIB."
    );
}

fn utf16(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn load() -> Host {
    let path = host_path();
    Host::load(&path).unwrap_or_else(|error| panic!("load {}: {error}", path.display()))
}

/// Startup arguments, activation items and picker options need no UI thread, so
/// they are exercised before `Run` is ever called.
#[ignore = "requires a desktop session: host bootstrap needs a display (X11) or the macOS main thread (tracked)"]
#[test]
fn startup_arguments_and_activation_items_round_trip_without_a_ui_thread() {
    let host = load();
    let application = host
        .activation_factory()
        .unwrap()
        .create_application()
        .unwrap();
    let desktop: ComPtr<IAvnApplication3> = application.desktop_files().unwrap();

    desktop.clear_startup_arguments().unwrap();
    assert_eq!(0, desktop.startup_argument_count().unwrap());
    assert_eq!(
        E_INVALIDARG,
        desktop.startup_argument(0).unwrap_err().0,
        "an out-of-range startup argument must be rejected, not returned empty"
    );

    let file = std::env::temp_dir().join(format!("avn-abi-open-with-{}.log", std::process::id()));
    std::fs::write(&file, b"stage 29").unwrap();
    let file_argument = file.to_string_lossy().to_string();

    for argument in [
        "--verbose",
        file_argument.as_str(),
        file_argument.as_str(),
        "myapp://open/7",
    ] {
        desktop
            .add_startup_argument(Some(&utf16(argument)))
            .unwrap();
    }

    // Verbatim, ordered, nothing dropped.
    assert_eq!(4, desktop.startup_argument_count().unwrap());
    assert_eq!(
        Some("--verbose".to_string()),
        desktop.startup_argument(0).unwrap()
    );
    assert_eq!(
        Some("myapp://open/7".to_string()),
        desktop.startup_argument(3).unwrap()
    );

    // Normalized: the switch is skipped, the duplicate collapses, order and the
    // non-local URI are preserved.
    let items = desktop.activation_items().unwrap();
    std::fs::remove_file(&file).ok();
    assert_eq!(2, items.len(), "{items:?}");
    assert_eq!(0, items[0].kind);
    assert!(items[0].uri.as_deref().unwrap().starts_with("file:///"));
    assert!(items[0].local_path.is_some());
    assert_eq!(Some("myapp://open/7"), items[1].uri.as_deref());
    assert_eq!(None, items[1].local_path);

    desktop.clear_startup_arguments().unwrap();
    assert_eq!(0, desktop.startup_argument_count().unwrap());
    assert!(desktop.activation_items().unwrap().is_empty());
}

#[ignore = "requires a desktop session: host bootstrap needs a display (X11) or the macOS main thread (tracked)"]
#[test]
fn picker_options_accept_every_documented_field() {
    let host = load();
    let application = host
        .activation_factory()
        .unwrap()
        .create_application()
        .unwrap();
    let desktop = application.desktop_files().unwrap();
    let options = desktop.create_picker_options().unwrap();

    options.set_title(Some(&utf16("Open logs"))).unwrap();
    options.set_allow_multiple(true).unwrap();
    options
        .set_suggested_file_name(Some(&utf16("trace.log")))
        .unwrap();
    options
        .set_suggested_start_location(Some(&utf16("file:///logs/")))
        .unwrap();
    options.set_suggested_start_well_known_folder(1).unwrap();
    options.set_default_extension(Some(&utf16("log"))).unwrap();
    options.set_show_overwrite_prompt(1).unwrap();

    let logs = options.add_file_type(Some(&utf16("Log files"))).unwrap();
    assert_eq!(0, logs);
    options
        .add_file_type_pattern(logs, &utf16("*.log"))
        .unwrap();
    options
        .add_file_type_mime_type(logs, &utf16("text/plain"))
        .unwrap();
    options
        .add_file_type_apple_uniform_type_identifier(logs, &utf16("public.plain-text"))
        .unwrap();
    let all = options.add_file_type(Some(&utf16("All files"))).unwrap();
    assert_eq!(1, all);
    options.set_suggested_file_type_index(all).unwrap();

    // Out-of-range indices and unknown well-known folders are rejected; -1
    // clears an optional value instead of failing.
    assert_eq!(
        E_INVALIDARG,
        options
            .add_file_type_pattern(9, &utf16("*.txt"))
            .unwrap_err()
            .0
    );
    assert_eq!(
        E_INVALIDARG,
        options.set_suggested_file_type_index(9).unwrap_err().0
    );
    assert_eq!(
        E_INVALIDARG,
        options
            .set_suggested_start_well_known_folder(99)
            .unwrap_err()
            .0
    );
    options.set_suggested_start_well_known_folder(-1).unwrap();
    options.set_suggested_file_type_index(-1).unwrap();
    options.set_show_overwrite_prompt(-1).unwrap();
    options.set_title(None).unwrap();
}

#[ignore = "requires a desktop session: host bootstrap needs a display (X11) or the macOS main thread (tracked)"]
#[test]
fn null_arguments_are_rejected_before_any_work_happens() {
    let host = load();
    let application = host
        .activation_factory()
        .unwrap()
        .create_application()
        .unwrap();
    let desktop = application.desktop_files().unwrap();

    // `AddStartupArgument(null)` is the one string parameter that must not be
    // silently treated as an empty argument: an activation list is ordered and
    // positional, so a missing entry has to fail loudly.
    assert_eq!(E_POINTER, desktop.add_startup_argument(None).unwrap_err().0);
    assert_eq!(0, desktop.startup_argument_count().unwrap());
}

/// Drop subscriptions, activation advises and picker starts are UI-thread
/// affine, so they run inside the application's startup handler.
#[ignore = "requires a desktop session: host bootstrap needs a display (X11) or the macOS main thread (tracked)"]
#[test]
fn drop_and_activation_subscriptions_attach_and_detach_on_the_ui_thread() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let application = factory.create_application().unwrap();
    let controls = factory.create_control_factory().unwrap();
    let ran = Arc::new(AtomicBool::new(false));
    let ran_in_handler = ran.clone();

    let application_for_handler = application.clone();
    let handler = app_handler(move || {
        let desktop = application_for_handler.desktop_files()?;

        let border = controls.create_border()?;
        let target: ComPtr<IAvnControl> = border.query_interface()?;
        let drop_handler = file_drop_handler(|_| Ok(()));

        // DragDropEffects.Copy | DragDropEffects.Link
        let subscription =
            desktop.subscribe_file_drop(Some(&target), 1 | 4, Some(&drop_handler))?;
        desktop.unsubscribe_file_drop(subscription)?;
        assert_eq!(
            E_INVALIDARG,
            desktop.unsubscribe_file_drop(subscription).unwrap_err().0,
            "unsubscribing twice must be rejected"
        );
        assert_eq!(
            E_POINTER,
            desktop
                .subscribe_file_drop(None, 1, Some(&drop_handler))
                .unwrap_err()
                .0
        );
        assert_eq!(
            E_POINTER,
            desktop
                .subscribe_file_drop(Some(&target), 1, None)
                .unwrap_err()
                .0
        );

        let activation = activation_handler(|_| Ok(()));
        let activation_subscription = desktop.advise_activation(Some(&activation))?;
        desktop.unadvise_activation(activation_subscription)?;
        assert_eq!(
            E_INVALIDARG,
            desktop
                .unadvise_activation(activation_subscription)
                .unwrap_err()
                .0
        );
        assert_eq!(E_POINTER, desktop.advise_activation(None).unwrap_err().0);

        // Capability discovery is the only picker-family slot that can be
        // exercised against a real window without opening a dialog.
        let window = controls.create_window()?;
        let capabilities = desktop.storage_capabilities(Some(&window))?;
        assert_eq!(
            0,
            capabilities & !0b111,
            "unexpected capability bits: {capabilities:#b}"
        );
        assert_eq!(
            E_POINTER,
            desktop.storage_capabilities(None).unwrap_err().0,
            "a null window must be rejected"
        );

        // The three picker starts share one signature, and the host rejects a
        // null window or options before any dialog work happens. Calling each
        // one is what proves its vtable slot and call frame line up; a real
        // dialog would block this test forever.
        let options = desktop.create_picker_options()?;
        let completion = storage_completion(|_| Ok(()));
        for start in [
            IAvnApplication3Start::OpenFile,
            IAvnApplication3Start::OpenFolder,
            IAvnApplication3Start::SaveFile,
        ] {
            assert_eq!(
                E_POINTER,
                start
                    .call(&desktop, None, Some(&options), Some(&completion))
                    .unwrap_err()
                    .0,
                "{start:?}: a null window must be rejected"
            );
            assert_eq!(
                E_POINTER,
                start
                    .call(&desktop, Some(&window), None, Some(&completion))
                    .unwrap_err()
                    .0,
                "{start:?}: null options must be rejected"
            );
            assert_eq!(
                E_POINTER,
                start
                    .call(&desktop, Some(&window), Some(&options), None)
                    .unwrap_err()
                    .0,
                "{start:?}: a null completion must be rejected"
            );
        }

        window.close()?;

        ran_in_handler.store(true, Ordering::SeqCst);
        Err(Error(AVN_E_FIXTURE))
    });

    let error = application.run(&handler).unwrap_err();
    assert_eq!(
        error.0,
        AVN_E_FIXTURE,
        "host startup failed: {}",
        host.last_error().unwrap_or_default()
    );
    assert!(ran.load(Ordering::SeqCst));
}

/// The three picker starts share one ABI shape; this keeps the conformance
/// assertions above identical for all of them.
#[derive(Clone, Copy, Debug)]
enum IAvnApplication3Start {
    OpenFile,
    OpenFolder,
    SaveFile,
}

impl IAvnApplication3Start {
    fn call(
        self,
        desktop: &ComPtr<IAvnApplication3>,
        window: Option<&ComPtr<IAvnWindow>>,
        options: Option<&ComPtr<IAvnFilePickerOptions>>,
        completion: Option<&ComPtr<IAvnStorageCompletion>>,
    ) -> Result<i64, Error> {
        match self {
            Self::OpenFile => desktop.start_open_file_picker(window, options, completion),
            Self::OpenFolder => desktop.start_open_folder_picker(window, options, completion),
            Self::SaveFile => desktop.start_save_file_picker(window, options, completion),
        }
    }
}

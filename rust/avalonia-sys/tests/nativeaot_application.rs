use avalonia_sys::{app_handler, button_click_handler, Error, Host, AVN_E_FIXTURE};
use libloading::Library;
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct ProjectionDiagnosticSnapshot {
    wrappers_created: i64,
    tracked_object_ids: i32,
    live_managed_objects: i32,
    active_subscriptions: i64,
    native_ownership_releases: i64,
}

type GetProjectionDiagnosticsFn = unsafe extern "C" fn(*mut ProjectionDiagnosticSnapshot) -> i32;

fn projection_diagnostics(
    get_diagnostics: GetProjectionDiagnosticsFn,
) -> ProjectionDiagnosticSnapshot {
    let mut snapshot = ProjectionDiagnosticSnapshot::default();
    assert_eq!(unsafe { get_diagnostics(&mut snapshot) }, 0);
    snapshot
}

#[ignore = "requires a desktop session: host bootstrap needs a display (X11) or the macOS main thread (tracked)"]
#[test]
fn application_runs_generated_object_model_through_rust_handler() {
    let path = host_path();
    let host = Host::load(&path).unwrap_or_else(|error| panic!("load {}: {error}", path.display()));
    let library = unsafe { Library::new(path).unwrap() };
    let get_diagnostics = unsafe {
        *library
            .get::<GetProjectionDiagnosticsFn>(b"avn_get_projection_diagnostics")
            .unwrap()
    };
    let ownership_before = projection_diagnostics(get_diagnostics);
    let activation = host.activation_factory().unwrap();
    let application = activation.create_application().unwrap();
    let controls = activation.create_control_factory().unwrap();
    let called = Arc::new(AtomicBool::new(false));
    let called_from_handler = called.clone();
    let clicked = Arc::new(AtomicBool::new(false));
    let clicked_from_handler = clicked.clone();
    let handler = app_handler(move || {
        let ownership_probe = controls.create_button()?;
        let ownership_created = projection_diagnostics(get_diagnostics);
        assert_eq!(
            ownership_created.wrappers_created,
            ownership_before.wrappers_created + 1
        );
        assert_eq!(
            ownership_created.tracked_object_ids,
            ownership_before.tracked_object_ids + 1
        );
        drop(ownership_probe);
        let ownership_released = projection_diagnostics(get_diagnostics);
        assert_eq!(
            ownership_released.native_ownership_releases,
            ownership_before.native_ownership_releases + 1
        );
        assert_eq!(
            ownership_released.tracked_object_ids,
            ownership_before.tracked_object_ids
        );

        let button = controls.create_button()?;
        let text = controls.create_text_block()?;
        assert!(button.get_is_enabled()?);
        button.set_is_enabled(false)?;
        assert!(!button.get_is_enabled()?);

        let text_as_control = text.query_interface::<avalonia_sys::IAvnControl>()?;
        button.set_content(Some(&text_as_control))?;
        let content = button.get_content()?.unwrap();
        assert_eq!(text.object_id()?, content.object_id()?);

        let click_handler = button_click_handler(move || {
            clicked_from_handler.store(true, Ordering::SeqCst);
            Ok(())
        });
        click_handler.invoke()?;
        let click_subscription = button.advise_click(&click_handler)?;
        button.unadvise_click(click_subscription)?;

        let panel = controls.create_stack_panel()?;
        let children = panel.get_children()?;
        children.add(&text_as_control)?;
        let button_as_control = button.query_interface::<avalonia_sys::IAvnControl>()?;
        children.add(&button_as_control)?;
        assert_eq!(children.len()?, 2);
        assert_eq!(children.get(0)?.object_id()?, text.object_id()?);
        children.remove(1)?;
        assert_eq!(children.len()?, 1);
        children.clear()?;
        assert_eq!(children.len()?, 0);

        called_from_handler.store(true, Ordering::SeqCst);
        Err(Error(AVN_E_FIXTURE))
    });

    let error = application.run(&handler).unwrap_err();
    assert_eq!(
        error.0,
        AVN_E_FIXTURE,
        "host startup failed: {}",
        host.last_error().unwrap_or_default()
    );
    assert!(called.load(Ordering::SeqCst));
    assert!(clicked.load(Ordering::SeqCst));
}

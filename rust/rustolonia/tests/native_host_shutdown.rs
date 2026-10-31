#![cfg(target_os = "macos")]

use std::process::Command;

#[test]
#[ignore = "requires a macOS desktop session and a built native host; process-exit teardown is under test"]
fn shutdown_releases_native_dispatcher_before_process_exit() {
    let output = Command::new(env!("CARGO_BIN_EXE_native_host_shutdown"))
        .output()
        .expect("run the main-thread shutdown harness");
    assert!(
        output.status.success(),
        "shutdown harness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires a macOS desktop session and a built native host"]
fn tray_icon_menu_and_shutdown_run_through_native_host() {
    let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../avalonia-src/tests/Avalonia.RenderTests/Assets/Star512.png");
    assert!(icon.is_file(), "native tray fixture icon is missing");
    let output = Command::new(env!("CARGO_BIN_EXE_native_host_shutdown"))
        .arg(icon)
        .output()
        .expect("run the native tray harness");
    assert!(
        output.status.success(),
        "native tray harness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

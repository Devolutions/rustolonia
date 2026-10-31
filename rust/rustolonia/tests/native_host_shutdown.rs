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

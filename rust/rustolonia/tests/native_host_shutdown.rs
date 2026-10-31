#![cfg(target_os = "macos")]

use std::process::Command;

#[test]
#[ignore = "requires a macOS desktop session and a built native host"]
fn auxiliary_popup_gate_teardown_matrix() {
    for menu in [false, true] {
        for revealed in [false, true] {
            for shutdown in [false, true] {
                let mut command = Command::new(env!("CARGO_BIN_EXE_native_popup_host"));
                command.arg("--teardown");
                if menu {
                    command.arg("--menu");
                }
                if revealed {
                    command.arg("--revealed");
                }
                if shutdown {
                    command.arg("--shutdown");
                }
                let output = command.output().unwrap();
                println!("{}", String::from_utf8_lossy(&output.stdout));
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}

#[test]
#[ignore = "requires a macOS desktop session and a built native host"]
fn native_tooltip_placement_matches_after_direct_appkit_owner_moves() {
    let output = Command::new(env!("CARGO_BIN_EXE_native_popup_host"))
        .arg("--placement")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
#[ignore = "requires a macOS desktop session and a built native host"]
fn native_popup_gate_defers_show_and_rejects_stale_tokens() {
    let output = Command::new(env!("CARGO_BIN_EXE_native_popup_host"))
        .arg("--gate")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
#[ignore = "requires a macOS desktop session and a built native host"]
fn scoped_popup_hosting_uses_no_native_windows() {
    let output = Command::new(env!("CARGO_BIN_EXE_native_popup_host"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

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

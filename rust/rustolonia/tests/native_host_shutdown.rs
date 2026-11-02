#![cfg(target_os = "macos")]

use std::process::Command;

#[test]
#[ignore = "requires a macOS desktop session and a built native host"]
fn file_drag_preserves_clicks_and_rejects_cancelled_gestures() {
    for mode in [
        "click",
        "release",
        "cancel",
        "owner-close",
        "native-cancel",
        "native-owner-close",
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_native_popup_host"))
            .args(["--file-drag", mode])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while child.try_wait().unwrap().is_none() {
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!(
                    "drag fixture {mode} timed out:\n{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let output = child.wait_with_output().unwrap();
        println!("{}", String::from_utf8_lossy(&output.stdout));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
#[ignore = "requires a macOS desktop session and a built native host"]
fn application_termination_defers_cleanup_for_tray_only_and_popup_owners() {
    for native in [false, true] {
        for popup in [false, true] {
            if !native && popup {
                continue;
            }
            for os in [false, true] {
                if os && !native {
                    continue;
                }
                let mut command = Command::new(env!("CARGO_BIN_EXE_native_popup_host"));
                command.arg("--termination");
                if native {
                    command.arg("--native");
                }
                if popup {
                    command.arg("--popup");
                }
                if os {
                    command.arg("--os");
                }
                let mut child = command
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
                while child.try_wait().unwrap().is_none() {
                    if std::time::Instant::now() >= deadline {
                        child.kill().unwrap();
                        let output = child.wait_with_output().unwrap();
                        panic!(
                            "termination fixture timed out:\n{}\n{}",
                            String::from_utf8_lossy(&output.stdout),
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                let output = child.wait_with_output().unwrap();
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("{stdout}");
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(stdout.contains("asynchronous cleanup complete"));
                if native {
                    assert!(stdout.contains("NSApplicationWillTerminateNotification after cleanup"));
                } else {
                    assert!(stdout.contains("App::run returned after cleanup"));
                }
                if popup {
                    assert!(stdout.contains("termination popup Closed after cleanup"));
                    assert!(stdout.contains("termination owner Unloaded after cleanup"));
                }
            }
        }
    }
}

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
fn auxiliary_gate_clears_in_owner_unloaded_before_automatic_shutdown() {
    for menu in [false, true] {
        for revealed in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_native_popup_host"));
            command.args(["--teardown", "--automatic"]);
            if menu {
                command.arg("--menu");
            }
            if revealed {
                command.arg("--revealed");
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

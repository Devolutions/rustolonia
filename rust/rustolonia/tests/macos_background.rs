#![cfg(target_os = "macos")]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

fn check_launch(executable: &std::path::Path, mode: &str, icon: &std::path::Path) {
    let mut child = Command::new(executable)
        .args([mode, icon.to_str().unwrap()])
        .env_remove(rustolonia::HOST_NATIVE_LIB_ENV_VAR)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("background test timed out: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "mode {mode}, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("checks passed"));
}

#[test]
#[ignore = "requires a macOS desktop session and a matching locally built patched host"]
fn one_agent_bundle_supports_background_promotion_and_foreground_launches() {
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "rustolonia-background-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let result = std::panic::catch_unwind(|| {
        let contents = root.join("BackgroundTest.app/Contents");
        let macos = contents.join("MacOS");
        let frameworks = contents.join("Frameworks");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::create_dir(&frameworks).unwrap();
        let host = rustolonia::discover_host_path().unwrap();
        for entry in std::fs::read_dir(host.parent().unwrap()).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .is_some_and(|extension| extension == "dylib")
            {
                std::fs::copy(&path, frameworks.join(path.file_name().unwrap())).unwrap();
            }
        }
        let executable = macos.join("background-test");
        std::fs::copy(env!("CARGO_BIN_EXE_native_macos_background"), &executable).unwrap();
        std::fs::write(
            contents.join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>background-test</string>
<key>CFBundleIdentifier</key><string>com.rustolonia.background-test</string>
<key>CFBundleName</key><string>Rustolonia Background Test</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSUIElement</key><true/>
</dict></plist>"#,
        )
        .unwrap();
        let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../avalonia-src/tests/Avalonia.RenderTests/Assets/Star512.png");
        for mode in ["background", "promote", "startup-promote", "foreground"] {
            check_launch(&executable, mode, &icon);
        }
    });
    std::fs::remove_dir_all(&root).unwrap();
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

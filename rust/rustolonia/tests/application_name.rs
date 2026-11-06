#![cfg(target_os = "macos")]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn check_launch(executable: &std::path::Path, name: &str, bundled: bool) {
    let mut command = Command::new(executable);
    command.arg(name);
    if bundled {
        command.env_remove(rustolonia::HOST_NATIVE_LIB_ENV_VAR);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("name test timed out: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("menus preserved"));
}

#[test]
#[ignore = "requires a macOS desktop session and a built native host"]
fn application_name_survives_window_switching_in_standalone_and_bundle_launches() {
    let executable = std::path::Path::new(env!("CARGO_BIN_EXE_native_application_name"));
    let name = "Rustolonia Name Test";
    check_launch(executable, name, false);

    let root = std::env::temp_dir().join(format!("rustolonia-name-{}", std::process::id()));
    let contents = root.join("NameTest.app/Contents");
    let macos = contents.join("MacOS");
    std::fs::create_dir_all(&macos).unwrap();
    let frameworks = contents.join("Frameworks");
    std::fs::create_dir_all(&frameworks).unwrap();
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
    let bundled = macos.join("name-test");
    std::fs::copy(executable, &bundled).unwrap();
    std::fs::write(
        contents.join("Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>name-test</string>
<key>CFBundleIdentifier</key><string>com.rustolonia.application-name-test</string>
<key>CFBundleName</key><string>{name}</string>
<key>CFBundleDisplayName</key><string>{name}</string>
<key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>"#
        ),
    )
    .unwrap();
    let result = std::panic::catch_unwind(|| check_launch(&bundled, name, true));
    std::fs::remove_dir_all(root).unwrap();
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

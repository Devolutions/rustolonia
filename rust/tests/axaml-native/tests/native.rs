use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires a matching native host and a desktop display"]
fn generated_axaml_tracks_resources_content_items_and_disposal() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_axaml-native-fixture"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start AXAML native fixture");
    let deadline = Instant::now() + Duration::from_secs(30);
    while child
        .try_wait()
        .expect("check AXAML native fixture")
        .is_none()
    {
        if Instant::now() >= deadline {
            child.kill().expect("stop timed-out AXAML native fixture");
            let output = child.wait_with_output().unwrap();
            panic!(
                "AXAML native fixture timed out:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "AXAML native fixture failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("AXAML native fixture passed"),
        "AXAML native fixture did not report completion"
    );
}

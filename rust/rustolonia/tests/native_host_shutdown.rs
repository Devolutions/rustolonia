#![cfg(target_os = "macos")]

use rustolonia::App;

#[test]
#[ignore = "requires a macOS desktop session and a built native host; process-exit teardown is under test"]
fn shutdown_releases_native_dispatcher_before_process_exit() {
    let app = App::load_from_env().expect("load the native host");
    app.run(|scope| scope.shutdown())
        .expect("shutdown the native host");
}

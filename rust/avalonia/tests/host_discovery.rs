//! Integration tests for `avalonia::discover_host_path`, exercised the way a
//! consumer application would use it: through the crate's public API only.
//!
//! These tests intentionally never call `Host::load`, so they do not require
//! a published `Avalonia.Host` and always run, unlike the `nativeaot_*`
//! suites in this directory.

use avalonia::{discover_host_path, HOST_NATIVE_LIB_ENV_VAR};
use std::ffi::OsString;
use std::sync::Mutex;

// Environment variables are process-wide state, and `cargo test` runs the
// tests in one binary on multiple threads by default, so every test here
// that touches `HOST_NATIVE_LIB_ENV_VAR` must hold this lock for its whole
// duration to avoid racing the other test.
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvVarGuard {
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set(value: &str) -> Self {
        let previous = std::env::var_os(HOST_NATIVE_LIB_ENV_VAR);
        std::env::set_var(HOST_NATIVE_LIB_ENV_VAR, value);
        Self { previous }
    }

    fn unset() -> Self {
        let previous = std::env::var_os(HOST_NATIVE_LIB_ENV_VAR);
        std::env::remove_var(HOST_NATIVE_LIB_ENV_VAR);
        Self { previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(HOST_NATIVE_LIB_ENV_VAR, value),
            None => std::env::remove_var(HOST_NATIVE_LIB_ENV_VAR),
        }
    }
}

#[test]
fn explicit_override_always_wins() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let _guard = EnvVarGuard::set("definitely-not-a-real-host-path");
    let resolved = discover_host_path().expect("the explicit override must always resolve");
    assert_eq!(resolved.as_os_str(), "definitely-not-a-real-host-path");
}

#[test]
fn missing_override_and_missing_adjacent_host_names_both_mechanisms() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let _guard = EnvVarGuard::unset();
    // `cargo test`'s own binary directory legitimately has no Avalonia.Host
    // next to it, so this exercises the real "nothing found" path end to end
    // (through `std::env::current_exe`), not just the pure lookup helper.
    let error = discover_host_path().expect_err("neither mechanism should resolve here");
    let message = error.to_string();
    assert!(
        message.contains(HOST_NATIVE_LIB_ENV_VAR),
        "expected the error to mention the override variable, got: {message}"
    );
    assert!(
        message.contains("Avalonia.Host"),
        "expected the error to name the host file it looked for, got: {message}"
    );
}

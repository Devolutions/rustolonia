//! Entry point for Rustolonia applications running inside an Android APK.
//!
//! On Android the process belongs to the Android runtime, so a Rust
//! application cannot own `main`. Instead the application crate builds a
//! `cdylib` and declares its entry point with [`android_main!`]:
//!
//! ```ignore
//! // src/lib.rs
//! mod ui;
//! rustolonia::android_main!(ui::build);
//! ```
//!
//! The .NET Android host (`host/Android`) starts Avalonia, loads the library
//! and calls the generated `rustolonia_android_main` on the UI thread with
//! its [`HostExports`](crate::HostExports) table, the same in-process table
//! the iOS host passes. `ui::build` is the same function the desktop
//! `main.rs` passes to [`App::run`], so one UI module serves both. See
//! `rust/PLATFORMS.md#android`.

use crate::{App, AppScope, Error, Result};
use rustolonia_sys as sys;

/// Declares `extern "C" fn rustolonia_android_main`, the symbol the Android
/// host calls to start this application with `build` as its
/// [`App::run`] callback.
///
/// `build` has the signature `fn(&AppScope) -> rustolonia::Result<()>`.
#[macro_export]
macro_rules! android_main {
    ($build:expr) => {
        /// Called by the Rustolonia Android host on the UI thread.
        ///
        /// # Safety
        /// `exports` must be the host's export table.
        #[no_mangle]
        pub unsafe extern "C" fn rustolonia_android_main(
            exports: *const $crate::HostExports,
        ) -> i32 {
            $crate::android::start(exports, $build)
        }
    };
}

/// Runs `build` against the host behind `exports`, returning an HRESULT for
/// the host to report. Panics and errors are written to logcat (tag
/// `rustolonia`) on Android and to stderr elsewhere; they never unwind into
/// the host.
///
/// # Safety
/// `exports` must be null or point to the host's [`sys::HostExports`] table,
/// valid for the rest of the process.
#[doc(hidden)]
pub unsafe fn start(
    exports: *const sys::HostExports,
    build: impl FnOnce(&AppScope) -> Result<()> + Send + 'static,
) -> i32 {
    log::install();
    if exports.is_null() {
        eprintln!("rustolonia: the Android host passed no export table");
        return sys::E_POINTER;
    }
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        App::from_in_process(&*exports)?.run(build)
    }));
    match outcome {
        Ok(Ok(())) => sys::S_OK,
        Ok(Err(error)) => {
            eprintln!("rustolonia: the application failed to start: {error}");
            match error {
                Error::Abi(error) => error.0,
                _ => sys::E_FAIL,
            }
        }
        // The panic hook has already reported the payload.
        Err(_) => sys::E_FAIL,
    }
}

#[cfg(target_os = "android")]
mod log {
    //! Android discards a process's stdout and stderr. Route both into
    //! logcat so `println!`, `eprintln!` and panic messages stay visible
    //! (`adb logcat -s rustolonia`).
    use std::ffi::{c_char, c_int, CString};
    use std::io::Read;
    use std::os::fd::FromRawFd;
    use std::sync::Once;

    const ANDROID_LOG_INFO: c_int = 4;
    const ANDROID_LOG_WARN: c_int = 5;

    #[link(name = "log")]
    extern "C" {
        fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    }

    extern "C" {
        fn pipe(fds: *mut c_int) -> c_int;
        fn dup2(old: c_int, new: c_int) -> c_int;
    }

    pub(super) fn install() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            redirect(1, ANDROID_LOG_INFO);
            redirect(2, ANDROID_LOG_WARN);
        });
    }

    fn redirect(target: c_int, priority: c_int) {
        let mut fds = [0 as c_int; 2];
        // SAFETY: plain POSIX calls on descriptors this function owns.
        unsafe {
            if pipe(fds.as_mut_ptr()) != 0 || dup2(fds[1], target) < 0 {
                return;
            }
        }
        // SAFETY: the read end was just created and is owned only by `reader`.
        let reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
        let _ = std::thread::Builder::new()
            .name(format!("rustolonia-log-{target}"))
            .spawn(move || pump(reader, priority));
    }

    fn pump(mut reader: std::fs::File, priority: c_int) {
        let mut pending = Vec::new();
        let mut buffer = [0u8; 1024];
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => count,
            };
            pending.extend_from_slice(&buffer[..count]);
            while let Some(end) = pending.iter().position(|&byte| byte == b'\n') {
                let line: Vec<u8> = pending.drain(..=end).collect();
                write(priority, &line[..line.len() - 1]);
            }
        }
        if !pending.is_empty() {
            write(priority, &pending);
        }
    }

    fn write(priority: c_int, line: &[u8]) {
        let text: Vec<u8> = line.iter().copied().filter(|&byte| byte != 0).collect();
        if let Ok(text) = CString::new(text) {
            // SAFETY: both strings are NUL-terminated.
            unsafe { __android_log_write(priority, c"rustolonia".as_ptr(), text.as_ptr()) };
        }
    }
}

#[cfg(not(target_os = "android"))]
mod log {
    pub(super) fn install() {}
}

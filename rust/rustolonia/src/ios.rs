//! Bootstrap helpers for an iOS application linked with a prebuilt host.

use crate::{App, AppScope, AsControl, Error, HostExports, Result, SingleViewSession};
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};

const E_FAIL: i32 = 0x8000_4005u32 as i32;
const E_POINTER: i32 = 0x8000_4003u32 as i32;

thread_local! {
    static SESSION: RefCell<Option<SingleViewSession>> = const { RefCell::new(None) };
}

/// Starts the Rust UI when the prebuilt iOS host has initialized Avalonia.
///
/// Most applications should use [`ios_application!`](crate::ios_application)
/// rather than calling this function directly.
///
/// # Safety
///
/// `exports` must be null or point to a complete, valid host export table.
/// Its functions and allocator must remain valid until process exit. The host
/// must call this function on its initialized UI thread.
#[doc(hidden)]
pub unsafe fn start<T: AsControl>(
    exports: *const HostExports,
    build: impl FnOnce(&AppScope) -> Result<T>,
) -> i32 {
    if exports.is_null() {
        return E_POINTER;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        SESSION.with(|session| {
            if session.borrow().is_some() {
                return Err(Error::UiContextAlreadyActive);
            }
            let app = unsafe { App::from_in_process(&*exports)? };
            let running = app.start_single_view(build)?;
            *session.borrow_mut() = Some(running);
            Ok(())
        })
    }));
    match result {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            eprintln!("failed to start the iOS application: {error}");
            E_FAIL
        }
        Err(_) => {
            eprintln!("the iOS application panicked during startup");
            E_FAIL
        }
    }
}

/// Stops the Rust UI on the same thread that started it.
#[doc(hidden)]
pub fn stop() -> i32 {
    match catch_unwind(AssertUnwindSafe(|| {
        SESSION.with(|session| match session.borrow_mut().take() {
            Some(running) => running.close(),
            None => Ok(()),
        })
    })) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            eprintln!("failed to stop the iOS application: {error}");
            E_FAIL
        }
        Err(_) => {
            eprintln!("the iOS application panicked during shutdown");
            E_FAIL
        }
    }
}

/// Exports the callbacks expected by Rustolonia's prebuilt iOS host.
///
/// Pass a function that creates a root control from an [`AppScope`]. The
/// application crate must be built as a Rust `staticlib`; the prebuilt host
/// owns UIKit startup and invokes the function on the UI thread.
///
/// ```no_run
/// use rustolonia::{AppScope, Result, TextBlock};
///
/// rustolonia::ios_application!(build_view);
///
/// fn build_view(_: &AppScope) -> Result<TextBlock> {
///     TextBlock::new()?.text("Hello from Rust")
/// }
/// ```
#[macro_export]
macro_rules! ios_application {
    ($build:path $(,)?) => {
        /// # Safety
        ///
        /// The iOS host must pass a valid export table on its UI thread.
        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn rustolonia_ios_main(exports: *const $crate::HostExports) -> i32 {
            unsafe { $crate::ios::start(exports, $build) }
        }

        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub extern "C" fn rustolonia_ios_stop() -> i32 {
            $crate::ios::stop()
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_exports_do_not_call_the_builder() {
        let result = unsafe {
            start::<crate::TextBlock>(std::ptr::null(), |_| {
                panic!("a null export table must not call the builder")
            })
        };
        assert_eq!(result, E_POINTER);
    }

    #[test]
    fn stopping_without_a_session_is_idempotent() {
        assert_eq!(stop(), 0);
        assert_eq!(stop(), 0);
    }
}

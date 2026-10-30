//! The toasts the model asks for (the `toast` signal). Port of `ToastWindow.cs`.
//!
//! STUB: the interface; the implementation is pending.

use rustolonia::{AppScope, Result};

/// Starts following the main view model's signals. Called once, on the UI
/// thread, after the main window is mounted.
pub fn install(_scope: &AppScope) -> Result<()> {
    Ok(())
}

//! The Capture Widget (the `widget` signal) and Grab Text's result window (the `grab_text` signal). Port of `CaptureExtras.cs (WidgetWindow, GrabTextWindow)`.
//!
//! STUB: the interface; the implementation is pending.

use rustolonia::{AppScope, Result};

/// Starts following the main view model's signals. Called once, on the UI
/// thread, after the main window is mounted.
pub fn install(_scope: &AppScope) -> Result<()> {
    Ok(())
}

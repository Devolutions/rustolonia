//! The selection overlay (the `overlay` signal), one window per monitor, and Screen Draw (the `screen_draw` signal). Port of `OverlayWindows.cs`.
//!
//! STUB: the interface; the implementation is pending.

use rustolonia::{AppScope, Result};

/// Starts following the main view model's signals. Called once, on the UI
/// thread, after the main window is mounted.
pub fn install(_scope: &AppScope) -> Result<()> {
    Ok(())
}

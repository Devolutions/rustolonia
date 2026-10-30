//! The capture panels and outlines on screen and the recording toolbar (the `hud` signal). Port of `HudWindows.cs and RecordingToolbar.cs`.
//!
//! STUB: the interface; the implementation is pending.

use rustolonia::{AppScope, Result};

/// Starts following the main view model's signals. Called once, on the UI
/// thread, after the main window is mounted.
pub fn install(_scope: &AppScope) -> Result<()> {
    Ok(())
}

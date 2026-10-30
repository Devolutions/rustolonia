//! The Capture window. Port of `CaptureWindow.axaml` and
//! `CaptureWindow.axaml.cs`.
//!
//! STUB: a placeholder window; the implementation is pending.

use crate::vm::CaptureBinding;
use rustolonia::{AppScope, Result, Window};

/// Builds the Capture window for `binding` (not yet shown).
pub fn build(scope: &AppScope, _binding: &CaptureBinding) -> Result<Window> {
    Window::el()
        .title("Jiffshot Capture")
        .width(720.0)
        .height(520.0)
        .icon(super::icon_file())
        .build(scope)
}

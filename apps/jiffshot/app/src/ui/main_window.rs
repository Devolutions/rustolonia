//! The editor window. Port of `MainWindow.axaml` and `MainWindow.axaml.cs`.
//!
//! STUB: a placeholder window; the implementation is pending.

use crate::vm::MainBinding;
use rustolonia::{AppScope, Result, TextBlock, Window};

/// Builds the editor window for `binding` (not yet shown).
pub fn build(scope: &AppScope, binding: &MainBinding) -> Result<Window> {
    let sink = binding.sink;
    Window::el()
        .title(sink.title)
        .width(1280.0)
        .height(800.0)
        .icon(super::icon_file())
        .content(TextBlock::el().text(sink.status))
        .build(scope)
}

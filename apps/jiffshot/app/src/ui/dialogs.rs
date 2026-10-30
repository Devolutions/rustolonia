//! Modal dialogs: keyboard shortcuts, About, resize image, trim. Port of
//! `Dialogs.cs`.
//!
//! STUB: the interface; the implementation is pending.

use rustolonia::{AppScope, Result, Window};

/// Shows the About dialog over `owner`.
pub fn about(_scope: &AppScope, _owner: &Window) -> Result<()> {
    Ok(())
}

/// Shows the keyboard shortcuts over `owner`.
pub fn keyboard_shortcuts(_scope: &AppScope, _owner: &Window) -> Result<()> {
    Ok(())
}

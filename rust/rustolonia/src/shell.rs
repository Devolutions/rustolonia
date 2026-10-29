//! Desktop shell integration: the separately versioned stage 33
//! `IAvnApplication6` capability.
//!
//! A tray or capture tool needs a few things the rest of the projection does
//! not cover: an application that keeps running with no window open
//! ([`AppContext::set_shutdown_mode`]), windows created hidden and shown later
//! ([`AppScope::mount_dynamic_view_model_hidden`], [`Window::show`]),
//! notification-area icons whose menu comes from the view-model schema
//! ([`AppScope::mount_dynamic_tray`], usually through a generated
//! `mount_<tray>`), and window-level settings: the native handle, topmost,
//! per-pixel transparency and exclusion from screen capture.

use crate::{AppContext, AppScope, Result, Window};
use rustolonia_sys as sys;

/// When the application exits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShutdownMode {
    /// When its last window closes (the default).
    #[default]
    OnLastWindowClose,
    /// When the first window mounted closes.
    OnMainWindowClose,
    /// Only through [`AppScope::shutdown`]: a tray application keeps running
    /// with every window closed or hidden.
    OnExplicitShutdown,
}

impl ShutdownMode {
    fn to_abi(self) -> i32 {
        match self {
            ShutdownMode::OnLastWindowClose => 0,
            ShutdownMode::OnMainWindowClose => 1,
            ShutdownMode::OnExplicitShutdown => 2,
        }
    }
}

impl AppContext {
    fn shell(&self) -> Result<sys::ComPtr<sys::IAvnApplication6>> {
        Ok(self.application().shell()?)
    }

    /// Sets when the application exits. Takes effect at once; call it from the
    /// startup callback, before the first window can close.
    pub fn set_shutdown_mode(&self, mode: ShutdownMode) -> Result<()> {
        Ok(self.shell()?.set_shutdown_mode(mode.to_abi())?)
    }

    /// The window's native handle (an `HWND` on Windows), or 0 while the
    /// platform window does not exist yet (before it is first shown).
    pub fn platform_handle(&self, window: &Window) -> Result<isize> {
        Ok(self.shell()?.platform_handle(&window.raw)? as isize)
    }

    /// Keeps `window` above every window that is not topmost.
    pub fn set_topmost(&self, window: &Window, topmost: bool) -> Result<()> {
        Ok(self.shell()?.set_window_topmost(&window.raw, topmost)?)
    }

    /// Asks for per-pixel transparency, so a transparent background shows
    /// what is behind the window.
    pub fn set_transparent(&self, window: &Window, transparent: bool) -> Result<()> {
        Ok(self
            .shell()?
            .set_window_transparent(&window.raw, transparent)?)
    }

    /// Leaves `window` out of screenshots and screen recordings. Windows only:
    /// an error elsewhere. The window must have been shown once, so that its
    /// native window exists.
    pub fn set_excluded_from_capture(&self, window: &Window, excluded: bool) -> Result<()> {
        Ok(self
            .shell()?
            .set_window_excluded_from_capture(&window.raw, excluded)?)
    }
}

impl AppScope {
    /// Like [`AppScope::mount`], without showing the window: it is kept with
    /// the other mounted windows and shown later with [`Window::show`].
    pub fn mount_hidden(&self, window: Window) -> Result<()> {
        self.record_window(window);
        Ok(())
    }
}

impl Window {
    /// Shows the window (a window mounted hidden, or one hidden since).
    pub fn show(&self) -> Result<()> {
        Ok(self.raw.show()?)
    }
}

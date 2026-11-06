//! Desktop shell integration.
//!
//! A tray or capture tool needs a few things the rest of the projection does
//! not cover: an application that keeps running with no window open
//! ([`AppContext::set_shutdown_mode`]), windows created hidden and shown later
//! ([`AppScope::mount_dynamic_view_model_hidden`], [`Window::show`]),
//! notification-area icons whose menu comes from the view-model schema
//! ([`AppScope::mount_dynamic_tray`], usually through a generated
//! `mount_<tray>`, or [`AppContext::create_tray`] with no schema at all), and
//! window-level settings: the native handle, topmost,
//! per-pixel transparency, platform-typed native handles, AppKit window
//! numbers and mouse passthrough, and explicit show/activate behavior.

use crate::{AppContext, AppScope, Result, Window};
use rustolonia_sys as sys;
use std::marker::PhantomData;
pub use sys::NativePopupEvent;

/// The descriptor of a borrowed native window handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeWindowHandleKind {
    Unknown(i32),
    Win32Hwnd,
    X11Window,
    MacOsNsView,
    MacOsNsWindow,
}

impl NativeWindowHandleKind {
    fn from_abi(value: i32) -> Self {
        match value {
            1 => Self::Win32Hwnd,
            2 => Self::X11Window,
            3 => Self::MacOsNsView,
            4 => Self::MacOsNsWindow,
            other => Self::Unknown(other),
        }
    }
}

/// A borrowed native window handle.
///
/// The raw value is platform-specific and valid only while its native
/// top-level exists. It must not be retained past that native lifetime, used
/// after the associated [`Window`] is dropped, or accessed outside the UI
/// thread. Closing a window may invalidate the handle even while its Rust
/// [`Window`] value still exists; query it again after recreating the native
/// top-level.
#[derive(Debug)]
pub struct NativeWindowHandle<'window> {
    kind: NativeWindowHandleKind,
    raw: isize,
    _window: PhantomData<&'window Window>,
}

impl NativeWindowHandle<'_> {
    pub fn kind(&self) -> NativeWindowHandleKind {
        self.kind
    }

    /// Returns the platform-specific native handle value.
    ///
    /// Do not retain it beyond the native top-level's lifetime or use it
    /// outside the UI thread. Closing the window may invalidate this value
    /// before the Rust `Window` is dropped.
    pub fn raw(&self) -> isize {
        self.raw
    }
}

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
    /// Gates native popups belonging to this owner (including nested popups).
    /// macOS only. Install before opening any popup; the callback runs on the
    /// UI thread before native show and must return promptly. Preparing supplies
    /// a unique token and window number, never a borrowed pointer. Obtain that
    /// hidden SCWindow, update all stream filters asynchronously, then post
    /// `acknowledge_native_popup` to the UI thread ONLY after acknowledgement.
    /// Closed invalidates the token; stale/duplicate acknowledgement returns
    /// E_INVALIDARG. No acknowledgement means the popup remains hidden.
    /// This is incompatible with embedded popup hosting on the same owner.
    pub fn set_native_popup_gate(
        &self,
        window: &Window,
        mut callback: impl FnMut(sys::NativePopupEvent) -> Result<()> + Send + 'static,
    ) -> Result<()> {
        let handler = sys::native_popup_handler(move |event| {
            callback(event).map_err(|error| match error {
                crate::Error::Abi(error) => error,
                _ => sys::Error(sys::E_FAIL),
            })
        });
        Ok(self
            .application()
            .set_native_popup_gate(&window.raw, Some(&handler))?)
    }

    /// UI thread only. Reveal the still-live popup after capture-filter acknowledgement.
    pub fn acknowledge_native_popup(&self, token: i64) -> Result<()> {
        Ok(self.application().acknowledge_native_popup(token)?)
    }

    /// Removes an owner's gate. First close all its popups; pending popups
    /// are never implicitly acknowledged by policy removal.
    pub fn clear_native_popup_gate(&self, window: &Window) -> Result<()> {
        Ok(self
            .application()
            .set_native_popup_gate(&window.raw, None)?)
    }

    /// Keeps popups targeting `window` inside its client-area overlay rather
    /// than allocating native popup windows. Applies to tooltips, flyouts,
    /// context menus and nested popups; other top-levels are unaffected.
    ///
    /// Set before opening any popups, preferably before revealing the window.
    /// Already-open native popups are not migrated. Overlay popups cannot extend
    /// outside the client area, so small toolbars may need a larger host area.
    ///
    /// This does not perform capture exclusion. On macOS, first acknowledge
    /// exclusion of the owning window in every active ScreenCaptureKit stream
    /// before revealing it. This API exposes no additional borrowed handles.
    pub fn set_overlay_popups(&self, window: &Window, enabled: bool) -> Result<()> {
        Ok(self
            .application()
            .set_window_overlay_popups(&window.raw, enabled)?)
    }

    /// Sets when the application exits. Takes effect at once; call it from the
    /// startup callback, before the first window can close.
    pub fn set_shutdown_mode(&self, mode: ShutdownMode) -> Result<()> {
        Ok(self.application().set_shutdown_mode(mode.to_abi())?)
    }

    /// The window's native handle (an `HWND` on Windows), or 0 while the
    /// platform window does not exist yet (before it is first shown).
    pub fn platform_handle(&self, window: &Window) -> Result<isize> {
        Ok(self.application().platform_handle(&window.raw)? as isize)
    }

    /// Returns the typed native handle once the window has been created.
    ///
    /// The handle is borrowed from `window`, is only valid on the UI thread,
    /// and must not outlive it. Use [`NativeWindowHandle::kind`] before
    /// interpreting its raw value.
    pub fn native_window_handle<'window>(
        &self,
        window: &'window Window,
    ) -> Result<Option<NativeWindowHandle<'window>>> {
        let (raw, kind) = self.application().native_window_handle(&window.raw)?;
        let raw = isize::try_from(raw).map_err(|_| sys::Error(sys::E_FAIL))?;
        if raw == 0 {
            return Ok(None);
        }
        Ok(Some(NativeWindowHandle {
            kind: NativeWindowHandleKind::from_abi(kind),
            raw,
            _window: PhantomData,
        }))
    }

    /// Gets AppKit's `NSWindow.windowNumber` for ScreenCaptureKit window
    /// matching. macOS only; the window must have a native window.
    ///
    /// This does not exclude the window from capture by other applications.
    pub fn macos_window_number(&self, window: &Window) -> Result<i32> {
        Ok(self.application().macos_window_number(&window.raw)?)
    }

    /// Enables or disables AppKit mouse-event passthrough for a window.
    /// macOS only; unsupported platforms return `E_NOTIMPL`.
    pub fn set_mouse_passthrough(&self, window: &Window, enabled: bool) -> Result<()> {
        Ok(self
            .application()
            .set_macos_window_ignores_mouse_events(&window.raw, enabled)?)
    }

    /// Shows a hidden window, optionally activating it. If it is already
    /// visible, `activate = true` activates it and `false` leaves its current
    /// activation unchanged.
    pub fn show_window(&self, window: &Window, activate: bool) -> Result<()> {
        Ok(self.application().show_window(&window.raw, activate)?)
    }

    /// Activates a visible window.
    pub fn activate_window(&self, window: &Window) -> Result<()> {
        Ok(self.application().activate_window(&window.raw)?)
    }

    /// Keeps `window` above every window that is not topmost.
    pub fn set_topmost(&self, window: &Window, topmost: bool) -> Result<()> {
        Ok(self
            .application()
            .set_window_topmost(&window.raw, topmost)?)
    }

    /// Asks for per-pixel transparency, so a transparent background shows
    /// what is behind the window.
    pub fn set_transparent(&self, window: &Window, transparent: bool) -> Result<()> {
        Ok(self
            .application()
            .set_window_transparent(&window.raw, transparent)?)
    }

    /// Leaves `window` out of screenshots and screen recordings. Windows only:
    /// an error elsewhere. The window must have been shown once, so that its
    /// native window exists.
    pub fn set_excluded_from_capture(&self, window: &Window, excluded: bool) -> Result<()> {
        Ok(self
            .application()
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

/// One entry of a [`ShellTray`] menu. Ids are the application's own: a click
/// is reported as [`TrayEvent::MenuItem`] with the id, and
/// [`ShellTray::update_item`] finds an entry by it. Ids must be unique and
/// below 2^30.
#[derive(Clone, Debug, PartialEq)]
pub enum TrayMenuItem {
    Command {
        id: u32,
        header: String,
        enabled: bool,
    },
    /// A check box item. Clicking it does not toggle it: the application flips
    /// its state and calls [`ShellTray::update_item`].
    Check {
        id: u32,
        header: String,
        checked: bool,
        enabled: bool,
    },
    Separator,
    Submenu {
        header: String,
        items: Vec<TrayMenuItem>,
    },
}

impl TrayMenuItem {
    pub fn command(id: u32, header: impl Into<String>) -> Self {
        Self::Command {
            id,
            header: header.into(),
            enabled: true,
        }
    }

    pub fn check(id: u32, header: impl Into<String>, checked: bool) -> Self {
        Self::Check {
            id,
            header: header.into(),
            checked,
            enabled: true,
        }
    }

    pub fn submenu(header: impl Into<String>, items: Vec<TrayMenuItem>) -> Self {
        Self::Submenu {
            header: header.into(),
            items,
        }
    }
}

/// What was clicked on a [`ShellTray`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    /// The icon itself.
    Activate,
    /// The menu entry with this id.
    MenuItem(u32),
}

/// A notification-area icon built from Rust (stage 34): unlike a schema
/// tray ([`AppScope::mount_dynamic_tray`]) it needs nothing compiled into the
/// host, so a code-first application on the prebuilt host can have one. It is
/// removed by [`ShellTray::remove`] or when the application shuts down.
#[derive(Clone)]
pub struct ShellTray {
    raw: sys::ComPtr<sys::IAvnShellTray>,
}

const FLAG_CHECKED: i32 = 1;
const FLAG_DISABLED: i32 = 2;
/// Submenus get ids from here up, out of the application's range.
const FIRST_SUBMENU_ID: i32 = 1 << 30;

fn flags(checked: bool, enabled: bool) -> i32 {
    (if checked { FLAG_CHECKED } else { 0 }) | (if enabled { 0 } else { FLAG_DISABLED })
}

fn item_id(id: u32) -> Result<i32> {
    match i32::try_from(id) {
        Ok(id) if id > 0 && id < FIRST_SUBMENU_ID => Ok(id),
        _ => Err(sys::Error(sys::E_INVALIDARG).into()),
    }
}

impl ShellTray {
    /// The image: a file path or an `avares://` URI (`.ico` on Windows).
    pub fn set_icon(&self, source: &str) -> Result<()> {
        Ok(self.raw.set_icon(Some(source))?)
    }

    pub fn set_tool_tip(&self, text: &str) -> Result<()> {
        Ok(self.raw.set_tool_tip(Some(text))?)
    }

    pub fn set_visible(&self, visible: bool) -> Result<()> {
        Ok(self.raw.set_visible(visible)?)
    }

    /// Replaces the whole menu.
    pub fn set_menu(&self, items: &[TrayMenuItem]) -> Result<()> {
        self.raw.clear_menu()?;
        let mut next_submenu = FIRST_SUBMENU_ID;
        self.add_items(0, items, &mut next_submenu)
    }

    fn add_items(&self, parent: i32, items: &[TrayMenuItem], next_submenu: &mut i32) -> Result<()> {
        for item in items {
            match item {
                TrayMenuItem::Command {
                    id,
                    header,
                    enabled,
                } => self.raw.add_menu_item(
                    parent,
                    item_id(*id)?,
                    0,
                    Some(header),
                    flags(false, *enabled),
                )?,
                TrayMenuItem::Check {
                    id,
                    header,
                    checked,
                    enabled,
                } => self.raw.add_menu_item(
                    parent,
                    item_id(*id)?,
                    1,
                    Some(header),
                    flags(*checked, *enabled),
                )?,
                TrayMenuItem::Separator => {
                    let id = *next_submenu;
                    *next_submenu += 1;
                    self.raw.add_menu_item(parent, id, 2, None, 0)?
                }
                TrayMenuItem::Submenu { header, items } => {
                    let id = *next_submenu;
                    *next_submenu += 1;
                    self.raw.add_menu_item(parent, id, 3, Some(header), 0)?;
                    self.add_items(id, items, next_submenu)?;
                }
            }
        }
        Ok(())
    }

    /// Changes an entry's state, and its header unless `None`.
    pub fn update_item(
        &self,
        id: u32,
        header: Option<&str>,
        checked: bool,
        enabled: bool,
    ) -> Result<()> {
        Ok(self
            .raw
            .update_menu_item(item_id(id)?, header, flags(checked, enabled))?)
    }

    /// Takes the icon down for good.
    pub fn remove(&self) -> Result<()> {
        Ok(self.raw.remove()?)
    }
}

impl AppContext {
    /// Shows a tray icon (no image or menu yet) reporting clicks to `handler`,
    /// on the UI thread.
    pub fn create_tray(
        &self,
        mut handler: impl FnMut(TrayEvent) -> Result<()> + Send + 'static,
    ) -> Result<ShellTray> {
        let sys_handler = sys::tray_handler(move |input| {
            let event = match input {
                sys::TrayInput::Activate => TrayEvent::Activate,
                sys::TrayInput::MenuItem(id) => TrayEvent::MenuItem(id as u32),
            };
            handler(event).map_err(|error| match error {
                crate::Error::Abi(error) => error,
                _ => sys::Error(sys::E_FAIL),
            })
        });
        let raw = self.application().create_tray_icon(Some(&sys_handler))?;
        Ok(ShellTray { raw })
    }
}

#[cfg(test)]
mod native_window_handle_tests {
    use super::*;

    #[test]
    fn native_handle_kinds_decode_platform_descriptors() {
        assert_eq!(
            NativeWindowHandleKind::Win32Hwnd,
            NativeWindowHandleKind::from_abi(1)
        );
        assert_eq!(
            NativeWindowHandleKind::X11Window,
            NativeWindowHandleKind::from_abi(2)
        );
        assert_eq!(
            NativeWindowHandleKind::MacOsNsView,
            NativeWindowHandleKind::from_abi(3)
        );
        assert_eq!(
            NativeWindowHandleKind::MacOsNsWindow,
            NativeWindowHandleKind::from_abi(4)
        );
        assert_eq!(
            NativeWindowHandleKind::Unknown(12),
            NativeWindowHandleKind::from_abi(12)
        );
    }
}

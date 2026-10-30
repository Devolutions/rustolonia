//! Desktop shell integration: the separately versioned stage 33
//! `IAvnApplication6` capability.
//!
//! A tray or capture tool needs a few things the rest of the projection does
//! not cover: an application that keeps running with no window open
//! ([`AppContext::set_shutdown_mode`]), windows created hidden and shown later
//! ([`AppScope::mount_dynamic_view_model_hidden`], [`Window::show`]),
//! notification-area icons whose menu comes from the view-model schema
//! ([`AppScope::mount_dynamic_tray`], usually through a generated
//! `mount_<tray>`, or [`AppContext::create_tray`] with no schema at all), and
//! window-level settings: the native handle, topmost,
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
                } => self
                    .raw
                    .add_menu_item(parent, item_id(*id)?, 0, Some(header), flags(false, *enabled))?,
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
    pub fn update_item(&self, id: u32, header: Option<&str>, checked: bool, enabled: bool) -> Result<()> {
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
        let raw = self
            .application()
            .pixel_surfaces()?
            .create_tray_icon(Some(&sys_handler))?;
        Ok(ShellTray { raw })
    }
}

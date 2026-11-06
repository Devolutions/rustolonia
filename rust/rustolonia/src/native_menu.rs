//! Code-first native menus: Avalonia's `NativeMenu` built from Rust.
//!
//! A [`NativeMenu`] is a tree of [`NativeMenuItem`]s numbered by the
//! application. Attached to a window ([`AppContext::set_window_menu`]) it is the
//! macOS menu bar while that window is active; on Windows and Linux a
//! [`AppContext::create_native_menu_bar`] control placed in the window draws
//! the same menu (and hides itself on macOS), so one code path serves every
//! desktop. Attached to the application ([`AppContext::set_application_menu`])
//! its items open the macOS application menu (About, Settings…), which other
//! platforms do not show.
//!
//! Requires a host with the `IAvnApplication15` capability; on an older host
//! [`AppContext::create_native_menu`] fails with `E_NOINTERFACE`.

use crate::{AppContext, Control, Result, Window};
use rustolonia_sys as sys;

/// Generated ids for separators and submenus start here, out of the
/// application's range.
const FIRST_GENERATED_ID: i32 = 1 << 30;

/// The checked, enabled and visible state of a [`NativeMenuItem`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuItemState {
    pub checked: bool,
    pub enabled: bool,
    pub visible: bool,
}

impl Default for MenuItemState {
    fn default() -> Self {
        Self {
            checked: false,
            enabled: true,
            visible: true,
        }
    }
}

impl MenuItemState {
    fn flags(self) -> i32 {
        (if self.checked {
            sys::NATIVE_MENU_FLAG_CHECKED
        } else {
            0
        }) | (if self.enabled {
            0
        } else {
            sys::NATIVE_MENU_FLAG_DISABLED
        }) | (if self.visible {
            0
        } else {
            sys::NATIVE_MENU_FLAG_HIDDEN
        })
    }
}

/// What a [`NativeMenuItem`] is. Ids are the application's own, unique within
/// a menu and in `1..2^30`; a click is reported with the id and
/// [`NativeMenu::update_item`] finds an item by it.
#[derive(Clone, Debug, PartialEq)]
pub enum NativeMenuItemKind {
    Command(u32),
    /// A check box. Clicking it does not toggle it: the application flips its
    /// state with [`NativeMenu::update_item`].
    Check(u32),
    /// One choice of a group. Like a check box, the application moves the
    /// check mark itself.
    Radio(u32),
    Separator,
    Submenu(Vec<NativeMenuItem>),
}

/// One entry of a [`NativeMenu`].
#[derive(Clone, Debug, PartialEq)]
pub struct NativeMenuItem {
    pub kind: NativeMenuItemKind,
    /// The text; `_` marks the access key (`_File`).
    pub header: String,
    /// A key gesture such as `Ctrl+O`, `Cmd+Shift+S` or `F5`. `Primary` is
    /// `Cmd` on Apple platforms and `Ctrl` elsewhere (`Primary+O`).
    pub gesture: Option<String>,
    pub state: MenuItemState,
}

impl NativeMenuItem {
    fn new(kind: NativeMenuItemKind, header: impl Into<String>) -> Self {
        Self {
            kind,
            header: header.into(),
            gesture: None,
            state: MenuItemState::default(),
        }
    }

    pub fn command(id: u32, header: impl Into<String>) -> Self {
        Self::new(NativeMenuItemKind::Command(id), header)
    }

    pub fn check(id: u32, header: impl Into<String>, checked: bool) -> Self {
        Self::new(NativeMenuItemKind::Check(id), header).checked(checked)
    }

    pub fn radio(id: u32, header: impl Into<String>, checked: bool) -> Self {
        Self::new(NativeMenuItemKind::Radio(id), header).checked(checked)
    }

    pub fn separator() -> Self {
        Self::new(NativeMenuItemKind::Separator, String::new())
    }

    pub fn submenu(header: impl Into<String>, items: Vec<NativeMenuItem>) -> Self {
        Self::new(NativeMenuItemKind::Submenu(items), header)
    }

    /// The keyboard shortcut, shown in the menu and handled in every window
    /// the menu is attached to.
    pub fn gesture(mut self, gesture: impl Into<String>) -> Self {
        self.gesture = Some(gesture.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.state.checked = checked;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state.enabled = enabled;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.state.visible = visible;
        self
    }
}

/// Resolves the `Primary` modifier for the platform this code runs on.
fn platform_gesture(gesture: &str) -> String {
    let primary = if cfg!(any(target_os = "macos", target_os = "ios")) {
        "Cmd"
    } else {
        "Ctrl"
    };
    gesture
        .split('+')
        .map(|part| {
            if part.trim().eq_ignore_ascii_case("primary") {
                primary
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn item_id(id: u32) -> Result<i32> {
    match i32::try_from(id) {
        Ok(id) if id > 0 && id < FIRST_GENERATED_ID => Ok(id),
        _ => Err(sys::Error(sys::E_INVALIDARG).into()),
    }
}

/// One item as the host's `AddMenuItem` receives it.
#[derive(Debug, PartialEq)]
struct FlatItem {
    parent: i32,
    id: i32,
    kind: i32,
    header: Option<String>,
    gesture: Option<String>,
    flags: i32,
}

fn flatten(
    parent: i32,
    items: &[NativeMenuItem],
    next_generated: &mut i32,
    out: &mut Vec<FlatItem>,
) -> Result<()> {
    for item in items {
        let mut generated = || {
            let id = *next_generated;
            *next_generated += 1;
            id
        };
        let (id, kind) = match &item.kind {
            NativeMenuItemKind::Command(id) => (item_id(*id)?, sys::NATIVE_MENU_KIND_COMMAND),
            NativeMenuItemKind::Check(id) => (item_id(*id)?, sys::NATIVE_MENU_KIND_CHECK_BOX),
            NativeMenuItemKind::Radio(id) => (item_id(*id)?, sys::NATIVE_MENU_KIND_RADIO),
            NativeMenuItemKind::Separator => (generated(), sys::NATIVE_MENU_KIND_SEPARATOR),
            NativeMenuItemKind::Submenu(_) => (generated(), sys::NATIVE_MENU_KIND_SUBMENU),
        };
        let header = match item.kind {
            NativeMenuItemKind::Separator => None,
            _ => Some(item.header.clone()),
        };
        out.push(FlatItem {
            parent,
            id,
            kind,
            header,
            gesture: item.gesture.as_deref().map(platform_gesture),
            flags: item.state.flags(),
        });
        if let NativeMenuItemKind::Submenu(children) = &item.kind {
            flatten(id, children, next_generated, out)?;
        }
    }
    Ok(())
}

/// A menu built from Rust, attachable to windows or to the application. Clones
/// share the same menu. UI thread only.
#[derive(Clone)]
pub struct NativeMenu {
    raw: sys::ComPtr<sys::IAvnNativeMenu>,
}

impl NativeMenu {
    /// Replaces every item.
    pub fn set_items(&self, items: &[NativeMenuItem]) -> Result<()> {
        let mut flat = Vec::new();
        let mut next_generated = FIRST_GENERATED_ID;
        flatten(0, items, &mut next_generated, &mut flat)?;
        self.raw.clear_menu()?;
        for item in flat {
            self.raw.add_menu_item(
                item.parent,
                item.id,
                item.kind,
                item.header.as_deref(),
                item.gesture.as_deref(),
                item.flags,
            )?;
        }
        Ok(())
    }

    /// Changes an item's state, and its header unless `None`.
    pub fn update_item(&self, id: u32, header: Option<&str>, state: MenuItemState) -> Result<()> {
        Ok(self
            .raw
            .update_menu_item(item_id(id)?, header, state.flags())?)
    }
}

impl AppContext {
    fn native_menus(&self) -> Result<sys::ComPtr<sys::IAvnApplication15>> {
        Ok(self.application().native_menus()?)
    }

    /// Creates an empty native menu reporting the id of each clicked item (or
    /// pressed shortcut) to `handler`, on the UI thread.
    pub fn create_native_menu(
        &self,
        mut handler: impl FnMut(u32) -> Result<()> + Send + 'static,
    ) -> Result<NativeMenu> {
        let sys_handler = sys::native_menu_handler(move |id| {
            handler(id as u32).map_err(|error| match error {
                crate::Error::Abi(error) => error,
                _ => sys::Error(sys::E_FAIL),
            })
        });
        let raw = self
            .native_menus()?
            .create_native_menu(Some(&sys_handler))?;
        Ok(NativeMenu { raw })
    }

    /// Puts `menu`'s items at the top of the macOS application menu, in place
    /// of the default "About Avalonia" item; `None` takes them out again. A
    /// menu cannot be both the application menu and a window menu.
    pub fn set_application_menu(&self, menu: Option<&NativeMenu>) -> Result<()> {
        Ok(self
            .native_menus()?
            .set_application_menu(menu.map(|m| &m.raw))?)
    }

    /// Makes `menu` the window's menu bar and installs its shortcuts on the
    /// window; `None` removes both. One menu can serve several windows.
    pub fn set_window_menu(&self, window: &Window, menu: Option<&NativeMenu>) -> Result<()> {
        Ok(self
            .native_menus()?
            .set_window_menu(&window.raw, menu.map(|m| &m.raw))?)
    }

    /// Whether the platform shows `window`'s menu outside the window (the
    /// macOS menu bar), so no in-window bar is visible.
    pub fn is_native_menu_exported(&self, window: &Window) -> Result<bool> {
        Ok(self.native_menus()?.is_native_menu_exported(&window.raw)?)
    }

    /// A control drawing its window's menu in-window (Avalonia's
    /// `NativeMenuBar`). It hides itself while the platform exports the menu,
    /// so put one at the top of every window that has a menu.
    pub fn create_native_menu_bar(&self) -> Result<Control> {
        let raw = self.native_menus()?.create_native_menu_bar()?;
        Ok(Control { raw })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(items: &[NativeMenuItem]) -> Vec<FlatItem> {
        let mut out = Vec::new();
        let mut next_generated = FIRST_GENERATED_ID;
        flatten(0, items, &mut next_generated, &mut out).unwrap();
        out
    }

    #[test]
    fn items_flatten_parent_first_with_generated_ids() {
        let items = flat(&[
            NativeMenuItem::submenu(
                "_File",
                vec![
                    NativeMenuItem::command(1, "_Open").gesture("Ctrl+O"),
                    NativeMenuItem::separator(),
                    NativeMenuItem::check(2, "Wrap", true).enabled(false),
                ],
            ),
            NativeMenuItem::radio(3, "Dark", false).visible(false),
        ]);
        let g = FIRST_GENERATED_ID;
        assert_eq!(
            vec![
                (0, g, sys::NATIVE_MENU_KIND_SUBMENU, Some("_File"), None, 0),
                (
                    g,
                    1,
                    sys::NATIVE_MENU_KIND_COMMAND,
                    Some("_Open"),
                    Some("Ctrl+O"),
                    0
                ),
                (g, g + 1, sys::NATIVE_MENU_KIND_SEPARATOR, None, None, 0),
                (
                    g,
                    2,
                    sys::NATIVE_MENU_KIND_CHECK_BOX,
                    Some("Wrap"),
                    None,
                    sys::NATIVE_MENU_FLAG_CHECKED | sys::NATIVE_MENU_FLAG_DISABLED
                ),
                (
                    0,
                    3,
                    sys::NATIVE_MENU_KIND_RADIO,
                    Some("Dark"),
                    None,
                    sys::NATIVE_MENU_FLAG_HIDDEN
                ),
            ],
            items
                .iter()
                .map(|i| (
                    i.parent,
                    i.id,
                    i.kind,
                    i.header.as_deref(),
                    i.gesture.as_deref(),
                    i.flags
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn ids_outside_the_application_range_are_rejected() {
        for id in [0, 1 << 30, u32::MAX] {
            let mut out = Vec::new();
            let mut next_generated = FIRST_GENERATED_ID;
            let items = [NativeMenuItem::command(id, "Bad")];
            assert!(flatten(0, &items, &mut next_generated, &mut out).is_err());
        }
    }

    #[test]
    fn primary_modifier_follows_the_platform() {
        let primary = if cfg!(any(target_os = "macos", target_os = "ios")) {
            "Cmd"
        } else {
            "Ctrl"
        };
        assert_eq!(
            format!("{primary}+Shift+S"),
            platform_gesture("Primary+Shift+S")
        );
        assert_eq!(
            format!("Alt+{primary}+P"),
            platform_gesture("Alt+primary+P")
        );
        assert_eq!("Ctrl+O", platform_gesture("Ctrl+O"));
        assert_eq!("F5", platform_gesture("F5"));
    }
}

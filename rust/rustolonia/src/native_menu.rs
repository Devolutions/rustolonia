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

use crate::reactive::{Effect, Signal, TwoWay, UiCallbackHandle};
use crate::runtime::{report_ui_error, IntoEventResult};
use crate::style::Shortcut;
use crate::view::{Command, El, IntoProp, PropBinding};
use crate::{AppContext, AppScope, Control, Result, Variant, Window};
use rustolonia_sys as sys;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

    /// Sets the keyboard shortcut from a typed [`Shortcut`].
    pub fn shortcut(self, shortcut: Shortcut) -> Self {
        self.gesture(shortcut.to_gesture_string())
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
    /// Creates an empty native menu reporting the id of each clicked item (or
    /// pressed shortcut) to `handler`, on the UI thread.
    pub fn create_native_menu<R: IntoEventResult>(
        &self,
        mut handler: impl FnMut(u32) -> R + 'static,
    ) -> Result<NativeMenu> {
        let cb = UiCallbackHandle::new(move |id: u32| handler(id).into_event_result());
        let sys_handler = sys::native_menu_handler(move |id| match cb.with(|f| f(id as u32)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(crate::Error::Abi(error))) => Err(error),
            Ok(Err(other)) => {
                report_ui_error("native menu callback failed", &other);
                Err(sys::Error(sys::E_FAIL))
            }
            Err(_) => Err(sys::Error(sys::E_FAIL)),
        });
        let raw = self.application().create_native_menu(Some(&sys_handler))?;
        Ok(NativeMenu { raw })
    }

    /// Puts `menu`'s items at the top of the macOS application menu, in place
    /// of the default "About Avalonia" item; `None` takes them out again. A
    /// menu cannot be both the application menu and a window menu.
    pub fn set_application_menu(&self, menu: Option<&NativeMenu>) -> Result<()> {
        Ok(self
            .application()
            .set_application_menu(menu.map(|m| &m.raw))?)
    }

    /// Makes `menu` the window's menu bar and installs its shortcuts on the
    /// window; `None` removes both. One menu can serve several windows.
    pub fn set_window_menu(&self, window: &Window, menu: Option<&NativeMenu>) -> Result<()> {
        Ok(self
            .application()
            .set_window_menu(&window.raw, menu.map(|m| &m.raw))?)
    }

    /// Whether the platform shows `window`'s menu outside the window (the
    /// macOS menu bar), so no in-window bar is visible.
    pub fn is_native_menu_exported(&self, window: &Window) -> Result<bool> {
        Ok(self.application().is_native_menu_exported(&window.raw)?)
    }

    /// A control drawing its window's menu in-window (Avalonia's
    /// `NativeMenuBar`). It hides itself while the platform exports the menu,
    /// so put one at the top of every window that has a menu.
    pub fn create_native_menu_bar(&self) -> Result<Control> {
        let raw = self.application().create_native_menu_bar()?;
        Ok(Control { raw })
    }
}

/// Declarative, ID-free reactive builder for cross-platform [`NativeMenu`] bars
/// and application menus.
pub struct MenuBar {
    entries: Vec<ReactiveEntry>,
    next_id: u32,
}

type ClickAction = Box<dyn FnMut() -> Result<()>>;
type ReactiveAttach = Box<dyn FnOnce(&AppScope, &NativeMenu, u32) -> Result<()>>;

enum ReactiveEntry {
    Item {
        item: NativeMenuItem,
        id: u32,
        on_click: ClickAction,
        bind: Option<ReactiveAttach>,
    },
    Separator,
    Submenu {
        header: String,
        children: Vec<ReactiveEntry>,
    },
}

/// Scoped submenu builder passed to [`MenuBar::submenu`].
pub struct MenuBuilder<'a> {
    next_id: &'a mut u32,
    entries: Vec<ReactiveEntry>,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuBar {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    /// Appends a top-level submenu (such as `"_File"`, `"_Edit"`, `"_View"`).
    pub fn submenu(
        mut self,
        header: impl Into<String>,
        build: impl FnOnce(&mut MenuBuilder<'_>),
    ) -> Self {
        let mut sub = MenuBuilder {
            next_id: &mut self.next_id,
            entries: Vec::new(),
        };
        build(&mut sub);
        self.entries.push(ReactiveEntry::Submenu {
            header: header.into(),
            children: sub.entries,
        });
        self
    }

    /// Appends a top-level command item (commonly used for macOS application menus).
    pub fn action<R: IntoEventResult + 'static>(
        mut self,
        header: impl Into<String>,
        shortcut: Option<Shortcut>,
        on_click: impl FnMut() -> R + 'static,
    ) -> Self {
        let mut sub = MenuBuilder {
            next_id: &mut self.next_id,
            entries: Vec::new(),
        };
        sub.action(header, shortcut, on_click);
        self.entries.extend(sub.entries);
        self
    }

    /// Materializes this declarative specification into a live [`NativeMenu`],
    /// wiring all click callbacks and reactive `checked`/`enabled` bindings.
    pub fn build(self, scope: &AppScope) -> Result<NativeMenu> {
        let actions: Rc<RefCell<HashMap<u32, ClickAction>>> = Rc::new(RefCell::new(HashMap::new()));
        let mut binders: Vec<(u32, ReactiveAttach)> = Vec::new();
        let items = collect_entries(self.entries, &mut *actions.borrow_mut(), &mut binders);

        let actions_for_click = actions.clone();
        let menu = scope.create_native_menu(move |id| {
            let mut map = actions_for_click.borrow_mut();
            if let Some(action) = map.get_mut(&id) {
                action()?;
            }
            Ok(())
        })?;
        menu.set_items(&items)?;

        for (id, bind) in binders {
            bind(scope, &menu, id)?;
        }
        Ok(menu)
    }
}

impl MenuBuilder<'_> {
    fn alloc_id(&mut self) -> u32 {
        let id = *self.next_id;
        *self.next_id += 1;
        id
    }

    /// Adds a menu item invoking `on_click`.
    pub fn action<R: IntoEventResult + 'static>(
        &mut self,
        header: impl Into<String>,
        shortcut: Option<Shortcut>,
        mut on_click: impl FnMut() -> R + 'static,
    ) -> &mut Self {
        let id = self.alloc_id();
        let mut item = NativeMenuItem::command(id, header);
        if let Some(sc) = shortcut {
            item = item.shortcut(sc);
        }
        self.entries.push(ReactiveEntry::Item {
            item,
            id,
            on_click: Box::new(move || on_click().into_event_result()),
            bind: None,
        });
        self
    }

    /// Adds a menu item bound to a [`Command`] (automatically syncing its
    /// `CanExecute` enabled state!).
    pub fn command(
        &mut self,
        header: impl Into<String>,
        shortcut: Option<Shortcut>,
        command: Command,
    ) -> &mut Self {
        let id = self.alloc_id();
        let mut item = NativeMenuItem::command(id, header).enabled(command.is_enabled());
        if let Some(sc) = shortcut {
            item = item.shortcut(sc);
        }
        let cmd_for_click = command.clone();
        let cmd_for_bind = command;
        let bind: ReactiveAttach = Box::new(move |scope, menu, item_id| {
            let menu = menu.clone();
            let raw_cmd = cmd_for_bind.as_com_ptr().clone();
            let raw_for_query = raw_cmd.clone();
            let handler = sys::command_can_execute_changed_handler(move || {
                if let Ok(enabled) = Command::can_execute_interface(&raw_for_query, Variant::None) {
                    let _ = menu.update_item(
                        item_id,
                        None,
                        MenuItemState {
                            checked: false,
                            enabled,
                            visible: true,
                        },
                    );
                }
                Ok(())
            });
            let sub_id = raw_cmd.advise_can_execute_changed(&handler)?;
            scope.retain_subscription(crate::EventSubscription::new(move || {
                raw_cmd.unadvise_can_execute_changed(sub_id)
            }));
            Ok(())
        });
        self.entries.push(ReactiveEntry::Item {
            item,
            id,
            on_click: Box::new(move || cmd_for_click.execute(Variant::None)),
            bind: Some(bind),
        });
        self
    }

    /// Adds a checkable menu item two-way bound to `checked`.
    pub fn check(
        &mut self,
        header: impl Into<String>,
        shortcut: Option<Shortcut>,
        checked: TwoWay<bool>,
    ) -> &mut Self {
        let id = self.alloc_id();
        let sig = checked.signal();
        let mut item = NativeMenuItem::check(id, header, sig.get_untracked());
        if let Some(sc) = shortcut {
            item = item.shortcut(sc);
        }
        let bind: ReactiveAttach = Box::new(move |_, menu, item_id| {
            let menu = menu.clone();
            Effect::new(move || {
                menu.update_item(
                    item_id,
                    None,
                    MenuItemState {
                        checked: sig.get(),
                        enabled: true,
                        visible: true,
                    },
                )
            })
            .map(|_| ())
        });
        self.entries.push(ReactiveEntry::Item {
            item,
            id,
            on_click: Box::new(move || {
                sig.update(|v| *v = !*v);
                Ok(())
            }),
            bind: Some(bind),
        });
        self
    }

    /// Adds a radio menu item bound to `selected == value`.
    pub fn radio<T: Copy + PartialEq + 'static>(
        &mut self,
        header: impl Into<String>,
        shortcut: Option<Shortcut>,
        selected: Signal<T>,
        value: T,
    ) -> &mut Self {
        let id = self.alloc_id();
        let mut item = NativeMenuItem::radio(id, header, selected.get_untracked() == value);
        if let Some(sc) = shortcut {
            item = item.shortcut(sc);
        }
        let bind: ReactiveAttach = Box::new(move |_, menu, item_id| {
            let menu = menu.clone();
            Effect::new(move || {
                menu.update_item(
                    item_id,
                    None,
                    MenuItemState {
                        checked: selected.get() == value,
                        enabled: true,
                        visible: true,
                    },
                )
            })
            .map(|_| ())
        });
        self.entries.push(ReactiveEntry::Item {
            item,
            id,
            on_click: Box::new(move || {
                selected.set_if_changed(value);
                Ok(())
            }),
            bind: Some(bind),
        });
        self
    }

    /// Adds a menu item whose enabled state follows a reactive `IntoProp<bool>`.
    pub fn action_enabled<M, R: IntoEventResult + 'static>(
        &mut self,
        header: impl Into<String>,
        shortcut: Option<Shortcut>,
        enabled: impl IntoProp<bool, M> + 'static,
        mut on_click: impl FnMut() -> R + 'static,
    ) -> &mut Self {
        let id = self.alloc_id();
        let mut item = NativeMenuItem::command(id, header);
        if let Some(sc) = shortcut {
            item = item.shortcut(sc);
        }
        let prop = enabled.into_prop();
        let bind: ReactiveAttach = Box::new(move |_, menu, item_id| {
            let menu = menu.clone();
            match prop {
                PropBinding::Static(en) => menu.update_item(
                    item_id,
                    None,
                    MenuItemState {
                        checked: false,
                        enabled: en,
                        visible: true,
                    },
                ),
                PropBinding::Reactive(mut compute) => Effect::new(move || {
                    menu.update_item(
                        item_id,
                        None,
                        MenuItemState {
                            checked: false,
                            enabled: compute(),
                            visible: true,
                        },
                    )
                })
                .map(|_| ()),
                PropBinding::Fallible(mut compute) => Effect::new(move || {
                    menu.update_item(
                        item_id,
                        None,
                        MenuItemState {
                            checked: false,
                            enabled: compute()?,
                            visible: true,
                        },
                    )
                })
                .map(|_| ()),
                _ => Ok(()),
            }
        });
        self.entries.push(ReactiveEntry::Item {
            item,
            id,
            on_click: Box::new(move || on_click().into_event_result()),
            bind: Some(bind),
        });
        self
    }

    pub fn separator(&mut self) -> &mut Self {
        self.entries.push(ReactiveEntry::Separator);
        self
    }

    pub fn submenu(
        &mut self,
        header: impl Into<String>,
        build: impl FnOnce(&mut MenuBuilder<'_>),
    ) -> &mut Self {
        let mut sub = MenuBuilder {
            next_id: self.next_id,
            entries: Vec::new(),
        };
        build(&mut sub);
        self.entries.push(ReactiveEntry::Submenu {
            header: header.into(),
            children: sub.entries,
        });
        self
    }
}

fn collect_entries(
    entries: Vec<ReactiveEntry>,
    actions: &mut HashMap<u32, ClickAction>,
    binders: &mut Vec<(u32, ReactiveAttach)>,
) -> Vec<NativeMenuItem> {
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        match entry {
            ReactiveEntry::Item {
                item,
                id,
                on_click,
                bind,
            } => {
                actions.insert(id, on_click);
                if let Some(b) = bind {
                    binders.push((id, b));
                }
                out.push(item);
            }
            ReactiveEntry::Separator => out.push(NativeMenuItem::separator()),
            ReactiveEntry::Submenu { header, children } => {
                let child_items = collect_entries(children, actions, binders);
                out.push(NativeMenuItem::submenu(header, child_items));
            }
        }
    }
    out
}

impl AppScope {
    /// Returns a declarative [`El<Control>`] rendering the enclosing window's
    /// native menu bar on Windows/Linux (auto-hiding on macOS).
    pub fn native_menu_bar_el(&self) -> El<Control> {
        let ctx = self.clone_context();
        El::new(move || ctx.create_native_menu_bar())
    }
}

impl El<Window> {
    /// Builds and attaches a declarative [`MenuBar`] onto this window (including
    /// its keyboard shortcuts).
    pub fn menu_bar(self, menu_bar: MenuBar) -> Self {
        self.with_op(move |scope, window| {
            let menu = menu_bar.build(scope)?;
            scope.set_window_menu(window, Some(&menu))?;
            scope.retain_object(menu.raw);
            Ok(())
        })
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

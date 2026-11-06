//! Native menus from code: a window menu bar (the macOS menu bar, or an
//! in-window bar on Windows and Linux), items in the macOS application menu,
//! shortcuts, check boxes and radio groups.
//!
//! Run with a packaged host next to the executable, or point
//! `RUSTOLONIA_HOST_LIB` at a published `Avalonia.Host` native library.

use std::sync::{Arc, Mutex};

use rustolonia::{
    App, AppScope, DockPanel, MenuItemState, NativeMenu, NativeMenuItem, Result, TextBlock,
    Thickness, Window,
};

const ABOUT: u32 = 1;
const NEW: u32 = 10;
const OPEN: u32 = 11;
const QUIT: u32 = 12;
const WORD_WRAP: u32 = 20;
const LIGHT: u32 = 21;
const DARK: u32 = 22;

fn main() -> Result<()> {
    App::load_from_env()?.run(build_ui)
}

#[derive(Default)]
struct State {
    word_wrap: bool,
    dark: bool,
}

fn view_items(state: &State) -> Vec<NativeMenuItem> {
    vec![
        NativeMenuItem::check(WORD_WRAP, "_Word wrap", state.word_wrap).gesture("Primary+Shift+W"),
        NativeMenuItem::separator(),
        NativeMenuItem::radio(LIGHT, "_Light", !state.dark),
        NativeMenuItem::radio(DARK, "_Dark", state.dark),
    ]
}

fn build_ui(scope: &AppScope) -> Result<()> {
    let status = TextBlock::new()?
        .text("Pick something from the menu")?
        .margin(Thickness::uniform(16.0))?;
    let state = Arc::new(Mutex::new(State::default()));

    // The window menu. Its handler needs the menu itself to move check marks,
    // so the menu is shared through a slot filled right after creation.
    let menu_slot: Arc<Mutex<Option<NativeMenu>>> = Arc::default();
    let menu = scope.create_native_menu({
        let context = (**scope).clone();
        let status = status.clone();
        let state = state.clone();
        let menu_slot = menu_slot.clone();
        move |id| {
            let mut state = state.lock().unwrap();
            let menu = menu_slot.lock().unwrap().clone();
            match id {
                NEW => status.set_text("New")?,
                OPEN => status.set_text("Open…")?,
                QUIT => context.shutdown()?,
                WORD_WRAP => state.word_wrap = !state.word_wrap,
                LIGHT => state.dark = false,
                DARK => state.dark = true,
                _ => {}
            }
            if let (Some(menu), WORD_WRAP | LIGHT | DARK) = (menu, id) {
                let checked = |checked| MenuItemState {
                    checked,
                    ..MenuItemState::default()
                };
                menu.update_item(WORD_WRAP, None, checked(state.word_wrap))?;
                menu.update_item(LIGHT, None, checked(!state.dark))?;
                menu.update_item(DARK, None, checked(state.dark))?;
                status.set_text(format!(
                    "Word wrap: {}, theme: {}",
                    state.word_wrap,
                    if state.dark { "dark" } else { "light" }
                ))?;
            }
            Ok(())
        }
    })?;
    *menu_slot.lock().unwrap() = Some(menu.clone());
    menu.set_items(&[
        NativeMenuItem::submenu(
            "_File",
            vec![
                NativeMenuItem::command(NEW, "_New").gesture("Primary+N"),
                NativeMenuItem::command(OPEN, "_Open…").gesture("Primary+O"),
                NativeMenuItem::separator(),
                NativeMenuItem::command(QUIT, "_Quit").gesture("Primary+Q"),
            ],
        ),
        NativeMenuItem::submenu("_View", view_items(&State::default())),
    ])?;

    // macOS application menu: "About" replaces the default "About Avalonia".
    let app_menu = scope.create_native_menu({
        let status = status.clone();
        move |id| {
            if id == ABOUT {
                status.set_text("Native menu sample")?;
            }
            Ok(())
        }
    })?;
    app_menu.set_items(&[
        NativeMenuItem::command(ABOUT, "About Native Menu"),
        NativeMenuItem::separator(),
    ])?;
    scope.set_application_menu(Some(&app_menu))?;

    // The in-window bar shows the menu where the platform has no menu bar and
    // hides itself on macOS.
    let bar = scope.create_native_menu_bar()?;
    bar.set_dock_panel_dock(rustolonia::Dock::Top)?;
    let root = DockPanel::new()?;
    root.children()?.add(bar)?;
    root.children()?.add(status)?;

    let window = Window::new()?
        .title("Native menu")?
        .width(480.0)?
        .height(240.0)?;
    window.set_content(Some(&root))?;
    scope.set_window_menu(&window, Some(&menu))?;
    scope.mount(window)
}

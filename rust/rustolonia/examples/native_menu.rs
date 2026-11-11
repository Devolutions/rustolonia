//! Native menus from Rust: a reactive `MenuBar` (the macOS menu bar, or an
//! in-window bar on Windows and Linux), items in the macOS application menu,
//! typed `Shortcut`s, reactive checkboxes, and radio groups.
//!
//! Run with a packaged host next to the executable, or point
//! `RUSTOLONIA_HOST_LIB` at a published `Avalonia.Host` native library.

use rustolonia::{
    rsx, App, AppScope, DockPanel, Key, MenuBar, NativeMenuItem, Result, Shortcut, TextBlock,
    Window,
};

const ABOUT: u32 = 1;

fn main() -> Result<()> {
    App::load_from_env()?.run(build_ui)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Theme {
    Light,
    Dark,
}

fn build_ui(scope: &AppScope) -> Result<()> {
    let status = scope.signal(String::from("Pick something from the menu"));
    let word_wrap = scope.signal(false);
    let theme = scope.signal(Theme::Light);

    let quit_scope = scope.clone();
    let menu_bar = MenuBar::new()
        .submenu("_File", |file| {
            file.action("_New", Some(Shortcut::primary(Key::N)), move || {
                status.set(String::from("New"));
            });
            file.action("_Open…", Some(Shortcut::primary(Key::O)), move || {
                status.set(String::from("Open…"));
            });
            file.separator();
            file.action("_Quit", Some(Shortcut::primary(Key::Q)), move || {
                quit_scope.shutdown()
            });
        })
        .submenu("_View", |view| {
            view.check(
                "_Word wrap",
                Some(Shortcut::primary_shift(Key::W)),
                word_wrap.two_way(),
            );
            view.separator();
            view.radio("_Light", None, theme, Theme::Light);
            view.radio("_Dark", None, theme, Theme::Dark);
        });

    let readout = scope.memo(move || {
        let wrap = word_wrap.get();
        let mode = match theme.get() {
            Theme::Light => "light",
            Theme::Dark => "dark",
        };
        format!("{} (word wrap: {wrap}, theme: {mode})", status.get())
    });

    // macOS application menu: "About" replaces the default "About Avalonia".
    let app_menu = scope.create_native_menu(move |id| {
        if id == ABOUT {
            status.set(String::from("Native menu sample"));
        }
        Ok(())
    })?;
    app_menu.set_items(&[
        NativeMenuItem::command(ABOUT, "About Native Menu"),
        NativeMenuItem::separator(),
    ])?;
    scope.set_application_menu(Some(&app_menu))?;

    let bar = scope.native_menu_bar_el();
    scope.mount_view(
        rsx! {
            Window {
                title: "Native menu",
                width: 480.0,
                height: 240.0,
                DockPanel {
                    { bar }
                    TextBlock {
                        margin: 16.0,
                        text: readout,
                    }
                }
            }
        }
        .menu_bar(menu_bar),
    )?;
    Ok(())
}

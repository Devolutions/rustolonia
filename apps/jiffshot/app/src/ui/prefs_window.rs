//! Jiffshot's one Preferences dialog: Capture, Editor, Library, Share and
//! Hotkeys.
//!
//! The model asks with the `prefs` signal ("serial\x1fJSON", from
//! `crate::prefs::dialog`): the tabs and their rows, in the Capture window's
//! row kinds (number, check, choice, text, folder, note). OK sends one value
//! per row and tab as `prefs-apply:{"capture":[…],…}`. Each request builds a
//! new dialog, replacing the one showing.

use super::capture_view::parts::{self, post, Form, Input, Subscriptions};
use rustolonia::{
    AnyEl, AppScope, Button, Dock, DockPanel, Effect, HorizontalAlignment, NodeRef, Orientation, Result, ScrollViewer, StackPanel, TabControl, TabItem,
    Window, WindowStartupLocation,
};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::cell::RefCell;

/// The dialog's JSON.
#[derive(Deserialize)]
struct Dialog {
    /// The tab to open on.
    #[serde(default)]
    tab: String,
    #[serde(default)]
    tabs: Vec<Tab>,
}

#[derive(Deserialize)]
struct Tab {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    rows: Vec<parts::Row>,
}

/// The dialog showing, if any. UI thread only.
struct Open {
    window: Window,
    /// Its rows' and buttons' subscriptions, ended when it is replaced.
    subs: Subscriptions,
}

struct State {
    scope: AppScope,
    open: Option<Open>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}

/// Starts following the main view model's signals. Called once, on the UI
/// thread, after the main window is mounted.
pub fn install(scope: &AppScope) -> Result<()> {
    let prefs = super::main().sink.prefs;
    STATE.with(|s| *s.borrow_mut() = Some(State { scope: scope.clone(), open: None }));
    let scope = scope.clone();
    Effect::new(move || {
        if let Some(json) = parts::payload(&prefs.get()).map(str::to_owned) {
            post(&scope, "Preferences", move || show(&json));
        }
        Ok(())
    })?;
    Ok(())
}

/// Opens the dialog from its JSON, replacing the one showing.
fn show(json: &str) -> Result<()> {
    let Ok(dialog) = serde_json::from_str::<Dialog>(json) else { return Ok(()) };
    let Some((scope, previous)) = with(|s| (s.scope.clone(), s.open.take())) else { return Ok(()) };
    if let Some(previous) = previous {
        let _ = previous.window.close();
        previous.subs.clear();
    }

    let subs = Subscriptions::default();
    let window_ref = NodeRef::new();
    let mut tabs: Vec<(String, Vec<Input>)> = Vec::new();
    let mut items = Vec::new();
    for tab in &dialog.tabs {
        let rows = StackPanel::new()?.spacing(10.0)?.margin((18.0, 16.0, 18.0, 16.0))?;
        let list = rows.children()?;
        let mut inputs = Vec::new();
        for row in &tab.rows {
            list.add(parts::row(&scope, Form::Prefs, row, window_ref, &mut inputs, &subs)?)?;
        }
        tabs.push((tab.id.clone(), inputs));
        items.push(TabItem::el().header(tab.title.clone()).content(ScrollViewer::el().content(AnyEl::from_control(rows))));
    }
    let selected = dialog.tabs.iter().position(|t| t.id == dialog.tab).unwrap_or(0);

    let ok = parts::on_click(
        Button::el().content("OK").min_width(90.0).default(true).horizontal_content_alignment(HorizontalAlignment::Center),
        &subs,
        move |_| {
            // The values to send, by tab.
            let values: Map<String, Value> = tabs.iter().map(|(id, inputs)| (id.clone(), parts::values(inputs))).collect();
            let action = format!("prefs-apply:{}", Value::Object(values));
            super::main().model.run(|m| m.run(action));
            close_later();
        },
    );
    let cancel = parts::on_click(
        Button::el().content("Cancel").min_width(90.0).cancel(true).horizontal_content_alignment(HorizontalAlignment::Center),
        &subs,
        |_| close_later(),
    );
    let window = Window::el()
        .title("Jiffshot Preferences")
        .icon(super::icon_file())
        .width(640.0)
        .height(560.0)
        .min_width(520.0)
        .min_height(420.0)
        .window_startup_location(WindowStartupLocation::CenterScreen)
        .content(
            DockPanel::el()
                .child(
                    StackPanel::el()
                        .orientation(Orientation::Horizontal)
                        .spacing(8.0)
                        .horizontal_alignment(HorizontalAlignment::Right)
                        .margin(14.0)
                        .dock_panel_dock(Dock::Bottom)
                        .child(ok)
                        .child(cancel),
                )
                .child(TabControl::el().items_iter(items).selected_index(i32::try_from(selected).unwrap_or(0))),
        )
        .node_ref(window_ref)
        .build(&scope)?;

    // Over the editor when it is showing.
    match scope.main_window().filter(|main| main.get_is_visible().unwrap_or(false)) {
        Some(owner) => window.show_with_window(&owner)?,
        None => window.show()?,
    }
    if let Some(hwnd) = parts::hwnd(&scope, &window) {
        crate::winutil::bring_forward(hwnd as isize);
    }
    with(|s| s.open = Some(Open { window, subs }));
    Ok(())
}

/// Closes the dialog once the click that asked for it has returned (its
/// subscriptions end with it).
fn close_later() {
    let Some(scope) = with(|s| s.scope.clone()) else { return };
    post(&scope, "Preferences", || {
        if let Some(open) = with(|s| s.open.take()).flatten() {
            let _ = open.window.close();
            open.subs.clear();
        }
        Ok(())
    });
}

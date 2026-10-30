//! The Capture window: the tabs on the left, the current tab's dropdowns
//! and switches, the big Capture button, and menus in the title bar and
//! the bottom bar.
//!
//! Rust decides what it shows (`crate::capture_window`, in
//! `jiffshot_workflow::panel`'s JSON): the `view` signal lays out the tab,
//! and `menu`, `panel` and `problem` ("serial\x1fJSON") open a menu, an
//! option panel or a note under the control that asked for it. This module
//! only lays them out and sends back actions.

pub(super) mod parts;

use self::parts::{on_click, payload, post, Form, Input, Subscriptions, CTRL, GLYPHS};
use crate::vm::CaptureBinding;
use rustolonia::{
    AnyEl, AppScope, AsControl, Border, Button, Color, Control, Dock, DockPanel, Effect, El, Flyout, FontWeight, Grid, HorizontalAlignment, Image, Key, MenuFlyout,
    MenuItem, MenuItemToggleType, NodeRef, Orientation, PlacementMode, Result, Separator, Signal, StackPanel, TextBlock, TextWrapping, ToggleSwitch,
    Upcast, VerticalAlignment, Window, WindowStartupLocation,
};
use serde::Deserialize;
use std::cell::RefCell;

/// The window's colors.
struct Palette {
    back: Color,
    card: Color,
    edge: Color,
    frame: Color,
    text: Color,
    muted: Color,
    tab_on: Color,
}

const LIGHT: Palette = Palette {
    back: parts::rgb(0xF3F3F3),
    card: parts::rgb(0xFFFFFF),
    edge: parts::rgb(0xE0E0E0),
    frame: parts::rgb(0xC8C8C8),
    text: parts::rgb(0x1F1F1F),
    muted: parts::rgb(0x606060),
    tab_on: parts::rgb(0x1971C7),
};

const DARK: Palette = Palette {
    back: parts::rgb(0x1F2023),
    card: parts::rgb(0x2B2D31),
    edge: parts::rgb(0x3A3C40),
    frame: parts::rgb(0x4A4D52),
    text: parts::rgb(0xEDEEF0),
    muted: parts::rgb(0xA9ADB4),
    tab_on: parts::rgb(0x2F7FD6),
};

const TITLE: &str = "Jiffshot Capture";

const WHITE: Color = parts::rgb(0xFFFFFF);
const CAPTURE_RED: Color = parts::rgb(0xDB3C30);
const CAPTURE_RED_HOVER: Color = parts::rgb(0xC73126);
const CAPTURE_RING: Color = parts::rgb(0xF5B0A8);

/// The palette for the application's current theme.
fn palette(dark: Signal<bool>) -> &'static Palette {
    if dark.get() { &DARK } else { &LIGHT }
}

/// The tabs: tooltip, glyph and label.
const TABS: [(&str, &str, &str); 3] = [
    ("All-in-One (Ctrl+1): select an area, then choose image, video or scrolling", "\u{E7B3}", "All-in-One"),
    ("Image (Ctrl+2)", "\u{E722}", "Image"),
    ("Video (Ctrl+3)", "\u{E714}", "Video"),
];

// ---- the JSON (jiffshot_workflow::panel) -----------------------------------

/// What the window shows for the current tab (`panel::View`).
#[derive(Deserialize)]
struct View {
    tab: usize,
    #[serde(default)]
    fields: Vec<FieldRow>,
    #[serde(default)]
    switches: Vec<SwitchRow>,
    #[serde(default)]
    capture: String,
    #[serde(default)]
    hint: String,
}

#[derive(Deserialize)]
struct FieldRow {
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    text: String,
    /// The gear button's panel.
    gear: Option<String>,
}

#[derive(Deserialize)]
struct SwitchRow {
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    on: bool,
    /// The › button's panel.
    more: Option<String>,
}

/// One menu entry (`panel::MenuItem`).
#[derive(Deserialize)]
struct MenuEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    keep_open: bool,
    #[serde(default)]
    separator: bool,
    #[serde(default)]
    items: Vec<MenuEntry>,
}

/// An option panel (`panel::Panel`).
#[derive(Deserialize)]
struct Panel {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    rows: Vec<parts::Row>,
    /// The confirm button's text; none for a read-only panel.
    ok: Option<String>,
}

// ---- state ------------------------------------------------------------------

/// The window and what its callbacks need. UI thread only.
struct Ui {
    scope: AppScope,
    window: NodeRef<Window>,
    dark: Signal<bool>,
    tab: Signal<usize>,
    capture_label: Signal<String>,
    hint: Signal<String>,
    fields: NodeRef<StackPanel>,
    switches: NodeRef<StackPanel>,
    /// The static controls' subscriptions, for the window's life.
    _controls: Subscriptions,
    /// The rows' subscriptions, ended when the rows are rebuilt.
    rows: Subscriptions,
    /// The last menu's or panel's.
    popup: Subscriptions,
    /// The control the next menu, panel or note opens under.
    anchor: Option<Control>,
    topmost: bool,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
}

fn with_ui<R>(f: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    UI.with(|ui| ui.borrow_mut().as_mut().map(f))
}

/// Sends `action` to the model, as the window's controls do.
fn run(action: String) {
    super::capture().model.run(|m| m.run(action));
}

/// Asks the model for a menu or panel (`verb` is "menu" or "panel"), to open
/// under `anchor`.
fn request(verb: &str, which: &str, anchor: Control) {
    with_ui(|ui| ui.anchor = Some(anchor));
    run(format!("{verb}:{which}"));
}

/// The window, and the control the next popup opens under.
fn popup_target() -> Option<(AppScope, Window, Control, Subscriptions)> {
    with_ui(|ui| {
        let window = ui.window.get()?;
        let anchor = ui.anchor.clone().unwrap_or_else(|| window.upcast());
        Some((ui.scope.clone(), window, anchor, ui.popup.clone()))
    })
    .flatten()
}

// ---- building ----------------------------------------------------------------

/// Builds the Capture window for `binding` (not yet shown).
pub fn build(scope: &AppScope, binding: &CaptureBinding) -> Result<Window> {
    let dark = scope.signal(parts::dark_theme(scope));
    let tab = scope.signal(0_usize);
    let capture_label = scope.signal(String::from("Capture"));
    let hint = scope.signal(String::new());
    let (window_ref, fields, switches) = (NodeRef::new(), NodeRef::new(), NodeRef::new());
    // The static controls' subscriptions last as long as the window.
    let subs = Subscriptions::default();

    let menu_button = |label: &str, which: &'static str| {
        on_click(flat(dark).content(label.to_owned()), &subs, move |button: &Button| request("menu", which, button.upcast()))
    };
    let title_bar = Grid::el()
        .column_definitions("Auto,Auto,*")
        .child(Image::el().source(super::icon_file()).width(20.0).height(20.0).margin((14.0, 0.0, 10.0, 0.0)).vertical_alignment(VerticalAlignment::Center))
        .child(
            StackPanel::el()
                .grid_column(1)
                .orientation(Orientation::Horizontal)
                .spacing(2.0)
                .vertical_alignment(VerticalAlignment::Center)
                .child(menu_button("File", "file"))
                .child(menu_button("Edit", "edit"))
                .child(menu_button("View", "view"))
                .child(menu_button("Help", "help")),
        );

    let tabs = StackPanel::el()
        .spacing(4.0)
        .margin((4.0, 0.0, 4.0, 0.0))
        .children_iter(TABS.iter().enumerate().map(|(i, &(tip, glyph, label))| tab_button(dark, tab, &subs, i, tip, glyph, label)));

    let capture_button = {
        let hover = scope.signal(false);
        Border::el()
            .width(104.0)
            .height(104.0)
            .corner_radius(52.0)
            .border_thickness(3.0)
            .border_brush(CAPTURE_RING)
            .background(move || if hover.get() { CAPTURE_RED_HOVER } else { CAPTURE_RED })
            .on_pointer_entered(move |_| hover.set(true))
            .on_pointer_exited(move |_| hover.set(false))
            .on_tapped(|_| run("capture".into()))
            .child(
                TextBlock::el()
                    .text(capture_label)
                    .font_size(17.0)
                    .font_weight(FontWeight::DemiBold)
                    .foreground(WHITE)
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .vertical_alignment(VerticalAlignment::Center),
            )
    };
    let reset = on_click(
        flat(dark)
            .horizontal_alignment(HorizontalAlignment::Right)
            .vertical_alignment(VerticalAlignment::Top)
            .margin((0.0, 4.0, 4.0, 0.0))
            .tool_tip_tip("Reset this tab's options")
            .content(glyph("\u{E72C}", 15.0)),
        &subs,
        |_| run("pick:reset".into()),
    );
    let card = Border::el()
        .grid_column(1)
        .background(move || palette(dark).card)
        .border_brush(move || palette(dark).edge)
        .border_thickness(1.0)
        .corner_radius(8.0)
        .child(
            Grid::el()
                .column_definitions("340,*,170")
                .child(StackPanel::el().spacing(14.0).margin((10.0, 16.0, 0.0, 0.0)).node_ref(fields))
                .child(StackPanel::el().grid_column(1).margin((26.0, 10.0, 6.0, 0.0)).node_ref(switches))
                .child(
                    Grid::el().grid_column(2).child(reset).child(
                        StackPanel::el()
                            .vertical_alignment(VerticalAlignment::Center)
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .spacing(9.0)
                            .margin((0.0, 8.0, 0.0, 0.0))
                            .child(capture_button)
                            .child(TextBlock::el().text(hint).horizontal_alignment(HorizontalAlignment::Center).font_size(12.5)),
                    ),
                ),
        );

    let editor = on_click(
        flat(dark).dock_panel_dock(Dock::Right).vertical_alignment(VerticalAlignment::Center).content(
            StackPanel::el()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .child(glyph("\u{E8A7}", 14.0).foreground(move || palette(dark).muted))
                .child(TextBlock::el().text("Open Jiffshot Editor").vertical_alignment(VerticalAlignment::Center)),
        ),
        &subs,
        |_| run("editor".into()),
    );
    let presets = on_click(
        flat(dark).tool_tip_tip("Run, edit, import or export presets").content(
            StackPanel::el()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .child(glyph("\u{E713}", 15.0))
                .child(TextBlock::el().text("Presets").vertical_alignment(VerticalAlignment::Center)),
        ),
        &subs,
        |button: &Button| request("menu", "presets", button.upcast()),
    );
    let new_preset = on_click(
        flat(dark)
            .tool_tip_tip("New preset from these settings")
            .content(StackPanel::el().orientation(Orientation::Horizontal).spacing(4.0).child(glyph("\u{E710}", 14.0)).child(glyph("\u{E70D}", 9.0))),
        &subs,
        |button: &Button| request("menu", "new-preset", button.upcast()),
    );
    let bottom_bar = DockPanel::el()
        .grid_row(2)
        .margin((8.0, 0.0, 8.0, 0.0))
        .child(editor)
        .child(StackPanel::el().orientation(Orientation::Horizontal).spacing(2.0).vertical_alignment(VerticalAlignment::Center).child(presets).child(new_preset));

    let theme_scope = scope.clone();
    let closing_scope = scope.clone();
    let loaded_scope = scope.clone();
    let window = Window::el()
        // The drawn title bar holds the icon and the menus, so the title is
        // only the native window's (the taskbar's and Alt+Tab's).
        .title("")
        .on_loaded(move |_| {
            if let Some(hwnd) = window_ref.get().and_then(|window| parts::hwnd(&loaded_scope, &window)) {
                parts::set_native_title(hwnd, TITLE);
            }
        })
        .icon(super::icon_file())
        .width(890.0)
        .height(290.0)
        .can_resize(false)
        .can_maximize(false)
        .window_startup_location(WindowStartupLocation::CenterScreen)
        .extend_client_area_to_decorations_hint(true)
        .extend_client_area_title_bar_height_hint(40.0)
        .background(move || palette(dark).back)
        .foreground(move || palette(dark).text)
        .on_actual_theme_variant_changed(move |_| dark.set_if_changed(parts::dark_theme(&theme_scope)))
        .on_key_down(on_key)
        .on_closing(move |e| {
            // Closing keeps it for next time; only Jiffshot's Exit ends it.
            if parts::closing_for_good(e.close_reason) || std::env::var_os("JIFFSHOT_UITEST").is_some() {
                return;
            }
            e.cancel = true;
            post(&closing_scope, "Capture window", || {
                close_to_tray();
                Ok(())
            });
        })
        .content(
            Grid::el()
                .row_definitions("40,*,44")
                .child(title_bar)
                .child(Grid::el().grid_row(1).column_definitions("96,*").margin((0.0, 0.0, 8.0, 0.0)).child(tabs).child(card))
                .child(bottom_bar),
        )
        .node_ref(window_ref)
        .build(scope)?;

    UI.with(|ui| {
        *ui.borrow_mut() = Some(Ui {
            scope: scope.clone(),
            window: window_ref,
            dark,
            tab,
            capture_label,
            hint,
            fields,
            switches,
            _controls: subs,
            rows: Subscriptions::default(),
            popup: Subscriptions::default(),
            anchor: None,
            topmost: false,
        })
    });
    follow(scope, binding)?;
    Ok(window)
}

/// Follows the model's signals. Each is handled after
/// the action that changed it has returned: rebuilding the rows from inside
/// a row's own click would end the subscription that is running.
fn follow(scope: &AppScope, binding: &CaptureBinding) -> Result<()> {
    let sink = binding.sink;
    let s = scope.clone();
    Effect::new(move || {
        let json = sink.view.get();
        post(&s, "Capture window view", move || apply_view(&json));
        Ok(())
    })?;
    let s = scope.clone();
    Effect::new(move || {
        if let Some(json) = payload(&sink.menu.get()).map(str::to_owned) {
            post(&s, "Capture window menu", move || show_menu(&json));
        }
        Ok(())
    })?;
    let s = scope.clone();
    Effect::new(move || {
        if let Some(json) = payload(&sink.panel.get()).map(str::to_owned) {
            post(&s, "Capture window panel", move || show_panel(&json));
        }
        Ok(())
    })?;
    let s = scope.clone();
    Effect::new(move || {
        if let Some(text) = payload(&sink.problem.get()).map(str::to_owned) {
            post(&s, "Capture window note", move || show_problem(&text));
        }
        Ok(())
    })?;
    Ok(())
}

/// A borderless button (`Button.flat`).
fn flat(dark: Signal<bool>) -> El<Button> {
    Button::el().background(parts::TRANSPARENT).padding((9.0, 4.0)).corner_radius(5.0).foreground(move || palette(dark).text)
}

/// An icon-font glyph.
fn glyph(text: &str, size: f64) -> El<TextBlock> {
    TextBlock::el()
        .text(text.to_owned())
        .font_family(GLYPHS)
        .font_size(size)
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center)
}

/// One of the tabs on the left; the current one has its glyph on the accent.
fn tab_button(dark: Signal<bool>, tab: Signal<usize>, subs: &Subscriptions, i: usize, tip: &str, icon: &str, label: &str) -> El<Button> {
    let on = move || tab.get() == i;
    let content = StackPanel::el()
        .spacing(3.0)
        .child(
            Border::el()
                .width(32.0)
                .height(28.0)
                .corner_radius(5.0)
                .background(move || if on() { palette(dark).tab_on } else { parts::TRANSPARENT })
                .child(glyph(icon, 18.0).foreground(move || if on() { WHITE } else { palette(dark).text })),
        )
        .child(TextBlock::el().text(label.to_owned()).font_size(12.0).horizontal_alignment(HorizontalAlignment::Center));
    let button = Button::el()
        .background(parts::TRANSPARENT)
        .foreground(move || palette(dark).text)
        .width(88.0)
        .height(58.0)
        .padding(0.0)
        .corner_radius(6.0)
        .horizontal_content_alignment(HorizontalAlignment::Center)
        .tool_tip_tip(tip.to_owned())
        .content(content);
    on_click(button, subs, move |_| run(format!("tab:{i}")))
}

// ---- the view ----------------------------------------------------------------

fn apply_view(json: &str) -> Result<()> {
    let Ok(view) = serde_json::from_str::<View>(json) else { return Ok(()) };
    let Some((scope, dark, fields, switches, rows, (tab, capture_label, hint))) =
        with_ui(|ui| Some((ui.scope.clone(), ui.dark, ui.fields.get()?, ui.switches.get()?, ui.rows.clone(), (ui.tab, ui.capture_label, ui.hint)))).flatten()
    else {
        return Ok(());
    };
    tab.set_if_changed(view.tab);
    capture_label.set_if_changed(view.capture.clone());
    hint.set_if_changed(view.hint.clone());
    rows.clear();

    let list = fields.children()?;
    list.clear()?;
    for field in &view.fields {
        list.add(field_row(dark, &rows, field).build(&scope)?)?;
    }
    let list = switches.children()?;
    list.clear()?;
    let height = if view.switches.len() > 4 { 31.0 } else { 45.0 };
    for row in &view.switches {
        list.add(switch_row(dark, &rows, row, height).build(&scope)?)?;
    }
    Ok(())
}

/// A dropdown row: label, the dropdown (a menu) and maybe a gear (a panel).
fn field_row(dark: Signal<bool>, subs: &Subscriptions, field: &FieldRow) -> El<Grid> {
    let id = field.id.clone();
    let drop = Button::el()
        .grid_column(1)
        .width(200.0)
        .height(30.0)
        .background(move || palette(dark).card)
        .border_brush(move || palette(dark).frame)
        .border_thickness(1.0)
        .corner_radius(4.0)
        .padding((9.0, 0.0, 7.0, 0.0))
        .horizontal_content_alignment(HorizontalAlignment::Stretch)
        .vertical_content_alignment(VerticalAlignment::Center)
        .tool_tip_tip(field.text.clone())
        .content(
            DockPanel::el()
                .child(glyph("\u{E70D}", 10.0).dock_panel_dock(Dock::Right).margin((6.0, 0.0, 0.0, 0.0)))
                .child(TextBlock::el().text(field.text.clone()).text_trimming("CharacterEllipsis").vertical_alignment(VerticalAlignment::Center).font_size(13.0)),
        );
    let grid = Grid::el()
        .column_definitions("100,*,Auto")
        .height(30.0)
        .child(TextBlock::el().text(field.label.clone()).vertical_alignment(VerticalAlignment::Center).font_size(13.5))
        .child(on_click(drop, subs, move |button: &Button| request("menu", &id, button.upcast())));
    match field.gear.clone() {
        Some(panel) => grid.child(on_click(
            flat(dark).grid_column(2).padding((6.0, 4.0)).margin((4.0, 0.0, 0.0, 0.0)).tool_tip_tip("Options").content(glyph("\u{E713}", 14.0)),
            subs,
            move |button: &Button| request("panel", &panel, button.upcast()),
        )),
        None => grid,
    }
}

/// A switch row: label, the switch, and maybe a › button (a panel).
fn switch_row(dark: Signal<bool>, subs: &Subscriptions, row: &SwitchRow, height: f64) -> El<Grid> {
    let id = row.id.clone();
    let toggle = ToggleSwitch::el()
        .grid_column(1)
        .checked(Some(row.on))
        .on_content(None::<AnyEl>)
        .off_content(None::<AnyEl>)
        .vertical_alignment(VerticalAlignment::Center)
        .margin((8.0, 0.0, 0.0, 0.0));
    let grid = Grid::el()
        .column_definitions("*,Auto,26")
        .height(height)
        .child(TextBlock::el().text(row.label.clone()).vertical_alignment(VerticalAlignment::Center).font_size(13.5))
        .child(on_click(toggle, subs, move |_| run(format!("switch:{id}"))));
    match row.more.clone() {
        Some(panel) => grid.child(on_click(
            flat(dark).grid_column(2).padding((5.0, 3.0)).tool_tip_tip("Options").content(glyph("\u{E76C}", 11.0)),
            subs,
            move |button: &Button| request("panel", &panel, button.upcast()),
        )),
        None => grid,
    }
}

// ---- keys and closing ------------------------------------------------------

/// Enter captures, Escape closes, Ctrl+1…3 pick a tab.
fn on_key(e: &mut rustolonia::ControlKeyDownEventArgs) {
    if e.handled {
        return;
    }
    let ctrl = e.key_modifiers & CTRL != 0;
    match Key::try_from(e.key) {
        Ok(Key::Return) => run("capture".into()),
        Ok(Key::Escape) => close_to_tray(),
        Ok(key @ (Key::D1 | Key::D2 | Key::D3)) if ctrl => run(format!("tab:{}", key as i32 - Key::D1 as i32)),
        _ => return,
    }
    e.handled = true;
}

/// Hides the window (it stays mounted for next time) and tells the model.
fn close_to_tray() {
    if let Some(window) = with_ui(|ui| ui.window.get()).flatten() {
        let _ = window.hide();
    }
    run("closed".into());
}

// ---- menus -------------------------------------------------------------------

fn show_menu(json: &str) -> Result<()> {
    let Ok(entries) = serde_json::from_str::<Vec<MenuEntry>>(json) else { return Ok(()) };
    let Some((_, _, anchor, popup)) = popup_target() else { return Ok(()) };
    popup.clear();
    let flyout = MenuFlyout::new()?.placement(PlacementMode::BottomEdgeAlignedLeft)?;
    let items = flyout.items()?;
    for entry in &entries {
        items.add(menu_entry(entry, &popup)?)?;
    }
    flyout.show_at_with_control(&anchor)
}

fn menu_entry(entry: &MenuEntry, subs: &Subscriptions) -> Result<Control> {
    if entry.separator {
        return Ok(Separator::new()?.upcast());
    }
    let (label, keys) = match entry.label.split_once('\t') {
        Some((label, keys)) => (label, Some(keys)),
        None => (entry.label.as_str(), None),
    };
    let item = MenuItem::new()?.enabled(entry.enabled)?.stays_open_on_click(entry.keep_open)?;
    match keys {
        // A shortcut or preset hotkey, as text: not every one parses as a gesture.
        Some(keys) => {
            let header = DockPanel::new()?
                .min_width(220.0)?
                .child(TextBlock::new()?.text(keys)?.opacity(0.65)?.margin((24.0, 0.0, 0.0, 0.0))?.dock_panel_dock(Dock::Right)?)?
                .child(TextBlock::new()?.text(label)?)?;
            item.set_header(Some(&header as &dyn AsControl))?;
        }
        None => item.set_header_text(label)?,
    }
    if !entry.items.is_empty() {
        let children = item.items()?;
        for child in &entry.items {
            children.add(menu_entry(child, subs)?)?;
        }
        if entry.checked {
            item.set_header_text(format!("{label}  \u{2713}"))?;
        }
        return Ok(item.upcast());
    }
    if entry.checked || entry.keep_open {
        item.set_toggle_type(MenuItemToggleType::CheckBox)?;
        item.set_checked(entry.checked)?;
    }
    let (id, keep_open, this) = (entry.id.clone(), entry.keep_open, item.clone());
    subs.keep(item.subscribe_click(move |_| {
        if keep_open {
            let checked = this.get_is_checked().unwrap_or(false);
            let _ = this.set_checked(!checked);
        }
        if let Some(command) = id.strip_prefix("window:") {
            window_command(command);
        } else if !id.is_empty() {
            run(format!("pick:{id}"));
        }
    })?);
    Ok(item.upcast())
}

/// Menu commands about this window itself.
fn window_command(command: &str) {
    let Some((scope, window, topmost)) = with_ui(|ui| {
        if command == "topmost" {
            ui.topmost = !ui.topmost;
        }
        Some((ui.scope.clone(), ui.window.get()?, ui.topmost))
    })
    .flatten() else {
        return;
    };
    let done = match command {
        "topmost" => scope.set_topmost(&window, topmost),
        "about" => super::dialogs::about(&scope, &window),
        _ => Ok(()),
    };
    if let Err(error) = done {
        eprintln!("Capture window: {error:?}");
    }
}

// ---- option panels and notes -----------------------------------------------

fn show_panel(json: &str) -> Result<()> {
    let Ok(panel) = serde_json::from_str::<Panel>(json) else { return Ok(()) };
    let Some((scope, _, anchor, popup)) = popup_target() else { return Ok(()) };
    let Some(owner) = with_ui(|ui| ui.window) else { return Ok(()) };
    popup.clear();

    let body = StackPanel::new()?.spacing(8.0)?.margin(4.0)?.min_width(280.0)?;
    let rows = body.children()?;
    rows.add(TextBlock::new()?.text(&panel.title)?.font_weight(FontWeight::DemiBold)?.font_size(14.0)?.margin((0.0, 0.0, 0.0, 4.0))?)?;
    let mut inputs: Vec<Input> = Vec::new();
    for row in &panel.rows {
        rows.add(parts::row(&scope, Form::Panel, row, owner, &mut inputs, &popup)?)?;
    }
    let flyout = Flyout::new()?.placement(PlacementMode::BottomEdgeAlignedLeft)?.content(Some(&body as &dyn AsControl))?;

    let buttons = StackPanel::new()?.orientation(Orientation::Horizontal)?.spacing(8.0)?.horizontal_alignment(HorizontalAlignment::Right)?.margin((0.0, 8.0, 0.0, 0.0))?;
    if let Some(ok) = &panel.ok {
        let apply = Button::new()?.content_text(ok)?.default(true)?.min_width(72.0)?.horizontal_content_alignment(HorizontalAlignment::Center)?;
        let (id, open) = (panel.id.clone(), flyout.clone());
        popup.keep(apply.subscribe_click(move |_| {
            let _ = open.hide();
            run(format!("apply:{id}:{}", parts::values(&inputs)));
        })?);
        buttons.children()?.add(&apply)?;
    }
    let close = Button::new()?
        .content_text(if panel.ok.is_some() { "Cancel" } else { "Close" })?
        .min_width(72.0)?
        .horizontal_content_alignment(HorizontalAlignment::Center)?;
    let open = flyout.clone();
    popup.keep(close.subscribe_click(move |_| {
        let _ = open.hide();
    })?);
    buttons.children()?.add(&close)?;
    rows.add(&buttons)?;
    flyout.show_at_with_control(&anchor)
}

fn show_problem(text: &str) -> Result<()> {
    let Some((_, _, anchor, _)) = popup_target() else { return Ok(()) };
    let note = TextBlock::new()?.text(text)?.text_wrapping(TextWrapping::Wrap)?.max_width(320.0)?.margin(4.0)?;
    Flyout::new()?.placement(PlacementMode::BottomEdgeAlignedLeft)?.content(Some(&note as &dyn AsControl))?.show_at_with_control(&anchor)
}

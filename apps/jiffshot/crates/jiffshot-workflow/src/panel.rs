//! The Capture window's behaviour, with no UI toolkit: what each tab shows,
//! what its dropdowns, menus and option panels offer, and what each choice
//! changes. The app's Avalonia window shows a [`View`], [`MenuItem`] lists
//! and [`Panel`]s (sent to it as JSON) and hands back the ids and values the
//! user picked.

use crate::settings::{Preset, Settings};
use crate::workflow::{SHARES, VIDEO_SHARES};
use serde::Serialize;
use serde_json::Value;

pub const TABS: [&str; 3] = ["All-in-One", "Image", "Video"];

/// The Image tab's selections, with the preset mode names they stand for.
const SELECTIONS: &[(&str, &str)] = &[
    ("Region", "image"),
    ("Window", "window"),
    ("Fullscreen", "fullscreen"),
    ("Scrolling Window", "scrolling"),
    ("Grab Text", "text"),
    ("Clipboard", "clipboard"),
    ("File", "file"),
];
/// The rest, under Advanced.
const ADVANCED: &[(&str, &str)] = &[
    ("Menu", "menu"),
    ("Freehand", "freehand"),
    ("Multiple Area", "multi"),
    ("Panoramic", "panoramic"),
    ("Scrolling Horizontal", "horizontal"),
    ("Scrolling Both Ways", "both"),
];
const VIDEO_SELECTIONS: &[(&str, &str)] = &[("Region", "region"), ("Window", "window"), ("Fullscreen", "fullscreen")];
/// Capture effects in groups, as (group, effects); a group of one is a
/// plain item.
const EFFECT_GROUPS: &[(&str, &[&str])] = &[
    ("Border", &["Border"]),
    ("Edges", &["Torn edge", "Torn bottom", "Sawtooth edge", "Wave edge", "Fade edges"]),
    ("Shadow", &["Shadow"]),
    ("Perspective", &["Perspective", "Tilt"]),
    ("Page Curl", &["Page curl"]),
    ("Rounded Corners", &["Rounded corners"]),
    ("Reflection", &["Reflection"]),
    ("Filters", &["Sharpen", "Blur image", "Monochrome", "Grayscale", "Sepia", "Invert", "Vignette"]),
    ("Color Adjustment", &["Brighten", "Darken", "More contrast", "Less contrast", "Saturate", "Desaturate", "Hue shift", "Gamma up", "Gamma down"]),
    ("Color Replacement", &["Replace color"]),
    ("Scale Image", &["Scale to 1920"]),
];
const CORNER_NAMES: [&str; 4] = ["Top Left", "Top Right", "Bottom Left", "Bottom Right"];
const CORNERS: [&str; 4] = ["top-left", "top-right", "bottom-left", "bottom-right"];
const TIMINGS: [&str; 3] = ["delay", "interval", "schedule"];
pub(crate) const FPS: [u32; 4] = [15, 24, 30, 60];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    Selection,
    Effects,
    Share,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Switch {
    Preview,
    Clipboard,
    Cursor,
    Delay,
    Webcam,
    Microphone,
    SystemAudio,
    RecordCursor,
    ScreenDraw,
}

impl Field {
    fn id(self) -> &'static str {
        match self {
            Field::Selection => "selection",
            Field::Effects => "effects",
            Field::Share => "share",
        }
    }
}

impl Switch {
    const ALL: [Switch; 9] = [
        Switch::Preview,
        Switch::Clipboard,
        Switch::Cursor,
        Switch::Delay,
        Switch::Webcam,
        Switch::Microphone,
        Switch::SystemAudio,
        Switch::RecordCursor,
        Switch::ScreenDraw,
    ];

    fn id(self) -> &'static str {
        match self {
            Switch::Preview => "preview",
            Switch::Clipboard => "clipboard",
            Switch::Cursor => "cursor",
            Switch::Delay => "delay",
            Switch::Webcam => "webcam",
            Switch::Microphone => "microphone",
            Switch::SystemAudio => "system-audio",
            Switch::RecordCursor => "record-cursor",
            Switch::ScreenDraw => "draw",
        }
    }

    pub fn from_id(id: &str) -> Option<Switch> {
        Switch::ALL.into_iter().find(|s| s.id() == id)
    }
}

pub fn fields(tab: usize) -> &'static [Field] {
    match tab {
        // All-in-One asks what to capture after the area is selected.
        0 => &[Field::Share],
        1 => &[Field::Selection, Field::Effects, Field::Share],
        _ => &[Field::Selection, Field::Share],
    }
}

pub fn switches(tab: usize) -> &'static [Switch] {
    match tab {
        0 | 1 => &[Switch::Preview, Switch::Clipboard, Switch::Cursor, Switch::Delay],
        _ => &[Switch::Webcam, Switch::Microphone, Switch::SystemAudio, Switch::RecordCursor, Switch::ScreenDraw, Switch::Preview],
    }
}

fn has_gear(tab: usize, field: Field) -> bool {
    matches!((tab, field), (1 | 2, Field::Selection))
}

/// The panel behind a switch's › button, if it has one.
fn more_panel(switch: Switch) -> Option<&'static str> {
    match switch {
        Switch::Delay => Some("delay"),
        Switch::Webcam => Some("webcam"),
        Switch::Microphone => Some("microphone"),
        Switch::RecordCursor => Some("cursor"),
        Switch::ScreenDraw => Some("draw"),
        _ => None,
    }
}

fn field_label(field: Field) -> &'static str {
    match field {
        Field::Selection => "Selection:",
        Field::Effects => "Effects:",
        Field::Share => "Share:",
    }
}

fn timing(s: &Settings) -> &str {
    s.capture_window.timing.as_str()
}

pub fn switch_label(s: &Settings, switch: Switch) -> String {
    match switch {
        Switch::Preview => "Preview in Editor".into(),
        Switch::Clipboard => "Copy to Clipboard".into(),
        Switch::Cursor | Switch::RecordCursor => "Capture Cursor".into(),
        Switch::Delay if !switch_on(s, switch) => "Time Delay".into(),
        Switch::Delay => match timing(s) {
            "interval" => format!("Interval ({} s)", s.interval_seconds.max(3)),
            "schedule" => format!("Schedule ({})", s.schedule_default.as_deref().unwrap_or("09:00")),
            _ => format!("Time Delay ({} s)", s.delay_seconds),
        },
        Switch::Webcam => "Record Webcam".into(),
        Switch::Microphone => "Record Microphone".into(),
        Switch::SystemAudio => "Record System Audio".into(),
        Switch::ScreenDraw => "Screen Draw".into(),
    }
}

fn label_of<'a>(list: &[(&'a str, &str)], value: &str) -> &'a str {
    list.iter().find(|(_, v)| *v == value).map_or(list[0].0, |(l, _)| l)
}

/// What a dropdown shows.
pub fn field_text(s: &Settings, tab: usize, field: Field) -> String {
    match field {
        Field::Selection if tab == 2 => label_of(VIDEO_SELECTIONS, &s.capture_window.video_selection).into(),
        Field::Selection => {
            let all: Vec<(&str, &str)> = SELECTIONS.iter().chain(ADVANCED).copied().collect();
            let name = label_of(&all, &s.capture_window.selection);
            if name == "Region" && fixed(s) {
                format!("Region ({} × {})", s.fixed_size.0, s.fixed_size.1)
            } else {
                name.into()
            }
        }
        Field::Effects if s.capture_effects.is_empty() => "None".into(),
        Field::Effects => s.capture_effects.join(", "),
        Field::Share if tab == 2 => label_of(VIDEO_SHARES, &s.video_share).into(),
        Field::Share => label_of(SHARES, &s.share).into(),
    }
}

/// A Region selection is a fixed size ("fixed" is the older setting).
pub fn fixed(s: &Settings) -> bool {
    match s.capture_window.selection.as_str() {
        "fixed" => true,
        "image" => s.capture_window.fixed,
        _ => false,
    }
}

fn audio(s: &Settings) -> (bool, bool) {
    (matches!(s.record_audio.as_str(), "system" | "both"), matches!(s.record_audio.as_str(), "microphone" | "both"))
}

fn set_audio(s: &mut Settings, system: bool, mic: bool) {
    s.record_audio = match (system, mic) {
        (true, true) => "both",
        (true, false) => "system",
        (false, true) => "microphone",
        (false, false) => "none",
    }
    .to_owned();
}

pub fn switch_on(s: &Settings, switch: Switch) -> bool {
    match switch {
        Switch::Preview => s.open_in_editor,
        Switch::Clipboard => s.copy_to_clipboard,
        Switch::Cursor => s.include_cursor,
        Switch::Delay if timing(s) == "delay" => s.delay_seconds > 0,
        Switch::Delay => s.capture_window.timer,
        Switch::Webcam => s.webcam.is_some(),
        Switch::SystemAudio => audio(s).0,
        Switch::Microphone => audio(s).1,
        Switch::RecordCursor => s.record_cursor,
        Switch::ScreenDraw => s.screen_draw,
    }
}

pub fn flip(s: &mut Settings, switch: Switch) {
    match switch {
        Switch::Preview => s.open_in_editor ^= true,
        Switch::Clipboard => s.copy_to_clipboard ^= true,
        Switch::Cursor => s.include_cursor ^= true,
        Switch::Delay if timing(s) != "delay" => s.capture_window.timer ^= true,
        Switch::Delay if s.delay_seconds > 0 => {
            s.capture_window.delay = s.delay_seconds;
            s.delay_seconds = 0;
        }
        Switch::Delay => s.delay_seconds = s.capture_window.delay.clamp(1, 600),
        Switch::Webcam => {
            s.webcam = match s.webcam.take() {
                Some(corner) => {
                    s.capture_window.webcam_corner = corner;
                    None
                }
                None => Some(s.capture_window.webcam_corner.clone()),
            }
        }
        Switch::RecordCursor => s.record_cursor ^= true,
        Switch::ScreenDraw => s.screen_draw ^= true,
        Switch::SystemAudio => {
            let (system, mic) = audio(s);
            set_audio(s, !system, mic);
        }
        Switch::Microphone => {
            let (system, mic) = audio(s);
            set_audio(s, system, !mic);
        }
    }
}

/// Puts the tab's options back to their defaults.
pub fn reset(s: &mut Settings, tab: usize) {
    let d = Settings::default();
    s.open_in_editor = d.open_in_editor;
    if tab == 2 {
        s.record_audio = d.record_audio;
        s.record_fps = d.record_fps;
        s.noise_gate = d.noise_gate;
        s.countdown = d.countdown;
        s.cursor_highlight = d.cursor_highlight;
        s.click_rings = d.click_rings;
        s.webcam = d.webcam;
        s.webcam_shape = d.webcam_shape;
        s.webcam_size = d.webcam_size;
        s.record_cursor = d.record_cursor;
        s.screen_draw = d.screen_draw;
        s.draw_fade_seconds = d.draw_fade_seconds;
        s.video_share = d.video_share;
        s.capture_window.video_selection = d.capture_window.video_selection;
    } else {
        s.copy_to_clipboard = d.copy_to_clipboard;
        s.include_cursor = d.include_cursor;
        s.delay_seconds = 0;
        s.cursor_as_object = d.cursor_as_object;
        s.capture_effects = d.capture_effects;
        s.fixed_size = d.fixed_size;
        s.share = d.share;
        s.capture_window.selection = d.capture_window.selection;
        s.capture_window.fixed = false;
        s.capture_window.fixed_at = None;
        s.capture_window.timer = false;
        s.capture_window.timing = d.capture_window.timing;
    }
}

/// The preset mode name a capture from `tab` starts.
pub fn mode_of(s: &Settings, tab: usize) -> String {
    match tab {
        0 => "all".into(),
        1 if fixed(s) => "fixed".into(),
        1 => s.capture_window.selection.clone(),
        _ if s.capture_window.video_selection == "fullscreen" => "video-fullscreen".into(),
        _ => "video".into(),
    }
}

/// "PrtScn" as it reads under the Capture button.
pub(crate) fn key_name(key: &str) -> String {
    key.replace("PrtScn", "Print Screen")
}

/// The Capture button's label and the line under it.
pub fn capture_button(s: &Settings, tab: usize, capture_key: &str, timelapse_running: bool) -> (&'static str, String) {
    if tab == 2 {
        return ("Capture", crate::recorder::key_names().1.map_or("The toolbar stops it".into(), |k| format!("{k} stops")));
    }
    match timing(s) {
        "interval" if s.capture_window.timer => {
            let every = format!("Every {} s", s.interval_seconds.max(3));
            (if timelapse_running { "Stop" } else { "Start" }, every)
        }
        "schedule" if s.capture_window.timer => {
            let at = s.schedule_default.clone().unwrap_or_else(|| "09:00".into());
            match &s.schedule {
                Some(on) => ("Cancel", format!("Daily at {on}")),
                None => ("Schedule", format!("Daily at {at}")),
            }
        }
        _ => ("Capture", key_name(capture_key)),
    }
}

// ---- the view -------------------------------------------------------------

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct FieldRow {
    pub id: &'static str,
    pub label: &'static str,
    pub text: String,
    /// The gear button's panel, if there is one.
    pub gear: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SwitchRow {
    pub id: &'static str,
    pub label: String,
    pub on: bool,
    /// The › button's panel, if there is one.
    pub more: Option<&'static str>,
}

/// What the window shows for the current tab.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct View {
    pub tab: usize,
    pub fields: Vec<FieldRow>,
    pub switches: Vec<SwitchRow>,
    pub capture: &'static str,
    pub hint: String,
    pub widget: bool,
}

pub fn view(s: &Settings, capture_key: &str, timelapse_running: bool) -> View {
    let tab = s.capture_window.tab.min(2);
    let (capture, hint) = capture_button(s, tab, capture_key, timelapse_running);
    View {
        tab,
        fields: fields(tab)
            .iter()
            .map(|&f| FieldRow { id: f.id(), label: field_label(f), text: field_text(s, tab, f), gear: has_gear(tab, f).then_some("selection") })
            .collect(),
        switches: switches(tab).iter().map(|&sw| SwitchRow { id: sw.id(), label: switch_label(s, sw), on: switch_on(s, sw), more: more_panel(sw) }).collect(),
        capture,
        hint,
        widget: s.show_widget,
    }
}

// ---- menus ----------------------------------------------------------------

/// One menu entry: an id the window sends back when it is picked, or a
/// separator, or a submenu.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub checked: bool,
    pub enabled: bool,
    /// The menu stays open after a pick (effects, which combine).
    pub keep_open: bool,
    pub separator: bool,
    pub items: Vec<MenuItem>,
}

fn item(id: impl Into<String>, label: impl Into<String>) -> MenuItem {
    MenuItem { id: id.into(), label: label.into(), enabled: true, ..MenuItem::default() }
}

fn check(id: impl Into<String>, label: impl Into<String>, checked: bool) -> MenuItem {
    MenuItem { checked, ..item(id, label) }
}

fn separator() -> MenuItem {
    MenuItem { separator: true, ..MenuItem::default() }
}

fn submenu(label: impl Into<String>, items: Vec<MenuItem>) -> MenuItem {
    MenuItem { items, ..item("", label) }
}

/// A menu by name: a dropdown ("selection", "effects", "share"), the bottom
/// bar ("presets", "new-preset") or a title menu ("file", "edit", "view",
/// "help").
pub fn menu(s: &Settings, which: &str) -> Vec<MenuItem> {
    let tab = s.capture_window.tab.min(2);
    match which {
        "selection" if tab == 2 => VIDEO_SELECTIONS.iter().map(|(l, v)| check(format!("vsel:{v}"), *l, s.capture_window.video_selection == *v)).collect(),
        "selection" => {
            let current = if s.capture_window.selection == "fixed" { "image" } else { s.capture_window.selection.as_str() };
            let mut items: Vec<MenuItem> = SELECTIONS.iter().map(|(l, v)| check(format!("sel:{v}"), *l, *v == current)).collect();
            items.push(separator());
            let advanced: Vec<MenuItem> = ADVANCED.iter().map(|(l, v)| check(format!("sel:{v}"), *l, *v == current)).collect();
            let in_advanced = ADVANCED.iter().any(|(_, v)| *v == current);
            items.push(MenuItem { checked: in_advanced, ..submenu("Advanced", advanced) });
            items
        }
        "share" => {
            let (list, current, prefix) = if tab == 2 { (VIDEO_SHARES, &s.video_share, "vshare") } else { (SHARES, &s.share, "share") };
            list.iter().map(|(l, v)| check(format!("{prefix}:{v}"), *l, current == v)).collect()
        }
        "effects" => {
            let names: Vec<&str> = jiffshot_doc::effects::catalog(jiffshot_doc::Rgba::BLACK).into_iter().map(|(n, _)| n).collect();
            let on = |name: &str| s.capture_effects.iter().any(|c| c == name);
            let effect = |name: &str| MenuItem { keep_open: true, ..check(format!("fx:{name}"), name, on(name)) };
            let mut items = Vec::new();
            for (group, effects) in EFFECT_GROUPS {
                let effects: Vec<&str> = effects.iter().copied().filter(|e| names.contains(e)).collect();
                match effects.as_slice() {
                    [] => {}
                    [one] => items.push(MenuItem { label: (*group).into(), ..effect(one) }),
                    many => items.push(MenuItem { checked: many.iter().any(|e| on(e)), ..submenu(*group, many.iter().map(|e| effect(e)).collect()) }),
                }
            }
            let grouped: Vec<&str> = EFFECT_GROUPS.iter().flat_map(|(_, e)| e.iter().copied()).collect();
            items.extend(names.iter().filter(|n| !grouped.contains(n)).map(|n| effect(n)));
            items.push(separator());
            items.push(MenuItem { enabled: !s.capture_effects.is_empty(), ..item("fx-last", "Remove last effect") });
            items.push(check("fx-none", "None", s.capture_effects.is_empty()));
            items
        }
        "presets" => {
            let mut items = Vec::new();
            if s.presets.is_empty() {
                items.push(MenuItem { enabled: false, ..item("", "No presets yet") });
            }
            for (i, p) in s.presets.iter().enumerate() {
                let label = match &p.hotkey {
                    Some(k) => format!("{}\t{k}", p.name),
                    None => p.name.clone(),
                };
                items.push(item(format!("preset:{i}"), label));
            }
            items.push(separator());
            items.push(item("step", "Step Capture"));
            items.push(separator());
            items.push(item("panel:new-preset", "New preset from these settings…"));
            if !s.presets.is_empty() {
                items.push(submenu("Edit", s.presets.iter().enumerate().map(|(i, p)| item(format!("panel:edit-preset:{i}"), p.name.clone())).collect()));
                items.push(submenu("Delete", s.presets.iter().enumerate().map(|(i, p)| item(format!("preset-delete:{i}"), p.name.clone())).collect()));
            }
            items.push(item("presets-import", "Import presets…"));
            items.push(MenuItem { enabled: !s.presets.is_empty(), ..item("presets-export", "Export presets…") });
            items
        }
        "new-preset" => vec![item("panel:new-preset", "New preset from these settings…"), item("presets-import", "Import presets…")],
        "file" => vec![
            item("panel:prefs", "Preferences…"),
            separator(),
            item("panel:new-preset", "New preset from these settings…"),
            item("presets-import", "Import Presets…"),
            item("presets-export", "Export Presets…"),
            separator(),
            item("open-editor", "Open Jiffshot Editor"),
            item("open-library", "Open Library Folder"),
            separator(),
            item("close", "Close Window\tEsc"),
            item("exit", "Exit Jiffshot"),
        ],
        "edit" => vec![item("reset", "Reset this tab's options"), separator(), item("panel:prefs", "Preferences…")],
        "view" => {
            let mut items: Vec<MenuItem> = TABS.iter().enumerate().map(|(t, name)| check(format!("tab:{t}"), format!("{name}\tCtrl+{}", t + 1), t == tab)).collect();
            items.push(separator());
            items.push(check("widget", "Show Capture Widget", s.show_widget));
            items.push(item("window:topmost", "Keep on Top"));
            items
        }
        "help" => vec![item("panel:shortcuts", "Keyboard Shortcuts"), separator(), item("window:about", "About Jiffshot")],
        _ => Vec::new(),
    }
}

/// What a pick asks for beyond changing the settings.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    /// Nothing was picked (an unknown id).
    Nothing,
    /// The settings changed; save them.
    Changed,
    /// Show this panel.
    Panel(String),
    RunPreset(usize),
    StepCapture,
    /// Replace the presets (one was deleted).
    Presets(Vec<Preset>),
    ImportPresets,
    ExportPresets,
    OpenEditor,
    OpenLibrary,
    Close,
    Exit,
}

/// Applies a picked menu id to `s`.
pub fn pick(s: &mut Settings, id: &str) -> Pick {
    let (verb, arg) = id.split_once(':').unwrap_or((id, ""));
    match verb {
        "sel" => {
            if s.capture_window.selection == "fixed" {
                s.capture_window.fixed = true;
            }
            s.capture_window.selection = arg.into();
            Pick::Changed
        }
        "vsel" => {
            s.capture_window.video_selection = arg.into();
            Pick::Changed
        }
        "share" => {
            s.share = arg.into();
            Pick::Changed
        }
        "vshare" => {
            s.video_share = arg.into();
            Pick::Changed
        }
        "fx" => {
            match s.capture_effects.iter().position(|c| c == arg) {
                Some(i) => {
                    s.capture_effects.remove(i);
                }
                None => s.capture_effects.push(arg.into()),
            }
            Pick::Changed
        }
        "fx-last" => {
            s.capture_effects.pop();
            Pick::Changed
        }
        "fx-none" => {
            s.capture_effects.clear();
            Pick::Changed
        }
        "tab" => {
            s.capture_window.tab = arg.parse::<usize>().unwrap_or(1).min(2);
            Pick::Changed
        }
        "reset" => {
            reset(s, s.capture_window.tab);
            Pick::Changed
        }
        "widget" => {
            s.show_widget ^= true;
            Pick::Changed
        }
        "panel" => Pick::Panel(arg.into()),
        "preset" => arg.parse().map_or(Pick::Nothing, Pick::RunPreset),
        "preset-delete" => match arg.parse::<usize>() {
            Ok(i) if i < s.presets.len() => {
                let mut presets = s.presets.clone();
                presets.remove(i);
                Pick::Presets(presets)
            }
            _ => Pick::Nothing,
        },
        "step" => Pick::StepCapture,
        "presets-import" => Pick::ImportPresets,
        "presets-export" => Pick::ExportPresets,
        "open-editor" => Pick::OpenEditor,
        "open-library" => Pick::OpenLibrary,
        "close" => Pick::Close,
        "exit" => Pick::Exit,
        _ => Pick::Nothing,
    }
}

// ---- option panels --------------------------------------------------------

/// One row of an option panel.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Row {
    Number { label: String, value: i64, min: i64, max: i64 },
    Check { label: String, value: bool },
    Choice { label: String, choices: Vec<String>, value: usize },
    Text { label: String, value: String },
    /// Read-only text.
    Note { text: String },
    /// A folder path, with a button to choose one.
    Folder { label: String, value: String },
}

pub fn number(label: &str, value: i64, min: i64, max: i64) -> Row {
    Row::Number { label: label.into(), value, min, max }
}

pub fn check_row(label: &str, value: bool) -> Row {
    Row::Check { label: label.into(), value }
}

pub fn choice(label: &str, choices: &[&str], value: usize) -> Row {
    Row::Choice { label: label.into(), choices: choices.iter().map(|c| c.to_string()).collect(), value }
}

pub fn text(label: &str, value: impl Into<String>) -> Row {
    Row::Text { label: label.into(), value: value.into() }
}

pub fn note(text: impl Into<String>) -> Row {
    Row::Note { text: text.into() }
}

/// A small panel of options behind a gear, a › button or a menu item.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub rows: Vec<Row>,
    /// The confirm button's text; none for a read-only panel.
    pub ok: Option<&'static str>,
}

fn panel_of(id: &str, title: &str, rows: Vec<Row>) -> Option<Panel> {
    Some(Panel { id: id.into(), title: title.into(), rows, ok: Some("OK") })
}

/// The panel named `id`: "selection", "delay", "webcam", "microphone",
/// "cursor", "draw", "prefs", "new-preset", "edit-preset:N", "shortcuts".
/// `runs_at_startup` and `capture_key` come from the capture side.
pub fn panel(s: &Settings, id: &str, runs_at_startup: bool, capture_key: &str) -> Option<Panel> {
    let tab = s.capture_window.tab.min(2);
    match id {
        "selection" if tab == 1 && matches!(s.capture_window.selection.as_str(), "image" | "fixed") => {
            let (x, y) = s.capture_window.fixed_at.unwrap_or((0, 0));
            panel_of(id, "Region options", vec![
                check_row("Fixed region", fixed(s)),
                number("Width", s.fixed_size.0 as i64, 16, 16384),
                number("Height", s.fixed_size.1 as i64, 16, 16384),
                check_row("Set position", s.capture_window.fixed_at.is_some()),
                number("X", x as i64, -32768, 32767),
                number("Y", y as i64, -32768, 32767),
                check_row("Keep the pointer as a movable object", s.cursor_as_object),
            ])
        }
        "selection" if tab == 1 => panel_of(id, "Selection options", vec![check_row("Keep the pointer as a movable object", s.cursor_as_object)]),
        "selection" => {
            let fps = FPS.iter().position(|&f| f == s.record_fps).unwrap_or(2);
            panel_of(id, "Recording options", vec![choice("Frames a second", &["15", "24", "30", "60"], fps), number("Countdown (seconds)", s.countdown as i64, 0, 10)])
        }
        "delay" => {
            let kind = TIMINGS.iter().position(|t| *t == timing(s)).unwrap_or(0);
            let seconds = if s.delay_seconds > 0 { s.delay_seconds } else { s.capture_window.delay.max(1) };
            panel_of(id, "Time delay", vec![
                choice("Type", &["Delay", "Interval", "Schedule"], kind),
                number("Delay (seconds)", seconds as i64, 1, 600),
                number("Interval (seconds)", s.interval_seconds.max(3) as i64, 3, 86_400),
                check_row("Skip identical captures", s.interval_skip_identical),
                text("Daily at", s.schedule_default.clone().unwrap_or_else(|| "09:00".into())),
            ])
        }
        "webcam" => {
            let current = s.webcam.clone().unwrap_or_else(|| s.capture_window.webcam_corner.clone());
            let corner = CORNERS.iter().position(|c| *c == current).unwrap_or(3);
            panel_of(id, "Webcam", vec![
                choice("Corner", &CORNER_NAMES, corner),
                choice("Shape", &["Circle", "Rectangle"], (s.webcam_shape == "rectangle") as usize),
                number("Size (% of height)", (s.webcam_size * 100.0).round() as i64, 10, 50),
            ])
        }
        "microphone" => panel_of(id, "Microphone", vec![check_row("Remove background noise", s.noise_gate)]),
        "cursor" => panel_of(id, "Cursor effects", vec![check_row("Highlight cursor", s.cursor_highlight), check_row("Click animation", s.click_rings)]),
        "draw" => panel_of(id, "Screen Draw", vec![number("Fade after (s, 0 keeps)", s.draw_fade_seconds as i64, 0, 60)]),
        "prefs" => {
            let fps = FPS.iter().position(|&f| f == s.record_fps).unwrap_or(2);
            panel_of(id, "Capture Preferences", vec![
                check_row("Run when Windows starts", runs_at_startup),
                check_row("Show the Capture Widget", s.show_widget),
                number("Recording countdown (s)", s.countdown as i64, 0, 10),
                choice("Recording frame rate", &["15", "24", "30", "60"], fps),
            ])
        }
        "new-preset" => Some(Panel { ok: Some("Save"), ..panel_of(id, "New preset from these settings", vec![text("Name", ""), text("Hotkey", "")])? }),
        "shortcuts" => {
            let key = key_name(capture_key);
            Some(Panel {
                id: id.into(),
                title: "Keyboard shortcuts".into(),
                ok: None,
                rows: [
                    format!("{key}: capture (region, window or screen)"),
                    "Ctrl+Print Screen: full screen".into(),
                    "Shift+Print Screen: repeat the last region".into(),
                    "While selecting: arrow keys nudge; Shift, Ctrl, Ctrl+Shift lock square, 16:9, 4:3; M hides the magnifier; F1 shows all keys; Enter captures; Esc cancels".into(),
                    format!("While recording: {}", crate::recorder::keys_text("; ")),
                    "In this window: Enter captures; Ctrl+1, 2, 3 switch tabs; Esc closes".into(),
                ]
                .into_iter()
                .map(note)
                .collect(),
            })
        }
        _ => {
            let i: usize = id.strip_prefix("edit-preset:")?.parse().ok()?;
            let p = s.presets.get(i)?;
            Some(Panel { ok: Some("Save"), ..panel_of(id, "Edit preset", vec![text("Name", p.name.clone()), text("Hotkey", p.hotkey.clone().unwrap_or_default())])? })
        }
    }
}

/// What applying a panel asks for beyond changing the settings.
#[derive(Clone, Debug, PartialEq)]
pub enum Applied {
    /// The settings changed; save them.
    Changed,
    /// The settings changed, and a schedule that is on moves to this time.
    MoveSchedule(String),
    /// Replace the presets.
    Presets(Vec<Preset>),
    /// Run at startup on or off (the settings may have changed too).
    Startup(bool),
    /// Tell the user why nothing was applied.
    Problem(String),
}

pub fn int(values: &[Value], i: usize) -> i64 {
    values.get(i).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)).or_else(|| v.as_bool().map(i64::from))).unwrap_or(0)
}

pub fn on(values: &[Value], i: usize) -> bool {
    values.get(i).and_then(|v| v.as_bool().or_else(|| v.as_i64().map(|n| n != 0))).unwrap_or(false)
}

pub fn string(values: &[Value], i: usize) -> String {
    values.get(i).and_then(Value::as_str).unwrap_or("").trim().to_owned()
}

/// Applies the values of panel `id` (one per row, in order: numbers,
/// booleans, choice indexes, strings) to `s`.
pub fn apply(s: &mut Settings, id: &str, values: &[Value], runs_at_startup: bool) -> Applied {
    let tab = s.capture_window.tab.min(2);
    let clamp = |i: usize, min: i64, max: i64| int(values, i).clamp(min, max);
    match id {
        "selection" if tab == 1 && values.len() == 7 => {
            s.capture_window.selection = "image".into();
            s.capture_window.fixed = on(values, 0);
            s.fixed_size = (clamp(1, 16, 16384) as i32, clamp(2, 16, 16384) as i32);
            s.capture_window.fixed_at = on(values, 3).then(|| (clamp(4, -32768, 32767) as i32, clamp(5, -32768, 32767) as i32));
            s.cursor_as_object = on(values, 6);
            Applied::Changed
        }
        "selection" if tab == 1 => {
            s.cursor_as_object = on(values, 0);
            Applied::Changed
        }
        "selection" => {
            s.record_fps = FPS[clamp(0, 0, 3) as usize];
            s.countdown = clamp(1, 0, 10) as u32;
            Applied::Changed
        }
        "delay" => {
            let kind = TIMINGS[clamp(0, 0, 2) as usize];
            let at = string(values, 4);
            let valid = crate::tray::minutes_until(&at).is_some();
            if kind == "schedule" && !valid {
                return Applied::Problem("Type the time as HH:MM, for example 09:30.".into());
            }
            s.capture_window.timing = kind.into();
            s.capture_window.delay = clamp(1, 1, 600) as u32;
            s.interval_seconds = clamp(2, 3, 86_400) as u32;
            s.interval_skip_identical = on(values, 3);
            if valid {
                s.schedule_default = Some(at.clone());
            }
            // The panel turns the switch on, as the kind chosen.
            s.delay_seconds = if kind == "delay" { s.capture_window.delay } else { 0 };
            s.capture_window.timer = kind != "delay";
            if kind == "schedule" && s.schedule.is_some() {
                Applied::MoveSchedule(at)
            } else {
                Applied::Changed
            }
        }
        "webcam" => {
            let corner = CORNERS[clamp(0, 0, 3) as usize].to_owned();
            if s.webcam.is_some() {
                s.webcam = Some(corner.clone());
            }
            s.capture_window.webcam_corner = corner;
            s.webcam_shape = if int(values, 1) == 1 { "rectangle" } else { "circle" }.into();
            s.webcam_size = clamp(2, 10, 50) as f32 / 100.0;
            Applied::Changed
        }
        "microphone" => {
            s.noise_gate = on(values, 0);
            Applied::Changed
        }
        "cursor" => {
            s.cursor_highlight = on(values, 0);
            s.click_rings = on(values, 1);
            Applied::Changed
        }
        "draw" => {
            s.draw_fade_seconds = clamp(0, 0, 60) as u32;
            Applied::Changed
        }
        "prefs" => {
            s.show_widget = on(values, 1);
            s.countdown = clamp(2, 0, 10) as u32;
            s.record_fps = FPS[clamp(3, 0, 3) as usize];
            if on(values, 0) != runs_at_startup {
                Applied::Startup(on(values, 0))
            } else {
                Applied::Changed
            }
        }
        _ if id == "new-preset" || id.starts_with("edit-preset:") => {
            let (name, hotkey) = (string(values, 0), string(values, 1));
            if name.is_empty() {
                return Applied::Problem("Type a name for the preset.".into());
            }
            if !hotkey.is_empty() && crate::tray::parse_hotkey(&hotkey).is_none() {
                return Applied::Problem("Hotkeys look like Ctrl+Shift+1, Alt+F9 or PrtScn.".into());
            }
            let hotkey = Some(hotkey).filter(|k| !k.is_empty());
            let mut presets = s.presets.clone();
            match id.strip_prefix("edit-preset:").and_then(|i| i.parse::<usize>().ok()) {
                Some(i) if i < presets.len() => presets[i] = Preset { name, hotkey, ..presets[i].clone() },
                Some(_) => return Applied::Problem("That preset is gone.".into()),
                None => {
                    let preset = new_preset(s, name, hotkey);
                    match presets.iter().position(|p| p.name == preset.name) {
                        Some(i) => presets[i] = preset,
                        None => presets.push(preset),
                    }
                }
            }
            Applied::Presets(presets)
        }
        _ => Applied::Problem(format!("Unknown panel {id}")),
    }
}

/// The current tab's setup as a preset.
pub fn new_preset(s: &Settings, name: String, hotkey: Option<String>) -> Preset {
    let tab = s.capture_window.tab.min(2);
    Preset {
        name,
        mode: mode_of(s, tab),
        delay: s.delay_seconds,
        cursor: s.include_cursor,
        hotkey,
        effects: if tab == 2 { Vec::new() } else { s.capture_effects.clone() },
        share: Some(if tab == 2 { s.video_share.clone() } else { s.share.clone() }),
    }
}

/// Merges presets read from a file: same names are replaced.
pub fn import_presets(current: &[Preset], json: &[u8]) -> Option<Vec<Preset>> {
    let more: Vec<Preset> = serde_json::from_slice(json).ok()?;
    let mut presets = current.to_vec();
    for p in more {
        presets.retain(|q| q.name != p.name);
        presets.push(p);
    }
    Some(presets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        let mut s = Settings { fixed_size: (800, 600), delay_seconds: 4, record_audio: "both".into(), webcam: Some("top-left".into()), share: "email".into(), ..Settings::default() };
        s.capture_effects = vec!["Border".into(), "Shadow".into()];
        s.capture_window.selection = "fixed".into();
        s
    }

    #[test]
    fn fields_show_the_current_choices() {
        let s = settings();
        assert_eq!(field_text(&s, 1, Field::Selection), "Region (800 × 600)");
        assert_eq!(field_text(&s, 1, Field::Effects), "Border, Shadow");
        assert_eq!(field_text(&s, 1, Field::Share), "Email");
        assert_eq!(field_text(&s, 2, Field::Selection), "Region");
        let mut m = s.clone();
        m.capture_window.selection = "multi".into();
        assert_eq!((field_text(&m, 1, Field::Selection), mode_of(&m, 1)), ("Multiple Area".to_owned(), "multi".to_owned()));
    }

    #[test]
    fn switches_flip_and_remember() {
        let mut s = settings();
        assert!(switch_on(&s, Switch::Webcam));
        flip(&mut s, Switch::Webcam);
        assert_eq!((s.webcam.as_deref(), s.capture_window.webcam_corner.as_str()), (None, "top-left"));
        flip(&mut s, Switch::Webcam);
        assert_eq!(s.webcam.as_deref(), Some("top-left"), "the corner comes back");
        flip(&mut s, Switch::SystemAudio);
        assert_eq!(s.record_audio, "microphone");
        flip(&mut s, Switch::Delay);
        assert_eq!((s.delay_seconds, s.capture_window.delay), (0, 4));
        flip(&mut s, Switch::Delay);
        assert_eq!(s.delay_seconds, 4, "the delay comes back");
    }

    #[test]
    fn modes_and_the_capture_button_follow_the_tab_and_timer() {
        let mut s = settings();
        assert_eq!(mode_of(&s, 1), "fixed");
        assert_eq!(mode_of(&s, 0), "all");
        s.capture_window.video_selection = "fullscreen".into();
        assert_eq!(mode_of(&s, 2), "video-fullscreen");
        s.capture_window.timing = "interval".into();
        s.delay_seconds = 0;
        flip(&mut s, Switch::Delay);
        assert_eq!(switch_label(&s, Switch::Delay), "Interval (10 s)");
        assert_eq!(capture_button(&s, 1, "PrtScn", false).0, "Start");
        assert_eq!(capture_button(&s, 1, "PrtScn", true).0, "Stop");
        s.capture_window.timing = "schedule".into();
        assert_eq!(capture_button(&s, 1, "PrtScn", false), ("Schedule", "Daily at 09:00".to_owned()));
        s.capture_window.timer = false;
        assert_eq!(capture_button(&s, 1, "PrtScn", false), ("Capture", "Print Screen".to_owned()));
    }

    #[test]
    fn the_view_lists_each_tabs_rows() {
        let mut s = settings();
        s.capture_window.tab = 0;
        assert_eq!(view(&s, "PrtScn", false).fields.iter().map(|f| f.id).collect::<Vec<_>>(), ["share"]);
        s.capture_window.tab = 2;
        let v = view(&s, "PrtScn", false);
        assert_eq!(v.switches.len(), 6);
        assert_eq!(v.switches[0].more, Some("webcam"));
        assert_eq!(v.fields[0].gear, Some("selection"));
        assert!(serde_json::to_string(&v).unwrap().contains("\"capture\":\"Capture\""));
    }

    #[test]
    fn picks_change_the_settings() {
        let mut s = settings();
        assert_eq!(pick(&mut s, "sel:multi"), Pick::Changed);
        assert!(s.capture_window.fixed, "the older fixed setting becomes Region's fixed option");
        assert_eq!(pick(&mut s, "fx:Border"), Pick::Changed);
        assert_eq!(s.capture_effects, ["Shadow"]);
        assert_eq!(pick(&mut s, "fx:Tilt"), Pick::Changed);
        assert_eq!(pick(&mut s, "fx-last"), Pick::Changed);
        assert_eq!(s.capture_effects, ["Shadow"]);
        assert_eq!(pick(&mut s, "tab:2"), Pick::Changed);
        assert_eq!(pick(&mut s, "panel:webcam"), Pick::Panel("webcam".into()));
        assert_eq!(pick(&mut s, "preset-delete:0"), Pick::Presets(s.presets[1..].to_vec()));
        assert_eq!(pick(&mut s, "nonsense"), Pick::Nothing);
    }

    #[test]
    fn effects_menu_groups_and_keeps_open() {
        let s = settings();
        let items = menu(&s, "effects");
        let edges = items.iter().find(|i| i.label == "Edges").unwrap();
        assert!(edges.items.len() >= 4 && edges.items.iter().all(|i| i.keep_open));
        assert!(items.iter().find(|i| i.label == "Border").unwrap().checked);
        assert!(items.iter().any(|i| i.id == "fx-none"));
    }

    #[test]
    fn panels_round_trip_their_values() {
        let mut s = settings();
        s.capture_window.tab = 1;
        let p = panel(&s, "selection", false, "PrtScn").unwrap();
        assert_eq!(p.rows.len(), 7);
        let values: Vec<Value> = vec![true.into(), 1024.into(), 768.into(), false.into(), 0.into(), 0.into(), true.into()];
        assert_eq!(apply(&mut s, "selection", &values, false), Applied::Changed);
        assert_eq!((s.fixed_size, s.capture_window.selection.as_str(), s.capture_window.fixed), ((1024, 768), "image", true));

        let bad: Vec<Value> = vec![2.into(), 3.into(), 10.into(), true.into(), "nine".into()];
        assert!(matches!(apply(&mut s, "delay", &bad, false), Applied::Problem(_)));
        let good: Vec<Value> = vec![1.into(), 3.into(), 30.into(), true.into(), "09:30".into()];
        assert_eq!(apply(&mut s, "delay", &good, false), Applied::Changed);
        assert_eq!((s.capture_window.timing.as_str(), s.capture_window.timer, s.interval_seconds), ("interval", true, 30));

        let preset: Vec<Value> = vec!["Mine".into(), "Ctrl+Shift+9".into()];
        match apply(&mut s, "new-preset", &preset, false) {
            Applied::Presets(p) => assert_eq!(p.last().map(|p| (p.name.as_str(), p.hotkey.as_deref())), Some(("Mine", Some("Ctrl+Shift+9")))),
            other => panic!("{other:?}"),
        }
        let nameless: Vec<Value> = vec!["".into(), "".into()];
        assert!(matches!(apply(&mut s, "new-preset", &nameless, false), Applied::Problem(_)));
        assert_eq!(apply(&mut s, "prefs", &[true.into(), false.into(), 3.into(), 1.into()], false), Applied::Startup(true));
    }

    #[test]
    fn presets_import_merges_by_name() {
        let current = Settings::default().presets;
        let json = serde_json::to_vec(&[Preset { name: current[0].name.clone(), mode: "video".into(), ..Preset::default() }]).unwrap();
        let merged = import_presets(&current, &json).unwrap();
        assert_eq!(merged.len(), current.len());
        assert_eq!(merged.last().unwrap().mode, "video");
        assert!(import_presets(&current, b"not json").is_none());
    }
}

//! The one Preferences dialog: Capture, Editor, Library, Share and Hotkeys
//! tabs. Each tab is a list of rows, as in the Capture window's panels
//! (`jiffshot_workflow::panel::Row`); the dialog sends back one value per
//! row and tab, and this module applies them: the capture side's tabs
//! through `jiffshot_workflow::prefs`, the editor's through its own actions.

use crate::app::App;
use jiffshot_workflow::panel::{check_row, choice, note, on, string, text, Row};
use jiffshot_workflow::{capture, prefs};
use serde::Serialize;
use serde_json::Value;

pub const THEMES: [&str; 3] = ["Dark", "Light", "System"];
/// Export formats: label, extension ("" for the one used last).
const FORMATS: [(&str, &str); 7] =
    [("The one used last", ""), ("PNG", "png"), ("JPEG", "jpg"), ("PDF", "pdf"), ("BMP", "bmp"), ("GIF", "gif"), ("WebP", "webp")];

#[derive(Serialize)]
struct Tab {
    id: &'static str,
    title: &'static str,
    rows: Vec<Row>,
}

#[derive(Serialize)]
struct Dialog {
    /// The tab to open on.
    tab: String,
    tabs: Vec<Tab>,
}

/// The dialog's JSON, opening on `tab`.
pub fn dialog(app: &App, tab: &str) -> String {
    let capture_settings = jiffshot_workflow::settings_snapshot();
    let s = &app.settings;
    let theme = THEMES.iter().position(|t| Some(*t) == s.theme.as_deref()).unwrap_or(0);
    let format = FORMATS.iter().position(|(_, ext)| *ext == s.export_format.as_deref().unwrap_or("")).unwrap_or(0);
    let mut share = vec![
        text("FTP destination (ftp://user@host/folder)", s.ftp.clone().unwrap_or_default()),
        note("The FTP password is the \u{201C}Jiffshot FTP\u{201D} entry in Windows Credential Manager."),
    ];
    if s.programs.is_empty() {
        share.push(note("No programs yet; Share \u{25B8} Add program\u{2026} in the editor adds one."));
    } else {
        share.push(note("Programs captures can be sent to (clear one to remove it):"));
        share.extend(s.programs.iter().map(|(name, _)| check_row(name, true)));
    }
    let dialog = Dialog {
        tab: tab.to_owned(),
        tabs: vec![
            Tab { id: "capture", title: "Capture", rows: prefs::capture(&capture_settings, capture::runs_at_startup()) },
            Tab {
                id: "editor",
                title: "Editor",
                rows: vec![
                    choice("Color theme (every Jiffshot window)", &THEMES, theme),
                    check_row("Show tool names in the toolbar", s.layout.labels),
                    check_row("Show the side bar (Quick Styles and properties)", s.layout.side_bar),
                    check_row("Show the Recent Captures tray", s.layout.tray),
                    choice("Export as", &FORMATS.map(|(label, _)| label), format),
                ],
            },
            Tab {
                id: "library",
                title: "Library",
                rows: vec![
                    Row::Folder { label: "Library folder".into(), value: app.library.root.display().to_string() },
                    note("New captures are saved here, and the editor and the tray's Recent captures list them."),
                ],
            },
            Tab { id: "share", title: "Share", rows: share },
            Tab { id: "hotkeys", title: "Hotkeys", rows: prefs::hotkeys(&capture_settings, &crate::capture_window::capture_key()) },
        ],
    };
    serde_json::to_string(&dialog).unwrap_or_default()
}

/// Applies the dialog's values (`{"capture": [...], "editor": [...], …}`).
pub fn apply(app: &mut App, json: &str) {
    let Ok(Value::Object(tabs)) = serde_json::from_str::<Value>(json) else { return };
    let values = |id: &str| -> Vec<Value> { tabs.get(id).and_then(Value::as_array).cloned().unwrap_or_default() };
    apply_capture(&values("capture"), &values("hotkeys"), app);
    apply_editor(app, &values("editor"), &values("library"), &values("share"));
}

fn apply_capture(values: &[Value], hotkeys: &[Value], app: &mut App) {
    let mut s = jiffshot_workflow::settings_snapshot();
    if !values.is_empty() {
        if let Some(on) = prefs::apply_capture(&mut s, values, capture::runs_at_startup()) {
            capture::set_run_at_startup(on);
        }
        capture::apply(s.clone());
    }
    if !hotkeys.is_empty() {
        match prefs::apply_hotkeys(&s.presets, hotkeys) {
            Ok(presets) if presets != s.presets => capture::set_presets(presets),
            Ok(_) => {}
            Err(problem) => app.toast("Hotkeys not changed", &problem, true),
        }
    }
}

fn apply_editor(app: &mut App, editor: &[Value], library: &[Value], share: &[Value]) {
    if !editor.is_empty() {
        let theme = THEMES[(editor[0].as_u64().unwrap_or(0) as usize).min(2)];
        if app.settings.theme.as_deref() != Some(theme) {
            app.run(&format!("theme:{theme}"));
        }
        let (labels, side_bar, tray) = (on(editor, 1), on(editor, 2), on(editor, 3));
        app.layout(|l| {
            l.labels = labels;
            l.side_bar = side_bar;
            l.tray = tray;
        });
        let format = FORMATS[(editor.get(4).and_then(Value::as_u64).unwrap_or(0) as usize).min(FORMATS.len() - 1)].1;
        if app.settings.export_format.as_deref().unwrap_or("") != format {
            app.run(&format!("export-format:{format}"));
        }
    }
    let folder = string(library, 0);
    if !folder.is_empty() && std::path::Path::new(&folder) != app.library.root {
        app.run(&format!("library-folder:{folder}"));
    }
    if !share.is_empty() {
        let ftp = string(share, 0);
        if ftp != app.settings.ftp.clone().unwrap_or_default() {
            app.run(&format!("ftp:{ftp}"));
        }
        // Rows: FTP, its note, a note, then a check per program.
        let removed: Vec<String> =
            app.settings.programs.iter().enumerate().filter(|(i, _)| share.get(3 + i).is_some_and(|v| v == &Value::Bool(false))).map(|(_, (n, _))| n.clone()).collect();
        for name in removed {
            app.run(&format!("program-remove:{name}"));
        }
    }
}

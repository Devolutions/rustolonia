//! The Capture window's model. What the window shows and offers comes from
//! `jiffshot_workflow::panel` as JSON ([`View`], menus, option panels); the
//! window sends back actions, and every change goes to the capture side,
//! which owns the settings ([`capture::apply`]).
//!
//! Actions: `tab:N`, `switch:ID`, `menu:NAME`, `pick:ID`, `panel:ID`,
//! `apply:ID:[values]`, `capture`, `editor`, `closed`.

use crate::app;
use crate::vm::{CaptureViewModel, CaptureViewModelSink};
use jiffshot_workflow::panel::{self, Applied, Pick};
use jiffshot_workflow::settings::Settings;
use jiffshot_workflow::{capture, TrayAction, TrayState};
use rustolonia::{AppScope, Result};
use std::cell::RefCell;

struct State {
    sink: Option<CaptureViewModelSink>,
    /// The settings being edited: the capture side's, plus changes not yet
    /// echoed back.
    settings: Settings,
    capture_key: String,
    timelapse: bool,
    window: Option<rustolonia::Window>,
    serial: u64,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        sink: None,
        settings: Settings::default(),
        capture_key: "PrtScn".into(),
        timelapse: false,
        window: None,
        serial: 0,
    });
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// Remembers the window mounted for this model (hidden until shown).
pub fn set_window(window: rustolonia::Window) {
    with(|s| s.window = Some(window));
}

/// Shows the window with the capture side's current settings, in front.
pub fn show(scope: &AppScope) {
    with(|s| s.settings = jiffshot_workflow::settings_snapshot());
    publish_view();
    let Some(window) = with(|s| s.window.clone()) else { return };
    let _ = window.show();
    if let Ok(hwnd) = scope.platform_handle(&window) {
        crate::winutil::bring_forward(hwnd);
    }
}

fn hide() {
    if let Some(window) = with(|s| s.window.clone()) {
        let _ = window.hide();
    }
}

/// The capture side changed something: show it.
pub fn refresh(state: &TrayState) {
    let tab = with(|s| {
        s.capture_key = state.capture_key.clone();
        s.timelapse = state.time_lapse;
        s.settings.capture_window.tab
    });
    let mut settings = jiffshot_workflow::settings_snapshot();
    // The tab is this window's; the capture side may not have it yet.
    settings.capture_window.tab = tab;
    with(|s| s.settings = settings);
    publish_view();
}

fn publish_view() {
    let (sink, json) = with(|s| {
        let view = panel::view(&s.settings, &s.capture_key, s.timelapse);
        (s.sink.clone(), serde_json::to_string(&view).unwrap_or_default())
    });
    if let Some(sink) = sink {
        let _ = sink.set_view(json);
    }
}

/// Sends `json` as the next value of a "serial\x1fJSON" property.
fn send(set: impl Fn(&CaptureViewModelSink, String) -> Result<()>, json: String) {
    let (sink, serial) = with(|s| {
        s.serial += 1;
        (s.sink.clone(), s.serial)
    });
    if let Some(sink) = sink {
        let _ = set(&sink, format!("{serial}\u{1f}{json}"));
    }
}

fn problem(text: &str) {
    send(|k, v| k.set_problem(v), text.to_owned());
}

/// Saves the edited settings on the capture side and shows them.
fn changed() {
    capture::apply(with(|s| s.settings.clone()));
    publish_view();
}

fn set_presets(presets: Vec<jiffshot_workflow::settings::Preset>) {
    with(|s| s.settings.presets = presets.clone());
    capture::set_presets(presets);
    publish_view();
}

/// The capture hotkey as the capture side last published it.
pub fn capture_key() -> String {
    with(|s| s.capture_key.clone())
}

fn open_panel(id: &str) {
    // The capture preferences are a tab of the one Preferences dialog.
    if id == "prefs" {
        crate::app::post(|app| app.open_prefs("capture"));
        return;
    }
    let (settings, key) = with(|s| (s.settings.clone(), s.capture_key.clone()));
    match panel::panel(&settings, id, capture::runs_at_startup(), &key) {
        Some(p) => send(|k, v| k.set_panel(v), serde_json::to_string(&p).unwrap_or_default()),
        None => problem("Nothing to set here."),
    }
}

fn picked(id: &str) {
    let result = with(|s| panel::pick(&mut s.settings, id));
    match result {
        Pick::Nothing => {}
        Pick::Changed => changed(),
        Pick::Panel(id) => open_panel(&id),
        Pick::RunPreset(i) => {
            hide();
            capture::run_preset(i);
        }
        Pick::StepCapture => {
            hide();
            capture::step_capture();
        }
        Pick::Presets(presets) => set_presets(presets),
        Pick::ImportPresets => app::post(|app| {
            app.pick_files("Import presets", false, "Presets", &["json"], |_, paths| {
                let Some(bytes) = paths.first().and_then(|p| std::fs::read(p).ok()) else { return };
                let current = with(|s| s.settings.presets.clone());
                match panel::import_presets(&current, &bytes) {
                    Some(presets) => set_presets(presets),
                    None => problem("That file has no presets in it."),
                }
            })
        }),
        Pick::ExportPresets => {
            let presets = with(|s| s.settings.presets.clone());
            app::post(move |app| {
                app.pick_save("Export presets", "Jiffshot presets.json", &[("Presets", "json")], move |_, path| {
                    let written = serde_json::to_vec_pretty(&presets).ok().and_then(|b| std::fs::write(&path, b).ok());
                    if written.is_none() {
                        problem("Could not write the file.");
                    }
                })
            });
        }
        Pick::OpenEditor => app::post(|app| app.show_editor()),
        Pick::OpenLibrary => capture::open_library(),
        Pick::Close => hide(),
        Pick::Exit => TrayAction::Exit.post(),
    }
}

fn applied(id: &str, values: &str) {
    let values: Vec<serde_json::Value> = serde_json::from_str(values).unwrap_or_default();
    let startup = capture::runs_at_startup();
    let result = with(|s| panel::apply(&mut s.settings, id, &values, startup));
    match result {
        Applied::Changed => changed(),
        Applied::MoveSchedule(at) => {
            changed();
            capture::move_schedule(at);
        }
        Applied::Presets(presets) => set_presets(presets),
        Applied::Startup(on) => {
            changed();
            capture::set_run_at_startup(on);
        }
        Applied::Problem(text) => problem(&text),
    }
}

fn act(action: &str) {
    let (verb, arg) = action.split_once(':').unwrap_or((action, ""));
    match verb {
        "tab" => picked(action),
        "switch" => {
            if let Some(switch) = panel::Switch::from_id(arg) {
                with(|s| panel::flip(&mut s.settings, switch));
                changed();
            }
        }
        "menu" => {
            let items = with(|s| panel::menu(&s.settings, arg));
            send(|k, v| k.set_menu(v), serde_json::to_string(&items).unwrap_or_default());
        }
        "pick" => picked(arg),
        "panel" => open_panel(arg),
        "apply" => {
            if let Some((id, values)) = arg.split_once(':') {
                applied(id, values);
            }
        }
        "capture" => {
            hide();
            capture::start(with(|s| s.settings.clone()));
        }
        "editor" => app::post(|app| app.show_editor()),
        _ => {}
    }
}

pub struct CaptureModel;

impl CaptureViewModel for CaptureModel {
    fn attach(&mut self, sink: CaptureViewModelSink) -> Result<()> {
        with(|s| {
            s.sink = Some(sink);
            s.settings = jiffshot_workflow::settings_snapshot();
        });
        publish_view();
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        with(|s| s.sink = None);
        Ok(())
    }

    fn set_action(&mut self, _value: String) -> Result<()> {
        Ok(())
    }

    fn run(&mut self, value: String) -> Result<()> {
        act(&value);
        Ok(())
    }
}

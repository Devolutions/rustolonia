//! The notification-area icon and its menu, on Rustolonia's Rust-built tray
//! ([`rustolonia::ShellTray`]). The switches show the [`TrayViewModelSink`]
//! values and the menu is rebuilt when Recent captures changes; left-click
//! captures.

use crate::vm::{TrayBinding, TrayViewModel, TrayViewModelSink};
use rustolonia::{AppScope, Effect, Result, ShellTray, TrayEvent, TrayMenuItem};

// Menu item ids. Recent captures are RECENT + their index.
const CAPTURE: u32 = 1;
const CAPTURE_DELAYED: u32 = 2;
const CAPTURE_SCREEN: u32 = 3;
const REPEAT_LAST: u32 = 4;
const CAPTURE_WINDOW: u32 = 5;
const SHOW_EDITOR: u32 = 6;
const SCROLLING: u32 = 7;
const SCROLLING_HORIZONTAL: u32 = 8;
const SCROLLING_BOTH: u32 = 9;
const PANORAMIC: u32 = 10;
const FREEHAND: u32 = 11;
const MULTIPLE_AREAS: u32 = 12;
const FIXED_SIZE: u32 = 13;
const MENU_CAPTURE: u32 = 14;
const GRAB_TEXT: u32 = 15;
const STEP_CAPTURE: u32 = 16;
const FROM_CLIPBOARD: u32 = 17;
const FROM_FILE: u32 = 18;
const RECORD_VIDEO: u32 = 19;
const TIME_LAPSE: u32 = 20;
const SCHEDULE: u32 = 21;
const COPY_TO_CLIPBOARD: u32 = 22;
const OPEN_IN_EDITOR: u32 = 23;
const INCLUDE_CURSOR: u32 = 24;
const SHOW_WIDGET: u32 = 25;
const RUN_AT_STARTUP: u32 = 26;
const OPEN_LIBRARY_FOLDER: u32 = 27;
const PREFERENCES: u32 = 28;
const EXIT: u32 = 29;
const RECENT: u32 = 100;

/// The switches: menu id, header, and the sink value it shows.
fn switches(sink: &TrayViewModelSink) -> [(u32, &'static str, rustolonia::Signal<bool>); 7] {
    [
        (TIME_LAPSE, "_Time-lapse", sink.time_lapse),
        (SCHEDULE, "Scheduled capture (_daily)", sink.schedule),
        (COPY_TO_CLIPBOARD, "Copy to clip_board", sink.copy_to_clipboard),
        (OPEN_IN_EDITOR, "Open in the edit_or", sink.open_in_editor),
        (INCLUDE_CURSOR, "Include the c_ursor", sink.include_cursor),
        (SHOW_WIDGET, "Show the Capture Wid_get", sink.show_widget),
        (RUN_AT_STARTUP, "Run when Windows _starts", sink.run_at_startup),
    ]
}

/// A recent capture's menu header: its file name.
fn recent_header(uri: &str) -> String {
    let path = uri.strip_prefix("file:///").unwrap_or(uri);
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    crate::ui::percent_decode(name)
}

fn menu(sink: &TrayViewModelSink) -> Vec<TrayMenuItem> {
    let check = |id: u32| {
        let (_, header, signal) = switches(sink).into_iter().find(|(i, ..)| *i == id).expect("switch id");
        TrayMenuItem::check(id, header, signal.get_untracked())
    };
    let recent = sink.recent_captures.get_untracked();
    let recent_items = if recent.is_empty() {
        vec![TrayMenuItem::Command {
            id: RECENT + 99,
            header: "(none yet)".into(),
            enabled: false,
        }]
    } else {
        recent
            .iter()
            .enumerate()
            .map(|(i, uri)| TrayMenuItem::command(RECENT + i as u32, recent_header(uri)))
            .collect()
    };
    vec![
        TrayMenuItem::command(CAPTURE, "_Capture"),
        TrayMenuItem::command(CAPTURE_DELAYED, "Capture after the _delay"),
        TrayMenuItem::command(CAPTURE_SCREEN, "Capture the _screen under the pointer"),
        TrayMenuItem::Command {
            id: REPEAT_LAST,
            header: "_Repeat the last area".into(),
            enabled: sink.can_repeat.get_untracked(),
        },
        TrayMenuItem::Separator,
        TrayMenuItem::command(CAPTURE_WINDOW, "Capture _window…"),
        TrayMenuItem::command(SHOW_EDITOR, "Open Jiffshot _Editor"),
        TrayMenuItem::submenu("Recent ca_ptures", recent_items),
        TrayMenuItem::Separator,
        TrayMenuItem::submenu(
            "More capture _modes",
            vec![
                TrayMenuItem::command(SCROLLING, "_Scrolling"),
                TrayMenuItem::command(SCROLLING_HORIZONTAL, "Scrolling, _horizontal"),
                TrayMenuItem::command(SCROLLING_BOTH, "Scrolling, _both directions"),
                TrayMenuItem::command(PANORAMIC, "_Panoramic"),
                TrayMenuItem::command(FREEHAND, "_Freehand"),
                TrayMenuItem::command(MULTIPLE_AREAS, "_Multiple areas"),
                TrayMenuItem::command(FIXED_SIZE, "Fixed si_ze"),
                TrayMenuItem::command(MENU_CAPTURE, "M_enu (3 second countdown)"),
                TrayMenuItem::command(GRAB_TEXT, "_Grab text"),
                TrayMenuItem::command(STEP_CAPTURE, "S_tep Capture"),
                TrayMenuItem::Separator,
                TrayMenuItem::command(FROM_CLIPBOARD, "From the _clipboard"),
                TrayMenuItem::command(FROM_FILE, "From a _file…"),
            ],
        ),
        TrayMenuItem::command(RECORD_VIDEO, "Record _video"),
        TrayMenuItem::Separator,
        check(TIME_LAPSE),
        check(SCHEDULE),
        TrayMenuItem::Separator,
        check(COPY_TO_CLIPBOARD),
        check(OPEN_IN_EDITOR),
        check(INCLUDE_CURSOR),
        TrayMenuItem::Separator,
        check(SHOW_WIDGET),
        check(RUN_AT_STARTUP),
        TrayMenuItem::command(OPEN_LIBRARY_FOLDER, "Open the _library folder"),
        TrayMenuItem::command(PREFERENCES, "_Preferences…"),
        TrayMenuItem::Separator,
        TrayMenuItem::command(EXIT, "E_xit Jiffshot"),
    ]
}

fn clicked(binding: &TrayBinding, event: TrayEvent) {
    let sink = binding.sink;
    let id = match event {
        TrayEvent::Activate => CAPTURE,
        TrayEvent::MenuItem(id) => id,
    };
    if let Some((_, _, signal)) = switches(&sink).into_iter().find(|(i, ..)| *i == id) {
        let on = !signal.get_untracked();
        binding.model.run(|m| match id {
            TIME_LAPSE => m.set_time_lapse(on),
            SCHEDULE => m.set_schedule(on),
            COPY_TO_CLIPBOARD => m.set_copy_to_clipboard(on),
            OPEN_IN_EDITOR => m.set_open_in_editor(on),
            INCLUDE_CURSOR => m.set_include_cursor(on),
            SHOW_WIDGET => m.set_show_widget(on),
            _ => m.set_run_at_startup(on),
        });
        return;
    }
    if id >= RECENT {
        let recent = sink.recent_captures.get_untracked();
        if let Some(uri) = recent.get((id - RECENT) as usize).cloned() {
            binding.model.run(|m| m.open_recent_capture(uri));
        }
        return;
    }
    binding.model.run(|m: &mut dyn TrayViewModel| match id {
        CAPTURE => m.capture(),
        CAPTURE_DELAYED => m.capture_delayed(),
        CAPTURE_SCREEN => m.capture_screen(),
        REPEAT_LAST => m.repeat_last(),
        CAPTURE_WINDOW => m.capture_window(),
        SHOW_EDITOR => m.show_editor(),
        SCROLLING => m.scrolling(),
        SCROLLING_HORIZONTAL => m.scrolling_horizontal(),
        SCROLLING_BOTH => m.scrolling_both(),
        PANORAMIC => m.panoramic(),
        FREEHAND => m.freehand(),
        MULTIPLE_AREAS => m.multiple_areas(),
        FIXED_SIZE => m.fixed_size(),
        MENU_CAPTURE => m.menu_capture(),
        GRAB_TEXT => m.grab_text(),
        STEP_CAPTURE => m.step_capture(),
        FROM_CLIPBOARD => m.from_clipboard(),
        FROM_FILE => m.from_file(),
        RECORD_VIDEO => m.record_video(),
        OPEN_LIBRARY_FOLDER => m.open_library_folder(),
        PREFERENCES => m.preferences(),
        EXIT => m.exit(),
        _ => Ok(()),
    });
}

/// Shows the tray icon for `model`; it stays until the application exits.
pub fn mount(scope: &AppScope, model: impl TrayViewModel) -> Result<TrayBinding> {
    let binding = TrayBinding::new(model)?;
    let handler_binding = binding.clone();
    // The handler runs on the UI thread; the binding never leaves it.
    let handler_binding = crate::ui::UiOnly::new(handler_binding);
    let tray: ShellTray = scope.create_tray(move |event| {
        clicked(handler_binding.get(), event);
        Ok(())
    })?;
    tray.set_icon(&crate::ui::icon_file())?;
    tray.set_tool_tip("Jiffshot")?;
    let sink = binding.sink;
    // Every value the menu shows: a change rebuilds it.
    Effect::new(move || {
        sink.recent_captures.with(|_| ());
        sink.can_repeat.get();
        for (_, _, signal) in switches(&sink) {
            signal.get();
        }
        tray.set_menu(&menu(&sink))
    })?;
    Ok(binding)
}

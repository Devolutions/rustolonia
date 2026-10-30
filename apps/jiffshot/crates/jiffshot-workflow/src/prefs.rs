//! The capture side's tabs of the Preferences dialog, Capture and Hotkeys:
//! their rows (as in the Capture window's panels, see `panel.rs`) and how
//! the values the dialog sends back change the settings.

use crate::panel::{check_row, choice, int, note, number, on, string, text, Row, FPS};
use crate::settings::{Preset, Settings};
use serde_json::Value;

const AUDIO: [(&str, &str); 4] = [("system", "What the computer plays"), ("microphone", "The microphone"), ("both", "Both, mixed"), ("none", "No sound")];

/// The Capture tab.
pub fn capture(s: &Settings, runs_at_startup: bool) -> Vec<Row> {
    let fps = FPS.iter().position(|&f| f == s.record_fps).unwrap_or(2);
    let audio = AUDIO.iter().position(|(v, _)| *v == s.record_audio).unwrap_or(0);
    vec![
        check_row("Run Jiffshot when Windows starts", runs_at_startup),
        check_row("Show the Capture Widget at the top of the screen", s.show_widget),
        check_row("Copy captures to the clipboard", s.copy_to_clipboard),
        check_row("Open captures in the editor", s.open_in_editor),
        check_row("Keep the pointer as a movable object in the editor", s.cursor_as_object),
        number("Time delay (s)", s.delay_seconds as i64, 0, 60),
        number("Recording countdown (s)", s.countdown as i64, 0, 10),
        choice("Recording frame rate", &["15", "24", "30", "60"], fps),
        choice("Recording sound", &AUDIO.map(|(_, label)| label), audio),
        check_row("Draw the pointer into recordings", s.record_cursor),
        check_row("Offer Screen Draw while recording", s.screen_draw),
        number("Screen Draw marks fade after (s, 0 keeps them)", s.draw_fade_seconds as i64, 0, 60),
    ]
}

/// Applies the Capture tab; returns the Run-at-startup choice when it changed.
pub fn apply_capture(s: &mut Settings, values: &[Value], runs_at_startup: bool) -> Option<bool> {
    let clamp = |i: usize, min: i64, max: i64| int(values, i).clamp(min, max);
    s.show_widget = on(values, 1);
    s.copy_to_clipboard = on(values, 2);
    s.open_in_editor = on(values, 3);
    s.cursor_as_object = on(values, 4);
    s.delay_seconds = clamp(5, 0, 60) as u32;
    s.countdown = clamp(6, 0, 10) as u32;
    s.record_fps = FPS[clamp(7, 0, 3) as usize];
    s.record_audio = AUDIO[clamp(8, 0, 3) as usize].0.to_owned();
    s.record_cursor = on(values, 9);
    s.screen_draw = on(values, 10);
    s.draw_fade_seconds = clamp(11, 0, 60) as u32;
    (on(values, 0) != runs_at_startup).then(|| on(values, 0))
}

/// The Hotkeys tab: the fixed keys, then a hotkey per preset.
pub fn hotkeys(s: &Settings, capture_key: &str) -> Vec<Row> {
    let key = crate::panel::key_name(capture_key);
    let mut rows = vec![
        note(format!("{key}: capture (region, window or screen). Print Screen is used when it is free; another key while some other app holds it.")),
        note("Ctrl+Print Screen: full screen · Shift+Print Screen: repeat the last region"),
        note(format!("While recording: {}", crate::recorder::keys_text(", "))),
        note("Presets: a hotkey for each, for example Ctrl+Shift+1; empty for none."),
    ];
    rows.extend(s.presets.iter().map(|p| text(&p.name, p.hotkey.clone().unwrap_or_default())));
    if s.presets.is_empty() {
        rows.push(note("No presets yet; the Capture window's Presets menu makes them."));
    }
    rows
}

/// The presets with the Hotkeys tab's keys, or what is wrong with them.
pub fn apply_hotkeys(presets: &[Preset], values: &[Value]) -> Result<Vec<Preset>, String> {
    // The first four rows are notes.
    let mut out = presets.to_vec();
    for (i, p) in out.iter_mut().enumerate() {
        let key = string(values, 4 + i);
        if !key.is_empty() && crate::tray::parse_hotkey(&key).is_none() {
            return Err(format!("\u{201C}{key}\u{201D} (for {}) is not a hotkey Jiffshot understands; try something like Ctrl+Shift+1.", p.name));
        }
        p.hotkey = (!key.is_empty()).then_some(key);
    }
    let mut seen = std::collections::HashSet::new();
    if let Some(p) = out.iter().filter_map(|p| p.hotkey.as_ref().map(|k| (p, k.to_ascii_lowercase()))).find(|(_, k)| !seen.insert(k.clone())) {
        return Err(format!("{} is used by more than one preset.", p.0.hotkey.as_deref().unwrap_or("")));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_capture_tab_round_trips_and_reports_a_startup_change() {
        let mut s = Settings::default();
        let rows = capture(&s, false);
        let values: Vec<Value> = rows
            .iter()
            .map(|r| match r {
                Row::Check { value, .. } => json!(value),
                Row::Number { value, .. } => json!(value),
                Row::Choice { value, .. } => json!(value),
                _ => Value::Null,
            })
            .collect();
        let before = serde_json::to_value(&s).unwrap();
        assert_eq!(apply_capture(&mut s, &values, false), None);
        assert_eq!(serde_json::to_value(&s).unwrap(), before, "unchanged values change nothing");
        let mut changed = values.clone();
        changed[0] = json!(true);
        changed[7] = json!(3);
        changed[8] = json!(1);
        assert_eq!(apply_capture(&mut s, &changed, false), Some(true));
        assert_eq!((s.record_fps, s.record_audio.as_str()), (60, "microphone"));
    }

    #[test]
    fn preset_hotkeys_are_checked() {
        let presets = Settings::default().presets;
        let notes = vec![Value::Null; 4];
        let with = |keys: &[&str]| notes.iter().cloned().chain(keys.iter().map(|k| json!(k))).collect::<Vec<_>>();
        let set = apply_hotkeys(&presets, &with(&["Ctrl+Shift+1", ""])).unwrap();
        assert_eq!(set[0].hotkey.as_deref(), Some("Ctrl+Shift+1"));
        assert_eq!(set[1].hotkey, None);
        assert!(apply_hotkeys(&presets, &with(&["Banana", ""])).is_err());
        assert!(apply_hotkeys(&presets, &with(&["Ctrl+Shift+1", "ctrl+shift+1"])).is_err());
    }
}

//! Capture preferences: the `capture` section of Jiffshot's settings file
//! (see `jiffshot_capture::config`). The library folder is shared with the
//! editor and lives there too.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub copy_to_clipboard: bool,
    pub open_in_editor: bool,
    pub include_cursor: bool,
    pub delay_seconds: u32,
    /// Path to `jiffshot.exe`; `None` searches next to this
    /// executable and in the development tree.
    pub editor: Option<PathBuf>,
    /// Recording audio: "system" (what the speakers play), "microphone",
    /// "both" (mixed) or "none".
    pub record_audio: String,
    pub record_fps: u32,
    /// Silence the microphone between words.
    pub noise_gate: bool,
    /// Seconds counted down before recording starts (0: none).
    pub countdown: u32,
    pub cursor_highlight: bool,
    pub click_rings: bool,
    /// Webcam picture-in-picture corner ("bottom-right", …); off when unset.
    pub webcam: Option<String>,
    /// "circle" or "rectangle".
    pub webcam_shape: String,
    /// Height of the webcam picture as a share of the video's.
    pub webcam_size: f32,
    /// Screen Draw marks fade after this many seconds (0: they stay).
    pub draw_fade_seconds: u32,
    /// Size of a fixed-size capture.
    pub fixed_size: (i32, i32),
    /// Seconds between time-lapse captures.
    pub interval_seconds: u32,
    /// Time-lapse skips a capture identical to the one before.
    pub interval_skip_identical: bool,
    /// The pointer is drawn into recordings.
    pub record_cursor: bool,
    /// The recording panel offers Screen Draw.
    pub screen_draw: bool,
    /// Keep the pointer as a separate, movable object in the editor
    /// instead of drawing it into the image.
    pub cursor_as_object: bool,
    /// Effects applied to every image capture, by name (as in the editor's
    /// Effects list).
    pub capture_effects: Vec<String>,
    pub presets: Vec<Preset>,
    pub show_widget: bool,
    /// Daily capture time, "HH:MM", when on.
    pub schedule: Option<String>,
    /// The time the Scheduled capture menu item turns on.
    pub schedule_default: Option<String>,
    /// Area for scheduled captures (x, y, width, height); the screen under
    /// the pointer when unset.
    pub schedule_area: Option<(i32, i32, i32, i32)>,
    /// Where image captures go after saving: "none", "file", "clipboard",
    /// "email", "word", "powerpoint", "excel", "onenote", "paint",
    /// "program" or "print".
    pub share: String,
    /// The same, for recordings.
    pub video_share: String,
    pub capture_window: CaptureWindow,
}

/// What the Capture window shows and remembers between uses.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureWindow {
    /// 0 All-in-One, 1 Image, 2 Video.
    pub tab: usize,
    /// The Image tab's selection, as a preset mode name.
    pub selection: String,
    /// The Video tab's selection: "region", "window" or "fullscreen".
    pub video_selection: String,
    /// A Region selection is a fixed-size rectangle (`Settings::fixed_size`).
    pub fixed: bool,
    /// Where a fixed-size capture goes, when it is placed exactly.
    pub fixed_at: Option<(i32, i32)>,
    /// The Time Delay switch is on.
    pub timer: bool,
    /// What it does: "delay", "interval" (time-lapse) or "schedule".
    pub timing: String,
    /// The delay it turns on, in seconds.
    pub delay: u32,
    /// The corner the Record Webcam switch puts the webcam in.
    pub webcam_corner: String,
}

impl Default for CaptureWindow {
    fn default() -> CaptureWindow {
        CaptureWindow {
            tab: 1,
            selection: "image".to_owned(),
            video_selection: "region".to_owned(),
            fixed: false,
            fixed_at: None,
            timer: false,
            timing: "delay".to_owned(),
            delay: 5,
            webcam_corner: "bottom-right".to_owned(),
        }
    }
}

/// A saved capture setup, run from the tray's Presets menu or its hotkey.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preset {
    pub name: String,
    /// all (All-in-One), image, window, fullscreen, scrolling, horizontal,
    /// both, panoramic, freehand, multi, fixed, menu, text, clipboard,
    /// file, video, video-fullscreen or steps.
    pub mode: String,
    pub delay: u32,
    pub cursor: bool,
    /// For example "Ctrl+Shift+1".
    pub hotkey: Option<String>,
    pub effects: Vec<String>,
    /// A share destination for this preset's captures (see
    /// [`Settings::share`]); the window's choice when unset.
    pub share: Option<String>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            copy_to_clipboard: true,
            open_in_editor: true,
            include_cursor: false,
            delay_seconds: 3,
            editor: None,
            record_audio: "system".to_owned(),
            record_fps: 30,
            noise_gate: true,
            countdown: 3,
            cursor_highlight: false,
            click_rings: true,
            webcam: None,
            webcam_shape: "circle".to_owned(),
            webcam_size: 0.25,
            draw_fade_seconds: 3,
            fixed_size: (1280, 720),
            interval_seconds: 10,
            interval_skip_identical: true,
            record_cursor: true,
            screen_draw: true,
            cursor_as_object: true,
            capture_effects: Vec::new(),
            presets: vec![
                Preset { name: "Full screen in 3 s".into(), mode: "image".into(), delay: 3, cursor: true, hotkey: None, effects: Vec::new(), share: None },
                Preset { name: "Window with border".into(), mode: "image".into(), delay: 0, cursor: false, hotkey: None, effects: vec!["Border".into()], share: None },
            ],
            show_widget: false,
            schedule: None,
            schedule_default: None,
            schedule_area: None,
            share: "none".to_owned(),
            video_share: "none".to_owned(),
            capture_window: CaptureWindow::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Settings {
        serde_json::from_value(jiffshot_capture::config::section("capture")).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(value) = serde_json::to_value(self) {
            jiffshot_capture::config::set_section("capture", value);
        }
    }
}

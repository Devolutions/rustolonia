//! The resident part of the capture side: a hidden window on its own thread
//! that owns the global hotkeys, the timers and the capture workflow. The
//! app's Avalonia tray icon sends it commands ([`TrayAction`]) and shows
//! what it publishes ([`TrayState`], notices).

use crate::overlay::{self, Overlay};
use crate::settings::Settings;
use crate::workflow::{self, Capture, Outcome};
use crate::{log, modes};
use jiffshot_capture::library::{CaptureKind, Library};
use jiffshot_capture::screen::{self, CursorSnapshot};
use jiffshot_capture::targets::Targets;
use jiffshot_capture::{wide, Rect};
use std::cell::RefCell;
use std::time::Instant;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, VK_F11,
    VK_SNAPSHOT,
};
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW, KillTimer, PostMessageW, PostQuitMessage,
    RegisterClassExW, SetTimer, TranslateMessage, MSG, SW_SHOWNORMAL, WM_APP, WM_DESTROY, WM_HOTKEY, WM_TIMER, WNDCLASSEXW,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

pub const CLASS: &str = "JiffshotCaptureTray";
/// Posted by a second instance (or a tray click) to start a capture.
pub const WM_APP_CAPTURE: u32 = WM_APP + 2;
const WM_APP_DONE: u32 = WM_APP + 3;
/// Posted (by a second instance) to capture open menus after a delay.
pub const WM_APP_MENU: u32 = WM_APP + 4;
/// A capture (wparam = mode) to start after lparam milliseconds, so the
/// window that asked can get out of the way.
pub const WM_APP_CAPTURE_LATER: u32 = WM_APP + 5;
/// A tray menu command (wparam = an `ID_*`), from [`TrayAction::post`].
const WM_APP_COMMAND: u32 = WM_APP + 6;
/// A closure to run on this thread (lparam = `Box<Box<dyn FnOnce()>>`), from
/// [`crate::on_capture_thread`].
pub(crate) const WM_APP_RUN: u32 = WM_APP + 7;
const TIMER_DELAY: usize = 1;
const TIMER_HOTKEY_RETRY: usize = 2;
const TIMER_MENU: usize = 3;
const TIMER_INITIAL: usize = 4;

/// What a capture produces, passed as WM_APP_CAPTURE's wparam.
pub const MODE_IMAGE: usize = 0;
pub const MODE_SCROLLING: usize = 1;
pub const MODE_PANORAMIC: usize = 2;
pub const MODE_TEXT: usize = 3;
pub const MODE_VIDEO: usize = 4;
pub const MODE_FREEHAND: usize = 5;
pub const MODE_MULTI: usize = 6;
pub const MODE_FIXED: usize = 7;
pub const MODE_SCROLL_H: usize = 8;
pub const MODE_STEPS: usize = 9;
pub const MODE_SCROLL_BOTH: usize = 10;
/// Select an area, then choose image, video, scrolling or panoramic.
pub const MODE_ALL: usize = 11;
pub const MODE_WINDOW: usize = 12;
pub const MODE_FULLSCREEN: usize = 13;
pub const MODE_CLIPBOARD: usize = 14;
pub const MODE_FILE: usize = 15;
pub const MODE_VIDEO_FULL: usize = 16;
/// Not a mode: the command line asks for the Capture window.
pub const OPEN_CAPTURE_WINDOW: usize = 99;
const ID_SCROLL_BOTH: usize = 48;
const ID_FROM_CLIPBOARD: usize = 49;
const ID_FROM_FILE: usize = 50;
const ID_SCHEDULE: usize = 51;
const TIMER_SCHEDULE: usize = 7;
const TIMER_MODE: usize = 8;
const ID_CAPWIN: usize = 52;
const TIMER_TIMELAPSE: usize = 5;
const TIMER_PRESET: usize = 6;
const HOTKEY_PRESET: i32 = 100;
const ID_FREEHAND: usize = 40;
const ID_MULTI: usize = 41;
const ID_FIXED: usize = 42;
const ID_SCROLL_H: usize = 43;
const ID_STEPS: usize = 44;
const ID_TIMELAPSE: usize = 45;
const ID_WIDGET: usize = 46;
const ID_STARTUP: usize = 47;
const ID_PRESET: usize = 100;

/// The capture mode a preset's `mode` names.
pub fn mode_named(name: &str) -> Option<usize> {
    Some(match name {
        "all" => MODE_ALL,
        "image" => MODE_IMAGE,
        "window" => MODE_WINDOW,
        "fullscreen" => MODE_FULLSCREEN,
        "clipboard" => MODE_CLIPBOARD,
        "file" => MODE_FILE,
        "video-fullscreen" => MODE_VIDEO_FULL,
        "scrolling" => MODE_SCROLLING,
        "horizontal" => MODE_SCROLL_H,
        "both" => MODE_SCROLL_BOTH,
        "panoramic" => MODE_PANORAMIC,
        "freehand" => MODE_FREEHAND,
        "multi" => MODE_MULTI,
        "fixed" => MODE_FIXED,
        "text" => MODE_TEXT,
        "video" => MODE_VIDEO,
        "steps" => MODE_STEPS,
        "menu" => usize::MAX,
        _ => return None,
    })
}

/// Parses "Ctrl+Shift+1" into hotkey modifiers and a virtual key.
pub fn parse_hotkey(text: &str) -> Option<(HOT_KEY_MODIFIERS, u16)> {
    let mut mods = 0;
    let mut key = None;
    for part in text.split('+').map(|p| p.trim().to_ascii_lowercase()) {
        match part.as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "shift" => mods |= MOD_SHIFT,
            "alt" => mods |= MOD_ALT,
            "win" => mods |= windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_WIN,
            "prtscn" | "printscreen" => key = Some(VK_SNAPSHOT),
            k if k.len() == 1 && k.chars().next().unwrap().is_ascii_alphanumeric() => key = Some(k.to_ascii_uppercase().as_bytes()[0] as u16),
            k if k.starts_with('f') && k[1..].parse::<u16>().is_ok_and(|n| (1..=24).contains(&n)) => key = Some(0x6F + k[1..].parse::<u16>().unwrap()),
            _ => return None,
        }
    }
    key.map(|k| (mods, k))
}

/// The Run key entry that starts the tray with Windows.
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

pub(crate) fn runs_at_startup() -> bool {
    run_value_exists(jiffshot_capture::paths::NAME)
}

fn run_value_exists(name: &str) -> bool {
    let mut size = 0u32;
    // SAFETY: a size query of one registry value.
    unsafe {
        windows_sys::Win32::System::Registry::RegGetValueW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            wide(name).as_ptr(),
            windows_sys::Win32::System::Registry::RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        ) == 0
    }
}

pub(crate) fn set_run_at_startup(on: bool) {
    use windows_sys::Win32::System::Registry::{RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegSetValueExW, HKEY, REG_SZ};
    let mut key: HKEY = std::ptr::null_mut();
    // SAFETY: opens, writes or deletes one value under our own user's Run key.
    unsafe {
        if RegCreateKeyW(HKEY_CURRENT_USER, wide(RUN_KEY).as_ptr(), &mut key) != 0 {
            return;
        }
        if on {
            let exe = std::env::current_exe().map(|p| format!("\"{}\"", p.display())).unwrap_or_default();
            let value = wide(&exe);
            RegSetValueExW(key, wide(jiffshot_capture::paths::NAME).as_ptr(), 0, REG_SZ, value.as_ptr() as *const u8, (value.len() * 2) as u32);
        } else {
            RegDeleteValueW(key, wide(jiffshot_capture::paths::NAME).as_ptr());
        }
        RegCloseKey(key);
    }
}

const HOTKEY_CAPTURE: i32 = 1;
const HOTKEY_SCREEN: i32 = 2;
const HOTKEY_REPEAT: i32 = 3;

const ID_CAPTURE: usize = 1;
const ID_DELAYED: usize = 2;
const ID_SCREEN: usize = 3;
const ID_REPEAT: usize = 4;
const ID_SCROLLING: usize = 5;
const ID_PANORAMIC: usize = 6;
const ID_MENU: usize = 7;
const ID_TEXT: usize = 8;
const ID_VIDEO: usize = 9;
const ID_CLIPBOARD: usize = 10;
const ID_EDITOR: usize = 11;
const ID_CURSOR: usize = 12;
const ID_LIBRARY: usize = 20;
const ID_EXIT: usize = 30;

struct App {
    hwnd: HWND,
    settings: Settings,
    busy: bool,
    last_region: Option<Rect>,
    /// How the capture hotkey reads in menus, e.g. "PrtScn".
    capture_key: String,
    /// A capture requested on the command line, started once the tray runs.
    pending: Option<usize>,
    /// Time-lapse area and the last frame's hash (identical frames are
    /// skipped).
    timelapse: Option<(Rect, u64)>,
    /// A preset waiting for its delay.
    pending_preset: Option<usize>,
    /// Overrides for the capture running now (from a preset).
    override_cursor: Option<bool>,
    override_effects: Option<Vec<String>>,
    override_share: Option<String>,
    scheduled_fired: Option<std::time::Instant>,
    /// A capture from the Capture window waiting for its delay: a mode, or
    /// a fixed rectangle.
    pending_mode: Option<Result<usize, Rect>>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` with the app state. Never call it across a nested message loop:
/// the overlay re-enters the window procedure.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|app| app.borrow_mut().as_mut().map(f))
}

/// A notice for the user: the app shows it as a toast (only logged when no
/// app hosts the capture side).
fn notify(_hwnd: HWND, title: &str, text: &str, warning: bool) {
    match crate::hooks() {
        Some(hooks) => (hooks.notify)(title, text, warning),
        None => log(&format!("{title}: {text}")),
    }
}

/// What the tray menu shows, as the capture side sees it now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrayState {
    pub copy_to_clipboard: bool,
    pub open_in_editor: bool,
    pub include_cursor: bool,
    pub show_widget: bool,
    pub run_at_startup: bool,
    pub time_lapse: bool,
    pub schedule: bool,
    /// A region was captured, so Repeat has something to repeat.
    pub can_repeat: bool,
    /// The capture hotkey, as the menu shows it ("PrtScn" or a fallback).
    pub capture_key: String,
}

/// Sends the current [`TrayState`] to the app.
pub(crate) fn publish_state() {
    let Some(hooks) = crate::hooks() else { return };
    let Some(state) = with_app(|app| TrayState {
        copy_to_clipboard: app.settings.copy_to_clipboard,
        open_in_editor: app.settings.open_in_editor,
        include_cursor: app.settings.include_cursor,
        show_widget: app.settings.show_widget,
        run_at_startup: false,
        time_lapse: app.timelapse.is_some(),
        schedule: app.settings.schedule.is_some(),
        can_repeat: app.last_region.is_some(),
        capture_key: app.capture_key.clone(),
    }) else {
        return;
    };
    if let Some(settings) = with_app(|app| app.settings.clone()) {
        *crate::SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) = Some(settings);
    }
    (hooks.state)(TrayState { run_at_startup: runs_at_startup(), ..state });
}

/// The tray menu's commands, run on the capture side's thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    Capture,
    CaptureDelayed,
    CaptureScreen,
    RepeatLast,
    CaptureWindow,
    Scrolling,
    ScrollingHorizontal,
    ScrollingBoth,
    Panoramic,
    Freehand,
    MultipleAreas,
    FixedSize,
    MenuCapture,
    GrabText,
    StepCapture,
    FromClipboard,
    FromFile,
    RecordVideo,
    TimeLapse,
    Schedule,
    CopyToClipboard,
    OpenInEditor,
    IncludeCursor,
    ShowWidget,
    RunAtStartup,
    OpenLibraryFolder,
    Exit,
}

impl TrayAction {
    fn id(self) -> usize {
        match self {
            TrayAction::Capture => ID_CAPTURE,
            TrayAction::CaptureDelayed => ID_DELAYED,
            TrayAction::CaptureScreen => ID_SCREEN,
            TrayAction::RepeatLast => ID_REPEAT,
            TrayAction::CaptureWindow => ID_CAPWIN,
            TrayAction::Scrolling => ID_SCROLLING,
            TrayAction::ScrollingHorizontal => ID_SCROLL_H,
            TrayAction::ScrollingBoth => ID_SCROLL_BOTH,
            TrayAction::Panoramic => ID_PANORAMIC,
            TrayAction::Freehand => ID_FREEHAND,
            TrayAction::MultipleAreas => ID_MULTI,
            TrayAction::FixedSize => ID_FIXED,
            TrayAction::MenuCapture => ID_MENU,
            TrayAction::GrabText => ID_TEXT,
            TrayAction::StepCapture => ID_STEPS,
            TrayAction::FromClipboard => ID_FROM_CLIPBOARD,
            TrayAction::FromFile => ID_FROM_FILE,
            TrayAction::RecordVideo => ID_VIDEO,
            TrayAction::TimeLapse => ID_TIMELAPSE,
            TrayAction::Schedule => ID_SCHEDULE,
            TrayAction::CopyToClipboard => ID_CLIPBOARD,
            TrayAction::OpenInEditor => ID_EDITOR,
            TrayAction::IncludeCursor => ID_CURSOR,
            TrayAction::ShowWidget => ID_WIDGET,
            TrayAction::RunAtStartup => ID_STARTUP,
            TrayAction::OpenLibraryFolder => ID_LIBRARY,
            TrayAction::Exit => ID_EXIT,
        }
    }

    /// Runs the command on the capture side's thread; safe from any thread.
    pub fn post(self) {
        // SAFETY: posts to our own tray window, found by class.
        unsafe {
            let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW(wide(CLASS).as_ptr(), std::ptr::null());
            if !hwnd.is_null() {
                PostMessageW(hwnd, WM_APP_COMMAND, self.id(), 0);
            }
        }
    }
}

fn register_hotkey(hwnd: HWND, id: i32, modifiers: HOT_KEY_MODIFIERS, vk: u16) -> bool {
    // SAFETY: registers a hotkey for our own window.
    unsafe { RegisterHotKey(hwnd, id, modifiers | MOD_NOREPEAT, vk as u32) != 0 }
}

/// Capture keys to try when PrtScn is taken, in order.
const FALLBACK_KEYS: &[(HOT_KEY_MODIFIERS, u16, &str)] = &[
    (MOD_CONTROL | MOD_SHIFT, b'X' as u16, "Ctrl+Shift+X"),
    (MOD_CONTROL | MOD_ALT, VK_SNAPSHOT, "Ctrl+Alt+PrtScn"),
    (MOD_CONTROL | MOD_SHIFT, VK_F11, "Ctrl+Shift+F11"),
];

/// Whether Windows routes PrtScn to the Snipping Tool (Settings ›
/// Accessibility › Keyboard).
fn snipping_owns_print_screen() -> bool {
    let mut value: u32 = 0;
    let mut size = 4u32;
    // SAFETY: reads one DWORD into a correctly sized buffer.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide("Control Panel\\Keyboard").as_ptr(),
            wide("PrintScreenKeyForSnippingEnabled").as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut value as *mut u32 as *mut _,
            &mut size,
        )
    };
    status == 0 && value != 0
}

fn print_screen_owner() -> &'static str {
    if snipping_owns_print_screen() {
        "Windows gives it to the Snipping Tool (Settings › Accessibility › Keyboard › Print screen key)"
    } else {
        "another app holds it (another screen capture tool, if one is running)"
    }
}

/// Registers PrtScn, or the first free fallback when another app or the
/// Snipping Tool owns it, plus the full-screen and repeat keys.
fn register_hotkeys(hwnd: HWND) -> (String, Vec<String>) {
    let mut problems = Vec::new();
    let capture_key = if register_hotkey(hwnd, HOTKEY_CAPTURE, 0, VK_SNAPSHOT) {
        "PrtScn".to_owned()
    } else if let Some(&(_, _, name)) =
        FALLBACK_KEYS.iter().find(|(modifiers, vk, _)| register_hotkey(hwnd, HOTKEY_CAPTURE, *modifiers, *vk))
    {
        problems.push(format!(
            "PrtScn is unavailable because {}; {name} captures until it frees up",
            print_screen_owner()
        ));
        name.to_owned()
    } else {
        problems.push(format!("PrtScn is unavailable because {} and no fallback key is free; use the tray icon", print_screen_owner()));
        "the tray icon".to_owned()
    };
    if !register_hotkey(hwnd, HOTKEY_SCREEN, MOD_CONTROL, VK_SNAPSHOT) {
        problems.push("Ctrl+PrtScn (full screen) is taken".to_owned());
    }
    if !register_hotkey(hwnd, HOTKEY_REPEAT, MOD_SHIFT, VK_SNAPSHOT) {
        problems.push("Shift+PrtScn (repeat last region) is taken".to_owned());
    }
    (capture_key, problems)
}

/// Called periodically while a fallback key is in use: takes PrtScn back
/// once its owner lets go, and tells the user.
fn retry_print_screen(hwnd: HWND) {
    if with_app(|app| app.capture_key == "PrtScn").unwrap_or(true) {
        // SAFETY: a timer on our own window.
        unsafe { KillTimer(hwnd, TIMER_HOTKEY_RETRY) };
        return;
    }
    // SAFETY: re-registers our own hotkey id.
    unsafe { UnregisterHotKey(hwnd, HOTKEY_CAPTURE) };
    if register_hotkey(hwnd, HOTKEY_CAPTURE, 0, VK_SNAPSHOT) {
        with_app(|app| app.capture_key = "PrtScn".to_owned());
        log("PrtScn became free; capture key is now PrtScn");
        publish_state();
        notify(hwnd, "Jiffshot hotkeys", "PrtScn now starts a Jiffshot capture", false);
        // SAFETY: as above.
        unsafe { KillTimer(hwnd, TIMER_HOTKEY_RETRY) };
    } else {
        let key = with_app(|app| app.capture_key.clone()).unwrap_or_default();
        if let Some(&(modifiers, vk, _)) = FALLBACK_KEYS.iter().find(|(_, _, name)| *name == key) {
            register_hotkey(hwnd, HOTKEY_CAPTURE, modifiers, vk);
        }
    }
}

fn cursor_pos() -> (i32, i32) {
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: out-parameter only.
    unsafe { GetCursorPos(&mut point) };
    (point.x, point.y)
}

fn spawn_finish(capture: Capture, settings: Settings, owner: HWND) {
    let owner = owner as isize;
    std::thread::spawn(move || {
        let outcome = workflow::finish(capture, &settings, owner);
        // SAFETY: the window outlives worker threads in practice; if it is
        // gone the post fails and the box is leaked, which is harmless at exit.
        unsafe { PostMessageW(owner as HWND, WM_APP_DONE, 0, Box::into_raw(Box::new(outcome)) as LPARAM) };
    });
}

/// Freezes the screen and runs the selection overlay; `mode` says what to
/// do with the selection.
fn capture_interactive(mode: usize) {
    let Some((hwnd, settings)) = with_app(|app| {
        if app.busy {
            return None;
        }
        app.busy = true;
        Some((app.hwnd, app.settings.clone()))
    })
    .flatten() else {
        return;
    };

    let started = Instant::now();
    let cursor = CursorSnapshot::take();
    let pointer = cursor_pos();
    let desktop = screen::virtual_screen();
    let prepared = screen::grab(desktop).and_then(|snapshot| {
        let grabbed = started.elapsed();
        let targets = Targets::enumerate(desktop, &[hwnd as isize]);
        let enumerated = started.elapsed();
        let overlay = Overlay::new(snapshot, (desktop.x, desktop.y), targets, screen::monitors(), pointer)?;
        log(&format!(
            "overlay ready in {:.0} ms (grab {:.0}, windows {:.0}, setup {:.0}) for {}x{}",
            started.elapsed().as_secs_f64() * 1000.0,
            grabbed.as_secs_f64() * 1000.0,
            (enumerated - grabbed).as_secs_f64() * 1000.0,
            (started.elapsed() - enumerated).as_secs_f64() * 1000.0,
            desktop.width,
            desktop.height
        ));
        Ok(overlay)
    });
    let overlay = match prepared {
        Ok(overlay) => overlay,
        Err(error) => {
            with_app(|app| app.busy = false);
            notify(hwnd, "Capture failed", &error.to_string(), true);
            return;
        }
    };

    let mut overlay = overlay;
    overlay.started = Some(started);
    overlay.mode = match mode {
        MODE_FREEHAND => overlay::Mode::Freehand,
        MODE_MULTI => overlay::Mode::MultiArea,
        MODE_FIXED => overlay::Mode::Fixed(settings.fixed_size.0.max(16), settings.fixed_size.1.max(16)),
        MODE_VIDEO => overlay::Mode::Video,
        _ => overlay::Mode::Normal,
    };
    let (overlay, selection) = overlay::run(overlay);
    // For checking the overlay live: the selection is logged, nothing is
    // saved or copied.
    if std::env::var_os("JIFFSHOT_CAPTURE_DRY_RUN").is_some() {
        log(&format!("dry run: selected {selection:?}"));
        with_app(|app| app.busy = false);
        return;
    }
    let mut settings = settings;
    if let Some((cursor, effects, share)) = with_app(|app| (app.override_cursor.take(), app.override_effects.take(), app.override_share.take())) {
        if let Some(c) = cursor {
            settings.include_cursor = c;
        }
        if let Some(to) = share {
            settings.share = to;
        }
        if let Some(e) = effects {
            settings.capture_effects = e;
        }
    }
    let Some(selection) = selection else {
        with_app(|app| app.busy = false);
        return;
    };
    let rect = selection.rect();
    let include_cursor = settings.include_cursor && matches!(mode, MODE_ALL | MODE_IMAGE | MODE_WINDOW | MODE_FREEHAND | MODE_MULTI | MODE_FIXED);
    let Some(capture) = workflow::from_overlay(&overlay, &selection, cursor.as_ref(), include_cursor, settings.cursor_as_object) else {
        with_app(|app| app.busy = false);
        return;
    };
    drop(overlay);
    with_app(|app| app.last_region = Some(rect));
    publish_state();
    // All-in-One asks what to make of the area once it is selected.
    let mode = if mode == MODE_ALL {
        match crate::hud::run_panel(rect, "Capture this area as", &["Image", "Video", "Scrolling", "Panoramic"], 1000, || None) {
            Some(0) => MODE_IMAGE,
            Some(1) => MODE_VIDEO,
            Some(2) => MODE_SCROLLING,
            Some(3) => MODE_PANORAMIC,
            _ => {
                with_app(|app| app.busy = false);
                return;
            }
        }
    } else {
        mode
    };
    match mode {
        MODE_TEXT => {
            match modes::text(&capture.frame) {
                Ok(text) if !text.trim().is_empty() => {
                    let copied = jiffshot_capture::clipboard::copy_text(&text, hwnd as isize);
                    let preview: String = text.chars().take(180).collect();
                    let title = if copied.is_ok() { "Text copied" } else { "Text found (clipboard busy)" };
                    log(&format!("grab text: {} characters", text.chars().count()));
                    notify(hwnd, title, &preview, false);
                    crate::show_text(&text);
                }
                Ok(_) => notify(hwnd, "Grab text", "No text found in the selection", true),
                Err(error) => notify(hwnd, "Grab text failed", &error, true),
            }
        }
        // These run until done, so they stay busy until then.
        MODE_SCROLLING => return spawn_scrolling(modes::scrolling, rect, settings, hwnd),
        MODE_SCROLL_BOTH => return spawn_scrolling(modes::scrolling_both, rect, settings, hwnd),
        MODE_SCROLL_H => return spawn_scrolling(modes::scrolling_horizontal, rect, settings, hwnd),
        MODE_PANORAMIC => match modes::panoramic(rect) {
            Ok(Some(frame)) => spawn_finish(workflow::from_frame(CaptureKind::Panoramic, rect, frame), settings, hwnd),
            Ok(None) => {}
            Err(error) => notify(hwnd, "Panoramic capture failed", &error.to_string(), true),
        },
        MODE_VIDEO => match crate::recorder::record(rect, &settings) {
            Ok(Some(path)) => spawn_video_finish(path, rect, settings, hwnd),
            Ok(None) => {}
            Err(error) => notify(hwnd, "Recording failed", &error, true),
        },
        _ => spawn_finish(capture, settings, hwnd),
    }
    with_app(|app| app.busy = false);
}

/// Runs a scrolling capture on a worker thread (it scrolls and grabs for a
/// while), then finishes it here; busy until then.
fn spawn_scrolling(scroll: fn(Rect) -> std::io::Result<jiffshot_capture::Frame>, rect: Rect, settings: Settings, owner: HWND) {
    let owner = owner as isize;
    std::thread::spawn(move || {
        let started = Instant::now();
        let result = scroll(rect);
        crate::on_capture_thread(move || {
            with_app(|app| app.busy = false);
            let hwnd = owner as HWND;
            match result {
                Ok(frame) => {
                    log(&format!("scrolling capture {}x{} in {:.1} s", frame.width, frame.height, started.elapsed().as_secs_f64()));
                    spawn_finish(workflow::from_frame(CaptureKind::Scrolling, rect, frame), settings, hwnd);
                }
                Err(error) => notify(hwnd, "Scrolling capture failed", &error.to_string(), true),
            }
        });
    });
}

/// Opens a finished recording like a capture (editor or notification).
fn spawn_video_finish(path: std::path::PathBuf, rect: Rect, settings: Settings, owner: HWND) {
    let owner = owner as isize;
    std::thread::spawn(move || {
        let outcome = workflow::finish_video(&path, rect, &settings, owner);
        // SAFETY: as in `spawn_finish`.
        unsafe { PostMessageW(owner as HWND, WM_APP_DONE, 0, Box::into_raw(Box::new(outcome)) as LPARAM) };
    });
}

/// Records a Step Capture and opens the guide it makes.
fn capture_steps() {
    let Some((hwnd, settings)) = with_app(|app| {
        if app.busy {
            return None;
        }
        app.busy = true;
        Some((app.hwnd, app.settings.clone()))
    })
    .flatten() else {
        return;
    };
    let steps = crate::stepcap::record();
    with_app(|app| app.busy = false);
    let Some(steps) = steps else { return };
    let doc = crate::stepcap::guide(&steps);
    let owner = hwnd as isize;
    std::thread::spawn(move || {
        let library = Library::new(jiffshot_capture::config::library_root());
        let frame = workflow::frame_from_pixmap(&doc.flatten());
        let rect = Rect::new(0, 0, frame.width as i32, frame.height as i32);
        let capture = workflow::from_frame(CaptureKind::Steps, rect, frame);
        let outcome = match library.save(&capture.frame, &capture.meta) {
            Ok(path) => {
                // The editable guide next to its image; the editor opens it.
                let _ = jiffshot_doc::io::save_document(&doc, &path.with_extension(jiffshot_capture::library::EDITS_EXT));
                let opened = settings.open_in_editor && workflow::open_editor(&settings, &path);
                Outcome { message: format!("Step Capture: {} steps saved", steps.len()), is_error: false, opened_editor: opened }
            }
            Err(error) => Outcome { message: format!("Could not save the guide: {error}"), is_error: true, opened_editor: false },
        };
        // SAFETY: as in `spawn_finish`.
        unsafe { PostMessageW(owner as HWND, WM_APP_DONE, 0, Box::into_raw(Box::new(outcome)) as LPARAM) };
    });
}

/// Takes the image on the clipboard (or an image file copied there) as a
/// capture.
fn capture_from_clipboard() {
    let Some((hwnd, settings)) = with_app(|app| (app.hwnd, app.settings.clone())) else { return };
    let frame = match jiffshot_capture::clipboard::read(hwnd as isize) {
        Ok(Some(jiffshot_capture::clipboard::Pasted::Image { width, height, rgba })) => {
            let mut f = jiffshot_capture::Frame::new(width, height);
            f.alpha = rgba.chunks_exact(4).any(|p| p[3] < 255);
            for (d, s) in f.pixels.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
                d.copy_from_slice(&[s[2], s[1], s[0], s[3]]);
            }
            Some(f)
        }
        Ok(Some(jiffshot_capture::clipboard::Pasted::Files(files))) => files.iter().find_map(|p| jiffshot_doc::io::load_image(p).ok()).map(|pm| workflow::frame_from_pixmap(&pm)),
        _ => None,
    };
    match frame {
        Some(frame) => {
            let rect = Rect::new(0, 0, frame.width as i32, frame.height as i32);
            spawn_finish(workflow::from_frame(CaptureKind::Region, rect, frame), settings, hwnd);
        }
        None => notify(hwnd, "Capture from clipboard", "There is no image on the clipboard", true),
    }
}

/// Asks for image files and adds each to the library as a capture.
fn capture_from_file() {
    use windows_sys::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, OFN_ALLOWMULTISELECT, OFN_EXPLORER, OFN_FILEMUSTEXIST, OPENFILENAMEW};
    let Some((hwnd, settings)) = with_app(|app| (app.hwnd, app.settings.clone())) else { return };
    let mut buf = vec![0u16; 32 * 1024];
    let filter: Vec<u16> = "Images\0*.png;*.jpg;*.jpeg;*.bmp;*.gif;*.webp\0All files\0*.*\0\0".encode_utf16().collect();
    let title = wide("Capture from file");
    // SAFETY: a plain C struct; zero is valid for every field.
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = hwnd;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = buf.as_mut_ptr();
    ofn.nMaxFile = buf.len() as u32;
    ofn.lpstrTitle = title.as_ptr();
    ofn.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_ALLOWMULTISELECT;
    // SAFETY: every pointer in `ofn` outlives the modal call.
    if unsafe { GetOpenFileNameW(&mut ofn) } == 0 {
        return;
    }
    // One file: its full path. Several: the folder, then each name.
    let parts: Vec<String> = buf.split(|&c| c == 0).take_while(|p| !p.is_empty()).map(String::from_utf16_lossy).collect();
    use std::path::PathBuf;
    let files: Vec<PathBuf> = match parts.as_slice() {
        [one] => vec![PathBuf::from(one)],
        [dir, names @ ..] => names.iter().map(|n| PathBuf::from(dir).join(n)).collect(),
        [] => Vec::new(),
    };
    for (i, path) in files.iter().enumerate() {
        match jiffshot_doc::io::load_image(path) {
            Ok(pm) => {
                let frame = workflow::frame_from_pixmap(&pm);
                let rect = Rect::new(0, 0, frame.width as i32, frame.height as i32);
                let mut s = settings.clone();
                // Only the last one opens in the editor.
                s.open_in_editor &= i + 1 == files.len();
                s.copy_to_clipboard = false;
                spawn_finish(workflow::from_frame(CaptureKind::Region, rect, frame), s, hwnd);
            }
            Err(error) => notify(hwnd, "Capture from file", &format!("{}: {error}", path.display()), true),
        }
    }
}

/// Minutes from now until the next `HH:MM` (local time).
pub(crate) fn minutes_until(at: &str) -> Option<u32> {
    let (h, m) = at.trim().split_once(':')?;
    let target = h.parse::<u32>().ok()?.min(23) * 60 + m.parse::<u32>().ok()?.min(59);
    // SAFETY: a plain C struct, filled by the call.
    let mut now = unsafe { std::mem::zeroed::<windows_sys::Win32::Foundation::SYSTEMTIME>() };
    // SAFETY: out-parameter only.
    unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut now) };
    let now = now.wHour as u32 * 60 + now.wMinute as u32;
    Some((target + 24 * 60 - now) % (24 * 60))
}

/// Arms (or disarms) the scheduled capture from `settings.schedule`.
fn arm_schedule(hwnd: HWND, schedule: Option<&str>) {
    // SAFETY: timers on our own window.
    unsafe {
        KillTimer(hwnd, TIMER_SCHEDULE);
        if schedule.and_then(minutes_until).is_some() {
            SetTimer(hwnd, TIMER_SCHEDULE, 20_000, None);
        }
    }
}

fn schedule_tick() {
    let Some((hwnd, settings, last)) = with_app(|app| (app.hwnd, app.settings.clone(), app.scheduled_fired)) else { return };
    let Some(at) = settings.schedule.as_deref() else { return };
    if minutes_until(at) != Some(0) || last.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(120)) {
        return;
    }
    with_app(|app| app.scheduled_fired = Some(std::time::Instant::now()));
    let (x, y) = cursor_pos();
    let rect = settings.schedule_area.map(|(x, y, w, h)| Rect::new(x, y, w, h)).or_else(|| screen::monitor_at(x, y).map(|m| m.rect)).unwrap_or_else(screen::virtual_screen);
    if let Ok(capture) = workflow::immediate(CaptureKind::Region, rect, false) {
        let mut quiet = settings;
        quiet.open_in_editor = false;
        quiet.copy_to_clipboard = false;
        spawn_finish(capture, quiet, hwnd);
    }
}

/// Starts or stops time-lapse capture of the last area (or the screen).
/// Starts or stops the time-lapse (an interval capture of the last area,
/// or the screen under the pointer).
pub fn toggle_timelapse() {
    let Some((hwnd, running, seconds, last)) = with_app(|app| (app.hwnd, app.timelapse.is_some(), app.settings.interval_seconds.max(1), app.last_region)) else { return };
    if running {
        with_app(|app| app.timelapse = None);
        // SAFETY: a timer on our own window.
        unsafe { KillTimer(hwnd, TIMER_TIMELAPSE) };
        notify(hwnd, "Time-lapse", "Stopped", false);
        return;
    }
    let (x, y) = cursor_pos();
    let rect = last.or_else(|| screen::monitor_at(x, y).map(|m| m.rect)).unwrap_or_else(screen::virtual_screen);
    with_app(|app| app.timelapse = Some((rect, 0)));
    notify(hwnd, "Time-lapse", &format!("Capturing every {seconds} s; choose Time-lapse again to stop"), false);
    // SAFETY: as above.
    unsafe { SetTimer(hwnd, TIMER_TIMELAPSE, seconds * 1000, None) };
    timelapse_tick();
}

fn timelapse_tick() {
    let Some((hwnd, settings, Some((rect, last)))) = with_app(|app| (app.hwnd, app.settings.clone(), app.timelapse)) else { return };
    let Ok(capture) = workflow::immediate(CaptureKind::Region, rect, false) else { return };
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in capture.frame.pixels.iter().step_by(7) {
        h = (h ^ *b as u64).wrapping_mul(0x100_0000_01b3);
    }
    if h == last && settings.interval_skip_identical {
        return;
    }
    with_app(|app| app.timelapse = Some((rect, h)));
    let mut quiet = settings;
    quiet.open_in_editor = false;
    quiet.copy_to_clipboard = false;
    spawn_finish(capture, quiet, hwnd);
}

// ---- for the Capture window ---------------------------------------------

/// Shows or hides the Capture Widget, and remembers it.
pub fn set_widget(on: bool) {
    with_app(|app| {
        app.settings.show_widget = on;
        app.settings.save();
    });
    // The app shows or hides the widget from the published state.
    publish_state();
}

/// Starts a capture with the Capture window's settings (already hidden):
/// its tab's mode, or the interval or schedule its Time Delay is set to.
pub fn start_from_panel(s: Settings) {
    let tab = s.capture_window.tab.min(2);
    apply_settings(s.clone());
    if tab != 2 && s.capture_window.timer {
        match s.capture_window.timing.as_str() {
            "interval" => return toggle_timelapse(),
            "schedule" => {
                let at = match s.schedule {
                    Some(_) => None,
                    None => Some(s.schedule_default.clone().unwrap_or_else(|| "09:00".into())),
                };
                return set_schedule(at);
            }
            _ => {}
        }
    }
    let mode = crate::panel::mode_of(&s, tab);
    match (mode.as_str(), s.capture_window.fixed_at) {
        ("fixed", Some((x, y))) if tab == 1 => capture_rect_after_delay(Rect::new(x, y, s.fixed_size.0, s.fixed_size.1)),
        _ => start_mode_after_delay(&mode),
    }
}

/// Turns the daily scheduled capture on at "HH:MM", or off.
pub fn set_schedule(at: Option<String>) {
    let Some(hwnd) = with_app(|app| {
        app.settings.schedule = at.clone();
        if at.is_some() {
            app.settings.schedule_default = at.clone();
        }
        app.settings.save();
        app.hwnd
    }) else {
        return;
    };
    arm_schedule(hwnd, at.as_deref());
    if let Some(at) = at {
        notify(hwnd, "Scheduled capture", &format!("The screen will be captured every day at {at}"), false);
    }
    publish_state();
}

pub fn current_settings() -> Settings {
    with_app(|app| app.settings.clone()).unwrap_or_default()
}

/// Keeps and saves settings edited in the Capture window.
pub fn apply_settings(settings: Settings) {
    with_app(|app| {
        let presets = std::mem::take(&mut app.settings.presets);
        app.settings = Settings { presets, ..settings };
        app.settings.save();
    });
    publish_state();
}

/// Keeps a recording's switches (as the toolbar left them) for the next one.
pub(crate) fn remember_recording(control: &crate::recorder::Control) {
    use std::sync::atomic::Ordering::Relaxed;
    with_app(|app| {
        let s = &mut app.settings;
        s.record_cursor = control.cursor.load(Relaxed);
        s.cursor_highlight = control.highlight.load(Relaxed);
        s.click_rings = control.clicks.load(Relaxed);
        s.webcam = control.webcam.load(Relaxed).then(|| control.webcam_corner.lock().unwrap().clone());
        s.webcam_shape = if control.webcam_circle.load(Relaxed) { "circle" } else { "rectangle" }.to_owned();
        s.record_audio = match (control.system.load(Relaxed), control.microphone.load(Relaxed)) {
            (true, true) => "both",
            (true, false) => "system",
            (false, true) => "microphone",
            (false, false) => "none",
        }
        .to_owned();
        s.save();
    });
    publish_state();
}

/// Replaces the presets and their hotkeys.
pub fn set_presets(presets: Vec<crate::settings::Preset>) {
    let Some((hwnd, old)) = with_app(|app| (app.hwnd, app.settings.presets.len())) else { return };
    for i in 0..old.max(presets.len()) {
        // SAFETY: hotkeys of our own window (unregistering a missing one
        // fails harmlessly).
        unsafe { UnregisterHotKey(hwnd, HOTKEY_PRESET + i as i32) };
    }
    register_preset_hotkeys(hwnd, &presets);
    with_app(|app| {
        app.settings.presets = presets;
        app.settings.save();
    });
}

pub fn run_preset_now(index: usize) {
    run_preset(index);
}

fn dispatch_mode(mode: usize) {
    if matches!(mode, MODE_FULLSCREEN | MODE_CLIPBOARD | MODE_FILE | MODE_VIDEO_FULL) {
        // A preset's choices apply to the overlay's captures only.
        with_app(|app| (app.override_cursor, app.override_effects, app.override_share) = (None, None, None));
    }
    match mode {
        usize::MAX => start_menu_capture(),
        MODE_STEPS => capture_steps(),
        MODE_FULLSCREEN => capture_screen_under_cursor(),
        MODE_CLIPBOARD => capture_from_clipboard(),
        MODE_FILE => capture_from_file(),
        MODE_VIDEO_FULL => record_screen_under_cursor(),
        m => capture_interactive(m),
    }
}

/// Records the whole screen under the pointer, without selecting.
fn record_screen_under_cursor() {
    let (x, y) = cursor_pos();
    let Some(monitor) = screen::monitor_at(x, y) else { return };
    let Some((hwnd, settings)) = with_app(|app| {
        if app.busy {
            return None;
        }
        app.busy = true;
        Some((app.hwnd, app.settings.clone()))
    })
    .flatten() else {
        return;
    };
    match crate::recorder::record(monitor.rect, &settings) {
        Ok(Some(path)) => spawn_video_finish(path, monitor.rect, settings, hwnd),
        Ok(None) => {}
        Err(error) => notify(hwnd, "Recording failed", &error, true),
    }
    with_app(|app| app.busy = false);
}

/// Starts a capture by preset mode name, after the delay setting (image
/// captures; recordings count down themselves).
pub fn start_mode_after_delay(name: &str) {
    let mode = if name == "video" { MODE_VIDEO } else { mode_named(name).unwrap_or(MODE_IMAGE) };
    let Some((hwnd, delay)) = with_app(|app| (app.hwnd, app.settings.delay_seconds)) else { return };
    if delay == 0 || mode == MODE_VIDEO {
        // Let the Capture window disappear from the screen first.
        with_app(|app| app.pending_mode = Some(Ok(mode)));
        // SAFETY: a timer on our own window.
        unsafe { SetTimer(hwnd, TIMER_MODE, 150, None) };
        return;
    }
    with_app(|app| app.pending_mode = Some(Ok(mode)));
    notify(hwnd, "Jiffshot", &format!("Capturing in {delay} seconds…"), false);
    // SAFETY: as above.
    unsafe { SetTimer(hwnd, TIMER_MODE, delay * 1000, None) };
}

/// Captures exactly `rect` after the delay setting.
pub fn capture_rect_after_delay(rect: Rect) {
    let Some((hwnd, delay)) = with_app(|app| (app.hwnd, app.settings.delay_seconds)) else { return };
    with_app(|app| app.pending_mode = Some(Err(rect)));
    // SAFETY: a timer on our own window.
    unsafe { SetTimer(hwnd, TIMER_MODE, (delay * 1000).max(150), None) };
}

fn run_pending_mode() {
    let Some((hwnd, settings, pending)) = with_app(|app| (app.hwnd, app.settings.clone(), app.pending_mode.take())) else { return };
    match pending {
        Some(Ok(mode)) => dispatch_mode(mode),
        Some(Err(rect)) => match workflow::immediate(CaptureKind::Region, rect, settings.include_cursor) {
            Ok(capture) => spawn_finish(capture, settings, hwnd),
            Err(error) => notify(hwnd, "Capture failed", &error.to_string(), true),
        },
        None => {}
    }
}

fn run_preset(index: usize) {
    let Some((hwnd, preset)) = with_app(|app| (app.hwnd, app.settings.presets.get(index).cloned())) else { return };
    let Some(preset) = preset else { return };
    if preset.delay > 0 && with_app(|app| app.pending_preset.is_none()).unwrap_or(false) {
        with_app(|app| app.pending_preset = Some(index));
        notify(hwnd, &preset.name, &format!("Capturing in {} seconds…", preset.delay), false);
        // SAFETY: a timer on our own window.
        unsafe { SetTimer(hwnd, TIMER_PRESET, preset.delay * 1000, None) };
        return;
    }
    with_app(|app| {
        app.override_cursor = Some(preset.cursor);
        app.override_effects = Some(preset.effects.clone());
        app.override_share = preset.share.clone();
    });
    dispatch_mode(mode_named(&preset.mode).unwrap_or(MODE_IMAGE));
}

fn register_preset_hotkeys(hwnd: HWND, presets: &[crate::settings::Preset]) {
    for (i, p) in presets.iter().enumerate() {
        if let Some((mods, vk)) = p.hotkey.as_deref().and_then(parse_hotkey) {
            if !register_hotkey(hwnd, HOTKEY_PRESET + i as i32, mods, vk) {
                log(&format!("preset hotkey {} for {} is taken", p.hotkey.as_deref().unwrap_or(""), p.name));
            }
        }
    }
}

/// Captures the menus open right now.
fn capture_menus() {
    let Some((hwnd, settings)) = with_app(|app| (app.hwnd, app.settings.clone())) else { return };
    match modes::menus() {
        Ok(Some(frame)) => {
            let rect = Rect::new(0, 0, frame.width as i32, frame.height as i32);
            spawn_finish(workflow::from_frame(CaptureKind::Menu, rect, frame), settings, hwnd);
        }
        Ok(None) => notify(hwnd, "Menu capture", "No open menu was found. Open the menu during the countdown.", true),
        Err(error) => notify(hwnd, "Menu capture failed", &error.to_string(), true),
    }
}

fn start_menu_capture() {
    if let Some(hwnd) = with_app(|app| app.hwnd) {
        notify(hwnd, "Menu capture", "Open the menu to capture; it is captured in 3 seconds.", false);
        // SAFETY: a timer on our own window.
        unsafe { SetTimer(hwnd, TIMER_MENU, 3000, None) };
    }
}

/// Captures a rectangle without the overlay.
fn capture_now(kind: CaptureKind, rect: Rect) {
    let Some((hwnd, settings)) = with_app(|app| (app.hwnd, app.settings.clone())) else { return };
    match workflow::immediate(kind, rect, settings.include_cursor) {
        Ok(capture) => {
            with_app(|app| app.last_region = Some(rect));
            spawn_finish(capture, settings, hwnd);
        }
        Err(error) => notify(hwnd, "Capture failed", &error.to_string(), true),
    }
}

fn capture_screen_under_cursor() {
    let (x, y) = cursor_pos();
    if let Some(monitor) = screen::monitor_at(x, y) {
        capture_now(CaptureKind::Monitor, monitor.rect);
    }
}

fn repeat_last_region() {
    match with_app(|app| app.last_region).flatten() {
        Some(rect) => capture_now(CaptureKind::Region, rect),
        None => capture_interactive(MODE_IMAGE),
    }
}

fn start_delayed() {
    if let Some((hwnd, seconds)) = with_app(|app| (app.hwnd, app.settings.delay_seconds.max(1))) {
        notify(hwnd, "Jiffshot", &format!("Capturing in {seconds} seconds…"), false);
        // SAFETY: a timer on our own window.
        unsafe { SetTimer(hwnd, TIMER_DELAY, seconds * 1000, None) };
    }
}

pub(crate) fn open_library() {
    let root = jiffshot_capture::config::library_root();
    let _ = std::fs::create_dir_all(&root);
    let path = wide(&root.to_string_lossy());
    // SAFETY: opens a folder in Explorer; all strings are NUL-terminated.
    unsafe {
        ShellExecuteW(std::ptr::null_mut(), wide("open").as_ptr(), path.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
    }
}

/// Runs a tray menu command (an `ID_*`), as the Avalonia tray menu asks
/// through [`TrayAction::post`].
fn run_menu_command(hwnd: HWND, command: usize) {
    let toggle = |f: fn(&mut Settings) -> &mut bool| {
        with_app(|app| {
            let value = f(&mut app.settings);
            *value = !*value;
            app.settings.save();
        });
    };
    match command {
        // Let the menu close before the screen is frozen.
        ID_CAPTURE => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, 0, 0);
        },
        ID_SCROLLING => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_SCROLLING, 0);
        },
        ID_PANORAMIC => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_PANORAMIC, 0);
        },
        ID_TEXT => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_TEXT, 0);
        },
        ID_VIDEO => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_VIDEO, 0);
        },
        ID_MENU => start_menu_capture(),
        ID_FREEHAND => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_FREEHAND, 0);
        },
        ID_MULTI => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_MULTI, 0);
        },
        ID_FIXED => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_FIXED, 0);
        },
        ID_SCROLL_H => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_SCROLL_H, 0);
        },
        ID_STEPS => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_STEPS, 0);
        },
        ID_TIMELAPSE => toggle_timelapse(),
        ID_CAPWIN => crate::show_capture_window(),
        ID_SCROLL_BOTH => unsafe {
            PostMessageW(hwnd, WM_APP_CAPTURE, MODE_SCROLL_BOTH, 0);
        },
        ID_FROM_CLIPBOARD => capture_from_clipboard(),
        ID_FROM_FILE => capture_from_file(),
        ID_SCHEDULE => {
            // Turns it off, or on at `schedule_default` (09:00 unless set).
            let schedule = with_app(|app| {
                app.settings.schedule = match app.settings.schedule.take() {
                    Some(_) => None,
                    None => Some(app.settings.schedule_default.clone().unwrap_or_else(|| "09:00".to_owned())),
                };
                app.settings.save();
                app.settings.schedule.clone()
            })
            .flatten();
            arm_schedule(hwnd, schedule.as_deref());
            if let Some(at) = schedule {
                notify(hwnd, "Scheduled capture", &format!("The screen will be captured every day at {at}"), false);
            }
        }
        ID_STARTUP => set_run_at_startup(!runs_at_startup()),
        ID_WIDGET => set_widget(!current_settings().show_widget),
        id if id >= ID_PRESET => run_preset(id - ID_PRESET),
        ID_DELAYED => start_delayed(),
        ID_SCREEN => capture_screen_under_cursor(),
        ID_REPEAT => repeat_last_region(),
        ID_CLIPBOARD => toggle(|s| &mut s.copy_to_clipboard),
        ID_EDITOR => toggle(|s| &mut s.open_in_editor),
        ID_CURSOR => toggle(|s| &mut s.include_cursor),
        ID_LIBRARY => open_library(),
        ID_EXIT => unsafe { PostQuitMessage(0) },
        _ => {}
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            match wparam as i32 {
                HOTKEY_CAPTURE => capture_interactive(MODE_IMAGE),
                HOTKEY_SCREEN => capture_screen_under_cursor(),
                HOTKEY_REPEAT => repeat_last_region(),
                id if id >= HOTKEY_PRESET => run_preset((id - HOTKEY_PRESET) as usize),
                _ => {}
            }
            0
        }
        WM_APP_RUN => {
            // SAFETY: posted by `on_capture_thread` with a leaked box.
            let run = Box::from_raw(lparam as *mut Box<dyn FnOnce() + Send>);
            run();
            0
        }
        WM_APP_COMMAND => {
            run_menu_command(hwnd, wparam);
            publish_state();
            0
        }
        WM_APP_CAPTURE => {
            if wparam == MODE_STEPS {
                capture_steps();
            } else if wparam == OPEN_CAPTURE_WINDOW {
                crate::show_capture_window();
            } else {
                dispatch_mode(wparam);
            }
            0
        }
        WM_APP_CAPTURE_LATER => {
            with_app(|app| app.pending = Some(wparam));
            SetTimer(hwnd, TIMER_INITIAL, (lparam as u32).max(1), None);
            0
        }
        WM_APP_MENU => {
            start_menu_capture();
            0
        }
        WM_APP_DONE => {
            // SAFETY: posted by `spawn_finish` with a leaked Box<Outcome>.
            let outcome = Box::from_raw(lparam as *mut Outcome);
            if outcome.is_error || !outcome.opened_editor {
                let title = if outcome.is_error { "Capture saved with problems" } else { "Capture saved" };
                notify(hwnd, title, &outcome.message, outcome.is_error);
            }
            0
        }
        WM_TIMER if wparam == TIMER_DELAY => {
            KillTimer(hwnd, TIMER_DELAY);
            capture_interactive(MODE_IMAGE);
            0
        }
        WM_TIMER if wparam == TIMER_MENU => {
            KillTimer(hwnd, TIMER_MENU);
            capture_menus();
            0
        }
        WM_TIMER if wparam == TIMER_MODE => {
            KillTimer(hwnd, TIMER_MODE);
            run_pending_mode();
            0
        }
        WM_TIMER if wparam == TIMER_SCHEDULE => {
            schedule_tick();
            0
        }
        WM_TIMER if wparam == TIMER_TIMELAPSE => {
            timelapse_tick();
            0
        }
        WM_TIMER if wparam == TIMER_PRESET => {
            KillTimer(hwnd, TIMER_PRESET);
            if let Some(i) = with_app(|app| app.pending_preset).flatten() {
                run_preset(i);
                with_app(|app| app.pending_preset = None);
            }
            0
        }
        WM_TIMER if wparam == TIMER_INITIAL => {
            KillTimer(hwnd, TIMER_INITIAL);
            if let Some(mode) = with_app(|app| app.pending.take()).flatten() {
                if mode == OPEN_CAPTURE_WINDOW {
                    crate::show_capture_window();
                } else {
                    dispatch_mode(mode);
                }
            }
            0
        }
        WM_TIMER if wparam == TIMER_HOTKEY_RETRY => {
            retry_print_screen(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Runs the tray app until Exit, first starting the capture `initial` asks
/// for (a mode, or `usize::MAX` for menus). Returns the exit code.
pub fn run(initial: Option<usize>) -> i32 {
    let class = wide(CLASS);
    // SAFETY: standard window class and hidden window setup; the class name
    // outlives the window because `class` lives until the end of `run`.
    let hwnd = unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassExW(&wc);
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class.as_ptr(),
            wide("Jiffshot Capture").as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        log("could not create the tray window");
        return 1;
    }

    let (capture_key, problems) = register_hotkeys(hwnd);
    // Which pause and stop keys recordings will use, for the windows that
    // show them.
    crate::recorder::check_keys();
    log(&format!("capture side started; capture key {capture_key}"));
    for problem in &problems {
        log(problem);
    }
    if !problems.is_empty() {
        notify(hwnd, "Jiffshot hotkeys", &problems.join(". "), true);
    }
    if capture_key != "PrtScn" {
        // SAFETY: a timer on our own window.
        unsafe { SetTimer(hwnd, TIMER_HOTKEY_RETRY, 30_000, None) };
    }
    APP.with(|app| {
        *app.borrow_mut() = Some(App {
            hwnd,
            settings: Settings::load(),
            busy: false,
            last_region: None,
            capture_key,
            pending: initial,
            timelapse: None,
            pending_preset: None,
            override_cursor: None,
            override_effects: None,
            override_share: None,
            scheduled_fired: None,
            pending_mode: None,
        })
    });
    publish_state();
    let settings_now = with_app(|app| app.settings.clone()).unwrap_or_default();
    register_preset_hotkeys(hwnd, &settings_now.presets);
    arm_schedule(hwnd, settings_now.schedule.as_deref());
    if initial.is_some() {
        // Give the window that asked (the editor) time to get out of the way.
        // SAFETY: a timer on our own window.
        unsafe { SetTimer(hwnd, TIMER_INITIAL, 350, None) };
    }

    // SAFETY: the standard message loop for this thread, then cleanup of the
    // icon and hotkeys registered above.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        for id in [HOTKEY_CAPTURE, HOTKEY_SCREEN, HOTKEY_REPEAT] {
            UnregisterHotKey(hwnd, id);
        }
    }
    log("tray stopped");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_parse() {
        assert_eq!(parse_hotkey("Ctrl+Shift+1"), Some((MOD_CONTROL | MOD_SHIFT, b'1' as u16)));
        assert_eq!(parse_hotkey("alt + F9"), Some((MOD_ALT, 0x78)));
        assert_eq!(parse_hotkey("PrtScn"), Some((0, VK_SNAPSHOT)));
        assert_eq!(parse_hotkey("Ctrl+"), None);
        assert_eq!(parse_hotkey("Hyper+K"), None);
    }

    #[test]
    fn preset_modes_resolve() {
        for p in crate::settings::Settings::default().presets {
            assert!(mode_named(&p.mode).is_some(), "{}", p.mode);
        }
        assert_eq!(mode_named("steps"), Some(MODE_STEPS));
    }
}

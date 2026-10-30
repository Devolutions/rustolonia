//! Jiffshot's capture side: the tray, hotkeys, the selection overlay,
//! recording, and the workflow after a capture. `jiffshot.exe` runs it on its
//! own thread ([`start_engine`]) next to the editor, in the same process.
//!
//! Command lines `jiffshot.exe` understands (see [`run_command`]):
//!
//! ```text
//! jiffshot                              tray (or, if already running, the Capture window)
//! jiffshot FILE...                      open files in the editor
//! jiffshot --capture|--scrolling|--panoramic|--text|--record|--menu|--steps|--freehand|--window
//!                                       start that capture (starting Jiffshot if needed)
//! jiffshot --grab X,Y,W,H OUT           capture a screen rectangle to a PNG
//! jiffshot --grab screen OUT            capture the monitor under the cursor
//! jiffshot --monitors                   list monitors
//! jiffshot --list-windows               list detected capture targets
//! jiffshot --selftest DIR               drive the overlay offscreen, write frames and timings
//! jiffshot --save-test X,Y,W,H DIR      save to a library in DIR and open the editor
//! jiffshot --ocr IMAGE.png              print the text recognized in an image
//! jiffshot --scroll-selftest DIR        scrolling capture of a generated page (moves the pointer)
//! jiffshot --record-selftest DIR SECS   record a generated page
//! jiffshot --quit                       stop the running instance
//! jiffshot --write-icon OUT.ico         write the program icon (assets/jiffshot.ico)
//! ```

mod hud;
mod screendraw;
mod stepcap;
mod vfx;
mod icon;
mod modes;
mod overlay;
mod recorder;
mod selftest_scroll;
pub mod panel;
pub mod prefs;
pub mod settings;
mod tray;
pub mod ui_api;
mod workflow;

use overlay::Overlay;
use jiffshot_capture::screen::{self, Monitor};
use jiffshot_capture::targets::Targets;
use jiffshot_capture::{wide, Frame, Rect};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
use windows_sys::Win32::System::Threading::CreateMutexW;
use std::sync::OnceLock;
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};

/// What the capture side asks of the app it runs in.
pub struct Hooks {
    /// Opens a saved capture in the editor; false when it could not.
    pub open_in_editor: Box<dyn Fn(&Path) -> bool + Send + Sync>,
    /// Shows the editor window (its library when nothing is open).
    pub show_editor: Box<dyn Fn() + Send + Sync>,
    /// The tray's Exit, or the engine stopping: ends the whole app.
    pub quit: Box<dyn Fn() + Send + Sync>,
    /// A notice for the user (title, text, whether it reports a problem).
    pub notify: Box<dyn Fn(&str, &str, bool) + Send + Sync>,
    /// What the tray menu should show, sent after anything in it changes.
    pub state: Box<dyn Fn(TrayState) + Send + Sync>,
    /// Shows the Capture window.
    pub show_capture_window: Box<dyn Fn() + Send + Sync>,
    /// Shows text recognized by Grab Text.
    pub show_text: Box<dyn Fn(String) + Send + Sync>,
    /// The capture panels and outlines on screen, as JSON (see `hud.rs`),
    /// sent after every change; the panels' buttons answer through
    /// [`answer_panel`].
    pub hud: Box<dyn Fn(String) + Send + Sync>,
    /// The selection overlay to show, as JSON (session, desktop, monitors;
    /// see `overlay.rs`), or empty to take it down. The app draws it and
    /// feeds it input through [`ui_api`].
    pub overlay: Box<dyn Fn(String) + Send + Sync>,
    /// Screen Draw to show over an area while recording, as JSON (session,
    /// area), or empty to end it.
    pub screen_draw: Box<dyn Fn(String) + Send + Sync>,
}

/// Button `button` of capture panel `id` was pressed (`None`: the panel was
/// closed some other way); safe from any thread.
pub fn answer_panel(id: u64, button: Option<usize>) {
    hud::answer(id, button);
}

/// A button of toolbar `id` (the recording toolbar) sent `action`; safe
/// from any thread.
pub fn panel_action(id: u64, action: &str) {
    hud::act(id, action.to_owned());
}

/// The capture side's settings as last published (see [`settings_snapshot`]).
pub(crate) static SNAPSHOT: std::sync::Mutex<Option<settings::Settings>> = std::sync::Mutex::new(None);

/// The capture side's settings, as of its last change.
pub fn settings_snapshot() -> settings::Settings {
    SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_else(settings::Settings::load)
}

/// Runs `f` on the capture side's thread, where its state lives; safe from
/// any thread. Dropped when the capture side is not running.
pub fn on_capture_thread(f: impl FnOnce() + Send + 'static) {
    let boxed: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(f));
    // SAFETY: posts to our own tray window, found by class; the box is
    // reclaimed by its handler, or here when the post fails.
    unsafe {
        let hwnd = FindWindowW(wide(tray::CLASS).as_ptr(), std::ptr::null());
        let raw = Box::into_raw(boxed);
        if hwnd.is_null() || PostMessageW(hwnd, tray::WM_APP_RUN, 0, raw as isize) == 0 {
            drop(Box::from_raw(raw));
        }
    }
}

/// Shows the Capture window (through the app).
pub(crate) fn show_capture_window() {
    if let Some(hooks) = hooks() {
        (hooks.show_capture_window)();
    }
}

/// Shows Grab Text's result (through the app).
pub(crate) fn show_text(text: &str) {
    if let Some(hooks) = hooks() {
        (hooks.show_text)(text.to_owned());
    }
}

/// What the Capture window asks the capture side for; each runs on its
/// thread.
pub mod capture {
    use super::{on_capture_thread, settings::Preset, settings::Settings, tray};

    /// Saves the window's settings.
    pub fn apply(s: Settings) {
        on_capture_thread(move || tray::apply_settings(s));
    }

    /// Saves the settings and starts the window's capture (see
    /// `tray::start_from_panel`).
    pub fn start(s: Settings) {
        on_capture_thread(move || tray::start_from_panel(s));
    }

    pub fn run_preset(index: usize) {
        on_capture_thread(move || tray::run_preset_now(index));
    }

    pub fn set_presets(presets: Vec<Preset>) {
        on_capture_thread(move || tray::set_presets(presets));
    }

    pub fn step_capture() {
        on_capture_thread(|| tray::start_mode_after_delay("steps"));
    }

    /// Moves a daily schedule that is on to `at`.
    pub fn move_schedule(at: String) {
        on_capture_thread(move || tray::set_schedule(Some(at)));
    }

    pub fn set_run_at_startup(on: bool) {
        on_capture_thread(move || {
            tray::set_run_at_startup(on);
            tray::publish_state();
        });
    }

    pub fn runs_at_startup() -> bool {
        tray::runs_at_startup()
    }

    pub fn open_library() {
        on_capture_thread(tray::open_library);
    }
}

static HOOKS: OnceLock<Hooks> = OnceLock::new();

/// Capture modes the editor asks for with [`request_capture`].
pub use tray::{TrayAction, TrayState, MODE_IMAGE, MODE_SCROLLING, MODE_TEXT, MODE_VIDEO, OPEN_CAPTURE_WINDOW};

/// The app's hooks; `None` for command-line modes, which run without an app.
pub(crate) fn hooks() -> Option<&'static Hooks> {
    HOOKS.get()
}

/// Appends a line to `%LOCALAPPDATA%\Jiffshot\capture.log` (and stderr).
pub fn log(line: &str) {
    let line = format!("{} {line}", jiffshot_capture::library::timestamp());
    eprintln!("{line}");
    let dir = jiffshot_capture::paths::local_dir();
    {
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("capture.log")) {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn parse_rect(text: &str) -> Option<Rect> {
    let v: Vec<i32> = text.split(',').map(|p| p.trim().parse().ok()).collect::<Option<_>>()?;
    match v.as_slice() {
        [x, y, w, h] => Some(Rect::new(*x, *y, *w, *h)),
        _ => None,
    }
}

fn print_monitors(monitors: &[Monitor]) {
    for m in monitors {
        println!(
            "{} {:?} work {:?} dpi {} ({:.0}%){}",
            m.device,
            m.rect,
            m.work,
            m.dpi,
            m.scale() * 100.0,
            if m.primary { " primary" } else { "" }
        );
    }
}

fn list_windows() {
    let targets = Targets::enumerate(screen::virtual_screen(), &[]);
    for t in &targets.list {
        let title: String = t.title.chars().take(70).collect();
        println!("{}{:?} {} {:?}", "  ".repeat(t.depth as usize), t.rect, t.class, title);
    }
    println!("{} targets", targets.list.len());
}

fn grab(spec: &str, out: &Path) -> std::io::Result<()> {
    let rect = if spec == "screen" {
        let mut point = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
        // SAFETY: out-parameter only.
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point) };
        screen::monitor_at(point.x, point.y).map(|m| m.rect)
    } else {
        parse_rect(spec)
    }
    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "expected X,Y,W,H or 'screen'"))?;
    let started = Instant::now();
    let frame = screen::grab(rect)?;
    let grabbed = started.elapsed();
    frame.save_png(out)?;
    let saved = started.elapsed() - grabbed;
    println!(
        "{rect:?} -> {} ({}x{}): grab {:.1} ms, PNG {:.1} ms, {} KiB",
        out.display(),
        frame.width,
        frame.height,
        ms(grabbed),
        ms(saved),
        std::fs::metadata(out)?.len() / 1024
    );
    Ok(())
}

struct Timing {
    samples: Vec<f64>,
}

impl Timing {
    fn new() -> Timing {
        Timing { samples: Vec::new() }
    }

    fn time<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let started = Instant::now();
        let r = f();
        self.samples.push(ms(started.elapsed()));
        r
    }

    fn summary(&self) -> String {
        let mut s = self.samples.clone();
        s.sort_by(f64::total_cmp);
        let avg = s.iter().sum::<f64>() / s.len().max(1) as f64;
        let p95 = s.get((s.len() as f64 * 0.95) as usize).or(s.last()).copied().unwrap_or(0.0);
        format!("{} updates, avg {avg:.2} ms, p95 {p95:.2} ms, max {:.2} ms", s.len(), s.last().copied().unwrap_or(0.0))
    }
}

/// Prints what the overlay would show (the app draws it; the UI self-test
/// renders it).
fn print_view(overlay: &mut Overlay) {
    let v = overlay.view();
    println!("  shows {:?} ({:?}, {}) on {:?} at {}x", v.lit, v.title, v.size, v.monitor, v.scale);
}

/// Builds the overlay exactly as the hotkey does, then feeds it scripted
/// input without showing a window, printing what it shows and timings.
fn selftest(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let desktop = screen::virtual_screen();
    let monitors = screen::monitors();
    println!("desktop {desktop:?}");
    print_monitors(&monitors);

    let started = Instant::now();
    let snapshot = screen::grab(desktop)?;
    let grabbed = started.elapsed();
    let targets = Targets::enumerate(desktop, &[]);
    let enumerated = started.elapsed();
    let primary = monitors.iter().find(|m| m.primary).or(monitors.first()).expect("a monitor").clone();
    let start = primary.rect.center();
    let mut overlay = Overlay::new(snapshot, (desktop.x, desktop.y), targets, monitors.clone(), start)?;
    println!(
        "overlay ready in {:.1} ms: grab {:.1} ms, {} targets in {:.1} ms, setup {:.1} ms",
        ms(started.elapsed()),
        ms(grabbed),
        overlay.targets.list.len(),
        ms(enumerated - grabbed),
        ms(started.elapsed() - enumerated)
    );
    let origin = overlay.origin;
    let client = |(x, y): (i32, i32)| (x - origin.0, y - origin.1);

    // Hover over the largest top-level window on the primary monitor.
    let window = overlay
        .targets
        .list
        .iter()
        .filter(|t| t.depth == 0 && primary.rect.intersect(t.rect).is_some_and(|r| r.area() * 2 > t.rect.area()))
        .max_by_key(|t| t.rect.area().min(primary.rect.area() * 9 / 10))
        .map(|t| t.rect);
    if let Some(rect) = window {
        let point = rect.inflate(-rect.width.min(rect.height) / 5).center();
        let (x, y) = client(point);
        overlay.pointer_moved(x, y);
        println!("hover {point:?}: {}", overlay.describe());
        print_view(&mut overlay);
        overlay.wheel(-120);
        overlay.wheel(120);
        println!("wheel up: {}", overlay.describe());
        overlay.wheel(-120);
        println!("wheel down: {}", overlay.describe());
    } else {
        println!("no window on the primary monitor to hover");
    }

    // Sweep the pointer across the primary monitor.
    let mut hover = Timing::new();
    let r = primary.rect;
    for i in 0..600 {
        let t = i as f64 / 599.0;
        let p = (r.x + (r.width as f64 * t) as i32, r.y + (r.height as f64 * (0.5 + 0.45 * (t * 12.0).sin())) as i32);
        let (x, y) = client(p);
        hover.time(|| {
            overlay.pointer_moved(x, y);
            overlay.view()
        });
    }
    println!("hover sweep: {}", hover.summary());

    // Drag a region.
    let a = (r.x + r.width / 5, r.y + r.height / 5);
    let b = (r.x + r.width * 3 / 5, r.y + r.height * 3 / 5);
    let (ax, ay) = client(a);
    overlay.pointer_pressed(ax, ay);
    let mut drag = Timing::new();
    for i in 1..=120 {
        let x = a.0 + (b.0 - a.0) * i / 120;
        let y = a.1 + (b.1 - a.1) * i / 120;
        let (cx, cy) = client((x, y));
        drag.time(|| {
            overlay.pointer_moved(cx, cy);
            overlay.view()
        });
    }
    println!("drag: {}", drag.summary());
    print_view(&mut overlay);
    let (bx, by) = client(b);
    overlay.pointer_released(bx, by);
    let selection = overlay.outcome.clone().flatten().expect("drag produces a region");
    println!("selection: {selection:?}");

    let finish = Instant::now();
    let capture = workflow::from_overlay(&overlay, &selection, None, false, false).expect("selection inside desktop");
    let cropped = finish.elapsed();
    let out = dir.join("3-selection.png");
    capture.frame.save_png(&out)?;
    println!(
        "crop {:.1} ms, PNG {:.1} ms for {}x{} -> {}",
        ms(cropped),
        ms(finish.elapsed() - cropped),
        capture.frame.width,
        capture.frame.height,
        out.display()
    );

    // Keyboard: F picks the monitor under the cursor.
    overlay.outcome = None;
    overlay.key(b'F' as u16, false);
    println!("key F: {:?}", overlay.outcome.clone().flatten());

    // Freehand: a circle; outside it the capture is transparent.
    overlay.outcome = None;
    overlay.mode = overlay::Mode::Freehand;
    let (cx, cy) = client(r.center());
    let radius = r.height / 4;
    overlay.pointer_pressed(cx + radius, cy);
    for i in 1..=90 {
        let t = i as f64 / 90.0 * std::f64::consts::TAU;
        overlay.pointer_moved(cx + (radius as f64 * t.cos()) as i32, cy + (radius as f64 * t.sin()) as i32);
    }
    overlay.pointer_released(cx + radius, cy);
    let freehand = overlay.outcome.clone().flatten().expect("freehand selection");
    let capture = workflow::from_overlay(&overlay, &freehand, None, false, false).expect("freehand capture");
    let px = |f: &Frame, x: u32, y: u32| f.pixels[(y * f.width + x) as usize * 4 + 3];
    let (w, h) = (capture.frame.width, capture.frame.height);
    println!("freehand: {w}x{h}, corner alpha {}, center alpha {}", px(&capture.frame, 0, 0), px(&capture.frame, w / 2, h / 2));
    assert!(w.abs_diff(2 * radius as u32) <= 3 && h.abs_diff(2 * radius as u32) <= 3, "freehand bounds follow the path");
    assert!(px(&capture.frame, 0, 0) == 0 && px(&capture.frame, w / 2, h / 2) == 255, "freehand masks outside the path");

    // Multiple areas: two drags, Backspace removes one, a third, Enter.
    overlay.outcome = None;
    overlay.mode = overlay::Mode::MultiArea;
    let drag = |o: &mut Overlay, a: (i32, i32), b: (i32, i32)| {
        o.pointer_pressed(a.0, a.1);
        o.pointer_moved((a.0 + b.0) / 2, (a.1 + b.1) / 2);
        o.pointer_moved(b.0, b.1);
        o.pointer_released(b.0, b.1);
    };
    let (ox, oy) = client((r.x, r.y));
    drag(&mut overlay, (ox + 100, oy + 100), (ox + 300, oy + 200));
    drag(&mut overlay, (ox + 400, oy + 400), (ox + 500, oy + 450));
    overlay.key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_BACK, false);
    drag(&mut overlay, (ox + 500, oy + 300), (ox + 600, oy + 380));
    overlay.key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RETURN, false);
    let multi = overlay.outcome.clone().flatten().expect("multi-area selection");
    let capture = workflow::from_overlay(&overlay, &multi, None, false, false).expect("multi capture");
    let (w, h) = (capture.frame.width, capture.frame.height);
    println!("multi: {multi:?} -> {w}x{h}");
    assert_eq!((w, h), (501, 281), "two areas kept after Backspace (drags include the end pixel)");
    assert!(px(&capture.frame, 1, 1) == 255 && px(&capture.frame, 1, h - 2) == 0, "gaps between areas are transparent");

    // Fixed size: a click places the area around the pointer.
    overlay.outcome = None;
    overlay.mode = overlay::Mode::Fixed(640, 360);
    overlay.pointer_moved(cx, cy);
    overlay.pointer_pressed(cx, cy);
    overlay.pointer_released(cx, cy);
    let fixed = overlay.outcome.clone().flatten().expect("fixed selection");
    println!("fixed: {fixed:?}");
    assert_eq!((fixed.rect().width, fixed.rect().height), (640, 360));

    // Capture effects run on the frame before it is saved.
    let mut frame = capture.frame.clone();
    workflow::apply_effects(&mut frame, &["Border".to_owned()]);
    println!("effects: {}x{} -> {}x{} with a border", w, h, frame.width, frame.height);
    assert!(frame.width >= w && frame.height >= h);
    Ok(())
}

/// Runs the post-capture workflow (library save, editor hand-off) into
/// `dir`, with the clipboard left alone.
fn save_test(spec: &str, dir: &Path) -> std::io::Result<()> {
    let rect = parse_rect(spec).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "expected X,Y,W,H"))?;
    let capture = workflow::immediate(jiffshot_capture::library::CaptureKind::Region, rect, true)?;
    // The library goes where the test says (this command runs alone).
    std::env::set_var("JIFFSHOT_LIBRARY", dir);
    let settings = settings::Settings {
        copy_to_clipboard: false,
        ..settings::Settings::default()
    };
    let outcome = workflow::finish(capture, &settings, 0);
    println!("{} (editor opened: {})", outcome.message, outcome.opened_editor);
    Ok(())
}

fn ocr(path: &Path) -> std::io::Result<()> {
    let frame = Frame::load_png(path)?;
    let started = Instant::now();
    let lines = jiffshot_capture::ocr::recognize(frame.width, frame.height, &frame.pixels).map_err(std::io::Error::other)?;
    for line in &lines {
        println!("{:?} {}", line.rect, line.text);
    }
    println!("{} lines in {:.0} ms", lines.len(), ms(started.elapsed()));
    Ok(())
}

fn usage() {
    println!("{}", include_str!("lib.rs").lines().take_while(|l| l.starts_with("//!")).map(|l| l.trim_start_matches("//!").trim_start_matches(' ')).collect::<Vec<_>>().join("\n"));
}

/// The capture a command-line flag asks for (`--window` opens the Capture
/// window); `None` for anything else.
pub fn capture_flag(args: &[String]) -> Option<usize> {
    Some(match args {
        [flag] => match flag.as_str() {
            "--capture" => tray::MODE_IMAGE,
            "--scrolling" => tray::MODE_SCROLLING,
            "--panoramic" => tray::MODE_PANORAMIC,
            "--text" => tray::MODE_TEXT,
            "--record" => tray::MODE_VIDEO,
            "--menu" => usize::MAX,
            "--steps" => tray::MODE_STEPS,
            "--window" => tray::OPEN_CAPTURE_WINDOW,
            "--freehand" => tray::MODE_FREEHAND,
            _ => return None,
        },
        _ => return None,
    })
}

/// Claims the one running Jiffshot for this session; false when another
/// process already has it (then [`forward_to_running`] hands it the request).
pub fn claim_instance() -> bool {
    // SAFETY: a named mutex for single-instance detection; the handle is
    // intentionally held for the life of the process.
    unsafe {
        CreateMutexW(std::ptr::null(), 0, wide("Local\\Jiffshot.Capture").as_ptr());
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

/// Asks the running Jiffshot to start `mode` (a [`capture_flag`]), or to
/// open its Capture window when there is none.
pub fn forward_to_running(mode: Option<usize>) {
    // SAFETY: looks up the running instance's tray window by class.
    unsafe {
        let hwnd = FindWindowW(wide(tray::CLASS).as_ptr(), std::ptr::null());
        if !hwnd.is_null() {
            // Pass on this launch's right to take the foreground (a shortcut
            // or a click started it), so the overlay gets the keyboard.
            let mut process = 0;
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, &mut process);
            windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(process);
            match mode {
                Some(usize::MAX) => PostMessageW(hwnd, tray::WM_APP_MENU, 0, 0),
                m => PostMessageW(hwnd, tray::WM_APP_CAPTURE, m.unwrap_or(tray::OPEN_CAPTURE_WINDOW), 0),
            };
        }
    }
}

/// Starts the capture side on its own thread: the tray icon, hotkeys and
/// everything they open, with `initial` started once it is up. When it stops
/// (the tray's Exit), `app.quit` runs on that thread.
pub fn start_engine(initial: Option<usize>, app: Hooks) -> std::io::Result<std::thread::JoinHandle<i32>> {
    let _ = HOOKS.set(app);
    std::thread::Builder::new().name("jiffshot-capture".into()).spawn(move || {
        let code = tray::run(initial);
        if let Some(hooks) = hooks() {
            (hooks.quit)();
        }
        code
    })
}

/// Asks the capture side to stop (as its Exit does); safe from any thread.
pub fn stop_engine() {
    // SAFETY: posts to our own tray window, found by class.
    unsafe {
        let hwnd = FindWindowW(wide(tray::CLASS).as_ptr(), std::ptr::null());
        if !hwnd.is_null() {
            PostMessageW(hwnd, windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE, 0, 0);
        }
    }
}

/// Starts a capture (by [`capture_flag`] mode) from the editor after
/// `delay_ms`, so its window can get out of the way first.
pub fn request_capture(mode: usize, delay_ms: u32) {
    // SAFETY: posts to our own tray window, found by class.
    unsafe {
        let hwnd = FindWindowW(wide(tray::CLASS).as_ptr(), std::ptr::null());
        if !hwnd.is_null() {
            PostMessageW(hwnd, tray::WM_APP_CAPTURE_LATER, mode, delay_ms as isize);
        }
    }
}

/// Runs a command-line mode that does its work and exits (`--grab`,
/// `--selftest` and the rest), returning its exit code; `None` when `args`
/// ask for the app itself (nothing, a capture flag, or files to open).
pub fn run_command(args: &[String]) -> Option<i32> {
    if args.is_empty() || capture_flag(args).is_some() || !args[0].starts_with("--") {
        return None;
    }
    // SAFETY: attaches to the launching console so CLI output is visible
    // from a GUI-subsystem build; harmless when there is none.
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    screen::enable_per_monitor_dpi();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["--write-icon", out] => std::fs::write(out, icon::ico()),
        ["--monitors"] => {
            print_monitors(&screen::monitors());
            Ok(())
        }
        ["--list-windows"] => {
            list_windows();
            Ok(())
        }
        ["--grab", spec, out] => grab(spec, Path::new(out)),
        ["--quit"] => {
            // SAFETY: asks the running instance's tray window to close.
            unsafe {
                let hwnd = FindWindowW(wide(tray::CLASS).as_ptr(), std::ptr::null());
                if !hwnd.is_null() {
                    PostMessageW(hwnd, windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE, 0, 0);
                }
            }
            Ok(())
        }
        ["--save-test", spec, dir] => save_test(spec, Path::new(dir)),
        ["--ocr", file] => ocr(Path::new(file)),
        ["--scroll-selftest", dir] => selftest_scroll::run(Path::new(dir)),
        ["--menu-selftest", dir] => selftest_scroll::menus(Path::new(dir)),
        ["--record-selftest", dir, seconds] => selftest_scroll::record(Path::new(dir), seconds.parse().unwrap_or(3.0)),
        ["--selftest", dir] => selftest(Path::new(dir)),
        _ => {
            usage();
            Ok(())
        }
    };
    Some(match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    })
}

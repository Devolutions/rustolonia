//! A scripted walk through the editor for testing without a mouse: draws one
//! object with each tool through the canvas, edits properties through the
//! view model, opens the other windows, and writes window screenshots, the
//! exported image and a log to the directory named by `JIFFSHOT_UITEST`.
//! `JIFFSHOT_UITEST_IMAGE` optionally names the image to annotate (and turns
//! on the image-tool, template and window steps), `JIFFSHOT_UITEST_VIDEO` a
//! video to step through; `JIFFSHOT_UITEST_KEEP` leaves the editor open.
//!
//! The steps run one after another on the UI thread, each followed by a pause
//! so the windows can render; screenshots come from the windows themselves
//! (`PrintWindow`), so they work for windows left out of screen captures.

use super::overlay::overlay_api as api;
use super::UiOnly;
use crate::canvas::{POINTER_MOVED, POINTER_PRESSED, POINTER_RELEASED};
use crate::vm::{MainViewModel, MainViewModelSink};
use jiffshot_capture::gdi::Dib;
use jiffshot_capture::Rect;
use rustolonia::{AppScope, Signal};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
use windows_sys::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowDisplayAffinity, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, SetWindowDisplayAffinity, WindowFromPoint, WDA_NONE, WM_CLOSE,
};

/// `PW_RENDERFULLCONTENT`: what the compositor shows, for GPU-drawn windows.
const PW_RENDERFULLCONTENT: PRINT_WINDOW_FLAGS = 2;

/// What the steps share.
struct Test {
    dir: PathBuf,
    log: String,
    /// The document size after the first fit.
    w: f64,
    h: f64,
    /// Values a step saves for a later one.
    note: String,
    windows_before: Vec<isize>,
}

type Step = Box<dyn FnOnce(&mut Test) -> u64>;

/// The steps still to run, and the pause after each.
struct Runner {
    scope: AppScope,
    test: Test,
    steps: VecDeque<Step>,
}

fn sink() -> MainViewModelSink {
    super::main().sink
}

/// Sends `action` to the model, as the window's controls do.
fn run(action: impl Into<String>) {
    let action = action.into();
    super::main().model.run(|m| m.run(action));
}

fn call(f: impl FnOnce(&mut dyn MainViewModel) -> rustolonia::Result<()>) {
    super::main().model.run(f);
}

/// Changes a property as a control would: the model's setter runs.
fn set<T: 'static>(signal: Signal<T>, value: T) {
    signal.set(value);
}

fn canvas_feed(kind: i32, x: f64, y: f64) {
    if let Some(canvas) = super::canvas() {
        canvas.simulate(kind, x, y, 0);
    }
}

// ---- windows ------------------------------------------------------------------

/// This process's visible top-level windows.
fn process_windows() -> Vec<isize> {
    unsafe extern "system" fn each(hwnd: HWND, list: LPARAM) -> i32 {
        let mut pid = 0;
        // SAFETY: `hwnd` comes from EnumWindows; `list` is the Vec below.
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == GetCurrentProcessId() && IsWindowVisible(hwnd) != 0 {
                (*(list as *mut Vec<isize>)).push(hwnd as isize);
            }
        }
        1
    }
    let mut list: Vec<isize> = Vec::new();
    // SAFETY: the callback only runs during the call, with `list` alive.
    unsafe { EnumWindows(Some(each), &mut list as *mut Vec<isize> as LPARAM) };
    list
}

fn window_rect(hwnd: isize) -> Rect {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: a plain query; a stale handle only fails.
    unsafe { GetWindowRect(hwnd as HWND, &mut r) };
    Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
}

/// The window's display affinity (0x11: left out of captures).
fn affinity(hwnd: isize) -> u32 {
    let mut a = 0;
    // SAFETY: a plain query.
    unsafe { GetWindowDisplayAffinity(hwnd as HWND, &mut a) };
    a
}

/// What the screen shows over `r`, with `hwnd` not left out of captures
/// while it is grabbed.
fn grab_shown(hwnd: isize, r: Rect) -> std::io::Result<jiffshot_capture::frame::Frame> {
    let was = affinity(hwnd);
    // SAFETY: changing, then restoring, one of this process's windows.
    unsafe { SetWindowDisplayAffinity(hwnd as HWND, WDA_NONE) };
    // The compositor needs a frame to show the window in captures.
    std::thread::sleep(Duration::from_millis(120));
    let frame = jiffshot_capture::screen::grab(r);
    unsafe { SetWindowDisplayAffinity(hwnd as HWND, was) };
    frame
}

/// "#RRGGBB, hits: yes|no": what is on screen at a point, and whether it is
/// `hwnd` there.
fn probe(x: i32, y: i32, hwnd: isize) -> String {
    // SAFETY: the screen DC is released before returning.
    let (color, hit) = unsafe {
        let dc = GetDC(std::ptr::null_mut());
        let color = GetPixel(dc, x, y);
        ReleaseDC(std::ptr::null_mut(), dc);
        (color, WindowFromPoint(POINT { x, y }) as isize == hwnd)
    };
    format!("#{:02X}{:02X}{:02X}, hits: {}", color & 0xFF, (color >> 8) & 0xFF, (color >> 16) & 0xFF, if hit { "yes" } else { "no" })
}

impl Test {
    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Saves what `hwnd` shows as `name`.
    fn shot_of(&mut self, hwnd: isize, name: &str) {
        let r = window_rect(hwnd);
        let result = Dib::new(r.width.max(1), r.height.max(1)).and_then(|dib| {
            // SAFETY: the DIB's DC is valid for the DIB's life.
            if unsafe { PrintWindow(hwnd as HWND, dib.dc(), PW_RENDERFULLCONTENT) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
            let mut frame = dib.to_frame();
            // A per-pixel transparent window prints black: take it from the
            // screen instead, with its capture exclusion lifted meanwhile.
            if frame.pixels.chunks_exact(4).all(|p| p[..3] == [0, 0, 0]) {
                frame = grab_shown(hwnd, r)?;
            }
            frame.save_png(&self.path(name))
        });
        match result {
            Ok(()) => writeln!(self.log, "wrote {}", self.path(name).display()),
            Err(e) => writeln!(self.log, "{name}: {e}"),
        }
        .ok();
    }

    /// Saves the editor window.
    fn shot(&mut self, name: &str) {
        match super::main_window_handle() {
            Some(hwnd) => self.shot_of(hwnd, name),
            None => self.line(format!("{name}: no editor window")),
        }
    }

    /// The windows shown since [`Test::remember_windows`].
    fn new_windows(&self) -> Vec<isize> {
        process_windows().into_iter().filter(|w| !self.windows_before.contains(w)).collect()
    }

    fn remember_windows(&mut self) {
        self.windows_before = process_windows();
    }

    fn line(&mut self, text: impl AsRef<str>) {
        self.log.push_str(text.as_ref());
        self.log.push('\n');
    }

    /// Keeps the log on disk current, so a hang shows where it stopped.
    fn flush(&self) {
        let _ = std::fs::write(self.path("ui-log.txt"), &self.log);
    }

    fn describe(&self) -> String {
        let s = sink();
        format!("selection={} panel={} status={}", s.selection.get_untracked(), s.panel.get_untracked(), s.status.get_untracked())
    }
}

// ---- the script ---------------------------------------------------------------

/// Builds the steps; each returns how long to wait after it, in milliseconds.
struct Script(VecDeque<Step>);

impl Script {
    fn step(&mut self, f: impl FnOnce(&mut Test) -> u64 + 'static) -> &mut Self {
        self.0.push_back(Box::new(f));
        self
    }

    fn wait(&mut self, ms: u64) -> &mut Self {
        self.step(move |_| ms)
    }

    /// Drags with `tool` between two points given as fractions of the document.
    fn drag(&mut self, tool: &'static str, a: (f64, f64), b: (f64, f64), steps: u32) -> &mut Self {
        self.step(move |t| {
            run(format!("tool:{tool}"));
            let (x0, y0, x1, y1) = (t.w * a.0, t.h * a.1, t.w * b.0, t.h * b.1);
            canvas_feed(POINTER_PRESSED, x0, y0);
            for i in 1..=steps {
                let f = f64::from(i) / f64::from(steps);
                canvas_feed(POINTER_MOVED, x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
            }
            canvas_feed(POINTER_RELEASED, x1, y1);
            120
        })
        .step(move |t| {
            let line = format!("{tool}: {}", t.describe());
            t.line(line);
            0
        })
    }

    fn click(&mut self, tool: &'static str, at: (f64, f64)) -> &mut Self {
        self.step(move |t| {
            run(format!("tool:{tool}"));
            canvas_feed(POINTER_PRESSED, t.w * at.0, t.h * at.1);
            canvas_feed(POINTER_RELEASED, t.w * at.0, t.h * at.1);
            120
        })
        .step(move |t| {
            let line = format!("{tool} click: selection={}", sink().selection.get_untracked());
            t.line(line);
            0
        })
    }

    fn shot(&mut self, name: &'static str) -> &mut Self {
        self.wait(250).step(move |t| {
            t.shot(name);
            0
        })
    }

    fn log(&mut self, f: impl FnOnce(&mut Test) -> String + 'static) -> &mut Self {
        self.step(move |t| {
            let line = f(t);
            t.line(line);
            0
        })
    }

    /// Runs `open`, then saves each window it showed and closes it.
    fn shot_new_windows(&mut self, name: &'static str, open: impl FnOnce(&mut Test) + 'static) -> &mut Self {
        self.step(move |t| {
            t.remember_windows();
            open(t);
            500
        })
        .step(move |t| {
            let shown = t.new_windows();
            if shown.is_empty() {
                t.line(format!("{name}: no window shown"));
            }
            for (i, hwnd) in shown.iter().enumerate() {
                let file = if i == 0 { name.to_owned() } else { name.replace(".png", &format!("-{i}.png")) };
                t.shot_of(*hwnd, &file);
            }
            for hwnd in shown {
                // SAFETY: posting to one of this process's windows.
                unsafe { PostMessageW(hwnd as HWND, WM_CLOSE, 0, 0) };
            }
            200
        })
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn editor_walk(s: &mut Script) {
    let image = env("JIFFSHOT_UITEST_IMAGE");
    s.wait(800);
    if let Some(image) = image.clone() {
        s.step(move |_| {
            run(format!("open:{image}"));
            600
        });
    }
    s.step(|t| {
        run("zoom:fit");
        let (w, h) = super::canvas().map(|c| c.document_size()).unwrap_or((0, 0));
        (t.w, t.h) = (f64::from(w), f64::from(h));
        let zoom = super::canvas().map(|c| c.zoom()).unwrap_or(0.0);
        t.line(format!("document {w}x{h}, zoom {zoom:.2}"));
        120
    });
}

/// The whole walk, in order.
fn script() -> Script {
    let mut s = Script(VecDeque::new());
    let image = env("JIFFSHOT_UITEST_IMAGE");
    editor_walk(&mut s);
    let k = sink();
    s.drag("Arrow", (0.08, 0.12), (0.30, 0.30), 12)
        .drag("Shape", (0.35, 0.10), (0.55, 0.30), 12)
        .step(move |_| {
            set(k.variant, "Rounded".into());
            set(k.stroke_color, "#FF1E88E5".into());
            120
        })
        .drag("Text", (0.60, 0.08), (0.92, 0.18), 12)
        .step(move |_| {
            set(k.text, "Jiffshot text".into());
            120
        })
        .drag("Callout", (0.06, 0.42), (0.30, 0.58), 12)
        .step(move |_| {
            set(k.text, "A callout with a tail".into());
            set(k.fill2, "#FF8E24AA".into());
            set(k.underline, true);
            set(k.v_align, "Middle".into());
            120
        })
        .click("Step", (0.40, 0.45))
        .click("Step", (0.47, 0.45))
        .click("Step", (0.54, 0.45))
        .drag("Pen", (0.62, 0.40), (0.90, 0.55), 30)
        .drag("Highlight", (0.60, 0.62), (0.90, 0.70), 12)
        .drag("Blur", (0.06, 0.68), (0.26, 0.90), 12)
        .drag("Magnify", (0.32, 0.66), (0.46, 0.88), 12)
        .click("Stamp", (0.52, 0.78))
        .drag("Line", (0.60, 0.80), (0.90, 0.92), 12)
        .step(move |_| {
            set(k.shadow, true);
            set(k.shadow_distance, 10.0);
            set(k.end_cap, "Diamond".into());
            120
        })
        .step(move |_| {
            run("tool:Shape");
            set(k.variant, "Polygon".into());
            120
        })
        .step(|t| {
            for (px, py) in [(0.92, 0.30), (0.98, 0.40), (0.90, 0.48)] {
                canvas_feed(POINTER_PRESSED, t.w * px, t.h * py);
                canvas_feed(POINTER_RELEASED, t.w * px, t.h * py);
            }
            run("finish");
            120
        })
        .log(move |_| format!("polygon: selection={} {}", k.selection.get_untracked(), k.document_info.get_untracked()))
        .step(move |_| {
            set(k.variant, "Rectangle".into());
            0
        })
        .shot("ui-1-annotated.png")
        .step(|_| {
            call(|m| m.cycle_theme());
            300
        })
        .log(move |_| format!("theme: {}", k.theme.get_untracked()))
        .shot("ui-1b-light.png")
        .step(|_| {
            call(|m| m.cycle_theme());
            call(|m| m.cycle_theme());
            300
        })
        .log(move |_| format!("theme back: {}", k.theme.get_untracked()))
        // Select, move and restyle the first arrow; undo and redo.
        .drag("Select", (0.19, 0.21), (0.24, 0.26), 12)
        .step(move |_| {
            set(k.stroke_width, 14.0);
            120
        })
        .step(|_| {
            call(|m| m.undo());
            call(|m| m.redo());
            120
        })
        .log(move |_| format!("after edits: {}; title {}", k.document_info.get_untracked(), k.title.get_untracked()))
        .step(|t| {
            run(format!("export-to:{}", t.path("ui-export.png").display()));
            run(format!("export-to:{}", t.path("ui-export.pdf").display()));
            run("deselect");
            120
        })
        .step(|t| {
            // A drag from the canvas's corner outwards (past the document).
            run("tool:Select");
            let (w, h) = (t.w, t.h);
            canvas_feed(POINTER_PRESSED, w, h);
            for i in 1..=12 {
                let f = f64::from(i) / 12.0;
                canvas_feed(POINTER_MOVED, w + 120.0 * f, h + 60.0 * f);
            }
            canvas_feed(POINTER_RELEASED, w + 120.0, h + 60.0);
            120
        })
        .log(move |_| format!("canvas drag: {}", k.document_info.get_untracked()))
        .step(|_| {
            call(|m| m.undo());
            240
        })
        .step(|_| {
            run("effect:Border");
            300
        })
        .step(|_| {
            run("effect:Shadow");
            300
        })
        .shot("ui-2-effects.png")
        .log(move |_| format!("after effects: {}", k.document_info.get_untracked()))
        .drag("Crop", (0.1, 0.1), (0.6, 0.6), 12)
        .shot("ui-3-cropped.png")
        .log(move |_| format!("after crop: {}", k.document_info.get_untracked()))
        .log(move |_| format!("status: {}", k.status.get_untracked()));

    if let Some(fresh) = image.clone() {
        image_tools(&mut s, fresh);
    }
    if let Some(video) = env("JIFFSHOT_UITEST_VIDEO") {
        video_steps(&mut s, video);
    }
    library_steps(&mut s);
    if let Some(drop) = image {
        windows_steps(&mut s, drop);
    }
    s
}

/// Image tools on a fresh copy of the image (coordinates assume the
/// synthetic screenshot from the doc crate's example).
fn image_tools(s: &mut Script, fresh: String) {
    let k = sink();
    let (f1, f2, f3) = (fresh.clone(), fresh.clone(), fresh);
    s.step(move |_| {
        run(format!("open:{f1}"));
        600
    })
    .step(|_| {
        run("zoom:fit");
        120
    })
    .drag("Text Replace", (0.26, 0.15), (0.84, 0.205), 12)
    .log(move |_| format!("text replace: selection={} text={}", k.selection.get_untracked(), k.text.get_untracked()))
    .step(move |_| {
        set(k.text, "Text Replace made this line editable.".into());
        300
    })
    .drag("Smart Move", (0.58, 0.54), (0.745, 0.88), 12)
    .log(move |_| format!("smart move: selection={} status={}", k.selection.get_untracked(), k.status.get_untracked()))
    .step(|t| {
        // Select drags in document pixels past fractions: 230 right, 40 up.
        run("tool:Select");
        let (x0, y0) = (t.w * 0.66, t.h * 0.71);
        canvas_feed(POINTER_PRESSED, x0, y0);
        for i in 1..=12 {
            let f = f64::from(i) / 12.0;
            canvas_feed(POINTER_MOVED, x0 + 230.0 * f, y0 - 40.0 * f);
        }
        canvas_feed(POINTER_RELEASED, x0 + 230.0, y0 - 40.0);
        120
    })
    .click("Fill", (0.02, 0.5))
    .drag("Spotlight", (0.25, 0.40), (0.50, 0.50), 12)
    .shot("ui-4-image-tools.png")
    .step(|_| {
        call(|m| m.undo());
        0
    })
    .drag("Cut Out", (0.3, 0.33), (0.3, 0.40), 12)
    .log(move |_| format!("after cut out: {}", k.document_info.get_untracked()))
    .shot("ui-5-cut-out.png")
    .step(move |_| {
        run(format!("open:{f2}"));
        600
    })
    .step(|_| {
        call(|m| m.smart_redact());
        1200
    })
    .log(move |_| format!("smart redact: {}", k.status.get_untracked()))
    .click("Magic Wand", (0.30, 0.38))
    .log(move |_| format!("magic wand: selection={}", k.selection.get_untracked()))
    .drag("Eraser", (0.05, 0.95), (0.20, 0.95), 12)
    .step(|_| {
        call(|m| m.select_all());
        120
    })
    .log(move |_| format!("select all: {} count={}", k.selection.get_untracked(), k.selection_count.get_untracked()))
    .step(|_| {
        call(|m| m.group());
        120
    })
    .log(move |_| format!("group: {}; {}", k.selection.get_untracked(), k.document_info.get_untracked()))
    .step(|_| {
        call(|m| m.ungroup());
        120
    })
    .log(move |_| format!("ungroup: {}; {}", k.selection.get_untracked(), k.document_info.get_untracked()))
    .shot("ui-5b-redact-wand.png")
    .step(move |_| {
        run(format!("open:{f3}"));
        600
    })
    .step(|_| {
        call(|m| m.simplify());
        1500
    })
    .log(move |_| format!("simplify: {}; {}", k.status.get_untracked(), k.document_info.get_untracked()))
    .shot("ui-6-simplified.png");
}

fn video_steps(s: &mut Script, video: String) {
    let k = sink();
    s.step(move |_| {
        run(format!("open:{video}"));
        800
    })
    .log(move |_| format!("video: mode={} duration={:.2} info={}", k.mode.get_untracked(), k.video_duration.get_untracked(), k.video_info.get_untracked()))
    .step(move |_| {
        set(k.video_position, k.video_duration.get_untracked() * 0.25);
        400
    })
    .step(move |_| {
        call(|m| m.set_trim_start());
        set(k.video_position, k.video_duration.get_untracked() * 0.75);
        400
    })
    .step(|_| {
        call(|m| m.set_trim_end());
        120
    })
    .log(move |_| format!("trim {:.2}..{:.2}; status {}", k.trim_start.get_untracked(), k.trim_end.get_untracked(), k.status.get_untracked()))
    .step(move |_| {
        set(k.video_position, k.video_duration.get_untracked() * 0.3);
        300
    })
    .step(|_| {
        call(|m| m.play_pause());
        800
    })
    .log(move |_| format!("playing={} position={:.2}", k.playing.get_untracked(), k.video_position.get_untracked()))
    .shot("ui-8-video.png")
    .step(move |t| {
        t.note = format!("{:.3}", k.video_position.get_untracked());
        call(|m| m.prev_frame());
        300
    })
    .step(|_| {
        call(|m| m.next_frame());
        call(|m| m.next_frame());
        300
    })
    .log(move |t| format!("frame step: {} -> {:.3}", t.note, k.video_position.get_untracked()))
    .step(move |_| {
        set(k.gif_preset, "High motion".into());
        set(k.gif_fade, true);
        call(|m| m.toggle_mute());
        200
    })
    .log(move |_| format!("gif {} fade={} muted={}", k.gif_preset.get_untracked(), k.gif_fade.get_untracked(), k.muted.get_untracked()))
    .step(move |_| {
        call(|m| m.toggle_mute());
        set(k.gif_preset, "Screen recording".into());
        set(k.gif_fade, false);
        200
    })
    .shot("ui-8b-video-options.png")
    .step(|_| {
        call(|m| m.play_pause());
        300
    })
    .step(|_| {
        call(|m| m.frame_to_image());
        500
    })
    .log(move |_| format!("frame to image: mode={} {}", k.mode.get_untracked(), k.document_info.get_untracked()));
}

/// The library item `index`'s model, if there is one.
fn library_item(index: usize) -> Option<Rc<crate::vm::LibraryItemItem>> {
    sink().library.get_untracked().get(index).cloned()
}

fn library_steps(s: &mut Script) {
    let k = sink();
    s.step(move |t| {
        let count = k.library.get_untracked().len();
        t.note = count.to_string();
        if count > 1 {
            if let Some(first) = library_item(0) {
                first.model.run(|m| m.set_tags("uitest, docs".into()));
            }
        }
        300
    })
    .step(move |t| {
        if t.note != "0" && t.note != "1" {
            if let Some(last) = library_item(k.library.get_untracked().len().saturating_sub(1)) {
                last.model.run(|m| m.toggle_pin());
            }
        }
        300
    })
    .step(move |t| {
        if t.note == "0" || t.note == "1" {
            t.line("library: fewer than two captures, skipped");
            return 0;
        }
        let items = k.library.get_untracked();
        let f = |i: usize| items.get(i).map(|e| e.fields.get_untracked()).unwrap_or_default();
        let (first, second) = (f(0), f(1));
        t.line(format!("library: {}; first={} pinned={} tags=[{}{}]", k.library_info.get_untracked(), first.label, first.is_pinned, second.tags, first.tags));
        set(k.library_filter, "Tag: uitest".into());
        300
    })
    .step(move |t| {
        if t.note == "0" || t.note == "1" {
            return 0;
        }
        t.line(format!("filter Tag: uitest -> {} item(s); filters: {}", k.library.get_untracked().len(), k.library_filters.get_untracked().join(" | ")));
        set(k.library_filter, "All".into());
        set(k.library_search, "zzz-nothing".into());
        300
    })
    .step(move |t| {
        if t.note == "0" || t.note == "1" {
            return 0;
        }
        t.line(format!("search zzz -> {}", k.library.get_untracked().len()));
        set(k.library_search, String::new());
        200
    })
    .step(move |t| {
        if t.note == "0" || t.note == "1" {
            return 0;
        }
        for i in [0, 1] {
            if let Some(item) = library_item(i) {
                item.model.run(|m| m.toggle_select());
            }
        }
        200
    })
    .step(move |t| {
        if t.note == "0" || t.note == "1" {
            return 0;
        }
        t.line(format!("picked {}", k.library_selected.get_untracked()));
        set(k.library_expanded, true);
        set(k.thumb_size, 180.0);
        600
    })
    .step(move |t| {
        if t.note != "0" && t.note != "1" {
            t.shot("ui-9-library.png");
            // Undo the test edits: unpin, untag, unpick, collapse.
            for item in k.library.get_untracked() {
                if item.fields.get_untracked().is_pinned {
                    item.model.run(|m| m.toggle_pin());
                }
            }
        }
        300
    })
    .step(move |t| {
        if t.note == "0" || t.note == "1" {
            return 0;
        }
        for item in k.library.get_untracked() {
            if !item.fields.get_untracked().tags.is_empty() {
                item.model.run(|m| m.set_tags(String::new()));
            }
        }
        call(|m| m.clear_library_selection());
        set(k.thumb_size, 120.0);
        set(k.library_expanded, false);
        300
    })
    .log(move |t| if t.note == "0" || t.note == "1" { String::new() } else { format!("library restored: {}", k.library_info.get_untracked()) });
}

/// Templates, the toolbar and layout switches, Image's commands, and the
/// other windows: dialogs, Preferences, toasts, the HUD, the overlay, Screen
/// Draw and the Capture window.
fn windows_steps(s: &mut Script, drop: String) {
    let k = sink();
    s.step(move |_| {
        set(k.template_theme, "Ocean".into());
        set(k.template_numbers, true);
        set(k.template_captions, true);
        set(k.template_footer, "Made with Jiffshot".into());
        200
    })
    .step(|_| {
        run("template:Two sections");
        500
    })
    .log(move |_| format!("template: {}; status {}", k.document_info.get_untracked(), k.status.get_untracked()))
    .step(move |_| {
        // The test image on the first zone, and twice more for the next ones.
        run(format!("drop:200,200|{drop}|{drop}"));
        500
    })
    .log(move |_| format!("dropped: {}; selection={} panel={}", k.status.get_untracked(), k.selection.get_untracked(), k.panel.get_untracked()))
    .step(|_| {
        run("zone-fit:1");
        300
    })
    .log(move |_| format!("zone fit={}", k.zone_fit.get_untracked()))
    .step(|_| {
        call(|m| m.zoom_fit());
        400
    })
    .shot("ui-10-template.png")
    .step(|_| {
        run("toolbar:hide:Line");
        run("toolbar:up:Text");
        300
    })
    .log(move |_| format!("toolbar: {}", k.tool_order.get_untracked()))
    .shot("ui-11-toolbar.png")
    // The layout switches: the Effects panel, an icon-only toolbar, and the
    // side bar and tray hidden.
    .step(move |_| {
        set(k.show_effects, true);
        300
    })
    .shot("ui-12-effects-panel.png")
    .step(move |_| {
        set(k.show_effects, false);
        set(k.tool_labels, false);
        set(k.show_side_bar, false);
        set(k.show_tray, false);
        300
    })
    .log(move |_| format!("layout: labels={} side={} tray={}", k.tool_labels.get_untracked(), k.show_side_bar.get_untracked(), k.show_tray.get_untracked()))
    .shot("ui-13-compact.png")
    .step(move |t| {
        set(k.tool_labels, true);
        set(k.show_side_bar, true);
        set(k.show_tray, true);
        t.note = k.document_info.get_untracked();
        run("resize:800,500");
        300
    })
    .log(move |t| format!("resize: {} -> {}", t.note, k.document_info.get_untracked()))
    .step(|_| {
        call(|m| m.undo());
        run("trim-image:10,20,30,40");
        300
    })
    .log(move |_| format!("trim: -> {}", k.document_info.get_untracked()))
    .step(|_| {
        call(|m| m.undo());
        200
    })
    .step(|_| {
        call(|m| m.auto_trim());
        300
    })
    .log(move |_| format!("auto trim: -> {}; {}", k.document_info.get_untracked(), k.status.get_untracked()))
    .log(move |_| format!("recent files: {}", k.recent_files.get_untracked().len()))
    .shot_new_windows("ui-14-resize.png", |_| {
        if let (Some(owner), Some(scope)) = (super::main_window(), super::scope()) {
            let _ = super::dialogs::resize_image(&scope, &owner, 1600, 1000, |_, _| {});
        }
    })
    // Preferences: Cancelled (closed), so nothing is applied.
    .shot_new_windows("ui-15-preferences.png", |_| run("prefs:capture"))
    .shot_new_windows("ui-16-shortcuts.png", |_| {
        if let (Some(owner), Some(scope)) = (super::main_window(), super::scope()) {
            let _ = super::dialogs::keyboard_shortcuts(&scope, &owner);
        }
    })
    .shot_new_windows("ui-17-toast.png", move |_| set(k.toast, "9001\u{1f}Capture saved\u{1f}Copied to the clipboard; opened in the editor\u{1f}0".into()))
    .shot_new_windows("ui-17b-toast-problem.png", move |_| set(k.toast, "9002\u{1f}Recording failed\u{1f}The microphone is in use by another app\u{1f}1".into()));
    hud_steps(s);
    overlay_steps(s);
    capture_window_steps(s);
    s.step(|_| {
        run("toolbar:reset");
        run("tool:Arrow");
        run("style-theme:Warm");
        300
    })
    .log(move |_| {
        let styles = k.quick_styles.get_untracked();
        format!("arrow quick styles themed: {}, first stroke {}", styles.len(), styles.first().map(|q| q.fields.get_untracked().stroke).unwrap_or_else(|| "-".into()))
    })
    .step(move |_| {
        if let Some(second) = k.quick_styles.get_untracked().get(1) {
            second.model.run(|m| m.toggle_favorite());
        }
        300
    })
    .step(move |t| {
        let styles = k.quick_styles.get_untracked();
        let favorite = styles.first().is_some_and(|q| q.fields.get_untracked().is_favorite);
        t.line(format!("favorite first: {favorite}"));
        if favorite {
            styles[0].model.run(|m| m.toggle_favorite());
        }
        run("style-theme:reset");
        300
    })
    .log(move |_| format!("toolbar reset: [{}], styles reset: {}", k.tool_order.get_untracked(), k.quick_styles.get_untracked().len()))
    .step(move |_| {
        set(k.template_theme, "Light".into());
        set(k.template_numbers, false);
        set(k.template_captions, false);
        set(k.template_footer, String::new());
        200
    });
}

const HUD_PANEL: &str = r#"{"panels":[{"id":9001,"area":[200,200,640,360],"status":"● Recording 0:07 · 210 frames\nShift+F9 pause · Shift+F10 stop · Esc cancels","buttons":["Stop","Pause","Draw","Cancel"]}],"borders":[{"id":9002,"area":[200,200,640,360]}]}"#;
const HUD_RECORDING: &str = r#"{"id":9003,"area":[200,200,640,360],"status":"","buttons":[],"toolbar":{"millis":29000,"frames":870,"paused":false,"cursor":true,"highlight":false,"clicks":true,"webcam":false,"webcam_corner":"bottom-right","webcam_circle":true,"system":false,"microphone":true,"draw":false,"draw_offered":true,"note":null}}"#;

/// A capture panel and outline from the capture side's JSON, then the
/// recording toolbar: recording, then paused with the webcam on and the
/// microphone off.
fn hud_steps(s: &mut Script) {
    let k = sink();
    s.step(move |t| {
        t.remember_windows();
        set(k.hud, HUD_PANEL.into());
        400
    })
    .step(|t| {
        for hwnd in t.new_windows() {
            let r = window_rect(hwnd);
            let name = if r.width > 600 { "ui-19-hud-border.png" } else { "ui-19-hud-panel.png" };
            t.shot_of(hwnd, name);
            let line = format!(
                "hud window at {},{} size {}x{} affinity 0x{:X}; on the frame: {}; inside: {}",
                r.x,
                r.y,
                r.width,
                r.height,
                affinity(hwnd),
                probe(r.x + 1, r.y + r.height / 2, hwnd),
                probe(r.x + r.width / 2, r.y + r.height / 2, hwnd)
            );
            t.line(line);
        }
        0
    })
    .step(move |t| {
        t.remember_windows();
        set(k.hud, format!(r#"{{"panels":[{HUD_RECORDING}],"borders":[]}}"#));
        500
    })
    .step(|t| {
        for hwnd in t.new_windows() {
            t.shot_of(hwnd, "ui-19b-recording-toolbar.png");
            let r = window_rect(hwnd);
            let line = format!("recording toolbar at {},{} size {}x{} affinity 0x{:X}", r.x, r.y, r.width, r.height, affinity(hwnd));
            t.line(line);
        }
        0
    })
    .step(move |_| {
        let paused = HUD_RECORDING
            .replace("\"paused\":false", "\"paused\":true")
            .replace("\"webcam\":false", "\"webcam\":true")
            .replace("\"microphone\":true", "\"microphone\":false")
            .replace("\"millis\":29000", "\"millis\":3725000")
            .replace("\"note\":null", "\"note\":\"no microphone: the device is in use\"");
        set(k.hud, format!(r#"{{"panels":[{paused}],"borders":[]}}"#));
        400
    })
    .step(|t| {
        for hwnd in t.new_windows() {
            t.shot_of(hwnd, "ui-19c-recording-toolbar-paused.png");
        }
        0
    })
    .step(move |_| {
        set(k.hud, r#"{"panels":[],"borders":[]}"#.into());
        200
    })
    .log(|t| format!("hud after clear: {} new windows still shown", t.new_windows().len()));
}

/// Input for the overlay showing, through its windows (so they redraw).
fn overlay_input(session: u64, kind: i32, x: f64, y: f64, value: i32, mods: i32) -> api::OverlayState {
    super::overlay::feed(kind, x, y, value, mods).unwrap_or_else(|| api::input(session, kind, x, y, value, mods))
}

fn session_of(json: &str) -> u64 {
    serde_json::from_str::<serde_json::Value>(json).ok().and_then(|v| v["id"].as_u64()).unwrap_or(0)
}

fn describe_overlay(session: u64, st: &api::OverlayState) -> String {
    let lit = st.lit();
    let monitor = st.monitor();
    format!(
        "lit {},{} {}x{} kind {} '{}' {} on {},{} {}x{} at {}x",
        lit.x,
        lit.y,
        lit.width,
        lit.height,
        st.lit_kind,
        api::text(session, api::TEXT_TITLE),
        api::text(session, api::TEXT_SIZE),
        monitor.x,
        monitor.y,
        monitor.width,
        monitor.height,
        st.scale
    )
}

/// The selection overlay over the capture side's generated desktop (two
/// screens, the second at 150%, two windows): hover, wheel, a Shift drag,
/// help, the second screen, freehand, multi-area and video guides, each shown
/// for real and saved, then input timings.
fn overlay_steps(s: &mut Script) {
    let k = sink();
    let start = move |t: &mut Test, mode: i32| -> u64 {
        let json = api::demo(mode);
        t.note = session_of(&json).to_string();
        t.remember_windows();
        set(k.overlay, json);
        600
    };
    let session = |t: &Test| t.note.parse::<u64>().unwrap_or(0);
    let shoot = |t: &mut Test, names: &[&str]| {
        let mut shown = t.new_windows();
        shown.sort_by_key(|w| window_rect(*w).x);
        for (hwnd, name) in shown.iter().zip(names) {
            if !name.is_empty() {
                t.shot_of(*hwnd, name);
            }
        }
    };
    let end = move |t: &mut Test| -> u64 {
        set(k.overlay, String::new());
        api::end_demo(t.note.parse().unwrap_or(0));
        200
    };
    s.step(move |t| start(t, 0))
        .step(move |t| {
            let id = session(t);
            let st = overlay_input(id, api::MOVE, 400.0, 300.0, 0, 0);
            t.line(format!("overlay hover: {}", describe_overlay(id, &st)));
            300
        })
        .step(move |t| {
            shoot(t, &["ui-20-overlay-hover.png", "ui-20b-overlay-other-screen.png"]);
            let id = session(t);
            let st = overlay_input(id, api::WHEEL, 400.0, 300.0, 120, 0);
            t.line(format!("overlay wheel up: {}", describe_overlay(id, &st)));
            overlay_input(id, api::PRESS, 200.0, 200.0, 0, 0);
            overlay_input(id, api::MOVE, 500.0, 400.0, 0, 0);
            let st = overlay_input(id, api::MOVE, 760.0, 460.0, 0, 1);
            t.line(format!("overlay shift drag: {}", describe_overlay(id, &st)));
            300
        })
        .step(move |t| {
            shoot(t, &["ui-21-overlay-drag.png"]);
            let st = overlay_input(session(t), api::RELEASE, 760.0, 460.0, 0, 1);
            t.line(format!("overlay released: done {}", st.has(api::FLAG_DONE)));
            end(t)
        })
        .step(move |t| start(t, 0))
        .step(move |t| {
            let id = session(t);
            overlay_input(id, api::KEY, 400.0, 300.0, 0x70, 0);
            let st = overlay_input(id, api::MOVE, 1900.0, 500.0, 0, 0);
            t.line(format!("overlay second screen: {}", describe_overlay(id, &st)));
            300
        })
        .step(move |t| {
            shoot(t, &["", "ui-22-overlay-help-150.png"]);
            // Timing: the pointer sweeping the first screen.
            let id = session(t);
            let mut times = Vec::with_capacity(240);
            for i in 0..240 {
                let f = f64::from(i) / 239.0;
                let begin = Instant::now();
                api::input(id, api::MOVE, 40.0 + 1200.0 * f, 360.0 + 300.0 * (f * 12.0).sin(), 0, 0);
                times.push(begin.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(f64::total_cmp);
            let avg = times.iter().sum::<f64>() / times.len() as f64;
            t.line(format!("overlay timing (240 moves): input avg {avg:.2} ms, p95 {:.2} ms, max {:.2} ms", times[228], times[239]));
            let st = overlay_input(id, api::CANCEL, 0.0, 0.0, 0, 0);
            t.line(format!("overlay cancelled: done {}", st.has(api::FLAG_DONE)));
            end(t)
        })
        .step(move |t| start(t, 1))
        .step(move |t| {
            let id = session(t);
            overlay_input(id, api::PRESS, 840.0, 360.0, 0, 0);
            let mut st = api::OverlayState::default();
            for i in 1..=60 {
                let a = f64::from(i) / 60.0 * std::f64::consts::PI * 1.7;
                st = overlay_input(id, api::MOVE, 640.0 + 200.0 * a.cos(), 360.0 + 200.0 * a.sin(), 0, 0);
            }
            let points = api::points(id, api::POINTS_PATH, 4096).len() / 2;
            t.line(format!("overlay freehand: {points} points, {}", describe_overlay(id, &st)));
            300
        })
        .step(move |t| {
            shoot(t, &["ui-23-overlay-freehand.png"]);
            end(t)
        })
        .step(move |t| start(t, 2))
        .step(move |t| {
            let id = session(t);
            for ((ax, ay), (bx, by)) in [((180.0, 150.0), (520.0, 330.0)), ((640.0, 400.0), (1000.0, 620.0))] {
                overlay_input(id, api::PRESS, ax, ay, 0, 0);
                overlay_input(id, api::MOVE, (ax + bx) / 2.0, (ay + by) / 2.0, 0, 0);
                overlay_input(id, api::MOVE, bx, by, 0, 0);
                overlay_input(id, api::RELEASE, bx, by, 0, 0);
            }
            overlay_input(id, api::MOVE, 1100.0, 200.0, 0, 0);
            t.line(format!("overlay multi-area: {} regions", api::points(id, api::POINTS_REGIONS, 4096).len() / 4));
            300
        })
        .step(move |t| {
            shoot(t, &["ui-24-overlay-multi.png"]);
            let st = overlay_input(session(t), api::KEY, 1100.0, 200.0, 0x0D, 0);
            t.line(format!("overlay multi-area Enter: done {}", st.has(api::FLAG_DONE)));
            end(t)
        })
        .step(move |t| start(t, 3))
        .step(move |t| {
            let id = session(t);
            overlay_input(id, api::KEY, 640.0, 360.0, i32::from(b'G'), 0);
            let st = overlay_input(id, api::MOVE, 640.0, 360.0, 0, 0);
            t.line(format!("overlay video guides: {}", st.has(api::FLAG_GUIDES)));
            300
        })
        .step(move |t| {
            shoot(t, &["ui-25-overlay-video-guides.png"]);
            end(t)
        })
        // Screen Draw over a small area: a box and an arrow, then the window.
        .step(move |t| {
            let json = api::draw_demo(Rect::new(120, 120, 420, 260));
            t.note = serde_json::from_str::<serde_json::Value>(&json).ok().and_then(|v| v["id"].as_u64()).unwrap_or(0).to_string();
            t.remember_windows();
            set(k.screen_draw, json);
            400
        })
        .step(move |_| {
            for (kind, x, y, value) in [
                (api::KEY, 0.0, 0.0, i32::from(b'S')),
                (api::PRESS, 60.0, 50.0, 0),
                (api::MOVE, 300.0, 200.0, 0),
                (api::RELEASE, 300.0, 200.0, 0),
                (api::KEY, 0.0, 0.0, i32::from(b'A')),
                (api::PRESS, 340.0, 40.0, 0),
                (api::MOVE, 220.0, 120.0, 0),
                (api::RELEASE, 220.0, 120.0, 0),
            ] {
                super::overlay::feed_draw(kind, x, y, value);
            }
            300
        })
        .step(move |t| {
            for hwnd in t.new_windows() {
                let r = window_rect(hwnd);
                let line = format!("screen draw window at {},{} size {}x{} affinity 0x{:X}", r.x, r.y, r.width, r.height, affinity(hwnd));
                t.line(line);
                t.shot_of(hwnd, "ui-26-screen-draw.png");
            }
            api::draw_end();
            set(k.screen_draw, String::new());
            200
        });
}

/// The Capture window: each tab, a switch, then closed.
fn capture_window_steps(s: &mut Script) {
    s.step(|t| {
        t.remember_windows();
        if let Some(scope) = super::scope() {
            crate::capture_window::show(&scope);
        }
        400
    })
    .step(|t| {
        t.note = t.new_windows().first().map(|w| w.to_string()).unwrap_or_default();
        if t.note.is_empty() {
            t.line("capture window: not shown");
        }
        0
    });
    for tab in 0..3 {
        s.step(move |_| {
            super::capture().model.run(|m| m.run(format!("tab:{tab}")));
            300
        })
        .step(move |t| {
            if let Ok(hwnd) = t.note.parse::<isize>() {
                t.shot_of(hwnd, &format!("ui-18-capture-tab{tab}.png"));
            }
            0
        });
    }
    s.step(|_| {
        let capture = super::capture();
        capture.model.run(|m| m.run("tab:1".into()));
        capture.model.run(|m| m.run("switch:clipboard".into()));
        300
    })
    .log(|_| format!("capture view: {}", super::capture().sink.view.get_untracked()))
    .step(|t| {
        super::capture().model.run(|m| m.run("switch:clipboard".into()));
        if let Ok(hwnd) = t.note.parse::<isize>() {
            // SAFETY: posting to one of this process's windows.
            unsafe { PostMessageW(hwnd as HWND, WM_CLOSE, 0, 0) };
        }
        200
    });
}

// ---- running ------------------------------------------------------------------

fn next(runner: UiOnly<Rc<RefCell<Runner>>>) {
    let step = runner.get().borrow_mut().steps.pop_front();
    let Some(step) = step else {
        finish(runner.get());
        return;
    };
    let wait = {
        let mut r = runner.get().borrow_mut();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| step(&mut r.test)));
        let wait = match outcome {
            Ok(wait) => wait,
            Err(_) => {
                r.test.line("FAILED: a step panicked");
                r.steps.clear();
                0
            }
        };
        r.test.flush();
        wait
    };
    let scope = runner.get().borrow().scope.clone();
    let delay = scope.delay(Duration::from_millis(wait.max(1)));
    let poster = scope.clone();
    let spawned = scope.spawn(async move {
        if let Ok(delay) = delay {
            let _ = delay.await;
        }
        let _ = poster.post(move || next(runner));
    });
    if let Err(e) = spawned {
        eprintln!("ui self-test: {e:?}");
    }
}

fn finish(runner: &Rc<RefCell<Runner>>) {
    let r = runner.borrow();
    r.test.flush();
    if env("JIFFSHOT_UITEST_KEEP").is_none() {
        // Hidden windows (Capture, the widget…) would keep it running.
        let _ = r.scope.shutdown();
    }
}

/// Starts the walk, writing into `dir`. On the UI thread, once the editor
/// window is mounted.
pub fn start(scope: &AppScope, dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let mut test = Test { dir: dir.to_path_buf(), log: String::new(), w: 0.0, h: 0.0, note: String::new(), windows_before: Vec::new() };
    test.line("started");
    let runner = Rc::new(RefCell::new(Runner { scope: scope.clone(), test, steps: script().0 }));
    next(UiOnly::new(runner));
}

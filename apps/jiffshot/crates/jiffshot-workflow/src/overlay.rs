//! The selection overlay: a frozen, dimmed screenshot of the whole desktop
//! with the window or region under the cursor lit, plus a crosshair, a
//! magnifier and size labels.
//!
//! This module is the overlay's logic: what is lit, what a click, drag,
//! wheel or key does, and what to show ([`Overlay::view`]). The app draws
//! it, one window per monitor, and feeds the input back through the C ABI
//! in `ui_api.rs`; [`run`] hands an overlay to the app and waits for the
//! selection.

use jiffshot_capture::screen::Monitor;
use jiffshot_capture::targets::Targets;
use jiffshot_capture::{Frame, Rect};
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT, VK_BACK, VK_DOWN, VK_ESCAPE, VK_F1, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SPACE, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, PostQuitMessage, TranslateMessage, MSG, PM_REMOVE, QS_ALLINPUT,
    WM_HOTKEY, WM_QUIT,
};

/// How the overlay selects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Windows, regions, screens.
    #[default]
    Normal,
    /// A freehand outline.
    Freehand,
    /// Several regions, finished with Enter.
    MultiArea,
    /// A rectangle of this size, placed by clicking.
    Fixed(i32, i32),
    /// For recording: G shows standard-size guides.
    Video,
}

/// What the user picked, in screen coordinates.
#[derive(Clone, Debug)]
pub enum Selection {
    Region(Rect),
    /// A freehand outline (screen points) inside `rect`.
    Freehand { rect: Rect, points: Vec<(i32, i32)> },
    /// Several regions, all inside `rect`.
    Multi { rect: Rect, regions: Vec<Rect> },
    /// A window or child window; `target` indexes [`Overlay::targets`].
    Window { rect: Rect, target: usize },
    Monitor(Rect),
    Desktop(Rect),
}

impl Selection {
    pub fn rect(&self) -> Rect {
        match self {
            Selection::Region(r)
            | Selection::Window { rect: r, .. }
            | Selection::Monitor(r)
            | Selection::Desktop(r)
            | Selection::Freehand { rect: r, .. }
            | Selection::Multi { rect: r, .. } => *r,
        }
    }
}

/// The lit area while hovering or dragging, in client coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Lit {
    Region(Rect),
    Window(usize, Rect),
    Monitor(Rect),
}

impl Lit {
    fn rect(self) -> Rect {
        match self {
            Lit::Region(r) | Lit::Window(_, r) | Lit::Monitor(r) => r,
        }
    }
}

const DRAG_THRESHOLD: i32 = 3;

/// Standard recording sizes the G key shows.
pub const GUIDES: &[(i32, i32)] = &[(1920, 1080), (1280, 720), (640, 480), (320, 240)];

/// The F1 help, one line each.
pub const HELP: &[&str] = &[
    "Click: window (wheel: parent / child)   ·   Drag: region",
    "Shift: square   ·   Ctrl: 16:9   ·   Ctrl+Shift: 4:3",
    "Arrows: move 1 px (Shift: 10)   ·   Shift+Enter: start a region at the pointer",
    "Enter / Space: capture   ·   F: this screen   ·   A: all screens",
    "M: magnifier   ·   G: video size guides   ·   F1: this help   ·   Esc: cancel",
];

/// The key hints shown at the top of the screen.
fn hint(mode: Mode) -> &'static str {
    match mode {
        Mode::Freehand => "Draw around the area to capture  ·  Esc: cancel  ·  F1: help",
        Mode::MultiArea => "Drag each area to capture, then press Enter  ·  Backspace: remove the last  ·  Esc: cancel",
        Mode::Fixed(..) => "Click to place the fixed-size area  ·  Arrows: nudge  ·  Esc: cancel",
        Mode::Video => "Click a window or drag the area to record  ·  G: size guides  ·  Shift: square, Ctrl: 16:9, Ctrl+Shift: 4:3  ·  Esc: cancel",
        Mode::Normal => "Click a window or drag a region  ·  Wheel: parent / child  ·  F: screen  ·  A: all screens  ·  F1: help  ·  Esc: cancel",
    }
}

/// The drag rectangle, constrained by the modifier keys held: Shift makes
/// a square, Ctrl 16:9, Ctrl+Shift 4:3.
fn constrained(a: (i32, i32), b: (i32, i32), shift: bool, ctrl: bool) -> Rect {
    let ratio = match (ctrl, shift) {
        (true, true) => Some(4.0 / 3.0),
        (true, false) => Some(16.0 / 9.0),
        (false, true) => Some(1.0),
        _ => None,
    };
    let Some(ratio) = ratio else { return Rect::from_points(a, b) };
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let w = (dx.abs() as f32).max(dy.abs() as f32 * ratio);
    let h = w / ratio;
    let sx = if dx < 0 { -1.0 } else { 1.0 };
    let sy = if dy < 0 { -1.0 } else { 1.0 };
    Rect::from_points(a, (a.0 + (w * sx) as i32, a.1 + (h * sy) as i32))
}

/// What the overlay shows right now, in desktop pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub cursor: (i32, i32),
    pub lit: Rect,
    /// 0 a region, 1 a window, 2 a screen.
    pub lit_kind: i32,
    /// The window's name (and child level), or "Screen".
    pub title: Option<String>,
    /// "W × H".
    pub size: String,
    /// The monitor under the cursor, and its UI scale.
    pub monitor: Rect,
    pub scale: f32,
    /// The snapshot's color under the cursor.
    pub color: Option<(u8, u8, u8)>,
    pub hint: &'static str,
    pub magnifier: bool,
    pub help: bool,
    pub guides: bool,
    pub dragging: bool,
}

pub struct Overlay {
    /// Screen position of the snapshot's top-left pixel.
    pub origin: (i32, i32),
    pub snapshot: Frame,
    pub targets: Targets,
    pub monitors: Vec<Monitor>,
    /// When the capture started (the hotkey), for the timing log.
    pub started: Option<Instant>,
    cursor: (i32, i32),
    press: Option<(i32, i32)>,
    dragging: bool,
    shift: bool,
    ctrl: bool,
    levels_up: u32,
    deepest: Option<usize>,
    pub outcome: Option<Option<Selection>>,
    pub mode: Mode,
    show_magnifier: bool,
    show_help: bool,
    show_guides: bool,
    /// Freehand outline so far (client coordinates).
    path: Vec<(i32, i32)>,
    /// Regions already added in multi-area mode (client coordinates).
    regions: Vec<Rect>,
}

impl Overlay {
    /// `snapshot` covers the virtual screen starting at `origin`; `cursor`
    /// is the initial pointer position in screen pixels.
    pub fn new(snapshot: Frame, origin: (i32, i32), targets: Targets, monitors: Vec<Monitor>, cursor: (i32, i32)) -> io::Result<Overlay> {
        Ok(Overlay {
            cursor: (cursor.0 - origin.0, cursor.1 - origin.1),
            origin,
            snapshot,
            targets,
            monitors,
            started: None,
            press: None,
            dragging: false,
            shift: false,
            ctrl: false,
            levels_up: 0,
            deepest: None,
            outcome: None,
            mode: Mode::Normal,
            show_magnifier: true,
            show_help: false,
            show_guides: false,
            path: Vec::new(),
            regions: Vec::new(),
        })
    }

    fn bounds(&self) -> Rect {
        self.snapshot.bounds()
    }

    fn to_screen(&self, r: Rect) -> Rect {
        r.offset(self.origin.0, self.origin.1)
    }

    fn screen_cursor(&self) -> (i32, i32) {
        (self.cursor.0 + self.origin.0, self.cursor.1 + self.origin.1)
    }

    /// The monitor under the cursor, in client coordinates.
    fn monitor(&self) -> (Rect, f32) {
        let (x, y) = self.screen_cursor();
        let monitor = self
            .monitors
            .iter()
            .find(|m| m.rect.contains(x, y))
            .or_else(|| self.monitors.iter().find(|m| m.primary))
            .or(self.monitors.first());
        match monitor {
            Some(m) => (m.rect.offset(-self.origin.0, -self.origin.1), m.scale()),
            None => (self.bounds(), 1.0),
        }
    }

    fn lit_now(&mut self) -> Lit {
        if self.mode == Mode::Freehand && !self.path.is_empty() {
            let r = self.path.iter().fold(Rect::new(self.path[0].0, self.path[0].1, 1, 1), |r, p| r.union(Rect::new(p.0, p.1, 1, 1)));
            return Lit::Region(r);
        }
        if let (true, Some(press)) = (self.dragging, self.press) {
            return Lit::Region(constrained(press, self.cursor, self.shift, self.ctrl).intersect(self.bounds()).unwrap_or_default());
        }
        if let Mode::Fixed(w, h) = self.mode {
            let (cx, cy) = self.cursor;
            let b = self.bounds();
            let x = (cx - w / 2).clamp(b.x, (b.right() - w).max(b.x));
            let y = (cy - h / 2).clamp(b.y, (b.bottom() - h).max(b.y));
            return Lit::Region(Rect::new(x, y, w, h).intersect(b).unwrap_or_default());
        }
        let (x, y) = self.screen_cursor();
        let deepest = self.targets.hit(x, y, 0);
        if deepest != self.deepest {
            self.deepest = deepest;
            self.levels_up = 0;
        }
        match self.targets.hit(x, y, self.levels_up) {
            Some(i) => Lit::Window(i, self.targets.list[i].rect.offset(-self.origin.0, -self.origin.1)),
            None => Lit::Monitor(self.monitor().0),
        }
    }

    /// What to draw now.
    pub fn view(&mut self) -> View {
        let lit = self.lit_now();
        let area = lit.rect().intersect(self.bounds()).unwrap_or_default();
        let title = match lit {
            Lit::Window(i, _) => {
                let t = &self.targets.list[i];
                let name = if t.title.is_empty() { t.class.clone() } else { t.title.clone() };
                let name: String = if name.chars().count() > 60 { name.chars().take(59).chain(['…']).collect() } else { name };
                let level = self.targets.depth_of(i);
                Some(if level > 0 { format!("{name}  ·  child {level}") } else { name })
            }
            Lit::Monitor(_) => Some("Screen".to_owned()),
            Lit::Region(_) => None,
        };
        let (monitor, scale) = self.monitor();
        View {
            cursor: self.screen_cursor(),
            lit: self.to_screen(area),
            lit_kind: match lit {
                Lit::Region(_) => 0,
                Lit::Window(..) => 1,
                Lit::Monitor(_) => 2,
            },
            title,
            size: format!("{} × {}", lit.rect().width, lit.rect().height),
            monitor: self.to_screen(monitor),
            scale,
            color: self.snapshot.rgb(self.cursor.0, self.cursor.1),
            hint: hint(self.mode),
            magnifier: self.show_magnifier,
            help: self.show_help,
            guides: self.show_guides && self.mode == Mode::Video,
            dragging: self.dragging,
        }
    }

    /// The freehand outline so far, in screen pixels.
    pub fn path(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.path.iter().map(|&(x, y)| (x + self.origin.0, y + self.origin.1))
    }

    /// The areas added so far in multi-area mode, in screen pixels.
    pub fn regions(&self) -> impl Iterator<Item = Rect> + '_ {
        self.regions.iter().map(|r| self.to_screen(*r))
    }

    /// The modifier keys held, which constrain a drag.
    pub fn set_modifiers(&mut self, shift: bool, ctrl: bool) {
        self.shift = shift;
        self.ctrl = ctrl;
    }

    // ---- input (client coordinates) -----------------------------------

    pub fn pointer_moved(&mut self, x: i32, y: i32) {
        self.cursor = (x, y);
        if let (false, Some((px, py))) = (self.dragging, self.press) {
            if (x - px).abs() > DRAG_THRESHOLD || (y - py).abs() > DRAG_THRESHOLD {
                self.dragging = true;
            }
        }
        if self.mode == Mode::Freehand && self.press.is_some() && self.path.last() != Some(&(x, y)) {
            self.path.push((x, y));
        }
    }

    pub fn pointer_pressed(&mut self, x: i32, y: i32) {
        self.cursor = (x, y);
        self.press = Some((x, y));
    }

    pub fn pointer_released(&mut self, x: i32, y: i32) {
        self.cursor = (x, y);
        if self.mode == Mode::Freehand {
            if self.path.len() >= 3 {
                let rect = self.lit_now().rect().intersect(self.bounds()).unwrap_or_default();
                let points = self.path.iter().map(|&(px, py)| (px + self.origin.0, py + self.origin.1)).collect();
                self.outcome = Some(Some(Selection::Freehand { rect: self.to_screen(rect), points }));
            }
            self.path.clear();
            self.press = None;
            self.dragging = false;
            return;
        }
        if self.mode == Mode::MultiArea {
            if self.dragging {
                if let Some(r) = self.press.and_then(|p| constrained(p, self.cursor, self.shift, self.ctrl).intersect(self.bounds())) {
                    self.regions.push(r);
                }
            }
            self.press = None;
            self.dragging = false;
            return;
        }
        if self.dragging {
            if let Some(press) = self.press {
                let region = constrained(press, self.cursor, self.shift, self.ctrl).intersect(self.bounds());
                self.outcome = Some(region.map(|r| Selection::Region(self.to_screen(r))));
            }
        } else if self.press.is_some() {
            self.select_lit();
        }
        self.press = None;
        self.dragging = false;
    }

    fn select_lit(&mut self) {
        let selection = match self.lit_now() {
            Lit::Window(target, r) => Selection::Window { rect: self.to_screen(r), target },
            Lit::Monitor(r) => Selection::Monitor(self.to_screen(r)),
            Lit::Region(r) => Selection::Region(self.to_screen(r)),
        };
        self.outcome = Some(Some(selection));
    }

    /// Mouse wheel: up selects the parent of the lit window, down goes back
    /// towards the deepest child under the cursor.
    pub fn wheel(&mut self, delta: i32) {
        if self.dragging {
            return;
        }
        let (x, y) = self.screen_cursor();
        let max = self.targets.hit(x, y, 0).map_or(0, |i| self.targets.depth_of(i));
        self.levels_up = if delta > 0 { (self.levels_up + 1).min(max) } else { self.levels_up.saturating_sub(1) };
    }

    pub fn cancel(&mut self) {
        self.outcome = Some(None);
    }

    /// Handles a key; returns a new cursor position (client coordinates)
    /// when the key nudges the pointer.
    pub fn key(&mut self, vk: u16, shift: bool) -> Option<(i32, i32)> {
        let step = if shift { 10 } else { 1 };
        let (x, y) = self.cursor;
        match vk {
            VK_ESCAPE => self.cancel(),
            VK_LEFT => return Some((x - step, y)),
            VK_RIGHT => return Some((x + step, y)),
            VK_UP => return Some((x, y - step)),
            VK_DOWN => return Some((x, y + step)),
            VK_RETURN if shift && !self.dragging => {
                // A region started from the keyboard: arrows size it, Enter
                // finishes.
                self.press = Some((x, y));
                self.dragging = true;
            }
            VK_RETURN | VK_SPACE if self.mode == Mode::MultiArea => {
                if self.dragging {
                    self.pointer_released(x, y);
                }
                if let Some(first) = self.regions.first().copied() {
                    let all = self.regions.iter().fold(first, |a, r| a.union(*r));
                    let regions = self.regions.iter().map(|r| self.to_screen(*r)).collect();
                    self.outcome = Some(Some(Selection::Multi { rect: self.to_screen(all), regions }));
                }
            }
            VK_BACK if self.mode == Mode::MultiArea => {
                self.regions.pop();
            }
            VK_RETURN | VK_SPACE => {
                if self.dragging {
                    self.pointer_released(x, y);
                } else {
                    self.select_lit();
                }
            }
            VK_F1 => self.show_help = !self.show_help,
            _ if vk == b'M' as u16 => self.show_magnifier = !self.show_magnifier,
            _ if vk == b'G' as u16 => self.show_guides = !self.show_guides,
            _ if vk == b'F' as u16 => self.outcome = Some(Some(Selection::Monitor(self.to_screen(self.monitor().0)))),
            _ if vk == b'A' as u16 => self.outcome = Some(Some(Selection::Desktop(self.to_screen(self.bounds())))),
            _ => {}
        }
        None
    }

    /// Describes what is lit right now, for self-tests and logs.
    pub fn describe(&mut self) -> String {
        match self.lit_now() {
            Lit::Window(i, r) => {
                let t = &self.targets.list[i];
                format!("window #{i} (depth {}) {:?} class={} title={:?}", t.depth, self.to_screen(r), t.class, t.title)
            }
            Lit::Monitor(r) => format!("screen {:?}", self.to_screen(r)),
            Lit::Region(r) => format!("region {:?}", self.to_screen(r)),
        }
    }
}

// ---- the overlay on screen -----------------------------------------------------

/// The overlay the app is showing, driven by its input (see `ui_api.rs`).
pub(crate) struct Session {
    pub id: u64,
    pub overlay: Overlay,
}

pub(crate) static SESSION: Mutex<Option<Session>> = Mutex::new(None);
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// Esc on the capture thread: cancels even when the overlay could not
/// take the focus.
const HOTKEY_ESC: i32 = 0x5340;

pub(crate) fn session() -> std::sync::MutexGuard<'static, Option<Session>> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner())
}

/// What the app needs to put the overlay up: the session, the desktop and
/// its monitors (desktop pixels).
fn announcement(id: u64, overlay: &Overlay) -> String {
    let monitors: Vec<_> = overlay
        .monitors
        .iter()
        .map(|m| serde_json::json!({ "rect": [m.rect.x, m.rect.y, m.rect.width, m.rect.height], "scale": m.scale(), "primary": m.primary }))
        .collect();
    let (x, y) = overlay.screen_cursor();
    serde_json::json!({
        "id": id,
        "origin": [overlay.origin.0, overlay.origin.1],
        "size": [overlay.snapshot.width, overlay.snapshot.height],
        "cursor": [x, y],
        "monitors": monitors,
    })
    .to_string()
}

/// Has the app show the overlay and waits, with this thread's messages
/// flowing, until the user selects or cancels. Returns the overlay (for
/// cropping its snapshot) and the selection; `None` at once without the
/// app.
pub fn run(overlay: Overlay) -> (Overlay, Option<Selection>) {
    let Some(hooks) = crate::hooks() else { return (overlay, None) };
    let id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let announce = announcement(id, &overlay);
    *session() = Some(Session { id, overlay });
    let mut quit = false;
    // SAFETY: a thread hotkey and a plain message pump; the hotkey is
    // released below.
    unsafe {
        RegisterHotKey(std::ptr::null_mut(), HOTKEY_ESC, MOD_NOREPEAT, VK_ESCAPE as u32);
        (hooks.overlay)(announce);
        let mut msg: MSG = std::mem::zeroed();
        while !session().as_ref().is_some_and(|s| s.overlay.outcome.is_some()) && !quit {
            MsgWaitForMultipleObjects(0, std::ptr::null(), 0, 100, QS_ALLINPUT);
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    quit = true;
                    break;
                }
                if msg.message == WM_HOTKEY && msg.hwnd.is_null() && msg.wParam as i32 == HOTKEY_ESC {
                    if let Some(s) = session().as_mut() {
                        s.overlay.cancel();
                    }
                    continue;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        UnregisterHotKey(std::ptr::null_mut(), HOTKEY_ESC);
        (hooks.overlay)(String::new());
        if quit {
            PostQuitMessage(0);
        }
    }
    let mut overlay = session().take().expect("the session ends here").overlay;
    let selection = overlay.outcome.take().flatten();
    (overlay, selection)
}

/// Starts an overlay session over a generated desktop (two screens, the
/// second at 150%, with two windows) without the capture thread, for the
/// UI self-test; returns its announcement. `mode`: 0 normal, 1 freehand,
/// 2 multi-area, 3 video.
pub fn demo(mode: i32) -> String {
    use jiffshot_capture::targets::Target;
    let (w, h) = (2560, 720);
    let mut snapshot = Frame::new(w, h);
    let stride = snapshot.stride();
    for y in 0..h as usize {
        for x in 0..w as usize {
            let p = &mut snapshot.pixels[y * stride + x * 4..][..4];
            // A soft diagonal gradient with a grid, so dimming and the
            // magnifier show.
            let grid = x % 64 == 0 || y % 64 == 0;
            let v = |a: usize| if grid { 0x40 } else { (a % 256) as u8 };
            p.copy_from_slice(&[v(120 + x / 12), v(150 + y / 6), v(200 + (x + y) / 30), 255]);
        }
    }
    let window = |rect: Rect, title: &str, fill: [u8; 3], snapshot: &mut Frame| {
        for y in rect.y..rect.bottom() {
            for x in rect.x..rect.right() {
                let at = y as usize * stride + x as usize * 4;
                let bar = y < rect.y + 32;
                snapshot.pixels[at..at + 3].copy_from_slice(if bar { &[0x30, 0x30, 0x30] } else { &fill });
            }
        }
        Target { hwnd: 0, rect, title: title.to_owned(), class: "Demo".into(), process_id: 0, depth: 0, parent: None }
    };
    let targets = Targets {
        list: vec![
            window(Rect::new(140, 120, 640, 400), "Quarterly report.xlsx - Spreadsheet", [0xF4, 0xF6, 0xF8], &mut snapshot),
            window(Rect::new(1500, 160, 820, 460), "Inbox - Mail", [0xFA, 0xEE, 0xE0], &mut snapshot),
        ],
    };
    let monitors = vec![
        Monitor { rect: Rect::new(0, 0, 1280, 720), work: Rect::new(0, 0, 1280, 680), dpi: 96, primary: true, device: "DEMO1".into() },
        Monitor { rect: Rect::new(1280, 0, 1280, 720), work: Rect::new(1280, 0, 1280, 720), dpi: 144, primary: false, device: "DEMO2".into() },
    ];
    let mut overlay = Overlay::new(snapshot, (0, 0), targets, monitors, (400, 300)).expect("a demo overlay");
    overlay.mode = match mode {
        1 => Mode::Freehand,
        2 => Mode::MultiArea,
        3 => Mode::Video,
        _ => Mode::Normal,
    };
    let id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let announce = announcement(id, &overlay);
    *session() = Some(Session { id, overlay });
    announce
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlay() -> Overlay {
        let monitors = vec![
            Monitor { rect: Rect::new(0, 0, 1000, 600), work: Rect::new(0, 0, 1000, 560), dpi: 96, primary: true, device: "A".into() },
            Monitor { rect: Rect::new(1000, 0, 800, 600), work: Rect::new(1000, 0, 800, 600), dpi: 144, primary: false, device: "B".into() },
        ];
        Overlay::new(Frame::new(1800, 600), (0, 0), Targets::default(), monitors, (100, 100)).unwrap()
    }

    #[test]
    fn hover_lights_the_screen_under_the_pointer() {
        let mut o = overlay();
        let v = o.view();
        assert_eq!((v.lit, v.lit_kind, v.scale), (Rect::new(0, 0, 1000, 600), 2, 1.0));
        o.pointer_moved(1200, 300);
        let v = o.view();
        assert_eq!((v.lit, v.title.as_deref(), v.scale), (Rect::new(1000, 0, 800, 600), Some("Screen"), 1.5));
    }

    #[test]
    fn a_drag_selects_a_region_and_modifiers_constrain_it() {
        let mut o = overlay();
        o.pointer_pressed(100, 100);
        o.pointer_moved(300, 150);
        assert_eq!(o.view().lit, Rect::from_points((100, 100), (300, 150)));
        o.set_modifiers(true, false);
        o.pointer_moved(300, 150);
        let v = o.view();
        assert_eq!((v.lit.width, v.lit.height), (v.lit.width, v.lit.width), "Shift makes a square");
        assert!(v.dragging);
        o.pointer_released(300, 150);
        assert!(matches!(o.outcome, Some(Some(Selection::Region(r))) if r.width == r.height));
    }

    #[test]
    fn keys_toggle_help_and_pick_the_screen() {
        let mut o = overlay();
        o.key(VK_F1, false);
        assert!(o.view().help);
        assert_eq!(o.key(VK_RIGHT, true), Some((110, 100)));
        o.key(b'F' as u16, false);
        assert!(matches!(o.outcome, Some(Some(Selection::Monitor(r))) if r == Rect::new(0, 0, 1000, 600)));
    }
}

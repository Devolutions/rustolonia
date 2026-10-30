//! What the app's overlay and Screen Draw windows call, on the UI thread:
//! input in, what to draw out.

use crate::overlay::{self, View};
use crate::screendraw;
use jiffshot_capture::Rect;
use jiffshot_doc::Pt;
use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

pub const INPUT_PRESS: i32 = 0;
pub const INPUT_MOVE: i32 = 1;
pub const INPUT_RELEASE: i32 = 2;
pub const INPUT_WHEEL: i32 = 3;
pub const INPUT_KEY: i32 = 4;
pub const INPUT_CANCEL: i32 = 5;
// Any other kind only refreshes the state (and the modifier keys).

pub const MOD_SHIFT: i32 = 1;
pub const MOD_CTRL: i32 = 2;

pub const FLAG_DONE: i32 = 1;
pub const FLAG_MAGNIFIER: i32 = 2;
pub const FLAG_HELP: i32 = 4;
pub const FLAG_GUIDES: i32 = 8;
pub const FLAG_COLOR: i32 = 16;
pub const FLAG_DRAGGING: i32 = 32;

/// Texts for [`overlay_text`].
pub const TEXT_TITLE: i32 = 0;
pub const TEXT_SIZE: i32 = 1;
pub const TEXT_HINT: i32 = 2;
pub const TEXT_HELP: i32 = 3;

/// Integer lists for [`overlay_points`].
pub const POINTS_PATH: i32 = 0;
pub const POINTS_REGIONS: i32 = 1;
pub const POINTS_GUIDES: i32 = 2;

/// What the overlay shows, in desktop pixels (see [`View`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OverlayState {
    pub flags: i32,
    pub cursor: [i32; 2],
    pub lit: [i32; 4],
    pub lit_kind: i32,
    pub monitor: [i32; 4],
    pub scale: f32,
    pub color: [i32; 3],
    pub path_count: i32,
    pub region_count: i32,
}

impl OverlayState {
    pub fn has(&self, flag: i32) -> bool {
        self.flags & flag != 0
    }

    pub fn lit(&self) -> Rect {
        let [x, y, w, h] = self.lit;
        Rect::new(x, y, w, h)
    }

    pub fn monitor(&self) -> Rect {
        let [x, y, w, h] = self.monitor;
        Rect::new(x, y, w, h)
    }
}

fn state_of(view: &View, done: bool, path: usize, regions: usize) -> OverlayState {
    let flag = |on: bool, f: i32| if on { f } else { 0 };
    let r = |r: Rect| [r.x, r.y, r.width, r.height];
    OverlayState {
        flags: flag(done, FLAG_DONE)
            | flag(view.magnifier, FLAG_MAGNIFIER)
            | flag(view.help, FLAG_HELP)
            | flag(view.guides, FLAG_GUIDES)
            | flag(view.color.is_some(), FLAG_COLOR)
            | flag(view.dragging, FLAG_DRAGGING),
        cursor: [view.cursor.0, view.cursor.1],
        lit: r(view.lit),
        lit_kind: view.lit_kind,
        monitor: r(view.monitor),
        scale: view.scale,
        color: view.color.map_or([0; 3], |(r, g, b)| [r as i32, g as i32, b as i32]),
        path_count: path as i32,
        region_count: regions as i32,
    }
}

/// Input for overlay `session` (desktop pixels; `value` is the wheel delta
/// or the virtual key), then the state. A session that is over answers
/// [`FLAG_DONE`].
pub fn overlay_input(session: u64, kind: i32, x: f64, y: f64, value: i32, mods: i32) -> OverlayState {
    let mut guard = overlay::session();
    let Some(s) = guard.as_mut().filter(|s| s.id == session) else {
        return OverlayState { flags: FLAG_DONE, ..OverlayState::default() };
    };
    let o = &mut s.overlay;
    let shift = mods & MOD_SHIFT != 0;
    o.set_modifiers(shift, mods & MOD_CTRL != 0);
    let (cx, cy) = (x.round() as i32 - o.origin.0, y.round() as i32 - o.origin.1);
    let was_done = o.outcome.is_some();
    match kind {
        INPUT_PRESS => o.pointer_pressed(cx, cy),
        INPUT_MOVE => o.pointer_moved(cx, cy),
        INPUT_RELEASE => o.pointer_released(cx, cy),
        INPUT_WHEEL => o.wheel(value),
        INPUT_KEY => {
            if let Some((nx, ny)) = o.key(value as u16, shift) {
                // The pointer follows; its move comes back as input.
                // SAFETY: plain cursor move.
                unsafe { SetCursorPos(nx + o.origin.0, ny + o.origin.1) };
            }
        }
        INPUT_CANCEL => o.cancel(),
        _ => {}
    }
    let done = o.outcome.is_some();
    let view = o.view();
    let state = state_of(&view, done, o.path().count(), o.regions().count());
    drop(guard);
    if done && !was_done {
        // Wake the capture thread, which waits for the outcome.
        crate::on_capture_thread(|| {});
    }
    state
}

/// One of the overlay's texts ([`TEXT_TITLE`]…); the help's lines are
/// joined by '\n'.
pub fn overlay_text(session: u64, which: i32) -> String {
    let mut guard = overlay::session();
    let Some(s) = guard.as_mut().filter(|s| s.id == session) else { return String::new() };
    let view = s.overlay.view();
    match which {
        TEXT_TITLE => view.title.unwrap_or_default(),
        TEXT_SIZE => view.size,
        TEXT_HINT => view.hint.to_owned(),
        _ => overlay::HELP.join("\n"),
    }
}

/// One of the overlay's integer lists ([`POINTS_PATH`]…): the freehand path
/// (x, y), the multi-area regions (x, y, w, h) or the video guides (w, h).
pub fn overlay_points(session: u64, which: i32) -> Vec<i32> {
    let guard = overlay::session();
    let Some(s) = guard.as_ref().filter(|s| s.id == session) else { return Vec::new() };
    match which {
        POINTS_PATH => s.overlay.path().flat_map(|(x, y)| [x, y]).collect(),
        POINTS_REGIONS => s.overlay.regions().flat_map(|r| [r.x, r.y, r.width, r.height]).collect(),
        _ => overlay::GUIDES.iter().flat_map(|&(w, h)| [w, h]).collect(),
    }
}

/// Copies the snapshot's `r` (desktop pixels) as BGRA into `dst`, rows
/// `stride` bytes apart; false when the session is over or `dst` is too
/// small.
pub fn overlay_copy(session: u64, r: Rect, dst: &mut [u8], stride: usize) -> bool {
    let guard = overlay::session();
    let Some(s) = guard.as_ref().filter(|s| s.id == session) else { return false };
    let (w, h) = (r.width, r.height);
    if w <= 0 || h <= 0 || stride < w as usize * 4 || dst.len() < stride * h as usize {
        return false;
    }
    let snap = &s.overlay.snapshot;
    let (ox, oy) = (r.x - s.overlay.origin.0, r.y - s.overlay.origin.1);
    let src_stride = snap.stride();
    // Rows split across cores: this is between the hotkey and the overlay
    // appearing.
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    let per = (h as usize).div_ceil(threads);
    std::thread::scope(|scope| {
        for (band, rows) in dst[..stride * h as usize].chunks_mut(per * stride).enumerate() {
            scope.spawn(move || {
                for (i, line) in rows.chunks_mut(stride).enumerate() {
                    let line = &mut line[..w as usize * 4];
                    line.fill(0);
                    let sy = oy + (band * per + i) as i32;
                    if sy < 0 || sy >= snap.height as i32 {
                        continue;
                    }
                    let (x0, x1) = (ox.max(0), (ox + w).min(snap.width as i32));
                    if x1 > x0 {
                        let from = sy as usize * src_stride + x0 as usize * 4;
                        let at = (x0 - ox) as usize * 4;
                        line[at..at + (x1 - x0) as usize * 4].copy_from_slice(&snap.pixels[from..from + (x1 - x0) as usize * 4]);
                    }
                }
            });
        }
    });
    true
}

/// The overlay is on screen (for the timing log).
pub fn overlay_shown(session: u64) {
    let guard = overlay::session();
    if let Some(started) = guard.as_ref().filter(|s| s.id == session).and_then(|s| s.overlay.started) {
        crate::log(&format!("overlay on screen {:.0} ms after the hotkey", started.elapsed().as_secs_f64() * 1000.0));
    }
}

/// Starts a demo overlay (see `overlay::demo`) and returns its
/// announcement. For the UI self-test.
pub fn overlay_demo(mode: i32) -> String {
    overlay::demo(mode)
}

/// Ends a demo overlay.
pub fn overlay_end_demo(session: u64) {
    let mut guard = overlay::session();
    if guard.as_ref().is_some_and(|s| s.id == session) {
        *guard = None;
    }
}

/// Input for Screen Draw `session` (area pixels); true when the marks
/// changed.
pub fn draw_input(session: u64, kind: i32, x: f64, y: f64, value: i32) -> bool {
    let p = Pt::new(x as f32, y as f32);
    let ended = {
        let mut guard = screendraw::state();
        let Some(s) = guard.as_mut().filter(|s| s.id == session) else { return false };
        match kind {
            INPUT_PRESS => s.press(p),
            INPUT_MOVE => return s.moved(p),
            INPUT_RELEASE => s.release(),
            INPUT_KEY => {
                s.key(value as u16);
                return false;
            }
            INPUT_CANCEL => {}
            _ => return false,
        }
        kind == INPUT_CANCEL
    };
    if ended {
        screendraw::hide();
    }
    true
}

/// Re-renders fading marks; true when they changed.
pub fn draw_tick(session: u64) -> bool {
    screendraw::state().as_mut().filter(|s| s.id == session).is_some_and(|s| s.tick())
}

/// Copies the marks (premultiplied BGRA, `w` × `h`, tightly packed) into
/// `dst`, clear where there are none; false when the session is over or
/// `dst` is too small.
pub fn draw_copy(session: u64, dst: &mut [u8], w: u32, h: u32) -> bool {
    let image = {
        let guard = screendraw::state();
        let Some(s) = guard.as_ref().filter(|s| s.id == session) else { return false };
        s.image.clone()
    };
    let stride = w as usize * 4;
    if w == 0 || h == 0 || dst.len() < stride * h as usize {
        return false;
    }
    for (row, line) in dst.chunks_mut(stride).take(h as usize).enumerate() {
        line.fill(0);
        let Some(image) = &image else { continue };
        if row >= image.height() as usize {
            continue;
        }
        let n = (image.width() as usize).min(w as usize);
        let src = &image.data()[row * image.width() as usize * 4..][..n * 4];
        for (d, s) in line.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
            d.copy_from_slice(&[s[2], s[1], s[0], s[3]]);
        }
    }
    true
}

/// Starts Screen Draw over `r` without a recording and returns its
/// announcement. For the UI self-test.
pub fn draw_demo(r: Rect) -> String {
    screendraw::show(r, std::time::Duration::ZERO);
    let guard = screendraw::state();
    let Some(s) = guard.as_ref() else { return String::new() };
    serde_json::json!({ "id": s.id, "area": [r.x, r.y, r.width, r.height] }).to_string()
}

/// Ends Screen Draw.
pub fn draw_end() {
    screendraw::hide();
}

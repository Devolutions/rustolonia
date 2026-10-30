//! The C ABI the app's overlay and Screen Draw windows call, on the UI
//! thread: input in, what to draw out, published as a table of function
//! pointers ([`address`]).

use crate::overlay::{self, View};
use crate::screendraw;
use jiffshot_doc::Pt;
use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

pub const INPUT_PRESS: i32 = 0;
pub const INPUT_MOVE: i32 = 1;
pub const INPUT_RELEASE: i32 = 2;
pub const INPUT_WHEEL: i32 = 3;
pub const INPUT_KEY: i32 = 4;
pub const INPUT_CANCEL: i32 = 5;
// Any other kind only refreshes the state (and the modifier keys).

const MOD_SHIFT: i32 = 1;
const MOD_CTRL: i32 = 2;

pub const FLAG_DONE: i32 = 1;
pub const FLAG_MAGNIFIER: i32 = 2;
pub const FLAG_HELP: i32 = 4;
pub const FLAG_GUIDES: i32 = 8;
pub const FLAG_COLOR: i32 = 16;
pub const FLAG_DRAGGING: i32 = 32;

/// What the overlay shows, in desktop pixels (see [`View`]).
#[repr(C)]
#[derive(Default)]
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

#[repr(C)]
pub struct UiApi {
    /// Input for overlay `session` (desktop pixels; `value` is the wheel
    /// delta or the virtual key), then the state.
    pub overlay_input: extern "C" fn(session: u64, kind: i32, x: f64, y: f64, value: i32, mods: i32, out: *mut OverlayState),
    /// UTF-16 text: 0 the title, 1 the size, 2 the hint, 3 the help (lines
    /// joined by '\n'). Returns the length, at most `cap`.
    pub overlay_text: extern "C" fn(session: u64, which: i32, buf: *mut u16, cap: i32) -> i32,
    /// Integers: 0 the freehand path (x, y), 1 the multi-area regions
    /// (x, y, w, h), 2 the video guides (w, h). Returns how many integers.
    pub overlay_points: extern "C" fn(session: u64, which: i32, buf: *mut i32, cap: i32) -> i32,
    /// Copies the snapshot's `x, y, w, h` (desktop pixels) as BGRA into
    /// `dst`; false when the session is over.
    pub overlay_copy: extern "C" fn(session: u64, x: i32, y: i32, w: i32, h: i32, dst: *mut u8, stride: i32) -> i32,
    /// The overlay is on screen (for the timing log).
    pub overlay_shown: extern "C" fn(session: u64),
    /// Input for Screen Draw `session` (area pixels); true when the marks
    /// changed.
    pub draw_input: extern "C" fn(session: u64, kind: i32, x: f64, y: f64, value: i32) -> i32,
    /// Re-renders fading marks; true when they changed.
    pub draw_tick: extern "C" fn(session: u64) -> i32,
    /// Copies the marks (premultiplied BGRA, `w` × `h`) into `dst`, clear
    /// where there are none; false when the session is over.
    pub draw_copy: extern "C" fn(session: u64, dst: *mut u8, stride: i32, w: i32, h: i32) -> i32,
    /// Starts a demo overlay (see `overlay::demo`) and writes its
    /// announcement as UTF-16; returns the length. For the UI self-test.
    pub overlay_demo: extern "C" fn(mode: i32, buf: *mut u16, cap: i32) -> i32,
    /// Ends a demo overlay.
    pub overlay_end_demo: extern "C" fn(session: u64),
    /// Starts Screen Draw over `x, y, w, h` without a recording and writes
    /// its announcement as UTF-16; returns the length. For the UI self-test.
    pub draw_demo: extern "C" fn(x: i32, y: i32, w: i32, h: i32, buf: *mut u16, cap: i32) -> i32,
    /// Ends Screen Draw.
    pub draw_end: extern "C" fn(),
}

pub static API: UiApi =
    UiApi { overlay_input, overlay_text, overlay_points, overlay_copy, overlay_shown, draw_input, draw_tick, draw_copy, overlay_demo, overlay_end_demo, draw_demo, draw_end };

/// The table's address, for the app to publish.
pub fn address() -> i64 {
    &API as *const UiApi as i64
}

fn write<T>(out: *mut T, value: T) {
    if !out.is_null() {
        // SAFETY: the caller passes a valid, writable struct.
        unsafe { out.write(value) };
    }
}

fn state_of(view: &View, done: bool, path: usize, regions: usize) -> OverlayState {
    let flag = |on: bool, f: i32| if on { f } else { 0 };
    let r = |r: jiffshot_capture::Rect| [r.x, r.y, r.width, r.height];
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

extern "C" fn overlay_input(session: u64, kind: i32, x: f64, y: f64, value: i32, mods: i32, out: *mut OverlayState) {
    let mut guard = overlay::session();
    let Some(s) = guard.as_mut().filter(|s| s.id == session) else {
        write(out, OverlayState { flags: FLAG_DONE, ..OverlayState::default() });
        return;
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
    write(out, state);
}

fn copy_utf16(text: &str, buf: *mut u16, cap: i32) -> i32 {
    let units: Vec<u16> = text.encode_utf16().take(cap.max(0) as usize).collect();
    if !buf.is_null() {
        // SAFETY: the caller's buffer holds `cap` units.
        unsafe { std::ptr::copy_nonoverlapping(units.as_ptr(), buf, units.len()) };
    }
    units.len() as i32
}

extern "C" fn overlay_text(session: u64, which: i32, buf: *mut u16, cap: i32) -> i32 {
    let mut guard = overlay::session();
    let Some(s) = guard.as_mut().filter(|s| s.id == session) else { return 0 };
    let view = s.overlay.view();
    let text = match which {
        0 => view.title.unwrap_or_default(),
        1 => view.size,
        2 => view.hint.to_owned(),
        _ => overlay::HELP.join("\n"),
    };
    copy_utf16(&text, buf, cap)
}

extern "C" fn overlay_points(session: u64, which: i32, buf: *mut i32, cap: i32) -> i32 {
    let guard = overlay::session();
    let Some(s) = guard.as_ref().filter(|s| s.id == session) else { return 0 };
    let values: Vec<i32> = match which {
        0 => s.overlay.path().flat_map(|(x, y)| [x, y]).collect(),
        1 => s.overlay.regions().flat_map(|r| [r.x, r.y, r.width, r.height]).collect(),
        _ => overlay::GUIDES.iter().flat_map(|&(w, h)| [w, h]).collect(),
    };
    let n = values.len().min(cap.max(0) as usize);
    if !buf.is_null() {
        // SAFETY: the caller's buffer holds `cap` integers.
        unsafe { std::ptr::copy_nonoverlapping(values.as_ptr(), buf, n) };
    }
    n as i32
}

extern "C" fn overlay_copy(session: u64, x: i32, y: i32, w: i32, h: i32, dst: *mut u8, stride: i32) -> i32 {
    let guard = overlay::session();
    let Some(s) = guard.as_ref().filter(|s| s.id == session) else { return 0 };
    if dst.is_null() || w <= 0 || h <= 0 {
        return 0;
    }
    let snap = &s.overlay.snapshot;
    let (ox, oy) = (x - s.overlay.origin.0, y - s.overlay.origin.1);
    let src_stride = snap.stride();
    let dst_addr = dst as usize;
    // Rows split across cores: this is between the hotkey and the overlay
    // appearing.
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    let per = (h as usize).div_ceil(threads);
    std::thread::scope(|scope| {
        for t in 0..threads {
            let rows = (t * per)..((t + 1) * per).min(h as usize);
            scope.spawn(move || {
                for row in rows {
                    let sy = oy + row as i32;
                    // SAFETY: each thread writes its own rows of the caller's
                    // `h` × `stride` buffer.
                    let line = unsafe { std::slice::from_raw_parts_mut((dst_addr + row * stride as usize) as *mut u8, w as usize * 4) };
                    if sy < 0 || sy >= snap.height as i32 {
                        line.fill(0);
                        continue;
                    }
                    let (x0, x1) = (ox.max(0), (ox + w).min(snap.width as i32));
                    line.fill(0);
                    if x1 > x0 {
                        let from = sy as usize * src_stride + x0 as usize * 4;
                        let at = (x0 - ox) as usize * 4;
                        line[at..at + (x1 - x0) as usize * 4].copy_from_slice(&snap.pixels[from..from + (x1 - x0) as usize * 4]);
                    }
                }
            });
        }
    });
    1
}

extern "C" fn overlay_demo(mode: i32, buf: *mut u16, cap: i32) -> i32 {
    copy_utf16(&overlay::demo(mode), buf, cap)
}

extern "C" fn overlay_end_demo(session: u64) {
    let mut guard = overlay::session();
    if guard.as_ref().is_some_and(|s| s.id == session) {
        *guard = None;
    }
}

extern "C" fn draw_demo(x: i32, y: i32, w: i32, h: i32, buf: *mut u16, cap: i32) -> i32 {
    screendraw::show(jiffshot_capture::Rect::new(x, y, w, h), std::time::Duration::ZERO);
    let guard = screendraw::state();
    let Some(s) = guard.as_ref() else { return 0 };
    let json = serde_json::json!({ "id": s.id, "area": [x, y, w, h] }).to_string();
    copy_utf16(&json, buf, cap)
}

extern "C" fn draw_end() {
    screendraw::hide();
}

extern "C" fn overlay_shown(session: u64) {
    let guard = overlay::session();
    if let Some(started) = guard.as_ref().filter(|s| s.id == session).and_then(|s| s.overlay.started) {
        crate::log(&format!("overlay on screen {:.0} ms after the hotkey", started.elapsed().as_secs_f64() * 1000.0));
    }
}

extern "C" fn draw_input(session: u64, kind: i32, x: f64, y: f64, value: i32) -> i32 {
    let p = Pt::new(x as f32, y as f32);
    let ended = {
        let mut guard = screendraw::state();
        let Some(s) = guard.as_mut().filter(|s| s.id == session) else { return 0 };
        match kind {
            INPUT_PRESS => s.press(p),
            INPUT_MOVE => return s.moved(p) as i32,
            INPUT_RELEASE => s.release(),
            INPUT_KEY => {
                s.key(value as u16);
                return 0;
            }
            INPUT_CANCEL => {}
            _ => return 0,
        }
        kind == INPUT_CANCEL
    };
    if ended {
        screendraw::hide();
    }
    1
}

extern "C" fn draw_tick(session: u64) -> i32 {
    screendraw::state().as_mut().filter(|s| s.id == session).is_some_and(|s| s.tick()) as i32
}

extern "C" fn draw_copy(session: u64, dst: *mut u8, stride: i32, w: i32, h: i32) -> i32 {
    let image = {
        let guard = screendraw::state();
        let Some(s) = guard.as_ref().filter(|s| s.id == session) else { return 0 };
        s.image.clone()
    };
    if dst.is_null() || w <= 0 || h <= 0 {
        return 0;
    }
    for row in 0..h as usize {
        // SAFETY: the caller's buffer holds `h` rows of `stride` bytes.
        let line = unsafe { std::slice::from_raw_parts_mut(dst.add(row * stride as usize), w as usize * 4) };
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
    1
}

//! The capture side's overlay and Screen Draw calls (`ui_api.rs` in
//! `jiffshot-workflow`), wrapped for the overlay windows: input in, state,
//! texts, points and pixels out.
//!
//! The workflow crate publishes them as a C function table
//! ([`jiffshot_workflow::ui_api_address`]), the way the managed windows
//! called them; [`Table`] mirrors its layout. Everything here runs on the UI
//! thread.

use jiffshot_capture::Rect;
use rustolonia::pixel_surface::Modifiers;
use rustolonia::Key;

pub const PRESS: i32 = 0;
pub const MOVE: i32 = 1;
pub const RELEASE: i32 = 2;
pub const WHEEL: i32 = 3;
pub const KEY: i32 = 4;
pub const CANCEL: i32 = 5;
/// Any other kind only refreshes the state (and the modifier keys).
pub const REFRESH: i32 = 6;

const MOD_SHIFT: i32 = 1;
const MOD_CTRL: i32 = 2;

pub const FLAG_DONE: i32 = 1;
pub const FLAG_MAGNIFIER: i32 = 2;
pub const FLAG_HELP: i32 = 4;
pub const FLAG_GUIDES: i32 = 8;
pub const FLAG_COLOR: i32 = 16;

/// Text kinds for [`text`].
pub const TEXT_TITLE: i32 = 0;
pub const TEXT_SIZE: i32 = 1;
pub const TEXT_HINT: i32 = 2;
pub const TEXT_HELP: i32 = 3;

/// Integer lists for [`points`].
pub const POINTS_PATH: i32 = 0;
pub const POINTS_REGIONS: i32 = 1;
pub const POINTS_GUIDES: i32 = 2;

/// What the overlay shows, in desktop pixels (`OverlayState` in `ui_api.rs`).
#[repr(C)]
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

/// The function table, field for field as `UiApi` in `ui_api.rs`.
#[repr(C)]
struct Table {
    overlay_input: extern "C" fn(session: u64, kind: i32, x: f64, y: f64, value: i32, mods: i32, out: *mut OverlayState),
    overlay_text: extern "C" fn(session: u64, which: i32, buf: *mut u16, cap: i32) -> i32,
    overlay_points: extern "C" fn(session: u64, which: i32, buf: *mut i32, cap: i32) -> i32,
    overlay_copy: extern "C" fn(session: u64, x: i32, y: i32, w: i32, h: i32, dst: *mut u8, stride: i32) -> i32,
    overlay_shown: extern "C" fn(session: u64),
    draw_input: extern "C" fn(session: u64, kind: i32, x: f64, y: f64, value: i32) -> i32,
    draw_tick: extern "C" fn(session: u64) -> i32,
    draw_copy: extern "C" fn(session: u64, dst: *mut u8, stride: i32, w: i32, h: i32) -> i32,
    // The UI self-test's entry points.
    _overlay_demo: extern "C" fn(mode: i32, buf: *mut u16, cap: i32) -> i32,
    _overlay_end_demo: extern "C" fn(session: u64),
    _draw_demo: extern "C" fn(x: i32, y: i32, w: i32, h: i32, buf: *mut u16, cap: i32) -> i32,
    _draw_end: extern "C" fn(),
}

fn table() -> &'static Table {
    // SAFETY: the address is that of the workflow crate's `static API:
    // UiApi`, a `repr(C)` table of these function pointers in this order,
    // alive for the whole program.
    unsafe { &*(jiffshot_workflow::ui_api_address() as *const Table) }
}

/// Sends input for overlay `session` (desktop pixels; `value` is the wheel
/// delta or the virtual key) and returns the new state. A session that is
/// over answers [`FLAG_DONE`].
pub fn input(session: u64, kind: i32, x: f64, y: f64, value: i32, mods: i32) -> OverlayState {
    let mut state = OverlayState::default();
    (table().overlay_input)(session, kind, x, y, value, mods, &mut state);
    state
}

/// One of the overlay's texts ([`TEXT_TITLE`]…).
pub fn text(session: u64, which: i32) -> String {
    let mut buf = [0u16; 2048];
    let n = (table().overlay_text)(session, which, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..n.clamp(0, buf.len() as i32) as usize])
}

/// One of the overlay's integer lists ([`POINTS_PATH`]…), at most `cap`
/// integers.
pub fn points(session: u64, which: i32, cap: usize) -> Vec<i32> {
    let mut values = vec![0; cap];
    let n = (table().overlay_points)(session, which, values.as_mut_ptr(), cap.min(i32::MAX as usize) as i32);
    values.truncate(n.clamp(0, cap as i32) as usize);
    values
}

/// Copies the snapshot's `r` (desktop pixels) as BGRA into `dst`, rows
/// `stride` bytes apart; false when the session is over or `dst` is too
/// small.
pub fn copy(session: u64, r: Rect, dst: &mut [u8], stride: usize) -> bool {
    if r.is_empty() || stride < r.width as usize * 4 || dst.len() < stride * r.height as usize || stride > i32::MAX as usize {
        return false;
    }
    (table().overlay_copy)(session, r.x, r.y, r.width, r.height, dst.as_mut_ptr(), stride as i32) != 0
}

/// Tells the capture side the overlay is on screen (for its timing log).
pub fn shown(session: u64) {
    (table().overlay_shown)(session)
}

/// Sends input for Screen Draw `session` (area pixels); true when the
/// marks changed.
pub fn draw_input(session: u64, kind: i32, x: f64, y: f64, value: i32) -> bool {
    (table().draw_input)(session, kind, x, y, value) != 0
}

/// Re-renders fading marks; true when they changed.
pub fn draw_tick(session: u64) -> bool {
    (table().draw_tick)(session) != 0
}

/// Copies the marks (premultiplied BGRA, `w` × `h`, tightly packed) into
/// `dst`; false when the session is over or `dst` is too small.
pub fn draw_copy(session: u64, dst: &mut [u8], w: u32, h: u32) -> bool {
    let stride = w as usize * 4;
    if w == 0 || h == 0 || dst.len() < stride * h as usize || stride > i32::MAX as usize || h > i32::MAX as u32 {
        return false;
    }
    (table().draw_copy)(session, dst.as_mut_ptr(), stride as i32, w as i32, h as i32) != 0
}

/// The Windows virtual key for a key (0 for keys the capture side ignores).
pub fn virtual_key(key: Option<Key>) -> i32 {
    let Some(key) = key else { return 0 };
    let k = key as i32;
    let between = |first: Key, last: Key| (first as i32..=last as i32).contains(&k).then(|| k - first as i32);
    match key {
        Key::Escape => 0x1B,
        Key::Left => 0x25,
        Key::Up => 0x26,
        Key::Right => 0x27,
        Key::Down => 0x28,
        Key::Return => 0x0D,
        Key::Space => 0x20,
        Key::Back => 0x08,
        Key::F1 => 0x70,
        _ => {
            if let Some(i) = between(Key::A, Key::Z) {
                'A' as i32 + i
            } else if let Some(i) = between(Key::D0, Key::D9).or_else(|| between(Key::NumPad0, Key::NumPad9)) {
                '0' as i32 + i
            } else {
                0
            }
        }
    }
}

/// The capture side's modifier bits for Avalonia's.
pub fn modifiers(m: Modifiers) -> i32 {
    (if m.shift() { MOD_SHIFT } else { 0 }) | (if m.control() { MOD_CTRL } else { 0 })
}

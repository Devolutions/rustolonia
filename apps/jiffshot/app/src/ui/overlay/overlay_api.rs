//! The capture side's overlay and Screen Draw calls (`jiffshot_workflow::ui_api`)
//! under the names the overlay windows use, and the mapping of Avalonia's
//! keys and modifiers to the capture side's. On the UI thread.

pub use jiffshot_workflow::ui_api::{
    draw_copy, draw_demo, draw_end, draw_input, draw_tick, overlay_copy as copy, overlay_demo as demo, overlay_end_demo as end_demo,
    overlay_input as input, overlay_shown as shown, overlay_text as text, OverlayState, FLAG_COLOR, FLAG_DONE, FLAG_GUIDES,
    FLAG_HELP, FLAG_MAGNIFIER, INPUT_CANCEL as CANCEL, INPUT_KEY as KEY, INPUT_MOVE as MOVE, INPUT_PRESS as PRESS,
    INPUT_RELEASE as RELEASE, INPUT_WHEEL as WHEEL, MOD_CTRL, MOD_SHIFT, POINTS_GUIDES, POINTS_PATH, POINTS_REGIONS, TEXT_HELP,
    TEXT_HINT, TEXT_SIZE, TEXT_TITLE,
};
use rustolonia::pixel_surface::Modifiers;
use rustolonia::Key;

/// Any input kind the capture side does not know only refreshes the state
/// (and the modifier keys).
pub const REFRESH: i32 = 6;

/// One of the overlay's integer lists ([`POINTS_PATH`]…), at most `cap`
/// integers.
pub fn points(session: u64, which: i32, cap: usize) -> Vec<i32> {
    let mut values = jiffshot_workflow::ui_api::overlay_points(session, which);
    values.truncate(cap);
    values
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

//! The C ABI the managed `PixelCanvas` control calls. Rust owns the
//! document and the display raster (in [`EDITOR`]); the control copies the
//! damaged part of the raster into its bitmap after each call and draws the
//! selection overlay from [`Overlay`].
//!
//! The managed side reaches these functions through [`CanvasApi`], a table
//! of function pointers published as an integer view-model property. The
//! host is loaded into this process, so pointers and pixels are shared
//! directly.

use parking_lot::Mutex;
use jiffshot_doc::editor::{Editor, Modifiers, EVENT_FOCUS_TEXT, EVENT_RESIZED};
use jiffshot_doc::{IRect, Pt};
use std::time::Instant;

pub const POINTER_PRESSED: i32 = 0;
pub const POINTER_MOVED: i32 = 1;
pub const POINTER_RELEASED: i32 = 2;

const MOD_SHIFT: i32 = 1;
const MOD_CTRL: i32 = 2;
const MOD_ALT: i32 = 4;

/// Damage flags: the view model should refresh, the whole frame must be
/// re-read (the size changed), the text box should take focus.
pub const FLAG_CHANGED: i32 = 1;
pub const FLAG_FULL: i32 = 2;
pub const FLAG_FOCUS_TEXT: i32 = 4;

/// The only editor; `None` until the app has started.
pub static EDITOR: Mutex<Option<Editor>> = Mutex::new(None);

/// Events taken from the editor during canvas calls, for the view model to
/// handle on its next sync.
pub static PENDING_EVENTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A view of the display raster. Only valid between `lock_frame` and
/// `unlock_frame`.
#[repr(C)]
pub struct CanvasFrame {
    pub pixels: *const u8,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
}

/// The changed rectangle (zero width when nothing changed), flags, the
/// pointer cursor to show and the time Rust spent.
#[repr(C)]
pub struct CanvasDamage {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub flags: i32,
    pub cursor: i32,
    pub render_micros: i64,
}

pub const MAX_HANDLES: usize = 16;
pub const MAX_OTHERS: usize = 64;

/// What the control draws over the image, in document pixels: the
/// selection's box and handles, and the marquee of crop-like tools.
#[repr(C)]
pub struct Overlay {
    /// 1: selection box valid, 2: marquee valid.
    pub flags: i32,
    pub handle_count: i32,
    pub rotation: f32,
    pub selection: [f32; 4],
    pub marquee: [f32; 4],
    pub handles: [f32; MAX_HANDLES * 2],
    pub handle_ids: [i32; MAX_HANDLES],
    pub other_count: i32,
    /// Boxes of the other selected objects, x, y, w, h each.
    pub others: [f32; MAX_OTHERS * 4],
}

#[repr(C)]
pub struct CanvasApi {
    pub pointer: extern "C" fn(kind: i32, x: f64, y: f64, modifiers: i32, tolerance: f64, out: *mut CanvasDamage),
    pub hover: extern "C" fn(x: f64, y: f64, tolerance: f64) -> i32,
    pub flush: extern "C" fn(out: *mut CanvasDamage),
    pub lock_frame: extern "C" fn(out: *mut CanvasFrame),
    pub unlock_frame: extern "C" fn(),
    pub overlay: extern "C" fn(out: *mut Overlay),
    pub nudge: extern "C" fn(dx: f64, dy: f64, out: *mut CanvasDamage),
}

pub static API: CanvasApi = CanvasApi { pointer, hover, flush, lock_frame, unlock_frame, overlay, nudge };

fn modifiers(bits: i32) -> Modifiers {
    Modifiers { shift: bits & MOD_SHIFT != 0, ctrl: bits & MOD_CTRL != 0, alt: bits & MOD_ALT != 0 }
}

/// Renders pending damage and reports it with the editor's new events.
fn damage_of(editor: &mut Editor, started: Instant) -> CanvasDamage {
    let r = editor.take_damage().unwrap_or(IRect::default());
    let events = editor.take_events();
    PENDING_EVENTS.fetch_or(events, std::sync::atomic::Ordering::Relaxed);
    let mut flags = 0;
    if events != 0 {
        flags |= FLAG_CHANGED;
    }
    if events & EVENT_RESIZED != 0 {
        flags |= FLAG_FULL;
    }
    if events & EVENT_FOCUS_TEXT != 0 {
        flags |= FLAG_FOCUS_TEXT;
    }
    CanvasDamage { x: r.x, y: r.y, width: r.w, height: r.h, flags, cursor: 0, render_micros: started.elapsed().as_micros() as i64 }
}

fn write<T>(out: *mut T, value: T) {
    // SAFETY: the managed caller passes a pointer to a stack-allocated
    // struct of this layout that outlives the call.
    unsafe { out.write(value) };
}

extern "C" fn pointer(kind: i32, x: f64, y: f64, mods: i32, tolerance: f64, out: *mut CanvasDamage) {
    let _t = crate::trace::span("pointer");
    let started = Instant::now();
    let mut guard = EDITOR.lock();
    let Some(editor) = guard.as_mut() else {
        return write(out, CanvasDamage { x: 0, y: 0, width: 0, height: 0, flags: 0, cursor: 0, render_micros: 0 });
    };
    let p = Pt::new(x as f32, y as f32);
    let m = modifiers(mods);
    match kind {
        POINTER_PRESSED => editor.pointer_down(p, m, tolerance as f32),
        POINTER_MOVED => editor.pointer_move(p, m),
        POINTER_RELEASED => editor.pointer_up(p, m),
        _ => {}
    }
    let mut damage = damage_of(editor, started);
    damage.cursor = editor.hover(p, tolerance as f32) as i32;
    write(out, damage);
}

extern "C" fn hover(x: f64, y: f64, tolerance: f64) -> i32 {
    let _t = crate::trace::span("hover");
    EDITOR.lock().as_ref().map_or(0, |e| e.hover(Pt::new(x as f32, y as f32), tolerance as f32) as i32)
}

extern "C" fn flush(out: *mut CanvasDamage) {
    let _t = crate::trace::span("flush");
    let started = Instant::now();
    let mut guard = EDITOR.lock();
    let damage = match guard.as_mut() {
        Some(editor) => damage_of(editor, started),
        None => CanvasDamage { x: 0, y: 0, width: 0, height: 0, flags: 0, cursor: 0, render_micros: 0 },
    };
    write(out, damage);
}

extern "C" fn nudge(dx: f64, dy: f64, out: *mut CanvasDamage) {
    let _t = crate::trace::span("nudge");
    let started = Instant::now();
    let mut guard = EDITOR.lock();
    let damage = match guard.as_mut() {
        Some(editor) => {
            editor.nudge(dx as f32, dy as f32);
            damage_of(editor, started)
        }
        None => CanvasDamage { x: 0, y: 0, width: 0, height: 0, flags: 0, cursor: 0, render_micros: 0 },
    };
    write(out, damage);
}

/// Locks the editor and describes the display raster. The lock is held
/// until `unlock_frame`, so the managed copy never races an edit.
extern "C" fn lock_frame(out: *mut CanvasFrame) {
    let _t = crate::trace::span("lock_frame");
    let guard = EDITOR.lock();
    let frame = match guard.as_ref() {
        Some(editor) => {
            let d = editor.display();
            CanvasFrame { pixels: d.data().as_ptr(), width: d.width() as i32, height: d.height() as i32, stride: d.width() as i32 * 4 }
        }
        None => CanvasFrame { pixels: std::ptr::null(), width: 0, height: 0, stride: 0 },
    };
    std::mem::forget(guard);
    write(out, frame);
}

extern "C" fn unlock_frame() {
    let _t = crate::trace::span("unlock_frame");
    // SAFETY: paired with the guard forgotten in `lock_frame`, on the same
    // (UI) thread.
    unsafe { EDITOR.force_unlock() };
}

extern "C" fn overlay(out: *mut Overlay) {
    let _t = crate::trace::span("overlay");
    let mut o = Overlay {
        flags: 0,
        handle_count: 0,
        rotation: 0.0,
        selection: [0.0; 4],
        marquee: [0.0; 4],
        handles: [0.0; MAX_HANDLES * 2],
        handle_ids: [0; MAX_HANDLES],
        other_count: 0,
        others: [0.0; MAX_OTHERS * 4],
    };
    if let Some(editor) = EDITOR.lock().as_ref() {
        if let Some(b) = editor.selection_box() {
            o.flags |= 1;
            o.selection = [b.x0, b.y0, b.width(), b.height()];
            o.rotation = editor.selected().map_or(0.0, |s| if s.rotatable() { s.rotation } else { 0.0 });
        }
        for (i, (id, p)) in editor.selection_handles().into_iter().take(MAX_HANDLES).enumerate() {
            o.handles[i * 2] = p.x;
            o.handles[i * 2 + 1] = p.y;
            o.handle_ids[i] = id as i32;
            o.handle_count = i as i32 + 1;
        }
        if let Some(m) = editor.marquee() {
            o.flags |= 2;
            o.marquee = [m.x0, m.y0, m.width(), m.height()];
            // Box selection, not an area of the image: no shade or size.
            if editor.tool() == jiffshot_doc::Tool::Select {
                o.flags |= 4;
            }
        }
        let canvas = editor.canvas_handles();
        if !canvas.is_empty() && o.flags & 1 == 0 {
            o.flags |= 8;
            for (i, (id, p)) in canvas.into_iter().enumerate() {
                o.handles[i * 2] = p.x;
                o.handles[i * 2 + 1] = p.y;
                o.handle_ids[i] = id as i32;
                o.handle_count = i as i32 + 1;
            }
        }
        for (i, b) in editor.other_boxes().into_iter().take(MAX_OTHERS).enumerate() {
            o.others[i * 4..i * 4 + 4].copy_from_slice(&[b.x0, b.y0, b.width(), b.height()]);
            o.other_count = i as i32 + 1;
        }
    }
    write(out, o);
}

/// Runs `f` on the editor (if started) without holding the lock afterwards.
pub fn with_editor<R>(f: impl FnOnce(&mut Editor) -> R) -> Option<R> {
    EDITOR.lock().as_mut().map(f)
}

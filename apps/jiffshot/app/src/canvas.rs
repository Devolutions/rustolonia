//! The editor and the calls the canvas view makes on it. Rust owns the
//! document and the display raster (in [`EDITOR`]); after each call the view
//! ([`crate::ui::canvas_view`]) copies the damaged part of the raster to its
//! surface and draws the selection overlay from [`Overlay`].

use jiffshot_doc::editor::{Editor, Modifiers, EVENT_FOCUS_TEXT, EVENT_RESIZED};
use jiffshot_doc::object::Handle;
use jiffshot_doc::{Box2, IRect, Pixmap, Pt};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

pub const POINTER_PRESSED: i32 = 0;
pub const POINTER_MOVED: i32 = 1;
pub const POINTER_RELEASED: i32 = 2;

/// Modifier bits of [`pointer`].
pub const MOD_SHIFT: i32 = 1;
pub const MOD_CTRL: i32 = 2;
pub const MOD_ALT: i32 = 4;

/// Damage flags: the view model should refresh, the whole frame must be
/// re-read (the size changed), the text box should take focus.
pub const FLAG_CHANGED: i32 = 1;
pub const FLAG_FULL: i32 = 2;
pub const FLAG_FOCUS_TEXT: i32 = 4;

/// The only editor; `None` until the app has started.
pub static EDITOR: Mutex<Option<Editor>> = Mutex::new(None);

/// Events taken from the editor during canvas calls, for the view model to
/// handle on its next sync.
pub static PENDING_EVENTS: AtomicU32 = AtomicU32::new(0);

/// What a canvas call changed: the display rectangle to copy (empty when
/// nothing changed), the `FLAG_*` bits and the pointer cursor to show (a
/// `jiffshot_doc::editor::Cursor` code).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Damage {
    pub rect: IRect,
    pub flags: i32,
    pub cursor: i32,
}

impl Damage {
    /// Adds `later` to this damage: the union of the rectangles and flags,
    /// and the later cursor.
    pub fn merge(&mut self, later: Damage) {
        self.rect = self.rect.union(&later.rect);
        self.flags |= later.flags;
        self.cursor = later.cursor;
    }
}

/// What the view draws over the image, in document pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    /// The selected object's box, before `rotation` (degrees, about its
    /// center).
    pub selection: Option<Box2>,
    pub rotation: f32,
    /// The marquee of crop-like tools or of a box selection.
    pub marquee: Option<Box2>,
    /// The marquee selects objects: no crop shade or size label.
    pub box_select: bool,
    /// The selection's handles, or the canvas edge handles when nothing is
    /// selected (see `canvas_handles`).
    pub handles: Vec<(Handle, Pt)>,
    /// `handles` are the canvas edge handles (drag to resize the canvas).
    pub canvas_handles: bool,
    /// Boxes of the other selected objects.
    pub others: Vec<Box2>,
}

fn modifiers(bits: i32) -> Modifiers {
    Modifiers { shift: bits & MOD_SHIFT != 0, ctrl: bits & MOD_CTRL != 0, alt: bits & MOD_ALT != 0 }
}

/// Renders pending damage and reports it with the editor's new events.
fn damage_of(editor: &mut Editor) -> Damage {
    let rect = editor.take_damage().unwrap_or_default();
    let events = editor.take_events();
    PENDING_EVENTS.fetch_or(events, Ordering::Relaxed);
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
    Damage { rect, flags, cursor: 0 }
}

/// Feeds one pointer event at (`x`, `y`) in document pixels: `kind` is a
/// `POINTER_*`, `mods` `MOD_*` bits and `tolerance` the hit distance in
/// document pixels. Returns the damage and the cursor for that point.
pub fn pointer(kind: i32, x: f64, y: f64, mods: i32, tolerance: f64) -> Damage {
    let _t = crate::trace::span("pointer");
    let mut guard = EDITOR.lock();
    let Some(editor) = guard.as_mut() else { return Damage::default() };
    let p = Pt::new(x as f32, y as f32);
    let m = modifiers(mods);
    match kind {
        POINTER_PRESSED => editor.pointer_down(p, m, tolerance as f32),
        POINTER_MOVED => editor.pointer_move(p, m),
        POINTER_RELEASED => editor.pointer_up(p, m),
        _ => {}
    }
    let mut damage = damage_of(editor);
    damage.cursor = editor.hover(p, tolerance as f32) as i32;
    damage
}

/// The cursor code for the pointer resting at (`x`, `y`) in document pixels.
pub fn hover(x: f64, y: f64, tolerance: f64) -> i32 {
    let _t = crate::trace::span("hover");
    EDITOR.lock().as_ref().map_or(0, |e| e.hover(Pt::new(x as f32, y as f32), tolerance as f32) as i32)
}

/// Renders whatever changed since the last call (edits made outside the
/// canvas) and reports it.
pub fn flush() -> Damage {
    let _t = crate::trace::span("flush");
    EDITOR.lock().as_mut().map(damage_of).unwrap_or_default()
}

/// Moves the selection by (`dx`, `dy`) document pixels.
pub fn nudge(dx: f64, dy: f64) -> Damage {
    let _t = crate::trace::span("nudge");
    let mut guard = EDITOR.lock();
    let Some(editor) = guard.as_mut() else { return Damage::default() };
    editor.nudge(dx as f32, dy as f32);
    damage_of(editor)
}

/// Runs `f` on the display raster (premultiplied RGBA, tightly packed rows)
/// with the editor locked, so the copy never races an edit.
pub fn with_display<R>(f: impl FnOnce(&Pixmap) -> R) -> Option<R> {
    let _t = crate::trace::span("with_display");
    EDITOR.lock().as_ref().map(|e| f(e.display()))
}

/// The selection overlay as it is now.
pub fn overlay() -> Overlay {
    let _t = crate::trace::span("overlay");
    let mut o = Overlay::default();
    let guard = EDITOR.lock();
    let Some(editor) = guard.as_ref() else { return o };
    if let Some(b) = editor.selection_box() {
        o.selection = Some(b);
        o.rotation = editor.selected().map_or(0.0, |s| if s.rotatable() { s.rotation } else { 0.0 });
        o.handles = editor.selection_handles();
    }
    if let Some(m) = editor.marquee() {
        o.marquee = Some(m);
        // Box selection, not an area of the image: no shade or size.
        o.box_select = editor.tool() == jiffshot_doc::Tool::Select;
    }
    if o.selection.is_none() {
        let canvas = editor.canvas_handles();
        if !canvas.is_empty() {
            o.handles = canvas;
            o.canvas_handles = true;
        }
    }
    o.others = editor.other_boxes();
    o
}

/// Runs `f` on the editor (if started) without holding the lock afterwards.
pub fn with_editor<R>(f: impl FnOnce(&mut Editor) -> R) -> Option<R> {
    EDITOR.lock().as_mut().map(f)
}

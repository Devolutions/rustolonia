//! The editor canvas: the document raster on a Rustolonia pixel surface,
//! with zoom, pan and the selection overlay.
//!
//! The surface has two layers: the document bitmap, placed at its zoomed and
//! panned rectangle so the compositor scales it, and a control-sized overlay
//! drawn by Rust in physical pixels ([`super::canvas_overlay`]). After each
//! editor call only the damaged rectangle of the raster is copied.
//!
//! Pointer input goes to the editor in document pixels (see
//! [`crate::canvas`]); the middle or right button, or Alt with the left one,
//! pans instead, and the wheel zooms around the pointer.

mod canvas_overlay;

use super::UiOnly;
use canvas_overlay::{OverlayLayer, Scene};
use crate::canvas::{self, Damage, Overlay, MOD_ALT, MOD_CTRL, MOD_SHIFT, POINTER_MOVED, POINTER_PRESSED, POINTER_RELEASED};
use jiffshot_doc::IRect;
use rustolonia::pixel_surface::{Cursor, Interpolation, Modifiers, MouseButton, PixelFormat, Placement, PointerInput, PointerKind, SurfaceInput, SurfaceMetrics};
use rustolonia::{AppScope, Control, Effect, Key, PixelSurface, Result, Signal, ThemeVariant};
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

/// Damage flags reported to [`CanvasView::on_changed`] (see `crate::canvas`).
pub use crate::canvas::{FLAG_CHANGED, FLAG_FULL};

const DOCUMENT_LAYER: usize = 0;
const OVERLAY_LAYER: usize = 1;

const MIN_ZOOM: f64 = 0.05;
const MAX_ZOOM: f64 = 32.0;
/// Zoom in/out step, and the factor per wheel notch.
const ZOOM_STEP: f64 = 1.25;
const WHEEL_STEP: f64 = 1.15;
/// Room left around the document when fitting it to the view (DIPs).
const FIT_MARGIN: f64 = 48.0;
/// From this zoom pixels are drawn as crisp squares.
const NEAREST_ZOOM: f64 = 2.0;
/// Hit distance in DIPs (divided by the zoom for the editor).
const HIT_TOLERANCE: f64 = 6.0;

/// The backdrop around the document (the main window's `SwChip`).
const BACKDROP_DARK: u32 = 0xFF26_282C;
const BACKDROP_LIGHT: u32 = 0xFFE8_EAED;

/// The canvas control and its view state. Cheap to clone; UI thread only.
///
/// The surface's input handler keeps the canvas alive, so it lives as long
/// as the control does (the editor window's lifetime).
#[derive(Clone)]
pub struct CanvasView {
    inner: Rc<Inner>,
}

struct Inner {
    scope: AppScope,
    surface: PixelSurface,
    state: RefCell<State>,
    handlers: Handlers,
}

#[derive(Default)]
struct Handlers {
    changed: RefCell<Option<Rc<dyn Fn(i32)>>>,
    edit_requested: RefCell<Option<Rc<dyn Fn()>>>,
    zoom_changed: RefCell<Option<Rc<dyn Fn(f64)>>>,
    cancel_requested: RefCell<Option<Rc<dyn Fn()>>>,
}

/// What a pointer drag is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Gesture {
    Idle,
    /// Moving the view; `anchor` is the last pointer position (DIPs).
    Pan { anchor: (f64, f64) },
    /// Feeding the editor; `last` is the last document point sent.
    Draw { last: (f64, f64), mods: i32 },
}

/// What to tell the handlers once the state is released.
#[derive(Clone, Copy, Debug, Default)]
#[must_use]
struct Notice {
    /// Damage flags of an edit (0: none).
    changed: i32,
    zoom: bool,
}

impl Notice {
    fn or(self, other: Notice) -> Notice {
        Notice { changed: self.changed | other.changed, zoom: self.zoom || other.zoom }
    }
}

struct State {
    metrics: SurfaceMetrics,
    /// The document size, (0, 0) before the first frame.
    image: (u32, u32),
    zoom: f64,
    /// Where the document's top-left corner is in the control (DIPs).
    offset: (f64, f64),
    /// Fit the document to the view as soon as both sizes are known.
    fit_pending: bool,
    gesture: Gesture,
    grid: bool,
    overlay: Overlay,
    layer: OverlayLayer,
    /// The document layer's last placement.
    placed: Option<(Placement, Interpolation)>,
    cursor: Cursor,
    /// Damage of drag moves not yet shown, and whether showing it is posted.
    pending: Option<Damage>,
    present_posted: bool,
}

impl State {
    fn to_document(&self, x: f64, y: f64) -> (f64, f64) {
        ((x - self.offset.0) / self.zoom, (y - self.offset.1) / self.zoom)
    }

    fn tolerance(&self) -> f64 {
        HIT_TOLERANCE / self.zoom
    }

    fn has_image(&self) -> bool {
        self.image.0 > 0 && self.image.1 > 0
    }

    /// Zooms to `zoom` keeping the document point under `anchor` (DIPs).
    fn zoom_around(&mut self, anchor: (f64, f64), zoom: f64) {
        let document = self.to_document(anchor.0, anchor.1);
        self.zoom = if zoom.is_finite() { zoom.clamp(MIN_ZOOM, MAX_ZOOM) } else { self.zoom };
        self.offset = (anchor.0 - document.0 * self.zoom, anchor.1 - document.1 * self.zoom);
        self.fit_pending = false;
    }

    fn center(&self) -> (f64, f64) {
        (self.metrics.width / 2.0, self.metrics.height / 2.0)
    }

    /// Fits the document in the view (never above 100%), centered; false
    /// when a size is still unknown.
    fn fit(&mut self) -> bool {
        let (width, height) = (self.metrics.width, self.metrics.height);
        if !self.has_image() || width <= 0.0 || height <= 0.0 {
            return false;
        }
        let (w, h) = (self.image.0 as f64, self.image.1 as f64);
        let zoom = ((width - FIT_MARGIN) / w).min((height - FIT_MARGIN) / h).min(1.0);
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        self.offset = (((width - w * self.zoom) / 2.0).round(), ((height - h * self.zoom) / 2.0).round());
        self.fit_pending = false;
        true
    }

    fn scene(&self) -> Scene {
        Scene {
            overlay: self.overlay.clone(),
            zoom: self.zoom,
            offset: self.offset,
            image: self.image,
            grid: self.grid,
            view: (self.metrics.width, self.metrics.height),
            scaling: self.metrics.scaling,
        }
    }
}

/// The editor's modifier bits for a surface event.
fn modifier_bits(m: Modifiers) -> i32 {
    (if m.shift() { MOD_SHIFT } else { 0 }) | (if m.control() { MOD_CTRL } else { 0 }) | (if m.alt() { MOD_ALT } else { 0 })
}

/// The pointer shape for an editor cursor code.
fn cursor_of(code: i32) -> Cursor {
    match code {
        1 => Cursor::SizeAll,
        2 => Cursor::TopLeftCorner,
        3 => Cursor::TopRightCorner,
        4 => Cursor::SizeNorthSouth,
        5 => Cursor::SizeWestEast,
        6 => Cursor::Cross,
        7 => Cursor::Hand,
        8 => Cursor::Ibeam,
        _ => Cursor::Arrow,
    }
}

/// Records a failure of a call that has no caller to return it to.
fn report(what: &str, result: Result<()>) {
    if let Err(e) = result {
        crate::trace::line(&format!("canvas: {what}: {e}"));
        if cfg!(debug_assertions) {
            eprintln!("canvas: {what}: {e}");
        }
    }
}

impl CanvasView {
    /// Creates the canvas, showing the document in `crate::canvas::EDITOR`.
    pub fn new(scope: &AppScope) -> Result<Self> {
        let slot: Arc<OnceLock<UiOnly<Rc<Inner>>>> = Arc::default();
        let surface = {
            let slot = slot.clone();
            scope.create_pixel_surface(move |input| match slot.get() {
                Some(inner) => {
                    let inner = inner.get().clone();
                    inner.on_input(input)
                }
                None => Ok(false),
            })?
        };
        surface.set_background(Some(BACKDROP_DARK))?;
        let inner = Rc::new(Inner {
            scope: scope.clone(),
            surface,
            state: RefCell::new(State {
                metrics: SurfaceMetrics::default(),
                image: (0, 0),
                zoom: 1.0,
                offset: (0.0, 0.0),
                fit_pending: true,
                gesture: Gesture::Idle,
                grid: false,
                overlay: Overlay::default(),
                layer: OverlayLayer::default(),
                placed: None,
                cursor: Cursor::Inherit,
                pending: None,
                present_posted: false,
            }),
            handlers: Handlers::default(),
        });
        let _ = slot.set(UiOnly::new(inner.clone()));
        let view = Self { inner };
        view.refresh();
        Ok(view)
    }

    /// The control to place in the window.
    pub fn control(&self) -> &Control {
        self.inner.surface.control()
    }

    /// Fetches pending damage from the editor and redraws (after the model
    /// changed the document: the `frame_id` signal).
    pub fn refresh(&self) {
        let notice = self.inner.with_state("refresh", |inner, st| {
            let full = !st.has_image();
            let damage = inner.take_pending(st, canvas::flush());
            inner.present(st, damage, full)
        });
        self.inner.notify(notice);
    }

    /// "in", "out", "fit" or "actual".
    pub fn zoom_to(&self, verb: &str) {
        let notice = self.inner.with_state("zoom", |inner, st| {
            let center = st.center();
            match verb {
                "fit" => st.fit_pending = true,
                "actual" => st.zoom_around(center, 1.0),
                "in" => st.zoom_around(center, st.zoom * ZOOM_STEP),
                "out" => st.zoom_around(center, st.zoom / ZOOM_STEP),
                _ => return Ok(Notice::default()),
            }
            Ok(Notice { zoom: true, ..Notice::default() }.or(inner.update_view(st)?))
        });
        self.inner.notify(notice);
    }

    /// Zooms to `zoom` (1 = 100%) around the middle of the view.
    pub fn set_zoom(&self, zoom: f64) {
        let notice = self.inner.with_state("set zoom", |inner, st| {
            let center = st.center();
            st.zoom_around(center, zoom);
            Ok(Notice { zoom: true, ..Notice::default() }.or(inner.update_view(st)?))
        });
        self.inner.notify(notice);
    }

    pub fn zoom(&self) -> f64 {
        self.inner.state.try_borrow().map_or(1.0, |st| st.zoom)
    }

    /// Lines between pixels at [`GRID_ZOOM`] and closer.
    pub fn set_show_pixel_grid(&self, on: bool) {
        let notice = self.inner.with_state("pixel grid", |inner, st| {
            st.grid = on;
            inner.update_view(st)
        });
        self.inner.notify(notice);
    }

    pub fn show_pixel_grid(&self) -> bool {
        self.inner.state.try_borrow().is_ok_and(|st| st.grid)
    }

    /// The document's size in pixels (0×0 before the first frame).
    pub fn document_size(&self) -> (u32, u32) {
        self.inner.state.try_borrow().map_or((0, 0), |st| st.image)
    }

    /// Feeds one pointer event in document pixels (scripted tests): `kind` is
    /// `crate::canvas::POINTER_*`, `modifiers` 1 shift, 2 ctrl, 4 alt.
    pub fn simulate(&self, kind: i32, x: f64, y: f64, modifiers: i32) {
        let notice = self.inner.with_state("simulate", |inner, st| {
            let damage = canvas::pointer(kind, x, y, modifiers, st.tolerance());
            let damage = inner.take_pending(st, damage);
            inner.present(st, damage, false)
        });
        self.inner.notify(notice);
    }

    /// Follows the editor's color theme ("Dark", "Light" or "System") for the
    /// backdrop around the document. Dark until called.
    pub fn follow_theme(&self, theme: Signal<String>) -> Result<()> {
        let inner = self.inner.clone();
        Effect::new(move || {
            let light = match theme.with(|t| t.clone()).as_str() {
                "Light" => true,
                "Dark" => false,
                _ => inner.scope.actual_theme_variant().is_ok_and(|v| v == ThemeVariant::Light),
            };
            inner.surface.set_background(Some(if light { BACKDROP_LIGHT } else { BACKDROP_DARK }))
        })?;
        Ok(())
    }

    /// After an edit, with the damage flags.
    pub fn on_changed(&self, f: impl Fn(i32) + 'static) {
        *self.inner.handlers.changed.borrow_mut() = Some(Rc::new(f));
    }

    /// A double-click on an object (to edit its text).
    pub fn on_edit_requested(&self, f: impl Fn() + 'static) {
        *self.inner.handlers.edit_requested.borrow_mut() = Some(Rc::new(f));
    }

    /// The zoom changed, with the new factor. Raised after the current
    /// input or call has finished.
    pub fn on_zoom_changed(&self, f: impl Fn(f64) + 'static) {
        *self.inner.handlers.zoom_changed.borrow_mut() = Some(Rc::new(f));
    }

    /// Escape on the canvas.
    pub fn on_cancel_requested(&self, f: impl Fn() + 'static) {
        *self.inner.handlers.cancel_requested.borrow_mut() = Some(Rc::new(f));
    }
}

impl Inner {
    /// Runs `f` with the state borrowed; reports a failure or a reentrant
    /// call (a handler calling back into the canvas mid-update) instead of
    /// panicking.
    fn with_state(&self, what: &str, f: impl FnOnce(&Self, &mut State) -> Result<Notice>) -> Notice {
        let Ok(mut st) = self.state.try_borrow_mut() else {
            crate::trace::line(&format!("canvas: {what}: skipped (reentrant call)"));
            return Notice::default();
        };
        match f(self, &mut st) {
            Ok(notice) => notice,
            Err(e) => {
                report(what, Err(e));
                Notice::default()
            }
        }
    }

    fn state(&self) -> Option<RefMut<'_, State>> {
        self.state.try_borrow_mut().ok()
    }

    /// Tells the handlers what changed. The state must not be borrowed.
    fn notify(self: &Rc<Self>, notice: Notice) {
        if notice.changed != 0 {
            let changed = self.handlers.changed.borrow().clone();
            if let Some(f) = changed {
                f(notice.changed);
            }
        }
        if notice.zoom {
            // After the current pass: a handler
            // that sets the zoom again does not reenter this update.
            let carried = UiOnly::new(self.clone());
            report(
                "zoom notification",
                self.scope.post(move || {
                    let inner = carried.get();
                    let zoom = inner.state.try_borrow().map_or(1.0, |st| st.zoom);
                    let changed = inner.handlers.zoom_changed.borrow().clone();
                    if let Some(f) = changed {
                        f(zoom);
                    }
                }),
            );
        }
    }

    fn set_cursor(&self, st: &mut State, cursor: Cursor) -> Result<()> {
        if st.cursor != cursor {
            self.surface.set_cursor(cursor)?;
            st.cursor = cursor;
        }
        Ok(())
    }

    /// `damage` with the not yet shown drag damage before it.
    fn take_pending(&self, st: &mut State, damage: Damage) -> Damage {
        match st.pending.take() {
            Some(mut pending) => {
                pending.merge(damage);
                pending
            }
            None => damage,
        }
    }

    /// Copies the damaged raster (all of it when `full` or when the size
    /// changed), then updates the view and the overlay.
    fn present(&self, st: &mut State, damage: Damage, full: bool) -> Result<Notice> {
        let _t = crate::trace::span("canvas present");
        let surface = &self.surface;
        let copied = canvas::with_display(|display| -> Result<()> {
            let (width, height) = (display.width(), display.height());
            let mut full = full || damage.flags & FLAG_FULL != 0;
            if st.image != (width, height) {
                surface.set_layer(DOCUMENT_LAYER, width, height, PixelFormat::Rgba8)?;
                st.image = (width, height);
                st.placed = None;
                st.fit_pending = true;
                full = true;
            }
            let bounds = IRect::of_size(width, height);
            let area = if full { Some(bounds) } else { damage.rect.intersect(&bounds) };
            if let Some(r) = area {
                let stride = width as usize * 4;
                let start = r.y as usize * stride + r.x as usize * 4;
                surface.write_layer(DOCUMENT_LAYER, r.x, r.y, r.w as u32, r.h as u32, &display.data()[start..], stride)?;
            }
            Ok(())
        });
        if let Some(result) = copied {
            result?;
        }
        st.overlay = canvas::overlay();
        let notice = self.update_view(st)?;
        Ok(Notice { changed: damage.flags, ..notice })
    }

    /// Fits if pending, places the document layer and redraws the overlay.
    fn update_view(&self, st: &mut State) -> Result<Notice> {
        let zoom = st.fit_pending && st.fit();
        if st.has_image() {
            let (w, h) = (st.image.0 as f64, st.image.1 as f64);
            let placement = Placement::Rect { x: st.offset.0, y: st.offset.1, width: w * st.zoom, height: h * st.zoom };
            // Nearest neighbor when zoomed in, so pixels stay crisp.
            let interpolation = if st.zoom >= NEAREST_ZOOM { Interpolation::None } else { Interpolation::Medium };
            if st.placed != Some((placement, interpolation)) {
                self.surface.place_layer(DOCUMENT_LAYER, placement, interpolation, 1.0)?;
                st.placed = Some((placement, interpolation));
            }
        }
        let scene = st.scene();
        st.layer.present(&scene, &self.surface, OVERLAY_LAYER)?;
        Ok(Notice { zoom, ..Notice::default() })
    }

    /// Shows the drag damage collected since the last pointer event batch.
    fn present_pending(self: &Rc<Self>) {
        let notice = self.with_state("drag", |inner, st| {
            st.present_posted = false;
            match st.pending.take() {
                Some(damage) => inner.present(st, damage, false),
                None => Ok(Notice::default()),
            }
        });
        self.notify(notice);
    }

    fn on_input(self: &Rc<Self>, input: &SurfaceInput) -> Result<bool> {
        match input {
            SurfaceInput::Pointer(p) => self.on_pointer(p),
            SurfaceInput::Key { down: true, key: Some(key), modifiers } => Ok(self.on_key(*key, *modifiers)),
            SurfaceInput::Resize(metrics) => {
                let notice = self.with_state("resize", |inner, st| {
                    st.metrics = *metrics;
                    st.fit_pending = true;
                    inner.update_view(st)
                });
                self.notify(notice);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn on_key(self: &Rc<Self>, key: Key, modifiers: Modifiers) -> bool {
        let step = if modifiers.shift() { 10.0 } else { 1.0 };
        let (dx, dy) = match key {
            Key::Left => (-step, 0.0),
            Key::Right => (step, 0.0),
            Key::Up => (0.0, -step),
            Key::Down => (0.0, step),
            Key::Escape => {
                let cancel = self.handlers.cancel_requested.borrow().clone();
                if let Some(f) = cancel {
                    f();
                }
                return true;
            }
            _ => return false,
        };
        let notice = self.with_state("nudge", |inner, st| {
            let damage = inner.take_pending(st, canvas::nudge(dx, dy));
            inner.present(st, damage, false)
        });
        self.notify(notice);
        true
    }

    fn on_pointer(self: &Rc<Self>, p: &PointerInput) -> Result<bool> {
        let Some(mut st) = self.state() else { return Ok(false) };
        let mut notice = Notice::default();
        let mut edit_requested = false;
        let handled = match (p.kind, st.gesture) {
            // One drag at a time: other buttons are ignored until it ends.
            (PointerKind::Pressed, Gesture::Idle) => {
                let pan = matches!(p.button, MouseButton::Middle | MouseButton::Right) || (p.button == MouseButton::Left && p.modifiers.alt());
                if pan {
                    st.gesture = Gesture::Pan { anchor: (p.x, p.y) };
                    self.set_cursor(&mut st, Cursor::SizeAll)?;
                    true
                } else if p.button != MouseButton::Left || !st.has_image() {
                    false
                } else if p.click_count == 2 {
                    edit_requested = true;
                    true
                } else {
                    let (x, y) = st.to_document(p.x, p.y);
                    let mods = modifier_bits(p.modifiers);
                    st.gesture = Gesture::Draw { last: (x, y), mods };
                    let damage = canvas::pointer(POINTER_PRESSED, x, y, mods, st.tolerance());
                    let damage = self.take_pending(&mut st, damage);
                    self.set_cursor(&mut st, cursor_of(damage.cursor))?;
                    notice = self.present(&mut st, damage, false)?;
                    true
                }
            }
            (PointerKind::Pressed, _) => true,
            (PointerKind::Moved, Gesture::Pan { anchor }) => {
                st.offset = (st.offset.0 + p.x - anchor.0, st.offset.1 + p.y - anchor.1);
                st.gesture = Gesture::Pan { anchor: (p.x, p.y) };
                st.fit_pending = false;
                notice = self.update_view(&mut st)?;
                true
            }
            (PointerKind::Moved, Gesture::Draw { .. }) => {
                // Every coalesced sample goes to the editor; the raster is
                // copied once for the batch.
                let (x, y) = st.to_document(p.x, p.y);
                let mods = modifier_bits(p.modifiers);
                st.gesture = Gesture::Draw { last: (x, y), mods };
                let damage = canvas::pointer(POINTER_MOVED, x, y, mods, st.tolerance());
                self.set_cursor(&mut st, cursor_of(damage.cursor))?;
                st.pending = Some(self.take_pending(&mut st, damage));
                if !st.present_posted {
                    let carried = UiOnly::new(self.clone());
                    match self.scope.post(move || carried.get().present_pending()) {
                        Ok(()) => st.present_posted = true,
                        Err(_) => {
                            let damage = st.pending.take().unwrap_or_default();
                            notice = self.present(&mut st, damage, false)?;
                        }
                    }
                }
                true
            }
            (PointerKind::Moved, Gesture::Idle) => {
                if st.has_image() {
                    let (x, y) = st.to_document(p.x, p.y);
                    let code = canvas::hover(x, y, st.tolerance());
                    self.set_cursor(&mut st, cursor_of(code))?;
                }
                false
            }
            (PointerKind::Released, Gesture::Pan { .. }) => {
                st.gesture = Gesture::Idle;
                let (x, y) = st.to_document(p.x, p.y);
                let code = if st.has_image() { canvas::hover(x, y, st.tolerance()) } else { 0 };
                self.set_cursor(&mut st, cursor_of(code))?;
                true
            }
            (PointerKind::Released, Gesture::Draw { .. }) if p.button == MouseButton::Left => {
                st.gesture = Gesture::Idle;
                let (x, y) = st.to_document(p.x, p.y);
                let damage = canvas::pointer(POINTER_RELEASED, x, y, modifier_bits(p.modifiers), st.tolerance());
                let damage = self.take_pending(&mut st, damage);
                self.set_cursor(&mut st, cursor_of(damage.cursor))?;
                notice = self.present(&mut st, damage, false)?;
                true
            }
            (PointerKind::CaptureLost, Gesture::Pan { .. }) => {
                st.gesture = Gesture::Idle;
                self.set_cursor(&mut st, Cursor::Arrow)?;
                false
            }
            // The capture ended mid-drag (another window took it): finish the
            // drag where it was rather than leave the editor mid-gesture.
            (PointerKind::CaptureLost, Gesture::Draw { last, mods }) => {
                st.gesture = Gesture::Idle;
                let damage = canvas::pointer(POINTER_RELEASED, last.0, last.1, mods, st.tolerance());
                let damage = self.take_pending(&mut st, damage);
                notice = self.present(&mut st, damage, false)?;
                false
            }
            (PointerKind::Wheel, _) => {
                let zoom = st.zoom * WHEEL_STEP.powf(p.delta_y);
                st.zoom_around((p.x, p.y), zoom);
                notice = Notice { zoom: true, ..Notice::default() }.or(self.update_view(&mut st)?);
                true
            }
            _ => false,
        };
        drop(st);
        self.notify(notice);
        if edit_requested {
            let edit = self.handlers.edit_requested.borrow().clone();
            if let Some(f) = edit {
                f();
            }
        }
        Ok(handled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(view: (f64, f64), image: (u32, u32)) -> State {
        State {
            metrics: SurfaceMetrics { width: view.0, height: view.1, scaling: 1.0 },
            image,
            zoom: 1.0,
            offset: (0.0, 0.0),
            fit_pending: true,
            gesture: Gesture::Idle,
            grid: false,
            overlay: Overlay::default(),
            layer: OverlayLayer::default(),
            placed: None,
            cursor: Cursor::Inherit,
            pending: None,
            present_posted: false,
        }
    }

    #[test]
    fn fit_leaves_a_margin_and_never_enlarges() {
        let mut st = state((1048.0, 548.0), (2000, 1000));
        assert!(st.fit());
        assert_eq!(0.5, st.zoom);
        assert_eq!((24.0, 24.0), st.offset);
        let mut small = state((1000.0, 800.0), (100, 50));
        assert!(small.fit());
        assert_eq!(1.0, small.zoom);
        assert_eq!((450.0, 375.0), small.offset);
        assert!(!state((0.0, 0.0), (100, 50)).fit());
    }

    #[test]
    fn zoom_keeps_the_anchor_and_clamps() {
        let mut st = state((800.0, 600.0), (400, 300));
        st.offset = (100.0, 50.0);
        let before = st.to_document(300.0, 200.0);
        st.zoom_around((300.0, 200.0), 3.0);
        let after = st.to_document(300.0, 200.0);
        assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
        st.zoom_around((0.0, 0.0), 1000.0);
        assert_eq!(MAX_ZOOM, st.zoom);
        st.zoom_around((0.0, 0.0), 0.0);
        assert_eq!(MIN_ZOOM, st.zoom);
        assert!(!st.fit_pending);
    }

    #[test]
    fn cursor_codes_map_to_shapes() {
        assert_eq!(Cursor::SizeAll, cursor_of(1));
        assert_eq!(Cursor::Ibeam, cursor_of(8));
        assert_eq!(Cursor::Arrow, cursor_of(42));
    }
}

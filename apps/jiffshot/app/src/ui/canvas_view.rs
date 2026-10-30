//! The editor canvas: the document raster on a Rustolonia pixel surface,
//! with zoom, pan and the selection overlay. Port of `PixelCanvas.cs`.
//!
//! STUB: the interface the main window uses; the implementation is pending.

use rustolonia::{AppScope, Control, Result};

/// Damage flags reported to [`CanvasView::on_changed`] (see `crate::canvas`).
pub use crate::canvas::{FLAG_CHANGED, FLAG_FOCUS_TEXT, FLAG_FULL};

/// The zoom from which View ▸ Pixel grid draws its lines.
pub const GRID_ZOOM: f64 = 6.0;

/// The canvas control and its view state. Cheap to clone; UI thread only.
#[derive(Clone)]
pub struct CanvasView {
    control: Control,
}

impl CanvasView {
    /// Creates the canvas, showing the document in `crate::canvas::EDITOR`.
    pub fn new(_scope: &AppScope) -> Result<Self> {
        Ok(Self { control: Control::new()? })
    }

    /// The control to place in the window.
    pub fn control(&self) -> &Control {
        &self.control
    }

    /// Fetches pending damage from the editor and redraws (after the model
    /// changed the document: the `frame_id` signal).
    pub fn refresh(&self) {}

    /// "in", "out", "fit" or "actual".
    pub fn zoom_to(&self, _verb: &str) {}

    /// Zooms to `zoom` (1 = 100%) around the middle of the view.
    pub fn set_zoom(&self, _zoom: f64) {}

    pub fn zoom(&self) -> f64 {
        1.0
    }

    /// Lines between pixels at [`GRID_ZOOM`] and closer.
    pub fn set_show_pixel_grid(&self, _on: bool) {}

    pub fn show_pixel_grid(&self) -> bool {
        false
    }

    /// A point in the control (device-independent pixels) in document pixels.
    pub fn document_point(&self, x: f64, y: f64) -> (f64, f64) {
        (x, y)
    }

    /// The document's size in pixels (0×0 before the first frame).
    pub fn document_size(&self) -> (u32, u32) {
        (0, 0)
    }

    /// Feeds one pointer event in document pixels (scripted tests): `kind` is
    /// `crate::canvas::POINTER_*`, `modifiers` 1 shift, 2 ctrl, 4 alt.
    pub fn simulate(&self, _kind: i32, _x: f64, _y: f64, _modifiers: i32) {}

    /// Moves keyboard focus to the canvas.
    pub fn focus(&self) {}

    /// After an edit, with the damage flags.
    pub fn on_changed(&self, _f: impl Fn(i32) + 'static) {}

    /// A double-click on an object (to edit its text).
    pub fn on_edit_requested(&self, _f: impl Fn() + 'static) {}

    /// The zoom changed, with the new factor.
    pub fn on_zoom_changed(&self, _f: impl Fn(f64) + 'static) {}

    /// Escape on the canvas.
    pub fn on_cancel_requested(&self, _f: impl Fn() + 'static) {}
}

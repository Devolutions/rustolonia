//! Rust-drawn surfaces: the separately versioned stage 34 `IAvnApplication7`
//! capability.
//!
//! A [`PixelSurface`] is a control that shows pixels Rust produced and reports
//! the raw pointer stream (press, move, release, wheel) and keyboard input to a
//! Rust closure. It is what an image editor canvas, a selection overlay or a
//! custom chart is built on: Rust decides what to draw and how input maps to
//! it, the host only blits bitmaps.
//!
//! A surface draws up to [`PixelSurface::MAX_LAYERS`] bitmap layers in order
//! over an optional background. Each layer is placed at a rectangle in
//! device-independent pixels, or stretched over the whole control, so a large
//! document can be scaled by the compositor while a control-sized overlay is
//! drawn 1:1 in physical pixels (control size × [`SurfaceMetrics::scaling`]).
//!
//! ```no_run
//! # fn demo(scope: &rustolonia::AppScope) -> rustolonia::Result<()> {
//! use rustolonia::pixel_surface::{PixelFormat, SurfaceInput};
//! let surface = scope.create_pixel_surface(|input| {
//!     if let SurfaceInput::Pointer(p) = input {
//!         println!("{:?} at {}, {}", p.kind, p.x, p.y);
//!     }
//!     Ok(true)
//! })?;
//! surface.set_layer(0, 2, 1, PixelFormat::Rgba8)?;
//! surface.write_layer(0, 0, 0, 2, 1, &[255, 0, 0, 255, 0, 0, 255, 255], 8)?;
//! # Ok(()) }
//! ```

use crate::{AppContext, Control, Key, Result};
use rustolonia_sys as sys;

/// The pixel layout of a layer; both are premultiplied, 4 bytes per pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PixelFormat {
    /// Red, green, blue, alpha (what `tiny-skia` and `image` produce).
    #[default]
    Rgba8,
    /// Blue, green, red, alpha (what Windows GDI captures produce).
    Bgra8,
}

/// How a scaled layer is filtered (Avalonia's `BitmapInterpolationMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Interpolation {
    #[default]
    Default,
    /// Nearest neighbor: crisp pixels when zoomed in.
    None,
    Low,
    Medium,
    High,
}

/// Where a layer is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    /// Stretched over the whole control (the default).
    Fill,
    /// At a rectangle in device-independent pixels relative to the control.
    Rect { x: f64, y: f64, width: f64, height: f64 },
}

/// A standard pointer shape (Avalonia's `StandardCursorType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum Cursor {
    /// Whatever the parent shows.
    Inherit = -1,
    Arrow = 0,
    Ibeam,
    Wait,
    Cross,
    UpArrow,
    SizeWestEast,
    SizeNorthSouth,
    SizeAll,
    No,
    Hand,
    AppStarting,
    Help,
    TopSide,
    BottomSide,
    LeftSide,
    RightSide,
    TopLeftCorner,
    TopRightCorner,
    BottomLeftCorner,
    BottomRightCorner,
    DragMove,
    DragCopy,
    DragLink,
    None,
}

/// What happened to the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    /// A button went down; the surface captures the pointer until it is released.
    Pressed,
    Moved,
    Released,
    Wheel,
    Entered,
    Exited,
    /// The capture ended without a release (another window took it).
    CaptureLost,
}

/// A pointer button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    None,
    Left,
    Right,
    Middle,
    X1,
    X2,
}

impl MouseButton {
    fn from_abi(value: i32) -> Self {
        match value {
            1 => Self::Left,
            2 => Self::Right,
            3 => Self::Middle,
            4 => Self::X1,
            5 => Self::X2,
            _ => Self::None,
        }
    }

    fn bit(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Left => 1,
            Self::Right => 2,
            Self::Middle => 4,
            Self::X1 => 8,
            Self::X2 => 16,
        }
    }
}

/// Keyboard modifiers held during an event (Avalonia's `KeyModifiers`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers(pub i32);

impl Modifiers {
    pub fn alt(self) -> bool {
        self.0 & crate::key_modifiers::ALT != 0
    }

    pub fn control(self) -> bool {
        self.0 & crate::key_modifiers::CONTROL != 0
    }

    pub fn shift(self) -> bool {
        self.0 & crate::key_modifiers::SHIFT != 0
    }

    pub fn meta(self) -> bool {
        self.0 & crate::key_modifiers::META != 0
    }
}

/// A pointer event; positions are device-independent pixels relative to the
/// surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerInput {
    pub kind: PointerKind,
    pub x: f64,
    pub y: f64,
    /// The button that went down or up; [`MouseButton::None`] otherwise.
    pub button: MouseButton,
    buttons: i32,
    pub modifiers: Modifiers,
    /// 2 for the second press of a double click.
    pub click_count: i32,
    /// Wheel movement, in notches.
    pub delta_x: f64,
    pub delta_y: f64,
}

impl PointerInput {
    /// Whether `button` is held after this event.
    pub fn is_pressed(&self, button: MouseButton) -> bool {
        self.buttons & button.bit() != 0
    }
}

/// One input notification from a surface.
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceInput {
    Pointer(PointerInput),
    /// A key went down or up while the surface had focus. `key` is `None` for
    /// a key the projection does not know.
    Key { down: bool, key: Option<Key>, modifiers: Modifiers },
    /// Text typed while the surface had focus.
    Text(String),
    /// The surface was attached to a window, resized, or its scaling changed.
    Resize(SurfaceMetrics),
}

/// A surface's size and scaling.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfaceMetrics {
    /// Device-independent pixels.
    pub width: f64,
    pub height: f64,
    /// Physical pixels per device-independent pixel.
    pub scaling: f64,
}

impl SurfaceMetrics {
    /// The size in physical pixels, rounded up: what a 1:1 layer needs.
    pub fn physical_size(&self) -> (u32, u32) {
        let pixels = |dips: f64| (dips * self.scaling).ceil().max(0.0) as u32;
        (pixels(self.width), pixels(self.height))
    }
}

impl SurfaceInput {
    fn from_abi(input: &sys::PixelSurfaceInput) -> Self {
        match *input {
            sys::PixelSurfaceInput::Pointer {
                kind,
                x,
                y,
                button,
                buttons,
                modifiers,
                click_count,
                delta_x,
                delta_y,
            } => SurfaceInput::Pointer(PointerInput {
                kind: match kind {
                    0 => PointerKind::Pressed,
                    1 => PointerKind::Moved,
                    2 => PointerKind::Released,
                    3 => PointerKind::Wheel,
                    4 => PointerKind::Entered,
                    5 => PointerKind::Exited,
                    _ => PointerKind::CaptureLost,
                },
                x,
                y,
                button: MouseButton::from_abi(button),
                buttons,
                modifiers: Modifiers(modifiers),
                click_count,
                delta_x,
                delta_y,
            }),
            sys::PixelSurfaceInput::Key { kind, key, modifiers } => SurfaceInput::Key {
                down: kind == 0,
                key: Key::try_from(key).ok(),
                modifiers: Modifiers(modifiers),
            },
            sys::PixelSurfaceInput::Text(ref text) => SurfaceInput::Text(text.clone()),
            sys::PixelSurfaceInput::Resize {
                width,
                height,
                scaling,
            } => SurfaceInput::Resize(SurfaceMetrics {
                width,
                height,
                scaling,
            }),
        }
    }
}

/// A control drawing Rust-owned bitmap layers. Cheap to clone; every method
/// must be called on the UI thread.
#[derive(Clone)]
pub struct PixelSurface {
    raw: sys::ComPtr<sys::IAvnPixelSurface>,
    control: Control,
}

impl PixelSurface {
    /// How many layers a surface draws.
    pub const MAX_LAYERS: usize = 8;

    /// The control to place in a layout.
    pub fn control(&self) -> &Control {
        &self.control
    }

    /// Allocates layer `layer` as `width` by `height` transparent pixels, or
    /// removes it when either is 0. Keeps the pixels when the size and format
    /// are unchanged.
    pub fn set_layer(&self, layer: usize, width: u32, height: u32, format: PixelFormat) -> Result<()> {
        let format = match format {
            PixelFormat::Rgba8 => 0,
            PixelFormat::Bgra8 => 1,
        };
        Ok(self
            .raw
            .set_layer(layer as i32, abi_len(width)?, abi_len(height)?, format)?)
    }

    /// Copies `width` by `height` pixels into the layer at (`x`, `y`) and
    /// redraws. `pixels` holds rows `stride` bytes apart, in the layer's
    /// format; the rectangle is clipped to the layer.
    #[allow(clippy::too_many_arguments)]
    pub fn write_layer(
        &self,
        layer: usize,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        pixels: &[u8],
        stride: usize,
    ) -> Result<()> {
        Ok(self.raw.write_layer(
            layer as i32,
            x,
            y,
            abi_len(width)?,
            abi_len(height)?,
            pixels,
            i32::try_from(stride).map_err(|_| sys::Error(sys::E_INVALIDARG))?,
        )?)
    }

    /// Allocates the layer at `width` by `height` and fills it with `pixels`
    /// (tightly packed rows).
    pub fn set_layer_pixels(
        &self,
        layer: usize,
        width: u32,
        height: u32,
        format: PixelFormat,
        pixels: &[u8],
    ) -> Result<()> {
        self.set_layer(layer, width, height, format)?;
        self.write_layer(layer, 0, 0, width, height, pixels, width as usize * 4)
    }

    /// Where and how the layer is drawn.
    pub fn place_layer(
        &self,
        layer: usize,
        placement: Placement,
        interpolation: Interpolation,
        opacity: f64,
    ) -> Result<()> {
        let (x, y, width, height, fill) = match placement {
            Placement::Fill => (0.0, 0.0, 0.0, 0.0, true),
            Placement::Rect { x, y, width, height } => (x, y, width, height, false),
        };
        let interpolation = match interpolation {
            Interpolation::Default => 0,
            Interpolation::None => 1,
            Interpolation::Low => 2,
            Interpolation::Medium => 3,
            Interpolation::High => 4,
        };
        Ok(self
            .raw
            .place_layer(layer as i32, x, y, width, height, fill, interpolation, opacity)?)
    }

    /// Fills the control under the layers; `None` leaves it transparent.
    pub fn set_background(&self, argb: Option<u32>) -> Result<()> {
        Ok(self.raw.set_background(argb.unwrap_or(0))?)
    }

    pub fn set_cursor(&self, cursor: Cursor) -> Result<()> {
        Ok(self.raw.set_cursor(cursor as i32)?)
    }

    /// The size the surface asks for in layout; `(0, 0)` (the default) takes
    /// whatever the layout gives.
    pub fn set_desired_size(&self, width: f64, height: f64) -> Result<()> {
        Ok(self.raw.set_desired_size(width, height)?)
    }

    /// Moves keyboard focus to the surface.
    pub fn focus(&self) -> Result<()> {
        Ok(self.raw.focus()?)
    }

    pub fn metrics(&self) -> Result<SurfaceMetrics> {
        let (width, height, scaling) = self.raw.metrics()?;
        Ok(SurfaceMetrics {
            width,
            height,
            scaling,
        })
    }

    /// Stops reporting input and drops the handler closure (and whatever it
    /// captured). The surface keeps drawing.
    pub fn detach(&self) -> Result<()> {
        Ok(self.raw.detach()?)
    }
}

fn abi_len(value: u32) -> Result<i32> {
    Ok(i32::try_from(value).map_err(|_| sys::Error(sys::E_INVALIDARG))?)
}

impl AppContext {
    fn surfaces(&self) -> Result<sys::ComPtr<sys::IAvnApplication7>> {
        Ok(self.application().pixel_surfaces()?)
    }

    /// Creates a surface reporting its input to `handler`, which returns
    /// whether it handled the event (a handled event does not bubble). The
    /// handler runs on the UI thread; an input raised while it is already
    /// running (from something it called) is not delivered.
    pub fn create_pixel_surface(
        &self,
        mut handler: impl FnMut(&SurfaceInput) -> Result<bool> + Send + 'static,
    ) -> Result<PixelSurface> {
        let sys_handler = sys::pixel_surface_handler(move |input| {
            handler(&SurfaceInput::from_abi(input)).map_err(|error| match error {
                crate::Error::Abi(error) => error,
                _ => sys::Error(sys::E_FAIL),
            })
        });
        self.wrap(self.surfaces()?.create_pixel_surface(Some(&sys_handler))?)
    }

    /// Creates a surface that only draws: it reports no input.
    pub fn create_display_surface(&self) -> Result<PixelSurface> {
        self.wrap(self.surfaces()?.create_pixel_surface(None)?)
    }

    fn wrap(&self, raw: sys::ComPtr<sys::IAvnPixelSurface>) -> Result<PixelSurface> {
        let control = Control {
            raw: raw.control()?,
        };
        Ok(PixelSurface { raw, control })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_input_maps_kinds_buttons_and_modifiers() {
        let input = SurfaceInput::from_abi(&sys::PixelSurfaceInput::Pointer {
            kind: 2,
            x: 3.0,
            y: 4.0,
            button: 1,
            buttons: 4,
            modifiers: crate::key_modifiers::SHIFT | crate::key_modifiers::CONTROL,
            click_count: 1,
            delta_x: 0.0,
            delta_y: 0.0,
        });
        let SurfaceInput::Pointer(p) = input else { panic!("not a pointer event") };
        assert_eq!(PointerKind::Released, p.kind);
        assert_eq!(MouseButton::Left, p.button);
        assert!(!p.is_pressed(MouseButton::Left));
        assert!(p.is_pressed(MouseButton::Middle));
        assert!(p.modifiers.shift() && p.modifiers.control() && !p.modifiers.alt());
    }

    #[test]
    fn physical_size_rounds_up() {
        let m = SurfaceMetrics {
            width: 100.5,
            height: 10.0,
            scaling: 1.5,
        };
        assert_eq!((151, 15), m.physical_size());
    }
}

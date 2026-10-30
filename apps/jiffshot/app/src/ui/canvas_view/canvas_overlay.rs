//! The canvas's overlay layer: the pixel grid, the selection box and
//! handles, the marquee with its crop shade and size label, and the canvas
//! edge handles, drawn with `tiny-skia` into a control-sized pixmap in
//! physical pixels.
//!
//! Only the part that changed is cleared and sent to the surface: the layer
//! remembers the area it last painted.

use crate::canvas::Overlay;
use jiffshot_doc::text::{self, Align, TextStyle};
use jiffshot_doc::{Box2, IRect, Rgba};
use rustolonia::pixel_surface::{Interpolation, PixelFormat, Placement};
use rustolonia::{PixelSurface, Result};
use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform};

/// The zoom from which the pixel grid (when shown) draws its lines.
pub const GRID_ZOOM: f64 = 6.0;

/// The rotate handle; other handles from 8 up are round, 0–7 square.
const HANDLE_ROTATE: u8 = 12;
const HANDLE_SIZE: f32 = 9.0;

/// How far outside the view geometry is still drawn as is (in DIPs); larger
/// rectangles are clipped there, which keeps paths small at high zoom.
const CLIP_MARGIN: f32 = 4096.0;

/// Extra room around painted geometry for stroke widths and antialiasing,
/// in DIPs.
const PAINT_MARGIN: f32 = 3.0;

const BLUE: [u8; 3] = [0x3D, 0x8B, 0xFF];

/// Everything the overlay shows; the layer is redrawn when it changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub overlay: Overlay,
    pub zoom: f64,
    /// Where the document's top-left corner is in the control (DIPs).
    pub offset: (f64, f64),
    /// The document size in pixels.
    pub image: (u32, u32),
    pub grid: bool,
    /// The control size (DIPs) and its physical pixels per DIP.
    pub view: (f64, f64),
    pub scaling: f64,
}

impl Scene {
    fn physical_size(&self) -> (u32, u32) {
        let pixels = |dips: f64| (dips * self.scaling).ceil().max(0.0) as u32;
        (pixels(self.view.0), pixels(self.view.1))
    }

    /// A document point in the control (DIPs).
    fn to_view(&self, x: f32, y: f32) -> (f32, f32) {
        ((x as f64 * self.zoom + self.offset.0) as f32, (y as f64 * self.zoom + self.offset.1) as f32)
    }

    /// A document box in the control, normalized.
    fn view_box(&self, b: &Box2) -> Box2 {
        let (x0, y0) = self.to_view(b.x0.min(b.x1), b.y0.min(b.y1));
        let (x1, y1) = self.to_view(b.x0.max(b.x1), b.y0.max(b.y1));
        Box2 { x0, y0, x1, y1 }
    }

    /// The document's area in the control.
    fn image_box(&self) -> Box2 {
        self.view_box(&Box2 { x0: 0.0, y0: 0.0, x1: self.image.0 as f32, y1: self.image.1 as f32 })
    }

    fn view_box_of_control(&self) -> Box2 {
        Box2 { x0: 0.0, y0: 0.0, x1: self.view.0 as f32, y1: self.view.1 as f32 }
    }
}

/// The overlay's pixmap and what it last drew.
#[derive(Default)]
pub struct OverlayLayer {
    pixmap: Option<Pixmap>,
    /// The area holding non-transparent pixels, in physical pixels.
    painted: Option<IRect>,
    drawn: Option<Scene>,
}

impl OverlayLayer {
    /// Redraws `scene` into `layer` of `surface` unless it is what the layer
    /// already shows.
    pub fn present(&mut self, scene: &Scene, surface: &PixelSurface, layer: usize) -> Result<()> {
        if self.drawn.as_ref() == Some(scene) {
            return Ok(());
        }
        let (width, height) = scene.physical_size();
        if self.pixmap.as_ref().map(|p| (p.width(), p.height())) != Some((width, height)) {
            // A new (or removed) layer bitmap starts transparent.
            self.pixmap = Pixmap::new(width, height);
            self.painted = None;
            match &self.pixmap {
                Some(_) => {
                    surface.set_layer(layer, width, height, PixelFormat::Rgba8)?;
                    surface.place_layer(layer, Placement::Fill, Interpolation::None, 1.0)?;
                }
                None => surface.set_layer(layer, 0, 0, PixelFormat::Rgba8)?,
            }
        }
        self.drawn = Some(scene.clone());
        let Some(pixmap) = self.pixmap.as_mut() else { return Ok(()) };
        let bounds = IRect::of_size(width, height);
        if let Some(old) = self.painted {
            clear(pixmap, old);
        }
        let painted = paint(pixmap, scene).and_then(|r| r.intersect(&bounds));
        let dirty = match (self.painted, painted) {
            (Some(a), Some(b)) => Some(a.union(&b)),
            (a, b) => a.or(b),
        };
        self.painted = painted;
        if let Some(r) = dirty {
            let stride = width as usize * 4;
            let start = r.y as usize * stride + r.x as usize * 4;
            surface.write_layer(layer, r.x, r.y, r.w as u32, r.h as u32, &pixmap.data()[start..], stride)?;
        }
        Ok(())
    }
}

/// Makes `area` (inside the pixmap) transparent.
fn clear(pixmap: &mut Pixmap, area: IRect) {
    let stride = pixmap.width() as usize * 4;
    let data = pixmap.data_mut();
    for row in area.y..area.bottom() {
        let start = row as usize * stride + area.x as usize * 4;
        if let Some(span) = data.get_mut(start..start + area.w as usize * 4) {
            span.fill(0);
        }
    }
}

fn paint_of(r: u8, g: u8, b: u8, a: u8) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, a);
    paint.anti_alias = true;
    paint
}

/// A stroke `width` DIPs wide; `dashes` are in multiples of the width, as
/// Avalonia's dash styles are.
fn stroke_of(width: f32, dashes: Option<[f32; 2]>) -> Stroke {
    Stroke { width, dash: dashes.and_then(|d| StrokeDash::new(vec![d[0] * width, d[1] * width], 0.0)), ..Stroke::default() }
}

fn rect_path(b: &Box2) -> Option<Path> {
    Rect::from_ltrb(b.x0, b.y0, b.x1, b.y1).map(PathBuilder::from_rect)
}

fn inflate(b: &Box2, d: f32) -> Box2 {
    Box2 { x0: b.x0 - d, y0: b.y0 - d, x1: b.x1 + d, y1: b.y1 + d }
}

fn intersect(a: &Box2, b: &Box2) -> Option<Box2> {
    let r = Box2 { x0: a.x0.max(b.x0), y0: a.y0.max(b.y0), x1: a.x1.min(b.x1), y1: a.y1.min(b.y1) };
    (r.x1 > r.x0 && r.y1 > r.y0).then_some(r)
}

/// A rectangle with corners rounded by `radius`.
fn rounded_rect(b: &Box2, radius: f32) -> Option<Path> {
    let r = radius.min((b.x1 - b.x0) / 2.0).min((b.y1 - b.y0) / 2.0).max(0.0);
    let mut p = PathBuilder::new();
    p.move_to(b.x0 + r, b.y0);
    p.line_to(b.x1 - r, b.y0);
    p.quad_to(b.x1, b.y0, b.x1, b.y0 + r);
    p.line_to(b.x1, b.y1 - r);
    p.quad_to(b.x1, b.y1, b.x1 - r, b.y1);
    p.line_to(b.x0 + r, b.y1);
    p.quad_to(b.x0, b.y1, b.x0, b.y1 - r);
    p.line_to(b.x0, b.y0 + r);
    p.quad_to(b.x0, b.y0, b.x0 + r, b.y0);
    p.close();
    p.finish()
}

/// Draws in DIPs onto a physical-pixel pixmap and tracks the area touched.
struct Painter<'a> {
    pixmap: &'a mut Pixmap,
    scale: f32,
    /// Touched area in DIPs.
    touched: Option<Box2>,
}

impl Painter<'_> {
    fn transform(&self) -> Transform {
        Transform::from_scale(self.scale, self.scale)
    }

    fn touch(&mut self, path: &Path) {
        let b = path.bounds();
        let b = Box2 { x0: b.left(), y0: b.top(), x1: b.right(), y1: b.bottom() };
        self.touched = Some(match self.touched {
            Some(t) => Box2 { x0: t.x0.min(b.x0), y0: t.y0.min(b.y0), x1: t.x1.max(b.x1), y1: t.y1.max(b.y1) },
            None => b,
        });
    }

    fn fill(&mut self, path: Option<Path>, paint: &Paint, rule: FillRule) {
        let Some(path) = path else { return };
        self.pixmap.fill_path(&path, paint, rule, self.transform(), None);
        self.touch(&path);
    }

    fn stroke(&mut self, path: Option<Path>, paint: &Paint, stroke: &Stroke) {
        let Some(path) = path else { return };
        self.pixmap.stroke_path(&path, paint, stroke, self.transform(), None);
        self.touch(&path);
    }

    /// The touched area in physical pixels, with room for strokes.
    fn painted(&self) -> Option<IRect> {
        let t = inflate(&self.touched?, PAINT_MARGIN);
        let s = self.scale;
        let (x0, y0) = ((t.x0 * s).floor().max(0.0) as i32, (t.y0 * s).floor().max(0.0) as i32);
        let (x1, y1) = ((t.x1 * s).ceil().min(i32::MAX as f32) as i32, (t.y1 * s).ceil().min(i32::MAX as f32) as i32);
        (x1 > x0 && y1 > y0).then(|| IRect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

/// Draws the scene onto a transparent pixmap; returns the area painted.
fn paint(pixmap: &mut Pixmap, scene: &Scene) -> Option<IRect> {
    let mut painter = Painter { pixmap, scale: scene.scaling.max(0.01) as f32, touched: None };
    let view = scene.view_box_of_control();
    let clip = inflate(&view, CLIP_MARGIN);
    let image = scene.image_box();
    let o = &scene.overlay;
    let selection_pen = stroke_of(1.2, Some([4.0, 3.0]));
    let handle_pen = stroke_of(1.5, None);
    let blue = paint_of(BLUE[0], BLUE[1], BLUE[2], 255);
    let white = paint_of(255, 255, 255, 255);

    if scene.grid && scene.zoom >= GRID_ZOOM && scene.image.0 > 0 && scene.image.1 > 0 {
        draw_grid(&mut painter, scene, &image, &view);
    }
    for b in &o.others {
        let r = inflate(&scene.view_box(b), 1.0);
        painter.stroke(intersect(&r, &clip).as_ref().and_then(rect_path), &blue, &selection_pen);
    }
    if let Some(marquee) = &o.marquee {
        let m = scene.view_box(marquee);
        let shown = intersect(&m, &clip);
        if o.box_select {
            painter.fill(shown.as_ref().and_then(rect_path), &paint_of(BLUE[0], BLUE[1], BLUE[2], 0x22), FillRule::Winding);
            painter.stroke(shown.as_ref().and_then(rect_path), &blue, &selection_pen);
        } else {
            // Shade what a crop would remove.
            if let Some(area) = intersect(&image, &clip) {
                let mut shade = PathBuilder::new();
                if let Some(r) = Rect::from_ltrb(area.x0, area.y0, area.x1, area.y1) {
                    shade.push_rect(r);
                }
                if let Some(r) = intersect(&m, &area).and_then(|k| Rect::from_ltrb(k.x0, k.y0, k.x1, k.y1)) {
                    shade.push_rect(r);
                }
                painter.fill(shade.finish(), &paint_of(0, 0, 0, 0x70), FillRule::EvenOdd);
            }
            painter.stroke(shown.as_ref().and_then(rect_path), &white, &stroke_of(1.5, Some([5.0, 4.0])));
            draw_size_label(&mut painter, &m, marquee);
        }
    }
    if o.canvas_handles {
        let fill = paint_of(0xC8, 0xCC, 0xD2, 255);
        for (_, p) in &o.handles {
            let (x, y) = scene.to_view(p.x, p.y);
            let r = Box2 { x0: x - 4.0, y0: y - 4.0, x1: x + 4.0, y1: y + 4.0 };
            painter.fill(rect_path(&r), &fill, FillRule::Winding);
            painter.stroke(rect_path(&r), &blue, &handle_pen);
        }
    }
    if let Some(s) = &o.selection {
        let s = inflate(&scene.view_box(s), 1.0);
        let (cx, cy) = ((s.x0 + s.x1) / 2.0, (s.y0 + s.y1) / 2.0);
        let turned = intersect(&s, &clip).as_ref().and_then(rect_path).and_then(|p| p.transform(Transform::from_rotate_at(o.rotation, cx, cy)));
        painter.stroke(turned, &blue, &selection_pen);
        for (id, p) in &o.handles {
            let (x, y) = scene.to_view(p.x, p.y);
            let shape = if *id == HANDLE_ROTATE {
                PathBuilder::from_circle(x, y, HANDLE_SIZE / 2.0 + 1.0)
            } else if *id >= 8 {
                PathBuilder::from_circle(x, y, HANDLE_SIZE / 2.0)
            } else {
                let h = HANDLE_SIZE / 2.0;
                rect_path(&Box2 { x0: x - h, y0: y - h, x1: x + h, y1: y + h })
            };
            painter.fill(shape.clone(), &white, FillRule::Winding);
            painter.stroke(shape, &blue, &handle_pen);
        }
    }
    painter.painted()
}

/// One line between each pair of image pixels in view, 1 DIP wide.
fn draw_grid(painter: &mut Painter, scene: &Scene, image: &Box2, view: &Box2) {
    let Some(shown) = intersect(image, view) else { return };
    let zoom = scene.zoom as f32;
    let (w, h) = (scene.image.0 as i64, scene.image.1 as i64);
    let x0 = ((shown.x0 - image.x0) / zoom).floor() as i64;
    let x1 = ((shown.x1 - image.x0) / zoom).ceil() as i64;
    let y0 = ((shown.y0 - image.y0) / zoom).floor() as i64;
    let y1 = ((shown.y1 - image.y0) / zoom).ceil() as i64;
    let mut lines = PathBuilder::new();
    for x in x0.max(1)..x1.min(w) {
        let at = (image.x0 + x as f32 * zoom).round() + 0.5;
        lines.move_to(at, shown.y0);
        lines.line_to(at, shown.y1);
    }
    for y in y0.max(1)..y1.min(h) {
        let at = (image.y0 + y as f32 * zoom).round() + 0.5;
        lines.move_to(shown.x0, at);
        lines.line_to(shown.x1, at);
    }
    painter.stroke(lines.finish(), &paint_of(0x80, 0x80, 0x80, 0x50), &stroke_of(1.0, None));
}

/// "W × H" in a dark tag above the marquee `m` (control DIPs) of document
/// size `size`.
fn draw_size_label(painter: &mut Painter, m: &Box2, size: &Box2) {
    let label = format!("{} × {}", (size.x1 - size.x0).abs().round(), (size.y1 - size.y0).abs().round());
    let s = painter.scale;
    let layout = text::layout(&label, &TextStyle { family: text::DEFAULT_FAMILY, size: 12.0 * s, bold: false, italic: false }, None);
    let (width, height) = (layout.width() / s, layout.height() / s);
    let (x, y) = (m.x0, (m.y0 - 20.0).max(0.0));
    let tag = Box2 { x0: x, y0: y, x1: x + width + 10.0, y1: y + height + 4.0 };
    painter.fill(rounded_rect(&tag, 3.0), &paint_of(0x20, 0x20, 0x20, 0xC0), FillRule::Winding);
    text::draw(painter.pixmap, (0, 0), &layout, (x + 5.0) * s, (y + 2.0) * s, layout.width(), Align::Left, Rgba::new(255, 255, 255, 255));
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiffshot_doc::Pt;

    fn scene() -> Scene {
        Scene { zoom: 2.0, offset: (10.0, 20.0), image: (100, 50), view: (400.0, 300.0), scaling: 1.5, ..Scene::default() }
    }

    #[test]
    fn empty_scene_paints_nothing() {
        let s = scene();
        let (w, h) = s.physical_size();
        let mut pixmap = Pixmap::new(w, h).unwrap();
        assert_eq!(None, paint(&mut pixmap, &s));
        assert!(pixmap.data().iter().all(|b| *b == 0));
    }

    #[test]
    fn painted_area_covers_handles() {
        let mut s = scene();
        s.overlay.selection = Some(Box2 { x0: 10.0, y0: 10.0, x1: 20.0, y1: 20.0 });
        s.overlay.handles = vec![(0, Pt::new(10.0, 10.0)), (4, Pt::new(20.0, 20.0))];
        let (w, h) = s.physical_size();
        let mut pixmap = Pixmap::new(w, h).unwrap();
        let r = paint(&mut pixmap, &s).unwrap();
        // Handles at view (30, 40) and (50, 60) DIPs, 9 DIPs wide, at 1.5x.
        assert!(r.x <= ((30.0 - 4.5) * 1.5) as i32 && r.right() >= ((50.0 + 4.5) * 1.5) as i32);
        let stride = w as usize * 4;
        let outside = pixmap.data().chunks(stride).enumerate().any(|(y, row)| {
            row.chunks(4).enumerate().any(|(x, px)| px[3] != 0 && !(x as i32 >= r.x && (x as i32) < r.right() && y as i32 >= r.y && (y as i32) < r.bottom()))
        });
        assert!(!outside, "pixels painted outside the reported area");
    }

    #[test]
    fn crop_marquee_shades_outside_only() {
        let mut s = scene();
        s.scaling = 1.0;
        s.overlay.marquee = Some(Box2 { x0: 20.0, y0: 10.0, x1: 60.0, y1: 40.0 });
        let (w, h) = s.physical_size();
        let mut pixmap = Pixmap::new(w, h).unwrap();
        paint(&mut pixmap, &s).unwrap();
        let alpha = |x: u32, y: u32| pixmap.pixel(x, y).unwrap().alpha();
        // Inside the image, outside the marquee: shaded.
        assert!((0x6E..=0x71).contains(&alpha(15, 25)));
        // Inside the marquee: clear.
        assert_eq!(0, alpha(90, 60));
        // Outside the image: clear.
        assert_eq!(0, alpha(300, 250));
    }
}

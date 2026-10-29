//! Annotation objects: geometry, style, hit testing, handles and drawing.
//!
//! Objects are drawn in document order onto whatever is already on the
//! target (the background and the objects below). Blur, magnify and
//! spotlight read those pixels, which is why [`Object::reads`] exists:
//! re-rendering part of the canvas must include everything such an object
//! samples.

use crate::color::Rgba;
use crate::geom::{dist_to_segment, quad_at, Box2, IRect, Pt};
use crate::raster;
use crate::stamps::{self, Stamp};
use crate::text::{self, Align, TextStyle};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tiny_skia::{
    FillRule, FilterQuality, LineCap, LineJoin, Paint, Path, PathBuilder, Pattern, Pixmap, SpreadMode, Stroke,
    StrokeDash, Transform,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cap {
    #[default]
    None,
    Arrow,
    Dot,
    /// An open, two-stroke arrowhead.
    OpenArrow,
    Square,
    Diamond,
    /// The line narrows to a point at this end.
    Taper,
}

pub const CAPS: &[(&str, Cap)] = &[
    ("None", Cap::None),
    ("Arrow", Cap::Arrow),
    ("Open arrow", Cap::OpenArrow),
    ("Dot", Cap::Dot),
    ("Square", Cap::Square),
    ("Diamond", Cap::Diamond),
    ("Taper", Cap::Taper),
];

impl Cap {
    pub fn name(self) -> &'static str {
        CAPS.iter().find(|(_, c)| *c == self).map_or("None", |(n, _)| n)
    }

    pub fn from_name(name: &str) -> Cap {
        CAPS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map_or(Cap::None, |(_, c)| *c)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dash {
    #[default]
    Solid,
    Dash,
    Dot,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind {
    #[default]
    Rectangle,
    Rounded,
    Ellipse,
    Triangle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlurMode {
    #[default]
    Blur,
    Pixelate,
    Redact,
}

/// Visual properties shared by every kind; each kind uses the ones that
/// apply to it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    pub stroke: Rgba,
    pub fill: Rgba,
    pub width: f32,
    pub opacity: f32,
    pub shadow: bool,
    pub dash: Dash,
    pub start_cap: Cap,
    pub end_cap: Cap,
    pub font: String,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub text_color: Rgba,
    pub align: Align,
    /// Multiply blending, for highlighters.
    pub multiply: bool,
    /// Arrowhead size relative to the line width.
    pub head_size: f32,
    pub shadow_color: Rgba,
    /// Direction the shadow falls, in degrees (0 = right, 90 = down).
    pub shadow_angle: f32,
    pub shadow_distance: f32,
    pub shadow_blur: f32,
    /// Outline drawn around each letter (transparent or zero width: none).
    pub outline: Rgba,
    pub outline_width: f32,
    pub underline: bool,
    pub strike: bool,
    pub valign: VAlign,
    /// Space between a box and its text; negative means automatic.
    pub padding: f32,
    /// Second gradient color for fills (transparent: a solid fill).
    pub fill2: Rgba,
    pub gradient_radial: bool,
    /// Direction of a linear gradient, in degrees.
    pub gradient_angle: f32,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            stroke: Rgba::RED,
            fill: Rgba::TRANSPARENT,
            width: 6.0,
            opacity: 1.0,
            shadow: false,
            dash: Dash::Solid,
            start_cap: Cap::None,
            end_cap: Cap::None,
            font: text::DEFAULT_FAMILY.to_owned(),
            font_size: 28.0,
            bold: false,
            italic: false,
            text_color: Rgba::BLACK,
            align: Align::Left,
            multiply: false,
            head_size: 1.0,
            shadow_color: Rgba::BLACK.with_alpha(115),
            shadow_angle: 45.0,
            shadow_distance: 5.0,
            shadow_blur: 5.0,
            outline: Rgba::TRANSPARENT,
            outline_width: 0.0,
            underline: false,
            strike: false,
            valign: VAlign::Top,
            padding: -1.0,
            fill2: Rgba::TRANSPARENT,
            gradient_radial: false,
            gradient_angle: 90.0,
        }
    }
}

impl Style {
    fn text_style(&self) -> TextStyle<'_> {
        TextStyle { family: &self.font, size: self.font_size, bold: self.bold, italic: self.italic }
    }
}

/// A raster used by an [`Kind::Image`] object. Only the key is serialized;
/// files store the pixels next to the document JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageRef {
    pub key: String,
    #[serde(skip)]
    pub pixmap: Option<Arc<Pixmap>>,
}

impl ImageRef {
    pub fn new(key: impl Into<String>, pixmap: Pixmap) -> ImageRef {
        ImageRef { key: key.into(), pixmap: Some(Arc::new(pixmap)) }
    }
}

impl PartialEq for ImageRef {
    fn eq(&self, other: &ImageRef) -> bool {
        self.key == other.key
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Kind {
    /// A line or arrow (caps come from the style), optionally bent through
    /// a quadratic control point.
    Line { a: Pt, b: Pt, #[serde(default)] control: Option<Pt> },
    Shape { rect: Box2, shape: ShapeKind },
    Text { rect: Box2, text: String },
    /// A text box with a tail pointing at `tail`: a wedge, or an arrow
    /// when `arrow` is set.
    Callout {
        rect: Box2,
        tail: Pt,
        text: String,
        shape: ShapeKind,
        #[serde(default)]
        arrow: bool,
        /// Tails besides the first.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        more_tails: Vec<Pt>,
    },
    Step { center: Pt, radius: f32, label: String, #[serde(default = "default_step_shape")] shape: ShapeKind },
    Pen { points: Vec<Pt> },
    Highlight { rect: Box2, #[serde(default)] shape: ShapeKind },
    Blur { rect: Box2, mode: BlurMode, amount: f32 },
    /// A lens showing the content under its center enlarged by `zoom`.
    Magnify { rect: Box2, zoom: f32, ellipse: bool, #[serde(default)] offset: Pt },
    /// A closed shape through `points`.
    Polygon { points: Vec<Pt> },
    /// Dims (and optionally blurs) everything outside `rect`.
    Spotlight { rect: Box2, ellipse: bool, dim: f32, blur: f32, #[serde(default)] rounded: bool },
    Stamp { rect: Box2, stamp: Stamp },
    Image { rect: Box2, image: ImageRef },
    /// Objects that move, resize and stack as one.
    Group { objects: Vec<Object> },
}

/// Handle ids. 0–7 run clockwise around a box from the top-left corner.
pub type Handle = u8;
pub const H_TL: Handle = 0;
pub const H_T: Handle = 1;
pub const H_TR: Handle = 2;
pub const H_R: Handle = 3;
pub const H_BR: Handle = 4;
pub const H_B: Handle = 5;
pub const H_BL: Handle = 6;
pub const H_L: Handle = 7;
pub const H_START: Handle = 8;
pub const H_END: Handle = 9;
pub const H_CONTROL: Handle = 10;
pub const H_TAIL: Handle = 11;
pub const H_ROTATE: Handle = 12;
/// Where a magnifier shows its view.
pub const H_VIEW: Handle = 13;
/// A callout's extra tails are `H_MORE_TAILS + index` (up to six).
pub const H_MORE_TAILS: Handle = 14;
/// Polygon corners are `H_VERTEX + index`.
pub const H_VERTEX: Handle = 20;

fn default_step_shape() -> ShapeKind {
    ShapeKind::Ellipse
}

const EVERYWHERE: f32 = 1.0e6;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub id: u64,
    pub kind: Kind,
    pub style: Style,
    /// Clockwise rotation in degrees about the center of the object's box.
    #[serde(default)]
    pub rotation: f32,
    /// Makes an image object a template drop zone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<Zone>,
}

/// A template placeholder: the image fills it (cropped) or fits inside
/// it, moved by `pan` from the middle; empty, it shows where to drop one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    #[serde(default)]
    pub fit: bool,
    #[serde(default)]
    pub pan: Pt,
}

impl Zone {
    /// Where an `iw` x `ih` image goes in `rect`, with the pan kept so a
    /// filled zone never shows a gap.
    pub fn placement(&self, rect: &Box2, iw: f32, ih: f32) -> (Box2, Pt) {
        let (zw, zh) = (rect.width(), rect.height());
        let scale = if self.fit { (zw / iw).min(zh / ih) } else { (zw / iw).max(zh / ih) };
        let (w, h) = (iw * scale, ih * scale);
        let (mx, my) = (((w - zw) / 2.0).abs(), ((h - zh) / 2.0).abs());
        let pan = Pt::new(self.pan.x.clamp(-mx, mx), self.pan.y.clamp(-my, my));
        let c = rect.center();
        (Box2::around(Pt::new(c.x + pan.x, c.y + pan.y), w / 2.0, h / 2.0), pan)
    }
}

/// A unique key for a new image in a document.
pub fn fresh_key(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_micros()).unwrap_or(0);
    format!("{prefix}-{t:x}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Rotates `p` by `degrees` (clockwise on screen) about `c`.
pub fn rotate_about(p: Pt, c: Pt, degrees: f32) -> Pt {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let d = p.sub(c);
    Pt::new(c.x + d.x * cos - d.y * sin, c.y + d.x * sin + d.y * cos)
}

pub fn arrow_head(width: f32) -> f32 {
    width * 2.6 + 8.0
}

fn text_padding(style: &Style) -> f32 {
    if style.padding >= 0.0 {
        style.padding
    } else {
        (style.font_size * 0.25).max(4.0)
    }
}

impl Object {
    pub fn new(id: u64, kind: Kind, style: Style) -> Object {
        Object { id, kind, style, rotation: 0.0, zone: None }
    }

    /// Whether this object can be rotated: everything but objects that
    /// sample the pixels below them, and lines and strokes (whose points
    /// are simply moved instead).
    pub fn rotatable(&self) -> bool {
        !matches!(
            self.kind,
            Kind::Line { .. }
                | Kind::Pen { .. }
                | Kind::Polygon { .. }
                | Kind::Blur { .. }
                | Kind::Magnify { .. }
                | Kind::Spotlight { .. }
                | Kind::Group { .. }
        )
    }

    fn rotated(&self) -> bool {
        self.rotation.abs() > 0.01 && self.rotatable()
    }

    /// The point rotation turns around.
    pub fn pivot(&self) -> Pt {
        self.rect().map(|r| r.center()).unwrap_or_default()
    }

    /// `p` in the object's unrotated frame.
    fn local(&self, p: Pt) -> Pt {
        if self.rotated() {
            rotate_about(p, self.pivot(), -self.rotation)
        } else {
            p
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            Kind::Line { .. } if self.style.end_cap == Cap::Arrow || self.style.start_cap == Cap::Arrow => "Arrow",
            Kind::Line { .. } => "Line",
            Kind::Shape { .. } => "Shape",
            Kind::Text { .. } => "Text",
            Kind::Callout { .. } => "Callout",
            Kind::Step { .. } => "Step",
            Kind::Pen { .. } if self.style.multiply => "Marker",
            Kind::Pen { .. } => "Pen",
            Kind::Highlight { .. } => "Highlight",
            Kind::Polygon { .. } => "Shape",
            Kind::Blur { .. } => "Blur",
            Kind::Magnify { .. } => "Magnify",
            Kind::Spotlight { .. } => "Spotlight",
            Kind::Stamp { .. } => "Stamp",
            Kind::Image { .. } => "Image",
            Kind::Group { .. } => "Group",
        }
    }

    /// Calls `f` on this object and, for groups, every object inside.
    pub fn walk(&self, f: &mut dyn FnMut(&Object)) {
        f(self);
        if let Kind::Group { objects } = &self.kind {
            for o in objects {
                o.walk(f);
            }
        }
    }

    pub fn walk_mut(&mut self, f: &mut dyn FnMut(&mut Object)) {
        f(self);
        if let Kind::Group { objects } = &mut self.kind {
            for o in objects {
                o.walk_mut(f);
            }
        }
    }

    /// The box of box-shaped kinds.
    pub fn rect(&self) -> Option<Box2> {
        match &self.kind {
            Kind::Shape { rect, .. }
            | Kind::Text { rect, .. }
            | Kind::Callout { rect, .. }
            | Kind::Highlight { rect, .. }
            | Kind::Blur { rect, .. }
            | Kind::Magnify { rect, .. }
            | Kind::Spotlight { rect, .. }
            | Kind::Stamp { rect, .. }
            | Kind::Image { rect, .. } => Some(*rect),
            Kind::Step { center, radius, .. } => Some(Box2::around(*center, *radius, *radius)),
            Kind::Pen { points } => Box2::bounding(points.iter().copied()),
            Kind::Line { a, b, control } => Box2::bounding([*a, *b].into_iter().chain(*control)),
            Kind::Polygon { points } => Box2::bounding(points.iter().copied()),
            Kind::Group { objects } => objects.iter().filter_map(|o| o.rect()).reduce(|a, b| a.union(&b)),
        }
    }

    fn rect_mut(&mut self) -> Option<&mut Box2> {
        match &mut self.kind {
            Kind::Shape { rect, .. }
            | Kind::Text { rect, .. }
            | Kind::Callout { rect, .. }
            | Kind::Highlight { rect, .. }
            | Kind::Blur { rect, .. }
            | Kind::Magnify { rect, .. }
            | Kind::Spotlight { rect, .. }
            | Kind::Stamp { rect, .. }
            | Kind::Image { rect, .. } => Some(rect),
            _ => None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match &self.kind {
            Kind::Text { text, .. } | Kind::Callout { text, .. } => Some(text),
            Kind::Step { label, .. } => Some(label),
            _ => None,
        }
    }

    pub fn set_text(&mut self, value: &str) {
        match &mut self.kind {
            Kind::Text { text, .. } | Kind::Callout { text, .. } => *text = value.to_owned(),
            Kind::Step { label, .. } => *label = value.to_owned(),
            _ => return,
        }
        self.fit_text();
    }

    /// Grows a text box (never shrinks it) so its text fits.
    pub fn fit_text(&mut self) {
        let style = self.style.clone();
        if let Kind::Text { rect, text } = &mut self.kind {
            let pad = text_padding(&style);
            let layout = text::layout(text, &style.text_style(), Some(rect.width() - 2.0 * pad));
            let needed = layout.height() + 2.0 * pad;
            if rect.height() < needed {
                rect.y1 = rect.y0 + needed;
            }
        }
    }

    /// Pixels this object can change, including shadow and antialiasing.
    pub fn bounds(&self) -> Box2 {
        let b = self.base_bounds();
        if !self.rotated() {
            return b;
        }
        let c = self.pivot();
        let corners = [b.min(), Pt::new(b.x1, b.y0), b.max(), Pt::new(b.x0, b.y1)];
        Box2::bounding(corners.map(|q| rotate_about(q, c, self.rotation))).unwrap().inflate(2.0)
    }

    fn base_bounds(&self) -> Box2 {
        let w = self.style.width.max(0.0);
        let b = match &self.kind {
            Kind::Line { a, b, control } => {
                let caps = if self.style.start_cap == Cap::None && self.style.end_cap == Cap::None {
                    w / 2.0
                } else {
                    (arrow_head(w) * self.style.head_size.max(0.3)).max(w * 1.6)
                };
                Box2::bounding([*a, *b].into_iter().chain(*control)).unwrap().inflate(caps)
            }
            Kind::Pen { points } | Kind::Polygon { points } => Box2::bounding(points.iter().copied()).unwrap_or_default().inflate(w / 2.0),
            Kind::Magnify { rect, offset, .. } => rect.union(&rect.offset(offset.x, offset.y)).inflate(w / 2.0),
            Kind::Callout { rect, tail, more_tails, .. } => more_tails.iter().fold(rect.union(&Box2::from_points(*tail, *tail)), |b, t| b.union(&Box2::from_points(*t, *t))).inflate(w),
            Kind::Step { center, radius, .. } => Box2::around(*center, *radius, *radius).inflate(w / 2.0),
            Kind::Spotlight { .. } => Box2::new(-EVERYWHERE, -EVERYWHERE, EVERYWHERE, EVERYWHERE),
            Kind::Blur { rect, .. } | Kind::Highlight { rect, .. } => *rect,
            Kind::Group { objects } => return objects.iter().map(|o| o.bounds()).reduce(|a, b| a.union(&b)).unwrap_or_default(),
            _ => self.rect().unwrap().inflate(w / 2.0),
        };
        let b = b.inflate(2.0 + self.style.outline_width.max(0.0));
        if self.style.shadow {
            b.inflate(shadow_margin(&self.style))
        } else {
            b
        }
    }

    /// Pixels under the object this one samples when drawn.
    pub fn reads(&self) -> Option<Box2> {
        match &self.kind {
            Kind::Blur { rect, .. } | Kind::Magnify { rect, .. } => Some(*rect),
            Kind::Spotlight { blur, .. } if *blur > 0.0 => Some(self.bounds()),
            Kind::Group { objects } => objects.iter().filter_map(|o| o.reads()).reduce(|a, b| a.union(&b)),
            _ => None,
        }
    }

    pub fn hit(&self, p: Pt, tolerance: f32) -> bool {
        let p = self.local(p);
        let reach = self.style.width / 2.0 + tolerance;
        match &self.kind {
            Kind::Group { objects } => objects.iter().any(|o| o.hit(p, tolerance)),
            Kind::Polygon { points } => {
                let n = points.len();
                let edge = (0..n).any(|i| dist_to_segment(p, points[i], points[(i + 1) % n]) <= reach);
                edge || (self.style.fill.is_visible() && point_in_polygon(p, points))
            }
            Kind::Magnify { rect, offset, .. } => rect.inflate(tolerance).contains(p) || rect.offset(offset.x, offset.y).inflate(tolerance).contains(p),
            Kind::Line { a, b, control } => match control {
                None => dist_to_segment(p, *a, *b) <= reach,
                Some(c) => (0..24).any(|i| {
                    let t0 = i as f32 / 24.0;
                    let t1 = (i + 1) as f32 / 24.0;
                    dist_to_segment(p, quad_at(*a, *c, *b, t0), quad_at(*a, *c, *b, t1)) <= reach
                }),
            },
            Kind::Pen { points } => {
                points.windows(2).any(|s| dist_to_segment(p, s[0], s[1]) <= reach)
                    || points.first().is_some_and(|q| q.dist(p) <= reach)
            }
            Kind::Shape { rect, .. } if !self.style.fill.is_visible() => {
                rect.inflate(reach).contains(p) && !rect.inflate(-reach).contains(p)
            }
            Kind::Step { center, radius, .. } => center.dist(p) <= radius + tolerance,
            Kind::Callout { rect, tail, more_tails, .. } => {
                rect.inflate(tolerance).contains(p) || std::iter::once(tail).chain(more_tails).any(|t| dist_to_segment(p, rect.center(), *t) <= tolerance + 4.0)
            }
            Kind::Spotlight { rect, .. } => {
                rect.inflate(tolerance).contains(p) && !rect.inflate(-tolerance * 2.0).contains(p)
            }
            _ => self.rect().is_some_and(|r| r.inflate(tolerance).contains(p)),
        }
    }

    pub fn handles(&self) -> Vec<(Handle, Pt)> {
        match &self.kind {
            Kind::Line { a, b, control } => {
                let mid = match control {
                    Some(c) => quad_at(*a, *c, *b, 0.5),
                    None => a.lerp(*b, 0.5),
                };
                vec![(H_START, *a), (H_END, *b), (H_CONTROL, mid)]
            }
            Kind::Step { center, radius, .. } => vec![(H_R, center.offset(*radius, 0.0))],
            Kind::Polygon { points } => points.iter().enumerate().take(200).map(|(i, q)| (H_VERTEX + i as u8, *q)).collect(),
            Kind::Magnify { rect, offset, .. } if offset.len() > 0.5 => {
                let mut h = box_handles(rect);
                h.push((H_VIEW, rect.center().add(*offset)));
                h
            }
            _ if self.rotatable() => {
                let r = self.rect().unwrap_or_default();
                let c = r.center();
                let mut h: Vec<(Handle, Pt)> = [
                    (H_TL, Pt::new(r.x0, r.y0)),
                    (H_T, Pt::new(c.x, r.y0)),
                    (H_TR, Pt::new(r.x1, r.y0)),
                    (H_R, Pt::new(r.x1, c.y)),
                    (H_BR, Pt::new(r.x1, r.y1)),
                    (H_B, Pt::new(c.x, r.y1)),
                    (H_BL, Pt::new(r.x0, r.y1)),
                    (H_L, Pt::new(r.x0, c.y)),
                    (H_ROTATE, Pt::new(c.x, r.y0 - 28.0)),
                ]
                .into_iter()
                .map(|(id, q)| (id, rotate_about(q, c, self.rotation)))
                .collect();
                if let Kind::Callout { tail, more_tails, .. } = &self.kind {
                    h.push((H_TAIL, *tail));
                    h.extend(more_tails.iter().take(6).enumerate().map(|(i, t)| (H_MORE_TAILS + i as u8, *t)));
                }
                h
            }
            _ => {
                let r = self.rect().unwrap_or_default();
                let c = r.center();
                let mut h = vec![
                    (H_TL, Pt::new(r.x0, r.y0)),
                    (H_T, Pt::new(c.x, r.y0)),
                    (H_TR, Pt::new(r.x1, r.y0)),
                    (H_R, Pt::new(r.x1, c.y)),
                    (H_BR, Pt::new(r.x1, r.y1)),
                    (H_B, Pt::new(c.x, r.y1)),
                    (H_BL, Pt::new(r.x0, r.y1)),
                    (H_L, Pt::new(r.x0, c.y)),
                ];
                if let Kind::Callout { tail, more_tails, .. } = &self.kind {
                    h.push((H_TAIL, *tail));
                    h.extend(more_tails.iter().take(6).enumerate().map(|(i, t)| (H_MORE_TAILS + i as u8, *t)));
                }
                h
            }
        }
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        match &mut self.kind {
            Kind::Line { a, b, control } => {
                *a = a.offset(dx, dy);
                *b = b.offset(dx, dy);
                if let Some(c) = control {
                    *c = c.offset(dx, dy);
                }
            }
            Kind::Pen { points } | Kind::Polygon { points } => points.iter_mut().for_each(|p| *p = p.offset(dx, dy)),
            Kind::Step { center, .. } => *center = center.offset(dx, dy),
            Kind::Group { objects } => objects.iter_mut().for_each(|o| o.translate(dx, dy)),
            Kind::Callout { rect, tail, more_tails, .. } => {
                *rect = rect.offset(dx, dy);
                *tail = tail.offset(dx, dy);
                more_tails.iter_mut().for_each(|t| *t = t.offset(dx, dy));
            }
            _ => {
                if let Some(r) = self.rect_mut() {
                    *r = r.offset(dx, dy);
                }
            }
        }
    }

    /// Moves handle `h` to `p`. `square` keeps boxes square and snaps lines
    /// to 45° steps.
    pub fn move_handle(&mut self, h: Handle, p: Pt, square: bool) {
        if h == H_ROTATE {
            let c = self.pivot();
            let mut angle = (p.y - c.y).atan2(p.x - c.x).to_degrees() + 90.0;
            if square {
                angle = (angle / 15.0).round() * 15.0;
            }
            self.rotation = (angle + 360.0) % 360.0;
            return;
        }
        if self.rotated() && h <= H_L {
            // Resize in the object's own frame, keeping the opposite handle
            // where it is on screen.
            let anchor_id = (h + 4) % 8;
            let anchor = |o: &Object| o.handles().into_iter().find(|(id, _)| *id == anchor_id).map(|(_, q)| q).unwrap_or_default();
            let before = anchor(self);
            let local = self.local(p);
            let rotation = std::mem::replace(&mut self.rotation, 0.0);
            self.move_handle(h, local, square);
            self.rotation = rotation;
            let after = anchor(self);
            self.translate(before.x - after.x, before.y - after.y);
            return;
        }
        match &mut self.kind {
            Kind::Line { a, b, control } => match h {
                H_START => *a = if square { snap45(*b, p) } else { p },
                H_END => *b = if square { snap45(*a, p) } else { p },
                H_CONTROL => {
                    // The handle sits on the curve at t = 0.5; solve for the
                    // control point that puts it under the pointer.
                    let c = Pt::new(2.0 * p.x - (a.x + b.x) / 2.0, 2.0 * p.y - (a.y + b.y) / 2.0);
                    *control = (dist_to_segment(p, *a, *b) > 3.0).then_some(c);
                }
                _ => {}
            },
            Kind::Step { center, radius, .. } => *radius = center.dist(p).max(6.0),
            Kind::Callout { tail, .. } if h == H_TAIL => *tail = p,
            Kind::Callout { more_tails, .. } if (H_MORE_TAILS..H_VERTEX).contains(&h) => {
                if let Some(t) = more_tails.get_mut((h - H_MORE_TAILS) as usize) {
                    *t = p;
                }
            }
            Kind::Polygon { points } if h >= H_VERTEX => {
                if let Some(q) = points.get_mut((h - H_VERTEX) as usize) {
                    *q = p;
                }
            }
            Kind::Magnify { rect, offset, .. } if h == H_VIEW => *offset = p.sub(rect.center()),
            Kind::Group { objects } => {
                let Some(old) = objects.iter().filter_map(|o| o.rect()).reduce(|a, b| a.union(&b)) else { return };
                let new = resize_box(old, h, p, square);
                if new.is_degenerate(2.0) {
                    return;
                }
                for o in objects.iter_mut() {
                    o.map_geometry(&old, &new);
                }
            }
            Kind::Pen { points } => {
                let Some(old) = Box2::bounding(points.iter().copied()) else { return };
                let new = resize_box(old, h, p, square);
                for q in points.iter_mut() {
                    *q = old.map(*q, &new);
                }
            }
            _ => {
                if let Some(r) = self.rect_mut() {
                    *r = resize_box(*r, h, p, square);
                }
                self.fit_text();
            }
        }
    }

    /// Scales the object's geometry from `from` to `to` (used for image
    /// resizes and layouts).
    pub fn map_geometry(&mut self, from: &Box2, to: &Box2) {
        let scale = to.width() / from.width().max(1e-6);
        self.map_points(&|q| from.map(q, to), scale);
        self.style.width *= scale;
        self.style.font_size *= scale;
    }

    /// Applies `f` to every defining point. Boxes stay axis-aligned, so `f`
    /// must be a scale, translation, flip or quarter turn. `scale` resizes
    /// radii.
    pub fn map_points(&mut self, f: &dyn Fn(Pt) -> Pt, scale: f32) {
        match &mut self.kind {
            Kind::Line { a, b, control } => {
                *a = f(*a);
                *b = f(*b);
                if let Some(c) = control {
                    *c = f(*c);
                }
            }
            Kind::Pen { points } | Kind::Polygon { points } => points.iter_mut().for_each(|q| *q = f(*q)),
            Kind::Magnify { rect, offset, .. } => {
                let view = f(rect.center().add(*offset));
                *rect = Box2::from_points(f(rect.min()), f(rect.max()));
                *offset = view.sub(rect.center());
            }
            Kind::Group { objects } => objects.iter_mut().for_each(|o| o.map_points(f, scale)),
            Kind::Step { center, radius, .. } => {
                *radius *= scale;
                *center = f(*center);
            }
            Kind::Callout { rect, tail, more_tails, .. } => {
                *rect = Box2::from_points(f(rect.min()), f(rect.max()));
                *tail = f(*tail);
                more_tails.iter_mut().for_each(|t| *t = f(*t));
            }
            _ => {
                if let Some(r) = self.rect_mut() {
                    *r = Box2::from_points(f(r.min()), f(r.max()));
                }
            }
        }
    }

    /// The object's outline, used for shadows of objects drawn without a
    /// layer.
    fn silhouette(&self, t: Transform) -> Option<Path> {
        let r = match &self.kind {
            Kind::Magnify { rect, offset, .. } => rect.offset(offset.x, offset.y),
            _ => self.rect()?,
        };
        let ellipse = matches!(self.kind, Kind::Magnify { ellipse: true, .. });
        let path = if ellipse { PathBuilder::from_oval(r.to_skia()?)? } else { PathBuilder::from_rect(r.to_skia()?) };
        path.transform(t)
    }

    /// Draws the object onto `target`, whose top-left pixel is at document
    /// position `origin`. Shadow and opacity are applied here.
    pub fn draw(&self, target: &mut Pixmap, origin: (i32, i32)) {
        if self.rotated() {
            return self.draw_rotated(target, origin);
        }
        let reads = self.reads().is_some() || matches!(self.kind, Kind::Spotlight { .. });
        let layered = !reads && (self.style.shadow || self.style.opacity < 0.999);
        if !layered {
            if self.style.shadow && reads {
                // Sample what the lens shows before its own shadow lands
                // on it.
                let source = match &self.kind {
                    Kind::Magnify { rect, zoom, .. } => magnify_source(target, origin, rect, *zoom),
                    _ => None,
                };
                let t = Transform::from_translate(-origin.0 as f32, -origin.1 as f32);
                if let Some(path) = self.silhouette(t) {
                    shadow_from_path(target, &path, &self.style);
                }
                if let (Kind::Magnify { rect, zoom, ellipse, offset }, Some(src)) = (&self.kind, source) {
                    draw_magnify_from(target, origin, rect, *offset, *zoom, *ellipse, &self.style, src);
                    return;
                }
            }
            self.draw_plain(target, origin);
            return;
        }
        let area = self.bounds().to_irect().intersect(&IRect::new(origin.0, origin.1, target.width() as i32, target.height() as i32));
        let Some(area) = area else { return };
        let Some(mut layer) = Pixmap::new(area.w as u32, area.h as u32) else { return };
        // Multiply blending needs the pixels below; blend the finished
        // layer instead.
        let mut flat = self.clone();
        flat.style.multiply = false;
        flat.draw_plain(&mut layer, (area.x, area.y));
        let (x, y) = (area.x - origin.0, area.y - origin.1);
        if self.style.shadow {
            let shadow = shadow_of(&layer, &self.style);
            let (sx, sy) = shadow_offset(&self.style);
            target.draw_pixmap(
                x + sx,
                y + sy,
                shadow.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
        let paint = tiny_skia::PixmapPaint {
            opacity: self.style.opacity.clamp(0.0, 1.0),
            blend_mode: if self.style.multiply { tiny_skia::BlendMode::Multiply } else { tiny_skia::BlendMode::SourceOver },
            quality: FilterQuality::Nearest,
        };
        target.draw_pixmap(x, y, layer.as_ref(), &paint, Transform::identity(), None);
    }

    /// Draws the object upright into a layer, then turns the layer onto
    /// the target.
    fn draw_rotated(&self, target: &mut Pixmap, origin: (i32, i32)) {
        let mut upright = self.clone();
        upright.rotation = 0.0;
        let area = upright.bounds().to_irect();
        let Some(mut layer) = Pixmap::new(area.w.max(1) as u32, area.h.max(1) as u32) else { return };
        upright.style.multiply = false;
        upright.draw(&mut layer, (area.x, area.y));
        let c = self.pivot();
        let (cx, cy) = (c.x - origin.0 as f32, c.y - origin.1 as f32);
        let t = Transform::from_translate((area.x - origin.0) as f32, (area.y - origin.1) as f32)
            .post_concat(Transform::from_rotate_at(self.rotation, cx, cy));
        let paint = tiny_skia::PixmapPaint {
            quality: FilterQuality::Bilinear,
            blend_mode: if self.style.multiply { tiny_skia::BlendMode::Multiply } else { tiny_skia::BlendMode::SourceOver },
            ..Default::default()
        };
        target.draw_pixmap(0, 0, layer.as_ref(), &paint, t, None);
    }

    fn draw_plain(&self, pm: &mut Pixmap, origin: (i32, i32)) {
        let t = Transform::from_translate(-origin.0 as f32, -origin.1 as f32);
        let s = &self.style;
        match &self.kind {
            Kind::Group { objects } => {
                for o in objects {
                    o.draw(pm, origin);
                }
            }
            Kind::Line { a, b, control } => draw_line(pm, t, *a, *b, *control, s),
            Kind::Shape { rect, shape } => {
                if let Some(path) = shape_path(rect, *shape) {
                    fill_styled(pm, &path, s, rect, t, s.multiply);
                    stroke(pm, &path, s.stroke, s.width, s.dash, t);
                }
            }
            Kind::Polygon { points } => {
                if let Some(path) = polygon_path(points) {
                    if let Some(b) = Box2::bounding(points.iter().copied()) {
                        fill_styled(pm, &path, s, &b, t, s.multiply);
                    }
                    stroke(pm, &path, s.stroke, s.width, s.dash, t);
                }
            }
            Kind::Text { rect, text } => {
                let r = *rect;
                if let Some(path) = shape_path(&r, ShapeKind::Rectangle) {
                    fill_styled(pm, &path, s, &r, t, false);
                    stroke(pm, &path, s.stroke, s.width, s.dash, t);
                }
                let pad = text_padding(s);
                let layout = text::layout(text, &s.text_style(), Some(r.width() - 2.0 * pad));
                let y = match s.valign {
                    VAlign::Top => r.y0 + pad,
                    VAlign::Middle => r.center().y - layout.height() / 2.0,
                    VAlign::Bottom => r.y1 - pad - layout.height(),
                };
                draw_text(pm, origin, &layout, r.x0 + pad, y, r.width() - 2.0 * pad, s);
            }
            Kind::Callout { rect, tail, text, shape, arrow, more_tails } => {
                let tails: Vec<Pt> = std::iter::once(*tail).chain(more_tails.iter().copied()).collect();
                draw_callout(pm, origin, t, rect, &tails, text, *shape, *arrow, s)
            }
            Kind::Step { center, radius, label, shape } => {
                let r = Box2::around(*center, *radius, *radius);
                if let Some(path) = shape_path(&r, *shape) {
                    fill_styled(pm, &path, s, &r, t, false);
                    stroke(pm, &path, s.stroke, s.width, Dash::Solid, t);
                }
                let style = TextStyle { family: &s.font, size: radius * 1.15, bold: true, italic: false };
                let layout = text::layout(label, &style, None);
                text::draw(
                    pm,
                    origin,
                    &layout,
                    r.x0,
                    center.y - layout.height() / 2.0,
                    r.width(),
                    Align::Center,
                    s.text_color,
                );
            }
            Kind::Pen { points } => draw_pen(pm, t, points, s),
            Kind::Highlight { rect, shape } => {
                if let Some(path) = shape_path(rect, *shape) {
                    fill(pm, &path, s.fill, t, true);
                }
            }
            Kind::Blur { rect, mode, amount } => {
                let area = rect.to_irect();
                let area = IRect::new(area.x - origin.0, area.y - origin.1, area.w, area.h);
                match mode {
                    BlurMode::Blur => raster::blur_area(pm, area, amount.max(1.0) as u32),
                    BlurMode::Pixelate => raster::pixelate_area(pm, area, amount.max(2.0) as u32),
                    BlurMode::Redact => {
                        if let Some(path) = shape_path(rect, ShapeKind::Rectangle) {
                            fill(pm, &path, s.fill.with_alpha(255), t, false);
                        }
                    }
                }
            }
            Kind::Magnify { rect, zoom, ellipse, offset } => draw_magnify(pm, origin, rect, *offset, *zoom, *ellipse, s),
            Kind::Spotlight { rect, ellipse, dim, blur, rounded } => {
                let shape = if *ellipse { ShapeKind::Ellipse } else if *rounded { ShapeKind::Rounded } else { ShapeKind::Rectangle };
                draw_spotlight(pm, origin, rect, shape, *dim, *blur)
            }
            Kind::Stamp { rect, stamp } => stamps::draw(*stamp, rect, pm, t),
            Kind::Image { rect, image } if self.zone.is_some() => {
                let zone = self.zone.unwrap();
                draw_zone(pm, t, origin, rect, image.pixmap.as_deref(), &zone, s);
            }
            Kind::Image { rect, image } => {
                if let Some(src) = &image.pixmap {
                    draw_image(pm, t, rect, src);
                }
                if s.width > 0.0 {
                    if let Some(path) = shape_path(rect, ShapeKind::Rectangle) {
                        stroke(pm, &path, s.stroke, s.width, s.dash, t);
                    }
                }
            }
        }
    }
}

fn box_handles(r: &Box2) -> Vec<(Handle, Pt)> {
    let c = r.center();
    vec![
        (H_TL, Pt::new(r.x0, r.y0)),
        (H_T, Pt::new(c.x, r.y0)),
        (H_TR, Pt::new(r.x1, r.y0)),
        (H_R, Pt::new(r.x1, c.y)),
        (H_BR, Pt::new(r.x1, r.y1)),
        (H_B, Pt::new(c.x, r.y1)),
        (H_BL, Pt::new(r.x0, r.y1)),
        (H_L, Pt::new(r.x0, c.y)),
    ]
}

pub fn point_in_polygon(p: Pt, points: &[Pt]) -> bool {
    let n = points.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

fn polygon_path(points: &[Pt]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(points.first()?.x, points[0].y);
    for q in &points[1..] {
        pb.line_to(q.x, q.y);
    }
    pb.close();
    pb.finish()
}

pub fn shadow_margin(s: &Style) -> f32 {
    s.shadow_distance.abs() + s.shadow_blur.max(0.0) * 3.0 + 2.0
}

fn shadow_offset(s: &Style) -> (i32, i32) {
    let (sin, cos) = s.shadow_angle.to_radians().sin_cos();
    ((cos * s.shadow_distance).round() as i32, (sin * s.shadow_distance).round() as i32)
}

/// Fills `path` with the style's fill: a solid color, or a linear or
/// radial gradient to `fill2` across `area`.
pub fn fill_styled(pm: &mut Pixmap, path: &Path, s: &Style, area: &Box2, t: Transform, multiply: bool) {
    if !s.fill2.is_visible() || !s.fill.is_visible() {
        return fill(pm, path, s.fill, t, multiply);
    }
    let stops = vec![tiny_skia::GradientStop::new(0.0, s.fill.to_color()), tiny_skia::GradientStop::new(1.0, s.fill2.to_color())];
    let c = area.center();
    let shader = if s.gradient_radial {
        let r = area.width().max(area.height()) / 2.0;
        tiny_skia::RadialGradient::new(tiny_skia::Point::from_xy(c.x, c.y), tiny_skia::Point::from_xy(c.x, c.y), r.max(1.0), stops, SpreadMode::Pad, Transform::identity())
    } else {
        let (sin, cos) = s.gradient_angle.to_radians().sin_cos();
        let half = (area.width() * cos.abs() + area.height() * sin.abs()) / 2.0;
        let d = Pt::new(cos * half, sin * half);
        tiny_skia::LinearGradient::new(
            tiny_skia::Point::from_xy(c.x - d.x, c.y - d.y),
            tiny_skia::Point::from_xy(c.x + d.x, c.y + d.y),
            stops,
            SpreadMode::Pad,
            Transform::identity(),
        )
    };
    let Some(shader) = shader else { return fill(pm, path, s.fill, t, multiply) };
    let mut paint = Paint::default();
    paint.anti_alias = true;
    paint.shader = shader;
    if multiply {
        paint.blend_mode = tiny_skia::BlendMode::Multiply;
    }
    pm.fill_path(path, &paint, FillRule::Winding, t, None);
}

/// Draws laid-out text with the style's letter outline, underline and
/// strikethrough.
fn draw_text(pm: &mut Pixmap, origin: (i32, i32), layout: &text::Layout, x: f32, y: f32, w: f32, s: &Style) {
    if s.outline.is_visible() && s.outline_width > 0.0 {
        let r = s.outline_width;
        let steps = 12;
        for i in 0..steps {
            let a = i as f32 / steps as f32 * std::f32::consts::TAU;
            text::draw(pm, origin, layout, x + a.cos() * r, y + a.sin() * r, w, s.align, s.outline);
        }
    }
    text::draw(pm, origin, layout, x, y, w, s.align, s.text_color);
    if s.underline || s.strike {
        let thick = (s.font_size / 14.0).max(1.0);
        for (lx, lw, baseline) in text::line_extents(layout, x, y, w, s.align) {
            for (on, dy) in [(s.underline, thick * 1.5), (s.strike, -s.font_size * 0.28)] {
                if on && lw > 0.0 {
                    if let Some(r) = tiny_skia::Rect::from_xywh(lx - origin.0 as f32, baseline + dy - origin.1 as f32, lw, thick) {
                        pm.fill_rect(r, &paint_of(s.text_color, false), Transform::identity(), None);
                    }
                }
            }
        }
    }
}

/// Resizes `r` by dragging handle `h` to `p`.
pub fn resize_box(r: Box2, h: Handle, p: Pt, square: bool) -> Box2 {
    let (mut x0, mut y0, mut x1, mut y1) = (r.x0, r.y0, r.x1, r.y1);
    match h {
        H_TL => (x0, y0) = (p.x, p.y),
        H_T => y0 = p.y,
        H_TR => (x1, y0) = (p.x, p.y),
        H_R => x1 = p.x,
        H_BR => (x1, y1) = (p.x, p.y),
        H_B => y1 = p.y,
        H_BL => (x0, y1) = (p.x, p.y),
        H_L => x0 = p.x,
        _ => {}
    }
    if square && matches!(h, H_TL | H_TR | H_BR | H_BL) {
        let side = (x1 - x0).abs().max((y1 - y0).abs());
        match h {
            H_TL => (x0, y0) = (x1 - side * (x1 - x0).signum(), y1 - side * (y1 - y0).signum()),
            H_TR => (x1, y0) = (x0 + side * (x1 - x0).signum(), y1 - side * (y1 - y0).signum()),
            H_BR => (x1, y1) = (x0 + side * (x1 - x0).signum(), y0 + side * (y1 - y0).signum()),
            _ => (x0, y1) = (x1 - side * (x1 - x0).signum(), y0 + side * (y1 - y0).signum()),
        }
    }
    Box2::new(x0, y0, x1, y1)
}

/// `p` moved onto the nearest 45° ray from `from`.
pub fn snap45(from: Pt, p: Pt) -> Pt {
    let d = p.sub(from);
    let len = d.len();
    let angle = d.y.atan2(d.x);
    let step = std::f32::consts::FRAC_PI_4;
    let snapped = (angle / step).round() * step;
    from.add(Pt::new(snapped.cos() * len, snapped.sin() * len))
}

pub fn shape_path(r: &Box2, shape: ShapeKind) -> Option<Path> {
    let rect = r.to_skia()?;
    match shape {
        ShapeKind::Rectangle => Some(PathBuilder::from_rect(rect)),
        ShapeKind::Ellipse => PathBuilder::from_oval(rect),
        ShapeKind::Rounded => rounded_rect(r, (r.width().min(r.height()) * 0.18).min(40.0)),
        ShapeKind::Triangle => {
            let mut pb = PathBuilder::new();
            pb.move_to(r.center().x, r.y0);
            pb.line_to(r.x1, r.y1);
            pb.line_to(r.x0, r.y1);
            pb.close();
            pb.finish()
        }
    }
}

pub fn rounded_rect(r: &Box2, radius: f32) -> Option<Path> {
    let rad = radius.min(r.width() / 2.0).min(r.height() / 2.0).max(0.0);
    if rad < 0.5 {
        return r.to_skia().map(PathBuilder::from_rect);
    }
    let k = rad * 0.552_284_8;
    let mut pb = PathBuilder::new();
    pb.move_to(r.x0 + rad, r.y0);
    pb.line_to(r.x1 - rad, r.y0);
    pb.cubic_to(r.x1 - rad + k, r.y0, r.x1, r.y0 + rad - k, r.x1, r.y0 + rad);
    pb.line_to(r.x1, r.y1 - rad);
    pb.cubic_to(r.x1, r.y1 - rad + k, r.x1 - rad + k, r.y1, r.x1 - rad, r.y1);
    pb.line_to(r.x0 + rad, r.y1);
    pb.cubic_to(r.x0 + rad - k, r.y1, r.x0, r.y1 - rad + k, r.x0, r.y1 - rad);
    pb.line_to(r.x0, r.y0 + rad);
    pb.cubic_to(r.x0, r.y0 + rad - k, r.x0 + rad - k, r.y0, r.x0 + rad, r.y0);
    pb.close();
    pb.finish()
}

fn paint_of(color: Rgba, multiply: bool) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.anti_alias = true;
    paint.set_color(color.to_color());
    if multiply {
        paint.blend_mode = tiny_skia::BlendMode::Multiply;
    }
    paint
}

pub fn fill(pm: &mut Pixmap, path: &Path, color: Rgba, t: Transform, multiply: bool) {
    if color.is_visible() {
        pm.fill_path(path, &paint_of(color, multiply), FillRule::Winding, t, None);
    }
}

pub fn stroke(pm: &mut Pixmap, path: &Path, color: Rgba, width: f32, dash: Dash, t: Transform) {
    if !color.is_visible() || width <= 0.0 {
        return;
    }
    let mut s = Stroke { width, line_join: LineJoin::Round, line_cap: LineCap::Round, ..Stroke::default() };
    s.dash = match dash {
        Dash::Solid => None,
        Dash::Dash => StrokeDash::new(vec![width * 3.0, width * 2.0], 0.0),
        Dash::Dot => StrokeDash::new(vec![0.01, width * 2.0], 0.0),
    };
    pm.stroke_path(path, &paint_of(color, false), &s, t, None);
}

fn draw_line(pm: &mut Pixmap, t: Transform, a: Pt, b: Pt, control: Option<Pt>, s: &Style) {
    let w = s.width.max(0.5);
    let head = arrow_head(w) * s.head_size.clamp(0.3, 4.0);
    let len = a.dist(b);
    let end_dir = b.sub(control.unwrap_or(a)).normalized();
    let start_dir = a.sub(control.unwrap_or(b)).normalized();
    let fits = len > head * 1.2;
    let (mut a2, mut b2) = (a, b);
    if s.end_cap == Cap::Arrow && fits {
        b2 = b.sub(end_dir.scale(head * 0.8));
    }
    if s.start_cap == Cap::Arrow && fits {
        a2 = a.sub(start_dir.scale(head * 0.8));
    }
    if s.start_cap == Cap::Taper || s.end_cap == Cap::Taper {
        // A filled outline that narrows towards the tapered end(s).
        let samples: Vec<Pt> = (0..=24).map(|i| {
            let t = i as f32 / 24.0;
            match control {
                Some(c) => quad_at(a2, c, b2, t),
                None => a2.lerp(b2, t),
            }
        }).collect();
        let width_at = |t: f32| {
            let from = if s.start_cap == Cap::Taper { 0.15 } else { 1.0 };
            let to = if s.end_cap == Cap::Taper { 0.15 } else { 1.0 };
            w * (from + (to - from) * t) / 2.0
        };
        let mut left = Vec::new();
        let mut right = Vec::new();
        for (i, q) in samples.iter().enumerate() {
            let next = samples[(i + 1).min(samples.len() - 1)];
            let prev = samples[i.saturating_sub(1)];
            let d = next.sub(prev).normalized();
            let n = Pt::new(-d.y, d.x).scale(width_at(i as f32 / 24.0));
            left.push(q.add(n));
            right.push(q.sub(n));
        }
        right.reverse();
        left.extend(right);
        if let Some(path) = polygon_path(&left) {
            fill(pm, &path, s.stroke, t, false);
        }
    } else {
        let mut pb = PathBuilder::new();
        pb.move_to(a2.x, a2.y);
        match control {
            Some(c) => pb.quad_to(c.x, c.y, b2.x, b2.y),
            None => pb.line_to(b2.x, b2.y),
        }
        if len < 0.5 {
            pb.line_to(a.x + 0.01, a.y);
        }
        if let Some(path) = pb.finish() {
            stroke(pm, &path, s.stroke, w, s.dash, t);
        }
    }
    for (cap, tip, dir) in [(s.end_cap, b, end_dir), (s.start_cap, a, start_dir)] {
        let perp = Pt::new(-dir.y, dir.x);
        match cap {
            Cap::OpenArrow if fits => {
                let base = tip.sub(dir.scale(head));
                let mut pb = PathBuilder::new();
                let (l, r) = (base.add(perp.scale(head * 0.55)), base.sub(perp.scale(head * 0.55)));
                pb.move_to(l.x, l.y);
                pb.line_to(tip.x, tip.y);
                pb.line_to(r.x, r.y);
                if let Some(path) = pb.finish() {
                    stroke(pm, &path, s.stroke, w, Dash::Solid, t);
                }
            }
            Cap::Square | Cap::Diamond => {
                let r = w * 1.4;
                let pts: Vec<Pt> = if cap == Cap::Square {
                    [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)].iter().map(|(u, v)| tip.add(dir.scale(u * r)).add(perp.scale(v * r))).collect()
                } else {
                    [(1.4, 0.0), (0.0, 1.0), (-1.4, 0.0), (0.0, -1.0)].iter().map(|(u, v)| tip.add(dir.scale(u * r)).add(perp.scale(v * r))).collect()
                };
                if let Some(path) = polygon_path(&pts) {
                    fill(pm, &path, s.stroke, t, false);
                }
            }
            Cap::Arrow if fits => {
                let base = tip.sub(dir.scale(head));
                let perp = Pt::new(-dir.y, dir.x).scale(head * 0.55);
                let mut pb = PathBuilder::new();
                pb.move_to(tip.x, tip.y);
                pb.line_to(base.x + perp.x, base.y + perp.y);
                pb.line_to(base.x - perp.x, base.y - perp.y);
                pb.close();
                if let Some(path) = pb.finish() {
                    fill(pm, &path, s.stroke, t, false);
                }
            }
            Cap::Dot => {
                if let Some(path) = PathBuilder::from_circle(tip.x, tip.y, w * 1.3) {
                    fill(pm, &path, s.stroke, t, false);
                }
            }
            _ => {}
        }
    }
}

fn draw_pen(pm: &mut Pixmap, t: Transform, points: &[Pt], s: &Style) {
    let Some(first) = points.first() else { return };
    let mut pb = PathBuilder::new();
    pb.move_to(first.x, first.y);
    if points.len() == 1 {
        pb.line_to(first.x + 0.01, first.y);
    } else {
        // Quadratic smoothing through segment midpoints.
        for i in 1..points.len() - 1 {
            let (p, q) = (points[i], points[i + 1]);
            let mid = p.lerp(q, 0.5);
            pb.quad_to(p.x, p.y, mid.x, mid.y);
        }
        let last = points[points.len() - 1];
        pb.line_to(last.x, last.y);
    }
    let Some(path) = pb.finish() else { return };
    let mut st = Stroke { width: s.width.max(0.5), line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
    if s.multiply {
        st.line_cap = LineCap::Square;
    }
    pm.stroke_path(&path, &paint_of(s.stroke, s.multiply), &st, t, None);
}

#[allow(clippy::too_many_arguments)]
fn draw_callout(pm: &mut Pixmap, origin: (i32, i32), t: Transform, rect: &Box2, tails: &[Pt], text: &str, shape: ShapeKind, arrow: bool, s: &Style) {
    let body = shape_path(rect, shape);
    for &tail in tails.iter().filter(|t| arrow && !rect.contains(**t)) {
        // An arrow from the edge of the body to the tail point.
        let c = rect.center();
        let d = tail.sub(c);
        let k = (rect.width() / 2.0 / d.x.abs().max(1e-3)).min(rect.height() / 2.0 / d.y.abs().max(1e-3)).min(1.0);
        let start = c.add(d.scale(k));
        let color = if s.fill.is_visible() { s.fill } else { s.stroke };
        let line = Style { width: (s.font_size * 0.18).max(3.0), end_cap: Cap::Arrow, start_cap: Cap::None, stroke: color, dash: Dash::Solid, ..s.clone() };
        draw_line(pm, t, start, tail, None, &line);
    }
    let wedges: Vec<Path> = tails
        .iter()
        .filter(|t| !arrow && !rect.contains(**t))
        .filter_map(|&tail| {
            let c = rect.center();
            let dir = tail.sub(c).normalized();
            let perp = Pt::new(-dir.y, dir.x).scale(rect.width().min(rect.height()) * 0.18);
            let mut pb = PathBuilder::new();
            pb.move_to(c.x + perp.x, c.y + perp.y);
            pb.line_to(tail.x, tail.y);
            pb.line_to(c.x - perp.x, c.y - perp.y);
            pb.close();
            pb.finish()
        })
        .collect();
    let parts: Vec<&Path> = body.iter().chain(wedges.iter()).collect();
    if s.fill.is_visible() {
        // A doubled stroke under the fill leaves exactly `width` outside the
        // union and hides the seam between body and tail.
        for path in &parts {
            stroke(pm, path, s.stroke, s.width * 2.0, Dash::Solid, t);
        }
        for path in &parts {
            fill_styled(pm, path, s, rect, t, false);
        }
    } else {
        for path in &parts {
            stroke(pm, path, s.stroke, s.width, s.dash, t);
        }
    }
    let pad = text_padding(s) + if shape == ShapeKind::Ellipse { rect.width() * 0.12 } else { 0.0 };
    let inner = rect.width() - 2.0 * pad;
    let layout = text::layout(text, &s.text_style(), Some(inner));
    let y = match s.valign {
        VAlign::Top => rect.y0 + pad,
        VAlign::Bottom => rect.y1 - pad - layout.height(),
        VAlign::Middle => rect.center().y - layout.height() / 2.0,
    };
    draw_text(pm, origin, &layout, rect.x0 + pad, y, inner, s);
}

/// The pixels a lens shows, with the document position of their top-left.
fn magnify_source(pm: &Pixmap, origin: (i32, i32), rect: &Box2, zoom: f32) -> Option<(Pixmap, IRect)> {
    let zoom = zoom.max(1.0);
    let source = Box2::around(rect.center(), rect.width() / (2.0 * zoom), rect.height() / (2.0 * zoom));
    let e = source.to_irect().inflate(1);
    raster::extract(pm, IRect::new(e.x - origin.0, e.y - origin.1, e.w, e.h)).map(|p| (p, e))
}

fn draw_magnify(pm: &mut Pixmap, origin: (i32, i32), rect: &Box2, offset: Pt, zoom: f32, ellipse: bool, s: &Style) {
    if let Some(src) = magnify_source(pm, origin, rect, zoom) {
        draw_magnify_from(pm, origin, rect, offset, zoom, ellipse, s, src);
    }
}

/// Draws the lens: the area around `rect`'s center enlarged into `rect`,
/// or, with an `offset`, into a view moved by it, with a thin outline of
/// the source area and a connector.
#[allow(clippy::too_many_arguments)]
fn draw_magnify_from(pm: &mut Pixmap, origin: (i32, i32), source_rect: &Box2, offset: Pt, zoom: f32, ellipse: bool, s: &Style, (src, e): (Pixmap, IRect)) {
    let zoom = zoom.max(1.0);
    let moved = offset.len() > 0.5;
    let rect = &source_rect.offset(offset.x, offset.y);
    if moved {
        let t = Transform::from_translate(-origin.0 as f32, -origin.1 as f32);
        let source = Box2::around(source_rect.center(), source_rect.width() / (2.0 * zoom), source_rect.height() / (2.0 * zoom));
        let outline = if ellipse { source.to_skia().and_then(PathBuilder::from_oval) } else { shape_path(&source, ShapeKind::Rectangle) };
        if let Some(path) = outline {
            stroke(pm, &path, s.stroke, (s.width / 2.0).max(1.5), Dash::Solid, t);
        }
        let (a, b) = (source.center(), rect.center());
        let d = b.sub(a).normalized();
        let from = a.add(d.scale(source.width().min(source.height()) / 2.0));
        let to = b.sub(d.scale(rect.width().min(rect.height()) / 2.0));
        let mut pb = PathBuilder::new();
        pb.move_to(from.x, from.y);
        pb.line_to(to.x, to.y);
        if let Some(path) = pb.finish() {
            stroke(pm, &path, s.stroke, (s.width / 2.0).max(1.5), Dash::Solid, t);
        }
    }
    let c = source_rect.center();
    let view = rect.center();
    let tx = view.x + (e.x as f32 - c.x) * zoom - origin.0 as f32;
    let ty = view.y + (e.y as f32 - c.y) * zoom - origin.1 as f32;
    let shader_t = Transform::from_row(zoom, 0.0, 0.0, zoom, tx, ty);
    let lens = rect.offset(-origin.0 as f32, -origin.1 as f32);
    let path = if ellipse { lens.to_skia().and_then(PathBuilder::from_oval) } else { shape_path(&lens, ShapeKind::Rectangle) };
    let Some(path) = path else { return };
    let mut paint = Paint::default();
    paint.anti_alias = true;
    paint.shader = Pattern::new(src.as_ref(), SpreadMode::Pad, FilterQuality::Bicubic, 1.0, shader_t);
    pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    stroke(pm, &path, s.stroke, s.width, s.dash, Transform::identity());
}

fn draw_spotlight(pm: &mut Pixmap, origin: (i32, i32), rect: &Box2, shape: ShapeKind, dim: f32, blur: f32) {
    let spot = rect.offset(-origin.0 as f32, -origin.1 as f32);
    let Some(spot_path) = shape_path(&spot, shape) else {
        return;
    };
    if blur > 0.0 {
        let original = pm.clone();
        raster::blur_area(pm, IRect::of_size(pm.width(), pm.height()), blur as u32);
        let mut paint = Paint::default();
        paint.anti_alias = true;
        paint.shader = Pattern::new(original.as_ref(), SpreadMode::Pad, FilterQuality::Nearest, 1.0, Transform::identity());
        pm.fill_path(&spot_path, &paint, FillRule::Winding, Transform::identity(), None);
    }
    let mut pb = PathBuilder::new();
    pb.push_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, pm.width() as f32, pm.height() as f32).unwrap());
    pb.push_path(&spot_path);
    if let Some(path) = pb.finish() {
        let color = Rgba::BLACK.with_alpha((dim.clamp(0.0, 1.0) * 255.0) as u8);
        pm.fill_path(&path, &paint_of(color, false), FillRule::EvenOdd, Transform::identity(), None);
    }
}

pub fn draw_image(pm: &mut Pixmap, t: Transform, rect: &Box2, src: &Pixmap) {
    let (sx, sy) = (rect.width() / src.width() as f32, rect.height() / src.height() as f32);
    let exact = (sx - 1.0).abs() < 1e-3 && (sy - 1.0).abs() < 1e-3;
    let quality = if exact { FilterQuality::Nearest } else { FilterQuality::Bicubic };
    let shader_t = Transform::from_row(sx, 0.0, 0.0, sy, rect.x0, rect.y0);
    let mut paint = Paint::default();
    paint.shader = Pattern::new(src.as_ref(), SpreadMode::Pad, quality, 1.0, shader_t);
    if let Some(path) = shape_path(rect, ShapeKind::Rectangle) {
        pm.fill_path(&path, &paint, FillRule::Winding, t, None);
    }
}

/// A drop zone: its image filling or fitting it, clipped to it; or, when
/// empty, a dashed placeholder saying what to do.
fn draw_zone(pm: &mut Pixmap, t: Transform, origin: (i32, i32), rect: &Box2, src: Option<&Pixmap>, zone: &Zone, s: &Style) {
    let Some(area) = shape_path(rect, ShapeKind::Rectangle) else { return };
    match src {
        Some(src) => {
            if s.fill.is_visible() {
                fill(pm, &area, s.fill, t, false);
            }
            let (placed, _) = zone.placement(rect, src.width() as f32, src.height() as f32);
            let (sx, sy) = (placed.width() / src.width() as f32, placed.height() / src.height() as f32);
            let mut paint = Paint::default();
            paint.shader = Pattern::new(src.as_ref(), SpreadMode::Pad, FilterQuality::Bicubic, 1.0, Transform::from_row(sx, 0.0, 0.0, sy, placed.x0, placed.y0));
            let shown = Box2::new(placed.x0.max(rect.x0), placed.y0.max(rect.y0), placed.x1.min(rect.x1), placed.y1.min(rect.y1));
            if let Some(path) = shape_path(&shown, ShapeKind::Rectangle) {
                pm.fill_path(&path, &paint, FillRule::Winding, t, None);
            }
        }
        None => {
            fill(pm, &area, Rgba::rgb(0xE9, 0xEC, 0xF1).with_alpha(230), t, false);
            stroke(pm, &area, Rgba::rgb(0x8E, 0x97, 0xA3), 2.0, Dash::Dash, t);
            let size = (rect.height() / 9.0).clamp(12.0, 28.0);
            let ts = text::TextStyle { family: text::DEFAULT_FAMILY, size, bold: false, italic: false };
            let layout = text::layout("Drop an image here", &ts, Some(rect.width() - 16.0));
            let label = Style { text_color: Rgba::rgb(0x5B, 0x63, 0x6E), font_size: size, align: text::Align::Center, ..Style::default() };
            let y = rect.center().y - layout.height() / 2.0;
            draw_text(pm, origin, &layout, rect.x0 + 8.0, y, rect.width() - 16.0, &label);
        }
    }
}

/// A blurred copy of `layer`'s alpha in the shadow color.
fn shadow_of(layer: &Pixmap, s: &Style) -> Pixmap {
    let mut shadow = layer.clone();
    let c = s.shadow_color;
    for px in shadow.data_mut().chunks_exact_mut(4) {
        let a = px[3] as f32 / 255.0 * c.a as f32 / 255.0;
        px.copy_from_slice(&[(c.r as f32 * a) as u8, (c.g as f32 * a) as u8, (c.b as f32 * a) as u8, (a * 255.0) as u8]);
    }
    raster::blur_area(&mut shadow, IRect::of_size(layer.width(), layer.height()), s.shadow_blur.max(0.0) as u32);
    shadow
}

fn shadow_from_path(pm: &mut Pixmap, path: &Path, s: &Style) {
    let b = path.bounds();
    let area = Box2::new(b.left(), b.top(), b.right(), b.bottom()).inflate(shadow_margin(s)).to_irect();
    let Some(mut layer) = Pixmap::new(area.w.max(1) as u32, area.h.max(1) as u32) else { return };
    let t = Transform::from_translate(-area.x as f32, -area.y as f32);
    layer.fill_path(path, &paint_of(Rgba::BLACK, false), FillRule::Winding, t, None);
    let shadow = shadow_of(&layer, s);
    let (sx, sy) = shadow_offset(s);
    pm.draw_pixmap(
        area.x + sx,
        area.y + sy,
        shadow.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arrow() -> Object {
        let style = Style { end_cap: Cap::Arrow, ..Style::default() };
        Object::new(1, Kind::Line { a: Pt::new(10.0, 10.0), b: Pt::new(90.0, 10.0), control: None }, style)
    }

    #[test]
    fn hit_and_handles() {
        let a = arrow();
        assert!(a.hit(Pt::new(50.0, 12.0), 2.0));
        assert!(!a.hit(Pt::new(50.0, 30.0), 2.0));
        assert_eq!(a.kind_name(), "Arrow");
        let mut a = a;
        a.move_handle(H_CONTROL, Pt::new(50.0, 40.0), false);
        let Kind::Line { control: Some(c), .. } = a.kind else { panic!() };
        assert!((quad_at(Pt::new(10.0, 10.0), c, Pt::new(90.0, 10.0), 0.5).y - 40.0).abs() < 1e-3);
        let rect = Object::new(2, Kind::Shape { rect: Box2::new(0.0, 0.0, 100.0, 50.0), shape: ShapeKind::Rectangle }, Style::default());
        assert!(rect.hit(Pt::new(1.0, 25.0), 2.0));
        assert!(!rect.hit(Pt::new(50.0, 25.0), 2.0), "unfilled shapes are hit on the border only");
        assert_eq!(resize_box(Box2::new(0.0, 0.0, 10.0, 10.0), H_BR, Pt::new(20.0, 5.0), true), Box2::new(0.0, 0.0, 20.0, 20.0));
    }

    #[test]
    fn draws_into_offset_targets() {
        let mut full = raster::solid(100, 30, Rgba::WHITE).unwrap();
        arrow().draw(&mut full, (0, 0));
        let mut part = raster::solid(40, 30, Rgba::WHITE).unwrap();
        arrow().draw(&mut part, (60, 0));
        for x in 0..40 {
            for y in 0..30 {
                assert_eq!(raster::pixel(&full, x + 60, y), raster::pixel(&part, x, y), "pixel {x},{y}");
            }
        }
        assert_eq!(raster::pixel(&full, 50, 10), Rgba::RED);
    }

    #[test]
    fn shadows_and_opacity_use_a_layer() {
        let mut style = Style { shadow: true, opacity: 0.5, fill: Rgba::RED, ..Style::default() };
        style.width = 0.0;
        let o = Object::new(1, Kind::Shape { rect: Box2::new(10.0, 10.0, 40.0, 40.0), shape: ShapeKind::Rectangle }, style);
        let mut pm = raster::solid(80, 80, Rgba::WHITE).unwrap();
        o.draw(&mut pm, (0, 0));
        let inside = raster::pixel(&pm, 20, 20);
        assert!(inside.g > 100 && inside.g < 160, "half-transparent red over white: {inside:?}");
        let shadow = raster::pixel(&pm, 42, 42);
        assert!(shadow.r < 250, "shadow below and right: {shadow:?}");
    }
}

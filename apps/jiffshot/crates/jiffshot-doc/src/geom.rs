//! Document-space geometry. Points and boxes are `f32` document pixels;
//! [`IRect`] is a whole-pixel area of a raster.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pt {
    pub x: f32,
    pub y: f32,
}

impl Pt {
    pub const fn new(x: f32, y: f32) -> Pt {
        Pt { x, y }
    }

    pub fn offset(self, dx: f32, dy: f32) -> Pt {
        Pt::new(self.x + dx, self.y + dy)
    }

    pub fn sub(self, o: Pt) -> Pt {
        Pt::new(self.x - o.x, self.y - o.y)
    }

    pub fn add(self, o: Pt) -> Pt {
        Pt::new(self.x + o.x, self.y + o.y)
    }

    pub fn scale(self, s: f32) -> Pt {
        Pt::new(self.x * s, self.y * s)
    }

    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn dist(self, o: Pt) -> f32 {
        self.sub(o).len()
    }

    pub fn lerp(self, o: Pt, t: f32) -> Pt {
        Pt::new(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }

    /// Unit vector, or zero for a zero-length vector.
    pub fn normalized(self) -> Pt {
        let l = self.len();
        if l < 1e-6 {
            Pt::default()
        } else {
            self.scale(1.0 / l)
        }
    }
}

/// Distance from `p` to the segment `a`–`b`.
pub fn dist_to_segment(p: Pt, a: Pt, b: Pt) -> f32 {
    let ab = b.sub(a);
    let len2 = ab.x * ab.x + ab.y * ab.y;
    if len2 < 1e-9 {
        return p.dist(a);
    }
    let t = (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / len2).clamp(0.0, 1.0);
    p.dist(a.lerp(b, t))
}

/// Point on the quadratic Bézier `a`, `c`, `b` at `t`.
pub fn quad_at(a: Pt, c: Pt, b: Pt, t: f32) -> Pt {
    a.lerp(c, t).lerp(c.lerp(b, t), t)
}

/// An axis-aligned box. Constructors normalize so `x0 <= x1`, `y0 <= y1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Box2 {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Box2 {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Box2 {
        Box2 { x0: x0.min(x1), y0: y0.min(y1), x1: x0.max(x1), y1: y0.max(y1) }
    }

    pub fn from_points(a: Pt, b: Pt) -> Box2 {
        Box2::new(a.x, a.y, b.x, b.y)
    }

    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Box2 {
        Box2::new(x, y, x + w, y + h)
    }

    pub fn around(c: Pt, rx: f32, ry: f32) -> Box2 {
        Box2::new(c.x - rx, c.y - ry, c.x + rx, c.y + ry)
    }

    /// Bounding box of a set of points; `None` when empty.
    pub fn bounding(points: impl IntoIterator<Item = Pt>) -> Option<Box2> {
        let mut it = points.into_iter();
        let first = it.next()?;
        let mut b = Box2 { x0: first.x, y0: first.y, x1: first.x, y1: first.y };
        for p in it {
            b.x0 = b.x0.min(p.x);
            b.y0 = b.y0.min(p.y);
            b.x1 = b.x1.max(p.x);
            b.y1 = b.y1.max(p.y);
        }
        Some(b)
    }

    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    pub fn center(&self) -> Pt {
        Pt::new((self.x0 + self.x1) / 2.0, (self.y0 + self.y1) / 2.0)
    }

    pub fn min(&self) -> Pt {
        Pt::new(self.x0, self.y0)
    }

    pub fn max(&self) -> Pt {
        Pt::new(self.x1, self.y1)
    }

    pub fn contains(&self, p: Pt) -> bool {
        p.x >= self.x0 && p.x <= self.x1 && p.y >= self.y0 && p.y <= self.y1
    }

    pub fn inflate(&self, d: f32) -> Box2 {
        Box2 { x0: self.x0 - d, y0: self.y0 - d, x1: self.x1 + d, y1: self.y1 + d }
    }

    pub fn offset(&self, dx: f32, dy: f32) -> Box2 {
        Box2 { x0: self.x0 + dx, y0: self.y0 + dy, x1: self.x1 + dx, y1: self.y1 + dy }
    }

    pub fn union(&self, o: &Box2) -> Box2 {
        Box2 { x0: self.x0.min(o.x0), y0: self.y0.min(o.y0), x1: self.x1.max(o.x1), y1: self.y1.max(o.y1) }
    }

    pub fn intersects(&self, o: &Box2) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }

    pub fn is_degenerate(&self, min: f32) -> bool {
        self.width() < min || self.height() < min
    }

    pub fn to_skia(&self) -> Option<tiny_skia::Rect> {
        tiny_skia::Rect::from_ltrb(self.x0, self.y0, self.x1.max(self.x0 + 0.01), self.y1.max(self.y0 + 0.01))
    }

    /// Smallest whole-pixel rectangle covering this box.
    pub fn to_irect(&self) -> IRect {
        let x = self.x0.floor() as i32;
        let y = self.y0.floor() as i32;
        IRect { x, y, w: self.x1.ceil() as i32 - x, h: self.y1.ceil() as i32 - y }
    }

    /// Maps `p` from this box onto `to`, proportionally.
    pub fn map(&self, p: Pt, to: &Box2) -> Pt {
        let sx = if self.width().abs() < 1e-6 { 1.0 } else { to.width() / self.width() };
        let sy = if self.height().abs() < 1e-6 { 1.0 } else { to.height() / self.height() };
        Pt::new(to.x0 + (p.x - self.x0) * sx, to.y0 + (p.y - self.y0) * sy)
    }
}

/// Whole-pixel rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> IRect {
        IRect { x, y, w, h }
    }

    pub fn of_size(w: u32, h: u32) -> IRect {
        IRect::new(0, 0, w as i32, h as i32)
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn union(&self, o: &IRect) -> IRect {
        if self.is_empty() {
            return *o;
        }
        if o.is_empty() {
            return *self;
        }
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        IRect::new(x, y, self.right().max(o.right()) - x, self.bottom().max(o.bottom()) - y)
    }

    pub fn intersect(&self, o: &IRect) -> Option<IRect> {
        let x = self.x.max(o.x);
        let y = self.y.max(o.y);
        let r = self.right().min(o.right());
        let b = self.bottom().min(o.bottom());
        (r > x && b > y).then(|| IRect::new(x, y, r - x, b - y))
    }

    pub fn contains_rect(&self, o: &IRect) -> bool {
        o.x >= self.x && o.y >= self.y && o.right() <= self.right() && o.bottom() <= self.bottom()
    }

    pub fn inflate(&self, d: i32) -> IRect {
        IRect::new(self.x - d, self.y - d, self.w + 2 * d, self.h + 2 * d)
    }

    pub fn to_box(&self) -> Box2 {
        Box2::from_xywh(self.x as f32, self.y as f32, self.w as f32, self.h as f32)
    }

    pub fn area(&self) -> i64 {
        if self.is_empty() {
            0
        } else {
            self.w as i64 * self.h as i64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_normalize_and_cover_pixels() {
        let b = Box2::new(10.5, 20.0, 2.2, 5.9);
        assert_eq!((b.x0, b.y0, b.x1, b.y1), (2.2, 5.9, 10.5, 20.0));
        assert_eq!(b.to_irect(), IRect::new(2, 5, 9, 15));
        assert_eq!(IRect::new(0, 0, 10, 10).intersect(&IRect::new(5, 5, 10, 10)), Some(IRect::new(5, 5, 5, 5)));
        assert_eq!(IRect::new(0, 0, 1, 1).union(&IRect::default()), IRect::new(0, 0, 1, 1));
        assert!((dist_to_segment(Pt::new(5.0, 3.0), Pt::new(0.0, 0.0), Pt::new(10.0, 0.0)) - 3.0).abs() < 1e-5);
    }
}

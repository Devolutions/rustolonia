use serde::{Deserialize, Serialize};

/// An integer rectangle; `width`/`height` of zero or less is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect { x, y, width, height }
    }

    pub const fn from_ltrb(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect { x: left, y: top, width: right - left, height: bottom - top }
    }

    /// The rectangle spanning two pixels, both included.
    pub fn from_points(a: (i32, i32), b: (i32, i32)) -> Rect {
        Rect::from_ltrb(a.0.min(b.0), a.1.min(b.1), a.0.max(b.0) + 1, a.1.max(b.1) + 1)
    }

    pub const fn right(self) -> i32 {
        self.x + self.width
    }

    pub const fn bottom(self) -> i32 {
        self.y + self.height
    }

    pub const fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub const fn area(self) -> i64 {
        if self.is_empty() {
            0
        } else {
            self.width as i64 * self.height as i64
        }
    }

    pub const fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }

    pub fn intersect(self, other: Rect) -> Option<Rect> {
        let r = Rect::from_ltrb(
            self.x.max(other.x),
            self.y.max(other.y),
            self.right().min(other.right()),
            self.bottom().min(other.bottom()),
        );
        (!r.is_empty()).then_some(r)
    }

    pub fn union(self, other: Rect) -> Rect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Rect::from_ltrb(
            self.x.min(other.x),
            self.y.min(other.y),
            self.right().max(other.right()),
            self.bottom().max(other.bottom()),
        )
    }

    pub const fn inflate(self, d: i32) -> Rect {
        Rect::new(self.x - d, self.y - d, self.width + 2 * d, self.height + 2 * d)
    }

    pub const fn offset(self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    pub const fn center(self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::Rect;

    #[test]
    fn from_points_includes_both_ends() {
        assert_eq!(Rect::from_points((10, 20), (5, 30)), Rect::new(5, 20, 6, 11));
        assert_eq!(Rect::from_points((3, 3), (3, 3)), Rect::new(3, 3, 1, 1));
    }

    #[test]
    fn intersect_and_union() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert_eq!(a.intersect(b), Some(Rect::new(5, 5, 5, 5)));
        assert_eq!(a.intersect(Rect::new(10, 0, 5, 5)), None);
        assert_eq!(a.union(b), Rect::new(0, 0, 15, 15));
        assert_eq!(Rect::default().union(b), b);
    }
}

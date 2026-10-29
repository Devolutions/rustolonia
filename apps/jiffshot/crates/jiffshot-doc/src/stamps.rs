//! Built-in vector stamps, drawn from code in a unit square so they stay
//! sharp at any size.

use crate::color::Rgba;
use crate::geom::{Box2, Pt};
use serde::{Deserialize, Serialize};
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stamp {
    #[default]
    Check,
    Cross,
    Star,
    Heart,
    Warning,
    Info,
    Question,
    Exclamation,
    Plus,
    Minus,
    Pointer,
    ThumbsUp,
}

pub const ALL: &[Stamp] = &[
    Stamp::Check,
    Stamp::Cross,
    Stamp::Star,
    Stamp::Heart,
    Stamp::Warning,
    Stamp::Info,
    Stamp::Question,
    Stamp::Exclamation,
    Stamp::Plus,
    Stamp::Minus,
    Stamp::Pointer,
    Stamp::ThumbsUp,
];

impl Stamp {
    pub fn name(self) -> &'static str {
        match self {
            Stamp::Check => "Check",
            Stamp::Cross => "Cross",
            Stamp::Star => "Star",
            Stamp::Heart => "Heart",
            Stamp::Warning => "Warning",
            Stamp::Info => "Info",
            Stamp::Question => "Question",
            Stamp::Exclamation => "Exclamation",
            Stamp::Plus => "Plus",
            Stamp::Minus => "Minus",
            Stamp::Pointer => "Pointer",
            Stamp::ThumbsUp => "Thumbs up",
        }
    }

    pub fn from_name(name: &str) -> Option<Stamp> {
        ALL.iter().copied().find(|s| s.name().eq_ignore_ascii_case(name))
    }
}

const GREEN: Rgba = Rgba::rgb(0x2E, 0x9E, 0x4F);
const RED: Rgba = Rgba::rgb(0xD9, 0x30, 0x25);
const GOLD: Rgba = Rgba::rgb(0xF5, 0xB4, 0x00);
const BLUE: Rgba = Rgba::rgb(0x1E, 0x6F, 0xD9);
const PURPLE: Rgba = Rgba::rgb(0x7B, 0x4C, 0xC9);

struct Pen<'a> {
    pm: &'a mut Pixmap,
    t: Transform,
    r: Box2,
}

impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> Pt {
        Pt::new(self.r.x0 + x * self.r.width(), self.r.y0 + y * self.r.height())
    }

    fn unit(&self) -> f32 {
        self.r.width().min(self.r.height())
    }

    fn paint(color: Rgba) -> Paint<'static> {
        let mut paint = Paint::default();
        paint.anti_alias = true;
        paint.set_color(color.to_color());
        paint
    }

    fn polygon(&mut self, pts: &[(f32, f32)], color: Rgba) {
        let mut pb = PathBuilder::new();
        for (i, &(x, y)) in pts.iter().enumerate() {
            let q = self.p(x, y);
            if i == 0 {
                pb.move_to(q.x, q.y);
            } else {
                pb.line_to(q.x, q.y);
            }
        }
        pb.close();
        if let Some(path) = pb.finish() {
            self.pm.fill_path(&path, &Self::paint(color), FillRule::Winding, self.t, None);
        }
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, color: Rgba) {
        let c = self.p(cx, cy);
        let rr = Box2::around(c, r * self.r.width(), r * self.r.height());
        if let Some(path) = rr.to_skia().and_then(PathBuilder::from_oval) {
            self.pm.fill_path(&path, &Self::paint(color), FillRule::Winding, self.t, None);
        }
    }

    fn polyline(&mut self, pts: &[(f32, f32)], width: f32, color: Rgba) {
        let mut pb = PathBuilder::new();
        for (i, &(x, y)) in pts.iter().enumerate() {
            let q = self.p(x, y);
            if i == 0 {
                pb.move_to(q.x, q.y);
            } else {
                pb.line_to(q.x, q.y);
            }
        }
        if let Some(path) = pb.finish() {
            let stroke = Stroke { width: width * self.unit(), line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
            self.pm.stroke_path(&path, &Self::paint(color), &stroke, self.t, None);
        }
    }

    fn badge(&mut self, color: Rgba) {
        self.circle(0.5, 0.5, 0.48, color);
    }

    /// A glyph-like bar-and-dot mark (for ! and i).
    fn bar_dot(&mut self, flip: bool) {
        let (bar, dot) = if flip { ((0.44, 0.68), 0.3) } else { ((0.22, 0.6), 0.74) };
        self.polyline(&[(0.5, bar.0 + 0.06), (0.5, bar.1)], 0.13, Rgba::WHITE);
        self.circle(0.5, dot, 0.075, Rgba::WHITE);
    }
}

pub fn draw(stamp: Stamp, rect: &Box2, pm: &mut Pixmap, t: Transform) {
    let mut pen = Pen { pm, t, r: *rect };
    match stamp {
        Stamp::Check => {
            pen.badge(GREEN);
            pen.polyline(&[(0.27, 0.52), (0.43, 0.68), (0.74, 0.34)], 0.11, Rgba::WHITE);
        }
        Stamp::Cross => {
            pen.badge(RED);
            pen.polyline(&[(0.32, 0.32), (0.68, 0.68)], 0.11, Rgba::WHITE);
            pen.polyline(&[(0.68, 0.32), (0.32, 0.68)], 0.11, Rgba::WHITE);
        }
        Stamp::Star => {
            let pts: Vec<(f32, f32)> = (0..10)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                    let r = if i % 2 == 0 { 0.5 } else { 0.2 };
                    (0.5 + r * a.cos(), 0.53 + r * a.sin())
                })
                .collect();
            pen.polygon(&pts, GOLD);
        }
        Stamp::Heart => {
            let pts: Vec<(f32, f32)> = (0..64)
                .map(|i| {
                    let a = i as f32 / 64.0 * std::f32::consts::TAU;
                    let x = 16.0 * a.sin().powi(3);
                    let y = 13.0 * a.cos() - 5.0 * (2.0 * a).cos() - 2.0 * (3.0 * a).cos() - (4.0 * a).cos();
                    (0.5 + x / 34.0, 0.47 - y / 34.0)
                })
                .collect();
            pen.polygon(&pts, RED);
        }
        Stamp::Warning => {
            pen.polygon(&[(0.5, 0.04), (0.98, 0.92), (0.02, 0.92)], GOLD);
            pen.polyline(&[(0.5, 0.34), (0.5, 0.62)], 0.11, Rgba::BLACK);
            pen.circle(0.5, 0.78, 0.065, Rgba::BLACK);
        }
        Stamp::Info => {
            pen.badge(BLUE);
            pen.bar_dot(true);
        }
        Stamp::Question => {
            pen.badge(PURPLE);
            pen.polyline(&[(0.36, 0.36), (0.42, 0.25), (0.56, 0.23), (0.65, 0.33), (0.6, 0.45), (0.5, 0.52), (0.5, 0.6)], 0.11, Rgba::WHITE);
            pen.circle(0.5, 0.75, 0.07, Rgba::WHITE);
        }
        Stamp::Exclamation => {
            pen.badge(RED);
            pen.bar_dot(false);
        }
        Stamp::Plus => {
            pen.badge(GREEN);
            pen.polyline(&[(0.5, 0.28), (0.5, 0.72)], 0.12, Rgba::WHITE);
            pen.polyline(&[(0.28, 0.5), (0.72, 0.5)], 0.12, Rgba::WHITE);
        }
        Stamp::Minus => {
            pen.badge(RED);
            pen.polyline(&[(0.28, 0.5), (0.72, 0.5)], 0.12, Rgba::WHITE);
        }
        Stamp::Pointer => {
            let arrow = [(0.25, 0.05), (0.25, 0.8), (0.42, 0.65), (0.55, 0.95), (0.66, 0.9), (0.53, 0.61), (0.76, 0.61)];
            let outline: Vec<(f32, f32)> = arrow.to_vec();
            pen.polygon(&outline, Rgba::BLACK);
            let inner: Vec<(f32, f32)> = arrow.iter().map(|&(x, y)| (0.45 + (x - 0.45) * 0.8, 0.55 + (y - 0.55) * 0.82)).collect();
            pen.polygon(&inner, Rgba::WHITE);
        }
        Stamp::ThumbsUp => {
            pen.badge(BLUE);
            pen.polygon(&[(0.24, 0.47), (0.36, 0.47), (0.36, 0.76), (0.24, 0.76)], Rgba::WHITE);
            pen.polygon(
                &[(0.4, 0.47), (0.52, 0.24), (0.58, 0.24), (0.6, 0.3), (0.56, 0.44), (0.74, 0.44), (0.77, 0.5), (0.72, 0.74), (0.66, 0.77), (0.4, 0.77)],
                Rgba::WHITE,
            );
        }
    }
}

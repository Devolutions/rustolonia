//! A document: a background raster plus annotation objects drawn over it.
//! Cloning is cheap (the background is shared), which is what the undo
//! history relies on.

use crate::color::Rgba;
use crate::geom::{Box2, IRect, Pt};
use crate::object::{Kind, Object, Style};
use crate::raster;
use std::sync::Arc;
use tiny_skia::Pixmap;

#[derive(Clone, Debug)]
pub struct Document {
    width: u32,
    height: u32,
    background: Arc<Pixmap>,
    pub objects: Vec<Object>,
    next_id: u64,
}

impl Document {
    pub fn new(background: Pixmap) -> Document {
        Document { width: background.width(), height: background.height(), background: Arc::new(background), objects: Vec::new(), next_id: 1 }
    }

    pub fn blank(width: u32, height: u32, color: Rgba) -> Document {
        Document::new(raster::solid(width, height, color).expect("non-zero size"))
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn bounds(&self) -> IRect {
        IRect::of_size(self.width, self.height)
    }

    pub fn background(&self) -> &Pixmap {
        &self.background
    }

    pub fn background_arc(&self) -> Arc<Pixmap> {
        self.background.clone()
    }

    /// The background for modification (copied first if shared with the
    /// history).
    pub fn background_mut(&mut self) -> &mut Pixmap {
        Arc::make_mut(&mut self.background)
    }

    /// Replaces the background; the canvas takes its size. Objects keep
    /// their positions.
    pub fn set_background(&mut self, background: Pixmap) {
        self.width = background.width();
        self.height = background.height();
        self.background = Arc::new(background);
    }

    pub fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add(&mut self, kind: Kind, style: Style) -> u64 {
        let id = self.allocate_id();
        self.objects.push(Object::new(id, kind, style));
        id
    }

    /// Adds an existing object under a fresh id.
    pub fn insert(&mut self, mut object: Object) -> u64 {
        object.id = self.allocate_id();
        let id = object.id;
        self.objects.push(object);
        id
    }

    pub fn index_of(&self, id: u64) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    pub fn get(&self, id: u64) -> Option<&Object> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Object> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    pub fn remove(&mut self, id: u64) -> Option<Object> {
        let i = self.index_of(id)?;
        Some(self.objects.remove(i))
    }

    /// Topmost object under `p`.
    pub fn hit(&self, p: Pt, tolerance: f32) -> Option<u64> {
        self.objects.iter().rev().find(|o| o.hit(p, tolerance)).map(|o| o.id)
    }

    /// Restores ids after loading, so new objects never reuse one.
    pub fn renumber_from_objects(&mut self) {
        self.next_id = self.objects.iter().map(|o| o.id).max().unwrap_or(0) + 1;
    }

    /// The label for the next step in `format` ("Numbers", "Letters",
    /// "Lowercase" or "Roman"), continuing from the steps of that format.
    pub fn next_step_label_as(&self, format: &str) -> String {
        let labels: Vec<&str> = self
            .objects
            .iter()
            .filter_map(|o| match &o.kind {
                Kind::Step { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        let value = |l: &str| -> Option<u32> {
            match format {
                "Letters" | "Lowercase" => {
                    let c = l.chars().next()?;
                    (l.len() == 1 && c.is_ascii_alphabetic() && (format == "Letters") == c.is_ascii_uppercase()).then(|| (c.to_ascii_uppercase() as u8 - b'A' + 1) as u32)
                }
                "Roman" => from_roman(l),
                _ => l.parse().ok(),
            }
        };
        let next = labels.iter().filter_map(|l| value(l)).max().unwrap_or(0) + 1;
        match format {
            "Letters" => char::from(b'A' + ((next - 1) % 26) as u8).to_string(),
            "Lowercase" => char::from(b'a' + ((next - 1) % 26) as u8).to_string(),
            "Roman" => to_roman(next),
            _ => next.to_string(),
        }
    }

    /// The label for the next step: one past the highest number used, or
    /// the next letter when the last step used letters.
    pub fn next_step_label(&self) -> String {
        let labels: Vec<&str> = self
            .objects
            .iter()
            .filter_map(|o| match &o.kind {
                Kind::Step { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        if let Some(last) = labels.last() {
            let mut chars = last.chars();
            if let (Some(c), None) = (chars.next(), chars.next()) {
                if c.is_ascii_alphabetic() && c != 'z' && c != 'Z' {
                    return char::from(c as u8 + 1).to_string();
                }
            }
        }
        let max = labels.iter().filter_map(|l| l.parse::<u32>().ok()).max().unwrap_or(0);
        (max + 1).to_string()
    }

    /// Grows `area` until it includes every pixel that objects touching it
    /// read, so a partial render matches a full one.
    fn closure(&self, area: Box2) -> Box2 {
        let mut area = area;
        loop {
            let mut grown = area;
            for o in &self.objects {
                if let Some(r) = o.reads() {
                    if o.bounds().intersects(&grown) {
                        grown = grown.union(&r).union(&o.bounds());
                    }
                }
            }
            if grown == area {
                return area;
            }
            area = grown;
        }
    }

    /// Renders `area` (grown as needed, clipped to the canvas) and returns
    /// the pixels with the document area they cover.
    pub fn render_area(&self, area: IRect, checker: bool) -> Option<(IRect, Pixmap)> {
        let area = self.closure(area.to_box()).to_irect().intersect(&self.bounds())?;
        // Paths clipped at the edge of the target antialias slightly
        // differently, so draw with a margin and keep only the inside.
        let padded = area.inflate(2).intersect(&self.bounds())?;
        let mut scratch = raster::extract(&self.background, padded)?;
        let region = padded.to_box();
        for o in &self.objects {
            if o.bounds().intersects(&region) {
                o.draw(&mut scratch, (padded.x, padded.y));
            }
        }
        let mut scratch = if padded == area {
            scratch
        } else {
            raster::extract(&scratch, IRect::new(area.x - padded.x, area.y - padded.y, area.w, area.h))?
        };
        if checker {
            let whole = IRect::of_size(scratch.width(), scratch.height());
            raster::checkerboard_under(&mut scratch, whole, (area.x, area.y));
        }
        Some((area, scratch))
    }

    /// Re-renders `area` of `display` (a canvas-sized pixmap); returns the
    /// area actually updated.
    pub fn render_into(&self, display: &mut Pixmap, area: IRect, checker: bool) -> Option<IRect> {
        let (area, pixels) = self.render_area(area, checker)?;
        raster::copy_area(&pixels, IRect::of_size(pixels.width(), pixels.height()), display, area.x, area.y);
        Some(area)
    }

    /// The finished image with real transparency, for export.
    pub fn flatten(&self) -> Pixmap {
        self.render_area(self.bounds(), false).map(|(_, p)| p).unwrap_or_else(|| (*self.background).clone())
    }

    pub fn translate_objects(&mut self, dx: f32, dy: f32) {
        for o in &mut self.objects {
            o.translate(dx, dy);
        }
    }

    /// Crops the canvas to `r` (clipped to it). Objects keep their place on
    /// the image.
    pub fn crop(&mut self, r: IRect) -> bool {
        let Some(r) = r.intersect(&self.bounds()) else { return false };
        if r == self.bounds() {
            return false;
        }
        let Some(bg) = raster::extract(&self.background, r) else { return false };
        self.set_background(bg);
        self.translate_objects(-r.x as f32, -r.y as f32);
        true
    }

    /// Changes the canvas size; the old image lands at (`dx`, `dy`) and the
    /// new area is filled with `fill`.
    pub fn resize_canvas(&mut self, width: u32, height: u32, dx: i32, dy: i32, fill: Rgba) {
        let Some(mut bg) = raster::solid(width, height, fill) else { return };
        bg.draw_pixmap(dx, dy, self.background.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
        self.set_background(bg);
        self.translate_objects(dx as f32, dy as f32);
    }

    /// Resamples the image to `width` x `height`, scaling objects with it.
    pub fn scale(&mut self, width: u32, height: u32) {
        let Some(bg) = raster::resample(&self.background, width, height) else { return };
        let from = self.bounds().to_box();
        let to = IRect::of_size(width, height).to_box();
        for o in &mut self.objects {
            o.map_geometry(&from, &to);
        }
        self.set_background(bg);
    }

    /// Quarter turn, clockwise or not.
    pub fn rotate(&mut self, clockwise: bool) {
        let (w, h) = (self.width as usize, self.height as usize);
        let Some(mut out) = Pixmap::new(self.height, self.width) else { return };
        {
            let src = self.background.pixels();
            let dst = out.pixels_mut();
            for y in 0..h {
                for x in 0..w {
                    let (nx, ny) = if clockwise { (h - 1 - y, x) } else { (y, w - 1 - x) };
                    dst[ny * h + nx] = src[y * w + x];
                }
            }
        }
        let (wf, hf) = (w as f32, h as f32);
        for o in &mut self.objects {
            if clockwise {
                o.map_points(&|p| Pt::new(hf - p.y, p.x), 1.0);
            } else {
                o.map_points(&|p| Pt::new(p.y, wf - p.x), 1.0);
            }
        }
        self.set_background(out);
    }

    pub fn flip(&mut self, horizontal: bool) {
        let (w, h) = (self.width as usize, self.height as usize);
        let pixels = self.background_mut().pixels_mut();
        if horizontal {
            for row in pixels.chunks_exact_mut(w) {
                row.reverse();
            }
        } else {
            for y in 0..h / 2 {
                let (top, bottom) = pixels.split_at_mut((h - 1 - y) * w);
                top[y * w..y * w + w].swap_with_slice(&mut bottom[..w]);
            }
        }
        let (wf, hf) = (w as f32, h as f32);
        for o in &mut self.objects {
            if horizontal {
                o.map_points(&|p| Pt::new(wf - p.x, p.y), 1.0);
            } else {
                o.map_points(&|p| Pt::new(p.x, hf - p.y), 1.0);
            }
        }
    }

    /// Draws every object into the background and removes them.
    pub fn flatten_objects(&mut self) {
        let flat = self.flatten();
        self.objects.clear();
        self.set_background(flat);
    }
}

const ROMAN: &[(u32, &str)] = &[(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")];

fn to_roman(mut n: u32) -> String {
    let mut out = String::new();
    for &(v, s) in ROMAN {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

fn from_roman(s: &str) -> Option<u32> {
    let n = (1..4000).find(|&n| to_roman(n) == s)?;
    Some(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{BlurMode, ShapeKind};

    fn doc() -> Document {
        let mut d = Document::blank(200, 100, Rgba::WHITE);
        for x in 0..200 {
            for y in 0..100 {
                if (x / 10 + y / 10) % 2 == 0 {
                    raster::set_pixel(d.background_mut(), x, y, Rgba::BLACK);
                }
            }
        }
        d
    }

    #[test]
    fn partial_render_matches_full_with_reading_objects() {
        let mut d = doc();
        d.add(Kind::Shape { rect: Box2::new(20.0, 20.0, 120.0, 80.0), shape: ShapeKind::Ellipse }, Style::default());
        d.add(Kind::Blur { rect: Box2::new(50.0, 10.0, 150.0, 90.0), mode: BlurMode::Blur, amount: 6.0 }, Style::default());
        d.add(Kind::Magnify { rect: Box2::new(100.0, 30.0, 160.0, 90.0), zoom: 2.0, ellipse: true, offset: Pt::default() }, Style::default());
        let full = d.flatten();
        let mut display = raster::solid(200, 100, Rgba::RED).unwrap();
        d.render_into(&mut display, IRect::new(0, 0, 200, 100), false);
        let mut partial = full.clone();
        raster::copy_area(&raster::solid(200, 100, Rgba::RED).unwrap(), IRect::new(60, 40, 20, 20), &mut partial, 60, 40);
        let updated = d.render_into(&mut partial, IRect::new(60, 40, 20, 20), false).unwrap();
        assert!(updated.w >= 100, "grown to the blur box: {updated:?}");
        // tiny-skia antialiases a few edge pixels differently depending on
        // the target size; a missing dependency differs far more widely.
        let channels = |p: &tiny_skia::PremultipliedColorU8| [p.red(), p.green(), p.blue(), p.alpha()];
        let diff = |a: &Pixmap, b: &Pixmap| {
            a.pixels()
                .iter()
                .zip(b.pixels())
                .map(|(x, y)| channels(x).iter().zip(channels(y)).map(|(p, q)| p.abs_diff(q)).max().unwrap())
                .filter(|d| *d > 2)
                .collect::<Vec<u8>>()
        };
        let d1 = diff(&partial, &full);
        assert!(d1.len() <= 12 && d1.iter().all(|d| *d < 48), "partial render differs: {d1:?}");
        assert!(diff(&display, &full).is_empty());
    }

    #[test]
    fn crop_rotate_flip_keep_objects_on_the_image() {
        let mut d = doc();
        let id = d.add(Kind::Shape { rect: Box2::new(20.0, 10.0, 40.0, 30.0), shape: ShapeKind::Rectangle }, Style::default());
        assert!(d.crop(IRect::new(10, 5, 100, 50)));
        assert_eq!(d.size(), (100, 50));
        assert_eq!(d.get(id).unwrap().rect(), Some(Box2::new(10.0, 5.0, 30.0, 25.0)));
        d.rotate(true);
        assert_eq!(d.size(), (50, 100));
        assert_eq!(d.get(id).unwrap().rect(), Some(Box2::new(25.0, 10.0, 45.0, 30.0)));
        d.flip(true);
        assert_eq!(d.get(id).unwrap().rect(), Some(Box2::new(5.0, 10.0, 25.0, 30.0)));
        d.rotate(false);
        assert_eq!(d.size(), (100, 50));
    }

    #[test]
    fn step_labels_continue() {
        let mut d = doc();
        assert_eq!(d.next_step_label(), "1");
        let step = |label: &str| Kind::Step { center: Pt::new(5.0, 5.0), radius: 10.0, label: label.into(), shape: crate::object::ShapeKind::Ellipse };
        d.add(step("7"), Style::default());
        assert_eq!(d.next_step_label(), "8");
        d.add(step("B"), Style::default());
        assert_eq!(d.next_step_label(), "C");
        assert_eq!(d.next_step_label_as("Letters"), "C");
        assert_eq!(d.next_step_label_as("Lowercase"), "a");
        d.add(step("iii"), Style::default());
        assert_eq!(d.next_step_label_as("Roman"), "iv");
        assert_eq!(d.next_step_label_as("Numbers"), "8");
    }
}

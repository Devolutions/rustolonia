//! Image analysis behind Smart Move, Text Replace and Simplify. These work
//! on the flat, mostly uniform backgrounds of screenshots: the background
//! color of an area is the dominant color of its border, objects are what
//! differs from it, and holes are filled from their surroundings.

use crate::color::Rgba;
use crate::document::Document;
use crate::editor::{default_style, Tool};
use crate::geom::{Box2, IRect};
use crate::object::{Kind, ShapeKind, Style};
use crate::raster;
use std::collections::{HashMap, VecDeque};
use tiny_skia::Pixmap;

/// One line of recognized text, in document pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    pub area: IRect,
    pub text: String,
    /// The line's words and their boxes, when the recognizer gives them.
    pub words: Vec<(IRect, String)>,
}

fn distance(a: Rgba, b: Rgba) -> u32 {
    (a.r as i32 - b.r as i32).unsigned_abs() + (a.g as i32 - b.g as i32).unsigned_abs() + (a.b as i32 - b.b as i32).unsigned_abs()
}

/// The most common color (to within 8 levels) on the one-pixel ring
/// around `r`, or along its inside edge where `r` touches the image edge.
pub fn border_color(pm: &Pixmap, r: IRect) -> Rgba {
    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let mut counts: HashMap<(u8, u8, u8), (u32, [u32; 4])> = HashMap::new();
    let mut visit = |x: i32, y: i32| {
        let (x, y) = (x.clamp(0, w - 1), y.clamp(0, h - 1));
        let c = raster::pixel(pm, x, y);
        let e = counts.entry((c.r >> 3, c.g >> 3, c.b >> 3)).or_insert((0, [0; 4]));
        e.0 += 1;
        e.1[0] += c.r as u32;
        e.1[1] += c.g as u32;
        e.1[2] += c.b as u32;
        e.1[3] += c.a as u32;
    };
    for x in r.x - 1..=r.right() {
        visit(x, r.y - 1);
        visit(x, r.bottom());
    }
    for y in r.y..r.bottom() {
        visit(r.x - 1, y);
        visit(r.right(), y);
    }
    let Some((_, (n, sum))) = counts.into_iter().max_by_key(|(_, (n, _))| *n) else { return Rgba::WHITE };
    Rgba::new((sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, (sum[3] / n) as u8)
}

/// Fills `r` by blending the pixels around it inward (a Coons-style
/// interpolation): exact on flat backgrounds, smooth on gradients.
pub fn fill_hole(pm: &mut Pixmap, r: IRect) {
    let Some(r) = r.intersect(&IRect::of_size(pm.width(), pm.height())) else { return };
    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let edge = |pm: &Pixmap, x: i32, y: i32| raster::pixel(pm, x.clamp(0, w - 1), y.clamp(0, h - 1));
    let source = pm.clone();
    for y in r.y..r.bottom() {
        let left = edge(&source, r.x - 1, y);
        let right = edge(&source, r.right(), y);
        for x in r.x..r.right() {
            let top = edge(&source, x, r.y - 1);
            let bottom = edge(&source, x, r.bottom());
            let tx = (x - r.x + 1) as f32 / (r.w + 1) as f32;
            let ty = (y - r.y + 1) as f32 / (r.h + 1) as f32;
            let horizontal = left.lerp(right, tx);
            let vertical = top.lerp(bottom, ty);
            let dx = (x - r.x + 1).min(r.right() - x) as f32;
            let dy = (y - r.y + 1).min(r.bottom() - y) as f32;
            let wh = 1.0 / dx;
            let wv = 1.0 / dy;
            raster::set_pixel(pm, x, y, horizontal.lerp(vertical, wv / (wh + wv)));
        }
    }
}

/// Fills the masked pixels of `area` from the outside in: each pass sets
/// pixels next to known ones to the average of their known neighbors.
pub fn fill_mask(pm: &mut Pixmap, area: IRect, mask: &[bool]) {
    let (aw, ah) = (area.w as usize, area.h as usize);
    let mut known: Vec<bool> = mask.iter().map(|m| !m).collect();
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    let neighbors = |x: usize, y: usize| {
        [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)].into_iter().filter(move |&(nx, ny)| nx < aw && ny < ah)
    };
    for y in 0..ah {
        for x in 0..aw {
            if !known[y * aw + x] && neighbors(x, y).any(|(nx, ny)| known[ny * aw + nx]) {
                queue.push_back((x, y));
            }
        }
    }
    let mut queued: Vec<bool> = vec![false; aw * ah];
    for &(x, y) in &queue {
        queued[y * aw + x] = true;
    }
    while !queue.is_empty() {
        let layer: Vec<(usize, usize)> = queue.drain(..).collect();
        let mut colors = Vec::with_capacity(layer.len());
        for &(x, y) in &layer {
            let mut sum = [0u32; 4];
            let mut n = 0;
            for (nx, ny) in neighbors(x, y) {
                if known[ny * aw + nx] {
                    let c = raster::pixel(pm, area.x + nx as i32, area.y + ny as i32);
                    sum[0] += c.r as u32;
                    sum[1] += c.g as u32;
                    sum[2] += c.b as u32;
                    sum[3] += c.a as u32;
                    n += 1;
                }
            }
            colors.push((n > 0).then(|| Rgba::new((sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, (sum[3] / n) as u8)));
        }
        for (&(x, y), color) in layer.iter().zip(colors) {
            if let Some(c) = color {
                raster::set_pixel(pm, area.x + x as i32, area.y + y as i32, c);
                known[y * aw + x] = true;
            }
        }
        for &(x, y) in &layer {
            for (nx, ny) in neighbors(x, y) {
                let i = ny * aw + nx;
                if !known[i] && !queued[i] {
                    queued[i] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
    }
}

/// The object(s) inside `area` as a mask over it: the pixels that differ
/// from the background, grown a little for antialiased edges. `None` when
/// nothing stands out.
pub fn lift_mask(bg: &Pixmap, area: IRect) -> Option<(IRect, Vec<bool>)> {
    let area = area.intersect(&IRect::of_size(bg.width(), bg.height()))?;
    lift(bg, area).map(|_| (area, object_mask(bg, area)))
}

fn object_mask(bg: &Pixmap, area: IRect) -> Vec<bool> {
    let back = border_color(bg, area);
    let (aw, ah) = (area.w as usize, area.h as usize);
    let mut mask: Vec<bool> = (0..aw * ah).map(|i| distance(raster::pixel(bg, area.x + (i % aw) as i32, area.y + (i / aw) as i32), back) > 36).collect();
    for _ in 0..2 {
        mask = (0..aw * ah)
            .map(|i| {
                let (x, y) = (i % aw, i / aw);
                mask[i] || (x > 0 && mask[i - 1]) || (x + 1 < aw && mask[i + 1]) || (y > 0 && mask[i - aw]) || (y + 1 < ah && mask[i + aw])
            })
            .collect();
    }
    mask
}

/// Separates the object(s) inside `area` from its background. Returns the
/// cut-out piece (transparent around the object), where it sits, and the
/// background with the hole filled. `None` when nothing stands out.
pub fn lift(bg: &Pixmap, area: IRect) -> Option<(Pixmap, IRect, Pixmap)> {
    let area = area.intersect(&IRect::of_size(bg.width(), bg.height()))?;
    let back = border_color(bg, area);
    let (aw, ah) = (area.w as usize, area.h as usize);
    let mut mask = vec![false; aw * ah];
    for y in 0..ah {
        for x in 0..aw {
            mask[y * aw + x] = distance(raster::pixel(bg, area.x + x as i32, area.y + y as i32), back) > 36;
        }
    }
    let count = mask.iter().filter(|m| **m).count();
    if count < 4 || count * 200 < aw * ah {
        return None;
    }
    // Grow the mask a little so antialiased edges travel with the object.
    for _ in 0..2 {
        let grown: Vec<bool> = (0..aw * ah)
            .map(|i| {
                let (x, y) = (i % aw, i / aw);
                mask[i] || (x > 0 && mask[i - 1]) || (x + 1 < aw && mask[i + 1]) || (y > 0 && mask[i - aw]) || (y + 1 < ah && mask[i + aw])
            })
            .collect();
        mask = grown;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (aw, ah, 0, 0);
    for y in 0..ah {
        for x in 0..aw {
            if mask[y * aw + x] {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    let at = IRect::new(area.x + x0 as i32, area.y + y0 as i32, (x1 - x0 + 1) as i32, (y1 - y0 + 1) as i32);
    let mut piece = Pixmap::new(at.w as u32, at.h as u32)?;
    for y in 0..at.h {
        for x in 0..at.w {
            let (mx, my) = (x as usize + x0, y as usize + y0);
            if mask[my * aw + mx] {
                raster::set_pixel(&mut piece, x, y, raster::pixel(bg, at.x + x, at.y + y));
            }
        }
    }
    let mut filled = bg.clone();
    fill_mask(&mut filled, area, &mask);
    Some((piece, at, filled))
}

/// The color of text in `area`: the average of the pixels that differ most
/// from the background.
pub fn text_color(pm: &Pixmap, area: IRect) -> Rgba {
    let back = border_color(pm, area);
    let mut px: Vec<(u32, Rgba)> = Vec::new();
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let c = raster::pixel(pm, x, y);
            px.push((distance(c, back), c));
        }
    }
    px.sort_by_key(|p| std::cmp::Reverse(p.0));
    let top = &px[..(px.len() / 10).max(1).min(px.len())];
    if top.is_empty() || top[0].0 < 40 {
        return back.contrasting();
    }
    let n = top.len() as u32;
    let sum = top.iter().fold([0u32; 3], |s, (_, c)| [s[0] + c.r as u32, s[1] + c.g as u32, s[2] + c.b as u32]);
    Rgba::rgb((sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8)
}

pub fn erase_text(pm: &mut Pixmap, area: IRect) {
    fill_hole(pm, area.inflate(2));
}

/// Replaces a line of text on the background with an editable text box in
/// the same place, size and color. Returns the new object's id.
pub fn replace_line(doc: &mut Document, line: &TextLine) -> u64 {
    let color = text_color(doc.background(), line.area);
    erase_text(doc.background_mut(), line.area);
    let size = (line.area.h as f32 * 0.92).max(8.0);
    let style = Style { width: 0.0, fill: Rgba::TRANSPARENT, text_color: color, font_size: size, bold: false, ..default_style(Tool::Text) };
    let pad = (size * 0.25).max(4.0);
    let a = line.area.to_box();
    let rect = Box2::new(a.x0 - pad, a.y0 - pad - size * 0.12, a.x1 + pad + size, a.y1 + pad);
    let mut object = crate::object::Object::new(0, Kind::Text { rect, text: line.text.clone() }, style);
    object.fit_text();
    doc.insert(object)
}

/// Reduces a screenshot to a clean diagram: text lines become rounded bars
/// (editable shapes), and the remaining image is flattened to a small
/// palette. Returns how many bars were added.
pub fn simplify(doc: &mut Document, lines: &[TextLine], colors: usize) -> usize {
    let mut bars = Vec::new();
    for line in lines {
        let back = border_color(doc.background(), line.area);
        let ink = text_color(doc.background(), line.area);
        erase_text(doc.background_mut(), line.area);
        let a = line.area.to_box();
        let thick = (a.height() * 0.55).max(4.0);
        let cy = a.center().y;
        bars.push((Box2::new(a.x0, cy - thick / 2.0, a.x1, cy + thick / 2.0), ink.lerp(back, 0.45)));
    }
    quantize(doc.background_mut(), colors.max(2));
    for (rect, color) in &bars {
        let style = Style { width: 0.0, fill: *color, stroke: Rgba::TRANSPARENT, ..Style::default() };
        doc.add(Kind::Shape { rect: *rect, shape: ShapeKind::Rounded }, style);
    }
    bars.len()
}

/// Maps every pixel to the nearest of `k` colors found by k-means on a
/// sample of the image.
pub fn quantize(pm: &mut Pixmap, k: usize) {
    let pixels = pm.pixels();
    let step = (pixels.len() / 20_000).max(1);
    let sample: Vec<[f32; 3]> = pixels
        .iter()
        .step_by(step)
        .map(|p| {
            let c = p.demultiply();
            [c.red() as f32, c.green() as f32, c.blue() as f32]
        })
        .collect();
    if sample.is_empty() {
        return;
    }
    // Farthest-point seeding: start from the most common color, then keep
    // adding the color farthest from every center so far, so small but
    // distinct areas (a blue button) get a center of their own.
    let mut counts: HashMap<(u8, u8, u8), usize> = HashMap::new();
    for c in &sample {
        *counts.entry((c[0] as u8 >> 3, c[1] as u8 >> 3, c[2] as u8 >> 3)).or_default() += 1;
    }
    let common = counts.iter().max_by_key(|(_, n)| **n).map(|(k, _)| *k).unwrap();
    let mut centers: Vec<[f32; 3]> = vec![[(common.0 << 3) as f32 + 4.0, (common.1 << 3) as f32 + 4.0, (common.2 << 3) as f32 + 4.0]];
    let dist2 = |a: &[f32; 3], b: &[f32; 3]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2);
    let mut nearest_d: Vec<f32> = sample.iter().map(|c| dist2(c, &centers[0])).collect();
    while centers.len() < k {
        let (i, d) = nearest_d.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap();
        if *d < 64.0 {
            break;
        }
        let c = sample[i];
        centers.push(c);
        for (nd, s) in nearest_d.iter_mut().zip(&sample) {
            *nd = nd.min(dist2(s, &c));
        }
    }
    let k = centers.len();
    let nearest = |centers: &[[f32; 3]], c: &[f32; 3]| {
        centers
            .iter()
            .enumerate()
            .map(|(i, m)| (i, (m[0] - c[0]).powi(2) + (m[1] - c[1]).powi(2) + (m[2] - c[2]).powi(2)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
            .unwrap()
    };
    for _ in 0..8 {
        let mut sums = vec![[0f32; 4]; k];
        for c in &sample {
            let i = nearest(&centers, c);
            sums[i][0] += c[0];
            sums[i][1] += c[1];
            sums[i][2] += c[2];
            sums[i][3] += 1.0;
        }
        for (center, s) in centers.iter_mut().zip(&sums) {
            if s[3] > 0.0 {
                *center = [s[0] / s[3], s[1] / s[3], s[2] / s[3]];
            }
        }
    }
    for p in pm.pixels_mut() {
        let c = p.demultiply();
        let m = centers[nearest(&centers, &[c.red() as f32, c.green() as f32, c.blue() as f32])];
        *p = Rgba::new(m[0] as u8, m[1] as u8, m[2] as u8, c.alpha()).premultiplied();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifts_an_object_and_fills_the_hole() {
        let mut bg = raster::solid(100, 60, Rgba::rgb(240, 240, 240)).unwrap();
        for y in 20..30 {
            for x in 30..50 {
                raster::set_pixel(&mut bg, x, y, Rgba::rgb(20, 90, 200));
            }
        }
        let (piece, at, filled) = lift(&bg, IRect::new(20, 10, 50, 30)).unwrap();
        assert!(at.x <= 30 && at.right() >= 50 && at.w < 30, "{at:?}");
        assert_eq!(raster::pixel(&piece, 30 - at.x + 5, 25 - at.y), Rgba::rgb(20, 90, 200));
        assert_eq!(raster::pixel(&piece, 0, 0).a, 0, "corners are transparent");
        assert_eq!(raster::pixel(&filled, 40, 25), Rgba::rgb(240, 240, 240));
        assert!(lift(&raster::solid(50, 50, Rgba::WHITE).unwrap(), IRect::new(5, 5, 20, 20)).is_none());
    }

    #[test]
    fn hole_fill_interpolates_gradients() {
        let mut pm = raster::solid(10, 1, Rgba::BLACK).unwrap();
        for x in 0..10 {
            raster::set_pixel(&mut pm, x, 0, Rgba::rgb((x * 25) as u8, 0, 0));
        }
        fill_hole(&mut pm, IRect::new(3, 0, 4, 1));
        let mid = raster::pixel(&pm, 5, 0).r;
        assert!((100..=150).contains(&mid), "{mid}");
    }

    #[test]
    fn quantize_reduces_colors() {
        let mut pm = raster::solid(64, 64, Rgba::WHITE).unwrap();
        for i in 0..64 {
            raster::set_pixel(&mut pm, i, i, Rgba::rgb(i as u8 * 4, 10, 10));
        }
        quantize(&mut pm, 3);
        let distinct: std::collections::HashSet<_> = pm.pixels().iter().map(|p| (p.red(), p.green(), p.blue())).collect();
        assert!(distinct.len() <= 3);

        // Small areas of distinct colors keep their own colors.
        let mut pm = raster::solid(200, 200, Rgba::WHITE).unwrap();
        for (x0, c) in [(10, Rgba::rgb(0x1E, 0x6F, 0xD9)), (60, Rgba::rgb(0xD9, 0x30, 0x25)), (110, Rgba::rgb(0x7B, 0x4C, 0xC9))] {
            for y in 10..20 {
                for x in x0..x0 + 20 {
                    raster::set_pixel(&mut pm, x, y, c);
                }
            }
        }
        quantize(&mut pm, 6);
        let blue = raster::pixel(&pm, 15, 15);
        assert!(blue.b > 180 && blue.r < 80, "blue stays blue: {blue:?}");
        assert!(raster::pixel(&pm, 65, 15).r > 180);
        assert_eq!(raster::pixel(&pm, 150, 150), Rgba::WHITE);
    }
}

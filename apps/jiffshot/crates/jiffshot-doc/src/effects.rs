//! Whole-image effects and adjustments. Effects change the background (and
//! the canvas size, for borders and shadows); annotations stay editable on
//! top and move with the image.

use crate::color::Rgba;
use crate::document::Document;
use crate::geom::IRect;
use crate::raster;
use tiny_skia::{Pixmap, PixmapPaint, Transform};

/// The look of an [`Effect::Edges`] border.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeStyle {
    Torn,
    Sawtooth,
    Wave,
}

#[derive(Clone, Debug)]
pub enum Effect {
    Border { width: u32, color: Rgba },
    Shadow { size: u32 },
    TornEdge { depth: u32 },
    /// Jagged or wavy edges on the chosen sides (top, right, bottom, left).
    Edges { style: EdgeStyle, depth: u32, sides: [bool; 4] },
    /// Turns the image in depth: `amount` > 0 shrinks the right side (or
    /// the top, when `tilt`).
    Perspective { amount: f32, tilt: bool },
    PageCurl,
    /// A band below the image with these lines (date, window, app…).
    CaptureInfo { text: String },
    Reflection,
    Monochrome,
    HueShift(f32),
    Gamma(f32),
    /// Every pixel near `from` becomes `to`.
    ReplaceColor { from: Rgba, to: Rgba },
    /// An image laid over the bottom-right corner.
    Watermark { image: std::sync::Arc<Pixmap>, opacity: f32 },
    /// Shrinks the image to fit within this many pixels across.
    ScaleTo(u32),
    RoundedCorners { radius: u32 },
    FadeEdges { size: u32 },
    Grayscale,
    Sepia,
    Invert,
    /// Brightness and contrast change, each -1..=1.
    Adjust { brightness: f32, contrast: f32 },
    Saturation(f32),
    Sharpen,
    BlurImage(u32),
    Vignette,
}

/// Effects offered in the menu, with their default parameters.
/// Effects that need more than a name (a file, the capture's details) and
/// are applied by the editor itself.
pub const INTERACTIVE: &[&str] = &["Capture info", "Watermark", "Spotlight & magnify"];

pub fn catalog(color: Rgba) -> Vec<(&'static str, Effect)> {
    catalog_for(color, Rgba::WHITE)
}

/// Effects offered in the menu, with their default parameters; `stroke`
/// and `fill` are the current colors (border color, color replacement).
pub fn catalog_for(color: Rgba, fill: Rgba) -> Vec<(&'static str, Effect)> {
    vec![
        ("Border", Effect::Border { width: 8, color }),
        ("Shadow", Effect::Shadow { size: 14 }),
        ("Torn edge", Effect::TornEdge { depth: 14 }),
        ("Torn bottom", Effect::Edges { style: EdgeStyle::Torn, depth: 16, sides: [false, false, true, false] }),
        ("Sawtooth edge", Effect::Edges { style: EdgeStyle::Sawtooth, depth: 10, sides: [true; 4] }),
        ("Wave edge", Effect::Edges { style: EdgeStyle::Wave, depth: 10, sides: [true; 4] }),
        ("Perspective", Effect::Perspective { amount: 0.18, tilt: false }),
        ("Tilt", Effect::Perspective { amount: 0.18, tilt: true }),
        ("Page curl", Effect::PageCurl),
        ("Reflection", Effect::Reflection),
        ("Rounded corners", Effect::RoundedCorners { radius: 18 }),
        ("Fade edges", Effect::FadeEdges { size: 40 }),
        ("Grayscale", Effect::Grayscale),
        ("Sepia", Effect::Sepia),
        ("Invert", Effect::Invert),
        ("Brighten", Effect::Adjust { brightness: 0.12, contrast: 0.0 }),
        ("Darken", Effect::Adjust { brightness: -0.12, contrast: 0.0 }),
        ("More contrast", Effect::Adjust { brightness: 0.0, contrast: 0.2 }),
        ("Less contrast", Effect::Adjust { brightness: 0.0, contrast: -0.2 }),
        ("Saturate", Effect::Saturation(1.35)),
        ("Desaturate", Effect::Saturation(0.6)),
        ("Sharpen", Effect::Sharpen),
        ("Blur image", Effect::BlurImage(3)),
        ("Vignette", Effect::Vignette),
        ("Monochrome", Effect::Monochrome),
        ("Hue shift", Effect::HueShift(40.0)),
        ("Gamma up", Effect::Gamma(1.3)),
        ("Gamma down", Effect::Gamma(0.77)),
        ("Replace color", Effect::ReplaceColor { from: color, to: fill }),
        ("Scale to 1920", Effect::ScaleTo(1920)),
    ]
}

pub fn by_name(name: &str, color: Rgba) -> Option<Effect> {
    by_name_for(name, color, Rgba::WHITE)
}

pub fn by_name_for(name: &str, color: Rgba, fill: Rgba) -> Option<Effect> {
    catalog_for(color, fill).into_iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, e)| e)
}

/// A deterministic edge profile in 0..=1 at position `i` along a side.
fn profile(style: EdgeStyle, i: u32, depth: f32, seed: u32) -> f32 {
    match style {
        EdgeStyle::Torn => {
            let j = i / 3;
            let mut x = j.wrapping_mul(2_654_435_761).wrapping_add(seed.wrapping_mul(40_503));
            x ^= x >> 13;
            let noise = (x % 1000) as f32 / 1000.0;
            let wave = ((j as f32) * 0.35 + seed as f32).sin() * 0.5 + 0.5;
            0.25 + 0.45 * noise + 0.3 * wave
        }
        EdgeStyle::Sawtooth => {
            let period = depth * 2.0;
            let ph = (i as f32 % period) / period;
            if ph < 0.5 { ph * 2.0 } else { (1.0 - ph) * 2.0 }
        }
        EdgeStyle::Wave => ((i as f32 / (depth * 4.0)) * std::f32::consts::TAU).sin() * 0.5 + 0.5,
    }
}

/// Bilinear sample of straight color at a fractional position.
fn sample(pm: &Pixmap, x: f32, y: f32) -> Rgba {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let p = |dx: i32, dy: i32| raster::pixel(pm, x0 as i32 + dx, y0 as i32 + dy);
    let top = p(0, 0).lerp(p(1, 0), fx);
    let bottom = p(0, 1).lerp(p(1, 1), fx);
    top.lerp(bottom, fy)
}

fn map_pixels(pm: &mut Pixmap, f: impl Fn(Rgba) -> Rgba) {
    for p in pm.pixels_mut() {
        let c = Rgba::from_premultiplied(*p);
        if c.a > 0 {
            *p = f(c).premultiplied();
        }
    }
}

fn channel(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Applies `effect` to the document. Returns false when it changed nothing.
pub fn apply(doc: &mut Document, effect: Effect) -> bool {
    match effect {
        Effect::Border { width, color } => {
            let (w, h) = doc.size();
            doc.resize_canvas(w + 2 * width, h + 2 * width, width as i32, width as i32, color);
        }
        Effect::Shadow { size } => {
            let (w, h) = doc.size();
            let blur = size / 2 + 1;
            let margin = blur * 2;
            let (nw, nh) = (w + margin + size, h + margin + size);
            let Some(mut out) = Pixmap::new(nw, nh) else { return false };
            let mut shadow = Pixmap::new(nw, nh).unwrap();
            for (i, p) in doc.background().pixels().iter().enumerate() {
                let (x, y) = ((i as u32 % w) + margin / 2 + size, (i as u32 / w) + margin / 2 + size);
                raster::set_pixel(&mut shadow, x as i32, y as i32, Rgba::BLACK.with_alpha((p.alpha() as f32 * 0.55) as u8));
            }
            raster::blur_area(&mut shadow, IRect::of_size(nw, nh), blur);
            out.draw_pixmap(0, 0, shadow.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
            let off = (margin / 2) as i32;
            out.draw_pixmap(off, off, doc.background().as_ref(), &PixmapPaint::default(), Transform::identity(), None);
            doc.set_background(out);
            doc.translate_objects(off as f32, off as f32);
        }
        Effect::TornEdge { depth } => {
            let (w, h) = doc.size();
            let d = depth.min(w / 4).min(h / 4).max(2) as f32;
            let pm = doc.background_mut();
            // A deterministic jagged profile per edge.
            let profile = |i: u32, seed: u32| {
                let mut x = i.wrapping_mul(2_654_435_761).wrapping_add(seed.wrapping_mul(40_503));
                x ^= x >> 13;
                let noise = (x % 1000) as f32 / 1000.0;
                let wave = ((i as f32) * 0.35 + seed as f32).sin() * 0.5 + 0.5;
                d * (0.25 + 0.45 * noise + 0.3 * wave)
            };
            let tops: Vec<f32> = (0..w).map(|x| profile(x / 3, 1)).collect();
            let bottoms: Vec<f32> = (0..w).map(|x| profile(x / 3, 2)).collect();
            let lefts: Vec<f32> = (0..h).map(|y| profile(y / 3, 3)).collect();
            let rights: Vec<f32> = (0..h).map(|y| profile(y / 3, 4)).collect();
            for y in 0..h {
                for x in 0..w {
                    let inside = (y as f32) >= tops[x as usize]
                        && ((h - 1 - y) as f32) >= bottoms[x as usize]
                        && (x as f32) >= lefts[y as usize]
                        && ((w - 1 - x) as f32) >= rights[y as usize];
                    if !inside {
                        raster::set_pixel(pm, x as i32, y as i32, Rgba::TRANSPARENT);
                    }
                }
            }
        }
        Effect::Edges { style, depth, sides } => {
            let (w, h) = doc.size();
            let d = depth.min(w / 4).min(h / 4).max(2) as f32;
            let pm = doc.background_mut();
            for y in 0..h {
                for x in 0..w {
                    let cut = (sides[0] && (y as f32) < d * profile(style, x, d, 1))
                        || (sides[1] && ((w - 1 - x) as f32) < d * profile(style, y, d, 2))
                        || (sides[2] && ((h - 1 - y) as f32) < d * profile(style, x, d, 3))
                        || (sides[3] && (x as f32) < d * profile(style, y, d, 4));
                    if cut {
                        raster::set_pixel(pm, x as i32, y as i32, Rgba::TRANSPARENT);
                    }
                }
            }
        }
        Effect::Perspective { amount, tilt } => {
            let (w, h) = doc.size();
            let src = doc.background().clone();
            let Some(mut out) = Pixmap::new(w, h) else { return false };
            let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
            for y in 0..h {
                for x in 0..w {
                    // How much the line through this pixel is squeezed.
                    let f = if tilt { 1.0 - amount * (1.0 - y as f32 / h as f32) } else { 1.0 - amount * (x as f32 / w as f32) };
                    let (sx, sy) = if tilt { ((x as f32 - cx) / f + cx, y as f32) } else { (x as f32, (y as f32 - cy) / f + cy) };
                    if sx >= 0.0 && sy >= 0.0 && sx <= (w - 1) as f32 && sy <= (h - 1) as f32 {
                        raster::set_pixel(&mut out, x as i32, y as i32, sample(&src, sx, sy));
                    }
                }
            }
            doc.set_background(out);
        }
        Effect::PageCurl => {
            let (w, h) = (doc.width() as f32, doc.height() as f32);
            let s = w.min(h) * 0.22;
            let pm = doc.background_mut();
            // Clear the corner beyond the fold, then draw the turned page.
            for y in (h - s) as i32..h as i32 {
                for x in (w - s) as i32..w as i32 {
                    if x as f32 + y as f32 > w + h - s {
                        raster::set_pixel(pm, x, y, Rgba::TRANSPARENT);
                    }
                }
            }
            let mut pb = tiny_skia::PathBuilder::new();
            pb.move_to(w - s, h);
            pb.line_to(w, h - s);
            pb.line_to(w - s * 0.92, h - s * 0.92);
            pb.close();
            if let Some(path) = pb.finish() {
                let stops = vec![
                    tiny_skia::GradientStop::new(0.0, Rgba::rgb(0xC8, 0xCC, 0xD2).to_color()),
                    tiny_skia::GradientStop::new(1.0, Rgba::WHITE.to_color()),
                ];
                let mut paint = tiny_skia::Paint::default();
                paint.anti_alias = true;
                if let Some(shader) = tiny_skia::LinearGradient::new(
                    tiny_skia::Point::from_xy(w - s / 2.0, h - s / 2.0),
                    tiny_skia::Point::from_xy(w - s * 0.92, h - s * 0.92),
                    stops,
                    tiny_skia::SpreadMode::Pad,
                    Transform::identity(),
                ) {
                    paint.shader = shader;
                }
                let mut shadow = tiny_skia::Paint::default();
                shadow.set_color(Rgba::BLACK.with_alpha(60).to_color());
                shadow.anti_alias = true;
                pm.fill_path(&path, &shadow, tiny_skia::FillRule::Winding, Transform::from_translate(-3.0, -3.0), None);
                pm.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
            }
        }
        Effect::CaptureInfo { text } => {
            let (w, h) = doc.size();
            let size = (w as f32 / 60.0).clamp(12.0, 22.0);
            let ts = crate::text::TextStyle { family: crate::text::DEFAULT_FAMILY, size, bold: false, italic: false };
            let layout = crate::text::layout(&text, &ts, Some(w as f32 - 24.0));
            let band = (layout.height() + 20.0).ceil() as u32;
            doc.resize_canvas(w, h + band, 0, 0, Rgba::rgb(0x22, 0x26, 0x2E));
            let pm = doc.background_mut();
            crate::text::draw(pm, (0, 0), &layout, 12.0, h as f32 + 10.0, w as f32 - 24.0, crate::text::Align::Left, Rgba::rgb(0xE8, 0xEA, 0xED));
        }
        Effect::Reflection => {
            let (w, h) = doc.size();
            let r = (h as f32 * 0.35) as u32;
            let src = doc.background().clone();
            let Some(mut out) = Pixmap::new(w, h + r) else { return false };
            raster::copy_area(&src, IRect::of_size(w, h), &mut out, 0, 0);
            for y in 0..r {
                let fade = 0.35 * (1.0 - y as f32 / r as f32);
                for x in 0..w {
                    let c = raster::pixel(&src, x as i32, (h - 1 - y) as i32);
                    raster::set_pixel(&mut out, x as i32, (h + y) as i32, c.faded(fade));
                }
            }
            doc.set_background(out);
        }
        Effect::Monochrome => map_pixels(doc.background_mut(), |c| {
            let v = if c.luminance() > 0.5 { 255 } else { 0 };
            Rgba::new(v, v, v, c.a)
        }),
        Effect::HueShift(deg) => {
            // Rotation about the gray axis in RGB space.
            let (sin, cos) = deg.to_radians().sin_cos();
            let k = 1.0 / 3.0;
            let sq = (1.0f32 / 3.0).sqrt();
            let m = [
                [cos + (1.0 - cos) * k, k * (1.0 - cos) - sq * sin, k * (1.0 - cos) + sq * sin],
                [k * (1.0 - cos) + sq * sin, cos + k * (1.0 - cos), k * (1.0 - cos) - sq * sin],
                [k * (1.0 - cos) - sq * sin, k * (1.0 - cos) + sq * sin, cos + k * (1.0 - cos)],
            ];
            map_pixels(doc.background_mut(), |c| {
                let v = [c.r as f32, c.g as f32, c.b as f32];
                let f = |row: [f32; 3]| channel(row[0] * v[0] + row[1] * v[1] + row[2] * v[2]);
                Rgba::new(f(m[0]), f(m[1]), f(m[2]), c.a)
            })
        }
        Effect::Gamma(g) => map_pixels(doc.background_mut(), |c| {
            let f = |v: u8| channel(255.0 * (v as f32 / 255.0).powf(1.0 / g));
            Rgba::new(f(c.r), f(c.g), f(c.b), c.a)
        }),
        Effect::ReplaceColor { from, to } => {
            let near = |c: Rgba| (c.r as i32 - from.r as i32).abs() + (c.g as i32 - from.g as i32).abs() + (c.b as i32 - from.b as i32).abs() < 60;
            let mut changed = false;
            for p in doc.background_mut().pixels_mut() {
                let c = Rgba::from_premultiplied(*p);
                if c.a > 0 && near(c) {
                    *p = to.with_alpha(c.a).premultiplied();
                    changed = true;
                }
            }
            return changed;
        }
        Effect::Watermark { image, opacity } => {
            let (w, h) = doc.size();
            let scale = (w as f32 * 0.25 / image.width() as f32).min(h as f32 * 0.25 / image.height() as f32).min(1.0);
            let (iw, ih) = (image.width() as f32 * scale, image.height() as f32 * scale);
            let margin = (w.min(h) as f32 * 0.03).max(8.0);
            let paint = tiny_skia::PixmapPaint { opacity, quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() };
            let t = Transform::from_row(scale, 0.0, 0.0, scale, w as f32 - iw - margin, h as f32 - ih - margin);
            doc.background_mut().draw_pixmap(0, 0, image.as_ref().as_ref(), &paint, t, None);
        }
        Effect::ScaleTo(max) => {
            let (w, h) = doc.size();
            if w <= max {
                return false;
            }
            let nh = (h as f32 * max as f32 / w as f32).round().max(1.0) as u32;
            doc.scale(max, nh);
        }
        Effect::RoundedCorners { radius } => {
            let (w, h) = doc.size();
            let r = radius.min(w / 2).min(h / 2) as f32;
            let pm = doc.background_mut();
            for y in 0..h {
                for x in 0..w {
                    let cx = if (x as f32) < r { r } else if (x as f32) > w as f32 - r { w as f32 - r } else { continue };
                    let cy = if (y as f32) < r { r } else if (y as f32) > h as f32 - r { h as f32 - r } else { continue };
                    let dist = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                    let coverage = (r - dist + 0.5).clamp(0.0, 1.0);
                    if coverage < 1.0 {
                        let i = (y * w + x) as usize * 4;
                        for c in &mut pm.data_mut()[i..i + 4] {
                            *c = (*c as f32 * coverage).round() as u8;
                        }
                    }
                }
            }
        }
        Effect::FadeEdges { size } => {
            let (w, h) = doc.size();
            let s = size.max(1) as f32;
            let pm = doc.background_mut();
            for y in 0..h {
                for x in 0..w {
                    let d = x.min(y).min(w - 1 - x).min(h - 1 - y) as f32;
                    if d < s {
                        let k = d / s;
                        let i = (y * w + x) as usize * 4;
                        for c in &mut pm.data_mut()[i..i + 4] {
                            *c = (*c as f32 * k).round() as u8;
                        }
                    }
                }
            }
        }
        Effect::Grayscale => map_pixels(doc.background_mut(), |c| {
            let l = channel(c.luminance() * 255.0);
            Rgba::new(l, l, l, c.a)
        }),
        Effect::Sepia => map_pixels(doc.background_mut(), |c| {
            let (r, g, b) = (c.r as f32, c.g as f32, c.b as f32);
            Rgba::new(
                channel(0.393 * r + 0.769 * g + 0.189 * b),
                channel(0.349 * r + 0.686 * g + 0.168 * b),
                channel(0.272 * r + 0.534 * g + 0.131 * b),
                c.a,
            )
        }),
        Effect::Invert => map_pixels(doc.background_mut(), |c| Rgba::new(255 - c.r, 255 - c.g, 255 - c.b, c.a)),
        Effect::Adjust { brightness, contrast } => {
            let k = (1.0 + contrast).max(0.0);
            let add = brightness * 255.0;
            map_pixels(doc.background_mut(), |c| {
                let f = |v: u8| channel((v as f32 - 128.0) * k + 128.0 + add);
                Rgba::new(f(c.r), f(c.g), f(c.b), c.a)
            })
        }
        Effect::Saturation(s) => map_pixels(doc.background_mut(), |c| {
            let l = c.luminance() * 255.0;
            let f = |v: u8| channel(l + (v as f32 - l) * s);
            Rgba::new(f(c.r), f(c.g), f(c.b), c.a)
        }),
        Effect::Sharpen => {
            let src = doc.background().clone();
            let (w, h) = (src.width() as i32, src.height() as i32);
            let pm = doc.background_mut();
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let c = raster::pixel(&src, x, y);
                    let n = [raster::pixel(&src, x - 1, y), raster::pixel(&src, x + 1, y), raster::pixel(&src, x, y - 1), raster::pixel(&src, x, y + 1)];
                    let f = |get: fn(&Rgba) -> u8| channel(5.0 * get(&c) as f32 - n.iter().map(|q| get(q) as f32).sum::<f32>());
                    raster::set_pixel(pm, x, y, Rgba::new(f(|q| q.r), f(|q| q.g), f(|q| q.b), c.a));
                }
            }
        }
        Effect::BlurImage(radius) => {
            let r = doc.bounds();
            raster::blur_area(doc.background_mut(), r, radius);
        }
        Effect::Vignette => {
            let (w, h) = doc.size();
            let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
            let max = (cx * cx + cy * cy).sqrt();
            let pm = doc.background_mut();
            for y in 0..h {
                for x in 0..w {
                    let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / max;
                    let k = 1.0 - 0.55 * ((d - 0.45) / 0.55).clamp(0.0, 1.0).powi(2);
                    let i = (y * w + x) as usize * 4;
                    let px = &mut pm.data_mut()[i..i + 3];
                    for c in px {
                        *c = (*c as f32 * k).round() as u8;
                    }
                }
            }
        }
    }
    true
}

/// How the two sides of a cut out meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// Joined seamlessly.
    Straight,
    /// A zigzag gap.
    Sawtooth,
    /// A wavy gap.
    Wave,
}

/// Like [`cut_out`], with a sawtooth or wave gap between the two sides
/// instead of a seamless join.
pub fn cut_out_with(doc: &mut Document, r: IRect, vertical: bool, edge: Edge) -> bool {
    if edge == Edge::Straight {
        return cut_out(doc, r, vertical);
    }
    const GAP: i32 = 14;
    let (start, len) = if vertical { (r.x.max(0), r.w) } else { (r.y.max(0), r.h) };
    if len <= GAP {
        return false;
    }
    // Cut all but GAP pixels, then cut the remaining band into teeth.
    let inner = if vertical { IRect::new(start + GAP / 2, 0, len - GAP, doc.height() as i32) } else { IRect::new(0, start + GAP / 2, doc.width() as i32, len - GAP) };
    if !cut_out(doc, inner, vertical) {
        return false;
    }
    let (w, h) = doc.size();
    let along = if vertical { h } else { w } as i32;
    let pm = doc.background_mut();
    let period = 16.0;
    for i in 0..along {
        let phase = (i as f32 % period) / period;
        let amp = match edge {
            Edge::Sawtooth => (if phase < 0.5 { phase } else { 1.0 - phase }) * 2.0,
            _ => (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5,
        };
        // Each side's edge follows the profile; the band between is clear.
        let a = start as f32 + amp * (GAP as f32 / 2.0 - 2.0);
        let b = start as f32 + GAP as f32 / 2.0 + (1.0 - amp) * (GAP as f32 / 2.0 - 2.0) + 2.0;
        for k in a.floor() as i32..b.ceil() as i32 {
            let (x, y) = if vertical { (k, i) } else { (i, k) };
            raster::set_pixel(pm, x, y, Rgba::TRANSPARENT);
        }
    }
    true
}

/// Removes the band `r` (full width, or full height when `vertical`) and
/// closes the gap. Objects past the band move with the image.
pub fn cut_out(doc: &mut Document, r: IRect, vertical: bool) -> bool {
    let (w, h) = doc.size();
    let (start, len, total) = if vertical { (r.x.max(0), r.w, w as i32) } else { (r.y.max(0), r.h, h as i32) };
    let len = len.min(total - start);
    if len <= 0 || len >= total {
        return false;
    }
    let (nw, nh) = if vertical { (w - len as u32, h) } else { (w, h - len as u32) };
    let Some(mut out) = Pixmap::new(nw, nh) else { return false };
    let bg = doc.background();
    if vertical {
        raster::copy_area(bg, IRect::new(0, 0, start, h as i32), &mut out, 0, 0);
        raster::copy_area(bg, IRect::new(start + len, 0, total - start - len, h as i32), &mut out, start, 0);
    } else {
        raster::copy_area(bg, IRect::new(0, 0, w as i32, start), &mut out, 0, 0);
        raster::copy_area(bg, IRect::new(0, start + len, w as i32, total - start - len), &mut out, 0, start);
    }
    let (s, e) = (start as f32, (start + len) as f32);
    for o in &mut doc.objects {
        let Some(b) = o.rect() else { continue };
        let c = if vertical { b.center().x } else { b.center().y };
        let shift = if c >= e { -(len as f32) } else if c > s { s - c } else { 0.0 };
        if vertical {
            o.translate(shift, 0.0);
        } else {
            o.translate(0.0, shift);
        }
    }
    doc.set_background(out);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_shadow_and_cut_out_resize_the_canvas() {
        let mut d = Document::blank(100, 50, Rgba::WHITE);
        apply(&mut d, Effect::Border { width: 5, color: Rgba::RED });
        assert_eq!(d.size(), (110, 60));
        assert_eq!(raster::pixel(d.background(), 1, 1), Rgba::RED);
        apply(&mut d, Effect::Shadow { size: 10 });
        assert!(d.width() > 110 && raster::pixel(d.background(), 0, 0).a == 0);
        let mut d = Document::blank(100, 100, Rgba::WHITE);
        assert!(cut_out(&mut d, IRect::new(0, 20, 100, 30), false));
        assert_eq!(d.size(), (100, 70));
        apply(&mut d, Effect::Grayscale);
        apply(&mut d, Effect::TornEdge { depth: 10 });
        for (name, effect) in catalog_for(Rgba::RED, Rgba::BLUE) {
            let mut d = Document::blank(120, 80, Rgba::rgb(200, 30, 30));
            apply(&mut d, effect);
            assert!(d.width() > 0, "{name}");
        }
        let mut info = Document::blank(200, 100, Rgba::WHITE);
        apply(&mut info, Effect::CaptureInfo { text: "Oct 28 2026 · Notepad".into() });
        assert!(info.height() > 100);
        let mut red = Document::blank(200, 100, Rgba::rgb(229, 57, 53));
        assert!(apply(&mut red, Effect::ReplaceColor { from: Rgba::RED, to: Rgba::BLUE }));
        assert_eq!(raster::pixel(red.background(), 5, 5), Rgba::BLUE);
        assert_eq!(raster::pixel(d.background(), 0, 0).a, 0);
        assert_eq!(raster::pixel(d.background(), 50, 35), Rgba::WHITE);
    }
}

//! Pixel-level helpers on premultiplied RGBA pixmaps: copies, coverage
//! blending, blur, pixelation, and the transparency checkerboard.

use crate::color::Rgba;
use crate::geom::IRect;
use tiny_skia::{Pixmap, PremultipliedColorU8};

/// Copies `area` of `src` to (`dx`, `dy`) in `dst`, clipped to both.
pub fn copy_area(src: &Pixmap, area: IRect, dst: &mut Pixmap, dx: i32, dy: i32) {
    let Some(a) = area.intersect(&IRect::of_size(src.width(), src.height())) else { return };
    let (dx, dy) = (dx + a.x - area.x, dy + a.y - area.y);
    let Some(d) = IRect::new(dx, dy, a.w, a.h).intersect(&IRect::of_size(dst.width(), dst.height())) else { return };
    let (sx, sy) = (a.x + d.x - dx, a.y + d.y - dy);
    let (ss, ds) = (src.width() as usize * 4, dst.width() as usize * 4);
    let row_bytes = d.w as usize * 4;
    let src = src.data();
    let dst = dst.data_mut();
    for row in 0..d.h as usize {
        let s = (sy as usize + row) * ss + sx as usize * 4;
        let t = (d.y as usize + row) * ds + d.x as usize * 4;
        dst[t..t + row_bytes].copy_from_slice(&src[s..s + row_bytes]);
    }
}

/// A new pixmap holding `area` of `src` (transparent outside it).
pub fn extract(src: &Pixmap, area: IRect) -> Option<Pixmap> {
    let mut out = Pixmap::new(area.w.max(1) as u32, area.h.max(1) as u32)?;
    copy_area(src, area, &mut out, 0, 0);
    Some(out)
}

/// Blends `color` at `coverage` (0..=1) over pixel (`x`, `y`).
#[inline]
pub fn blend(pixmap: &mut Pixmap, x: i32, y: i32, color: Rgba, coverage: f32) {
    if x < 0 || y < 0 || x >= pixmap.width() as i32 || y >= pixmap.height() as i32 || coverage <= 0.0 {
        return;
    }
    let a = color.a as f32 / 255.0 * coverage.min(1.0);
    let i = (y as usize * pixmap.width() as usize + x as usize) * 4;
    let d = &mut pixmap.data_mut()[i..i + 4];
    let inv = 1.0 - a;
    d[0] = (color.r as f32 * a + d[0] as f32 * inv).round().min(255.0) as u8;
    d[1] = (color.g as f32 * a + d[1] as f32 * inv).round().min(255.0) as u8;
    d[2] = (color.b as f32 * a + d[2] as f32 * inv).round().min(255.0) as u8;
    d[3] = (255.0 * a + d[3] as f32 * inv).round().min(255.0) as u8;
}

/// Three box-blur passes (close to a Gaussian of sigma ~ `radius` / 1.7)
/// over `area`, using only pixels inside it (edges are clamped).
pub fn blur_area(pixmap: &mut Pixmap, area: IRect, radius: u32) {
    let Some(a) = area.intersect(&IRect::of_size(pixmap.width(), pixmap.height())) else { return };
    if radius == 0 {
        return;
    }
    let mut buf = extract(pixmap, a).expect("non-empty area");
    let (w, h) = (buf.width() as usize, buf.height() as usize);
    let r = radius as usize;
    let mut tmp = vec![0u8; w * h * 4];
    for _ in 0..3 {
        box_pass(buf.data(), &mut tmp, w, h, r, true);
        box_pass(&tmp, buf.data_mut(), w, h, r, false);
    }
    copy_area(&buf, IRect::new(0, 0, a.w, a.h), pixmap, a.x, a.y);
}

/// One horizontal (or vertical) running-sum box blur from `src` to `dst`.
fn box_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horizontal: bool) {
    let (n, lines) = if horizontal { (w, h) } else { (h, w) };
    let at = |line: usize, i: usize| if horizontal { (line * w + i) * 4 } else { (i * w + line) * 4 };
    let window = (2 * r + 1) as u32;
    for line in 0..lines {
        let mut sum = [0u32; 4];
        for k in 0..=2 * r {
            let i = (k as isize - r as isize).clamp(0, n as isize - 1) as usize;
            let p = at(line, i);
            for c in 0..4 {
                sum[c] += src[p + c] as u32;
            }
        }
        for i in 0..n {
            let p = at(line, i);
            for c in 0..4 {
                dst[p + c] = ((sum[c] + window / 2) / window) as u8;
            }
            let out = at(line, (i as isize - r as isize).clamp(0, n as isize - 1) as usize);
            let inn = at(line, (i + r + 1).min(n - 1));
            for c in 0..4 {
                sum[c] = sum[c] + src[inn + c] as u32 - src[out + c] as u32;
            }
        }
    }
}

/// Replaces each `block`-sized cell of `area` with its average color.
pub fn pixelate_area(pixmap: &mut Pixmap, area: IRect, block: u32) {
    let Some(a) = area.intersect(&IRect::of_size(pixmap.width(), pixmap.height())) else { return };
    let block = block.max(2) as i32;
    let stride = pixmap.width() as usize * 4;
    let data = pixmap.data_mut();
    let mut y = a.y;
    while y < a.bottom() {
        let bh = block.min(a.bottom() - y);
        let mut x = a.x;
        while x < a.right() {
            let bw = block.min(a.right() - x);
            let mut sum = [0u32; 4];
            for row in y..y + bh {
                for col in x..x + bw {
                    let i = row as usize * stride + col as usize * 4;
                    for c in 0..4 {
                        sum[c] += data[i + c] as u32;
                    }
                }
            }
            let n = (bw * bh) as u32;
            let avg = sum.map(|s| ((s + n / 2) / n) as u8);
            for row in y..y + bh {
                for col in x..x + bw {
                    let i = row as usize * stride + col as usize * 4;
                    data[i..i + 4].copy_from_slice(&avg);
                }
            }
            x += block;
        }
        y += block;
    }
}

/// Composites the pixels of `area` over a light/dark checkerboard, so
/// transparency is visible on screen. `origin` keeps the pattern aligned to
/// document coordinates when `pixmap` is a piece of the document.
pub fn checkerboard_under(pixmap: &mut Pixmap, area: IRect, origin: (i32, i32)) {
    let Some(a) = area.intersect(&IRect::of_size(pixmap.width(), pixmap.height())) else { return };
    let stride = pixmap.width() as usize * 4;
    let data = pixmap.data_mut();
    for y in a.y..a.bottom() {
        for x in a.x..a.right() {
            let i = y as usize * stride + x as usize * 4;
            let alpha = data[i + 3];
            if alpha == 255 {
                continue;
            }
            let light = (((x + origin.0) >> 3) + ((y + origin.1) >> 3)) % 2 == 0;
            let bg = if light { 0xFF } else { 0xCC } as f32;
            let inv = 1.0 - alpha as f32 / 255.0;
            for c in 0..3 {
                data[i + c] = (data[i + c] as f32 + bg * inv).round().min(255.0) as u8;
            }
            data[i + 3] = 255;
        }
    }
}

/// Multiplies every pixel's alpha (and premultiplied color) by `opacity`.
pub fn fade(pixmap: &mut Pixmap, opacity: f32) {
    let o = opacity.clamp(0.0, 1.0);
    for b in pixmap.data_mut() {
        *b = (*b as f32 * o).round() as u8;
    }
}

/// Straight color of a pixel, or transparent outside the pixmap.
pub fn pixel(pixmap: &Pixmap, x: i32, y: i32) -> Rgba {
    if x < 0 || y < 0 || x >= pixmap.width() as i32 || y >= pixmap.height() as i32 {
        return Rgba::TRANSPARENT;
    }
    let p = pixmap.pixels()[y as usize * pixmap.width() as usize + x as usize];
    Rgba::from_premultiplied(p)
}

pub fn set_pixel(pixmap: &mut Pixmap, x: i32, y: i32, color: Rgba) {
    if x < 0 || y < 0 || x >= pixmap.width() as i32 || y >= pixmap.height() as i32 {
        return;
    }
    let w = pixmap.width() as usize;
    pixmap.pixels_mut()[y as usize * w + x as usize] = color.premultiplied();
}

/// A pixmap filled with one color.
pub fn solid(width: u32, height: u32, color: Rgba) -> Option<Pixmap> {
    let mut p = Pixmap::new(width.max(1), height.max(1))?;
    let c: PremultipliedColorU8 = color.premultiplied();
    p.pixels_mut().fill(c);
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_clips_on_every_side() {
        let src = solid(4, 4, Rgba::RED).unwrap();
        let mut dst = solid(4, 4, Rgba::WHITE).unwrap();
        copy_area(&src, IRect::new(-2, -2, 4, 4), &mut dst, 3, 3);
        assert_eq!(pixel(&dst, 3, 3), Rgba::WHITE);
        let mut dst = solid(4, 4, Rgba::WHITE).unwrap();
        copy_area(&src, IRect::new(0, 0, 4, 4), &mut dst, 2, 2);
        assert_eq!(pixel(&dst, 3, 3), Rgba::RED);
        assert_eq!(pixel(&dst, 1, 1), Rgba::WHITE);
    }

    #[test]
    fn blur_and_pixelate_average() {
        let mut p = solid(20, 1, Rgba::BLACK).unwrap();
        for x in 10..20 {
            set_pixel(&mut p, x, 0, Rgba::WHITE);
        }
        blur_area(&mut p, IRect::new(0, 0, 20, 1), 3);
        let mid = pixel(&p, 10, 0);
        assert!(mid.r > 60 && mid.r < 200, "{mid:?}");
        let mut q = solid(4, 4, Rgba::BLACK).unwrap();
        set_pixel(&mut q, 0, 0, Rgba::WHITE);
        pixelate_area(&mut q, IRect::new(0, 0, 4, 4), 2);
        assert_eq!(pixel(&q, 1, 1).r, 64);
        assert_eq!(pixel(&q, 3, 3).r, 0);
    }
}

/// Scanline flood fill of the region connected to (`x`, `y`) whose colors
/// are within `tolerance` (per channel) of the seed. Returns the filled
/// area.
pub fn flood_fill(pixmap: &mut Pixmap, x: i32, y: i32, color: Rgba, tolerance: u8) -> Option<IRect> {
    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    if x < 0 || y < 0 || x >= w || y >= h {
        return None;
    }
    let seed = pixmap.pixels()[(y * w + x) as usize];
    let fill = color.premultiplied();
    if seed == fill {
        return None;
    }
    let close = |p: PremultipliedColorU8| {
        let t = tolerance as i32;
        (p.red() as i32 - seed.red() as i32).abs() <= t
            && (p.green() as i32 - seed.green() as i32).abs() <= t
            && (p.blue() as i32 - seed.blue() as i32).abs() <= t
            && (p.alpha() as i32 - seed.alpha() as i32).abs() <= t
    };
    let mut done = vec![false; (w * h) as usize];
    let mut stack = vec![(x, y)];
    let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
    let pixels = pixmap.pixels_mut();
    while let Some((sx, sy)) = stack.pop() {
        let row = sy * w;
        let mut l = sx;
        while l > 0 && !done[(row + l - 1) as usize] && close(pixels[(row + l - 1) as usize]) {
            l -= 1;
        }
        let mut r = sx;
        while r + 1 < w && !done[(row + r + 1) as usize] && close(pixels[(row + r + 1) as usize]) {
            r += 1;
        }
        if done[(row + sx) as usize] || !close(pixels[(row + sx) as usize]) {
            continue;
        }
        for i in l..=r {
            done[(row + i) as usize] = true;
            pixels[(row + i) as usize] = fill;
        }
        (x0, x1, y0, y1) = (x0.min(l), x1.max(r), y0.min(sy), y1.max(sy));
        for ny in [sy - 1, sy + 1] {
            if ny < 0 || ny >= h {
                continue;
            }
            let mut i = l;
            while i <= r {
                let at = (ny * w + i) as usize;
                if !done[at] && close(pixels[at]) {
                    stack.push((i, ny));
                    while i <= r && !done[(ny * w + i) as usize] && close(pixels[(ny * w + i) as usize]) {
                        i += 1;
                    }
                }
                i += 1;
            }
        }
    }
    Some(IRect::new(x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

/// The area connected to (`x`, `y`) whose colors are within `tolerance` of
/// it, as a mask over its bounding rectangle (magic wand).
pub fn flood_mask(pixmap: &Pixmap, x: i32, y: i32, tolerance: u8) -> Option<(IRect, Vec<bool>)> {
    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    if x < 0 || y < 0 || x >= w || y >= h {
        return None;
    }
    let px = pixmap.pixels();
    let seed = px[(y * w + x) as usize];
    let t = tolerance as i32;
    let close = |p: PremultipliedColorU8| {
        (p.red() as i32 - seed.red() as i32).abs() <= t
            && (p.green() as i32 - seed.green() as i32).abs() <= t
            && (p.blue() as i32 - seed.blue() as i32).abs() <= t
            && (p.alpha() as i32 - seed.alpha() as i32).abs() <= t
    };
    let mut done = vec![false; (w * h) as usize];
    let mut stack = vec![(x, y)];
    done[(y * w + x) as usize] = true;
    let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
    while let Some((cx, cy)) = stack.pop() {
        (x0, x1, y0, y1) = (x0.min(cx), x1.max(cx), y0.min(cy), y1.max(cy));
        for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let i = (ny * w + nx) as usize;
            if !done[i] && close(px[i]) {
                done[i] = true;
                stack.push((nx, ny));
            }
        }
    }
    let area = IRect::new(x0, y0, x1 - x0 + 1, y1 - y0 + 1);
    let mask = (area.y..area.bottom()).flat_map(|yy| (area.x..area.right()).map(move |xx| (xx, yy))).map(|(xx, yy)| done[(yy * w + xx) as usize]).collect();
    Some((area, mask))
}

/// Every pixel within `tolerance` of the color at (`x`, `y`), anywhere in
/// the image, as a mask over its bounding rectangle (global magic wand).
pub fn color_mask(pixmap: &Pixmap, x: i32, y: i32, tolerance: u8) -> Option<(IRect, Vec<bool>)> {
    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    if x < 0 || y < 0 || x >= w || y >= h {
        return None;
    }
    let px = pixmap.pixels();
    let seed = px[(y * w + x) as usize];
    let t = tolerance as i32;
    let close = |p: &PremultipliedColorU8| {
        (p.red() as i32 - seed.red() as i32).abs() <= t
            && (p.green() as i32 - seed.green() as i32).abs() <= t
            && (p.blue() as i32 - seed.blue() as i32).abs() <= t
            && (p.alpha() as i32 - seed.alpha() as i32).abs() <= t
    };
    let all: Vec<bool> = px.iter().map(close).collect();
    let mut b = (w, h, -1, -1);
    for (i, on) in all.iter().enumerate() {
        if *on {
            let (xx, yy) = (i as i32 % w, i as i32 / w);
            b = (b.0.min(xx), b.1.min(yy), b.2.max(xx), b.3.max(yy));
        }
    }
    let area = IRect::new(b.0, b.1, b.2 - b.0 + 1, b.3 - b.1 + 1);
    let mask = (area.y..area.bottom()).flat_map(|yy| (area.x..area.right()).map(move |xx| (xx, yy))).map(|(xx, yy)| all[(yy * w + xx) as usize]).collect();
    Some((area, mask))
}

/// Replaces every pixel within `tolerance` of the color at (`x`, `y`) with
/// `color` (global fill). Returns whether anything changed.
pub fn replace_color(pixmap: &mut Pixmap, x: i32, y: i32, color: Rgba, tolerance: u8) -> bool {
    let Some((area, mask)) = color_mask(pixmap, x, y, tolerance) else { return false };
    let fill = color.premultiplied();
    let w = pixmap.width() as i32;
    let px = pixmap.pixels_mut();
    for (i, on) in mask.iter().enumerate() {
        if *on {
            let (xx, yy) = (area.x + i as i32 % area.w, area.y + i as i32 / area.w);
            px[(yy * w + xx) as usize] = fill;
        }
    }
    true
}

/// Resamples `src` to `width` x `height` with bicubic filtering.
pub fn resample(src: &Pixmap, width: u32, height: u32) -> Option<Pixmap> {
    let mut out = Pixmap::new(width.max(1), height.max(1))?;
    let t = tiny_skia::Transform::from_scale(width as f32 / src.width() as f32, height as f32 / src.height() as f32);
    let paint = tiny_skia::PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() };
    out.draw_pixmap(0, 0, src.as_ref(), &paint, t, None);
    Some(out)
}

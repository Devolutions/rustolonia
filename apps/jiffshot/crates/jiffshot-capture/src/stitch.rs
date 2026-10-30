//! Joining overlapping frames into one image, for scrolling capture
//! (frames of one area while its content scrolls vertically) and
//! panoramic capture (frames moved in any direction).

use crate::Frame;
use std::collections::HashMap;

/// Pixels at the right edge left out of row matching: scrollbars move.
const SCROLLBAR: usize = 24;

fn row_hashes(f: &Frame) -> Vec<u64> {
    let usable = (f.width as usize).saturating_sub(SCROLLBAR).max(1) * 4;
    (0..f.height as usize)
        .map(|y| {
            let row = &f.pixels[y * f.stride()..y * f.stride() + usable];
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for (i, b) in row.iter().enumerate() {
                if i % 4 != 3 {
                    h = (h ^ *b as u64).wrapping_mul(0x100_0000_01b3);
                }
            }
            h
        })
        .collect()
}

/// How `next` continues `prev` when the content between a fixed header and
/// footer scrolled up by `dy` rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub dy: usize,
    pub header: usize,
    pub footer: usize,
}

/// Finds how far the content scrolled between two frames of the same area.
/// `None` when nothing moved (the end was reached) or no overlap is found.
pub fn vertical_step(prev: &Frame, next: &Frame) -> Option<Step> {
    if (prev.width, prev.height) != (next.width, next.height) {
        return None;
    }
    let (a, b) = (row_hashes(prev), row_hashes(next));
    let h = a.len();
    if a == b {
        return None;
    }
    // Rows that stay put at the top and bottom are fixed headers and footers.
    let cap = h / 3;
    let header = a.iter().zip(&b).take_while(|(x, y)| x == y).count().min(cap);
    let footer = a.iter().rev().zip(b.iter().rev()).take_while(|(x, y)| x == y).count().min(cap);
    let (top, bottom) = (header, h - footer);
    if bottom <= top + 4 {
        return None;
    }
    // Vote for offsets with rows that are distinctive (blank rows repeat).
    let mut positions: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, &v) in a.iter().enumerate().take(bottom).skip(top) {
        positions.entry(v).or_default().push(i);
    }
    let mut votes: HashMap<usize, usize> = HashMap::new();
    for (i, v) in b.iter().enumerate().take(bottom).skip(top) {
        if let Some(rows) = positions.get(v) {
            if rows.len() > 6 {
                continue;
            }
            for &j in rows {
                if j > i {
                    *votes.entry(j - i).or_default() += 1;
                }
            }
        }
    }
    let (&dy, &count) = votes.iter().max_by_key(|(dy, n)| (**n, std::cmp::Reverse(**dy)))?;
    let overlap = bottom - top - dy;
    let distinctive = (top..bottom - dy).filter(|&i| positions.get(&b[i]).is_some_and(|r| r.len() <= 6)).count();
    (count * 2 >= distinctive.min(overlap) && count >= 3).then_some(Step { dy, header, footer })
}

/// Stacks scrolled frames: the first frame, the new rows each later frame
/// adds, and the last frame's footer. Rows can match by accident between
/// two frames (blank lines at the edge), so the fixed header and footer are
/// the bands that stayed put across every step.
pub fn stitch_vertical(frames: &[Frame]) -> Option<Frame> {
    let first = frames.first()?;
    let (w, h) = (first.width as usize, first.height as usize);
    let stride = w * 4;
    let steps: Vec<Step> = frames.windows(2).map_while(|p| vertical_step(&p[0], &p[1])).collect();
    let used = &frames[..steps.len() + 1];
    let footer = steps.iter().map(|s| s.footer).min().unwrap_or(0);
    let mut rows: Vec<u8> = Vec::with_capacity(stride * (h + steps.iter().map(|s| s.dy).sum::<usize>()));
    rows.extend_from_slice(&first.pixels[..(h - footer) * stride]);
    for (frame, step) in used[1..].iter().zip(&steps) {
        let end = h - footer;
        let start = end - step.dy;
        rows.extend_from_slice(&frame.pixels[start * stride..end * stride]);
    }
    let last = used.last().unwrap();
    rows.extend_from_slice(&last.pixels[(h - footer) * stride..h * stride]);
    let height = rows.len() / stride;
    Some(Frame { width: w as u32, height: height as u32, pixels: rows, alpha: false })
}

/// Luma, averaged over `k` x `k` blocks.
fn gray(f: &Frame, k: usize) -> (Vec<u8>, usize, usize) {
    let (w, h) = (f.width as usize / k, f.height as usize / k);
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0u32;
            for yy in 0..k {
                for xx in 0..k {
                    let i = (y * k + yy) * f.stride() + (x * k + xx) * 4;
                    let p = &f.pixels[i..i + 3];
                    sum += (p[0] as u32 * 29 + p[1] as u32 * 150 + p[2] as u32 * 77) >> 8;
                }
            }
            out[y * w + x] = (sum / (k * k) as u32) as u8;
        }
    }
    (out, w, h)
}

/// Mean absolute difference of `b` placed at (`dx`, `dy`) over `a`, on the
/// overlap (sampled every `step` pixels); `None` when the overlap is small.
fn sad(a: &[u8], b: &[u8], w: usize, h: usize, dx: i32, dy: i32, step: usize) -> Option<f32> {
    let (x0, x1) = (dx.max(0) as usize, (w as i32 + dx.min(0)) as usize);
    let (y0, y1) = (dy.max(0) as usize, (h as i32 + dy.min(0)) as usize);
    if x1 <= x0 + w / 5 || y1 <= y0 + h / 5 {
        return None;
    }
    let (mut sum, mut n) = (0u64, 0u64);
    for y in (y0..y1).step_by(step) {
        let by = (y as i32 - dy) as usize;
        for x in (x0..x1).step_by(step) {
            let bx = (x as i32 - dx) as usize;
            sum += (a[y * w + x] as i32 - b[by * w + bx] as i32).unsigned_abs() as u64;
            n += 1;
        }
    }
    Some(sum as f32 / n.max(1) as f32)
}

/// Where `next` sits relative to `prev` (content moved by the opposite
/// amount), found coarse to fine. `None` when no placement matches well.
pub fn translation(prev: &Frame, next: &Frame) -> Option<(i32, i32)> {
    if (prev.width, prev.height) != (next.width, next.height) {
        return None;
    }
    let k = 4;
    let (a, w, h) = gray(prev, k);
    let (b, _, _) = gray(next, k);
    let (rx, ry) = (w as i32 * 2 / 3, h as i32 * 2 / 3);
    let mut coarse: Vec<(f32, i32, i32)> = Vec::new();
    for dy in -ry..=ry {
        for dx in -rx..=rx {
            if let Some(d) = sad(&a, &b, w, h, dx, dy, 2) {
                // Prefer small moves on ties (flat areas match anywhere).
                coarse.push((d + (dx.abs() + dy.abs()) as f32 * 0.002, dx, dy));
            }
        }
    }
    // Block averages blur sub-block offsets, so refine several candidates.
    coarse.sort_by(|x, y| x.0.total_cmp(&y.0));
    let (af, wf, hf) = gray(prev, 1);
    let (bf, _, _) = gray(next, 1);
    let r = k as i32 + 1;
    let mut fine = (f32::MAX, 0, 0);
    for &(_, cx, cy) in coarse.iter().take(12) {
        for dy in cy * k as i32 - r..=cy * k as i32 + r {
            for dx in cx * k as i32 - r..=cx * k as i32 + r {
                if let Some(d) = sad(&af, &bf, wf, hf, dx, dy, 2) {
                    if d < fine.0 {
                        fine = (d, dx, dy);
                    }
                }
            }
        }
    }
    // Sampling every other pixel can tie neighbors; settle at full density.
    let mut exact = (f32::MAX, fine.1, fine.2);
    for dy in fine.2 - 1..=fine.2 + 1 {
        for dx in fine.1 - 1..=fine.1 + 1 {
            if let Some(d) = sad(&af, &bf, wf, hf, dx, dy, 1) {
                if d < exact.0 {
                    exact = (d, dx, dy);
                }
            }
        }
    }
    (exact.0 < 6.0).then_some((exact.1, exact.2))
}

/// Paints frames at their offsets onto one canvas (later frames on top).
pub fn compose(frames: &[(Frame, (i32, i32))]) -> Option<Frame> {
    let (f0, _) = frames.first()?;
    let (fw, fh) = (f0.width as i32, f0.height as i32);
    let same = frames.iter().all(|(f, _)| f.width == f0.width && f.height == f0.height);
    let x0 = frames.iter().map(|(_, (x, _))| *x).min()?;
    let y0 = frames.iter().map(|(_, (_, y))| *y).min()?;
    let x1 = frames.iter().map(|(f, (x, _))| *x + f.width as i32).max()?;
    let y1 = frames.iter().map(|(f, (_, y))| *y + f.height as i32).max()?;
    let mut out = Frame::new((x1 - x0) as u32, (y1 - y0) as u32);
    // A path that moved both ways (or frames of different sizes) leaves
    // places no frame covered; they stay transparent.
    out.alpha = !same || (x1 - x0 > fw && y1 - y0 > fh);
    for (f, (x, y)) in frames {
        for row in 0..f.height as usize {
            let dst = ((y - y0) as usize + row) * out.stride() + (x - x0) as usize * 4;
            let src = row * f.stride();
            out.pixels[dst..dst + f.stride()].copy_from_slice(&f.pixels[src..src + f.stride()]);
            if out.alpha {
                for px in out.pixels[dst..dst + f.stride()].chunks_exact_mut(4) {
                    px[3] = 255;
                }
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tall "page" with distinctive rows and some blank ones.
    fn page(width: u32, height: u32) -> Frame {
        let mut f = Frame::new(width, height);
        for y in 0..height as usize {
            let blank = y % 37 < 9;
            for x in 0..width as usize {
                let i = y * f.stride() + x * 4;
                // Blocky noise: structured like UI content, never periodic.
                let mut n = ((x / 6) as u32).wrapping_mul(0x9E37_79B9) ^ ((y / 6) as u32).wrapping_mul(0x85EB_CA6B);
                n ^= n >> 15;
                n = n.wrapping_mul(0x2C1B_3C6D);
                let v = if blank { 250 } else { (n >> 24) as u8 };
                f.pixels[i..i + 4].copy_from_slice(&[v, v.wrapping_add(40), (y % 251) as u8, 0]);
            }
        }
        f
    }

    /// The view of `page` scrolled to `top`, with a fixed header and footer.
    fn view(page: &Frame, top: usize, h: usize, header: usize, footer: usize) -> Frame {
        let mut f = Frame::new(page.width, h as u32);
        let s = f.stride();
        for y in 0..h {
            let src = if y < header {
                y
            } else if y >= h - footer {
                page.height as usize - (h - y)
            } else {
                top + y
            };
            f.pixels[y * s..(y + 1) * s].copy_from_slice(&page.pixels[src * s..(src + 1) * s]);
        }
        f
    }

    #[test]
    fn stitches_scrolled_frames_with_fixed_header_and_footer() {
        let p = page(200, 2000);
        let (h, header, footer) = (400, 30, 20);
        let tops = [0, 170, 340, 510, 680, 850, 1020, 1190, 1360, 1530, 1580, 1580];
        let frames: Vec<Frame> = tops.iter().map(|&t| view(&p, t, h, header, footer)).collect();
        assert_eq!(vertical_step(&frames[0], &frames[1]), Some(Step { dy: 170, header, footer }));
        assert_eq!(vertical_step(&frames[10], &frames[11]), None, "end of page");
        let out = stitch_vertical(&frames).unwrap();
        // The header, every content row down to the last view, the footer.
        assert_eq!(out.height as usize, 1580 + h);
        let s = out.stride();
        for y in [0usize, 100, 700, 1500, 1959] {
            assert_eq!(&out.pixels[y * s..(y + 1) * s], &p.pixels[y * s..(y + 1) * s], "row {y}");
        }
        let last = out.height as usize - 1;
        assert_eq!(&out.pixels[last * s..], &p.pixels[(p.height as usize - 1) * s..], "footer");
    }

    #[test]
    fn finds_panoramic_translations() {
        let p = page(900, 700);
        let crop = |x: i32, y: i32| p.crop(crate::Rect::new(x, y, 400, 300)).unwrap();
        let a = crop(100, 100);
        let b = crop(260, 150);
        assert_eq!(translation(&a, &b), Some((160, 50)));
        let c = crop(40, 30);
        assert_eq!(translation(&a, &c), Some((-60, -70)));
        let out = compose(&[(a, (0, 0)), (b, (160, 50))]).unwrap();
        assert_eq!((out.width, out.height), (560, 350));
    }
}

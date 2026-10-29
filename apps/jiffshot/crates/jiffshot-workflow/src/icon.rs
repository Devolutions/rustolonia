//! The program icon, drawn so it is crisp at every size: written to
//! `assets/jiffshot.ico` by `--write-icon`.

use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

pub(crate) fn draw(size: u32) -> Pixmap {
    let mut pixmap = Pixmap::new(size, size).expect("icon size is non-zero");
    let s = size as f32 / 32.0;
    let mut paint = Paint::default();
    paint.anti_alias = true;

    let mut body = PathBuilder::new();
    let (r, inset, edge) = (7.0 * s, 1.0 * s, 31.0 * s);
    body.move_to(inset + r, inset);
    body.line_to(edge - r, inset);
    body.quad_to(edge, inset, edge, inset + r);
    body.line_to(edge, edge - r);
    body.quad_to(edge, edge, edge - r, edge);
    body.line_to(inset + r, edge);
    body.quad_to(inset, edge, inset, edge - r);
    body.line_to(inset, inset + r);
    body.quad_to(inset, inset, inset + r, inset);
    body.close();
    paint.set_color_rgba8(0x2F, 0x8C, 0xFF, 0xFF);
    pixmap.fill_path(&body.finish().expect("closed path"), &paint, FillRule::Winding, Transform::identity(), None);

    // Viewfinder corners.
    paint.set_color_rgba8(0xFF, 0xFF, 0xFF, 0xFF);
    let stroke = Stroke { width: 3.0 * s, line_cap: LineCap::Round, ..Stroke::default() };
    let (a, b, l) = (8.0 * s, 24.0 * s, 6.0 * s);
    for (x, y, dx, dy) in [(a, a, 1.0, 1.0), (b, a, -1.0, 1.0), (a, b, 1.0, -1.0), (b, b, -1.0, -1.0)] {
        let mut corner = PathBuilder::new();
        corner.move_to(x + dx * l, y);
        corner.line_to(x, y);
        corner.line_to(x, y + dy * l);
        pixmap.stroke_path(&corner.finish().expect("open path"), &paint, &stroke, Transform::identity(), None);
    }
    // Shutter dot.
    let mut dot = PathBuilder::new();
    dot.push_circle(16.0 * s, 16.0 * s, 3.2 * s);
    paint.set_color_rgba8(0xFF, 0xC1, 0x07, 0xFF);
    pixmap.fill_path(&dot.finish().expect("circle"), &paint, FillRule::Winding, Transform::identity(), None);
    pixmap
}

/// The sizes in the program's `.ico`: Explorer, the taskbar and the title
/// bar each pick the nearest.
pub(crate) const ICO_SIZES: [u32; 9] = [16, 20, 24, 32, 40, 48, 64, 128, 256];

/// The icon as a `.ico` file of PNG images, one per [`ICO_SIZES`].
pub(crate) fn ico() -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = ICO_SIZES.iter().map(|&s| (s, draw(s).encode_png().expect("icon encodes"))).collect();
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in &images {
        // 256 is written as 0.
        let side = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[side, side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in &images {
        out.extend_from_slice(png);
    }
    out
}

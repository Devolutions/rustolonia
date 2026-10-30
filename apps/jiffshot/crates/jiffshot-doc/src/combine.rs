//! Combining images into one document, and layout templates. Images
//! become image objects and captions become text, so everything stays
//! editable; template pictures are drop zones that images can be dropped
//! into, filled or fitted, panned and swapped.

use crate::color::Rgba;
use crate::document::Document;
use crate::editor::{default_style, Tool};
use crate::geom::{Box2, Pt};
use crate::object::{ImageRef, Kind, ShapeKind, Style, Zone};
use crate::text::Align;
use tiny_skia::Pixmap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrangement {
    Horizontal,
    Vertical,
    Grid,
}

/// Places `images` side by side, stacked, or in a near-square grid with
/// `gap` pixels between them on a `background` canvas.
pub fn combine(images: &[Pixmap], arrangement: Arrangement, gap: f32, background: Rgba) -> Option<Document> {
    if images.is_empty() {
        return None;
    }
    let n = images.len();
    let cols = match arrangement {
        Arrangement::Horizontal => n,
        Arrangement::Vertical => 1,
        Arrangement::Grid => (n as f32).sqrt().ceil() as usize,
    };
    let rows = n.div_ceil(cols);
    let cell_w: Vec<f32> = (0..cols).map(|c| images.iter().skip(c).step_by(cols).map(|i| i.width() as f32).fold(0.0, f32::max)).collect();
    let cell_h: Vec<f32> = (0..rows).map(|r| images.iter().skip(r * cols).take(cols).map(|i| i.height() as f32).fold(0.0, f32::max)).collect();
    let width = cell_w.iter().sum::<f32>() + gap * (cols as f32 + 1.0);
    let height = cell_h.iter().sum::<f32>() + gap * (rows as f32 + 1.0);
    let mut doc = Document::blank(width.ceil() as u32, height.ceil() as u32, background);
    let mut y = gap;
    for (r, h) in cell_h.iter().enumerate() {
        let mut x = gap;
        for (c, w) in cell_w.iter().enumerate() {
            if let Some(img) = images.get(r * cols + c) {
                let (iw, ih) = (img.width() as f32, img.height() as f32);
                let rect = Box2::from_xywh(x + (w - iw) / 2.0, y + (h - ih) / 2.0, iw, ih);
                let key = format!("image-{}", r * cols + c + 1);
                doc.add(Kind::Image { rect, image: ImageRef::new(key, img.clone()) }, Style { width: 0.0, ..Style::default() });
            }
            x += w + gap;
        }
        y += h + gap;
    }
    Some(doc)
}

/// Colors and type for a template.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub background: Rgba,
    pub ink: Rgba,
    pub accent: Rgba,
    pub muted: Rgba,
    /// Behind each picture.
    pub frame: Rgba,
    pub font: &'static str,
}

pub const THEMES: &[Theme] = &[
    Theme { name: "Light", background: Rgba::WHITE, ink: Rgba::rgb(0x22, 0x26, 0x2E), accent: Rgba::rgb(0x1E, 0x6F, 0xD9), muted: Rgba::rgb(0x6B, 0x72, 0x80), frame: Rgba::rgb(0xD0, 0xD4, 0xDA), font: "Segoe UI" },
    Theme { name: "Dark", background: Rgba::rgb(0x1C, 0x1E, 0x24), ink: Rgba::rgb(0xF1, 0xF3, 0xF5), accent: Rgba::rgb(0x4D, 0xA3, 0xFF), muted: Rgba::rgb(0x9A, 0xA3, 0xAF), frame: Rgba::rgb(0x3A, 0x3E, 0x46), font: "Segoe UI" },
    Theme { name: "Ocean", background: Rgba::rgb(0xEE, 0xF6, 0xFA), ink: Rgba::rgb(0x0B, 0x2E, 0x45), accent: Rgba::rgb(0x00, 0x8C, 0x9E), muted: Rgba::rgb(0x4C, 0x6E, 0x80), frame: Rgba::rgb(0xB8, 0xD8, 0xE2), font: "Segoe UI" },
    Theme { name: "Warm", background: Rgba::rgb(0xFF, 0xF8, 0xF0), ink: Rgba::rgb(0x3A, 0x22, 0x14), accent: Rgba::rgb(0xE0, 0x6C, 0x1F), muted: Rgba::rgb(0x8A, 0x6A, 0x55), frame: Rgba::rgb(0xEC, 0xD5, 0xC0), font: "Georgia" },
    Theme { name: "Mono", background: Rgba::WHITE, ink: Rgba::BLACK, accent: Rgba::rgb(0x33, 0x33, 0x33), muted: Rgba::rgb(0x77, 0x77, 0x77), frame: Rgba::rgb(0xCC, 0xCC, 0xCC), font: "Consolas" },
];

pub fn theme(name: &str) -> &'static Theme {
    THEMES.iter().find(|t| t.name == name).unwrap_or(&THEMES[0])
}

/// What a template adds around its pictures.
#[derive(Clone, Debug)]
pub struct Options {
    pub theme: &'static Theme,
    /// A caption under each picture.
    pub captions: bool,
    /// A numbered badge on each picture.
    pub numbers: bool,
    /// Text in a band along the bottom.
    pub footer: Option<String>,
}

impl Default for Options {
    fn default() -> Options {
        Options { theme: &THEMES[0], captions: false, numbers: false, footer: None }
    }
}

pub const TEMPLATES: &[&str] = &["Before and after", "Step by step", "Grid with title", "Title and image", "Two sections", "Three columns"];

/// Builds the pieces of a template: text in the theme, and drop zones
/// that take the images in order.
struct Builder<'a> {
    doc: Document,
    images: &'a [Pixmap],
    next: usize,
    options: &'a Options,
}

impl Builder<'_> {
    fn text(&mut self, rect: Box2, text: &str, size: f32, bold: bool, align: Align, color: Rgba) {
        let style = Style { font_size: size, bold, align, text_color: color, width: 0.0, fill: Rgba::TRANSPARENT, font: self.options.theme.font.to_owned(), ..default_style(Tool::Text) };
        self.doc.add(Kind::Text { rect, text: text.to_owned() }, style);
    }

    /// A drop zone holding the next image (if any), with its number and
    /// caption when asked for.
    fn zone(&mut self, area: Box2) {
        let n = self.next + 1;
        let theme = self.options.theme;
        let caption_h = if self.options.captions { 44.0 } else { 0.0 };
        let rect = Box2::new(area.x0, area.y0, area.x1, area.y1 - caption_h);
        let image = match self.images.get(self.next) {
            Some(img) => ImageRef::new(format!("zone-{n}"), img.clone()),
            None => ImageRef { key: String::new(), pixmap: None },
        };
        self.next += 1;
        let style = Style { width: 1.5, stroke: theme.frame, fill: theme.frame.with_alpha(60), shadow: true, ..Style::default() };
        let id = self.doc.add(Kind::Image { rect, image }, style);
        self.doc.get_mut(id).unwrap().zone = Some(Zone::default());
        if self.options.numbers {
            let badge = Style { fill: theme.accent, text_color: Rgba::WHITE, width: 0.0, font_size: 22.0, ..default_style(Tool::Step) };
            self.doc.add(Kind::Step { center: Pt::new(rect.x0 + 22.0, rect.y0 + 22.0), radius: 20.0, label: n.to_string(), shape: ShapeKind::Ellipse }, badge);
        }
        if self.options.captions {
            self.text(Box2::new(area.x0, rect.y1 + 6.0, area.x1, area.y1), &format!("Caption {n}"), 20.0, false, Align::Center, theme.muted);
        }
    }

    fn band(&mut self, rect: Box2, color: Rgba) {
        self.doc.add(Kind::Shape { rect, shape: ShapeKind::Rectangle }, Style { fill: color, width: 0.0, ..Style::default() });
    }

    /// Adds the footer band (the canvas grows to hold it).
    fn finish(mut self) -> Document {
        if let Some(footer) = self.options.footer.clone().filter(|f| !f.trim().is_empty()) {
            let (w, h) = self.doc.size();
            let bg = self.options.theme.background;
            self.doc.resize_canvas(w, h + 56, 0, 0, bg);
            let theme = self.options.theme;
            self.band(Box2::from_xywh(0.0, h as f32, w as f32, 56.0), theme.frame.with_alpha(90));
            self.text(Box2::from_xywh(24.0, h as f32 + 10.0, w as f32 - 48.0, 40.0), &footer, 18.0, false, Align::Center, theme.muted);
        }
        self.doc
    }
}

/// Builds a template layout from `images` with the default options.
pub fn template(name: &str, images: &[Pixmap]) -> Option<Document> {
    template_with(name, images, &Options::default())
}

/// Builds a template layout; pictures beyond `images` are empty drop zones.
pub fn template_with(name: &str, images: &[Pixmap], options: &Options) -> Option<Document> {
    let theme = options.theme;
    let (ink, accent) = (theme.ink, theme.accent);
    let max_w = images.iter().map(|i| i.width()).max().unwrap_or(800).clamp(400, 1600) as f32;
    let max_h = images.iter().map(|i| i.height()).max().unwrap_or(500).clamp(250, 1000) as f32;
    let pad = 40.0;
    let cap = if options.captions { 44.0 } else { 0.0 };
    let new = |w: f32, h: f32| Document::blank(w as u32, h as u32, theme.background);
    let b = |doc: Document| Builder { doc, images, next: 0, options };
    let doc = match name {
        "Before and after" => {
            let (w, h) = (pad * 3.0 + max_w * 2.0, pad * 2.0 + 70.0 + max_h + cap);
            let mut t = b(new(w, h));
            for (i, label) in ["Before", "After"].iter().enumerate() {
                let x = pad + i as f32 * (max_w + pad);
                t.text(Box2::from_xywh(x, pad * 0.6, max_w, 50.0), label, 34.0, true, Align::Center, if i == 0 { ink } else { accent });
                t.zone(Box2::from_xywh(x, pad + 60.0, max_w, max_h + cap));
            }
            t.finish()
        }
        "Step by step" => {
            let n = images.len().max(3);
            let row = max_h.min(420.0) + pad + cap;
            let shot_w = max_w.min(1000.0);
            let (w, h) = (pad * 3.0 + 80.0 + shot_w + 360.0, pad + row * n as f32);
            let mut t = b(new(w, h));
            for i in 0..n {
                let y = pad + i as f32 * row;
                let step = Style { fill: accent, text_color: Rgba::WHITE, width: 0.0, ..default_style(Tool::Step) };
                t.doc.add(Kind::Step { center: Pt::new(pad + 30.0, y + 30.0), radius: 26.0, label: (i + 1).to_string(), shape: ShapeKind::Ellipse }, step);
                let area = Box2::from_xywh(pad * 2.0 + 60.0, y, shot_w, row - pad);
                t.zone(area);
                t.text(Box2::from_xywh(area.x1 + pad, y + 8.0, 340.0, 80.0), &format!("Describe step {}", i + 1), 26.0, false, Align::Left, ink);
            }
            t.finish()
        }
        "Grid with title" => {
            let cell = (max_w.min(900.0), max_h.min(600.0) + cap);
            let (w, h) = (pad * 3.0 + cell.0 * 2.0, pad * 3.0 + 90.0 + cell.1 * 2.0);
            let mut t = b(new(w, h));
            t.text(Box2::from_xywh(pad, pad * 0.5, w - 2.0 * pad, 70.0), "Title", 48.0, true, Align::Center, ink);
            for i in 0..4 {
                let (c, r) = ((i % 2) as f32, (i / 2) as f32);
                t.zone(Box2::from_xywh(pad + c * (cell.0 + pad), pad + 90.0 + r * (cell.1 + pad), cell.0, cell.1));
            }
            t.finish()
        }
        "Title and image" => {
            let (w, h) = (pad * 2.0 + max_w, pad * 2.0 + 120.0 + max_h + cap);
            let mut t = b(new(w, h));
            t.band(Box2::from_xywh(0.0, 0.0, w, 100.0), accent);
            t.text(Box2::from_xywh(pad, 18.0, w - 2.0 * pad, 64.0), "Title", 44.0, true, Align::Left, Rgba::WHITE);
            t.zone(Box2::from_xywh(pad, 100.0 + pad, max_w, max_h + cap));
            t.finish()
        }
        "Two sections" => {
            // A heading per section with two pictures under it.
            let cell = (max_w.min(800.0), max_h.min(500.0) + cap);
            let section = 70.0 + cell.1 + pad;
            let (w, h) = (pad * 3.0 + cell.0 * 2.0, pad + section * 2.0);
            let mut t = b(new(w, h));
            for s in 0..2 {
                let y = pad + s as f32 * section;
                t.band(Box2::from_xywh(pad, y + 52.0, w - 2.0 * pad, 3.0), accent);
                t.text(Box2::from_xywh(pad, y, w - 2.0 * pad, 50.0), &format!("Section {}", s + 1), 32.0, true, Align::Left, ink);
                for c in 0..2 {
                    t.zone(Box2::from_xywh(pad + c as f32 * (cell.0 + pad), y + 70.0, cell.0, cell.1));
                }
            }
            t.finish()
        }
        "Three columns" => {
            let cell = (max_w.min(600.0), max_h.min(450.0) + cap);
            let (w, h) = (pad * 4.0 + cell.0 * 3.0, pad * 2.0 + 90.0 + cell.1);
            let mut t = b(new(w, h));
            t.text(Box2::from_xywh(pad, pad * 0.5, w - 2.0 * pad, 70.0), "Title", 44.0, true, Align::Center, ink);
            for c in 0..3 {
                t.zone(Box2::from_xywh(pad + c as f32 * (cell.0 + pad), pad + 90.0, cell.0, cell.1));
            }
            t.finish()
        }
        _ => return None,
    };
    Some(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster;

    #[test]
    fn combines_and_templates() {
        let a = raster::solid(100, 50, Rgba::RED).unwrap();
        let b = raster::solid(60, 80, Rgba::YELLOW).unwrap();
        let d = combine(&[a.clone(), b.clone()], Arrangement::Horizontal, 10.0, Rgba::WHITE).unwrap();
        assert_eq!(d.size(), (190, 100));
        let d = combine(&[a.clone(), b.clone(), a.clone()], Arrangement::Grid, 0.0, Rgba::WHITE).unwrap();
        assert_eq!(d.size(), (160, 130));
        assert_eq!(d.objects.len(), 3);
        for t in TEMPLATES {
            let d = template(t, &[a.clone(), b.clone()]).unwrap();
            assert!(d.objects.iter().filter(|o| o.zone.is_some()).count() >= 1, "{t}");
        }
    }

    #[test]
    fn zones_numbers_captions_footer_and_themes() {
        let a = raster::solid(800, 400, Rgba::RED).unwrap();
        let options = Options { theme: theme("Dark"), captions: true, numbers: true, footer: Some("© Example".into()) };
        let d = template_with("Grid with title", &[a.clone()], &options).unwrap();
        let zones: Vec<_> = d.objects.iter().filter(|o| o.zone.is_some()).collect();
        assert_eq!(zones.len(), 4);
        let filled = zones.iter().filter(|o| matches!(&o.kind, Kind::Image { image, .. } if image.pixmap.is_some())).count();
        assert_eq!(filled, 1, "one image, three empty zones");
        assert_eq!(d.objects.iter().filter(|o| matches!(o.kind, Kind::Step { .. })).count(), 4);
        let texts: Vec<String> = d.objects.iter().filter_map(|o| match &o.kind { Kind::Text { text, .. } => Some(text.clone()), _ => None }).collect();
        assert!(texts.contains(&"Caption 3".to_string()) && texts.contains(&"© Example".to_string()));
        assert_eq!(raster::pixel(d.background(), 2, 2), theme("Dark").background);
        // The first zone is filled (cropped): red across its whole box.
        let flat = d.flatten();
        if let Some(out) = std::env::var_os("JIFFSHOT_TEMPLATE_OUT") {
            flat.save_png(out).unwrap();
        }
        let Kind::Image { rect: z, .. } = zones[0].kind.clone() else { unreachable!() };
        let c = raster::pixel(&flat, (z.x0 + 5.0) as i32, (z.center().y) as i32);
        assert!(c.r > 200 && c.g < 60, "{c:?}");
    }

    #[test]
    fn placement_fills_or_fits_and_clamps_the_pan() {
        let rect = Box2::from_xywh(0.0, 0.0, 200.0, 100.0);
        let fill = Zone { fit: false, pan: Pt::new(500.0, 0.0) };
        let (p, pan) = fill.placement(&rect, 100.0, 100.0);
        assert_eq!((p.width(), p.height()), (200.0, 200.0));
        assert_eq!(pan, Pt::new(0.0, 0.0), "no room to pan sideways");
        let fill = Zone { fit: false, pan: Pt::new(0.0, 500.0) };
        assert_eq!(fill.placement(&rect, 100.0, 100.0).1, Pt::new(0.0, 50.0));
        let fit = Zone { fit: true, pan: Pt::default() };
        let (p, _) = fit.placement(&rect, 100.0, 100.0);
        assert_eq!((p.width(), p.height(), p.x0), (100.0, 100.0, 50.0));
    }
}

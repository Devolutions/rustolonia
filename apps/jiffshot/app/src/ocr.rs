//! Text recognition for Grab Text, Text Replace and Simplify, through the
//! Windows OCR engine in `jiffshot-capture`.

use jiffshot_doc::editor::Analyzer;
use jiffshot_doc::smart::TextLine;
use jiffshot_doc::{IRect, Pixmap};

fn bgra(pm: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pm.data().len());
    for p in pm.pixels() {
        // Composite over white so transparent areas read as background.
        let inv = 255 - p.alpha() as u16;
        let c = |v: u8| (v as u16 + inv).min(255) as u8;
        out.extend_from_slice(&[c(p.blue()), c(p.green()), c(p.red()), 255]);
    }
    out
}

/// Lines of text in `pm`, in its pixel coordinates.
pub fn recognize(pm: &Pixmap) -> Result<Vec<TextLine>, String> {
    let lines = jiffshot_capture::ocr::recognize(pm.width(), pm.height(), &bgra(pm))?;
    Ok(lines
        .into_iter()
        .map(|l| TextLine {
            area: IRect::new(l.rect.x, l.rect.y, l.rect.width, l.rect.height),
            text: l.text,
            words: l.words.into_iter().map(|w| (IRect::new(w.rect.x, w.rect.y, w.rect.width, w.rect.height), w.text)).collect(),
        })
        .collect())
}

/// The editor's hook for Text Replace.
pub struct Recognizer;

impl Analyzer for Recognizer {
    fn text_in(&mut self, image: &Pixmap, area: IRect) -> Vec<TextLine> {
        let Some(area) = area.intersect(&IRect::of_size(image.width(), image.height())) else { return Vec::new() };
        let Some(part) = jiffshot_doc::raster::extract(image, area) else { return Vec::new() };
        recognize(&part)
            .unwrap_or_default()
            .into_iter()
            .map(|mut l| {
                l.area.x += area.x;
                l.area.y += area.y;
                for (r, _) in &mut l.words {
                    r.x += area.x;
                    r.y += area.y;
                }
                l
            })
            .collect()
    }
}

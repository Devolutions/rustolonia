//! Text layout and rasterization with system fonts (`ab_glyph`), plus
//! per-character fallback fonts for symbols and CJK.

use crate::color::Rgba;
use crate::raster::blend;
use ab_glyph::{Font, FontArc, GlyphId, PxScale, ScaleFont};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tiny_skia::Pixmap;

/// Families offered in the font list. Others fall back to the default.
pub const FAMILIES: &[&str] = &[
    "Segoe UI",
    "Arial",
    "Calibri",
    "Cambria",
    "Comic Sans MS",
    "Consolas",
    "Courier New",
    "Georgia",
    "Impact",
    "Tahoma",
    "Times New Roman",
    "Trebuchet MS",
    "Verdana",
];

pub const DEFAULT_FAMILY: &str = "Segoe UI";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// Regular, bold, italic and bold-italic file names for a family.
fn files(family: &str) -> Option<[&'static str; 4]> {
    Some(match family.to_ascii_lowercase().as_str() {
        "segoe ui" => ["segoeui.ttf", "segoeuib.ttf", "segoeuii.ttf", "segoeuiz.ttf"],
        "arial" => ["arial.ttf", "arialbd.ttf", "ariali.ttf", "arialbi.ttf"],
        "calibri" => ["calibri.ttf", "calibrib.ttf", "calibrii.ttf", "calibriz.ttf"],
        "cambria" => ["cambria.ttc", "cambriab.ttf", "cambriai.ttf", "cambriaz.ttf"],
        "comic sans ms" => ["comic.ttf", "comicbd.ttf", "comici.ttf", "comicz.ttf"],
        "consolas" => ["consola.ttf", "consolab.ttf", "consolai.ttf", "consolaz.ttf"],
        "courier new" => ["cour.ttf", "courbd.ttf", "couri.ttf", "courbi.ttf"],
        "georgia" => ["georgia.ttf", "georgiab.ttf", "georgiai.ttf", "georgiaz.ttf"],
        "impact" => ["impact.ttf", "impact.ttf", "impact.ttf", "impact.ttf"],
        "tahoma" => ["tahoma.ttf", "tahomabd.ttf", "tahoma.ttf", "tahomabd.ttf"],
        "times new roman" => ["times.ttf", "timesbd.ttf", "timesi.ttf", "timesbi.ttf"],
        "trebuchet ms" => ["trebuc.ttf", "trebucbd.ttf", "trebucit.ttf", "trebucbi.ttf"],
        "verdana" => ["verdana.ttf", "verdanab.ttf", "verdanai.ttf", "verdanaz.ttf"],
        // macOS families in imported files, mapped to the closest Windows font.
        "helvetica" | "helvetica neue" => return files("arial"),
        "proxima nova" | "sf pro" | "sf pro text" | ".sf ns" | "avenir" | "avenir next" => return files("segoe ui"),
        "menlo" | "monaco" | "sf mono" => return files("consolas"),
        // Common Linux fonts, so tests and exports work off Windows too.
        "dejavu sans" => ["DejaVuSans.ttf", "DejaVuSans-Bold.ttf", "DejaVuSans-Oblique.ttf", "DejaVuSans-BoldOblique.ttf"],
        _ => return None,
    })
}

const FALLBACK_FILES: &[&str] = &["seguisym.ttf", "msyh.ttc", "YuGothR.ttc", "malgun.ttf", "Nirmala.ttc", "seguiemj.ttf", "DejaVuSans.ttf"];

fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("Microsoft").join("Windows").join("Fonts"));
    }
    for d in ["/usr/share/fonts/truetype/dejavu", "/usr/share/fonts/TTF", "/usr/share/fonts", "/Library/Fonts", "/System/Library/Fonts"] {
        dirs.push(PathBuf::from(d));
    }
    dirs
}

fn load_file(name: &str) -> Option<FontArc> {
    font_dirs().into_iter().find_map(|dir| {
        let bytes = std::fs::read(dir.join(name)).ok()?;
        FontArc::try_from_vec(bytes).ok()
    })
}

type Cache = Mutex<HashMap<(String, bool, bool), Option<FontArc>>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Loads a family's face, falling back to regular, then to the default
/// family, then to any fallback font. `None` only if no font exists at all.
pub fn font(family: &str, bold: bool, italic: bool) -> Option<FontArc> {
    let key = (family.to_ascii_lowercase(), bold, italic);
    if let Some(found) = cache().lock().unwrap().get(&key) {
        return found.clone();
    }
    let index = bold as usize + 2 * italic as usize;
    let found = files(family)
        .and_then(|f| load_file(f[index]).or_else(|| load_file(f[0])))
        .or_else(|| (family != DEFAULT_FAMILY).then(|| font(DEFAULT_FAMILY, bold, italic)).flatten())
        .or_else(|| font_from_list(&["DejaVuSans.ttf", "arial.ttf"]))
        .or_else(|| fallbacks().first().cloned());
    cache().lock().unwrap().insert(key, found.clone());
    found
}

fn font_from_list(names: &[&str]) -> Option<FontArc> {
    names.iter().find_map(|n| load_file(n))
}

fn fallbacks() -> &'static [FontArc] {
    static FALLBACKS: OnceLock<Vec<FontArc>> = OnceLock::new();
    FALLBACKS.get_or_init(|| FALLBACK_FILES.iter().filter_map(|n| load_file(n)).collect())
}

/// One positioned glyph: the font it comes from (index into
/// [`Layout::fonts`]), its id and its x offset within the line.
#[derive(Clone, Debug)]
struct Placed {
    font: usize,
    id: GlyphId,
    x: f32,
}

#[derive(Clone, Debug, Default)]
struct Line {
    glyphs: Vec<Placed>,
    width: f32,
}

/// Laid-out text: lines of glyphs at a pixel size.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    fonts: Vec<FontArc>,
    scales: Vec<PxScale>,
    lines: Vec<Line>,
    ascent: f32,
    line_height: f32,
}

impl Layout {
    pub fn width(&self) -> f32 {
        self.lines.iter().map(|l| l.width).fold(0.0, f32::max)
    }

    pub fn height(&self) -> f32 {
        self.lines.len() as f32 * self.line_height
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

#[derive(Clone, Debug)]
pub struct TextStyle<'a> {
    pub family: &'a str,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}

/// Lays `text` out, wrapping at `max_width` when given. Words longer than
/// a line are broken between characters.
pub fn layout(text: &str, style: &TextStyle, max_width: Option<f32>) -> Layout {
    let Some(primary) = font(style.family, style.bold, style.italic) else {
        return Layout::default();
    };
    let mut fonts = vec![primary];
    fonts.extend(fallbacks().iter().cloned());
    // Font sizes are em sizes; ab_glyph scales by ascent-to-descent height.
    let scales: Vec<PxScale> = fonts.iter().map(|f| em_scale(f, style.size)).collect();
    let s0 = fonts[0].as_scaled(scales[0]);
    let ascent = s0.ascent();
    let line_height = (s0.ascent() - s0.descent() + s0.line_gap()).max(style.size);

    let pick = |c: char| -> (usize, GlyphId) {
        for (i, f) in fonts.iter().enumerate() {
            let id = f.glyph_id(c);
            if id.0 != 0 {
                return (i, id);
            }
        }
        (0, fonts[0].glyph_id(c))
    };
    let advance = |f: usize, id: GlyphId| fonts[f].as_scaled(scales[f]).h_advance(id);
    let max = max_width.unwrap_or(f32::INFINITY).max(style.size);

    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let paragraph = paragraph.trim_end_matches('\r');
        let mut line = Line::default();
        let mut prev: Option<(usize, GlyphId)> = None;
        // Split into words that keep their trailing spaces.
        let mut words: Vec<&str> = Vec::new();
        let mut start = 0;
        for (i, c) in paragraph.char_indices() {
            if c == ' ' {
                let next = i + 1;
                if paragraph[next..].starts_with(' ') {
                    continue;
                }
                words.push(&paragraph[start..next]);
                start = next;
            }
        }
        if start < paragraph.len() {
            words.push(&paragraph[start..]);
        }
        for word in words {
            let glyphs: Vec<(usize, GlyphId, f32)> = word
                .chars()
                .map(|c| {
                    let (f, id) = pick(c);
                    (f, id, advance(f, id))
                })
                .collect();
            let visible: f32 = word.trim_end().chars().zip(&glyphs).map(|(_, g)| g.2).sum();
            if line.width > 0.0 && line.width + visible > max {
                lines.push(std::mem::take(&mut line));
                prev = None;
            }
            for (f, id, adv) in glyphs {
                if line.width > 0.0 && line.width + adv > max && word.trim_end().len() > 1 && visible > max {
                    lines.push(std::mem::take(&mut line));
                    prev = None;
                }
                let kern = match prev {
                    Some((pf, pid)) if pf == f => fonts[f].as_scaled(scales[f]).kern(pid, id),
                    _ => 0.0,
                };
                line.glyphs.push(Placed { font: f, id, x: line.width + kern });
                line.width += adv + kern;
                prev = Some((f, id));
            }
        }
        // Trailing spaces do not count toward alignment.
        let trailing: f32 = paragraph.chars().rev().take_while(|c| *c == ' ').map(|c| advance(pick(c).0, pick(c).1)).sum();
        if line.glyphs.len() > 0 && trailing > 0.0 && trailing < line.width {
            line.width -= trailing;
        }
        lines.push(line);
    }
    Layout { fonts, scales, lines, ascent, line_height }
}

/// The ab_glyph scale that renders `font` at an em size of `size` pixels.
fn em_scale(font: &FontArc, size: f32) -> PxScale {
    let em = font.units_per_em().unwrap_or(1000.0);
    PxScale::from(size.max(1.0) * font.height_unscaled() / em)
}

/// For each line: its left edge, width and baseline, as `draw` places it.
pub fn line_extents(layout: &Layout, x: f32, y: f32, box_width: f32, align: Align) -> Vec<(f32, f32, f32)> {
    layout
        .lines
        .iter()
        .enumerate()
        .map(|(row, line)| {
            let dx = match align {
                Align::Left => 0.0,
                Align::Center => (box_width - line.width) / 2.0,
                Align::Right => box_width - line.width,
            };
            (x + dx, line.width, y + layout.ascent + row as f32 * layout.line_height)
        })
        .collect()
}

/// Draws `layout` with its top-left at (`x`, `y`) in document pixels, each
/// line aligned within `box_width`. `origin` is the document position of the
/// target pixmap's top-left pixel.
pub fn draw(target: &mut Pixmap, origin: (i32, i32), layout: &Layout, x: f32, y: f32, box_width: f32, align: Align, color: Rgba) {
    for (row, line) in layout.lines.iter().enumerate() {
        let dx = match align {
            Align::Left => 0.0,
            Align::Center => (box_width - line.width) / 2.0,
            Align::Right => box_width - line.width,
        };
        let baseline = y + layout.ascent + row as f32 * layout.line_height - origin.1 as f32;
        for g in &line.glyphs {
            let font = &layout.fonts[g.font];
            let glyph = g.id.with_scale_and_position(layout.scales[g.font], ab_glyph::point(x + dx + g.x - origin.0 as f32, baseline));
            if let Some(outlined) = font.outline_glyph(glyph) {
                let b = outlined.px_bounds();
                let (bx, by) = (b.min.x as i32, b.min.y as i32);
                outlined.draw(|px, py, coverage| blend(target, bx + px as i32, by + py as i32, color, coverage));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(size: f32) -> TextStyle<'static> {
        TextStyle { family: DEFAULT_FAMILY, size, bold: false, italic: false }
    }

    #[test]
    fn wraps_words_and_measures() {
        if font(DEFAULT_FAMILY, false, false).is_none() {
            return; // No fonts on this machine.
        }
        let one = layout("Hello world", &style(20.0), None);
        assert_eq!(one.line_count(), 1);
        assert!(one.width() > 60.0);
        let wrapped = layout("Hello world", &style(20.0), Some(one.width() * 0.7));
        assert_eq!(wrapped.line_count(), 2);
        let lines = layout("a\nb\n", &style(20.0), None);
        assert_eq!(lines.line_count(), 3);
        let mut p = Pixmap::new(200, 40).unwrap();
        draw(&mut p, (0, 0), &one, 0.0, 0.0, 200.0, Align::Left, Rgba::BLACK);
        assert!(p.data().chunks(4).any(|px| px[3] > 200), "glyphs were drawn");
    }
}

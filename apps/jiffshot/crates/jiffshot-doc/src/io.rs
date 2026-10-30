//! Reading and writing images and editable documents.
//!
//! The native `.jsd` format is a zip archive with `document.json` (canvas
//! size and objects), `background.png`, and one PNG per image object under
//! `images/`. Image exports flatten the annotations.

use crate::color::Rgba;
use crate::document::Document;
use crate::object::{Kind, Object};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::sync::Arc;
use tiny_skia::Pixmap;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
    Bmp,
    Gif,
    Webp,
    /// A one-page PDF holding the image as JPEG.
    Pdf,
}

impl Format {
    pub fn from_path(path: &Path) -> Option<Format> {
        Some(match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "png" => Format::Png,
            "jpg" | "jpeg" => Format::Jpeg,
            "bmp" => Format::Bmp,
            "gif" => Format::Gif,
            "webp" => Format::Webp,
            "pdf" => Format::Pdf,
            _ => return None,
        })
    }
}

/// Extensions `open` understands.
pub const OPEN_EXTENSIONS: &[&str] = &["jsd", "png", "jpg", "jpeg", "bmp", "gif", "webp"];

/// Whether `path` is a Jiffshot document (`.jsd`).
pub fn is_document(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("jsd"))
}

pub fn to_rgba_image(pm: &Pixmap) -> image::RgbaImage {
    let mut out = image::RgbaImage::new(pm.width(), pm.height());
    for (dst, src) in out.pixels_mut().zip(pm.pixels()) {
        let c = src.demultiply();
        *dst = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}

pub fn from_rgba_image(img: &image::RgbaImage) -> Option<Pixmap> {
    let mut pm = Pixmap::new(img.width(), img.height())?;
    for (dst, src) in pm.pixels_mut().iter_mut().zip(img.pixels()) {
        *dst = Rgba::new(src[0], src[1], src[2], src[3]).premultiplied();
    }
    Some(pm)
}

pub fn decode_image(bytes: &[u8]) -> Result<Pixmap> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    from_rgba_image(&img.to_rgba8()).ok_or_else(|| "image has no pixels".to_owned())
}

pub fn load_image(path: &Path) -> Result<Pixmap> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_image(&bytes)
}

/// Encodes `pm`. Formats without transparency get it flattened onto white.
pub fn encode(pm: &Pixmap, format: Format, jpeg_quality: u8) -> Result<Vec<u8>> {
    let rgba = to_rgba_image(pm);
    let mut out = Cursor::new(Vec::new());
    let flat = || {
        let mut rgb = image::RgbImage::new(rgba.width(), rgba.height());
        for (d, s) in rgb.pixels_mut().zip(rgba.pixels()) {
            let a = s[3] as u32;
            let f = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
            *d = image::Rgb([f(s[0]), f(s[1]), f(s[2])]);
        }
        rgb
    };
    match format {
        Format::Png => {
            let enc = image::codecs::png::PngEncoder::new_with_quality(&mut out, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::Adaptive);
            image::ImageEncoder::write_image(enc, rgba.as_raw(), rgba.width(), rgba.height(), image::ExtendedColorType::Rgba8)
        }
        Format::Jpeg => {
            let rgb = flat();
            let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, jpeg_quality.clamp(1, 100));
            image::ImageEncoder::write_image(enc, rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)
        }
        Format::Bmp => {
            let rgb = flat();
            image::DynamicImage::ImageRgb8(rgb).write_to(&mut out, image::ImageFormat::Bmp)
        }
        Format::Gif => image::DynamicImage::ImageRgba8(rgba).write_to(&mut out, image::ImageFormat::Gif),
        Format::Webp => image::DynamicImage::ImageRgba8(rgba).write_to(&mut out, image::ImageFormat::WebP),
        Format::Pdf => return pdf(pm, jpeg_quality.max(85)),
    }
    .map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

/// A minimal one-page PDF: the image as a JPEG stream on a page of its
/// size (96 pixels to the inch).
pub fn pdf(pm: &Pixmap, quality: u8) -> Result<Vec<u8>> {
    let jpeg = encode(pm, Format::Jpeg, quality)?;
    let (w, h) = (pm.width(), pm.height());
    let (pw, ph) = (w as f32 * 0.75, h as f32 * 0.75);
    let content = format!("q {pw:.2} 0 0 {ph:.2} 0 0 cm /Im0 Do Q");
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    let mut object = |out: &mut Vec<u8>, body: &[u8]| {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };
    object(&mut out, b"<< /Type /Catalog /Pages 2 0 R >>");
    object(&mut out, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    object(&mut out, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.2} {ph:.2}] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>").as_bytes());
    let mut image = format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", jpeg.len()).into_bytes();
    image.extend_from_slice(&jpeg);
    image.extend_from_slice(b"\nendstream");
    object(&mut out, &image);
    object(&mut out, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).as_bytes());
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for o in &offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1).as_bytes());
    Ok(out)
}

/// Writes the flattened document as an image, the format chosen by the
/// file extension (PNG when unknown).
pub fn export(doc: &Document, path: &Path) -> Result<()> {
    let format = Format::from_path(path).unwrap_or(Format::Png);
    let bytes = encode(&doc.flatten(), format, 90)?;
    write_atomic(path, &bytes)
}

/// Writes through a temporary file so a failed save never truncates the
/// previous version.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp~");
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    width: u32,
    height: u32,
    objects: Vec<Object>,
}

const FORMAT: &str = "jiffshot-document";

pub fn document_bytes(doc: &Document) -> Result<Vec<u8>> {
    let manifest = Manifest { format: FORMAT.to_owned(), version: 1, width: doc.width(), height: doc.height(), objects: doc.objects.clone() };
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let json = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    zip.start_file("document.json", deflated).map_err(|e| e.to_string())?;
    zip.write_all(&json).map_err(|e| e.to_string())?;
    zip.start_file("background.png", stored).map_err(|e| e.to_string())?;
    zip.write_all(&encode(doc.background(), Format::Png, 0)?).map_err(|e| e.to_string())?;
    let mut images: Vec<(String, Arc<Pixmap>)> = Vec::new();
    for o in &doc.objects {
        o.walk(&mut |o| {
            if let Kind::Image { image, .. } = &o.kind {
                if let Some(pm) = &image.pixmap {
                    if !images.iter().any(|(k, _)| *k == image.key) {
                        images.push((image.key.clone(), pm.clone()));
                    }
                }
            }
        });
    }
    for (key, pm) in images {
        zip.start_file(format!("images/{key}.png"), stored).map_err(|e| e.to_string())?;
        zip.write_all(&encode(&pm, Format::Png, 0)?).map_err(|e| e.to_string())?;
    }
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

pub fn save_document(doc: &Document, path: &Path) -> Result<()> {
    write_atomic(path, &document_bytes(doc)?)
}

fn entry(zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>, name: &str) -> Result<Vec<u8>> {
    let mut f = zip.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn document_from_bytes(bytes: Vec<u8>) -> Result<Document> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let manifest: Manifest = serde_json::from_slice(&entry(&mut zip, "document.json")?).map_err(|e| e.to_string())?;
    if manifest.format != FORMAT {
        return Err(format!("not a Jiffshot document ({})", manifest.format));
    }
    let background = decode_image(&entry(&mut zip, "background.png")?)?;
    let mut doc = Document::new(background);
    let mut cache: std::collections::HashMap<String, Arc<Pixmap>> = Default::default();
    for mut o in manifest.objects {
        let mut failure = None;
        o.walk_mut(&mut |o| {
            if let Kind::Image { image, .. } = &mut o.kind {
                if image.key.is_empty() {
                    return;
                }
                if !cache.contains_key(&image.key) {
                    match entry(&mut zip, &format!("images/{}.png", image.key)).and_then(|b| decode_image(&b)) {
                        Ok(px) => {
                            cache.insert(image.key.clone(), Arc::new(px));
                        }
                        Err(e) => failure = Some(e),
                    }
                }
                image.pixmap = cache.get(&image.key).cloned();
            }
        });
        if let Some(e) = failure {
            return Err(e);
        }
        doc.objects.push(o);
    }
    doc.renumber_from_objects();
    Ok(doc)
}

pub fn load_document(path: &Path) -> Result<Document> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    document_from_bytes(bytes)
}

/// Opens any supported file as a document.
pub fn open(path: &Path) -> Result<Document> {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("jsd") => load_document(path),
        _ => load_image(path).map(Document::new),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Box2, Pt};
    use crate::object::{ImageRef, Style};

    #[test]
    fn documents_round_trip() {
        let mut d = Document::blank(64, 32, Rgba::rgb(10, 20, 30));
        d.add(Kind::Line { a: Pt::new(1.0, 2.0), b: Pt::new(30.0, 20.0), control: None }, Style::default());
        d.add(Kind::Image { rect: Box2::new(5.0, 5.0, 15.0, 15.0), image: ImageRef::new("a", crate::raster::solid(4, 4, Rgba::RED).unwrap()) }, Style::default());
        let bytes = document_bytes(&d).unwrap();
        let mut back = document_from_bytes(bytes).unwrap();
        assert_eq!(back.size(), (64, 32));
        assert_eq!(back.objects, d.objects);
        assert_eq!(back.background().data(), d.background().data());
        let Kind::Image { image, .. } = &back.objects[1].kind else { panic!() };
        assert_eq!(image.pixmap.as_ref().unwrap().width(), 4);
        assert_eq!(back.add(Kind::Text { rect: Box2::default(), text: String::new() }, Style::default()), 3);
    }

    #[test]
    fn encodes_every_format() {
        let pm = crate::raster::solid(8, 8, Rgba::RED.with_alpha(128)).unwrap();
        let pdf = encode(&pm, Format::Pdf, 90).unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4") && pdf.ends_with(b"%%EOF\n"));
        for f in [Format::Png, Format::Jpeg, Format::Bmp, Format::Gif, Format::Webp] {
            let bytes = encode(&pm, f, 85).unwrap();
            let back = decode_image(&bytes).unwrap();
            assert_eq!((back.width(), back.height()), (8, 8), "{f:?}");
        }
    }
}

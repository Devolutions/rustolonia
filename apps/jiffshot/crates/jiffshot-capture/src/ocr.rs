//! Text recognition with the Windows OCR engine (`Windows.Media.Ocr`), in
//! the user's profile languages. Runs on its own multithreaded-apartment
//! thread, so callers on an STA (the UI thread) can block on it safely.

use crate::Rect;
use windows::Foundation::Rect as WinRect;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub rect: Rect,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub rect: Rect,
    pub text: String,
    pub words: Vec<Word>,
}

/// Images smaller than this are enlarged first: the engine misses text
/// below roughly 10 pixels high.
const UPSCALE_BELOW: u32 = 1200;

/// Recognizes text in a top-down BGRA image. Coordinates in the result are
/// pixels of that image.
pub fn recognize(width: u32, height: u32, bgra: &[u8]) -> Result<Vec<Line>, String> {
    if width == 0 || height == 0 || bgra.len() < (width * height * 4) as usize {
        return Ok(Vec::new());
    }
    let pixels = bgra.to_vec();
    std::thread::spawn(move || run(width, height, pixels)).join().map_err(|_| "text recognition crashed".to_owned())?
}

fn upscale(width: u32, height: u32, bgra: &[u8], k: u32) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    let mut out = vec![0u8; w * h * 4 * (k * k) as usize];
    let nw = w * k as usize;
    for y in 0..h * k as usize {
        let sy = y / k as usize;
        for x in 0..nw {
            let sx = x / k as usize;
            let s = (sy * w + sx) * 4;
            out[(y * nw + x) * 4..(y * nw + x) * 4 + 4].copy_from_slice(&bgra[s..s + 4]);
        }
    }
    out
}

fn run(width: u32, height: u32, bgra: Vec<u8>) -> Result<Vec<Line>, String> {
    let e = |error: windows::core::Error| error.message().to_string();
    // SAFETY: initializes WinRT on this fresh thread only.
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| "no OCR language is installed (Settings › Time & language › Language)".to_owned())?;
    let max = OcrEngine::MaxImageDimension().map_err(e)?;
    let k = if width.max(height) < UPSCALE_BELOW && width.max(height) * 2 <= max { 2 } else { 1 };
    let (w, h, pixels) = if k > 1 { (width * k, height * k, upscale(width, height, &bgra, k)) } else { (width, height, bgra) };
    // Tile tall or wide images (scrolling captures) to the engine's limit.
    let mut lines = Vec::new();
    let tile_h = h.min(max);
    let tile_w = w.min(max);
    let mut ty = 0;
    while ty < h {
        let th = tile_h.min(h - ty);
        let mut tx = 0;
        while tx < w {
            let tw = tile_w.min(w - tx);
            let mut tile = Vec::with_capacity((tw * th * 4) as usize);
            for row in ty..ty + th {
                let start = ((row * w + tx) * 4) as usize;
                tile.extend_from_slice(&pixels[start..start + (tw * 4) as usize]);
            }
            let writer = DataWriter::new().map_err(e)?;
            writer.WriteBytes(&tile).map_err(e)?;
            let buffer = writer.DetachBuffer().map_err(e)?;
            let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, tw as i32, th as i32).map_err(e)?;
            let result = engine.RecognizeAsync(&bitmap).map_err(e)?.join().map_err(e)?;
            let map = |r: WinRect| {
                let x0 = ((r.X + tx as f32) / k as f32).floor() as i32;
                let y0 = ((r.Y + ty as f32) / k as f32).floor() as i32;
                let x1 = ((r.X + r.Width + tx as f32) / k as f32).ceil() as i32;
                let y1 = ((r.Y + r.Height + ty as f32) / k as f32).ceil() as i32;
                Rect::from_ltrb(x0, y0, x1, y1)
            };
            for line in result.Lines().map_err(e)? {
                let mut words = Vec::new();
                for word in line.Words().map_err(e)? {
                    words.push(Word { rect: map(word.BoundingRect().map_err(e)?), text: word.Text().map_err(e)?.to_string() });
                }
                let Some(first) = words.first() else { continue };
                let rect = words.iter().skip(1).fold(first.rect, |a, w| a.union(w.rect));
                lines.push(Line { rect, text: line.Text().map_err(e)?.to_string(), words });
            }
            tx += tw;
        }
        ty += th;
    }
    Ok(lines)
}

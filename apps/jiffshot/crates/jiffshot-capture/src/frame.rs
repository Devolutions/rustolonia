use crate::Rect;
use std::io::{self, Write};
use std::path::Path;

/// A captured image: top-down BGRX rows (the GDI layout). The fourth byte
/// is ignored and the image is opaque, unless `alpha` is set (menu
/// captures): then it is straight alpha.
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub alpha: bool,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Frame {
        Frame { width, height, pixels: vec![0; width as usize * height as usize * 4], alpha: false }
    }

    pub fn stride(&self) -> usize {
        self.width as usize * 4
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width as i32, self.height as i32)
    }

    /// Returns `(r, g, b)` at a pixel, or `None` outside the frame.
    pub fn rgb(&self, x: i32, y: i32) -> Option<(u8, u8, u8)> {
        if !self.bounds().contains(x, y) {
            return None;
        }
        let at = y as usize * self.stride() + x as usize * 4;
        let p = &self.pixels[at..at + 4];
        Some((p[2], p[1], p[0]))
    }

    /// Reads a PNG (any bit depth or color type) as an opaque frame.
    pub fn load_png(path: &Path) -> io::Result<Frame> {
        let file = std::io::BufReader::new(std::fs::File::open(path)?);
        let mut decoder = png::Decoder::new(file);
        decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
        let mut reader = decoder.read_info().map_err(io::Error::other)?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(io::Error::other)?;
        let mut frame = Frame::new(info.width, info.height);
        let channels = info.color_type.samples();
        for (dst, src) in frame.pixels.chunks_exact_mut(4).zip(buf.chunks_exact(channels)) {
            let (r, g, b) = if channels >= 3 { (src[0], src[1], src[2]) } else { (src[0], src[0], src[0]) };
            dst.copy_from_slice(&[b, g, r, 255]);
        }
        Ok(frame)
    }

    /// Copies out `r` (frame coordinates), clipped to the frame.
    pub fn crop(&self, r: Rect) -> Option<Frame> {
        let r = r.intersect(self.bounds())?;
        let mut out = Frame::new(r.width as u32, r.height as u32);
        out.alpha = self.alpha;
        let row = r.width as usize * 4;
        for y in 0..r.height as usize {
            let from = (r.y as usize + y) * self.stride() + r.x as usize * 4;
            out.pixels[y * row..(y + 1) * row].copy_from_slice(&self.pixels[from..from + row]);
        }
        Some(out)
    }

    /// Writes an RGB (or RGBA) PNG. Captures favour encode speed over size:
    /// a 4K screenshot must save in well under a second.
    pub fn encode_png(&self, out: impl Write) -> io::Result<()> {
        let mut encoder = png::Encoder::new(out, self.width, self.height);
        encoder.set_color(if self.alpha { png::ColorType::Rgba } else { png::ColorType::Rgb });
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        encoder.set_filter(png::FilterType::Paeth);
        encoder.set_adaptive_filter(png::AdaptiveFilterType::NonAdaptive);
        let mut writer = encoder.write_header().map_err(io::Error::other)?;
        let mut rgb = Vec::with_capacity(self.width as usize * self.height as usize * 4);
        for px in self.pixels.chunks_exact(4) {
            rgb.extend_from_slice(&[px[2], px[1], px[0]]);
            if self.alpha {
                rgb.push(px[3]);
            }
        }
        writer.write_image_data(&rgb).map_err(io::Error::other)?;
        writer.finish().map_err(io::Error::other)
    }

    pub fn encode_png_vec(&self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.encode_png(&mut bytes)?;
        Ok(bytes)
    }

    pub fn save_png(&self, path: &Path) -> io::Result<()> {
        let file = std::fs::File::create(path)?;
        self.encode_png(io::BufWriter::with_capacity(1 << 20, file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(width: u32, height: u32) -> Frame {
        let mut frame = Frame::new(width, height);
        for (i, px) in frame.pixels.chunks_exact_mut(4).enumerate() {
            let (x, y) = (i as u32 % width, i as u32 / width);
            px.copy_from_slice(&[x as u8, y as u8, 7, 0]);
        }
        frame
    }

    #[test]
    fn crop_clips_and_copies_rows() {
        let frame = gradient(10, 8);
        let cropped = frame.crop(Rect::new(6, 5, 10, 10)).unwrap();
        assert_eq!((cropped.width, cropped.height), (4, 3));
        assert_eq!(cropped.rgb(0, 0), Some((7, 5, 6)));
        assert_eq!(cropped.rgb(3, 2), Some((7, 7, 9)));
        assert!(frame.crop(Rect::new(20, 20, 5, 5)).is_none());
    }

    #[test]
    fn png_round_trips_colors() {
        let frame = gradient(33, 17);
        let bytes = frame.encode_png_vec().unwrap();
        let decoder = png::Decoder::new(bytes.as_slice());
        let mut reader = decoder.read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buffer).unwrap();
        assert_eq!((info.width, info.height), (33, 17));
        let at = (4 * 33 + 9) * 3;
        assert_eq!(&buffer[at..at + 3], &[7, 4, 9]);
    }
}

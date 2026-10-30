//! Puts a capture on the clipboard as a 24-bit DIB (understood everywhere)
//! and as PNG (kept lossless by Office, browsers and chat apps), and reads
//! images, files and text back for pasting.

use crate::{wide, Frame};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use windows_sys::Win32::Foundation::{GlobalFree, HANDLE, HWND};
use windows_sys::Win32::Graphics::Gdi::{BITMAPINFOHEADER, BI_RGB};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};

const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;
const CF_DIBV5: u32 = 17;

/// Something on the clipboard the editor can paste.
pub enum Pasted {
    /// Straight-alpha RGBA, top-down.
    Image { width: u32, height: u32, rgba: Vec<u8> },
    Files(Vec<PathBuf>),
    Text(String),
}

fn dib_bytes(frame: &Frame) -> Vec<u8> {
    let row = (frame.width as usize * 3 + 3) & !3;
    let header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: frame.width as i32,
        // Bottom-up: some consumers mishandle top-down clipboard DIBs.
        biHeight: frame.height as i32,
        biPlanes: 1,
        biBitCount: 24,
        biCompression: BI_RGB,
        biSizeImage: (row * frame.height as usize) as u32,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    let header_len = std::mem::size_of::<BITMAPINFOHEADER>();
    let mut bytes = vec![0u8; header_len + row * frame.height as usize];
    // SAFETY: BITMAPINFOHEADER is plain old data and `bytes` is large enough.
    unsafe { std::ptr::write_unaligned(bytes.as_mut_ptr() as *mut BITMAPINFOHEADER, header) };
    for y in 0..frame.height as usize {
        let src = &frame.pixels[y * frame.stride()..(y + 1) * frame.stride()];
        let dst_row = header_len + (frame.height as usize - 1 - y) * row;
        let dst = &mut bytes[dst_row..dst_row + frame.width as usize * 3];
        for (d, s) in dst.chunks_exact_mut(3).zip(src.chunks_exact(4)) {
            d.copy_from_slice(&s[..3]);
        }
    }
    bytes
}

/// Decodes a packed DIB (header, optional masks, pixels) of 24 or 32 bits
/// into top-down straight RGBA.
pub fn decode_dib(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if bytes.len() < 40 {
        return None;
    }
    let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    let header_size = u32_at(0) as usize;
    let width = u32_at(4) as i32;
    let height = u32_at(8) as i32;
    let bits = u16::from_le_bytes([bytes[14], bytes[15]]);
    let compression = u32_at(16);
    if width <= 0 || height == 0 || !(bits == 24 || bits == 32) {
        return None;
    }
    // BI_BITFIELDS with a plain BITMAPINFOHEADER appends three masks.
    let masks = if compression == 3 && header_size == 40 { 12 } else { 0 };
    let offset = header_size + masks;
    let (w, h) = (width as usize, height.unsigned_abs() as usize);
    let row = (w * bits as usize / 8 + 3) & !3;
    if bytes.len() < offset + row * h {
        return None;
    }
    let mut rgba = vec![0u8; w * h * 4];
    let mut any_alpha = false;
    for y in 0..h {
        let src_y = if height > 0 { h - 1 - y } else { y };
        let src = &bytes[offset + src_y * row..];
        for x in 0..w {
            let (b, g, r, a) = if bits == 32 {
                let p = &src[x * 4..x * 4 + 4];
                (p[0], p[1], p[2], p[3])
            } else {
                let p = &src[x * 3..x * 3 + 3];
                (p[0], p[1], p[2], 255)
            };
            any_alpha |= a != 0;
            rgba[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&[r, g, b, a]);
        }
    }
    if bits == 32 && !any_alpha {
        // Most producers leave the fourth byte zero: the image is opaque.
        rgba.chunks_exact_mut(4).for_each(|p| p[3] = 255);
    }
    Some((w as u32, h as u32, rgba))
}

fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..w * h * 4].to_vec(),
        png::ColorType::GrayscaleAlpha => buf.chunks_exact(2).take(w * h).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        _ => return None,
    };
    Some((info.width, info.height, rgba))
}

fn global(bytes: &[u8]) -> io::Result<HANDLE> {
    // SAFETY: the block is allocated with the needed size and unlocked
    // before ownership passes to the clipboard.
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let target = GlobalLock(handle) as *mut u8;
        if target.is_null() {
            GlobalFree(handle);
            return Err(io::Error::last_os_error());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        GlobalUnlock(handle);
        Ok(handle)
    }
}

/// Copies the bytes of a clipboard HGLOBAL.
unsafe fn global_bytes(handle: HANDLE) -> Option<Vec<u8>> {
    if handle.is_null() {
        return None;
    }
    let p = GlobalLock(handle) as *const u8;
    if p.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(p, GlobalSize(handle)).to_vec();
    GlobalUnlock(handle);
    Some(bytes)
}

/// Opens the clipboard, retrying briefly while another app holds it.
fn open(owner: isize) -> io::Result<()> {
    for _ in 0..10 {
        // SAFETY: plain Win32 call; the caller closes the clipboard.
        if unsafe { OpenClipboard(owner as HWND) } != 0 {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(io::Error::last_os_error())
}

/// Replaces the clipboard contents with `(format, bytes)` pairs.
fn put(owner: isize, items: &[(u32, &[u8])]) -> io::Result<()> {
    open(owner)?;
    // SAFETY: the clipboard is open on this thread; each HGLOBAL is freed
    // if the clipboard does not take ownership of it.
    unsafe {
        let result = (|| {
            EmptyClipboard();
            for &(format, bytes) in items {
                let handle = global(bytes)?;
                if SetClipboardData(format, handle).is_null() {
                    let error = io::Error::last_os_error();
                    GlobalFree(handle);
                    return Err(error);
                }
            }
            Ok(())
        })();
        CloseClipboard();
        result
    }
}

/// Replaces the clipboard contents with `frame`. `owner` must be a window of
/// this process; the clipboard rejects data without an owner.
pub fn copy(frame: &Frame, owner: isize) -> io::Result<()> {
    let dib = dib_bytes(frame);
    let png = frame.encode_png_vec()?;
    // SAFETY: plain Win32 call with a NUL-terminated name.
    let png_format = unsafe { RegisterClipboardFormatW(wide("PNG").as_ptr()) };
    put(owner, &[(CF_DIB, &dib), (png_format, &png)])
}

/// Replaces the clipboard contents with text.
pub fn copy_text(text: &str, owner: isize) -> io::Result<()> {
    let units: Vec<u8> = text.encode_utf16().chain(Some(0)).flat_map(|u| u.to_le_bytes()).collect();
    put(owner, &[(CF_UNICODETEXT, &units)])
}

/// Reads what the editor can paste, preferring images, then files, then
/// text. `owner` is any window of this process.
pub fn read(owner: isize) -> io::Result<Option<Pasted>> {
    open(owner)?;
    // SAFETY: the clipboard is open on this thread; the handles it returns
    // belong to it and are only read while it stays open.
    let result = unsafe {
        let png_format = RegisterClipboardFormatW(wide("PNG").as_ptr());
        let mut image = None;
        if IsClipboardFormatAvailable(png_format) != 0 {
            image = global_bytes(GetClipboardData(png_format)).and_then(|b| decode_png(&b));
        }
        for format in [CF_DIBV5, CF_DIB] {
            if image.is_none() && IsClipboardFormatAvailable(format) != 0 {
                image = global_bytes(GetClipboardData(format)).and_then(|b| decode_dib(&b));
            }
        }
        if let Some((width, height, rgba)) = image {
            Some(Pasted::Image { width, height, rgba })
        } else if IsClipboardFormatAvailable(CF_HDROP) != 0 {
            let drop = GetClipboardData(CF_HDROP) as HDROP;
            let count = DragQueryFileW(drop, u32::MAX, std::ptr::null_mut(), 0);
            let files = (0..count)
                .map(|i| {
                    let len = DragQueryFileW(drop, i, std::ptr::null_mut(), 0) as usize;
                    let mut buf = vec![0u16; len + 1];
                    DragQueryFileW(drop, i, buf.as_mut_ptr(), buf.len() as u32);
                    PathBuf::from(crate::from_wide(&buf))
                })
                .collect();
            Some(Pasted::Files(files))
        } else if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
            global_bytes(GetClipboardData(CF_UNICODETEXT)).map(|b| {
                let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                Pasted::Text(crate::from_wide(&units))
            })
        } else {
            None
        }
    };
    // SAFETY: opened above, on this thread.
    unsafe { CloseClipboard() };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dib_is_bottom_up_bgr_with_padded_rows() {
        let mut frame = Frame::new(3, 2);
        frame.pixels[..4].copy_from_slice(&[1, 2, 3, 0]); // top-left
        frame.pixels[12..16].copy_from_slice(&[4, 5, 6, 0]); // bottom-left
        let bytes = dib_bytes(&frame);
        let header = std::mem::size_of::<BITMAPINFOHEADER>();
        assert_eq!(bytes.len(), header + 12 * 2);
        assert_eq!(&bytes[header..header + 3], &[4, 5, 6]);
        assert_eq!(&bytes[header + 12..header + 15], &[1, 2, 3]);
        let (w, h, rgba) = decode_dib(&bytes).unwrap();
        assert_eq!((w, h), (3, 2));
        assert_eq!(&rgba[..4], &[3, 2, 1, 255], "top-left back as RGBA");
        assert_eq!(&rgba[12..16], &[6, 5, 4, 255]);
    }
}

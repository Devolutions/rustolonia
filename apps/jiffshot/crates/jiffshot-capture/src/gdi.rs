//! A GDI DIB section bound to its own memory DC: the shared surface type for
//! screen grabs, the overlay's buffers and cursor compositing.

use crate::Frame;
use std::io;
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};

pub struct Dib {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
    pub width: i32,
    pub height: i32,
}

// SAFETY: the DC and bitmap are owned exclusively by this value; GDI objects
// may be used from any thread as long as access is not concurrent.
unsafe impl Send for Dib {}

impl Dib {
    /// Creates a top-down 32-bit DIB section of the given size.
    pub fn new(width: i32, height: i32) -> io::Result<Dib> {
        // SAFETY: plain GDI object creation; every handle is checked and owned
        // by the returned value.
        unsafe {
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            if dc.is_null() {
                return Err(io::Error::last_os_error());
            }
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
            if bitmap.is_null() || bits.is_null() {
                let error = io::Error::last_os_error();
                DeleteDC(dc);
                return Err(error);
            }
            let previous = SelectObject(dc, bitmap);
            Ok(Dib { dc, bitmap, previous, bits: bits.cast(), width, height })
        }
    }

    pub fn from_frame(frame: &Frame) -> io::Result<Dib> {
        let mut dib = Dib::new(frame.width as i32, frame.height as i32)?;
        dib.pixels_mut().copy_from_slice(&frame.pixels);
        Ok(dib)
    }

    pub fn dc(&self) -> HDC {
        self.dc
    }

    fn len(&self) -> usize {
        self.width as usize * self.height as usize * 4
    }

    /// The pixels, after any pending GDI drawing has been flushed.
    pub fn pixels(&self) -> &[u8] {
        // SAFETY: `bits` points at `len` bytes owned by the DIB section for as
        // long as `self` lives.
        unsafe {
            GdiFlush();
            std::slice::from_raw_parts(self.bits, self.len())
        }
    }

    pub fn pixels_mut(&mut self) -> &mut [u8] {
        // SAFETY: as in `pixels`, with exclusive access through `&mut self`.
        unsafe {
            GdiFlush();
            std::slice::from_raw_parts_mut(self.bits, self.len())
        }
    }

    pub fn to_frame(&self) -> Frame {
        Frame { width: self.width as u32, height: self.height as u32, pixels: self.pixels().to_vec(), alpha: false }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        // SAFETY: restores the DC's original bitmap before deleting both.
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

//! Monitors, the virtual desktop, GDI screen grabs and the mouse cursor.
//!
//! GDI `BitBlt` is used for now: it is fast enough to freeze the whole desktop
//! before the overlay opens. Content that GDI cannot read (some
//! hardware-overlay video) needs Windows.Graphics.Capture, planned later.

use crate::gdi::Dib;
use crate::{from_wide, Frame, Rect};
use std::io;
use windows_sys::Win32::Foundation::{BOOL, LPARAM, POINT, RECT, TRUE};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, EnumDisplayMonitors, GetDC, GetMonitorInfoW, MonitorFromPoint, ReleaseDC, CAPTUREBLT,
    HDC, HMONITOR, MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST, SRCCOPY,
};
use windows_sys::Win32::UI::HiDpi::{
    GetDpiForMonitor, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    MDT_EFFECTIVE_DPI,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CopyIcon, DestroyIcon, DrawIconEx, GetCursorInfo, GetIconInfo, GetSystemMetrics, CURSORINFO,
    CURSOR_SHOWING, DI_NORMAL, HICON, ICONINFO, MONITORINFOF_PRIMARY, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

/// Opts the process into per-monitor-v2 DPI awareness so every coordinate is
/// in physical pixels. Must run before any window is created.
pub fn enable_per_monitor_dpi() {
    // SAFETY: a process-wide setting with no pointer arguments; failure
    // (already set, e.g. by a manifest) is harmless.
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// The bounding rectangle of all monitors.
pub fn virtual_screen() -> Rect {
    // SAFETY: no arguments beyond a metric index.
    unsafe {
        Rect::new(
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Monitor {
    pub rect: Rect,
    pub work: Rect,
    pub dpi: u32,
    pub primary: bool,
    pub device: String,
}

impl Monitor {
    /// UI scale relative to 96 DPI.
    pub fn scale(&self) -> f32 {
        self.dpi as f32 / 96.0
    }
}

fn rect(r: RECT) -> Rect {
    Rect::from_ltrb(r.left, r.top, r.right, r.bottom)
}

fn describe_monitor(handle: HMONITOR) -> Option<Monitor> {
    // SAFETY: `info` is a correctly sized MONITORINFOEXW; the DPI outputs are
    // plain integers.
    unsafe {
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(handle, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO) == 0 {
            return None;
        }
        let (mut dpi_x, mut dpi_y) = (96, 96);
        GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        Some(Monitor {
            rect: rect(info.monitorInfo.rcMonitor),
            work: rect(info.monitorInfo.rcWork),
            dpi: dpi_x,
            primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
            device: from_wide(&info.szDevice),
        })
    }
}

pub fn monitors() -> Vec<Monitor> {
    unsafe extern "system" fn collect(handle: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        // SAFETY: `data` is the `Vec` passed below, alive for the whole call.
        let list = &mut *(data as *mut Vec<Monitor>);
        list.extend(describe_monitor(handle));
        TRUE
    }
    let mut list: Vec<Monitor> = Vec::new();
    // SAFETY: the callback only touches `list` through the pointer it is given.
    unsafe {
        EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(collect), &mut list as *mut _ as LPARAM);
    }
    list
}

/// The monitor containing (or nearest to) a screen point.
pub fn monitor_at(x: i32, y: i32) -> Option<Monitor> {
    // SAFETY: MonitorFromPoint with DEFAULTTONEAREST always returns a monitor.
    describe_monitor(unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) })
}

/// Copies a rectangle of the screen (screen coordinates) into a frame.
pub fn grab(r: Rect) -> io::Result<Frame> {
    if r.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty capture rectangle"));
    }
    let dib = Dib::new(r.width, r.height)?;
    // SAFETY: the screen DC is released before returning; `dib` owns its DC.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let copied = BitBlt(dib.dc(), 0, 0, r.width, r.height, screen, r.x, r.y, SRCCOPY | CAPTUREBLT);
        ReleaseDC(std::ptr::null_mut(), screen);
        if copied == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(dib.to_frame())
}

/// Repeated grabs of one rectangle into a reusable bitmap (screen
/// recording).
pub struct Grabber {
    rect: Rect,
    dib: Dib,
}

impl Grabber {
    pub fn new(rect: Rect) -> io::Result<Grabber> {
        Ok(Grabber { rect, dib: Dib::new(rect.width, rect.height)? })
    }

    /// Grabs the rectangle; returns its pixels (top-down BGRX).
    pub fn grab(&mut self) -> io::Result<&[u8]> {
        let r = self.rect;
        // SAFETY: as in `grab`.
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            let copied = BitBlt(self.dib.dc(), 0, 0, r.width, r.height, screen, r.x, r.y, SRCCOPY | CAPTUREBLT);
            ReleaseDC(std::ptr::null_mut(), screen);
            if copied == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(self.dib.pixels())
    }
}

/// The visible mouse cursor at one moment, kept so it can be composited onto
/// a capture taken from a frozen snapshot.
pub struct CursorSnapshot {
    icon: HICON,
    /// Top-left of the cursor image in screen coordinates.
    pub x: i32,
    pub y: i32,
}

// SAFETY: the icon is an owned copy, only drawn and destroyed.
unsafe impl Send for CursorSnapshot {}

impl CursorSnapshot {
    /// Returns `None` when the cursor is hidden.
    pub fn take() -> Option<CursorSnapshot> {
        // SAFETY: sized out-structs; the icon is copied so it outlives the
        // system's shared cursor, and the ICONINFO bitmaps are released.
        unsafe {
            let mut info: CURSORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
            if GetCursorInfo(&mut info) == 0 || info.flags & CURSOR_SHOWING == 0 || info.hCursor.is_null() {
                return None;
            }
            let icon = CopyIcon(info.hCursor);
            if icon.is_null() {
                return None;
            }
            let mut icon_info: ICONINFO = std::mem::zeroed();
            let (mut hot_x, mut hot_y) = (0, 0);
            if GetIconInfo(icon, &mut icon_info) != 0 {
                hot_x = icon_info.xHotspot as i32;
                hot_y = icon_info.yHotspot as i32;
                if !icon_info.hbmMask.is_null() {
                    windows_sys::Win32::Graphics::Gdi::DeleteObject(icon_info.hbmMask);
                }
                if !icon_info.hbmColor.is_null() {
                    windows_sys::Win32::Graphics::Gdi::DeleteObject(icon_info.hbmColor);
                }
            }
            Some(CursorSnapshot { icon, x: info.ptScreenPos.x - hot_x, y: info.ptScreenPos.y - hot_y })
        }
    }

    /// The cursor alone, with transparency (drawn once on black and once on
    /// white; the difference gives the alpha).
    pub fn image(&self) -> io::Result<Frame> {
        let size = 64;
        let on = |v: u8| -> io::Result<Frame> {
            let mut f = Frame::new(size, size);
            f.pixels.iter_mut().for_each(|b| *b = v);
            let probe = CursorSnapshot { icon: self.icon, x: 0, y: 0 };
            let r = probe.draw_onto(&mut f, (0, 0));
            std::mem::forget(probe);
            r.map(|_| f)
        };
        let (black, white) = (on(0)?, on(255)?);
        let mut out = Frame::new(size, size);
        out.alpha = true;
        for i in 0..(size * size) as usize {
            let a = 255 - (white.pixels[i * 4 + 1] as i32 - black.pixels[i * 4 + 1] as i32).clamp(0, 255);
            let un = |c: u8| if a > 0 { (c as i32 * 255 / a).min(255) as u8 } else { 0 };
            out.pixels[i * 4..i * 4 + 4].copy_from_slice(&[un(black.pixels[i * 4]), un(black.pixels[i * 4 + 1]), un(black.pixels[i * 4 + 2]), a as u8]);
        }
        Ok(out)
    }

    /// Draws the cursor onto `frame`, whose top-left is at `origin` in screen
    /// coordinates.
    pub fn draw_onto(&self, frame: &mut Frame, origin: (i32, i32)) -> io::Result<()> {
        let mut dib = Dib::from_frame(frame)?;
        // SAFETY: draws an owned icon into an owned DC.
        unsafe {
            DrawIconEx(dib.dc(), self.x - origin.0, self.y - origin.1, self.icon, 0, 0, 0, std::ptr::null_mut(), DI_NORMAL);
        }
        frame.pixels.copy_from_slice(dib.pixels_mut());
        Ok(())
    }
}

impl Drop for CursorSnapshot {
    fn drop(&mut self) {
        // SAFETY: destroys the copy made in `take`.
        unsafe {
            DestroyIcon(self.icon);
        }
    }
}

#[cfg(test)]
mod grab_timing {
    use super::*;

    /// Prints how long whole-desktop grabs take with and without CAPTUREBLT
    /// and split per monitor across threads (run with --ignored --nocapture).
    #[test]
    #[ignore]
    fn grab_timing() {
        let desktop = virtual_screen();
        let blt = |r: Rect, flags: u32| {
            let dib = Dib::new(r.width, r.height).unwrap();
            // SAFETY: as in `grab`.
            unsafe {
                let screen = GetDC(std::ptr::null_mut());
                BitBlt(dib.dc(), 0, 0, r.width, r.height, screen, r.x, r.y, flags);
                ReleaseDC(std::ptr::null_mut(), screen);
            }
            dib.to_frame()
        };
        for round in 0..3 {
            let t = std::time::Instant::now();
            drop(blt(desktop, SRCCOPY | CAPTUREBLT));
            let with = t.elapsed();
            let t = std::time::Instant::now();
            drop(blt(desktop, SRCCOPY));
            let without = t.elapsed();
            let t = std::time::Instant::now();
            std::thread::scope(|s| {
                for m in monitors() {
                    s.spawn(move || drop(blt(m.rect, SRCCOPY | CAPTUREBLT)));
                }
            });
            let split = t.elapsed();
            println!("round {round}: CAPTUREBLT {with:?}, plain {without:?}, per monitor in threads {split:?}");
        }
    }
}

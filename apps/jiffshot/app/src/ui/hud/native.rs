//! Window styles and placement Avalonia has no property for (Windows only),
//! shared by the capture panels, the recording toolbar, the overlay and
//! Screen Draw. Placement is in desktop (physical) pixels: Avalonia's own
//! sizing is in DIPs and goes wrong while a window moves between screens of
//! different scales.

use jiffshot_capture::Rect;
use rustolonia::{AppScope, Window};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, LWA_ALPHA,
    SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

/// The window's native handle, or 0 before it has one.
pub fn handle(scope: &AppScope, window: &Window) -> isize {
    scope.platform_handle(window).unwrap_or(0)
}

/// Whether the panels may show in screen captures (JIFFSHOT_HUD_CAPTURABLE,
/// to check them on screen).
pub fn capturable() -> bool {
    std::env::var_os("JIFFSHOT_HUD_CAPTURABLE").is_some()
}

/// Leaves the window out of screen captures, unless [`capturable`]. The
/// window must have been shown once.
pub fn exclude_from_capture(scope: &AppScope, window: &Window) {
    if !capturable() {
        if let Err(error) = scope.set_excluded_from_capture(window, true) {
            eprintln!("could not leave a window out of captures: {error:?}");
        }
    }
}

/// Never activated by a click, not in Alt+Tab, left out of screen captures
/// (unless [`capturable`]), and with `click_through`, transparent to the
/// pointer.
pub fn prepare(scope: &AppScope, window: &Window, click_through: bool) {
    let hwnd = handle(scope, window) as HWND;
    if hwnd.is_null() {
        return;
    }
    // SAFETY: plain style changes on our own, live window.
    unsafe {
        let mut style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) | (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) as isize;
        if click_through {
            style |= (WS_EX_LAYERED | WS_EX_TRANSPARENT) as isize;
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
        if click_through {
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
        }
    }
    exclude_from_capture(scope, window);
}

/// Puts the window exactly on `r` (desktop pixels), topmost, without
/// activating it; with `show`, shows it too.
pub fn place(hwnd: isize, r: Rect, show: bool) {
    if hwnd == 0 || r.is_empty() {
        return;
    }
    let flags = SWP_NOACTIVATE | if show { SWP_SHOWWINDOW } else { 0 };
    // SAFETY: plain Win32 call on our own window.
    unsafe { SetWindowPos(hwnd as HWND, HWND_TOPMOST, r.x, r.y, r.width, r.height, flags) };
}

/// Hides the window natively; Avalonia still counts it as open.
pub fn hide(hwnd: isize) {
    if hwnd != 0 {
        // SAFETY: plain Win32 call on our own window.
        unsafe { ShowWindow(hwnd as HWND, SW_HIDE) };
    }
}

/// Cloaks the window: it exists and is "shown", but nothing of it appears.
pub fn cloak(hwnd: isize, cloaked: bool) {
    if hwnd == 0 {
        return;
    }
    let value: i32 = cloaked.into();
    // SAFETY: DWMWA_CLOAK takes a BOOL, passed by pointer with its size.
    unsafe {
        DwmSetWindowAttribute(
            hwnd as HWND,
            DWMWA_CLOAK as u32,
            (&value as *const i32).cast(),
            std::mem::size_of::<i32>() as u32,
        )
    };
}

/// Makes the window the foreground window (it takes the keyboard).
pub fn activate(hwnd: isize) {
    if hwnd != 0 {
        // SAFETY: plain Win32 call on our own window.
        unsafe { SetForegroundWindow(hwnd as HWND) };
    }
}

/// The working area and scale of the screen showing `r` (its centre), in
/// desktop pixels.
pub fn screen_of(r: Rect) -> (Rect, f64) {
    match jiffshot_capture::screen::monitor_at(r.x + r.width / 2, r.y + r.height / 2) {
        Some(m) => (m.work, m.scale() as f64),
        None => (r, 1.0),
    }
}

/// Where a panel `w` × `h` goes next to `area`: below it, else above it,
/// else inside its bottom edge, `gap` away; `x` is clamped to the screen.
pub fn beside(area: Rect, work: Rect, x: i32, w: i32, h: i32, gap: i32) -> (i32, i32) {
    let x = x.clamp(work.x, (work.right() - w).max(work.x));
    let below = area.bottom() + gap;
    let above = area.y - gap - h;
    let y = if below + h <= work.bottom() {
        below
    } else if above >= work.y {
        above
    } else {
        area.bottom() - h - gap
    };
    (x, y)
}

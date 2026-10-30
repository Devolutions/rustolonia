//! Captures beyond a single frame: scrolling (the tool scrolls the area and
//! stitches), panoramic (the user scrolls or pans, frames are joined in any
//! direction), menus (open menus with transparency around them) and text.

use crate::hud::{self, Border};
use jiffshot_capture::screen;
use jiffshot_capture::stitch;
use jiffshot_capture::{from_wide, Frame, Rect};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::time::Duration;
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT, VK_ESCAPE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetCursorPos, GetWindowLongW, IsWindowVisible, SetCursorPos, GWL_STYLE, WS_POPUP,
};

/// Longest scrolling capture, in pixels.
const MAX_HEIGHT: u32 = 32_000;
const SCROLL_NOTCHES: i32 = 3;

fn escape_pressed() -> bool {
    // SAFETY: plain key-state query.
    unsafe { (GetAsyncKeyState(VK_ESCAPE as i32) as u16 & 0x8000) != 0 }
}

fn wheel(notches: i32) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: (notches * 120) as u32, dwFlags: MOUSEEVENTF_WHEEL, time: 0, dwExtraInfo: 0 },
        },
    };
    // SAFETY: one well-formed input event.
    unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) };
}

/// Scrolls the content under `rect` down with the mouse wheel, grabbing a
/// frame after each step, until it stops moving, Esc is pressed or the
/// image gets very tall; then stitches the frames.
pub fn scrolling(rect: Rect) -> io::Result<Frame> {
    let _border = Border::show(rect);
    let mut saved = POINT { x: 0, y: 0 };
    // SAFETY: out-parameter only; the cursor is restored below.
    unsafe {
        GetCursorPos(&mut saved);
        let c = rect.center();
        SetCursorPos(c.0, c.1);
    }
    std::thread::sleep(Duration::from_millis(150));
    let mut frames = vec![screen::grab(rect)?];
    let mut height = rect.height as u32;
    let mut stalls = 0;
    while height < MAX_HEIGHT && !escape_pressed() {
        wheel(-SCROLL_NOTCHES);
        // Smooth scrolling animates; give it time to settle.
        std::thread::sleep(Duration::from_millis(260));
        let next = screen::grab(rect)?;
        match stitch::vertical_step(frames.last().unwrap(), &next) {
            Some(step) => {
                height += step.dy as u32;
                frames.push(next);
                stalls = 0;
            }
            None => {
                stalls += 1;
                if stalls >= 2 {
                    break;
                }
            }
        }
    }
    // SAFETY: as above.
    unsafe { SetCursorPos(saved.x, saved.y) };
    stitch::stitch_vertical(&frames).ok_or_else(|| io::Error::other("nothing to stitch"))
}

fn hwheel(notches: i32) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::MOUSEEVENTF_HWHEEL;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: (notches * 120) as u32, dwFlags: MOUSEEVENTF_HWHEEL, time: 0, dwExtraInfo: 0 },
        },
    };
    // SAFETY: one well-formed input event.
    unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) };
}

/// Frames turned a quarter (columns become rows), so horizontal scrolling
/// can use the vertical stitcher.
fn transpose(f: &Frame) -> Frame {
    let mut out = Frame::new(f.height, f.width);
    for y in 0..f.height as usize {
        for x in 0..f.width as usize {
            let s = y * f.stride() + x * 4;
            let d = x * out.stride() + y * 4;
            out.pixels[d..d + 4].copy_from_slice(&f.pixels[s..s + 4]);
        }
    }
    out
}

/// Scrolls `rect` to the right (horizontal wheel) and stitches the frames.
pub fn scrolling_horizontal(rect: Rect) -> io::Result<Frame> {
    let _border = Border::show(rect);
    let mut saved = POINT { x: 0, y: 0 };
    // SAFETY: as in `scrolling`.
    unsafe {
        GetCursorPos(&mut saved);
        let c = rect.center();
        SetCursorPos(c.0, c.1);
    }
    std::thread::sleep(Duration::from_millis(150));
    let mut frames = vec![transpose(&screen::grab(rect)?)];
    let mut width = rect.width as u32;
    let mut stalls = 0;
    while width < MAX_HEIGHT && !escape_pressed() {
        hwheel(SCROLL_NOTCHES);
        std::thread::sleep(Duration::from_millis(260));
        let next = transpose(&screen::grab(rect)?);
        match stitch::vertical_step(frames.last().unwrap(), &next) {
            Some(step) => {
                width += step.dy as u32;
                frames.push(next);
                stalls = 0;
            }
            None => {
                stalls += 1;
                if stalls >= 2 {
                    break;
                }
            }
        }
    }
    // SAFETY: as above.
    unsafe { SetCursorPos(saved.x, saved.y) };
    stitch::stitch_vertical(&frames).map(|f| transpose(&f)).ok_or_else(|| io::Error::other("nothing to stitch"))
}

/// Scrolls both ways: a vertical scrolling capture per column, moving
/// right between columns, joined side by side.
pub fn scrolling_both(rect: Rect) -> io::Result<Frame> {
    let mut columns: Vec<(Frame, (i32, i32))> = Vec::new();
    let mut previous_top: Option<Frame> = None;
    let mut x = 0;
    let c = rect.center();
    while x < MAX_HEIGHT as i32 && !escape_pressed() {
        // SAFETY: as in `scrolling`.
        unsafe { SetCursorPos(c.0, c.1) };
        for _ in 0..40 {
            wheel(SCROLL_NOTCHES * 3);
        }
        std::thread::sleep(Duration::from_millis(400));
        let top = screen::grab(rect)?;
        if let Some(prev) = &previous_top {
            match stitch::vertical_step(&transpose(prev), &transpose(&top)) {
                Some(step) => x += step.dy as i32,
                None => break,
            }
        }
        columns.push((scrolling(rect)?, (x, 0)));
        previous_top = Some(top);
        // SAFETY: as above.
        unsafe { SetCursorPos(c.0, c.1) };
        hwheel(SCROLL_NOTCHES * 2);
        std::thread::sleep(Duration::from_millis(300));
    }
    stitch::compose(&columns).ok_or_else(|| io::Error::other("nothing to stitch"))
}

/// Records `rect` while the user scrolls or pans its content, joining
/// frames wherever they moved to. `None` when cancelled.
pub fn panoramic(rect: Rect) -> io::Result<Option<Frame>> {
    let _border = Border::show(rect);
    let first = screen::grab(rect)?;
    let frames = Rc::new(RefCell::new(vec![(first, (0i32, 0i32))]));
    let error: Rc<RefCell<Option<io::Error>>> = Rc::default();
    let (f, e) = (frames.clone(), error.clone());
    let choice = hud::run_panel(
        rect,
        "Scroll or pan the content slowly, in any direction. Finish when done; Esc cancels.",
        &["Finish", "Cancel"],
        120,
        move || {
            let next = match screen::grab(rect) {
                Ok(n) => n,
                Err(err) => {
                    *e.borrow_mut() = Some(err);
                    return None;
                }
            };
            let mut frames = f.borrow_mut();
            let (last, at) = frames.last().unwrap();
            if last.pixels == next.pixels {
                return None;
            }
            match stitch::translation(last, &next) {
                Some((0, 0)) => None,
                Some((dx, dy)) => {
                    let at = (at.0 + dx, at.1 + dy);
                    frames.push((next, at));
                    let (x0, x1) = frames.iter().fold((0, 0), |(a, b), (_, p)| (a.min(p.0), b.max(p.0)));
                    let (y0, y1) = frames.iter().fold((0, 0), |(a, b), (_, p)| (a.min(p.1), b.max(p.1)));
                    Some(format!(
                        "{} frames, {} × {} px so far. Keep going, or Finish.",
                        frames.len(),
                        x1 - x0 + rect.width,
                        y1 - y0 + rect.height
                    ))
                }
                None => Some("Lost track: move back a little, more slowly.".to_owned()),
            }
        },
    );
    if let Some(err) = error.borrow_mut().take() {
        return Err(err);
    }
    if choice != Some(0) {
        return Ok(None);
    }
    let frames = frames.borrow();
    Ok(stitch::compose(&frames))
}

fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    // SAFETY: buffer and length match.
    let len = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
    from_wide(&buf[..len])
}

/// Window classes that menus, drop-downs and context menus use.
fn is_menu_class(class: &str) -> bool {
    class == "#32768"
        || class == "Xaml_WindowedPopupClass"
        || class == "Net UI Tool Window"
        || class == "Chrome_WidgetWin_2"
        || class.contains("Popup")
        || class.contains("DropDown")
        || class.contains("Menu")
}

/// Screen rectangles of the menus that are open now, topmost first.
pub fn open_menus() -> Vec<Rect> {
    unsafe extern "system" fn visit(hwnd: HWND, out: LPARAM) -> BOOL {
        let out = &mut *(out as *mut Vec<Rect>);
        if IsWindowVisible(hwnd) == 0 || GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_POPUP == 0 || !is_menu_class(&class_of(hwnd)) {
            return 1;
        }
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS as u32, &mut r as *mut RECT as *mut _, std::mem::size_of::<RECT>() as u32) == 0 {
            let rect = Rect::from_ltrb(r.left, r.top, r.right, r.bottom);
            if rect.width > 8 && rect.height > 8 {
                out.push(rect);
            }
        }
        1
    }
    let mut found: Vec<Rect> = Vec::new();
    // SAFETY: the callback only writes to `found`, which outlives the call.
    unsafe { EnumWindows(Some(visit), &mut found as *mut Vec<Rect> as LPARAM) };
    found
}

/// The open menus, cut out of the screen with transparency around them.
pub fn menus() -> io::Result<Option<Frame>> {
    let rects = open_menus();
    let Some(bounds) = rects.iter().copied().reduce(|a, b| a.union(b)) else { return Ok(None) };
    let mut frame = screen::grab(bounds)?;
    frame.alpha = true;
    let stride = frame.stride();
    for y in 0..frame.height as i32 {
        for x in 0..frame.width as i32 {
            let inside = rects.iter().any(|r| r.contains(bounds.x + x, bounds.y + y));
            frame.pixels[y as usize * stride + x as usize * 4 + 3] = if inside { 255 } else { 0 };
        }
    }
    Ok(Some(frame))
}

/// Recognized text in a frame, one line per line.
pub fn text(frame: &Frame) -> Result<String, String> {
    let lines = jiffshot_capture::ocr::recognize(frame.width, frame.height, &frame.pixels)?;
    Ok(lines.into_iter().map(|l| l.text).collect::<Vec<_>>().join("\r\n"))
}

//! `--scroll-selftest DIR`: runs a real scrolling capture against a window
//! of our own that shows a tall generated page under a fixed header, then
//! checks the stitched image against the page. It moves the mouse pointer
//! over the window for a few seconds.

use crate::modes;
use jiffshot_capture::{wide, Frame, Rect};
use std::sync::atomic::{AtomicI32, AtomicIsize, Ordering};
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, ClientToScreen, EndPaint, InvalidateRect, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    PAINTSTRUCT, SRCCOPY,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const W: i32 = 640;
const H: i32 = 480;
const HEADER: i32 = 48;
const PAGE_H: i32 = 2600;
const STEP: i32 = 60;

static PAGE: OnceLock<Frame> = OnceLock::new();
static TOP: AtomicI32 = AtomicI32::new(0);
static HWND_: AtomicIsize = AtomicIsize::new(0);

fn page() -> Frame {
    let mut f = Frame::new(W as u32, PAGE_H as u32);
    let s = f.stride();
    for y in 0..PAGE_H as usize {
        for x in 0..W as usize {
            // Text-like blocks on white, never periodic.
            let mut n = ((x / 7) as u32).wrapping_mul(0x9E37_79B9) ^ ((y / 5) as u32).wrapping_mul(0x85EB_CA6B);
            n ^= n >> 15;
            n = n.wrapping_mul(0x2C1B_3C6D);
            let ink = y % 26 < 14 && x > 30 && x < W as usize - 60 && (n >> 28) > 6;
            let v = if ink { (n >> 24) as u8 / 3 } else { 250 };
            f.pixels[y * s + x * 4..y * s + x * 4 + 4].copy_from_slice(&[v, v, if ink { 120 } else { 250 }, 0]);
        }
    }
    // The fixed header the page scrolls under.
    for y in 0..HEADER as usize {
        for x in 0..W as usize {
            f.pixels[y * s + x * 4..y * s + x * 4 + 4].copy_from_slice(&[0x50, 0x30, 0x20, 0]);
        }
    }
    f
}

unsafe fn blit(dc: windows_sys::Win32::Graphics::Gdi::HDC, src: &Frame, sy: i32, dy: i32, h: i32) {
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: W,
        biHeight: -h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..std::mem::zeroed()
    };
    let start = sy as usize * src.stride();
    StretchDIBits(dc, 0, dy, W, h, 0, 0, W, h, src.pixels[start..].as_ptr() as *const _, &info, DIB_RGB_COLORS, SRCCOPY);
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            let page = PAGE.get().unwrap();
            blit(dc, page, 0, 0, HEADER);
            blit(dc, page, HEADER + TOP.load(Ordering::Relaxed), HEADER, H - HEADER);
            EndPaint(hwnd, &ps);
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) & 0xFFFF) as i16 as i32;
            let max = PAGE_H - H;
            let top = (TOP.load(Ordering::Relaxed) - delta / 120 * STEP).clamp(0, max);
            TOP.store(top, Ordering::Relaxed);
            InvalidateRect(hwnd, std::ptr::null(), 0);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Opens the test window on its own thread and returns its client area.
fn open_window() -> Rect {
    PAGE.get_or_init(page);
    std::thread::spawn(|| unsafe {
        let class = wide("JiffshotScrollTest");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(proc_),
            hInstance: GetModuleHandleW(std::ptr::null()),
            lpszClassName: class.as_ptr(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            ..std::mem::zeroed()
        };
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            class.as_ptr(),
            wide("Jiffshot scroll test").as_ptr(),
            WS_POPUP | WS_VISIBLE,
            120,
            120,
            W,
            H,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );
        HWND_.store(hwnd as isize, Ordering::Release);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
    while HWND_.load(Ordering::Acquire) == 0 {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: our own window.
    unsafe { ClientToScreen(HWND_.load(Ordering::Acquire) as HWND, &mut p) };
    Rect::new(p.x, p.y, W, H)
}

pub fn run(dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let rect = open_window();
    let started = std::time::Instant::now();
    let out = modes::scrolling(rect)?;
    println!("scrolling capture {}x{} in {:.1} s", out.width, out.height, started.elapsed().as_secs_f64());
    // SAFETY: closes our own window.
    unsafe { PostMessageW(HWND_.load(Ordering::Acquire) as HWND, WM_CLOSE, 0, 0) };
    let path = dir.join("scrolling.png");
    out.save_png(&path)?;
    println!("wrote {}", path.display());
    let page = PAGE.get().unwrap();
    let ok = out.width == page.width && out.height == page.height && out.pixels.chunks(4).zip(page.pixels.chunks(4)).all(|(a, b)| a[..3] == b[..3]);
    println!("{} the {}x{} page", if ok { "matches" } else { "DIFFERS FROM" }, page.width, page.height);
    if !ok && out.width == page.width {
        let bad: Vec<(usize, usize)> = out
            .pixels
            .chunks(4)
            .zip(page.pixels.chunks(4))
            .enumerate()
            .filter(|(_, (a, b))| a[..3] != b[..3])
            .map(|(i, _)| (i % W as usize, i / W as usize))
            .collect();
        let rows: std::collections::BTreeSet<usize> = bad.iter().map(|p| p.1).collect();
        println!("{} pixels differ in {} rows; first {:?}; rows {:?}", bad.len(), rows.len(), &bad[..bad.len().min(3)], rows.iter().take(12).collect::<Vec<_>>());
        if let Some(&(x, y)) = bad.first() {
            let i = (y * W as usize + x) * 4;
            println!("got {:?} want {:?}", &out.pixels[i..i + 3], &page.pixels[i..i + 3]);
        }
    }
    if ok {
        Ok(())
    } else {
        Err(std::io::Error::other("stitched image differs from the page"))
    }
}

/// `--menu-selftest DIR`: opens a popup menu of our own and captures it
/// with the menu capture.
pub fn menus(dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let shown = std::sync::Arc::new(std::sync::atomic::AtomicIsize::new(0));
    let flag = shown.clone();
    std::thread::spawn(move || unsafe {
        let class = wide("JiffshotMenuTest");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(DefWindowProcW),
            hInstance: GetModuleHandleW(std::ptr::null()),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassExW(&wc);
        let owner = CreateWindowExW(WS_EX_TOOLWINDOW, class.as_ptr(), class.as_ptr(), WS_POPUP, 0, 0, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), wc.hInstance, std::ptr::null());
        let menu = CreatePopupMenu();
        let sub = CreatePopupMenu();
        for (i, label) in ["New capture", "Scrolling capture", "Panoramic capture", "Open library"].iter().enumerate() {
            AppendMenuW(menu, MF_STRING, i + 1, wide(label).as_ptr());
        }
        AppendMenuW(sub, MF_STRING, 10, wide("PNG").as_ptr());
        AppendMenuW(sub, MF_STRING, 11, wide("JPEG").as_ptr());
        AppendMenuW(menu, MF_POPUP, sub as usize, wide("Export as").as_ptr());
        SetForegroundWindow(owner);
        flag.store(owner as isize, std::sync::atomic::Ordering::Release);
        TrackPopupMenu(menu, TPM_RETURNCMD, 300, 300, 0, owner, std::ptr::null());
        DestroyMenu(menu);
        DestroyWindow(owner);
    });
    while shown.load(std::sync::atomic::Ordering::Acquire) == 0 {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    std::thread::sleep(std::time::Duration::from_millis(600));
    let found = modes::open_menus();
    println!("open menus: {found:?}");
    let frame = modes::menus()?;
    // SAFETY: closes the menu our thread opened.
    unsafe {
        PostMessageW(shown.load(std::sync::atomic::Ordering::Acquire) as HWND, WM_CANCELMODE, 0, 0);
        EndMenu();
    }
    let Some(frame) = frame else { return Err(std::io::Error::other("no menu found")) };
    let path = dir.join("menu.png");
    frame.save_png(&path)?;
    println!("wrote {} ({}x{}, alpha {})", path.display(), frame.width, frame.height, frame.alpha);
    Ok(())
}

/// `--record-selftest DIR SECONDS`: records the generated page window of
/// the scrolling self-test (scrolling it as it records, so frames change)
/// and reads the file back.
pub fn record(dir: &std::path::Path, seconds: f32) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let rect = open_window();
    let path = dir.join("recording.mp4");
    let p = path.clone();
    let mut options = crate::recorder::Options::audio_only(30, &std::env::var("JIFFSHOT_RECORD_AUDIO").unwrap_or_else(|_| "system".into()));
    // JIFFSHOT_RECORD_FX=1: pointer highlight, click rings, noise gate
    // and the webcam in the bottom-right corner.
    if std::env::var_os("JIFFSHOT_RECORD_FX").is_some() {
        options.cursor_highlight = true;
        options.click_rings = true;
        options.noise_gate = true;
        options.webcam = Some("bottom-right".into());
    }
    // JIFFSHOT_RECORD_DRAW=1: a Screen Draw box (no window without the
    // app), which the recorder blends into every frame.
    let draw = std::env::var_os("JIFFSHOT_RECORD_DRAW").is_some();
    if draw {
        crate::screendraw::show(rect, std::time::Duration::ZERO);
        if let Some(s) = crate::screendraw::state().as_mut() {
            s.key(b'S' as u16);
            s.press(jiffshot_doc::Pt::new(40.0, 40.0));
            s.moved(jiffshot_doc::Pt::new(200.0, 140.0));
            s.release();
        }
    }
    let control = std::sync::Arc::new(crate::recorder::Control::new(&options));
    let c = control.clone();
    // JIFFSHOT_RECORD_SWITCH=1: system sound off for the middle third and
    // the pointer off for the last, as the toolbar would switch them.
    let switch = std::env::var_os("JIFFSHOT_RECORD_SWITCH").is_some();
    let worker = std::thread::spawn(move || crate::recorder::record_to(&p, rect, &options, &c));
    let hwnd = HWND_.load(Ordering::Acquire) as HWND;
    let started = std::time::Instant::now();
    while started.elapsed().as_secs_f32() < seconds {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if switch {
            let third = started.elapsed().as_secs_f32() / seconds * 3.0;
            control.system.store(!(1.0..2.0).contains(&third), Ordering::Relaxed);
            control.cursor.store(third < 2.0, Ordering::Relaxed);
        }
        // Scroll the page without moving the real pointer.
        let top = (TOP.load(Ordering::Relaxed) + 12).min(PAGE_H - H);
        TOP.store(top, Ordering::Relaxed);
        // SAFETY: our own window.
        unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
    }
    control.stop.store(true, Ordering::Release);
    let spec = worker.join().unwrap().map_err(std::io::Error::other)?;
    // SAFETY: closes our own window.
    unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
    let frames = control.frames.load(Ordering::Relaxed);
    if switch {
        println!("switched: system sound off and on again, pointer off; note: {:?}", control.audio.lock().unwrap());
    }
    println!(
        "recorded {}x{} @ {} fps: {frames} frames in {:.1} s ({:.1} fps achieved); audio: {}",
        spec.width,
        spec.height,
        spec.fps,
        seconds,
        frames as f32 / seconds,
        control.audio.lock().unwrap().clone().unwrap_or_else(|| "system".into())
    );
    let mut reader = jiffshot_capture::media::Reader::open(&path).map_err(std::io::Error::other)?;
    println!(
        "read back {}x{}, {:.2} s, {:.1} fps, audio {:?}, {} KiB",
        reader.width,
        reader.height,
        reader.duration as f64 / 1e7,
        reader.fps,
        reader.audio,
        std::fs::metadata(&path)?.len() / 1024
    );
    let a = reader.frame_at(5_000_000).map_err(std::io::Error::other)?.unwrap();
    let b = reader.frame_at(reader.duration * 4 / 5).map_err(std::io::Error::other)?.unwrap();
    println!("frames differ over time: {}", a.bgra != b.bgra);
    if std::env::var_os("JIFFSHOT_RECORD_FX").is_some() {
        // The webcam circle's white ring, 16 px in from the bottom-right.
        let size = (b.height as f32 * 0.25) as u32;
        let (cx, top) = (b.width - 16 - size / 2, b.height - 16 - size);
        let at = |x: u32, y: u32| &b.bgra[((y * b.width + x) * 4) as usize..((y * b.width + x) * 4 + 3) as usize];
        println!("webcam ring {:?}, just outside {:?}", at(cx, top + 1), at(cx, top - 3));
    }
    if draw {
        crate::screendraw::hide();
        // On the box's top edge, half way along.
        let at = ((40 * b.width + 120) * 4) as usize;
        let px = &b.bgra[at..at + 3];
        println!("screen draw box edge {px:?} (BGR): {}", if px[2] > 180 && px[0] < 90 && px[1] < 90 { "red, drawn into the video" } else { "NOT the mark" });
    }
    let shot = jiffshot_capture::Frame { width: b.width, height: b.height, pixels: b.bgra, alpha: false };
    shot.save_png(&dir.join("recording-frame.png"))?;
    Ok(())
}

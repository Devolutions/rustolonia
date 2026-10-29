//! Small Win32 helpers: the editor window's handle, bringing it forward,
//! opening folders and asking the capture side for a capture.

use jiffshot_capture::{wide, Frame};
use jiffshot_doc::Pixmap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicIsize, Ordering};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{IsIconic, SetForegroundWindow, ShowWindow, SW_MINIMIZE, SW_RESTORE, SW_SHOWNORMAL};

/// The editor window's handle, set once it is mounted.
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

pub fn set_main_hwnd(hwnd: isize) {
    MAIN_HWND.store(hwnd, Ordering::Release);
}

/// The editor's window (the capture side's windows share the process, so
/// this is the handle the host reported, not "the first visible window").
pub fn main_hwnd() -> isize {
    MAIN_HWND.load(Ordering::Acquire)
}

pub fn bring_to_front() {
    bring_forward(main_hwnd());
}

/// Restores `hwnd` when minimized and makes it the foreground window.
pub fn bring_forward(hwnd: isize) {
    let hwnd = hwnd as HWND;
    if hwnd.is_null() {
        return;
    }
    // SAFETY: plain Win32 calls on our own window.
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd);
    }
}

pub fn minimize() {
    let hwnd = main_hwnd() as HWND;
    if !hwnd.is_null() {
        // SAFETY: plain Win32 call on our own window.
        unsafe { ShowWindow(hwnd, SW_MINIMIZE) };
    }
}

fn shell(verb: &str, file: &Path, args: Option<&str>) -> bool {
    let verb = wide(verb);
    let file = wide(&file.to_string_lossy());
    let args = args.map(wide);
    // SAFETY: NUL-terminated strings that outlive the call.
    let r = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            args.as_ref().map_or(std::ptr::null(), |a| a.as_ptr()),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    r as isize > 32
}

/// Opens Explorer with `path` selected.
pub fn reveal(path: &Path) {
    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
}

pub fn open_folder(path: &Path) {
    shell("explore", path, None);
}

/// Opens the system print dialog for a file.
pub fn print_file(path: &Path) -> bool {
    shell("print", path, None)
}

/// Moves files to the Recycle Bin (so a mistaken delete can be undone).
pub fn recycle(files: &[PathBuf]) -> bool {
    use windows_sys::Win32::UI::Shell::{SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};
    if files.is_empty() {
        return true;
    }
    // A double-NUL-terminated list of paths.
    let mut list: Vec<u16> = Vec::new();
    for f in files {
        list.extend(f.to_string_lossy().encode_utf16());
        list.push(0);
    }
    list.push(0);
    let mut op = SHFILEOPSTRUCTW {
        hwnd: main_hwnd() as HWND,
        wFunc: FO_DELETE,
        pFrom: list.as_ptr(),
        pTo: std::ptr::null(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT) as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: std::ptr::null(),
    };
    // SAFETY: the list and struct outlive the call.
    unsafe { SHFileOperationW(&mut op) == 0 && op.fAnyOperationsAborted == 0 }
}

/// Starts a capture: the editor gets out of the way, then the capture side
/// opens its overlay and hands the result back.
pub fn start_capture() -> Result<(), String> {
    minimize();
    jiffshot_workflow::request_capture(jiffshot_workflow::MODE_IMAGE, 250);
    Ok(())
}

/// Starts a screen recording.
pub fn start_recording() -> Result<(), String> {
    minimize();
    jiffshot_workflow::request_capture(jiffshot_workflow::MODE_VIDEO, 250);
    Ok(())
}

/// Opens the Capture window over the editor.
pub fn open_capture_window() -> Result<(), String> {
    jiffshot_workflow::request_capture(jiffshot_workflow::OPEN_CAPTURE_WINDOW, 1);
    Ok(())
}

/// The flattened document as an opaque frame (composited over white) for
/// the clipboard.
pub fn frame_from_pixmap(pm: &Pixmap) -> Frame {
    let mut frame = Frame::new(pm.width(), pm.height());
    for (dst, src) in frame.pixels.chunks_exact_mut(4).zip(pm.pixels()) {
        let inv = 255 - src.alpha() as u32;
        let over = |c: u8| (c as u32 + inv).min(255) as u8;
        dst.copy_from_slice(&[over(src.blue()), over(src.green()), over(src.red()), 255]);
    }
    frame
}

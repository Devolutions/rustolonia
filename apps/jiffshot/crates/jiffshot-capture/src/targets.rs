//! Capture targets under the cursor: top-level windows in z-order and their
//! visible child windows, so the overlay can snap to a window or a pane.
//!
//! The list is a snapshot taken when the overlay opens, matching the frozen
//! screen image. Apps that draw their own panes without child windows
//! (browsers, WinUI) only expose their top-level frame; finer detection needs
//! UI Automation and is planned later.

use crate::{from_wide, Rect};
use std::ffi::c_void;
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, RECT, TRUE};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetLayeredWindowAttributes, GetWindow, GetWindowDisplayAffinity, GetWindowLongW, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, GWL_EXSTYLE, GW_CHILD,
    GW_HWNDNEXT, LWA_ALPHA, WDA_NONE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};

#[derive(Clone, Debug)]
pub struct Target {
    pub hwnd: isize,
    /// Screen rectangle, clipped to its top-level window.
    pub rect: Rect,
    pub title: String,
    pub class: String,
    pub process_id: u32,
    /// 0 for top-level windows.
    pub depth: u32,
    /// Index of the parent target, `None` for top-level windows.
    pub parent: Option<usize>,
}

/// Every target, top-level windows in z-order (topmost first), each followed
/// by its descendants in z-order.
#[derive(Clone, Debug, Default)]
pub struct Targets {
    pub list: Vec<Target>,
}

const MAX_DEPTH: u32 = 6;
const MAX_CHILDREN: usize = 4000;
const MIN_SIZE: i32 = 8;

/// Classes that make up the desktop itself; the overlay falls back to the
/// monitor there.
const DESKTOP_CLASSES: &[&str] = &["Progman", "WorkerW"];

fn text(hwnd: HWND, get: unsafe extern "system" fn(HWND, *mut u16, i32) -> i32) -> String {
    let mut buffer = [0u16; 256];
    // SAFETY: the buffer length is passed alongside it.
    unsafe { get(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    from_wide(&buffer)
}

fn window_rect(hwnd: HWND) -> Rect {
    // SAFETY: sized out-parameters only.
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut r);
        Rect::from_ltrb(r.left, r.top, r.right, r.bottom)
    }
}

/// Top-level bounds without the invisible resize borders Windows 10+ adds.
fn frame_rect(hwnd: HWND) -> Rect {
    // SAFETY: sized out-parameter for the DWM attribute.
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            &mut r as *mut RECT as *mut c_void,
            std::mem::size_of::<RECT>() as u32,
        );
        if hr == 0 {
            Rect::from_ltrb(r.left, r.top, r.right, r.bottom)
        } else {
            window_rect(hwnd)
        }
    }
}

fn is_capturable_top_level(hwnd: HWND, exclude: &[isize]) -> bool {
    if exclude.contains(&(hwnd as isize)) {
        return false;
    }
    // SAFETY: read-only queries on a window handle from EnumWindows.
    unsafe {
        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return false;
        }
        let mut cloaked: u32 = 0;
        DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED as u32, &mut cloaked as *mut u32 as *mut c_void, 4);
        if cloaked != 0 {
            return false;
        }
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TRANSPARENT != 0 {
            return false;
        }
        // Windows left out of screen captures (Jiffshot's own panels,
        // toasts and widget) are not capture targets either.
        let mut affinity = 0u32;
        if GetWindowDisplayAffinity(hwnd, &mut affinity) != 0 && affinity != WDA_NONE {
            return false;
        }
        if ex & WS_EX_LAYERED != 0 {
            let (mut key, mut alpha, mut flags) = (0u32, 255u8, 0u32);
            if GetLayeredWindowAttributes(hwnd, &mut key, &mut alpha, &mut flags) != 0
                && flags & LWA_ALPHA != 0
                && alpha == 0
            {
                return false;
            }
        }
    }
    true
}

impl Targets {
    /// Enumerates windows on screen, skipping `exclude` (the overlay itself)
    /// and anything outside `clip` (the virtual screen).
    pub fn enumerate(clip: Rect, exclude: &[isize]) -> Targets {
        struct Collect {
            tops: Vec<isize>,
        }
        unsafe extern "system" fn on_window(hwnd: HWND, data: LPARAM) -> BOOL {
            // SAFETY: `data` is the `Collect` below, alive for EnumWindows.
            (*(data as *mut Collect)).tops.push(hwnd as isize);
            TRUE
        }
        let mut collect = Collect { tops: Vec::new() };
        // SAFETY: the callback only appends to `collect`.
        unsafe { EnumWindows(Some(on_window), &mut collect as *mut Collect as LPARAM) };

        let mut targets = Targets::default();
        for hwnd in collect.tops {
            let hwnd = hwnd as HWND;
            if !is_capturable_top_level(hwnd, exclude) {
                continue;
            }
            let class = text(hwnd, GetClassNameW);
            if DESKTOP_CLASSES.contains(&class.as_str()) {
                continue;
            }
            let Some(rect) = frame_rect(hwnd).intersect(clip) else { continue };
            if rect.width < MIN_SIZE || rect.height < MIN_SIZE {
                continue;
            }
            let index = targets.push(hwnd, rect, class, 0, None);
            targets.children(hwnd, index, rect, 1);
        }
        targets
    }

    fn push(&mut self, hwnd: HWND, rect: Rect, class: String, depth: u32, parent: Option<usize>) -> usize {
        let mut process_id = 0;
        // SAFETY: out-parameter only.
        unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
        self.list.push(Target {
            hwnd: hwnd as isize,
            rect,
            title: text(hwnd, GetWindowTextW),
            class,
            process_id,
            depth,
            parent,
        });
        self.list.len() - 1
    }

    fn children(&mut self, parent: HWND, parent_index: usize, clip: Rect, depth: u32) {
        if depth > MAX_DEPTH {
            return;
        }
        // SAFETY: GetWindow walks the live child list; handles are only read.
        let mut child = unsafe { GetWindow(parent, GW_CHILD) };
        while !child.is_null() && self.list.len() < MAX_CHILDREN {
            // SAFETY: read-only query.
            if unsafe { IsWindowVisible(child) } != 0 {
                if let Some(rect) = window_rect(child).intersect(clip) {
                    if rect == self.list[parent_index].rect {
                        // Full-size hosts (Chromium's render and D3D
                        // windows) add nothing to select; look through them.
                        self.children(child, parent_index, rect, depth + 1);
                    } else if rect.width >= MIN_SIZE && rect.height >= MIN_SIZE {
                        let class = text(child, GetClassNameW);
                        let index = self.push(child, rect, class, depth, Some(parent_index));
                        self.children(child, index, rect, depth + 1);
                    }
                }
            }
            // SAFETY: as above.
            child = unsafe { GetWindow(child, GW_HWNDNEXT) };
        }
    }

    /// The target under a screen point: the deepest visible child of the
    /// topmost window there, moved `levels_up` steps towards its top-level
    /// window. Returns its index.
    pub fn hit(&self, x: i32, y: i32, levels_up: u32) -> Option<usize> {
        let top = self.list.iter().position(|t| t.depth == 0 && t.rect.contains(x, y))?;
        let end = self.list[top + 1..]
            .iter()
            .position(|t| t.depth == 0)
            .map_or(self.list.len(), |i| top + 1 + i);
        let mut best = top;
        for (i, t) in self.list.iter().enumerate().take(end).skip(top + 1) {
            // Siblings come in z-order, so the first match at a depth wins.
            if t.depth > self.list[best].depth && t.rect.contains(x, y) && self.is_ancestor(best, i) {
                best = i;
            }
        }
        for _ in 0..levels_up {
            match self.list[best].parent {
                Some(parent) => best = parent,
                None => break,
            }
        }
        Some(best)
    }

    fn is_ancestor(&self, ancestor: usize, mut index: usize) -> bool {
        while let Some(parent) = self.list[index].parent {
            if parent == ancestor {
                return true;
            }
            index = parent;
        }
        false
    }

    /// How many levels the target at `index` has above it.
    pub fn depth_of(&self, index: usize) -> u32 {
        self.list[index].depth
    }

    /// The top-level window that contains the target at `index`.
    pub fn top_level(&self, mut index: usize) -> &Target {
        while let Some(parent) = self.list[index].parent {
            index = parent;
        }
        &self.list[index]
    }
}

/// The executable file name of a process, e.g. `notepad.exe`.
pub fn process_name(process_id: u32) -> Option<String> {
    // SAFETY: the process handle is closed before returning; the buffer size
    // is passed in and updated by the call.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process.is_null() {
            return None;
        }
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut len);
        CloseHandle(process);
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        path.rsplit('\\').next().map(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(rect: Rect, depth: u32, parent: Option<usize>) -> Target {
        Target { hwnd: 0, rect, title: String::new(), class: String::new(), process_id: 0, depth, parent }
    }

    #[test]
    fn hit_prefers_deepest_child_of_topmost_window() {
        let targets = Targets {
            list: vec![
                target(Rect::new(0, 0, 100, 100), 0, None),   // 0: front window
                target(Rect::new(0, 0, 100, 20), 1, Some(0)), // 1: toolbar
                target(Rect::new(0, 20, 100, 80), 1, Some(0)), // 2: body
                target(Rect::new(10, 30, 30, 30), 2, Some(2)), // 3: button in body
                target(Rect::new(0, 0, 400, 400), 0, None),   // 4: window behind
            ],
        };
        assert_eq!(targets.hit(15, 35, 0), Some(3));
        assert_eq!(targets.hit(15, 35, 1), Some(2));
        assert_eq!(targets.hit(15, 35, 9), Some(0));
        assert_eq!(targets.hit(50, 5, 0), Some(1));
        assert_eq!(targets.hit(200, 200, 0), Some(4));
        assert_eq!(targets.hit(500, 500, 0), None);
    }
}

//! Step Capture: records a screenshot at each click, with a description of
//! what was clicked (from UI Automation), and assembles a step-by-step
//! guide: a numbered list of zoomed screenshots with the click marked and
//! editable captions. Typing is not recorded.

use crate::hud;
use jiffshot_capture::screen;
use jiffshot_capture::{Frame, Rect};
use jiffshot_doc::object::{Kind, ShapeKind, Style};
use jiffshot_doc::stamps::Stamp;
use jiffshot_doc::text::Align;
use jiffshot_doc::{Box2, Document, Pt, Rgba};
use std::cell::RefCell;
use std::rc::Rc;
use windows::Win32::Foundation::POINT as WPOINT;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_RBUTTONDOWN, WindowFromPoint,
    GetAncestor, GetWindowThreadProcessId, GA_ROOT,
};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;

/// One recorded click.
pub struct Step {
    pub frame: Frame,
    /// Where the frame sits on screen.
    pub area: Rect,
    pub click: (i32, i32),
    pub right: bool,
    pub description: String,
}

thread_local! {
    /// Clicks waiting to be described and captured (screen point, right).
    static CLICKS: RefCell<Vec<((i32, i32), bool)>> = const { RefCell::new(Vec::new()) };
    static PAUSED: RefCell<bool> = const { RefCell::new(false) };
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && (wparam as u32 == WM_LBUTTONDOWN || wparam as u32 == WM_RBUTTONDOWN) {
        let info = &*(lparam as *const MSLLHOOKSTRUCT);
        let p = (info.pt.x, info.pt.y);
        // Ignore clicks on our own windows (the panel).
        let top = GetAncestor(WindowFromPoint(POINT { x: p.0, y: p.1 }), GA_ROOT);
        let mut process = 0;
        GetWindowThreadProcessId(top, &mut process);
        let ours = process == GetCurrentProcessId();
        if !ours && !PAUSED.with(|p| *p.borrow()) {
            CLICKS.with(|c| c.borrow_mut().push((p, wparam as u32 == WM_RBUTTONDOWN)));
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

/// "Click the Save button", from what UI Automation says is at `p`.
fn describe(automation: Option<&IUIAutomation>, p: (i32, i32), right: bool) -> String {
    let verb = if right { "Right-click" } else { "Click" };
    let element = automation.and_then(|a| unsafe { a.ElementFromPoint(WPOINT { x: p.0, y: p.1 }).ok() });
    let (name, kind) = match &element {
        // SAFETY: plain property reads on a live element.
        Some(e) => unsafe {
            (
                e.CurrentName().map(|s| s.to_string()).unwrap_or_default(),
                e.CurrentLocalizedControlType().map(|s| s.to_string()).unwrap_or_default(),
            )
        },
        None => (String::new(), String::new()),
    };
    let name: String = name.trim().chars().take(60).collect();
    match (name.is_empty(), kind.is_empty()) {
        (false, false) => format!("{verb} the \u{201C}{name}\u{201D} {kind}."),
        (false, true) => format!("{verb} \u{201C}{name}\u{201D}."),
        (true, false) => format!("{verb} the {kind}."),
        (true, true) => format!("{verb} here."),
    }
}

/// Records until Stop or Esc; returns the steps (`None` when cancelled).
pub fn record() -> Option<Vec<Step>> {
    // SAFETY: COM for UI Automation on this (the tray) thread.
    let automation: Option<IUIAutomation> = unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()
    };
    CLICKS.with(|c| c.borrow_mut().clear());
    PAUSED.with(|p| *p.borrow_mut() = false);
    // SAFETY: a low-level hook for this thread's message loop, removed below.
    let hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(self::hook), std::ptr::null_mut(), 0) };
    let steps: Rc<RefCell<Vec<Step>>> = Rc::default();
    let corner = screen::monitors().into_iter().find(|m| m.primary).map(|m| m.work).unwrap_or_else(screen::virtual_screen);
    let anchor = Rect::new(corner.right() - 380, corner.y + 8, 370, 1);
    let (s, a) = (steps.clone(), automation.clone());
    let choice = loop {
        let paused = PAUSED.with(|p| *p.borrow());
        let buttons: &[&str] = if paused { &["Stop", "Resume", "Cancel"] } else { &["Stop", "Pause", "Cancel"] };
        let (s2, a2) = (s.clone(), a.clone());
        let c = hud::run_panel(anchor, "Step Capture: click through the workflow.", buttons, 120, move || {
            let clicks: Vec<_> = CLICKS.with(|c| c.borrow_mut().drain(..).collect());
            for (p, right) in clicks {
                // Let the click's immediate visual feedback settle, then grab
                // the screen it happened on.
                std::thread::sleep(std::time::Duration::from_millis(60));
                let area = screen::monitor_at(p.0, p.1).map(|m| m.rect).unwrap_or_else(screen::virtual_screen);
                if let Ok(frame) = screen::grab(area) {
                    let description = describe(a2.as_ref(), p, right);
                    s2.borrow_mut().push(Step { frame, area, click: p, right, description });
                }
            }
            let n = s2.borrow().len();
            Some(format!("Step Capture · {n} step{} recorded. Clicks on this panel are not recorded.", if n == 1 { "" } else { "s" }))
        });
        match c {
            Some(1) => PAUSED.with(|p| {
                let v = !*p.borrow();
                *p.borrow_mut() = v;
            }),
            other => break other,
        }
    };
    // SAFETY: removes the hook installed above.
    unsafe { UnhookWindowsHookEx(hook) };
    if choice != Some(0) {
        return None;
    }
    let steps = std::mem::take(&mut *steps.borrow_mut());
    (!steps.is_empty()).then_some(steps)
}

fn frame_to_pixmap(f: &Frame) -> Option<tiny_skia::Pixmap> {
    let mut rgba = Vec::with_capacity(f.pixels.len());
    for p in f.pixels.chunks_exact(4) {
        rgba.extend_from_slice(&[p[2], p[1], p[0], 255]);
    }
    tiny_skia::Pixmap::from_vec(rgba, tiny_skia::IntSize::from_wh(f.width, f.height)?)
}

/// Lays the steps out as an editable guide: a title, then for each step a
/// number, its caption and the screen zoomed around the click, with the
/// click circled and a pointer stamp on it.
pub fn guide(steps: &[Step]) -> Document {
    const W: f32 = 1400.0;
    const SHOT: (f32, f32) = (1000.0, 560.0);
    let pad = 40.0;
    let row = SHOT.1 + pad * 1.5;
    let h = 140.0 + steps.len() as f32 * row + pad;
    let mut doc = Document::blank(W as u32, h as u32, Rgba::WHITE);
    let ink = Rgba::rgb(0x22, 0x26, 0x2E);
    let accent = Rgba::rgb(0x1E, 0x6F, 0xD9);
    let text = |size: f32, bold: bool, color: Rgba| Style { font_size: size, bold, text_color: color, width: 0.0, fill: Rgba::TRANSPARENT, align: Align::Left, ..Style::default() };
    doc.add(Kind::Text { rect: Box2::from_xywh(pad, 30.0, W - 2.0 * pad, 60.0), text: "How to …".into() }, text(40.0, true, ink));
    doc.add(Kind::Text { rect: Box2::from_xywh(pad, 88.0, W - 2.0 * pad, 30.0), text: format!("{} steps", steps.len()) }, text(18.0, false, Rgba::rgb(0x6B, 0x72, 0x80)));
    for (i, step) in steps.iter().enumerate() {
        let y = 140.0 + i as f32 * row;
        // The part of the screen around the click, at most SHOT in size.
        let (fw, fh) = (step.frame.width as f32, step.frame.height as f32);
        let (cx, cy) = ((step.click.0 - step.area.x) as f32, (step.click.1 - step.area.y) as f32);
        let (vw, vh) = (SHOT.0.min(fw), SHOT.1.min(fh));
        let vx = (cx - vw / 2.0).clamp(0.0, fw - vw);
        let vy = (cy - vh / 2.0).clamp(0.0, fh - vh);
        let crop = step.frame.crop(Rect::new(vx as i32, vy as i32, vw as i32, vh as i32));
        let x0 = pad + 70.0;
        if let Some(pm) = crop.as_ref().and_then(frame_to_pixmap) {
            let rect = Box2::from_xywh(x0, y, pm.width() as f32, pm.height() as f32);
            let style = Style { width: 1.5, stroke: Rgba::rgb(0xD0, 0xD4, 0xDA), shadow: true, ..Style::default() };
            doc.add(Kind::Image { rect, image: jiffshot_doc::object::ImageRef::new(format!("step-{}", i + 1), pm) }, style);
            let (px, py) = (x0 + cx - vx, y + cy - vy);
            let ring = Style { stroke: if step.right { Rgba::rgb(0xF5, 0x9E, 0x0B) } else { Rgba::RED }, width: 4.0, fill: Rgba::TRANSPARENT, ..Style::default() };
            doc.add(Kind::Shape { rect: Box2::around(Pt::new(px, py), 22.0, 22.0), shape: ShapeKind::Ellipse }, ring);
            doc.add(Kind::Stamp { rect: Box2::from_xywh(px - 6.0, py - 4.0, 34.0, 40.0), stamp: Stamp::Pointer }, Style::default());
        }
        let badge = Style { fill: accent, text_color: Rgba::WHITE, width: 0.0, font_size: 24.0, ..Style::default() };
        doc.add(Kind::Step { center: Pt::new(pad + 26.0, y + 26.0), radius: 24.0, label: (i + 1).to_string(), shape: ShapeKind::Ellipse }, badge);
        doc.add(Kind::Text { rect: Box2::from_xywh(x0 + SHOT.0 + 24.0, y, W - x0 - SHOT.0 - 24.0 - pad, 120.0), text: step.description.clone() }, text(22.0, false, ink));
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_has_a_numbered_row_per_step() {
        let step = |x: i32, text: &str| {
            let mut frame = Frame::new(1600, 900);
            frame.pixels.iter_mut().for_each(|b| *b = 0xE0);
            Step { frame, area: Rect::new(0, 0, 1600, 900), click: (x, 450), right: false, description: text.to_owned() }
        };
        let doc = guide(&[step(100, "Click the \u{201C}File\u{201D} menu item."), step(1500, "Click the \u{201C}Save\u{201D} button.")]);
        let steps: Vec<_> = doc.objects.iter().filter_map(|o| match &o.kind { Kind::Step { label, .. } => Some(label.clone()), _ => None }).collect();
        assert_eq!(steps, ["1", "2"]);
        let images = doc.objects.iter().filter(|o| matches!(o.kind, Kind::Image { .. })).count();
        assert_eq!(images, 2);
        assert!(doc.objects.iter().any(|o| matches!(&o.kind, Kind::Text { text, .. } if text.contains("Save"))));
        let flat = doc.flatten();
        assert_eq!(flat.width(), 1400);
        if let Some(out) = std::env::var_os("JIFFSHOT_GUIDE_OUT") {
            flat.save_png(out).unwrap();
        }
    }
}

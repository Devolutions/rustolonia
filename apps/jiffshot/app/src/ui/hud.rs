//! The capture side's on-screen furniture (the `hud` signal): control panels
//! next to the area being captured (countdown, recording, scrolling, Step
//! Capture…), red outlines just outside it, and the recording toolbar.
//!
//! The capture side owns the state and sends all of it as JSON each time
//! (`jiffshot_workflow`'s `hud.rs`); [`apply`] makes the windows match. None
//! of them takes the focus or shows in screen captures. A panel's button
//! answers `hud:ID:INDEX`, a toolbar's `hud:ID:act:NAME`, through the main
//! view model's `run`.

mod hud_toolbar;
pub(in crate::ui) mod native;

use hud_toolbar::RecordingToolbar;
use jiffshot_capture::Rect;
use rustolonia::{
    size_to_content, AppScope, Border, Brush, Button, Color, Effect, EventSubscription, FontWeight, HorizontalAlignment, Orientation, Result, StackPanel,
    TextBlock, TextWrapping, ThemeVariantScope, VerticalAlignment, Window, WindowDecorations,
};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

thread_local! {
    static HUD: RefCell<Hud> = RefCell::new(Hud::default());
}

/// Sends a command to the main view model (`hud:…`), on the UI thread.
fn run(command: String) {
    crate::ui::main().model.run(|m| m.run(command));
}

/// The windows showing, by the capture side's ids.
#[derive(Default)]
struct Hud {
    panels: HashMap<u64, HudPanel>,
    borders: HashMap<u64, HudBorder>,
    toolbars: HashMap<u64, RecordingToolbar>,
}

/// A rectangle `[x, y, w, h]` (desktop pixels) of the JSON.
fn area_of(e: &Value) -> Rect {
    let a = e.get("area").and_then(Value::as_array);
    let v = |i: usize| a.and_then(|a| a.get(i)).and_then(Value::as_i64).unwrap_or(0) as i32;
    Rect::new(v(0), v(1), v(2), v(3))
}

/// `{"panels":[{"id","area":[x,y,w,h],"status","buttons":[…]} or {"id","area","toolbar":{…}}],"borders":[{"id","area"}]}`.
fn apply(scope: &AppScope, json: &str) {
    let _t = crate::trace::span("hud");
    if json.is_empty() {
        return;
    }
    let Ok(root) = serde_json::from_str::<Value>(json) else {
        eprintln!("hud: bad state");
        return;
    };
    let list = |name: &str| root.get(name).and_then(Value::as_array).cloned().unwrap_or_default();
    // Windows are made and closed outside the borrow: showing one runs its
    // event handlers.
    let mut closing: Vec<Box<dyn FnOnce()>> = Vec::new();
    let mut live = Vec::new();
    for p in list("panels") {
        let Some(id) = p.get("id").and_then(Value::as_u64) else { continue };
        live.push(id);
        if let Some(state) = p.get("toolbar") {
            let existing = HUD.with(|h| h.borrow().toolbars.get(&id).map(|t| t.update(state)).is_some());
            if !existing {
                match RecordingToolbar::new(scope, id, area_of(&p)) {
                    Ok(toolbar) => {
                        toolbar.update(state);
                        HUD.with(|h| h.borrow_mut().toolbars.insert(id, toolbar));
                    }
                    Err(error) => eprintln!("hud: no toolbar: {error:?}"),
                }
            }
            continue;
        }
        let status = p.get("status").and_then(Value::as_str).unwrap_or_default();
        let existing = HUD.with(|h| h.borrow().panels.get(&id).map(|panel| panel.set_status(status)).is_some());
        if !existing {
            let buttons: Vec<String> = p
                .get("buttons")
                .and_then(Value::as_array)
                .map(|b| b.iter().map(|b| b.as_str().unwrap_or_default().to_owned()).collect())
                .unwrap_or_default();
            match HudPanel::new(scope, id, area_of(&p), &buttons, status) {
                Ok(panel) => {
                    HUD.with(|h| h.borrow_mut().panels.insert(id, panel));
                }
                Err(error) => eprintln!("hud: no panel: {error:?}"),
            }
        }
    }
    HUD.with(|h| {
        let mut hud = h.borrow_mut();
        let gone: Vec<u64> = hud.toolbars.keys().filter(|id| !live.contains(id)).copied().collect();
        for id in gone {
            if let Some(toolbar) = hud.toolbars.remove(&id) {
                closing.push(Box::new(move || toolbar.close()));
            }
        }
        let gone: Vec<u64> = hud.panels.keys().filter(|id| !live.contains(id)).copied().collect();
        for id in gone {
            if let Some(panel) = hud.panels.remove(&id) {
                closing.push(Box::new(move || panel.dismiss()));
            }
        }
    });

    let mut live = Vec::new();
    for b in list("borders") {
        let Some(id) = b.get("id").and_then(Value::as_u64) else { continue };
        live.push(id);
        if HUD.with(|h| h.borrow().borders.contains_key(&id)) {
            continue;
        }
        match HudBorder::new(scope, area_of(&b)) {
            Ok(border) => {
                HUD.with(|h| h.borrow_mut().borders.insert(id, border));
            }
            Err(error) => eprintln!("hud: no outline: {error:?}"),
        }
    }
    HUD.with(|h| {
        let mut hud = h.borrow_mut();
        let gone: Vec<u64> = hud.borders.keys().filter(|id| !live.contains(id)).copied().collect();
        for id in gone {
            if let Some(border) = hud.borders.remove(&id) {
                closing.push(Box::new(move || border.close()));
            }
        }
    });
    for close in closing {
        close();
    }
}

/// Moves `window`, `size` DIPs, next to `area` (desktop pixels): below it,
/// else above it, else inside its bottom edge, `gap` DIPs away; centred
/// under the area or aligned with its right edge.
fn place_beside(window: &Window, area: Rect, size: (f64, f64), gap: f64, centred: bool) {
    let (work, scale) = native::screen_of(area);
    let (w, h) = ((size.0 * scale).ceil() as i32, (size.1 * scale).ceil() as i32);
    let gap = (gap * scale).round() as i32;
    let x = if centred { area.x + (area.width - w) / 2 } else { area.right() - w };
    let (x, y) = native::beside(area, work, x, w, h, gap);
    if let Err(error) = window.set_position((x, y)) {
        eprintln!("hud: placing a window failed: {error:?}");
    }
}

/// A borderless, topmost window that does not take the focus.
fn hud_window() -> Result<Window> {
    Window::new()?
        .title("Jiffshot")?
        .window_decorations(WindowDecorations::None)?
        .show_activated(false)?
        .show_in_taskbar(false)?
        .can_resize(false)
}

/// A capture panel: the status (live) and a row of buttons, the first one
/// the main action. It sits below the area, or above it, or inside its
/// bottom edge when neither fits on the screen.
struct HudPanel {
    window: Window,
    status: TextBlock,
    /// Set when the capture side closes the panel (any other close is Esc).
    dismissed: Arc<AtomicBool>,
    _subscriptions: Vec<EventSubscription>,
}

impl HudPanel {
    /// Gap between the area and the panel, past the outline (DIPs).
    const GAP: f64 = 11.0;

    fn new(scope: &AppScope, id: u64, area: Rect, buttons: &[String], status: &str) -> Result<Self> {
        let text = TextBlock::new()?
            .text(status)?
            .font_size(13.5)?
            .font_weight(FontWeight::DemiBold)?
            .text_wrapping(TextWrapping::Wrap)?
            .foreground(Brush::solid(Color::rgb(0xE8, 0xEA, 0xED)))?;
        let mut subscriptions = Vec::new();
        let row = StackPanel::new()?
            .orientation(Orientation::Horizontal)?
            .spacing(8.0)?
            .horizontal_alignment(HorizontalAlignment::Right)?;
        for (index, label) in buttons.iter().enumerate() {
            let button = Button::new()?
                .content_text(label)?
                .min_width(76.0)?
                .height(30.0)?
                .padding((12.0, 0.0))?
                .corner_radius(6.0)?
                .horizontal_content_alignment(HorizontalAlignment::Center)?
                .vertical_content_alignment(VerticalAlignment::Center)?
                .foreground(Brush::solid(Color::rgb(255, 255, 255)))?
                .background(Brush::solid(if index == 0 {
                    Color::rgb(0x2F, 0x5D, 0xA8)
                } else {
                    Color::rgb(0x3A, 0x3C, 0x42)
                }))?;
            subscriptions.push(button.subscribe_click(move |_| run(format!("hud:{id}:{index}")))?);
            row.children()?.add(&button)?;
        }
        let content = Border::new()?
            .border_brush(Brush::solid(Color::rgb(0x3A, 0x3C, 0x42)))?
            .border_thickness(1.0)?
            .padding((12.0, 9.0, 12.0, 10.0))?
            .child(Some(&StackPanel::new()?.spacing(10.0)?.child(&text)?.child(&row)?))?;
        let themed = ThemeVariantScope::new()?.requested_theme_variant("Dark")?.child(Some(&content))?;
        let window = hud_window()?
            .width(360.0)?
            .size_to_content(size_to_content::HEIGHT)?
            .background(Brush::solid(Color::rgb(0x23, 0x24, 0x28)))?
            .content(Some(&themed))?;
        scope.set_topmost(&window, true)?;
        let placed = window.clone();
        subscriptions.push(window.subscribe_size_changed(move |e| place_beside(&placed, area, (e.new_size.width, e.new_size.height), Self::GAP, false))?);
        let dismissed = Arc::new(AtomicBool::new(false));
        let closed = dismissed.clone();
        // Closed by anything but the capture side (which never happens in
        // normal use): the same as Esc.
        subscriptions.push(window.subscribe_closing(move |_| {
            if !closed.swap(true, Ordering::AcqRel) {
                run(format!("hud:{id}"));
            }
        })?);
        window.show()?;
        native::prepare(scope, &window, false);
        Ok(HudPanel {
            window,
            status: text,
            dismissed,
            _subscriptions: subscriptions,
        })
    }

    fn set_status(&self, status: &str) {
        let _ = self.status.set_text(status);
    }

    /// Closes the panel because the capture side is done with it.
    fn dismiss(&self) {
        self.dismissed.store(true, Ordering::Release);
        let _ = self.window.close();
    }
}

/// A capture outline: a see-through, click-through window just larger than
/// the area, drawing a thin red frame outside it. (One window rather than
/// four strips: Windows keeps top-level windows taller than a few pixels.)
struct HudBorder {
    window: Window,
}

impl HudBorder {
    /// The outline's thickness, in desktop pixels.
    const WEIGHT: i32 = 3;

    fn new(scope: &AppScope, area: Rect) -> Result<Self> {
        let outer = area.inflate(Self::WEIGHT);
        let (_, scale) = native::screen_of(outer);
        let frame = Border::new()?
            .border_brush(Brush::new(Color::rgb(0xE5, 0x39, 0x35), 220.0 / 255.0))?
            .border_thickness(Self::WEIGHT as f64 / scale)?;
        let window = hud_window()?
            .width(outer.width as f64 / scale)?
            .height(outer.height as f64 / scale)?
            .position((outer.x, outer.y))?
            .background(Brush::solid(Color::new(0, 0, 0, 0)))?
            .content(Some(&frame))?;
        scope.set_transparent(&window, true)?;
        scope.set_topmost(&window, true)?;
        window.show()?;
        native::prepare(scope, &window, true);
        native::place(native::handle(scope, &window), outer, false);
        Ok(HudBorder { window })
    }

    fn close(&self) {
        let _ = self.window.close();
    }
}

/// Starts following the main view model's signals. Called once, on the UI
/// thread, after the main window is mounted.
pub fn install(scope: &AppScope) -> Result<()> {
    let sink = crate::ui::main().sink;
    let scope = scope.clone();
    // The capture side's panels and outlines, all of them each time.
    Effect::new(move || {
        let json = sink.hud.get();
        let target = scope.clone();
        scope.post(move || apply(&target, &json))
    })?;
    Ok(())
}

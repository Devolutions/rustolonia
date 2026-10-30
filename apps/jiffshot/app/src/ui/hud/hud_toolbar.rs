//! The toolbar under the area while recording: Screen Draw on its own, then
//! Stop, Pause, the pointer and webcam switches (each with a ▾ menu), system
//! sound and the microphone, the time, Restart, Minimize and Cancel.
//! Minimized, it keeps the time and Stop. The capture side sends its state
//! (`recorder.rs`, `toolbar_state`) and takes its actions
//! (`hud:ID:act:NAME`). It never takes the focus and is left out of screen
//! captures.

use super::{native, place_beside, run};
use jiffshot_capture::Rect;
use rustolonia::{
    size_to_content, AppScope, AsControl, Border, Brush, Color, Control, Ellipse, EventSubscription, FontWeight, HorizontalAlignment, Line, MenuFlyout,
    MenuItem, MenuItemToggleType, Orientation, Panel, Path, PlacementMode, Rectangle, Result, Separator, StackPanel, TextBlock, TextWrapping, Upcast,
    VerticalAlignment, Window,
};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};

const PILL: Color = Color::rgb(0x1B, 0x1C, 0x1F);
const GREY: Color = Color::rgb(0x47, 0x4A, 0x51);
const BLUE: Color = Color::rgb(0x2F, 0x80, 0xED);
const GREEN: Color = Color::rgb(0x2E, 0xA0, 0x43);
const LIVE: Color = Color::rgb(0xFF, 0x52, 0x52);
const HELD: Color = Color::rgb(0xFF, 0xC1, 0x07);
const WHITE: Color = Color::rgb(0xFF, 0xFF, 0xFF);
const ICONS: &str = "Segoe Fluent Icons, Segoe MDL2 Assets";

/// Gap between the area and the toolbar (DIPs).
const GAP: f64 = 14.0;

thread_local! {
    /// The open ▾ menu's item subscriptions (one menu at a time).
    static MENU: RefCell<Vec<EventSubscription>> = const { RefCell::new(Vec::new()) };
}

/// The toolbar's latest state, shared with its menus.
type State = Arc<Mutex<Value>>;

fn on(state: &Value, name: &str) -> bool {
    state.get(name).and_then(Value::as_bool).unwrap_or(false)
}

/// `hh:mm:ss`.
fn clock(millis: u64) -> String {
    let s = millis / 1000;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// A rounded, flat, coloured button with a glyph or a shape. (A real button's
/// template restyles hover and press; this one only dims.)
#[derive(Clone)]
struct ToolButton {
    border: Border,
    content: Panel,
    glyph: TextBlock,
    strike: Line,
}

impl ToolButton {
    fn new(
        glyph: &str,
        width: f64,
        height: f64,
        fill: Color,
        size: f64,
        subscriptions: &mut Vec<EventSubscription>,
        click: impl FnMut() + Send + 'static,
    ) -> Result<Self> {
        let glyph = TextBlock::new()?
            .text(glyph)?
            .font_family(ICONS)?
            .font_size(size)?
            .foreground(Brush::solid(WHITE))?
            .horizontal_alignment(HorizontalAlignment::Center)?
            .vertical_alignment(VerticalAlignment::Center)?;
        let half = size * 0.55;
        let strike = Line::new()?
            .start_point((width / 2.0 - half, height / 2.0 + half))?
            .end_point((width / 2.0 + half, height / 2.0 - half))?
            .stroke(Brush::solid(WHITE))?
            .stroke_thickness(1.6)?
            .visible(false)?;
        let content = Panel::new()?.child(&glyph)?.child(&strike)?;
        let border = Border::new()?
            .width(width)?
            .height(height)?
            .background(Brush::solid(fill))?
            .corner_radius(10.0)?
            .child(Some(&content))?;
        let hover = border.clone();
        subscriptions.push(border.subscribe_pointer_entered(move |_| {
            let _ = hover.set_opacity(0.85);
        })?);
        let hover = border.clone();
        subscriptions.push(border.subscribe_pointer_exited(move |_| {
            let _ = hover.set_opacity(1.0);
        })?);
        let mut click = click;
        subscriptions.push(border.subscribe_tapped(move |_| click())?);
        Ok(ToolButton {
            border,
            content,
            glyph,
            strike,
        })
    }

    /// A button showing `shape` instead of a glyph.
    fn with_shape(
        shape: &dyn AsControl,
        width: f64,
        height: f64,
        fill: Color,
        subscriptions: &mut Vec<EventSubscription>,
        click: impl FnMut() + Send + 'static,
    ) -> Result<Self> {
        let button = Self::new("", width, height, fill, 17.0, subscriptions, click)?;
        button.set_shape(shape)?;
        Ok(button)
    }

    /// Replaces the glyph with a shape.
    fn set_shape(&self, shape: &dyn AsControl) -> Result<()> {
        let children = self.content.children()?;
        children.clear()?;
        children.add(shape)
    }

    fn tip(self, text: &str) -> Result<Self> {
        self.border.set_tool_tip_tip(text)?;
        Ok(self)
    }

    fn corners(self, radius: (f64, f64, f64, f64)) -> Result<Self> {
        self.border.set_corner_radius(radius)?;
        Ok(self)
    }

    fn fill(&self, color: Color) -> Result<()> {
        self.border.set_background(Brush::solid(color))
    }
}

/// Filled shapes for Stop, Pause and Resume (the icon font's are outlines).
fn stop_shape(size: f64) -> Result<Control> {
    Ok(Rectangle::new()?
        .width(size)?
        .height(size)?
        .radius_x(3.0)?
        .radius_y(3.0)?
        .fill(Brush::solid(WHITE))?
        .upcast())
}

fn pause_shape() -> Result<Control> {
    let bar = || -> Result<Rectangle> {
        Rectangle::new()?
            .width(6.0)?
            .height(20.0)?
            .radius_x(2.0)?
            .radius_y(2.0)?
            .fill(Brush::solid(WHITE))
    };
    Ok(StackPanel::new()?
        .orientation(Orientation::Horizontal)?
        .spacing(6.0)?
        .horizontal_alignment(HorizontalAlignment::Center)?
        .vertical_alignment(VerticalAlignment::Center)?
        .child(bar()?)?
        .child(bar()?)?
        .upcast())
}

fn resume_shape() -> Result<Control> {
    Ok(Path::new()?
        .data("M 0,0 L 17,10 L 0,20 Z")?
        .fill(Brush::solid(WHITE))?
        .horizontal_alignment(HorizontalAlignment::Center)?
        .vertical_alignment(VerticalAlignment::Center)?
        .margin((4.0, 0.0, 0.0, 0.0))?
        .upcast())
}

fn row(spacing: f64, items: &[&dyn AsControl]) -> Result<StackPanel> {
    let panel = StackPanel::new()?.orientation(Orientation::Horizontal)?.spacing(spacing)?;
    for item in items {
        panel.children()?.add(*item)?;
    }
    Ok(panel)
}

fn column(spacing: f64, items: &[&dyn AsControl]) -> Result<StackPanel> {
    let panel = StackPanel::new()?.spacing(spacing)?;
    for item in items {
        panel.children()?.add(*item)?;
    }
    Ok(panel)
}

fn dot(size: f64) -> Result<Ellipse> {
    Ellipse::new()?
        .width(size)?
        .height(size)?
        .fill(Brush::solid(LIVE))?
        .vertical_alignment(VerticalAlignment::Center)
}

fn time_text(size: f64) -> Result<TextBlock> {
    TextBlock::new()?
        .font_size(size)?
        .font_weight(FontWeight::DemiBold)?
        .foreground(Brush::solid(WHITE))?
        .vertical_alignment(VerticalAlignment::Center)
}

/// A ▾ menu entry sending `action`.
fn menu_item(header: &str, kind: MenuItemToggleType, on: bool, id: u64, action: String) -> Result<MenuItem> {
    let item = MenuItem::new()?.header_text(header)?.toggle_type(kind)?.checked(on)?;
    let subscription = item.subscribe_click(move |_| run(format!("hud:{id}:act:{action}")))?;
    MENU.with(|m| m.borrow_mut().push(subscription));
    Ok(item)
}

/// Opens a menu above `target`: `(header, toggle kind, checked, action)`
/// entries, `None` for a separator.
fn open_menu(target: &Border, id: u64, entries: &[Option<(&str, MenuItemToggleType, bool, String)>]) -> Result<()> {
    MENU.with(|m| m.borrow_mut().clear());
    let menu = MenuFlyout::new()?.placement(PlacementMode::Top)?;
    let items = menu.items()?;
    for entry in entries {
        match entry {
            Some((header, kind, on, action)) => items.add(menu_item(header, *kind, *on, id, action.clone())?)?,
            None => items.add(Separator::new()?)?,
        }
    }
    menu.show_at_with_control(target)
}

fn cursor_menu(target: &Border, id: u64, state: &State) -> Result<()> {
    let s = state.lock().map(|s| s.clone()).unwrap_or_default();
    let check = MenuItemToggleType::CheckBox;
    open_menu(
        target,
        id,
        &[
            Some(("Record the pointer", check, on(&s, "cursor"), "toggle:cursor".into())),
            Some(("Highlight the pointer", check, on(&s, "highlight"), "toggle:highlight".into())),
            Some(("Show clicks", check, on(&s, "clicks"), "toggle:clicks".into())),
        ],
    )
}

fn webcam_menu(target: &Border, id: u64, state: &State) -> Result<()> {
    let s = state.lock().map(|s| s.clone()).unwrap_or_default();
    let corner = s.get("webcam_corner").and_then(Value::as_str).unwrap_or("bottom-right").to_owned();
    let circle = on(&s, "webcam_circle");
    let radio = MenuItemToggleType::Radio;
    let mut entries = vec![
        Some(("Show the webcam", MenuItemToggleType::CheckBox, on(&s, "webcam"), "toggle:webcam".to_owned())),
        None,
    ];
    for (name, value) in [
        ("Top left", "top-left"),
        ("Top right", "top-right"),
        ("Bottom left", "bottom-left"),
        ("Bottom right", "bottom-right"),
    ] {
        entries.push(Some((name, radio, corner == value, format!("corner:{value}"))));
    }
    entries.push(None);
    entries.push(Some(("Circle", radio, circle, "shape:circle".into())));
    entries.push(Some(("Rectangle", radio, !circle, "shape:rectangle".into())));
    open_menu(target, id, &entries)
}

/// The recording toolbar's window.
pub struct RecordingToolbar {
    window: Window,
    state: State,
    draw: ToolButton,
    pause: ToolButton,
    stop: ToolButton,
    mini_stop: ToolButton,
    cursor: ToolButton,
    cursor_menu: ToolButton,
    webcam: ToolButton,
    webcam_menu: ToolButton,
    system: ToolButton,
    microphone: ToolButton,
    time: TextBlock,
    mini_time: TextBlock,
    dot: Ellipse,
    mini_dot: Ellipse,
    note: Border,
    note_text: TextBlock,
    shown_paused: Cell<bool>,
    /// The pause and stop tooltips shown, so they are set only when they change.
    shown_tips: std::cell::RefCell<(String, String)>,
    _subscriptions: Vec<EventSubscription>,
}

impl RecordingToolbar {
    pub fn new(scope: &AppScope, id: u64, area: Rect) -> Result<Self> {
        let state: State = Arc::default();
        let mut subs = Vec::new();
        let send = |action: &'static str| move || run(format!("hud:{id}:act:{action}"));

        let draw = ToolButton::new("\u{E70F}", 54.0, 54.0, GREY, 17.0, &mut subs, send("draw"))?
            .tip("Screen Draw: draw on the recording (A arrow, S box, E ellipse, L line, P pen, 1 step, C color)")?;
        let stop = ToolButton::with_shape(&stop_shape(18.0)?, 68.0, 54.0, BLUE, &mut subs, send("stop"))?.tip("Stop and save (Shift+F10)")?;
        let pause = ToolButton::with_shape(&pause_shape()?, 68.0, 54.0, GREY, &mut subs, send("pause"))?.tip("Pause (Shift+F9)")?;
        let cursor = ToolButton::new("\u{E8B0}", 42.0, 25.0, GREEN, 14.0, &mut subs, send("toggle:cursor"))?
            .tip("Record the pointer")?
            .corners((7.0, 0.0, 0.0, 7.0))?;
        // The menus open above their own buttons: those are made first, the
        // clicks find them through a slot.
        let cursor_target: Arc<Mutex<Option<Border>>> = Arc::default();
        let (target, menu_state) = (cursor_target.clone(), state.clone());
        let cursor_menu = ToolButton::new("\u{E70D}", 24.0, 25.0, GREEN, 9.0, &mut subs, move || {
            if let Some(b) = target.lock().ok().and_then(|t| t.clone()) {
                if let Err(error) = cursor_menu(&b, id, &menu_state) {
                    eprintln!("toolbar: menu failed: {error:?}");
                }
            }
        })?
        .tip("Pointer effects")?
        .corners((0.0, 7.0, 7.0, 0.0))?;
        if let Ok(mut t) = cursor_target.lock() {
            *t = Some(cursor_menu.border.clone());
        }
        let webcam = ToolButton::new("\u{E714}", 42.0, 25.0, GREY, 14.0, &mut subs, send("toggle:webcam"))?
            .tip("Webcam")?
            .corners((7.0, 0.0, 0.0, 7.0))?;
        let webcam_target: Arc<Mutex<Option<Border>>> = Arc::default();
        let (target, menu_state) = (webcam_target.clone(), state.clone());
        let webcam_menu = ToolButton::new("\u{E70D}", 24.0, 25.0, GREY, 9.0, &mut subs, move || {
            if let Some(b) = target.lock().ok().and_then(|t| t.clone()) {
                if let Err(error) = webcam_menu(&b, id, &menu_state) {
                    eprintln!("toolbar: menu failed: {error:?}");
                }
            }
        })?
        .tip("Webcam corner and shape")?
        .corners((0.0, 7.0, 7.0, 0.0))?;
        if let Ok(mut t) = webcam_target.lock() {
            *t = Some(webcam_menu.border.clone());
        }
        let system = ToolButton::new("\u{E767}", 66.0, 25.0, GREEN, 14.0, &mut subs, send("toggle:system"))?.tip("Record system sound")?;
        let microphone = ToolButton::new("\u{E720}", 66.0, 25.0, GREEN, 14.0, &mut subs, send("toggle:microphone"))?.tip("Record the microphone")?;
        let restart = ToolButton::new("\u{E72C}", 46.0, 54.0, GREY, 18.0, &mut subs, send("restart"))?.tip("Restart: throw this take away and record again")?;
        let cancel = ToolButton::new("\u{E711}", 58.0, 54.0, GREY, 20.0, &mut subs, send("cancel"))?.tip("Cancel the recording (Esc)")?;

        let time = time_text(24.0)?.font_features("tnum")?;
        let mini_time = time_text(16.0)?;
        let dot_big = dot(9.0)?;
        let mini_dot = dot(8.0)?;
        let note_text = TextBlock::new()?
            .font_size(12.0)?
            .foreground(Brush::solid(Color::rgb(0xE8, 0xEA, 0xED)))?
            .text_wrapping(TextWrapping::Wrap)?
            .max_width(640.0)?;
        let note = Border::new()?
            .background(Brush::solid(PILL))?
            .corner_radius(8.0)?
            .padding((10.0, 5.0))?
            .margin((66.0, 6.0, 0.0, 0.0))?
            .horizontal_alignment(HorizontalAlignment::Left)?
            .visible(false)?
            .child(Some(&note_text))?;

        let switches = row(
            8.0,
            &[
                &column(
                    4.0,
                    &[
                        &row(1.0, &[&cursor.border, &cursor_menu.border])?,
                        &row(1.0, &[&webcam.border, &webcam_menu.border])?,
                    ],
                )?,
                &column(4.0, &[&system.border, &microphone.border])?,
            ],
        )?;
        let clock_row = row(8.0, &[&dot_big, &time])?.margin((10.0, 0.0, 10.0, 0.0))?;

        // Minimize swaps the whole toolbar for the small one.
        let full_slot: Arc<Mutex<Option<(Control, Control)>>> = Arc::default();
        let slot = full_slot.clone();
        let minimize = ToolButton::new("\u{E921}", 46.0, 54.0, GREY, 16.0, &mut subs, move || {
            if let Some((full, mini)) = slot.lock().ok().and_then(|s| s.clone()) {
                let _ = full.set_visible(false);
                let _ = mini.set_visible(true);
            }
        })?
        .tip("Make the toolbar small")?;
        let slot = full_slot.clone();
        let expand = ToolButton::new("\u{E740}", 36.0, 36.0, GREY, 13.0, &mut subs, move || {
            if let Some((full, mini)) = slot.lock().ok().and_then(|s| s.clone()) {
                let _ = full.set_visible(true);
                let _ = mini.set_visible(false);
            }
        })?
        .tip("Show the whole toolbar")?;
        let mini_stop = ToolButton::with_shape(&stop_shape(13.0)?, 44.0, 36.0, BLUE, &mut subs, send("stop"))?.tip("Stop and save (Shift+F10)")?;

        let pill = Border::new()?
            .background(Brush::solid(PILL))?
            .corner_radius(14.0)?
            .padding(8.0)?
            .child(Some(&row(
                8.0,
                &[
                    &stop.border,
                    &pause.border,
                    &switches,
                    &clock_row,
                    &restart.border,
                    &minimize.border,
                    &cancel.border,
                ],
            )?))?;
        let draw_box = Border::new()?
            .background(Brush::solid(PILL))?
            .corner_radius(12.0)?
            .padding(5.0)?
            .vertical_alignment(VerticalAlignment::Center)?
            .child(Some(&draw.border))?;
        let full = column(0.0, &[&row(12.0, &[&draw_box, &pill])?, &note])?;
        let mini = Border::new()?
            .background(Brush::solid(PILL))?
            .corner_radius(12.0)?
            .padding(6.0)?
            .visible(false)?
            .child(Some(&row(
                8.0,
                &[
                    &mini_stop.border,
                    &row(6.0, &[&mini_dot, &mini_time])?.margin((4.0, 0.0, 4.0, 0.0))?,
                    &expand.border,
                ],
            )?))?;
        if let Ok(mut s) = full_slot.lock() {
            *s = Some((full.upcast(), mini.upcast()));
        }

        let window = super::hud_window()?
            .size_to_content(size_to_content::WIDTHANDHEIGHT)?
            .background(Brush::solid(Color::new(0, 0, 0, 0)))?
            .content(Some(&Panel::new()?.child(&full)?.child(&mini)?))?;
        scope.set_transparent(&window, true)?;
        scope.set_topmost(&window, true)?;
        let placed = window.clone();
        subs.push(window.subscribe_size_changed(move |e| place_beside(&placed, area, (e.new_size.width, e.new_size.height), GAP, true))?);
        window.show()?;
        native::prepare(scope, &window, false);
        Ok(RecordingToolbar {
            window,
            state,
            draw,
            pause,
            stop,
            mini_stop,
            cursor,
            cursor_menu,
            webcam,
            webcam_menu,
            system,
            microphone,
            time,
            mini_time,
            dot: dot_big,
            mini_dot,
            note,
            note_text,
            shown_paused: Cell::new(false),
            shown_tips: Default::default(),
            _subscriptions: subs,
        })
    }

    /// The capture side's state (see `toolbar_state`).
    pub fn update(&self, state: &Value) {
        if let Err(error) = self.show_state(state) {
            eprintln!("toolbar: {error:?}");
        }
        if let Ok(mut s) = self.state.lock() {
            *s = state.clone();
        }
    }

    fn show_state(&self, state: &Value) -> Result<()> {
        let is = |name: &str| on(state, name);
        let text = clock(state.get("millis").and_then(Value::as_u64).unwrap_or(0));
        self.time.set_text(&text)?;
        self.mini_time.set_text(&text)?;
        let paused = is("paused");
        let dot = Brush::solid(if paused { HELD } else { LIVE });
        self.dot.set_fill(dot)?;
        self.mini_dot.set_fill(dot)?;
        if paused != self.shown_paused.get() {
            self.pause.set_shape(&if paused { resume_shape()? } else { pause_shape()? })?;
            self.shown_paused.set(paused);
        }
        // The keys the capture side registered: another app may hold the usual ones.
        let key = |name: &str| state.get("keys").and_then(|k| k.get(name)).and_then(Value::as_str).map(|k| format!(" ({k})")).unwrap_or_default();
        let tips = (format!("{}{}", if paused { "Resume" } else { "Pause" }, key("pause")), format!("Stop and save{}", key("stop")));
        if *self.shown_tips.borrow() != tips {
            self.pause.border.set_tool_tip_tip(&tips.0)?;
            self.stop.border.set_tool_tip_tip(&tips.1)?;
            self.mini_stop.border.set_tool_tip_tip(&tips.1)?;
            *self.shown_tips.borrow_mut() = tips;
        }
        self.draw.border.set_visible(is("draw_offered"))?;
        self.draw.fill(if is("draw") { BLUE } else { GREY })?;
        let switch = |name: &str| if is(name) { GREEN } else { GREY };
        self.cursor.fill(switch("cursor"))?;
        self.cursor_menu.fill(switch("cursor"))?;
        self.webcam.fill(switch("webcam"))?;
        self.webcam_menu.fill(switch("webcam"))?;
        self.system.fill(switch("system"))?;
        self.system.glyph.set_text(if is("system") { "\u{E767}" } else { "\u{E74F}" })?;
        self.microphone.fill(switch("microphone"))?;
        self.microphone.strike.set_visible(!is("microphone"))?;
        let note = state.get("note").and_then(Value::as_str).unwrap_or_default();
        self.note_text.set_text(note)?;
        self.note.set_visible(!note.is_empty())
    }

    pub fn close(&self) {
        let _ = self.window.close();
    }
}

#[cfg(test)]
mod tests {
    use super::clock;

    #[test]
    fn the_clock_shows_hours_minutes_seconds() {
        assert_eq!(clock(0), "00:00:00");
        assert_eq!(clock(3_725_999), "01:02:05");
    }
}

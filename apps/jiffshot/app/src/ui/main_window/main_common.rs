//! What the editor window's parts share: model calls and commands, the
//! window's theme colors, its recurring little controls (sections, chips
//! and bars), lists that refill themselves, and value conversions.

use crate::ui::canvas_view::CanvasView;
use crate::ui::UiOnly;
use crate::vm::{MainViewModel, MainViewModelSink};
use rustolonia::{
    AnyEl, AppScope, Brush, Button, Color, ComboBox, ComboBoxItem, Command, ContextMenu, Control, Effect, El, FontWeight, ItemList, ReactiveScope, Rectangle, Result, Signal, TextBlock, TextBox, Upcast,
    Variant,
};
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// A call on the editor's model, as a plain function so `Send` callbacks can
/// carry it.
pub type Call = fn(&mut dyn MainViewModel) -> Result<()>;

/// What the window's builders share. UI thread only.
#[derive(Clone)]
pub struct Ctx {
    pub scope: AppScope,
    pub sink: MainViewModelSink,
    pub theme: Theme,
    pub canvas: CanvasView,
    /// The window's height, for the library view.
    pub window_height: Signal<f64>,
    /// The properties panel's text box, which [`crate::ui::main_window`]
    /// focuses when the model asks.
    pub text_editor: rustolonia::NodeRef<TextBox>,
}

impl Ctx {
    /// The canvas, for `Send` callbacks.
    pub fn canvas_handle(&self) -> UiOnly<CanvasView> {
        UiOnly::new(self.canvas.clone())
    }
}

/// Runs `f` on the editor's model.
pub fn call(f: Call) {
    crate::ui::main().model.run(f);
}

/// Sends `action` to the model's `Run` command.
pub fn run(action: impl Into<String>) {
    let action = action.into();
    crate::ui::main().model.run(move |m| m.run(action));
}

/// A command that runs `f`, enabled while `enabled` is.
pub fn command(enabled: Signal<bool>, f: Call) -> Command {
    let execute = move |_: Variant| {
        call(f);
        Ok(())
    };
    Command::with_can_execute(enabled, execute).unwrap_or_else(|_| Command::new(execute))
}

/// A command that sends `action` to `Run`.
pub fn action(action: impl Into<String>) -> Command {
    let action = action.into();
    Command::new(move |_| {
        run(action.clone());
        Ok(())
    })
}

/// A command that runs `f` (on the UI thread, where commands run).
pub fn act(mut f: impl FnMut() + Send + 'static) -> Command {
    Command::new(move |_| {
        f();
        Ok(())
    })
}

// ---- theme ------------------------------------------------------------------

/// The window's own colors, which follow the light or dark theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Window,
    Panel,
    Tray,
    Chip,
    Well,
    Border,
    BorderStrong,
    Muted,
    Caption,
    Subtle,
    /// Ordinary text, for controls that switch between it and another color.
    Text,
}

impl Tone {
    /// `#AARRGGBB` in the dark and the light theme.
    fn argb(self, dark: bool) -> u32 {
        let (dark_argb, light_argb) = match self {
            Tone::Window => (0xFF1B1C1F, 0xFFF3F4F6),
            Tone::Panel => (0xFF232428, 0xFFFFFFFF),
            Tone::Tray => (0xFF1F2023, 0xFFECEEF1),
            Tone::Chip => (0xFF26282C, 0xFFE8EAED),
            Tone::Well => (0xFF2C2E33, 0xFFE1E4E8),
            Tone::Border => (0xFF2E3034, 0xFFD5D8DD),
            Tone::BorderStrong => (0xFF3A3C40, 0xFFC4C8CE),
            Tone::Muted => (0xFF8A8F98, 0xFF6B7280),
            Tone::Caption => (0xFFC0C4CC, 0xFF4B5563),
            Tone::Subtle => (0xFFC8CACE, 0xFF374151),
            Tone::Text => (0xFFF2F3F5, 0xFF1F2328),
        };
        if dark {
            dark_argb
        } else {
            light_argb
        }
    }
}

/// Whether the window shows its dark colors; follows the application's
/// actual theme variant.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub dark: Signal<bool>,
}

impl Theme {
    /// Dark until the application's theme says otherwise.
    pub fn new() -> Self {
        Self { dark: Signal::new(true) }
    }

    /// `tone` now, tracked.
    pub fn get(self, tone: Tone) -> Brush {
        solid(tone.argb(self.dark.get()))
    }

    /// A brush binding that follows the theme.
    pub fn brush(self, tone: Tone) -> impl FnMut() -> Brush + 'static {
        move || self.get(tone)
    }
}

/// A solid `#AARRGGBB` brush.
pub fn solid(argb: u32) -> Brush {
    Brush::solid(Color::from(argb))
}

pub const TRANSPARENT: u32 = 0x00000000;
pub const WHITE: u32 = 0xFFFFFFFF;
/// The active tool's background.
pub const ACCENT: u32 = 0xFF2F5DA8;
/// The current capture's outline and the picked mark.
pub const BLUE: u32 = 0xFF3D8BFF;
/// Picked captures, favorites and unsaved edits.
pub const ORANGE: u32 = 0xFFF5A623;
/// Capture and narration.
pub const RED: u32 = 0xFFB3261E;

// ---- converters ---------------------------------------------------------------

/// `#AARRGGBB` text to a brush; transparent when it does not parse
/// (`ColorBrushConverter`).
pub fn color_brush(hex: &str) -> Brush {
    Color::parse_hex(hex).map(Brush::solid).unwrap_or_else(|| solid(TRANSPARENT))
}

/// Seconds as `m:ss.f` (`TimeConverter`).
pub fn time_text(seconds: f64) -> String {
    let seconds = if seconds.is_finite() { seconds.max(0.0) } else { 0.0 };
    format!("{}:{:04.1}", (seconds / 60.0) as i64, seconds % 60.0)
}

/// A slider's value as its label shows it: whole numbers without decimals.
pub fn number_text(value: f64) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// A file URI's or path's file name, decoded (recent file headers).
pub fn file_name(uri: &str) -> String {
    let path = uri.strip_prefix("file:///").unwrap_or(uri);
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    crate::ui::percent_decode(name)
}

// ---- recurring controls -----------------------------------------------------

/// A panel section's heading (`TextBlock.section`).
pub fn section(theme: Theme, text: &str) -> El<TextBlock> {
    TextBlock::el().text(text.to_owned()).font_size(11.0).font_weight(FontWeight::DemiBold).foreground(theme.brush(Tone::Muted)).margin((0.0, 10.0, 0.0, 4.0))
}

/// Small muted text.
pub fn note(theme: Theme, text: &str) -> El<TextBlock> {
    TextBlock::el().text(text.to_owned()).font_size(11.0).foreground(theme.brush(Tone::Muted)).text_wrapping(rustolonia::TextWrapping::Wrap)
}

/// A compact button for panel actions (`Button.chip`).
pub fn chip(label: &str) -> El<Button> {
    Button::el().content(label.to_owned()).padding((8.0, 3.0)).margin((0.0, 0.0, 4.0, 4.0)).font_size(11.5)
}

/// A flat status-bar button (`Button.bar`).
pub fn bar(label: impl Into<AnyEl>) -> El<Button> {
    Button::el().content(label.into()).padding((8.0, 2.0)).min_height(0.0).font_size(12.0).background(solid(TRANSPARENT))
}

/// A thin vertical divider between groups of buttons.
pub fn rule(theme: Theme, margin: (f64, f64)) -> El<Rectangle> {
    Rectangle::el().width(1.0).fill(theme.brush(Tone::BorderStrong)).margin(margin)
}

/// A label that shows a slider's value, docked right of it.
pub fn value_label(value: Signal<f64>) -> El<TextBlock> {
    TextBlock::el()
        .text(move || number_text(value.get()))
        .width(36.0)
        .text_alignment(rustolonia::TextAlignment::Right)
        .vertical_alignment(rustolonia::VerticalAlignment::Center)
        .dock_panel_dock(rustolonia::Dock::Right)
}

/// Gives the control a context menu of `items`.
pub fn with_context_menu<T>(el: El<T>, items: Vec<AnyEl>) -> El<T>
where
    T: Upcast<Control> + 'static,
{
    el.with_op(move |scope, control| {
        let menu = ContextMenu::el().items_iter(items).build(scope)?;
        Upcast::<Control>::upcast_ref(control).set_context_menu(Some(&menu))
    })
}

/// Keeps the items of a menu, menu item or flyout built from `data`: they
/// are rebuilt whenever what `data` reads changes to something new.
pub fn reactive_items<T, D, F, B>(el: El<T>, items: fn(&T) -> Result<ItemList>, mut data: F, build: B) -> El<T>
where
    T: Clone + 'static,
    D: PartialEq + 'static,
    F: FnMut() -> D + 'static,
    B: Fn(&D) -> Vec<AnyEl> + 'static,
{
    el.with_op(move |scope, owner| {
        let (scope, owner) = (scope.clone(), owner.clone());
        let shown: RefCell<Option<D>> = RefCell::new(None);
        let built_in: Cell<Option<ReactiveScope>> = Cell::new(None);
        Effect::new(move || {
            let next = data();
            if shown.borrow().as_ref() == Some(&next) {
                return Ok(());
            }
            if let Some(previous) = built_in.take() {
                previous.dispose();
            }
            let reactive = ReactiveScope::new();
            let controls = reactive.run(|| build(&next).into_iter().map(|el| el.build(&scope)).collect::<Result<Vec<_>>>());
            built_in.set(Some(reactive));
            *shown.borrow_mut() = Some(next);
            let list = items(&owner)?;
            list.clear()?;
            for control in controls? {
                list.add(control)?;
            }
            Ok(())
        })?;
        Ok(())
    })
}

/// A list of strings bound to a string property: picking an entry sets `value`, and
/// `value` selects its entry (none when it is not listed).
pub fn choice_box(theme: Theme, items: impl Fn() -> Vec<String> + 'static, value: Signal<String>) -> El<ComboBox> {
    choices(theme, items, move || value.get(), move |picked| value.set_if_changed(picked))
}

/// A list that runs `pick` with the chosen entry and goes back to showing
/// none (Quick styles' Theme… list).
pub fn action_box(theme: Theme, items: impl Fn() -> Vec<String> + 'static, pick: impl Fn(String) + Send + 'static) -> El<ComboBox> {
    choices(theme, items, String::new, pick).with_op(|scope, combo| {
        let (scope, reset) = (scope.clone(), combo.clone());
        combo.clone().on_selection_changed(&scope.clone(), move |_| {
            if reset.get_selected_index().unwrap_or(-1) >= 0 {
                let reset = reset.clone();
                let _ = scope.post(move || {
                    let _ = reset.set_selected_index(-1);
                });
            }
        })?;
        Ok(())
    })
}

fn choices(theme: Theme, items: impl Fn() -> Vec<String> + 'static, current: impl Fn() -> String + 'static, pick: impl Fn(String) + Send + 'static) -> El<ComboBox> {
    ComboBox::el().with_op(move |scope, combo| {
        let shown: Arc<Mutex<Shown>> = Arc::default();
        // Set while the list or its selection is changed from here.
        let syncing = Arc::new(AtomicBool::new(false));
        {
            let (combo, shown, syncing) = (combo.clone(), shown.clone(), syncing.clone());
            Effect::new(move || {
                let list = items();
                let value = current();
                let text = theme.get(Tone::Text);
                syncing.store(true, Ordering::SeqCst);
                let result = show_choices(&combo, &shown, list, text, &value);
                syncing.store(false, Ordering::SeqCst);
                result
            })?;
        }
        let reader = combo.clone();
        combo.clone().on_selection_changed(scope, move |_| {
            if syncing.load(Ordering::SeqCst) {
                return;
            }
            let index = reader.get_selected_index().unwrap_or(-1);
            let list = shown.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(picked) = usize::try_from(index).ok().and_then(|i| list.items.get(i)).cloned() {
                drop(list);
                pick(picked);
            }
        })?;
        Ok(())
    })
}

/// What a list shows: its entries, in the text color they were made with
/// (an entry draws itself in the closed box, outside the theme's styles).
#[derive(Default)]
struct Shown {
    items: Vec<String>,
    text: Option<Brush>,
}

/// Lists `list` in `combo` (when it or the text color changed) and selects
/// `value`.
fn show_choices(combo: &ComboBox, shown: &Mutex<Shown>, list: Vec<String>, text: Brush, value: &str) -> Result<()> {
    let mut current = shown.lock().unwrap_or_else(PoisonError::into_inner);
    if current.items != list || current.text != Some(text) {
        let items = combo.items()?;
        // Rebuilding the entries drops the selection; it is restored below.
        combo.set_selected_index(-1)?;
        items.clear()?;
        for entry in &list {
            items.add(ComboBoxItem::new()?.content(Some(&TextBlock::new()?.text(entry)?.foreground(text)?))?)?;
        }
        *current = Shown { items: list, text: Some(text) };
    }
    let index = current.items.iter().position(|s| s == value).and_then(|i| i32::try_from(i).ok()).unwrap_or(-1);
    if combo.get_selected_index()? != index {
        combo.set_selected_index(index)?;
    }
    Ok(())
}

/// A toggle that shows `value` (one way) and runs `toggle` when clicked; it
/// shows `value` again afterwards, so a toggle the model refused springs
/// back.
pub fn one_way_toggle(value: impl Fn() -> bool + Clone + Send + 'static, toggle: impl Fn() + Send + 'static) -> El<rustolonia::ToggleButton> {
    let shown = value.clone();
    rustolonia::ToggleButton::el().checked(move || Some(shown())).with_op(move |scope, button| {
        let target = button.clone();
        button.clone().on_click(scope, move |_| {
            toggle();
            let _ = target.set_checked(Some(value()));
        })?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_text_shows_minutes_and_tenths() {
        assert_eq!(time_text(0.0), "0:00.0");
        assert_eq!(time_text(65.25), "1:05.2");
        assert_eq!(time_text(f64::NAN), "0:00.0");
    }

    #[test]
    fn colors_that_do_not_parse_are_transparent() {
        assert_eq!(color_brush("#FF102030"), Brush::solid(Color::new(0xFF, 0x10, 0x20, 0x30)));
        assert_eq!(color_brush("not a color"), solid(TRANSPARENT));
    }

    #[test]
    fn labels_come_from_uris_and_values() {
        assert_eq!(file_name("file:///C:/Captures/My%20shot.png"), "My shot.png");
        assert_eq!(number_text(28.0), "28");
        assert_eq!(number_text(2.46), "2.5");
    }
}

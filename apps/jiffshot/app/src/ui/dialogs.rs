//! The editor's small modal dialogs: keyboard shortcuts, About, resize image
//! and trim (Preferences is its own window).

use rustolonia::{
    AppScope, AsControl, Button, CheckBox, Dock, DockPanel, FontWeight, Grid, HorizontalAlignment, NumericUpDown, Orientation, Result, StackPanel, TextBlock, TextWrapping, Thickness, VerticalAlignment, Window,
    WindowStartupLocation,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// `SizeToContent.WidthAndHeight`.
const SIZE_TO_CONTENT: i32 = 3;

/// What the OK button closes a dialog with.
const ACCEPTED: &str = "ok";

/// A dialog window: `body` above OK and Cancel, or Close alone when `ok` is
/// `None`.
fn frame(scope: &AppScope, title: &str, body: &impl AsControl, ok: Option<&str>) -> Result<Window> {
    let window = Window::new()?
        .title(title)?
        .size_to_content(SIZE_TO_CONTENT)?
        .can_resize(false)?
        .show_in_taskbar(false)?
        .window_startup_location(WindowStartupLocation::CenterOwner)?
        .icon(super::icon_file())?;
    let buttons = StackPanel::new()?.orientation(Orientation::Horizontal)?.spacing(8.0)?.horizontal_alignment(HorizontalAlignment::Right)?.margin(Thickness::new(0.0, 16.0, 0.0, 0.0))?;
    let button = |label: &str| -> Result<Button> { Button::new()?.content_text(label)?.min_width(80.0)?.horizontal_content_alignment(HorizontalAlignment::Center) };
    if let Some(ok) = ok {
        let closes = window.clone();
        buttons.children()?.add(button(ok)?.default(true)?.on_click(scope, move |_| {
            let _ = closes.close_with_object(ACCEPTED);
        })?)?;
    }
    let closes = window.clone();
    buttons.children()?.add(button(if ok.is_some() { "Cancel" } else { "Close" })?.cancel(true)?.on_click(scope, move |_| {
        let _ = closes.close();
    })?)?;
    let root = DockPanel::new()?.margin(Thickness::uniform(20.0))?;
    root.children()?.add(buttons.dock_panel_dock(Dock::Bottom)?)?;
    root.children()?.add(body)?;
    window.set_content(Some(&root))?;
    Ok(window)
}

/// Shows `dialog` over `owner`; `done` runs on the UI thread with whether
/// OK closed it.
fn ask(scope: &AppScope, owner: &Window, dialog: &Window, done: impl FnOnce(bool) + Send + 'static) -> Result<()> {
    let operation = scope.show_dialog(owner, dialog)?;
    scope.spawn(async move {
        let accepted = matches!(operation.await, Ok(Some(result)) if result == ACCEPTED);
        done(accepted);
    })
}

fn label(text: &str) -> Result<TextBlock> {
    TextBlock::new()?.text(text)?.vertical_alignment(VerticalAlignment::Center)?.margin(Thickness::new(0.0, 0.0, 12.0, 0.0))
}

fn note(text: &str) -> Result<TextBlock> {
    TextBlock::new()?.text(text)?.font_size(11.5)?.opacity(0.7)?.text_wrapping(TextWrapping::Wrap)?.max_width(380.0)?.margin(Thickness::new(0.0, 6.0, 0.0, 0.0))
}

fn number(value: u32, min: u32, max: u32) -> Result<NumericUpDown> {
    NumericUpDown::new()?.minimum(min.to_string())?.maximum(max.to_string())?.value(value.to_string())?.increment("1")?.format_string("0")?.width(150.0)
}

/// A number box's value, rounded; `fallback` when it is empty or not a
/// number.
fn value_of(number: &NumericUpDown, fallback: u32) -> u32 {
    number.get_value().ok().flatten().and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite() && *v >= 0.0).map(|v| v.round() as u32).unwrap_or(fallback)
}

/// A two-column grid of labels and controls.
fn rows(rows: &[(&str, &NumericUpDown)]) -> Result<Grid> {
    let grid = Grid::new()?.column_definitions("Auto,*")?.row_definitions(vec!["Auto"; rows.len()].join(","))?.row_spacing(8.0)?;
    for (i, (text, control)) in rows.iter().enumerate() {
        let row = i32::try_from(i).unwrap_or(i32::MAX);
        grid.children()?.add(label(text)?.grid_row(row)?)?;
        grid.children()?.add((*control).clone().grid_row(row)?.grid_column(1)?)?;
    }
    Ok(grid)
}

/// Image ▸ Resize image: pixels or percent, keeping proportions or not.
/// `done` gets the new size when the user resizes.
pub fn resize_image(scope: &AppScope, owner: &Window, w: u32, h: u32, done: impl FnOnce(u32, u32) + Send + 'static) -> Result<()> {
    let (w, h) = (w.max(1), h.max(1));
    let width = number(w, 1, 16384)?;
    let height = number(h, 1, 16384)?;
    let percent = number(100, 1, 1000)?;
    let keep = CheckBox::new()?.content_text("Keep proportions")?.checked(Some(true))?;
    // Set while one box updates the others.
    let syncing = Arc::new(AtomicBool::new(false));
    let linked = |source: &NumericUpDown, update: Box<dyn Fn(f64) + Send>| -> Result<()> {
        let syncing = syncing.clone();
        source.clone().on_value_changed(scope, move |args| {
            let Some(v) = args.new_value.as_deref().and_then(|v| v.trim().parse::<f64>().ok()) else { return };
            if !syncing.swap(true, Ordering::SeqCst) {
                update(v);
                syncing.store(false, Ordering::SeqCst);
            }
        })?;
        Ok(())
    };
    let (fw, fh) = (f64::from(w), f64::from(h));
    let set = |number: &NumericUpDown, value: f64| {
        let _ = number.set_value(format!("{}", value.round().max(1.0)));
    };
    {
        let (height, percent, keep) = (height.clone(), percent.clone(), keep.clone());
        linked(
            &width,
            Box::new(move |v| {
                if keep.get_is_checked().ok().flatten() == Some(true) {
                    set(&height, v * fh / fw);
                    set(&percent, v * 100.0 / fw);
                }
            }),
        )?;
    }
    {
        let (width, percent, keep) = (width.clone(), percent.clone(), keep.clone());
        linked(
            &height,
            Box::new(move |v| {
                if keep.get_is_checked().ok().flatten() == Some(true) {
                    set(&width, v * fw / fh);
                    set(&percent, v * 100.0 / fh);
                }
            }),
        )?;
    }
    {
        let (width, height) = (width.clone(), height.clone());
        linked(
            &percent,
            Box::new(move |p| {
                set(&width, fw * p / 100.0);
                set(&height, fh * p / 100.0);
            }),
        )?;
    }
    let body = StackPanel::new()?;
    let children = body.children()?;
    children.add(TextBlock::new()?.text(format!("Now {w} × {h} pixels."))?.margin(Thickness::new(0.0, 0.0, 0.0, 12.0))?)?;
    children.add(rows(&[("Width (px)", &width), ("Height (px)", &height), ("Percent", &percent)])?)?;
    children.add(StackPanel::new()?.height(8.0)?)?;
    children.add(keep)?;
    children.add(note("Objects on the image are scaled with it.")?)?;
    let dialog = frame(scope, "Resize image", &body, Some("Resize"))?;
    ask(scope, owner, &dialog, move |accepted| {
        if accepted {
            done(value_of(&width, w).max(1), value_of(&height, h).max(1));
        }
    })
}

/// Image ▸ Trim: pixels to remove from each side. `done` gets the left,
/// top, right and bottom amounts when the user trims.
pub fn trim(scope: &AppScope, owner: &Window, w: u32, h: u32, done: impl FnOnce([u32; 4]) + Send + 'static) -> Result<()> {
    let (max_x, max_y) = (w.saturating_sub(1), h.saturating_sub(1));
    let left = number(0, 0, max_x)?;
    let top = number(0, 0, max_y)?;
    let right = number(0, 0, max_x)?;
    let bottom = number(0, 0, max_y)?;
    let body = StackPanel::new()?;
    let children = body.children()?;
    children.add(TextBlock::new()?.text("Pixels to remove from each side:")?.margin(Thickness::new(0.0, 0.0, 0.0, 12.0))?)?;
    children.add(rows(&[("Left", &left), ("Top", &top), ("Right", &right), ("Bottom", &bottom)])?)?;
    children.add(note("Image ▸ Auto trim removes edges of one solid color by itself.")?)?;
    let dialog = frame(scope, "Trim", &body, Some("Trim"))?;
    ask(scope, owner, &dialog, move |accepted| {
        if accepted {
            done([value_of(&left, 0), value_of(&top, 0), value_of(&right, 0), value_of(&bottom, 0)]);
        }
    })
}

/// Help ▸ Keyboard shortcuts' list.
const SHORTCUTS: [(&str, &str); 26] = [
    ("Ctrl+Shift+N", "New capture"),
    ("Ctrl+N", "New image"),
    ("Ctrl+O", "Open"),
    ("Ctrl+S", "Save"),
    ("Ctrl+Shift+S", "Save as"),
    ("Ctrl+E", "Export"),
    ("Ctrl+P", "Print"),
    ("Ctrl+W", "Close"),
    ("Ctrl+Shift+V", "New from clipboard"),
    ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
    ("Ctrl+C / Ctrl+X / Ctrl+V", "Copy / cut / paste"),
    ("Ctrl+Shift+C", "Copy all"),
    ("Ctrl+A / Ctrl+D", "Select all / deselect"),
    ("Ctrl+Shift+I", "Invert selection"),
    ("Ctrl+Shift+G / U", "Group / ungroup"),
    ("Ctrl+Shift+F / B", "Bring forward / send backward"),
    ("Ctrl+T / Ctrl+Shift+T", "Flatten selected / all"),
    ("Ctrl+1 … Ctrl+9", "The first nine tools"),
    ("Ctrl++ / Ctrl+−", "Zoom in / out"),
    ("Ctrl+0", "Actual size"),
    ("Ctrl+Alt+0", "Fit"),
    ("Ctrl+G", "Pixel grid"),
    ("Ctrl+L", "Library view"),
    ("Ctrl+,", "Editor preferences"),
    ("Esc", "Deselect"),
    ("F1", "These shortcuts"),
];

/// Shows the keyboard shortcuts over `owner`.
pub fn keyboard_shortcuts(scope: &AppScope, owner: &Window) -> Result<()> {
    let grid = Grid::new()?.column_definitions("Auto,*")?.row_definitions(vec!["Auto"; SHORTCUTS.len()].join(","))?.row_spacing(4.0)?;
    for (i, (keys, what)) in SHORTCUTS.iter().enumerate() {
        let row = i32::try_from(i).unwrap_or(i32::MAX);
        grid.children()?.add(TextBlock::new()?.text(keys)?.font_family("Consolas, Cascadia Mono, monospace")?.margin(Thickness::new(0.0, 0.0, 18.0, 0.0))?.grid_row(row)?)?;
        grid.children()?.add(TextBlock::new()?.text(what)?.grid_row(row)?.grid_column(1)?)?;
    }
    let dialog = frame(scope, "Keyboard shortcuts", &grid, None)?;
    ask(scope, owner, &dialog, |_| ())
}

/// Shows the About dialog over `owner`.
pub fn about(scope: &AppScope, owner: &Window) -> Result<()> {
    let body = StackPanel::new()?.spacing(6.0)?;
    let children = body.children()?;
    children.add(TextBlock::new()?.text("Jiffshot Editor")?.font_size(18.0)?.font_weight(FontWeight::DemiBold)?)?;
    children.add(TextBlock::new()?.text(format!("Version {}", env!("CARGO_PKG_VERSION")))?)?;
    children.add(TextBlock::new()?.text("Screen capture and image editing.")?.margin(Thickness::new(0.0, 6.0, 0.0, 0.0))?)?;
    let dialog = frame(scope, "About Jiffshot", &body, None)?;
    ask(scope, owner, &dialog, |_| ())
}

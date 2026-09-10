//! Counter sample built entirely through the imperative generated bindings:
//! no AXAML, no compiled presentation, no view-model IR.
//!
//! Run with a packaged host next to the executable, or point
//! `AVN_HOST_NATIVE_LIB` at a published `Avalonia.Host` native library.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

use avalonia::{
    App, AppScope, Brush, Button, Color, HorizontalAlignment, ListBox, Orientation, Result, Slider,
    StackPanel, TextBlock, TextBox, Thickness, Window,
};

fn main() -> Result<()> {
    App::load_from_env()?.run(build_ui)
}

fn build_ui(scope: &AppScope) -> Result<()> {
    let count = Arc::new(AtomicI32::new(0));

    let heading = TextBlock::new()?
        .text("Rustolonia, imperatively")?
        .font_size(20.0)?
        .margin(Thickness::new(12.0, 12.0, 12.0, 0.0))?;

    let count_label = TextBlock::new()?
        .text("0")?
        .font_size(40.0)?
        .horizontal_alignment(HorizontalAlignment::Center)?
        .margin(Thickness::uniform(8.0))?;

    let log = ListBox::new()?
        .margin(Thickness::symmetric(12.0, 8.0))?
        .height(160.0)?;

    // Wave P payload: the selection-changed args interface carries the
    // added/removed item lists across the ABI.
    let selection_label = TextBlock::new()?
        .text("Selection: none")?
        .margin(Thickness::symmetric(12.0, 0.0))?;
    let log = log.on_selection_changed(scope, {
        let selection_label = selection_label.clone();
        move |args| {
            let added = args.added_items_count().unwrap_or(0);
            let removed = args.removed_items_count().unwrap_or(0);
            let _ = selection_label
                .set_text(format!("Selection: +{added} item(s), -{removed} item(s)"));
        }
    })?;

    // Wave P payload: RangeBase.ValueChanged carries the old/new pair as fields.
    let slider_label = TextBlock::new()?
        .text("Slider: 0")?
        .margin(Thickness::symmetric(12.0, 0.0))?;
    let slider = Slider::new()?
        .margin(Thickness::symmetric(12.0, 4.0))?
        .minimum(0.0)?
        .maximum(100.0)?
        .value(0.0)?
        .on_value_changed(scope, {
            let slider_label = slider_label.clone();
            move |args| {
                let _ = slider_label.set_text(format!("Slider: {:.0}", args.new_value));
            }
        })?;

    let increment = Button::new()?
        .content(Some(&TextBlock::new()?.text("Increment")?))?
        .margin(Thickness::uniform(4.0))?
        .on_click(scope, {
            let count = Arc::clone(&count);
            let count_label = count_label.clone();
            let log = log.clone();
            move |_| {
                let value = count.fetch_add(1, Ordering::Relaxed) + 1;
                let _ = count_label.set_text(value.to_string());
                if let Ok(row) = TextBlock::new().and_then(|row| row.text(format!("Count {value}")))
                {
                    if let Ok(items) = log.items() {
                        let _ = items.add(row);
                    }
                }
            }
        })?;

    let reset = Button::new()?
        .content(Some(&TextBlock::new()?.text("Reset")?))?
        .margin(Thickness::uniform(4.0))?
        .on_click(scope, {
            let count = Arc::clone(&count);
            let count_label = count_label.clone();
            let log = log.clone();
            move |_| {
                count.store(0, Ordering::Relaxed);
                let _ = count_label.set_text("0");
                if let Ok(items) = log.items() {
                    let _ = items.clear();
                }
            }
        })?;

    let buttons = StackPanel::new()?
        .orientation(Orientation::Horizontal)?
        .horizontal_alignment(HorizontalAlignment::Center)?;
    buttons.children()?.add(increment)?;
    buttons.children()?.add(reset)?;

    let echo = TextBlock::new()?.margin(Thickness::symmetric(12.0, 4.0))?;
    echo.set_text("Echo: ")?;

    let input = TextBox::new()?
        .margin(Thickness::symmetric(12.0, 4.0))?
        .placeholder_text("Type here")?;
    let input_reader = input.clone();
    let input = input.on_text_changed(scope, {
        let echo = echo.clone();
        move |_| {
            let text = input_reader
                .get_text()
                .unwrap_or_default()
                .unwrap_or_default();
            let _ = echo.set_text(format!("Echo: {text}"));
        }
    })?;

    let root = StackPanel::new()?
        .orientation(Orientation::Vertical)?
        .spacing(4.0)?;
    root.set_background(Brush::solid(Color::new(255, 245, 245, 250)))?;
    let children = root.children()?;
    children.add(heading)?;
    children.add(count_label)?;
    children.add(buttons)?;
    children.add(input)?;
    children.add(echo)?;
    children.add(slider)?;
    children.add(slider_label)?;
    children.add(log)?;
    children.add(selection_label)?;

    let window = Window::new()?
        .title("Imperative counter")?
        .width(420.0)?
        .height(680.0)?;
    window.set_content(Some(&root))?;
    scope.mount(window)?;
    Ok(())
}

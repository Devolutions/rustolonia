//! Counter sample built entirely through the imperative generated bindings:
//! no AXAML, no compiled presentation, no view-model IR.
//!
//! Run with a packaged host next to the executable, or point
//! `RUSTOLONIA_HOST_LIB` at a published `Avalonia.Host` native library.

use std::cell::Cell;
use std::rc::Rc;

use rustolonia::{
    App, AppScope, Border, Brush, Button, Color, Decimal, HorizontalAlignment, KeyModifiers,
    ListBox, NavigationMethod, NumericUpDown, Orientation, Result, Slider, StackPanel, TextBlock,
    TextBox, Thickness, Window,
};

fn main() -> Result<()> {
    App::load_from_env()?.run(build_ui)
}

fn build_ui(scope: &AppScope) -> Result<()> {
    let count = Rc::new(Cell::new(0_i32));

    let heading = TextBlock::new()?;
    heading.set_text("Rustolonia, imperatively")?;
    heading.set_font_size(20.0)?;
    heading.set_margin(Thickness::new(12.0, 12.0, 12.0, 0.0))?;

    let count_label = TextBlock::new()?;
    count_label.set_text("0")?;
    count_label.set_font_size(40.0)?;
    count_label.set_horizontal_alignment(HorizontalAlignment::Center)?;
    count_label.set_margin(Thickness::uniform(8.0))?;

    let log = ListBox::new()?;
    log.set_margin(Thickness::symmetric(12.0, 8.0))?;
    log.set_height(160.0)?;

    let selection_label = TextBlock::new()?;
    selection_label.set_text("Selection: none")?;
    selection_label.set_margin(Thickness::symmetric(12.0, 0.0))?;
    log.on_selection_changed(scope, {
        let selection_label = selection_label.clone();
        move |args| {
            let added = args.added_items.len();
            let removed = args.removed_items.len();
            selection_label.set_text(format!("Selection: +{added} item(s), -{removed} item(s)"))
        }
    })?;

    let slider_label = TextBlock::new()?;
    slider_label.set_text("Slider: 0")?;
    slider_label.set_margin(Thickness::symmetric(12.0, 0.0))?;
    let slider = Slider::new()?;
    slider.set_margin(Thickness::symmetric(12.0, 4.0))?;
    slider.set_minimum(0.0)?;
    slider.set_maximum(100.0)?;
    slider.set_value(0.0)?;
    slider.on_value_changed(scope, {
        let slider_label = slider_label.clone();
        move |args| slider_label.set_text(format!("Slider: {:.0}", args.new_value))
    })?;

    let wheel_label = TextBlock::new()?;
    wheel_label.set_text("Wheel: -")?;
    wheel_label.set_margin(Thickness::symmetric(12.0, 0.0))?;
    let wheel_zone = Border::new()?;
    wheel_zone.set_margin(Thickness::symmetric(12.0, 4.0))?;
    wheel_zone.set_padding(Thickness::uniform(6.0))?;
    wheel_zone.set_background(Brush::solid(Color::rgb(230, 235, 240)))?;
    let wheel_hint = TextBlock::new()?;
    wheel_hint.set_text("Hover here and scroll the wheel")?;
    wheel_zone.set_child(&wheel_hint)?;
    wheel_zone.on_pointer_wheel_changed(scope, {
        let wheel_label = wheel_label.clone();
        move |args| {
            wheel_label.set_text(format!("Wheel: ({:.0}, {:.0})", args.delta.x, args.delta.y))
        }
    })?;

    let numeric_label = TextBlock::new()?;
    numeric_label.set_text("Number: -")?;
    numeric_label.set_margin(Thickness::symmetric(12.0, 0.0))?;
    let numeric = NumericUpDown::new()?;
    numeric.set_margin(Thickness::symmetric(12.0, 4.0))?;
    numeric.set_minimum(Decimal::ZERO)?;
    numeric.set_maximum(Decimal::from(10))?;
    numeric.set_increment(Decimal::new(5, 1)?)?;
    numeric.set_value(Some(Decimal::new(15, 1)?))?;
    numeric.on_value_changed(scope, {
        let numeric_label = numeric_label.clone();
        move |args| {
            let old = args
                .old_value
                .map(|d| d.to_string())
                .unwrap_or_else(|| String::from("∅"));
            let new = args
                .new_value
                .map(|d| d.to_string())
                .unwrap_or_else(|| String::from("∅"));
            numeric_label.set_text(format!("Number: {old} -> {new}"))
        }
    })?;

    let dialog_label = TextBlock::new()?;
    dialog_label.set_text("Dialog: -")?;
    dialog_label.set_margin(Thickness::symmetric(12.0, 0.0))?;
    let ask = Button::new()?;
    ask.set_content("Ask")?;
    ask.set_margin(Thickness::uniform(4.0))?;
    ask.on_click(scope, {
        let scope = scope.clone();
        let dialog_label = dialog_label.clone();
        move |_| -> Result<()> {
            let d = Window::new()?;
            d.set_title("Question")?;
            d.set_width(260.0)?;
            d.set_height(120.0)?;
            let text = TextBlock::new()?;
            text.set_text("Accept or decline?")?;
            text.set_horizontal_alignment(HorizontalAlignment::Center)?;
            text.set_margin(Thickness::uniform(12.0))?;
            let accept = Button::new()?;
            accept.set_content("Accept")?;
            accept.set_margin(Thickness::uniform(4.0))?;
            let decline = Button::new()?;
            decline.set_content("Decline")?;
            decline.set_margin(Thickness::uniform(4.0))?;
            let buttons = StackPanel::new()?;
            buttons.set_orientation(Orientation::Horizontal)?;
            buttons.set_horizontal_alignment(HorizontalAlignment::Center)?;
            buttons.children()?.add(accept.clone())?;
            buttons.children()?.add(decline.clone())?;
            let root = StackPanel::new()?;
            root.set_orientation(Orientation::Vertical)?;
            root.children()?.add(text)?;
            root.children()?.add(buttons)?;
            d.set_content(&root)?;
            accept.on_click(&scope, {
                let d = d.clone();
                move |_| d.close_with_object("accepted")
            })?;
            decline.on_click(&scope, {
                let d = d.clone();
                move |_| d.close_with_object("declined")
            })?;
            let dialog_scope = scope.clone();
            let dialog_label = dialog_label.clone();
            scope.spawn_local(async move {
                let result = dialog_scope.show_modal(&d).await.unwrap_or(None);
                let _ = dialog_label
                    .set_text(format!("Dialog: {}", result.as_deref().unwrap_or("(none)")));
            })
        }
    })?;
    let dialog_row = StackPanel::new()?;
    dialog_row.set_orientation(Orientation::Horizontal)?;
    dialog_row.set_horizontal_alignment(HorizontalAlignment::Center)?;
    dialog_row.children()?.add(ask)?;

    let increment = Button::new()?;
    increment.set_content("Increment")?;
    increment.set_margin(Thickness::uniform(4.0))?;
    increment.on_click(scope, {
        let count = Rc::clone(&count);
        let count_label = count_label.clone();
        let log = log.clone();
        move |_| -> Result<()> {
            let value = count.get() + 1;
            count.set(value);
            count_label.set_text(value.to_string())?;
            let row = TextBlock::new()?;
            row.set_text(format!("Count {value}"))?;
            log.items()?.add(row)?;
            Ok(())
        }
    })?;

    let reset = Button::new()?;
    reset.set_content("Reset")?;
    reset.set_margin(Thickness::uniform(4.0))?;
    reset.on_click(scope, {
        let count = Rc::clone(&count);
        let count_label = count_label.clone();
        let log = log.clone();
        move |_| -> Result<()> {
            count.set(0);
            count_label.set_text("0")?;
            log.items()?.clear()?;
            Ok(())
        }
    })?;

    let buttons = StackPanel::new()?;
    buttons.set_orientation(Orientation::Horizontal)?;
    buttons.set_horizontal_alignment(HorizontalAlignment::Center)?;
    buttons.children()?.add(increment)?;
    buttons.children()?.add(reset)?;

    let echo = TextBlock::new()?;
    echo.set_margin(Thickness::symmetric(12.0, 4.0))?;
    echo.set_text("Echo: ")?;

    let input = TextBox::new()?;
    input.set_margin(Thickness::symmetric(12.0, 4.0))?;
    input.set_placeholder_text("Type here")?;
    let input_reader = input.clone();
    input.on_text_changed(scope, {
        let echo = echo.clone();
        move |_| {
            let text = input_reader.text()?.unwrap_or_default();
            echo.set_text(format!("Echo: {text}"))
        }
    })?;
    let focus_target = input.clone();

    let root = StackPanel::new()?;
    root.set_orientation(Orientation::Vertical)?;
    root.set_spacing(4.0)?;
    root.set_background(Color::rgb(245, 245, 250))?;
    let focus_label = TextBlock::new()?;
    focus_label.set_text("Focus: -")?;
    focus_label.set_margin(Thickness::symmetric(12.0, 0.0))?;
    let children = root.children()?;
    children.add(heading)?;
    children.add(count_label)?;
    children.add(buttons)?;
    children.add(input)?;
    children.add(echo)?;
    children.add(slider)?;
    children.add(slider_label)?;
    children.add(wheel_zone)?;
    children.add(wheel_label)?;
    children.add(numeric)?;
    children.add(numeric_label)?;
    children.add(dialog_row)?;
    children.add(dialog_label)?;
    children.add(focus_label.clone())?;
    children.add(log)?;
    children.add(selection_label)?;

    let window = Window::new()?;
    window.set_title("Imperative counter")?;
    window.set_width(420.0)?;
    window.set_height(720.0)?;
    window.set_content(&root)?;
    scope.mount(window)?;
    let took = focus_target
        .focus_with_navigation_method_and_key_modifiers(
            NavigationMethod::Pointer,
            KeyModifiers::NONE,
        )
        .unwrap_or(false);
    focus_label.set_text(format!("Focus: {took}"))?;
    Ok(())
}

//! A counter running in the browser.
//!
//! Build and serve with `pwsh rust/build-browser.ps1 -Serve`. The browser
//! host calls [`rustolonia_main`] on startup; `App::run` returns as soon as
//! the browser lifetime is started, and the page's event loop drives the UI.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

use rustolonia::{
    App, AppScope, Button, HorizontalAlignment, Orientation, Result, StackPanel, TextBlock,
    TextBox, Thickness, VerticalAlignment,
};

/// Entry point the browser host imports from `rustolonia_app.a`.
#[no_mangle]
pub extern "C" fn rustolonia_main() -> i32 {
    match App::load_from_env().and_then(|app| app.run(build_ui)) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("rustolonia browser sample failed: {error}");
            -1
        }
    }
}

fn build_ui(scope: &AppScope) -> Result<()> {
    let count = Arc::new(AtomicI32::new(0));

    let heading = TextBlock::new()?
        .text("Rustolonia in the browser")?
        .font_size(24.0)?
        .horizontal_alignment(HorizontalAlignment::Center)?;

    let count_label = TextBlock::new()?
        .text("0")?
        .font_size(48.0)?
        .horizontal_alignment(HorizontalAlignment::Center)?;

    let increment = Button::new()?
        .content(Some(&TextBlock::new()?.text("Increment")?))?
        .margin(Thickness::uniform(4.0))?
        .on_click(scope, {
            let count = Arc::clone(&count);
            let count_label = count_label.clone();
            move |_| {
                let value = count.fetch_add(1, Ordering::Relaxed) + 1;
                let _ = count_label.set_text(value.to_string());
            }
        })?;

    let reset = Button::new()?
        .content(Some(&TextBlock::new()?.text("Reset")?))?
        .margin(Thickness::uniform(4.0))?
        .on_click(scope, {
            let count = Arc::clone(&count);
            let count_label = count_label.clone();
            move |_| {
                count.store(0, Ordering::Relaxed);
                let _ = count_label.set_text("0");
            }
        })?;

    let buttons = StackPanel::new()?
        .orientation(Orientation::Horizontal)?
        .horizontal_alignment(HorizontalAlignment::Center)?;
    buttons.children()?.add(increment)?;
    buttons.children()?.add(reset)?;

    let echo = TextBlock::new()?
        .text("Echo: ")?
        .horizontal_alignment(HorizontalAlignment::Center)?;
    let input = TextBox::new()?
        .placeholder_text("Type here")?
        .width(240.0)?;
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
        .spacing(12.0)?
        .horizontal_alignment(HorizontalAlignment::Center)?
        .vertical_alignment(VerticalAlignment::Center)?;
    let children = root.children()?;
    children.add(heading)?;
    children.add(count_label)?;
    children.add(buttons)?;
    children.add(input)?;
    children.add(echo)?;

    scope.set_main_view(&root)
}

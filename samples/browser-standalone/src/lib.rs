//! A code-first Rustolonia app for the browser, linked into the prebuilt
//! browser host by `build.py` (no .NET SDK needed).
//!
//! The host calls [`rustolonia_main`] once the page has loaded; `App::run`
//! returns as soon as the single-view lifetime has started and the page's
//! event loop drives the UI from then on.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use rustolonia::{
    App, AppScope, Button, HorizontalAlignment, Orientation, Result, StackPanel, TextBlock,
    TextBox, Thickness, VerticalAlignment,
};

/// Entry point the browser host imports from the `rustolonia_app` module.
#[no_mangle]
pub extern "C" fn rustolonia_main() -> i32 {
    match App::load_from_env().and_then(|app| app.run(build_ui)) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("rustolonia browser app failed: {error}");
            -1
        }
    }
}

fn build_ui(scope: &AppScope) -> Result<()> {
    let count = Arc::new(AtomicU32::new(0));
    let label = TextBlock::new()?
        .text("Clicks: 0")?
        .font_size(28.0)?
        .horizontal_alignment(HorizontalAlignment::Center)?;
    let button = Button::new()?
        .content(Some(&TextBlock::new()?.text("Click me")?))?
        .horizontal_alignment(HorizontalAlignment::Center)?
        .on_click(scope, {
            let label = label.clone();
            move |_| {
                let value = count.fetch_add(1, Ordering::Relaxed) + 1;
                if let Err(error) = label.set_text(format!("Clicks: {value}")) {
                    eprintln!("failed to update the counter: {error}");
                }
            }
        })?;
    let input = TextBox::new()?
        .placeholder_text("Type here")?
        .width(240.0)?;

    let root = StackPanel::new()?
        .orientation(Orientation::Vertical)?
        .spacing(12.0)?
        .margin(Thickness::uniform(24.0))?
        .horizontal_alignment(HorizontalAlignment::Center)?
        .vertical_alignment(VerticalAlignment::Center)?;
    let children = root.children()?;
    children.add(TextBlock::new()?.text("Prebuilt Rustolonia browser host")?)?;
    children.add(label)?;
    children.add(button)?;
    children.add(input)?;
    scope.set_main_view(&root)?;
    // Rust stdout goes to the browser console; smoke-test.py waits for this.
    println!("rustolonia-browser-standalone: main view ready");
    Ok(())
}

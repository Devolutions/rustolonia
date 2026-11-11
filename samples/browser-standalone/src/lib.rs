//! A code-first Rustolonia app for the browser, linked into the prebuilt
//! browser host by `build.py` (no .NET SDK needed).
//!
//! The host calls [`rustolonia_main`] once the page has loaded; `App::run`
//! returns as soon as the single-view lifetime has started and the page's
//! event loop drives the UI from then on.

use rustolonia::{
    fmt, rsx, App, AppScope, Button, HorizontalAlignment, Orientation, Result, StackPanel,
    TextBlock, TextBox, VerticalAlignment,
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
    let count = scope.signal(0_u32);
    let root = rsx! {
        StackPanel {
            orientation: Orientation::Vertical,
            spacing: 12.0,
            margin: 24.0,
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Center,
            TextBlock { "Prebuilt Rustolonia browser host" }
            TextBlock {
                font_size: 28.0,
                horizontal_alignment: HorizontalAlignment::Center,
                text: fmt!("Clicks: {}", count.get()),
            }
            Button {
                horizontal_alignment: HorizontalAlignment::Center,
                on_click: move |_| count.update(|c| *c += 1),
                "Click me"
            }
            TextBox {
                placeholder_text: "Type here",
                width: 240.0,
            }
        }
    }
    .build(scope)?;
    scope.set_main_view(&root)?;
    // Rust stdout goes to the browser console; smoke-test.py waits for this.
    println!("rustolonia-browser-standalone: main view ready");
    Ok(())
}

//! A counter running in the browser.
//!
//! Build and serve with `pwsh rust/build-browser.ps1 -Serve`. The browser
//! host calls [`rustolonia_main`] on startup; `App::run` returns as soon as
//! the browser lifetime is started, and the page's event loop drives the UI.

use rustolonia::{
    fmt, rsx, App, AppScope, Button, HorizontalAlignment, Orientation, Result, StackPanel,
    TextBlock, TextBox, VerticalAlignment,
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
    let count = scope.signal(0_i32);
    let input = scope.signal(String::new());

    let root = rsx! {
        StackPanel {
            orientation: Orientation::Vertical,
            spacing: 12.0,
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Center,
            TextBlock {
                font_size: 24.0,
                horizontal_alignment: HorizontalAlignment::Center,
                "Rustolonia in the browser"
            }
            TextBlock {
                font_size: 48.0,
                horizontal_alignment: HorizontalAlignment::Center,
                text: fmt!("{}", count.get()),
            }
            StackPanel {
                orientation: Orientation::Horizontal,
                horizontal_alignment: HorizontalAlignment::Center,
                Button {
                    margin: 4.0,
                    on_click: move |_| count.update(|c| *c += 1),
                    "Increment"
                }
                Button {
                    margin: 4.0,
                    on_click: move |_| count.set(0),
                    "Reset"
                }
            }
            TextBox {
                placeholder_text: "Type here",
                width: 240.0,
                text: input.two_way(),
            }
            TextBlock {
                horizontal_alignment: HorizontalAlignment::Center,
                text: fmt!("Echo: {}", input.get()),
            }
        }
    }
    .build(scope)?;

    scope.set_main_view(&root)
}

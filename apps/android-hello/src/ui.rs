use rustolonia::{rsx, AppScope, Button, Orientation, Result, StackPanel, TextBlock, Window};

/// Builds the UI. The desktop binary (`main.rs`) and the Android entry point
/// (`lib.rs`) both pass this to `App::run`: the same window becomes a desktop
/// window or the Android activity's content.
pub fn build(scope: &AppScope) -> Result<()> {
    let greeting = scope.signal(String::from("Hello from Rustolonia!"));

    scope.mount_view(rsx! {
        Window {
            title: "Rustolonia Hello",
            width: 520.0,
            height: 260.0,
            StackPanel {
                orientation: Orientation::Vertical,
                margin: 24.0,
                spacing: 16.0,
                TextBlock {
                    font_size: 24.0,
                    text: greeting,
                }
                TextBlock { "Edit src/ui.rs to make this app yours." }
                Button {
                    on_click: move |_| greeting.set(String::from("Your Rust app is running.")),
                    "Say hello"
                }
            }
        }
    })?;
    Ok(())
}

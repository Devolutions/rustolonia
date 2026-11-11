use rustolonia::{rsx, App, Button, CheckBox, Orientation, Result, StackPanel, TextBlock, Window};

fn main() -> Result<()> {
    App::load_from_env()?.run(|scope| {
        let child_scope = scope.clone();
        scope.mount_view(rsx! {
            Window {
                title: "AppWithoutLifetime",
                StackPanel {
                    orientation: Orientation::Vertical,
                    spacing: 8.0,
                    margin: 16.0,
                    TextBlock { "Welcome to Avalonia!" }
                    CheckBox {
                        checked: Some(true),
                        "Welcome to Avalonia!"
                    }
                    Button {
                        on_click: move |_| -> Result<()> {
                            child_scope.mount_view(rsx! {
                                Window {
                                    title: "Sub",
                                    TextBlock { "Opened from a Rust event handler" }
                                }
                            })?;
                            Ok(())
                        },
                        "Open"
                    }
                }
            }
        })?;
        Ok(())
    })
}

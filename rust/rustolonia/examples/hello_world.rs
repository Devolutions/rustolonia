use rustolonia::{rsx, App, Button, Orientation, StackPanel, TextBlock, Window};

fn main() -> rustolonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        scope.mount_view(rsx! {
            Window {
                title: "Hello from Rust",
                StackPanel {
                    orientation: Orientation::Vertical,
                    margin: 24.0,
                    spacing: 12.0,
                    TextBlock { "Welcome to Avalonia!" }
                    Button { "Hello" }
                }
            }
        })?;
        Ok(())
    })
}

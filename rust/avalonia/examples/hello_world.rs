use avalonia::{App, Button, Orientation, StackPanel, TextBlock, Window};

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        scope.mount(
            Window::new()?.title("Hello from Rust")?.content(Some(
                &StackPanel::new()?
                    .orientation(Orientation::Vertical)?
                    .child(TextBlock::new()?.text("Welcome to Avalonia!")?)?
                    .child(Button::new()?.content(Some(&TextBlock::new()?.text("Hello")?))?)?,
            ))?,
        )
    })
}

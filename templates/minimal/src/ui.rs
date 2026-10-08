use rustolonia::{AppScope, Button, Orientation, Result, StackPanel, TextBlock, Thickness, Window};

pub fn build(scope: &AppScope) -> Result<()> {
    let greeting = TextBlock::new()?
        .text("Hello from Rustolonia!")?
        .font_size(24.0)?;

    let button = Button::new()?
        .content(Some(&TextBlock::new()?.text("Say hello")?))?
        .on_click(scope, {
            let greeting = greeting.clone();
            move |_| {
                if let Err(error) = greeting.set_text("Your standalone app is ready.") {
                    eprintln!("Could not update the greeting: {error}");
                }
            }
        })?;

    let content = StackPanel::new()?
        .orientation(Orientation::Vertical)?
        .margin(Thickness::uniform(24.0))?
        .spacing(16.0)?
        .child(greeting)?
        .child(TextBlock::new()?.text("Edit src/ui.rs to make this app yours.")?)?
        .child(button)?;

    scope.mount(
        Window::new()?
            .title("My Rustolonia app")?
            .width(520.0)?
            .height(260.0)?
            .content(Some(&content))?,
    )
}

use avalonia::{App, Button, CheckBox, Orientation, StackPanel, TextBlock, Window};

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let child_scope = scope.clone();
        let open = Button::new()?
            .content(Some(&TextBlock::new()?.text("Open")?))?
            .on_click(scope, move |_| {
                Window::new()
                    .and_then(|window| window.title("Sub"))
                    .and_then(|window| {
                        window.content(Some(
                            &TextBlock::new()?.text("Opened from a Rust event handler")?,
                        ))
                    })
                    .and_then(|window| child_scope.mount(window))
                    .expect("failed to open child window");
            })?;

        scope.mount(
            Window::new()?.title("AppWithoutLifetime")?.content(Some(
                &StackPanel::new()?
                    .orientation(Orientation::Vertical)?
                    .child(TextBlock::new()?.text("Welcome to Avalonia!")?)?
                    .child(
                        CheckBox::new()?
                            .checked(Some(true))?
                            .content(Some(&TextBlock::new()?.text("Welcome to Avalonia!")?))?,
                    )?
                    .child(open)?,
            ))?,
        )
    })
}

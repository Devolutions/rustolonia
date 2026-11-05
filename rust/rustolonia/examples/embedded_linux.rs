use rustolonia::{
    App, Border, Brush, Button, Color, HorizontalAlignment, Orientation, StackPanel, TextBlock,
    Thickness, VerticalAlignment,
};

fn main() -> rustolonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let status = TextBlock::new()?
            .text("Button presses: 0")?
            .font_size(28.0)?;
        let counter = status.clone();
        let mut presses = 0;
        let increment = Button::new()?
            .width(300.0)?
            .height(80.0)?
            .content(Some(&TextBlock::new()?.text("Increment")?.font_size(24.0)?))?
            .on_click(scope, move |_| {
                presses += 1;
                counter
                    .set_text(format!("Button presses: {presses}"))
                    .expect("failed to update the counter");
                println!("embedded-linux: count={presses}");
            })?;
        let shutdown_scope = scope.clone();
        let quit = Button::new()?
            .width(300.0)?
            .height(80.0)?
            .content(Some(&TextBlock::new()?.text("Shut down")?.font_size(24.0)?))?
            .on_click(scope, move |_| {
                println!("embedded-linux: shutdown");
                shutdown_scope.shutdown().expect("failed to shut down");
            })?;
        let content = StackPanel::new()?
            .orientation(Orientation::Vertical)?
            .spacing(24.0)?
            .horizontal_alignment(HorizontalAlignment::Center)?
            .vertical_alignment(VerticalAlignment::Center)?
            .child(
                TextBlock::new()?
                    .text("Rustolonia Embedded Linux")?
                    .font_size(36.0)?,
            )?
            .child(status)?
            .child(increment)?
            .child(quit)?;
        let root = Border::new()?
            .padding(Thickness::uniform(32.0))?
            .background(Brush::solid(Color::rgb(235, 240, 248)))?
            .child(Some(&content))?;
        scope.set_main_view(&root)?;
        println!("embedded-linux: ready");
        Ok(())
    })?;
    println!("embedded-linux: stopped");
    Ok(())
}

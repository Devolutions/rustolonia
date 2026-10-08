mod preview;

use rustolonia::{
    AppScope, Button, Orientation, Result, StackPanel, TextBlock, TextBox, Thickness, Window,
};

pub fn build(scope: &AppScope) -> Result<()> {
    let editor = TextBox::new()?
        .text("Hello from a second window!")?
        .placeholder_text("Text to preview")?;
    let status = TextBlock::new()?.text("Each preview gets a snapshot of your text.")?;

    let open = Button::new()?
        .content(Some(&TextBlock::new()?.text("Open preview")?))?
        .on_click(scope, {
            let scope = scope.clone();
            let editor = editor.clone();
            let status = status.clone();
            move |_| {
                let result = editor
                    .get_text()
                    .and_then(|text| preview::open(&scope, text.as_deref().unwrap_or("")))
                    .and_then(|()| status.set_text("Preview opened."));
                if let Err(error) = result {
                    eprintln!("Could not open the preview: {error}");
                    if let Err(status_error) = status.set_text(format!("Preview failed: {error}")) {
                        eprintln!("Could not display the preview error: {status_error}");
                    }
                }
            }
        })?;

    let content = StackPanel::new()?
        .orientation(Orientation::Vertical)?
        .margin(Thickness::uniform(24.0))?
        .spacing(16.0)?
        .child(
            TextBlock::new()?
                .text("Window workspace")?
                .font_size(24.0)?,
        )?
        .child(TextBlock::new()?.text("Preview text")?)?
        .child(editor)?
        .child(open)?
        .child(status)?;

    scope.mount(
        Window::new()?
            .title("My Rustolonia workspace")?
            .width(560.0)?
            .height(340.0)?
            .content(Some(&content))?,
    )
}

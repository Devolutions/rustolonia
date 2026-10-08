use rustolonia::{
    AppScope, Button, Orientation, Result, StackPanel, TextBlock, TextBox, Thickness, Window,
};

use crate::model;

pub fn build(scope: &AppScope) -> Result<()> {
    let name = TextBox::new()?.placeholder_text("Your name")?;
    let status = TextBlock::new()?.text("Enter your name and choose Submit.")?;

    let submit = Button::new()?
        .content(Some(&TextBlock::new()?.text("Submit")?))?
        .on_click(scope, {
            let name = name.clone();
            let status = status.clone();
            move |_| {
                let result = name.get_text().and_then(|value| {
                    let message = match model::greeting(value.as_deref().unwrap_or("")) {
                        Ok(greeting) => greeting,
                        Err(validation) => validation.to_owned(),
                    };
                    status.set_text(message)
                });
                if let Err(error) = result {
                    eprintln!("Could not submit the form: {error}");
                }
            }
        })?;

    let clear = Button::new()?
        .content(Some(&TextBlock::new()?.text("Clear")?))?
        .on_click(scope, {
            let name = name.clone();
            let status = status.clone();
            move |_| {
                let result = name
                    .set_text("")
                    .and_then(|()| status.set_text("Enter your name and choose Submit."));
                if let Err(error) = result {
                    eprintln!("Could not clear the form: {error}");
                }
            }
        })?;

    let buttons = StackPanel::new()?
        .orientation(Orientation::Horizontal)?
        .spacing(12.0)?
        .child(submit)?
        .child(clear)?;

    let content = StackPanel::new()?
        .orientation(Orientation::Vertical)?
        .margin(Thickness::uniform(24.0))?
        .spacing(16.0)?
        .child(TextBlock::new()?.text("Your first form")?.font_size(24.0)?)?
        .child(TextBlock::new()?.text("Name")?)?
        .child(name)?
        .child(buttons)?
        .child(status)?;

    scope.mount(
        Window::new()?
            .title("My Rustolonia form")?
            .width(520.0)?
            .height(340.0)?
            .content(Some(&content))?,
    )
}

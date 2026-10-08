use rustolonia::{AppScope, Result, ScrollViewer, TextBlock, Thickness, Window};

pub fn open(scope: &AppScope, text: &str) -> Result<()> {
    let content = ScrollViewer::new()?
        .margin(Thickness::uniform(24.0))?
        .content(Some(&TextBlock::new()?.text(text)?.font_size(20.0)?))?;

    scope.mount(
        Window::new()?
            .title("Text preview")?
            .width(480.0)?
            .height(280.0)?
            .content(Some(&content))?,
    )
}

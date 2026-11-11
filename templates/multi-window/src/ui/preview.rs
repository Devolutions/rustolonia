use rustolonia::{rsx, AppScope, Result, ScrollViewer, TextBlock, Window};

pub fn open(scope: &AppScope, text: &str) -> Result<()> {
    let snapshot = text.to_owned();
    scope.mount_view(rsx! {
        Window {
            title: "Text preview",
            width: 480.0,
            height: 280.0,
            ScrollViewer {
                margin: 24.0,
                TextBlock {
                    font_size: 20.0,
                    text: snapshot,
                }
            }
        }
    })?;
    Ok(())
}

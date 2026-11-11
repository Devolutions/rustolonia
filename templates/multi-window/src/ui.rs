mod preview;

use rustolonia::{rsx, AppScope, Button, Orientation, Result, StackPanel, TextBlock, TextBox, Window};

pub fn build(scope: &AppScope) -> Result<()> {
    let text = scope.signal(String::from("Hello from a second window!"));
    let status = scope.signal(String::from("Each preview gets a snapshot of your text."));
    let open_scope = scope.clone();

    scope.mount_view(rsx! {
        Window {
            title: "My Rustolonia workspace",
            width: 560.0,
            height: 340.0,
            StackPanel {
                orientation: Orientation::Vertical,
                margin: 24.0,
                spacing: 16.0,
                TextBlock {
                    font_size: 24.0,
                    "Window workspace"
                }
                TextBlock { "Preview text" }
                TextBox {
                    placeholder_text: "Text to preview",
                    text: text.two_way(),
                }
                Button {
                    on_click: move |_| -> Result<()> {
                        preview::open(&open_scope, &text.get_untracked())?;
                        status.set(String::from("Preview opened."));
                        Ok(())
                    },
                    "Open preview"
                }
                TextBlock { text: status }
            }
        }
    })?;
    Ok(())
}

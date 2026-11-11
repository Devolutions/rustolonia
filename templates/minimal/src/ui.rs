use rustolonia::{
    color, rsx, AppScope, Button, Orientation, Result, StackPanel, Style, StyleSheet, TextBlock,
    Window,
};

pub fn build(scope: &AppScope) -> Result<()> {
    let greeting = scope.signal(String::from("Hello from Rustolonia!"));

    scope.apply_styles(StyleSheet::from([
        Style::select("Button.primary")
            .background(color!("#2563eb"))
            .foreground(color!("#ffffff"))
            .corner_radius(6.0)
            .padding((16.0, 8.0)),
        Style::select("Button.primary:pointerover").background(color!("#1d4ed8")),
    ]))?;

    scope.mount_view(rsx! {
        Window {
            title: "My Rustolonia app",
            width: 520.0,
            height: 260.0,
            StackPanel {
                orientation: Orientation::Vertical,
                margin: 24.0,
                spacing: 16.0,
                TextBlock {
                    font_size: 24.0,
                    text: greeting,
                }
                TextBlock { "Edit src/ui.rs to make this app yours." }
                Button {
                    class: "primary",
                    on_click: move |_| greeting.set(String::from("Your standalone app is ready.")),
                    "Say hello"
                }
            }
        }
    })?;
    Ok(())
}

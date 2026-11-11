use rustolonia::{
    rsx, App, Button, Dock, DockPanel, Orientation, Result, ScrollViewer, StackPanel, TextBlock,
    TextBox, Window,
};

fn main() -> Result<()> {
    App::load_from_env()?.run(|scope| {
        let text = scope.signal(String::from("Hello!"));
        let child_scope = scope.clone();

        scope.mount_view(rsx! {
            Window {
                title: "Text Test App",
                DockPanel {
                    StackPanel {
                        dock_panel_dock: Dock::Top,
                        orientation: Orientation::Horizontal,
                        spacing: 8.0,
                        margin: 8.0,
                        TextBlock { "Text:" }
                        TextBox {
                            placeholder_text: "Text to preview",
                            text: text.two_way(),
                        }
                        Button {
                            on_click: move |_| -> Result<()> {
                                let snapshot = text.get_untracked();
                                child_scope.mount_view(rsx! {
                                    Window {
                                        title: "Text Test App",
                                        TextBlock { margin: 16.0, text: snapshot }
                                    }
                                })?;
                                Ok(())
                            },
                            "+"
                        }
                    }
                    ScrollViewer {
                        TextBlock {
                            margin: 8.0,
                            text: text,
                        }
                    }
                }
            }
        })?;
        Ok(())
    })
}

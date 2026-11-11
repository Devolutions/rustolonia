use rustolonia::{
    component, rsx, AppScope, Button, Orientation, Result, StackPanel, TextBlock, TextBox, TwoWay,
    Window,
};

use crate::model;

#[component]
fn FormField(label: String, placeholder: String, value: TwoWay<String>) -> rustolonia::El<StackPanel> {
    rsx! {
        StackPanel {
            orientation: Orientation::Vertical,
            spacing: 6.0,
            TextBlock { text: label }
            TextBox {
                placeholder_text: placeholder,
                text: value,
            }
        }
    }
}

pub fn build(scope: &AppScope) -> Result<()> {
    let name = scope.signal(String::new());
    let status = scope.signal(String::from("Enter your name and choose Submit."));

    scope.mount_view(rsx! {
        Window {
            title: "My Rustolonia form",
            width: 520.0,
            height: 340.0,
            StackPanel {
                orientation: Orientation::Vertical,
                margin: 24.0,
                spacing: 16.0,
                TextBlock {
                    font_size: 24.0,
                    "Your first form"
                }
                FormField {
                    label: "Name",
                    placeholder: "Your name",
                    value: name.two_way(),
                }
                StackPanel {
                    orientation: Orientation::Horizontal,
                    spacing: 12.0,
                    Button {
                        on_click: move |_| {
                            let message = match model::greeting(&name.get_untracked()) {
                                Ok(greeting) => greeting,
                                Err(validation) => validation.to_owned(),
                            };
                            status.set(message);
                        },
                        "Submit"
                    }
                    Button {
                        on_click: move |_| {
                            name.set(String::new());
                            status.set(String::from("Enter your name and choose Submit."));
                        },
                        "Clear"
                    }
                }
                TextBlock { text: status }
            }
        }
    })?;
    Ok(())
}

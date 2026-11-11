//! Declarative counter example showcasing `rsx!`, `#[component]`, `Signal`,
//! `Memo`, `TwoWay`, `StyleSheet`, `color!`, `fmt!`, reactive `show`, and `Command`.

use rustolonia::{
    color, component, fmt, rsx, App, Button, CheckBox, Command, Key, Orientation, Result, Shortcut,
    StackPanel, Style, StyleSheet, TextBlock, TextBox, Window,
};

#[component]
fn StatHeader(title: rustolonia::Memo<String>) -> rustolonia::El<TextBlock> {
    rsx! {
        TextBlock {
            font_size: 22.0,
            text: title,
        }
    }
}

fn main() -> Result<()> {
    let styles = StyleSheet::from([
        Style::select("Button.accent")
            .background(color!("#2563eb"))
            .foreground(color!("#ffffff"))
            .corner_radius(6.0)
            .padding((14.0, 6.0)),
        Style::select("Button.accent:pointerover").background(color!("#1d4ed8")),
    ]);

    App::load_from_env()?.run_with_styles(styles, |scope| {
        let name = scope.signal(String::from("Avalonia developer"));
        let count = scope.signal(0_i32);
        let show_details = scope.signal(true);

        let greeting = scope.memo(move || {
            let current = name.get();
            if current.trim().is_empty() {
                String::from("Hello, stranger!")
            } else {
                format!("Hello, {current}!")
            }
        });

        let can_reset = scope.memo(move || count.get() != 0);
        let reset_cmd = Command::with_can_execute(can_reset, move |_| {
            count.set(0);
            Ok(())
        })?;

        let window = rsx! {
            Window {
                title: "Declarative RSX Counter",
                width: 520.0,
                height: 360.0,
                StackPanel {
                    orientation: Orientation::Vertical,
                    margin: 24.0,
                    spacing: 12.0,
                    StatHeader { title: greeting }
                    TextBox {
                        placeholder_text: "Enter your name",
                        text: name.two_way(),
                    }
                    StackPanel {
                        orientation: Orientation::Horizontal,
                        spacing: 8.0,
                        Button {
                            class: "accent",
                            on_click: move |_| count.update(|c| *c += 1),
                            "Increment"
                        }
                        Button {
                            command: reset_cmd.clone(),
                            "Reset"
                        }
                        TextBlock {
                            text: fmt!("Count: {}", count.get()),
                        }
                    }
                    CheckBox {
                        checked: show_details.two_way(),
                        "Show details"
                    }
                    show show_details {
                        TextBlock {
                            text: fmt!("Double count is {}", count.get() * 2),
                        }
                    } else {
                        TextBlock { "Details hidden" }
                    }
                }
            }
        }
        .shortcut(Shortcut::primary(Key::R), reset_cmd);

        scope.mount_view(window)?;
        Ok(())
    })
}

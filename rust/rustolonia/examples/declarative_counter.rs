//! Declarative counter and task list example using `rsx!`, `Signal`, `Memo`,
//! `TwoWay`, `Show`, `For`, and `Command`.

use rustolonia::{
    rsx, App, Button, CheckBox, Command, Orientation, Result, Show, StackPanel, TextBlock, TextBox,
    Window,
};

fn main() -> Result<()> {
    App::load_from_env()?.run(|scope| {
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
                    TextBlock {
                        font_size: 22.0,
                        text: greeting,
                    }
                    TextBox {
                        placeholder_text: "Enter your name",
                        text: name.two_way(),
                    }
                    StackPanel {
                        orientation: Orientation::Horizontal,
                        spacing: 8.0,
                        Button {
                            on_click: move |_| count.update(|c| *c += 1),
                            "Increment"
                        }
                        Button {
                            command: reset_cmd,
                            "Reset"
                        }
                        TextBlock {
                            text: move || format!("Count: {}", count.get()),
                        }
                    }
                    CheckBox {
                        checked: show_details.two_way(),
                        "Show details"
                    }
                    {
                        Show::when_else(
                            show_details,
                            move || TextBlock::el().text(move || format!("Double count is {}", count.get() * 2)),
                            || TextBlock::el().text("Details hidden"),
                        )
                    }
                }
            }
        };

        scope.mount_view(window)?;
        Ok(())
    })
}

use rustolonia::{
    color, fmt, rsx, App, Border, Button, HorizontalAlignment, Orientation, StackPanel, TextBlock,
    VerticalAlignment,
};

fn main() -> rustolonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let presses = scope.signal(0_u32);
        let shutdown_scope = scope.clone();

        let root = rsx! {
            Border {
                padding: 32.0,
                background: color!("#ebf0f8"),
                StackPanel {
                    orientation: Orientation::Vertical,
                    spacing: 24.0,
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: VerticalAlignment::Center,
                    TextBlock {
                        font_size: 36.0,
                        "Rustolonia Embedded Linux"
                    }
                    TextBlock {
                        font_size: 28.0,
                        text: fmt!("Button presses: {}", presses.get()),
                    }
                    Button {
                        width: 300.0,
                        height: 80.0,
                        on_click: move |_| {
                            presses.update(|p| *p += 1);
                            println!("embedded-linux: count={}", presses.get_untracked());
                        },
                        TextBlock { font_size: 24.0, "Increment" }
                    }
                    Button {
                        width: 300.0,
                        height: 80.0,
                        on_click: move |_| {
                            println!("embedded-linux: shutdown");
                            shutdown_scope.shutdown()
                        },
                        TextBlock { font_size: 24.0, "Shut down" }
                    }
                }
            }
        }
        .build(scope)?;
        scope.set_main_view(&root)?;
        println!("embedded-linux: ready");
        Ok(())
    })?;
    println!("embedded-linux: stopped");
    Ok(())
}

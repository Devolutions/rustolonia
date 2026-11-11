use rustolonia::{
    fmt, rsx, AppScope, Button, HorizontalAlignment, Orientation, Result, StackPanel, TextBlock,
    TextBox, VerticalAlignment,
};

rustolonia::ios_application!(build_view);

fn build_view(scope: &AppScope) -> Result<StackPanel> {
    let count = scope.signal(0_u32);
    let label_ref = scope.node_ref::<TextBlock>();
    let input_ref = scope.node_ref::<TextBox>();

    let root = rsx! {
        StackPanel {
            orientation: Orientation::Vertical,
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Center,
            margin: 24.0,
            TextBlock {
                font_size: 20.0,
                "Standalone Rustolonia app"
            }
            TextBlock {
                ref: label_ref,
                font_size: 28.0,
                text: fmt!("Taps: {}", count.get()),
            }
            TextBox {
                ref: input_ref,
                text: "Editable from Rust",
            }
            Button {
                on_click: move |_| count.update(|c| *c += 1),
                "Tap me"
            }
        }
    }
    .build(scope)?;

    #[cfg(feature = "smoke")]
    {
        let dispatcher = std::ops::Deref::deref(scope).clone();
        let root_for_test = root.clone();
        let label = label_ref.get().expect("label built");
        let input = input_ref.get().expect("input built");
        let mut ran = false;
        root.on_loaded(scope, move |()| {
            if std::mem::replace(&mut ran, true) {
                return;
            }
            let root = root_for_test.clone();
            let label = label.clone();
            let input = input.clone();
            if let Err(error) = dispatcher.post(move || {
                let result = (|| -> std::result::Result<(), String> {
                    if !root.is_loaded().map_err(|error| error.to_string())? {
                        return Err("the root view was not loaded".into());
                    }
                    let check_box = rustolonia::CheckBox::new().map_err(|e| e.to_string())?;
                    check_box.set_checked(Some(true)).map_err(|e| e.to_string())?;
                    if check_box.is_checked().map_err(|error| error.to_string())? != Some(true) {
                        return Err("check box round trip failed".into());
                    }
                    let slider = rustolonia::Slider::new().map_err(|e| e.to_string())?;
                    slider.set_value(42.0).map_err(|e| e.to_string())?;
                    if slider.value().map_err(|error| error.to_string())? != 42.0 {
                        return Err("slider round trip failed".into());
                    }
                    rustolonia::ComboBox::new().map_err(|error| error.to_string())?;
                    rustolonia::Grid::new().map_err(|error| error.to_string())?;
                    if label.text().map_err(|error| error.to_string())?.as_deref() != Some("Taps: 0")
                    {
                        return Err("unexpected initial label".into());
                    }
                    input
                        .set_text("Verified from Rust")
                        .map_err(|error| error.to_string())?;
                    if input
                        .text()
                        .map_err(|error| error.to_string())?
                        .as_deref()
                        != Some("Verified from Rust")
                    {
                        return Err("text box round trip failed".into());
                    }
                    label
                        .set_text("Rust callback succeeded")
                        .map_err(|error| error.to_string())?;
                    Ok(())
                })();
                write_smoke_result(result);
            }) {
                write_smoke_result(Err(format!("failed to post the Rust callback: {error}")));
            }
        })?;
    }

    Ok(root)
}

#[cfg(feature = "smoke")]
fn write_smoke_result(result: std::result::Result<(), String>) {
    let result = result.map_or_else(|error| format!("FAILED: {error}"), |()| "ok".into());
    match std::env::var_os("HOME") {
        Some(home) => {
            let path = std::path::PathBuf::from(home)
                .join("Documents")
                .join("rustolonia-standalone-smoke.txt");
            if let Err(error) = std::fs::write(&path, result) {
                eprintln!("failed to write {}: {error}", path.display());
            }
        }
        None => eprintln!("HOME is not set; cannot report the iOS smoke result"),
    }
}

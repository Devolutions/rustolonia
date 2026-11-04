use rustolonia::{
    AppScope, Button, HorizontalAlignment, Orientation, Result, StackPanel, TextBlock, TextBox,
    Thickness, VerticalAlignment,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

rustolonia::ios_application!(build_view);

fn build_view(scope: &AppScope) -> Result<StackPanel> {
    let count = Arc::new(AtomicU32::new(0));
    let label = TextBlock::new()?.text("Taps: 0")?.font_size(28.0)?;
    let input = TextBox::new()?.text("Editable from Rust")?;
    let button = Button::new()?
        .content(Some(&TextBlock::new()?.text("Tap me")?))?
        .on_click(scope, {
            let label = label.clone();
            move |()| {
                let value = count.fetch_add(1, Ordering::Relaxed) + 1;
                if let Err(error) = label.set_text(format!("Taps: {value}")) {
                    eprintln!("failed to update the counter: {error}");
                }
            }
        })?;

    let root = StackPanel::new()?
        .orientation(Orientation::Vertical)?
        .horizontal_alignment(HorizontalAlignment::Center)?
        .vertical_alignment(VerticalAlignment::Center)?
        .margin(Thickness::uniform(24.0))?
        .child(
            TextBlock::new()?
                .text("Standalone Rustolonia app")?
                .font_size(20.0)?,
        )?
        .child(label.clone())?
        .child(input.clone())?
        .child(button)?;

    #[cfg(feature = "smoke")]
    let root = {
        let dispatcher = std::ops::Deref::deref(scope).clone();
        let root_for_test = root.clone();
        let label = label.clone();
        let input = input.clone();
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
                    let check_box = rustolonia::CheckBox::new()
                        .and_then(|control| control.checked(Some(true)))
                        .map_err(|error| error.to_string())?;
                    if check_box
                        .get_is_checked()
                        .map_err(|error| error.to_string())?
                        != Some(true)
                    {
                        return Err("check box round trip failed".into());
                    }
                    let slider = rustolonia::Slider::new()
                        .and_then(|control| control.value(42.0))
                        .map_err(|error| error.to_string())?;
                    if slider.get_value().map_err(|error| error.to_string())? != 42.0 {
                        return Err("slider round trip failed".into());
                    }
                    rustolonia::ComboBox::new().map_err(|error| error.to_string())?;
                    rustolonia::Grid::new().map_err(|error| error.to_string())?;
                    if label
                        .get_text()
                        .map_err(|error| error.to_string())?
                        .as_deref()
                        != Some("Taps: 0")
                    {
                        return Err("unexpected initial label".into());
                    }
                    input
                        .set_text("Verified from Rust")
                        .map_err(|error| error.to_string())?;
                    if input
                        .get_text()
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
        })?
    };

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

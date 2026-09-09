use avalonia::{
    App, CheckBox, Orientation, ProgressBar, Slider, StackPanel, TextBlock, TextBox, Window,
};

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let horizontal = ProgressBar::new()?
            .minimum(0.0)?
            .maximum(100.0)?
            .value(40.0)?
            .show_progress_text(true)?
            .progress_text_format("{0:0}%")?;
        let vertical = ProgressBar::new()?
            .minimum(0.0)?
            .maximum(100.0)?
            .value(60.0)?
            .orientation(Orientation::Vertical)?
            .show_progress_text(true)?
            .progress_text_format("{0:0}%")?;

        let horizontal_for_slider = horizontal.clone();
        let horizontal_slider = Slider::new()?.minimum(0.0)?.maximum(100.0)?.value(40.0)?;
        let horizontal_slider_for_handler = horizontal_slider.clone();
        let horizontal_slider = horizontal_slider.on_value_changed(scope, move |_| {
            let value = horizontal_slider_for_handler
                .get_value()
                .expect("failed to read horizontal slider");
            horizontal_for_slider
                .set_value(value)
                .expect("failed to update horizontal progress");
        })?;

        let vertical_for_slider = vertical.clone();
        let vertical_slider = Slider::new()?.minimum(0.0)?.maximum(100.0)?.value(60.0)?;
        let vertical_slider_for_handler = vertical_slider.clone();
        let vertical_slider = vertical_slider.on_value_changed(scope, move |_| {
            let value = vertical_slider_for_handler
                .get_value()
                .expect("failed to read vertical slider");
            vertical_for_slider
                .set_value(value)
                .expect("failed to update vertical progress");
        })?;

        let horizontal_for_text = horizontal.clone();
        let vertical_for_text = vertical.clone();
        let format = TextBox::new()?.text("{0:0}%")?;
        let format_for_handler = format.clone();
        let format = format.on_text_changed(scope, move |_| {
            let value = format_for_handler
                .get_text()
                .expect("failed to read progress format")
                .unwrap_or_default();
            horizontal_for_text
                .set_progress_text_format(&value)
                .expect("failed to update horizontal format");
            vertical_for_text
                .set_progress_text_format(value)
                .expect("failed to update vertical format");
        })?;

        let horizontal_for_show = horizontal.clone();
        let vertical_for_show = vertical.clone();
        let show_text = CheckBox::new()?
            .content(Some(&TextBlock::new()?.text("Show Progress Text")?))?
            .checked(Some(true))?;
        let show_text_for_handler = show_text.clone();
        let show_text = show_text.on_is_checked_changed(scope, move |_| {
            let value = show_text_for_handler
                .get_is_checked()
                .expect("failed to read show-text state")
                == Some(true);
            horizontal_for_show
                .set_show_progress_text(value)
                .expect("failed to update horizontal progress");
            vertical_for_show
                .set_show_progress_text(value)
                .expect("failed to update vertical progress");
        })?;

        let horizontal_for_indeterminate = horizontal.clone();
        let vertical_for_indeterminate = vertical.clone();
        let indeterminate =
            CheckBox::new()?.content(Some(&TextBlock::new()?.text("Toggle Indeterminate")?))?;
        let indeterminate_for_handler = indeterminate.clone();
        let indeterminate = indeterminate.on_is_checked_changed(scope, move |_| {
            let value = indeterminate_for_handler
                .get_is_checked()
                .expect("failed to read indeterminate state")
                == Some(true);
            horizontal_for_indeterminate
                .set_indeterminate(value)
                .expect("failed to update horizontal progress");
            vertical_for_indeterminate
                .set_indeterminate(value)
                .expect("failed to update vertical progress");
        })?;

        scope.mount(
            Window::new()?.title("ProgressBar")?.content(Some(
                &StackPanel::new()?
                    .spacing(8.0)?
                    .child(TextBlock::new()?.text("A progress bar control")?)?
                    .child(TextBlock::new()?.text("Progress Text Format")?)?
                    .child(format)?
                    .child(show_text)?
                    .child(indeterminate)?
                    .child(horizontal)?
                    .child(vertical)?
                    .child(horizontal_slider)?
                    .child(vertical_slider)?,
            ))?,
        )
    })
}

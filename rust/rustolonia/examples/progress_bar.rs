use rustolonia::{
    rsx, App, CheckBox, Orientation, ProgressBar, Slider, StackPanel, TextBlock, TextBox, Window,
};

fn main() -> rustolonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let h_val = scope.signal(40.0_f64);
        let v_val = scope.signal(60.0_f64);
        let format = scope.signal(String::from("{0:0}%"));
        let show_text = scope.signal(true);
        let indeterminate = scope.signal(false);

        scope.mount_view(rsx! {
            Window {
                title: "ProgressBar",
                StackPanel {
                    spacing: 8.0,
                    margin: 16.0,
                    TextBlock { "A progress bar control" }
                    TextBlock { "Progress Text Format" }
                    TextBox { text: format.two_way() }
                    CheckBox {
                        checked: show_text.two_way(),
                        "Show Progress Text"
                    }
                    CheckBox {
                        checked: indeterminate.two_way(),
                        "Toggle Indeterminate"
                    }
                    ProgressBar {
                        minimum: 0.0,
                        maximum: 100.0,
                        value: h_val,
                        show_progress_text: show_text,
                        indeterminate: indeterminate,
                        progress_text_format: format,
                    }
                    ProgressBar {
                        minimum: 0.0,
                        maximum: 100.0,
                        value: v_val,
                        orientation: Orientation::Vertical,
                        show_progress_text: show_text,
                        indeterminate: indeterminate,
                        progress_text_format: format,
                    }
                    Slider {
                        minimum: 0.0,
                        maximum: 100.0,
                        value: h_val.two_way(),
                    }
                    Slider {
                        minimum: 0.0,
                        maximum: 100.0,
                        value: v_val.two_way(),
                    }
                }
            }
        })?;
        Ok(())
    })
}

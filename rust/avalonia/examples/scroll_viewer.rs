use avalonia::{
    App, AppScope, Button, Orientation, ScrollBarVisibility, ScrollViewer, StackPanel, TextBlock,
    ToggleSwitch, Window,
};

fn command_button(
    scope: &AppScope,
    label: &str,
    action: impl FnMut(()) + Send + 'static,
) -> avalonia::Result<Button> {
    Button::new()?
        .content(Some(&TextBlock::new()?.text(label)?))?
        .on_click(scope, action)
}

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let mut content = StackPanel::new()?.spacing(12.0)?;
        for index in 1..=30 {
            content = content.child(TextBlock::new()?.text(format!("Scrollable item {index}"))?)?;
        }

        let status = TextBlock::new()?.text("Scroll events: 0")?;
        let status_for_handler = status.clone();
        let mut scroll_events = 0;
        let viewer = ScrollViewer::new()?
            .height(400.0)?
            .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)?
            .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)?
            .allow_auto_hide(true)?
            .scroll_inertia_enabled(true)?
            .content(Some(&content))?
            .on_scroll_changed(scope, move |_| {
                scroll_events += 1;
                status_for_handler
                    .set_text(format!("Scroll events: {scroll_events}"))
                    .expect("failed to update scroll status");
            })?;

        let viewer_for_auto_hide = viewer.clone();
        let auto_hide = ToggleSwitch::new()?
            .content(Some(&TextBlock::new()?.text("Allow auto hide")?))?
            .checked(Some(true))?;
        let auto_hide_for_handler = auto_hide.clone();
        let auto_hide = auto_hide.on_is_checked_changed(scope, move |_| {
            viewer_for_auto_hide
                .set_allow_auto_hide(
                    auto_hide_for_handler
                        .get_is_checked()
                        .expect("failed to read auto-hide state")
                        == Some(true),
                )
                .expect("failed to update auto-hide state");
        })?;

        let viewer_for_inertia = viewer.clone();
        let inertia = ToggleSwitch::new()?
            .content(Some(&TextBlock::new()?.text("Enable inertia")?))?
            .checked(Some(true))?;
        let inertia_for_handler = inertia.clone();
        let inertia = inertia.on_is_checked_changed(scope, move |_| {
            viewer_for_inertia
                .set_scroll_inertia_enabled(
                    inertia_for_handler
                        .get_is_checked()
                        .expect("failed to read inertia state")
                        == Some(true),
                )
                .expect("failed to update inertia state");
        })?;

        let viewer_for_deferred = viewer.clone();
        let deferred = ToggleSwitch::new()?
            .content(Some(&TextBlock::new()?.text("Enable deferred scrolling")?))?;
        let deferred_for_handler = deferred.clone();
        let deferred = deferred.on_is_checked_changed(scope, move |_| {
            viewer_for_deferred
                .set_deferred_scrolling_enabled(
                    deferred_for_handler
                        .get_is_checked()
                        .expect("failed to read deferred state")
                        == Some(true),
                )
                .expect("failed to update deferred state");
        })?;

        let up = viewer.clone();
        let down = viewer.clone();
        let home = viewer.clone();
        let end = viewer.clone();
        let commands = StackPanel::new()?
            .orientation(Orientation::Horizontal)?
            .spacing(8.0)?
            .child(command_button(scope, "Page up", move |_| {
                up.page_up().expect("failed to page up");
            })?)?
            .child(command_button(scope, "Page down", move |_| {
                down.page_down().expect("failed to page down");
            })?)?
            .child(command_button(scope, "Home", move |_| {
                home.scroll_to_home().expect("failed to scroll home");
            })?)?
            .child(command_button(scope, "End", move |_| {
                end.scroll_to_end().expect("failed to scroll end");
            })?)?;

        scope.mount(
            Window::new()?.title("ScrollViewer")?.content(Some(
                &StackPanel::new()?
                    .spacing(8.0)?
                    .child(TextBlock::new()?.text("ScrollViewer controls")?)?
                    .child(auto_hide)?
                    .child(inertia)?
                    .child(deferred)?
                    .child(commands)?
                    .child(status)?
                    .child(viewer)?,
            ))?,
        )
    })
}

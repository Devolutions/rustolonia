use rustolonia::{
    fmt, rsx, App, Button, Orientation, Result, ScrollBarVisibility, ScrollViewer, StackPanel,
    TextBlock, ToggleSwitch, Window,
};

fn main() -> Result<()> {
    App::load_from_env()?.run(|scope| {
        let scroll_events = scope.signal(0_u32);
        let auto_hide = scope.signal(true);
        let inertia = scope.signal(true);
        let deferred = scope.signal(false);
        let viewer_ref = scope.node_ref::<ScrollViewer>();

        let mut content = StackPanel::el().spacing(12.0);
        for index in 1..=30 {
            content = content.child(TextBlock::el().text(format!("Scrollable item {index}")));
        }

        let up_ref = viewer_ref.clone();
        let down_ref = viewer_ref.clone();
        let home_ref = viewer_ref.clone();
        let end_ref = viewer_ref.clone();

        scope.mount_view(rsx! {
            Window {
                title: "ScrollViewer",
                StackPanel {
                    spacing: 8.0,
                    margin: 16.0,
                    TextBlock { "ScrollViewer controls" }
                    ToggleSwitch {
                        checked: auto_hide.two_way(),
                        "Allow auto hide"
                    }
                    ToggleSwitch {
                        checked: inertia.two_way(),
                        "Enable inertia"
                    }
                    ToggleSwitch {
                        checked: deferred.two_way(),
                        "Enable deferred scrolling"
                    }
                    StackPanel {
                        orientation: Orientation::Horizontal,
                        spacing: 8.0,
                        Button {
                            on_click: move |_| -> Result<()> {
                                if let Some(v) = up_ref.get() { v.page_up()?; }
                                Ok(())
                            },
                            "Page up"
                        }
                        Button {
                            on_click: move |_| -> Result<()> {
                                if let Some(v) = down_ref.get() { v.page_down()?; }
                                Ok(())
                            },
                            "Page down"
                        }
                        Button {
                            on_click: move |_| -> Result<()> {
                                if let Some(v) = home_ref.get() { v.scroll_to_home()?; }
                                Ok(())
                            },
                            "Home"
                        }
                        Button {
                            on_click: move |_| -> Result<()> {
                                if let Some(v) = end_ref.get() { v.scroll_to_end()?; }
                                Ok(())
                            },
                            "End"
                        }
                    }
                    TextBlock {
                        text: fmt!("Scroll events: {}", scroll_events.get()),
                    }
                    ScrollViewer {
                        ref: viewer_ref,
                        height: 400.0,
                        horizontal_scroll_bar_visibility: ScrollBarVisibility::Auto,
                        vertical_scroll_bar_visibility: ScrollBarVisibility::Auto,
                        allow_auto_hide: auto_hide,
                        scroll_inertia_enabled: inertia,
                        deferred_scrolling_enabled: deferred,
                        on_scroll_changed: move |_| scroll_events.update(|n| *n += 1),
                        { content }
                    }
                }
            }
        })?;
        Ok(())
    })
}

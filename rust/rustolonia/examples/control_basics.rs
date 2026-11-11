use rustolonia::{
    color, fmt, rsx, tracks, App, Border, Brush, Button, CivilDate, CivilTime, ClickMode, Color,
    ComboBox, ComboBoxItem, CornerRadius, DatePicker, Dock, ExpandDirection, Expander, Flyout,
    FlyoutShowMode, FontWeight, GradientStop, Grid, HorizontalAlignment, Image, ListBox,
    ListBoxItem, Menu, MenuItem, MenuItemToggleType, Orientation, Paint, PlacementMode,
    RadioButton, Result, SelectionMode, Slider, SplitView, SplitViewDisplayMode,
    SplitViewPanePlacement, StackPanel, Stretch, StretchDirection, TabControl, TabItem,
    TextAlignment, TextBlock, TextBox, Thickness, TimePicker, ToggleSwitch, ToolTip, TreeView,
    TreeViewItem, VerticalAlignment, Window,
};

fn main() -> Result<()> {
    App::load_from_env()?.run(|scope| {
        let clicks = scope.signal(0_u32);
        let slider_val = scope.signal(25.0_f64);
        let typed = scope.signal(String::from("Hello from Avalonia"));
        let key_readout = scope.signal(String::from("Key: focus the text box and press a key"));
        let toggled = scope.signal(true);
        let combo_index = scope.signal(2_i32);
        let list_index = scope.signal(1_i32);
        let hover_state = scope.signal(String::from("out"));
        let clipboard_buf = scope.signal(String::from("Hello clipboard"));
        let clipboard_status = scope.signal(String::new());
        let menu_status = scope.signal(String::from("No menu item clicked yet"));
        let selected_date = scope.signal(Some(CivilDate::new(2027, 1, 15).expect("valid date")));
        let selected_time = scope.signal(Some(CivilTime::hm(17, 4).expect("valid time")));

        let list_ref = scope.node_ref::<ListBox>();
        let select_all_ref = list_ref.clone();
        let clear_ref = list_ref.clone();

        let copy_scope = scope.clone();
        let paste_scope = scope.clone();

        let image_source = std::env::var("AVN_IMAGE_SOURCE").unwrap_or_default();
        let image_el = {
            let el = Image::el()
                .stretch(Stretch::Uniform)
                .stretch_direction(StretchDirection::DownOnly)
                .max_height(96.0);
            if image_source.is_empty() {
                el
            } else {
                el.source(image_source.clone())
            }
        };
        let image_ref = scope.node_ref::<Image>();
        let image_el = image_el.node_ref(image_ref.clone());

        let flyout = Flyout::new()?;
        let flyout_body = TextBlock::new()?;
        flyout_body.set_text("Shown with flyout.show_at_with_control")?;
        flyout.set_content(&flyout_body)?;
        flyout.set_placement(PlacementMode::BottomEdgeAlignedLeft)?;
        flyout.set_show_mode(FlyoutShowMode::Transient)?;
        let flyout_button_ref = scope.node_ref::<Button>();
        let flyout_target_ref = flyout_button_ref.clone();

        let chrome_gradient = Paint::LinearGradient(rustolonia::LinearGradientBrush::new(
            vec![
                GradientStop::new(0.0, color!("#1e293b")),
                GradientStop::new(1.0, color!("#0f172a")),
            ],
            rustolonia::RelativePoint::new(0.0, 0.0, rustolonia::RelativeUnit::Relative),
            rustolonia::RelativePoint::new(1.0, 1.0, rustolonia::RelativeUnit::Relative),
        ));

        let copy_btn = Button::el().content("Copy").on_click_async(move || {
            let s = copy_scope.clone();
            async move {
                let text = clipboard_buf.get_untracked();
                match s.set_clipboard_text(&text).await {
                    Ok(()) => clipboard_status.set(format!("copied {} chars", text.len())),
                    Err(err) => clipboard_status.set(err.to_string()),
                }
                Ok(())
            }
        });
        let paste_btn = Button::el().content("Paste").on_click_async(move || {
            let s = paste_scope.clone();
            async move {
                match s.clipboard_text().await {
                    Ok(Some(text)) => {
                        clipboard_status.set(format!("pasted {} chars", text.len()));
                        clipboard_buf.set(text);
                    }
                    Ok(None) => clipboard_status.set(String::from("no text on clipboard")),
                    Err(err) => clipboard_status.set(err.to_string()),
                }
                Ok(())
            }
        });

        let grid_ref = scope.node_ref::<Grid>();
        let grid_readout_ref = scope.node_ref::<TextBlock>();

        let window = rsx! {
            Window {
                title: "Control basics",
                can_resize: true,
                StackPanel {
                    orientation: Orientation::Vertical,
                    spacing: 8.0,
                    margin: 12.0,
                    Button {
                        click_mode: ClickMode::Press,
                        default: true,
                        horizontal_content_alignment: HorizontalAlignment::Center,
                        on_click: move |_| clicks.update(|n| *n += 1),
                        "Click me"
                    }
                    TextBlock { text: fmt!("Clicked {} times", clicks.get()) }
                    Slider {
                        minimum: 0.0,
                        maximum: 100.0,
                        orientation: Orientation::Horizontal,
                        value: slider_val.two_way(),
                    }
                    TextBlock { text: fmt!("Slider: {:.0}", slider_val.get()) }
                    TextBox {
                        placeholder_text: "Type something",
                        text: typed.two_way(),
                        on_key_down: move |event| {
                            let symbol = event.key_symbol.as_deref().unwrap_or("");
                            key_readout.set(format!(
                                "Key={:?} Physical={:?} Modifiers={:?} Symbol={symbol}",
                                event.key, event.physical_key, event.key_modifiers
                            ));
                        },
                    }
                    TextBlock { text: fmt!("Text: {}", typed.get()) }
                    TextBlock { text: key_readout }
                    ToggleSwitch {
                        on_content: "On",
                        off_content: "Off",
                        checked: toggled.two_way(),
                        "ToggleSwitch"
                    }
                    TextBlock {
                        text: fmt!("Toggle: {}", if toggled.get() { "on" } else { "off" }),
                    }
                    TextBlock { "Layout" }
                    Border {
                        padding: 8.0,
                        StackPanel {
                            orientation: Orientation::Horizontal,
                            spacing: 8.0,
                            TextBlock {
                                name: "layout_readout",
                                margin: (16.0, 4.0),
                                horizontal_alignment: HorizontalAlignment::Right,
                                vertical_alignment: VerticalAlignment::Center,
                                min_width: 160.0,
                                opacity: 0.7,
                                "Right aligned, inset"
                            }
                        }
                    }
                    TextBlock { "Chrome" }
                    Border {
                        padding: 10.0,
                        background: chrome_gradient,
                        border_brush: Brush::new(Color::rgb(0x00, 0x7A, 0xCC), 0.6),
                        border_thickness: 1.0,
                        corner_radius: CornerRadius::uniform(6.0),
                        TextBlock {
                            foreground: color!("#eeeeee"),
                            font_size: 15.0,
                            font_weight: FontWeight::DemiBold,
                            text_alignment: TextAlignment::Center,
                            padding: Thickness::symmetric(6.0, 2.0),
                            "Gradient & solid brushes, border geometry and text metrics"
                        }
                    }
                    TextBlock { "Grid tracks" }
                    Grid {
                        ref: grid_ref,
                        column_definitions: tracks![auto, 1*, 120px],
                        row_definitions: tracks![auto, auto],
                        column_spacing: 8.0,
                        row_spacing: 4.0,
                        TextBlock { "Auto column" }
                        TextBlock { grid_column: 1, "Star column stretches" }
                        TextBlock { grid_column: 2, "120px" }
                        TextBlock {
                            ref: grid_readout_ref,
                            grid_row: 1,
                            grid_column_span: 3,
                            ""
                        }
                    }
                    TextBlock { "Image, tabs and trees" }
                    { image_el }
                    TextBlock {
                        text: if image_source.is_empty() {
                            String::from("Image source = none (set AVN_IMAGE_SOURCE to a file path)")
                        } else {
                            format!("Image source = {image_source}")
                        },
                    }
                    TabControl {
                        tab_strip_placement: Dock::Top,
                        selected_index: 0,
                        item: TabItem::el()
                            .header("Overview")
                            .content("Tabs inherit Items and SelectedIndex."),
                        item: TabItem::el()
                            .header("Details")
                            .content("TabItem adds only IsSelected."),
                    }
                    TreeView {
                        selection_mode: SelectionMode::SINGLE,
                        item: TreeViewItem::el()
                            .header("Projected controls")
                            .expanded(true)
                            .item(TreeViewItem::el().header("Image"))
                            .item(TreeViewItem::el().header("TabControl"))
                            .item(TreeViewItem::el().header("TreeView")),
                    }
                    TextBlock { "Menus, flyouts, panes and pickers" }
                    Menu {
                        item: MenuItem::el()
                            .header("_File")
                            .item(
                                MenuItem::el()
                                    .header("Save")
                                    .on_click(move |_| menu_status.set(String::from("Save clicked"))),
                            )
                            .item(
                                MenuItem::el()
                                    .header("Word wrap")
                                    .toggle_type(MenuItemToggleType::CheckBox)
                                    .checked(true),
                            ),
                    }
                    TextBlock { text: menu_status }
                    Button {
                        ref: flyout_button_ref,
                        on_click: move |_| -> Result<()> {
                            if let Some(btn) = flyout_target_ref.get() {
                                flyout.show_at_with_control(&btn)?;
                            }
                            Ok(())
                        },
                        "Show flyout"
                    }
                    SplitView {
                        display_mode: SplitViewDisplayMode::CompactInline,
                        pane_placement: SplitViewPanePlacement::Left,
                        open_pane_length: 160.0,
                        compact_pane_length: 40.0,
                        pane_open: true,
                        height: 120.0,
                        pane: StackPanel::el()
                            .orientation(Orientation::Vertical)
                            .spacing(4.0)
                            .child(TextBlock::el().text("Pane")),
                        "SplitView content"
                    }
                    DatePicker { selected_date: selected_date.two_way() }
                    TimePicker {
                        minute_increment: 15,
                        selected_time: selected_time.two_way(),
                    }
                    TextBlock {
                        text: fmt!(
                            "SelectedDate = {:?}, SelectedTime = {:?}",
                            selected_date.get(),
                            selected_time.get()
                        ),
                    }
                    TextBlock { "Selection & expand patterns" }
                    ComboBox {
                        placeholder_text: "Pick a color",
                        max_drop_down_height: 220.0,
                        selected_index: combo_index.two_way(),
                        item: ComboBoxItem::el().content("Red"),
                        item: ComboBoxItem::el().content("Green"),
                        item: ComboBoxItem::el().content("Blue"),
                    }
                    TextBlock { text: fmt!("ComboBox index: {}", combo_index.get()) }
                    ListBox {
                        ref: list_ref,
                        selection_mode: SelectionMode::MULTIPLE,
                        selected_index: list_index.two_way(),
                        item: ListBoxItem::el().content("Item 1"),
                        item: ListBoxItem::el().content("Item 2"),
                        item: ListBoxItem::el().content("Item 3"),
                        item: ListBoxItem::el().content("Item 4"),
                    }
                    StackPanel {
                        orientation: Orientation::Horizontal,
                        spacing: 8.0,
                        Button {
                            on_click: move |_| -> Result<()> {
                                if let Some(lb) = select_all_ref.get() { lb.select_all()?; }
                                Ok(())
                            },
                            "Select all"
                        }
                        Button {
                            on_click: move |_| -> Result<()> {
                                if let Some(lb) = clear_ref.get() { lb.unselect_all()?; }
                                Ok(())
                            },
                            "Clear selection"
                        }
                    }
                    TextBlock { text: fmt!("ListBox index: {}", list_index.get()) }
                    StackPanel {
                        orientation: Orientation::Horizontal,
                        spacing: 8.0,
                        RadioButton { group_name: "DemoGroup", checked: Some(true), "Option A" }
                        RadioButton { group_name: "DemoGroup", "Option B" }
                        RadioButton { group_name: "DemoGroup", "Option C" }
                    }
                    Expander {
                        header: "Expandable section",
                        expand_direction: ExpandDirection::Down,
                        expanded: true,
                        StackPanel {
                            spacing: 4.0,
                            TextBlock { "Content inside the expander." }
                            TextBlock { "Generated from the shared projection IR." }
                        }
                    }
                    TextBlock { "Hover" }
                    Border {
                        on_pointer_entered: move |_| hover_state.set(String::from("over")),
                        on_pointer_exited: move |_| hover_state.set(String::from("out")),
                        TextBlock { text: hover_state }
                    }
                    TextBlock { "Clipboard" }
                    TextBox { text: clipboard_buf.two_way() }
                    StackPanel {
                        orientation: Orientation::Horizontal,
                        spacing: 8.0,
                        { copy_btn }
                        { paste_btn }
                    }
                    TextBlock { text: clipboard_status }
                }
            }
        };

        scope.mount_view(window)?;
        if let Some(img) = image_ref.get() {
            ToolTip::set_tip(&img, "Loaded from Rust through Image.Source")?;
            ToolTip::set_placement(&img, PlacementMode::Top)?;
            ToolTip::set_show_delay(&img, 250)?;
        }
        if let (Some(grid), Some(readout)) = (grid_ref.get(), grid_readout_ref.get()) {
            readout.set_text(format!(
                "columns = {}, rows = {}",
                grid.column_definitions()?,
                grid.row_definitions()?
            ))?;
        }
        Ok(())
    })
}

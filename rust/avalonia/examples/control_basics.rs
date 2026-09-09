use avalonia::{
    selection_mode, App, Border, Brush, Button, ClickMode, Color, ComboBox, ComboBoxItem,
    CornerRadius, DatePicker, Dock, ExpandDirection, Expander, Flyout, FlyoutShowMode, FontWeight,
    Grid, HorizontalAlignment, Image, ListBox, ListBoxItem, Menu, MenuItem, MenuItemToggleType,
    Orientation, PlacementMode, RadioButton, Slider, SplitView, SplitViewDisplayMode,
    SplitViewPanePlacement, StackPanel, Stretch, StretchDirection, TabControl, TabItem,
    TextAlignment, TextBlock, TextBox, Thickness, TimePicker, ToggleSwitch, ToolTip, TreeView,
    TreeViewItem, VerticalAlignment, Window,
};

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let window = Window::new()?.title("Control basics")?.can_resize(true)?;
        let click_count = TextBlock::new()?.text("Clicked 0 times")?;
        let click_count_for_handler = click_count.clone();
        let mut clicks = 0;
        let button = Button::new()?
            .content(Some(&TextBlock::new()?.text("Click me")?))?
            // ClickMode::Press fires on button-down; `default` makes Enter activate it.
            .click_mode(ClickMode::Press)?
            .default(true)?
            .horizontal_content_alignment(HorizontalAlignment::Center)?
            .on_click(scope, move |_| {
                clicks += 1;
                click_count_for_handler
                    .set_text(format!("Clicked {clicks} times"))
                    .expect("failed to update click count");
            })?;

        let slider_value = TextBlock::new()?.text("Slider: 25")?;
        let slider_value_for_handler = slider_value.clone();
        let slider = Slider::new()?
            .minimum(0.0)?
            .maximum(100.0)?
            .value(25.0)?
            .orientation(Orientation::Horizontal)?;
        let slider_for_handler = slider.clone();
        let slider = slider.on_value_changed(scope, move |_| {
            let value = slider_for_handler
                .get_value()
                .expect("failed to read slider value");
            slider_value_for_handler
                .set_text(format!("Slider: {value:.0}"))
                .expect("failed to update slider readout");
        })?;

        let typed_text = TextBlock::new()?.text("Text: Hello from Avalonia")?;
        let typed_text_for_handler = typed_text.clone();
        let text_box = TextBox::new()?
            .placeholder_text("Type something")?
            .text("Hello from Avalonia")?;
        let text_box_for_handler = text_box.clone();
        let text_box = text_box.on_text_changed(scope, move |_| {
            let value = text_box_for_handler
                .get_text()
                .expect("failed to read text")
                .unwrap_or_default();
            typed_text_for_handler
                .set_text(format!("Text: {value}"))
                .expect("failed to update text readout");
        })?;
        let key_readout = TextBlock::new()?.text("Key: focus the text box and press a key")?;
        let key_readout_for_handler = key_readout.clone();
        let text_box = text_box.on_key_down(scope, move |event| {
            let symbol = event.key_symbol.as_deref().unwrap_or("");
            key_readout_for_handler
                .set_text(format!(
                    "Key={} Physical={} Modifiers={} Symbol={symbol}",
                    event.key, event.physical_key, event.key_modifiers
                ))
                .expect("failed to update key readout");
        })?;

        let toggle_value = TextBlock::new()?.text("Toggle: on")?;
        let toggle_value_for_handler = toggle_value.clone();
        let toggle = ToggleSwitch::new()?
            .content(Some(&TextBlock::new()?.text("ToggleSwitch")?))?
            .on_content(Some(&TextBlock::new()?.text("On")?))?
            .off_content(Some(&TextBlock::new()?.text("Off")?))?
            .checked(Some(true))?;
        let toggle_for_handler = toggle.clone();
        let toggle = toggle.on_is_checked_changed(scope, move |_| {
            let state = if toggle_for_handler
                .get_is_checked()
                .expect("failed to read toggle state")
                == Some(true)
            {
                "on"
            } else {
                "off"
            };
            toggle_value_for_handler
                .set_text(format!("Toggle: {state}"))
                .expect("failed to update toggle readout");
        })?;

        let radio_buttons = StackPanel::new()?
            .orientation(Orientation::Horizontal)?
            .spacing(8.0)?
            .child(
                RadioButton::new()?
                    .group_name("DemoGroup")?
                    .content(Some(&TextBlock::new()?.text("Option A")?))?
                    .checked(Some(true))?,
            )?
            .child(
                RadioButton::new()?
                    .group_name("DemoGroup")?
                    .content(Some(&TextBlock::new()?.text("Option B")?))?,
            )?
            .child(
                RadioButton::new()?
                    .group_name("DemoGroup")?
                    .content(Some(&TextBlock::new()?.text("Option C")?))?,
            )?;

        let expander = Expander::new()?
            .header(Some(&TextBlock::new()?.text("Expandable section")?))?
            .content(Some(
                &StackPanel::new()?
                    .spacing(4.0)?
                    .child(TextBlock::new()?.text("Content inside the expander.")?)?
                    .child(TextBlock::new()?.text("Generated from the shared projection IR.")?)?,
            ))?
            .expand_direction(ExpandDirection::Down)?
            .expanded(true)?;

        let combo_status = TextBlock::new()?.text("ComboBox: Blue")?;
        let combo_status_for_handler = combo_status.clone();
        let combo_box = ComboBox::new()?
            .placeholder_text("Pick a color")?
            .max_drop_down_height(220.0)?
            .item(ComboBoxItem::new()?.content(Some(&TextBlock::new()?.text("Red")?))?)?
            .item(ComboBoxItem::new()?.content(Some(&TextBlock::new()?.text("Green")?))?)?
            .item(ComboBoxItem::new()?.content(Some(&TextBlock::new()?.text("Blue")?))?)?
            .selected_index(2)?;
        let combo_box_for_handler = combo_box.clone();
        let combo_box = combo_box.on_selection_changed(scope, move |_| {
            let index = combo_box_for_handler
                .get_selected_index()
                .expect("failed to read ComboBox selection");
            combo_status_for_handler
                .set_text(format!("ComboBox index: {index}"))
                .expect("failed to update ComboBox readout");
        })?;

        let list_status = TextBlock::new()?.text("ListBox: Item 2")?;
        let list_status_for_handler = list_status.clone();
        let list_box = ListBox::new()?
            .selection_mode(selection_mode::MULTIPLE)?
            .item(ListBoxItem::new()?.content(Some(&TextBlock::new()?.text("Item 1")?))?)?
            .item(ListBoxItem::new()?.content(Some(&TextBlock::new()?.text("Item 2")?))?)?
            .item(ListBoxItem::new()?.content(Some(&TextBlock::new()?.text("Item 3")?))?)?
            .item(ListBoxItem::new()?.content(Some(&TextBlock::new()?.text("Item 4")?))?)?
            .selected_index(1)?;
        let list_box_for_handler = list_box.clone();
        let list_box = list_box.on_selection_changed(scope, move |_| {
            let index = list_box_for_handler
                .get_selected_index()
                .expect("failed to read ListBox selection");
            list_status_for_handler
                .set_text(format!("ListBox index: {index}"))
                .expect("failed to update ListBox readout");
        })?;

        let select_all_target = list_box.clone();
        let unselect_all_target = list_box.clone();
        let list_commands = StackPanel::new()?
            .orientation(Orientation::Horizontal)?
            .spacing(8.0)?
            .child(
                Button::new()?
                    .content(Some(&TextBlock::new()?.text("Select all")?))?
                    .on_click(scope, move |_| {
                        select_all_target
                            .select_all()
                            .expect("failed to select every item");
                    })?,
            )?
            .child(
                Button::new()?
                    .content(Some(&TextBlock::new()?.text("Clear selection")?))?
                    .on_click(scope, move |_| {
                        unselect_all_target
                            .unselect_all()
                            .expect("failed to clear the selection");
                    })?,
            )?;

        let hover_state = TextBlock::new()?.text("out")?;
        let entered_state = hover_state.clone();
        let exited_state = hover_state.clone();
        let hover_panel = Border::new()?
            .child(Some(&hover_state))?
            .on_pointer_entered(scope, move |_| {
                entered_state
                    .set_text("over")
                    .expect("failed to update hover state");
            })?
            .on_pointer_exited(scope, move |_| {
                exited_state
                    .set_text("out")
                    .expect("failed to update hover state");
            })?;

        let clipboard_text = TextBox::new()?.text("Hello clipboard")?;
        let clipboard_status = TextBlock::new()?.text("")?;

        let copy_scope = scope.clone();
        let copy_window = window.clone();
        let copy_text = clipboard_text.clone();
        let copy_status = clipboard_status.clone();
        let copy = Button::new()?
            .content(Some(&TextBlock::new()?.text("Copy")?))?
            .on_click(scope, move |_| {
                let text = copy_text
                    .get_text()
                    .expect("failed to read clipboard text")
                    .unwrap_or_default();
                match copy_scope.clipboard_set_text(&copy_window, &text) {
                    Ok(operation) => {
                        let status = copy_status.clone();
                        copy_scope
                            .spawn(async move {
                                match operation.await {
                                    Ok(()) => status
                                        .set_text(format!("copied {} chars", text.len()))
                                        .expect("failed to update clipboard status"),
                                    Err(error) => status
                                        .set_text(error.to_string())
                                        .expect("failed to update clipboard status"),
                                }
                            })
                            .expect("failed to spawn clipboard copy");
                    }
                    Err(error) => copy_status
                        .set_text(error.to_string())
                        .expect("failed to update clipboard status"),
                }
            })?;

        let paste_scope = scope.clone();
        let paste_window = window.clone();
        let paste_text = clipboard_text.clone();
        let paste_status = clipboard_status.clone();
        let paste = Button::new()?
            .content(Some(&TextBlock::new()?.text("Paste")?))?
            .on_click(scope, move |_| {
                match paste_scope.clipboard_get_text(&paste_window) {
                    Ok(operation) => {
                        let text_box = paste_text.clone();
                        let status = paste_status.clone();
                        paste_scope
                            .spawn(async move {
                                match operation.await {
                                    Ok(Some(text)) => {
                                        text_box
                                            .set_text(&text)
                                            .expect("failed to update clipboard text");
                                        status
                                            .set_text(format!("pasted {} chars", text.len()))
                                            .expect("failed to update clipboard status");
                                    }
                                    Ok(None) => status
                                        .set_text("no text on clipboard")
                                        .expect("failed to update clipboard status"),
                                    Err(error) => status
                                        .set_text(error.to_string())
                                        .expect("failed to update clipboard status"),
                                }
                            })
                            .expect("failed to spawn clipboard paste");
                    }
                    Err(error) => paste_status
                        .set_text(error.to_string())
                        .expect("failed to update clipboard status"),
                }
            })?;

        let layout_demo = Border::new()?
            .padding(Thickness::uniform(8.0))?
            .child(Some(
                &StackPanel::new()?
                    .orientation(Orientation::Horizontal)?
                    .spacing(8.0)?
                    .child(
                        TextBlock::new()?
                            .text("Right aligned, inset")?
                            .name("layout_readout")?
                            .margin(Thickness::symmetric(16.0, 4.0))?
                            .horizontal_alignment(HorizontalAlignment::Right)?
                            .vertical_alignment(VerticalAlignment::Center)?
                            .min_width(160.0)?
                            .opacity(0.7)?,
                    )?,
            ))?;

        let chrome_demo = Border::new()?
            .padding(Thickness::uniform(10.0))?
            .background(Brush::solid(Color::rgb(0x22, 0x27, 0x2E)))?
            .border_brush(Brush::new(Color::rgb(0x00, 0x7A, 0xCC), 0.6))?
            .border_thickness(Thickness::uniform(1.0))?
            .corner_radius(CornerRadius::uniform(6.0))?
            .child(Some(
                &TextBlock::new()?
                    .text("Solid brushes, border geometry and text metrics")?
                    .foreground(Brush::solid(Color::rgb(0xEE, 0xEE, 0xEE)))?
                    .font_size(15.0)?
                    .font_weight(FontWeight::DemiBold)?
                    .text_alignment(TextAlignment::Center)?
                    .padding(Thickness::symmetric(6.0, 2.0))?,
            ))?;

        // Grid tracks are the same comma-separated length list AXAML uses: `*` takes the
        // remaining space, `Auto` sizes to content, and a bare number is a fixed size.
        let grid_demo = Grid::new()?
            .column_definitions("Auto,*,120")?
            .row_definitions("Auto,Auto")?
            .column_spacing(8.0)?
            .row_spacing(4.0)?;
        let grid_label = TextBlock::new()?.text("Auto column")?;
        let grid_stretch = TextBlock::new()?.text("Star column stretches")?;
        let grid_fixed = TextBlock::new()?.text("120px")?;
        let grid_readout = TextBlock::new()?.text(format!(
            "columns = {}, rows = {}",
            grid_demo.get_column_definitions()?,
            grid_demo.get_row_definitions()?
        ))?;
        Grid::set_column(&grid_stretch, 1)?;
        Grid::set_column(&grid_fixed, 2)?;
        Grid::set_row(&grid_readout, 1)?;
        Grid::set_column_span(&grid_readout, 3)?;
        let grid_demo = grid_demo
            .child(grid_label)?
            .child(grid_stretch)?
            .child(grid_fixed)?
            .child(grid_readout)?;

        // Image.Source is the *source string* the host resolves into a bitmap: a file path, a
        // file:// URI, or an avares://-style asset URI. Point AVN_IMAGE_SOURCE at any image on
        // disk to see it; the label reports whatever the getter hands back, which is the string
        // that was set rather than the bitmap's identity.
        let image_source = std::env::var("AVN_IMAGE_SOURCE").unwrap_or_default();
        let image = Image::new()?
            .stretch(Stretch::Uniform)?
            .stretch_direction(StretchDirection::DownOnly)?
            .max_height(96.0)?;
        if !image_source.is_empty() {
            image.set_source(&image_source)?;
        }
        let image_readout = TextBlock::new()?.text(match image.get_source()? {
            Some(source) => format!("Image source = {source}"),
            None => "Image source = none (set AVN_IMAGE_SOURCE to a file path)".to_string(),
        })?;
        // A tooltip is an attached property, and over the ABI it carries text only.
        ToolTip::set_tip(&image, "Loaded from Rust through Image.Source")?;
        ToolTip::set_placement(&image, PlacementMode::Top)?;
        ToolTip::set_show_delay(&image, 250)?;

        let tabs = TabControl::new()?
            .tab_strip_placement(Dock::Top)?
            .item(
                TabItem::new()?
                    .header(Some(&TextBlock::new()?.text("Overview")?))?
                    .content(Some(
                        &TextBlock::new()?.text("Tabs inherit Items and SelectedIndex.")?,
                    ))?,
            )?
            .item(
                TabItem::new()?
                    .header(Some(&TextBlock::new()?.text("Details")?))?
                    .content(Some(
                        &TextBlock::new()?.text("TabItem adds only IsSelected.")?,
                    ))?,
            )?
            .selected_index(0)?;

        // A TreeViewItem is an ItemsControl, so its children go into the inherited Items slot.
        let tree = TreeView::new()?
            .selection_mode(selection_mode::SINGLE)?
            .item(
                TreeViewItem::new()?
                    .header(Some(&TextBlock::new()?.text("Projected controls")?))?
                    .item(TreeViewItem::new()?.header(Some(&TextBlock::new()?.text("Image")?))?)?
                    .item(
                        TreeViewItem::new()?
                            .header(Some(&TextBlock::new()?.text("TabControl")?))?,
                    )?
                    .item(TreeViewItem::new()?.header(Some(&TextBlock::new()?.text("TreeView")?))?)?
                    .expanded(true)?,
            )?;

        // A Menu here is the imperative ItemsControl, not the view-model NativeMenu: its items
        // are real projected controls and Click replaces ICommand, which has no ABI shape.
        let menu_status = TextBlock::new()?.text("No menu item clicked yet")?;
        let menu_status_for_click = menu_status.clone();
        let menu = Menu::new()?.item(
            MenuItem::new()?
                .header(Some(&TextBlock::new()?.text("_File")?))?
                .item(
                    MenuItem::new()?
                        .header(Some(&TextBlock::new()?.text("Save")?))?
                        .on_click(scope, move |_| {
                            menu_status_for_click.set_text("Save clicked").unwrap();
                        })?,
                )?
                .item(
                    MenuItem::new()?
                        .header(Some(&TextBlock::new()?.text("Word wrap")?))?
                        .toggle_type(MenuItemToggleType::CheckBox)?
                        .checked(true)?,
                )?,
        )?;

        // A flyout is an AvaloniaObject, not a Control, so it is not a child of the panel. It
        // reaches a control through show_at rather than through an attached property.
        let flyout = Flyout::new()?
            .content(Some(
                &TextBlock::new()?.text("Shown with flyout.show_at_with_control")?,
            ))?
            .placement(PlacementMode::BottomEdgeAlignedLeft)?
            .show_mode(FlyoutShowMode::Transient)?;
        let flyout_button =
            Button::new()?.content(Some(&TextBlock::new()?.text("Show flyout")?))?;
        let flyout_target = flyout_button.clone();
        let flyout_button = flyout_button.on_click(scope, move |_| {
            flyout.show_at_with_control(&flyout_target).unwrap();
        })?;

        let split_view = SplitView::new()?
            .display_mode(SplitViewDisplayMode::CompactInline)?
            .pane_placement(SplitViewPanePlacement::Left)?
            .open_pane_length(160.0)?
            .compact_pane_length(40.0)?
            .pane_open(true)?
            .height(120.0)?
            .pane(Some(
                &StackPanel::new()?
                    .orientation(Orientation::Vertical)?
                    .spacing(4.0)?
                    .child(TextBlock::new()?.text("Pane")?)?,
            ))?
            .content(Some(&TextBlock::new()?.text("SplitView content")?))?;

        // Dates and times cross as ISO-8601 text. Writing takes a bare yyyy-MM-dd or HH:mm;
        // reading normalises to the round-trip form, so the readout is not what was written.
        let date_picker = DatePicker::new()?.selected_date("2027-01-15")?;
        let time_picker = TimePicker::new()?
            .minute_increment(15)?
            .selected_time("17:04")?;
        let picker_readout = TextBlock::new()?.text(format!(
            "SelectedDate = {:?}, SelectedTime = {:?}",
            date_picker.get_selected_date()?,
            time_picker.get_selected_time()?
        ))?;

        window.set_content(Some(
            &StackPanel::new()?
                .orientation(Orientation::Vertical)?
                .spacing(8.0)?
                .margin(Thickness::uniform(12.0))?
                .child(button)?
                .child(click_count)?
                .child(slider)?
                .child(slider_value)?
                .child(text_box)?
                .child(typed_text)?
                .child(key_readout)?
                .child(toggle)?
                .child(toggle_value)?
                .child(TextBlock::new()?.text("Layout")?)?
                .child(layout_demo)?
                .child(TextBlock::new()?.text("Chrome")?)?
                .child(chrome_demo)?
                .child(TextBlock::new()?.text("Grid tracks")?)?
                .child(grid_demo)?
                .child(TextBlock::new()?.text("Image, tabs and trees")?)?
                .child(image)?
                .child(image_readout)?
                .child(tabs)?
                .child(tree)?
                .child(TextBlock::new()?.text("Menus, flyouts, panes and pickers")?)?
                .child(menu)?
                .child(menu_status)?
                .child(flyout_button)?
                .child(split_view)?
                .child(date_picker)?
                .child(time_picker)?
                .child(picker_readout)?
                .child(TextBlock::new()?.text("Selection & expand patterns")?)?
                .child(combo_box)?
                .child(combo_status)?
                .child(list_box)?
                .child(list_commands)?
                .child(list_status)?
                .child(radio_buttons)?
                .child(expander)?
                .child(TextBlock::new()?.text("Hover")?)?
                .child(hover_panel)?
                .child(TextBlock::new()?.text("Clipboard")?)?
                .child(clipboard_text)?
                .child(
                    StackPanel::new()?
                        .orientation(Orientation::Horizontal)?
                        .spacing(8.0)?
                        .child(copy)?
                        .child(paste)?,
                )?
                .child(clipboard_status)?,
        ))?;
        scope.mount(window)
    })
}

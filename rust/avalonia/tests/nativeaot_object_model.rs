use avalonia::{
    selection_mode, App, Border, Brush, Button, ClickMode, Color, ComboBox, ComboBoxItem,
    ContextMenu, CornerRadius, DatePicker, Dock, DockPanel, DropDownButton, ExpandDirection,
    Expander, FlexAlignItems, FlexDirection, FlexJustifyContent, FlexPanel, FlexWrap, Flyout,
    FlyoutShowMode, FontWeight, Grid, GridResizeBehavior, GridResizeDirection, GridSplitter,
    HorizontalAlignment, HyperlinkButton, Image, ListBox, ListBoxItem, Menu, MenuFlyout, MenuItem,
    MenuItemToggleType, Orientation, PlacementMode, ProgressBar, RadioButton, RelativePanel,
    RepeatButton, ScrollViewer, SplitButton, SplitView, SplitViewDisplayMode,
    SplitViewPanePlacement, StackPanel, Stretch, StretchDirection, TabControl, TabItem,
    TextAlignment, TextBlock, TextBox, ThemeVariant, Thickness, TimePicker, ToggleSplitButton,
    ToggleSwitch, ToolTip, TreeView, TreeViewItem, UniformGrid, VerticalAlignment, Viewbox, Window,
    WindowState, WrapPanel, WrapPanelItemsAlignment,
};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

/// A 1x1 transparent PNG, written to disk so `Image::set_source` has a real file to decode.
const ONE_PIXEL_PNG: [u8; 70] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0xFC, 0xCF, 0xC0, 0x50,
    0x0F, 0x00, 0x04, 0x85, 0x01, 0x80, 0x84, 0xA9, 0x8C, 0x21, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn one_pixel_png_path() -> PathBuf {
    let path = std::env::temp_dir().join("avalonia_rust_wave_a_pixel.png");
    std::fs::write(&path, ONE_PIXEL_PNG).expect("write the test image");
    path
}

fn host_path() -> PathBuf {
    if let Ok(path) = std::env::var("AVN_HOST_NATIVE_LIB") {
        return PathBuf::from(path);
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    #[cfg(target_os = "windows")]
    let candidates = [
        "host/bin/Release/net10.0/win-x64/publish/Avalonia.Host.dll",
        "host/bin/Release/net10.0/win-arm64/publish/Avalonia.Host.dll",
        "host/bin/Debug/net10.0/win-x64/publish/Avalonia.Host.dll",
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        "rust/target/dotnet-linux-x64/publish/Avalonia.Host/release_linux-x64/Avalonia.Host.so",
        "rust/target/dotnet-linux-arm64/publish/Avalonia.Host/release_linux-arm64/Avalonia.Host.so",
    ];
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let candidates: [&str; 0] = [];

    candidates
        .into_iter()
        .map(|relative| root.join(relative))
        .find(|path| path.exists())
        .unwrap_or_else(|| {
            panic!(
                "Avalonia.Host native library not found. Publish with \
                 `rust/build.ps1` on Windows or `rust/build.sh` on Linux, or set \
                 AVN_HOST_NATIVE_LIB."
            )
        })
}

struct ThreadWake(std::thread::Thread);

impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = Box::pin(future);
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    loop {
        match Pin::new(&mut future).poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

// Creates a real OS window and waits on routed events through the NativeAOT host; the
// event loop never services those events under headless CI runners or headless local
// runs (the assertion flags stay unset). Tracked for follow-up; the managed suite
// (tests/Avalonia.Host.Tests) covers the underlying ABI surface.
#[ignore = "routed events never fire under headless runners (tracked)"]
#[test]
fn builders_create_a_real_window_through_nativeaot() {
    let called = Arc::new(AtomicBool::new(false));
    let called_from_handler = called.clone();
    let posted = Arc::new(AtomicBool::new(false));
    let posted_from_handler = posted.clone();
    let text_changed = Arc::new(AtomicBool::new(false));
    let text_changed_from_handler = text_changed.clone();
    let toggle_changed = Arc::new(AtomicBool::new(false));
    let toggle_changed_from_handler = toggle_changed.clone();
    let combo_changed = Arc::new(AtomicBool::new(false));
    let combo_changed_from_handler = combo_changed.clone();
    let list_changed = Arc::new(AtomicBool::new(false));
    let list_changed_from_handler = list_changed.clone();
    let tree_collapsed = Arc::new(AtomicBool::new(false));
    let tree_collapsed_from_handler = tree_collapsed.clone();
    let menu_opened = Arc::new(AtomicBool::new(false));
    let menu_opened_from_handler = menu_opened.clone();
    let app = App::load(host_path()).unwrap();

    app.run(move |scope| {
        assert!(scope.check_access()?);
        scope.set_requested_theme_variant(ThemeVariant::Dark)?;
        assert_eq!(scope.requested_theme_variant()?, ThemeVariant::Dark);
        assert_eq!(
            scope.find_resource("__missing_rust_resource__", ThemeVariant::Dark)?,
            None
        );
        let button = Button::new()?.content(Some(&TextBlock::new()?.text("Close")?))?;
        let classes = button.classes()?;
        classes.add("primary")?;
        assert!(classes.contains("primary")?);
        assert!(classes.remove_value("primary")?);
        Grid::set_row(&button, 1)?;
        assert_eq!(Grid::get_row(&button)?, 1);
        button.subscribe_click(|_| {})?.unsubscribe()?;
        let button = button
            .on_click(scope, |_| {})?
            .background(Brush::solid(Color::rgb(0x00, 0x7A, 0xCC)))?
            .foreground(Brush::solid(Color::rgb(0xFF, 0xFF, 0xFF)))?
            .border_thickness(Thickness::uniform(1.0))?
            .corner_radius(CornerRadius::uniform(3.0))?
            .font_size(14.0)?
            .click_mode(ClickMode::Press)?
            .default(true)?
            .cancel(false)?
            .horizontal_content_alignment(HorizontalAlignment::Center)?
            .vertical_content_alignment(VerticalAlignment::Center)?;
        assert_eq!(button.get_font_size()?, 14.0);
        assert_eq!(button.get_click_mode()?, ClickMode::Press);
        assert!(button.get_is_default()?);
        assert!(!button.get_is_cancel()?);
        // IsPressed is raised by Avalonia's input handling, so it reads back but never sets.
        assert!(!button.is_pressed()?);
        assert_eq!(
            button.get_horizontal_content_alignment()?,
            HorizontalAlignment::Center
        );
        assert_eq!(
            button.get_vertical_content_alignment()?,
            VerticalAlignment::Center
        );
        assert_eq!(
            button.get_background()?,
            Some(Brush::solid(Color::rgb(0x00, 0x7A, 0xCC)))
        );
        assert_eq!(
            button.get_foreground()?,
            Some(Brush::solid(Color::rgb(0xFF, 0xFF, 0xFF)))
        );
        assert_eq!(button.get_corner_radius()?, CornerRadius::uniform(3.0));

        let text_box = TextBox::new()?
            .text("before")?
            .on_text_changed(scope, move |_| {
                text_changed_from_handler.store(true, Ordering::SeqCst);
            })?;
        DockPanel::set_dock(&text_box, Dock::Top)?;
        assert_eq!(DockPanel::get_dock(&text_box)?, Dock::Top);
        let text_box_for_post = text_box.clone();
        let text_box = ScrollViewer::new()?.content(Some(&DockPanel::new()?.child(text_box)?))?;

        let toggle = ToggleSwitch::new()?
            .content(Some(&TextBlock::new()?.text("ToggleSwitch")?))?
            .on_content(Some(&TextBlock::new()?.text("On")?))?
            .off_content(Some(&TextBlock::new()?.text("Off")?))?
            .checked(Some(true))?
            .three_state(true)?
            .on_is_checked_changed(scope, move |_| {
                toggle_changed_from_handler.store(true, Ordering::SeqCst);
            })?;
        assert!(toggle.get_is_three_state()?);
        let toggle_for_post = toggle.clone();

        let radio_one = RadioButton::new()?
            .group_name("TestGroup")?
            .content(Some(&TextBlock::new()?.text("One")?))?
            .checked(Some(true))?;
        let radio_two = RadioButton::new()?
            .group_name("TestGroup")?
            .content(Some(&TextBlock::new()?.text("Two")?))?;
        let radio_one_for_post = radio_one.clone();
        let radio_two_for_post = radio_two.clone();
        let radio_buttons = StackPanel::new()?.child(radio_one)?.child(radio_two)?;

        let expander = Expander::new()?
            .header(Some(&TextBlock::new()?.text("Header")?))?
            .content(Some(&TextBlock::new()?.text("Content")?))?
            .expand_direction(ExpandDirection::Down)?
            .expanded(true)?;
        assert_eq!(expander.get_expand_direction()?, ExpandDirection::Down);
        assert!(expander.get_is_expanded()?);
        expander.subscribe_expanded(|_| {})?.unsubscribe()?;

        let combo_box = ComboBox::new()?
            .placeholder_text("Pick")?
            .max_drop_down_height(240.0)?
            .item(ComboBoxItem::new()?.content(Some(&TextBlock::new()?.text("First")?))?)?
            .item(ComboBoxItem::new()?.content(Some(&TextBlock::new()?.text("Second")?))?)?
            .on_selection_changed(scope, move |_| {
                combo_changed_from_handler.store(true, Ordering::SeqCst);
            })?;
        assert_eq!(combo_box.items()?.len()?, 2);
        assert!(!combo_box.get_is_editable()?);
        assert!(!combo_box.get_is_drop_down_open()?);
        assert_eq!(combo_box.get_max_drop_down_height()?, 240.0);
        let combo_box_for_post = combo_box.clone();

        let list_box = ListBox::new()?
            .selection_mode(selection_mode::MULTIPLE)?
            .item(ListBoxItem::new()?.content(Some(&TextBlock::new()?.text("First")?))?)?
            .item(ListBoxItem::new()?.content(Some(&TextBlock::new()?.text("Second")?))?)?
            .on_selection_changed(scope, move |_| {
                list_changed_from_handler.store(true, Ordering::SeqCst);
            })?;
        assert_eq!(list_box.items()?.len()?, 2);
        assert_eq!(list_box.get_selection_mode()?, selection_mode::MULTIPLE);
        let list_box_for_post = list_box.clone();

        let hover_panel = Border::new()?.child(Some(&TextBlock::new()?.text("Hover")?))?;
        hover_panel
            .subscribe_pointer_entered(|_| {})?
            .unsubscribe()?;
        hover_panel
            .subscribe_pointer_exited(|_| {})?
            .unsubscribe()?;

        let progress = ProgressBar::new()?
            .minimum(0.0)?
            .maximum(100.0)?
            .value(40.0)?
            .show_progress_text(true)?
            .progress_text_format("{0:0}%")?;
        assert_eq!(progress.get_value()?, 40.0);
        assert!(progress.get_show_progress_text()?);

        let text_readout = TextBlock::new()?
            .text("Laid out from Rust")?
            .name("layout_readout")?
            .margin(Thickness::symmetric(12.0, 4.0))?
            .horizontal_alignment(HorizontalAlignment::Right)?
            .vertical_alignment(VerticalAlignment::Bottom)?
            .min_width(64.0)?
            .min_height(16.0)?
            .max_width(320.0)?
            .max_height(96.0)?
            .opacity(0.75)?
            .font_size(18.0)?
            .font_weight(FontWeight::Bold)?
            .text_alignment(TextAlignment::Center)?
            .padding(Thickness::symmetric(3.0, 5.0))?
            .foreground(Brush::solid(Color::rgb(0xFF, 0xFF, 0xFF)))?;
        assert_eq!(text_readout.get_font_size()?, 18.0);
        assert_eq!(text_readout.get_font_weight()?, FontWeight::Bold);
        assert_eq!(text_readout.get_text_alignment()?, TextAlignment::Center);
        assert_eq!(
            text_readout.get_padding()?,
            Thickness::new(3.0, 5.0, 3.0, 5.0)
        );
        assert_eq!(
            text_readout.get_foreground()?,
            Some(Brush::solid(Color::rgb(0xFF, 0xFF, 0xFF)))
        );

        let laid_out = Border::new()?
            .padding(Thickness::uniform(6.0))?
            .background(Brush::solid(Color::rgb(0x33, 0x66, 0x99)))?
            .border_brush(Brush::new(Color::rgb(0xAA, 0xBB, 0xCC), 0.5))?
            .border_thickness(Thickness::uniform(2.0))?
            .corner_radius(CornerRadius::uniform(4.0))?
            .child(Some(&text_readout))?;
        assert_eq!(laid_out.get_padding()?, Thickness::uniform(6.0));
        assert_eq!(
            laid_out.get_background()?,
            Some(Brush::solid(Color::rgb(0x33, 0x66, 0x99)))
        );
        assert_eq!(
            laid_out.get_border_brush()?,
            Some(Brush::new(Color::rgb(0xAA, 0xBB, 0xCC), 0.5))
        );
        assert_eq!(laid_out.get_border_thickness()?, Thickness::uniform(2.0));
        assert_eq!(laid_out.get_corner_radius()?, CornerRadius::uniform(4.0));

        // Clearing a brush is a first-class state, not a transparent colour.
        laid_out.set_border_brush(None)?;
        assert_eq!(laid_out.get_border_brush()?, None);
        laid_out.set_border_brush(Brush::new(Color::rgb(0xAA, 0xBB, 0xCC), 0.5))?;

        let readout = laid_out.get_child()?.expect("border child");
        assert_eq!(readout.get_margin()?, Thickness::new(12.0, 4.0, 12.0, 4.0));
        assert_eq!(
            readout.get_horizontal_alignment()?,
            HorizontalAlignment::Right
        );
        assert_eq!(readout.get_vertical_alignment()?, VerticalAlignment::Bottom);
        assert_eq!(readout.get_min_width()?, 64.0);
        assert_eq!(readout.get_max_height()?, 96.0);
        assert_eq!(readout.get_opacity()?, 0.75);
        assert_eq!(readout.get_name()?.as_deref(), Some("layout_readout"));
        assert!(readout.get_is_visible()?);
        readout.set_visible(false)?;
        assert!(!readout.get_is_visible()?);
        readout.set_visible(true)?;

        // Grid tracks cross as the comma-separated length list Avalonia already parses. `*` is
        // shorthand for `1*`, so the getter reports the list Avalonia normalised rather than the
        // exact characters that were written; writing that list back is a fixed point.
        let tracks = Grid::new()?
            .column_definitions("*,Auto,120")?
            .row_definitions("Auto,2*")?
            .column_spacing(4.0)?
            .row_spacing(2.0)?;
        assert_eq!(tracks.get_column_definitions()?, "1*,Auto,120");
        assert_eq!(tracks.get_row_definitions()?, "Auto,2*");
        tracks.set_column_definitions(tracks.get_column_definitions()?)?;
        assert_eq!(tracks.get_column_definitions()?, "1*,Auto,120");

        // Clearing the tracks is an empty list, not a null.
        tracks.set_row_definitions("")?;
        assert_eq!(tracks.get_row_definitions()?, "");
        tracks.set_row_definitions("Auto,2*")?;

        let cell = TextBlock::new()?.text("Cell")?;
        Grid::set_column(&cell, 1)?;
        Grid::set_row(&cell, 1)?;
        assert_eq!(Grid::get_column(&cell)?, 1);
        let tracks = tracks.child(cell)?;

        // Image.Source crosses as the source string the host resolves into a bitmap, so the
        // getter hands back the exact path that was set rather than the bitmap's type name.
        let image_path = one_pixel_png_path();
        let image_path_text = image_path.to_string_lossy().into_owned();
        let image = Image::new()?
            .source(&image_path_text)?
            .stretch(Stretch::UniformToFill)?
            .stretch_direction(StretchDirection::DownOnly)?;
        assert_eq!(
            image.get_source()?.as_deref(),
            Some(image_path_text.as_str())
        );
        assert_eq!(image.get_stretch()?, Stretch::UniformToFill);
        assert_eq!(image.get_stretch_direction()?, StretchDirection::DownOnly);
        // Clearing the source is a first-class state, not an empty path.
        image.set_source("")?;
        assert_eq!(image.get_source()?, None);
        image.set_source(&image_path_text)?;

        // ToolTip.Tip is an attached property that carries text and nothing else.
        ToolTip::set_tip(&image, "One pixel, loaded from Rust")?;
        ToolTip::set_show_delay(&image, 250)?;
        ToolTip::set_placement(&image, PlacementMode::Top)?;
        assert_eq!(
            ToolTip::get_tip(&image)?.as_deref(),
            Some("One pixel, loaded from Rust")
        );
        assert_eq!(ToolTip::get_show_delay(&image)?, 250);
        assert_eq!(ToolTip::get_placement(&image)?, PlacementMode::Top);

        let tabs = TabControl::new()?
            .tab_strip_placement(Dock::Top)?
            .item(
                TabItem::new()?
                    .header(Some(&TextBlock::new()?.text("First")?))?
                    .content(Some(&TextBlock::new()?.text("First page")?))?,
            )?
            .item(
                TabItem::new()?
                    .header(Some(&TextBlock::new()?.text("Second")?))?
                    .content(Some(&TextBlock::new()?.text("Second page")?))?,
            )?
            .selected_index(1)?;
        assert_eq!(tabs.items()?.len()?, 2);
        assert_eq!(tabs.get_tab_strip_placement()?, Dock::Top);
        assert_eq!(tabs.get_selected_index()?, 1);

        // A TreeViewItem is an ItemsControl, so children go into the inherited Items slot and
        // Level is maintained by the control rather than written from Rust.
        let leaf = TreeViewItem::new()?.header(Some(&TextBlock::new()?.text("Leaf")?))?;
        let branch = TreeViewItem::new()?
            .header(Some(&TextBlock::new()?.text("Branch")?))?
            .item(leaf)?
            .expanded(true)?;
        assert!(branch.get_is_expanded()?);
        assert_eq!(branch.level()?, 0);
        branch.subscribe_expanded(|_| {})?.unsubscribe()?;
        let branch = branch.on_collapsed(scope, move |_| {
            tree_collapsed_from_handler.store(true, Ordering::SeqCst);
        })?;
        let tree = TreeView::new()?
            .selection_mode(selection_mode::SINGLE)?
            .auto_scroll_to_selected_item(false)?
            .item(branch.clone())?;
        assert_eq!(tree.items()?.len()?, 1);
        assert_eq!(tree.get_selection_mode()?, selection_mode::SINGLE);
        assert!(!tree.get_auto_scroll_to_selected_item()?);
        tree.subscribe_selection_changed(|_| {})?.unsubscribe()?;
        tree.collapse_sub_tree_with_tree_view_item(&branch)?;
        assert!(!branch.get_is_expanded()?);
        tree.unselect_all()?;

        // Wave B. A Menu is an imperative ItemsControl, not the view-model NativeMenu: it opens
        // and closes through methods and its items are real controls.
        let save_item = MenuItem::new()?
            .header(Some(&TextBlock::new()?.text("Save")?))?
            .icon(Some(&Image::new()?))?
            .toggle_type(MenuItemToggleType::CheckBox)?
            .checked(true)?
            .group_name("edits")?;
        assert!(save_item.get_is_checked()?);
        assert_eq!(save_item.get_toggle_type()?, MenuItemToggleType::CheckBox);
        assert_eq!(save_item.get_group_name()?.as_deref(), Some("edits"));
        save_item.subscribe_click(|_| {})?.unsubscribe()?;
        let file_menu_item = MenuItem::new()?
            .header(Some(&TextBlock::new()?.text("File")?))?
            .item(save_item)?;
        assert_eq!(file_menu_item.items()?.len()?, 1);

        let menu = Menu::new()?.item(file_menu_item)?;
        assert!(!menu.is_open()?);
        let menu = menu.on_opened(scope, move |_| {
            menu_opened_from_handler.store(true, Ordering::SeqCst);
        })?;
        menu.subscribe_closed(|_| {})?.unsubscribe()?;
        menu.open()?;
        assert!(menu.is_open()?);
        menu.close()?;
        assert!(!menu.is_open()?);

        // A flyout is an AvaloniaObject rather than a Control, so it is not a child of anything;
        // it reaches a control through show_at instead of through an attached property.
        let flyout = Flyout::new()?
            .content(Some(&TextBlock::new()?.text("Flyout body")?))?
            .placement(PlacementMode::BottomEdgeAlignedLeft)?
            .show_mode(FlyoutShowMode::Transient)?
            .horizontal_offset(6.0)?;
        assert_eq!(
            flyout.get_placement()?,
            PlacementMode::BottomEdgeAlignedLeft
        );
        assert_eq!(flyout.get_show_mode()?, FlyoutShowMode::Transient);
        assert!(!flyout.get_is_open()?);
        assert!(flyout.target()?.is_none());
        flyout.subscribe_opened(|_| {})?.unsubscribe()?;
        flyout
            .subscribe_closing(|arguments| arguments.cancel = false)?
            .unsubscribe()?;
        // Hiding a flyout that was never shown is a no-op rather than an error.
        flyout.hide()?;

        let split_view = SplitView::new()?
            .display_mode(SplitViewDisplayMode::CompactOverlay)?
            .pane_placement(SplitViewPanePlacement::Left)?
            .open_pane_length(220.0)?
            .compact_pane_length(48.0)?
            .pane(Some(
                &StackPanel::new()?.child(TextBlock::new()?.text("Pane")?)?,
            ))?
            .pane_background(Brush::solid(Color::rgb(0x22, 0x22, 0x22)))?
            .content(Some(&TextBlock::new()?.text("Body")?))?;
        split_view.subscribe_pane_opened(|_| {})?.unsubscribe()?;
        split_view.set_pane_open(true)?;
        assert!(split_view.get_is_pane_open()?);
        assert_eq!(
            split_view.get_display_mode()?,
            SplitViewDisplayMode::CompactOverlay
        );
        assert_eq!(split_view.get_open_pane_length()?, 220.0);
        assert!(split_view.get_pane()?.is_some());
        assert_eq!(
            split_view.get_pane_background()?,
            Some(Brush::solid(Color::rgb(0x22, 0x22, 0x22)))
        );

        // A DateTimeOffset has no ABI shape here, so a date crosses as ISO-8601 text. Writing
        // accepts a bare yyyy-MM-dd; reading always produces the round-trip "o" form, so the
        // getter is not the string that was written.
        let date_picker = DatePicker::new()?
            .min_year("2000-01-01T00:00:00.0000000+00:00")?
            .max_year("2100-12-31T00:00:00.0000000+00:00")?
            .selected_date("2027-01-15T08:30:00.0000000+02:00")?
            .month_format("MMMM")?;
        assert_eq!(
            date_picker.get_selected_date()?.as_deref(),
            Some("2027-01-15T08:30:00.0000000+02:00")
        );
        assert_eq!(
            date_picker.get_min_year()?,
            "2000-01-01T00:00:00.0000000+00:00"
        );
        // A locale spelling is ambiguous, so it fails the call rather than being guessed at.
        assert!(date_picker.set_selected_date("03/09/2026").is_err());
        // Clearing the selection is a first-class state; MinYear has no such state.
        date_picker.set_selected_date("")?;
        assert_eq!(date_picker.get_selected_date()?, None);
        assert!(date_picker.set_min_year("").is_err());
        date_picker.clear()?;

        // A TimePicker selection is a time of day, so it crosses as ISO-8601 HH:mm:ss rather
        // than as an ISO-8601 duration.
        let time_picker = TimePicker::new()?
            .minute_increment(15)?
            .clock_identifier("24HourClock")?
            .selected_time("17:04")?;
        assert_eq!(
            time_picker.get_selected_time()?.as_deref(),
            Some("17:04:00")
        );
        assert!(time_picker.set_selected_time("PT8H15M").is_err());
        time_picker.set_selected_time("")?;
        assert_eq!(time_picker.get_selected_time()?, None);
        time_picker.clear()?;

        // Wave C. Remaining layout panels: a WrapPanel carries spacing as doubles and
        // ItemsAlignment as a closed enum; RelativePanel's Align*WithPanel bools cross as
        // attached properties; object-valued Above/LeftOf stay gaps.
        let wrap = WrapPanel::new()?
            .orientation(Orientation::Horizontal)?
            .item_spacing(8.0)?
            .line_spacing(4.0)?
            .items_alignment(WrapPanelItemsAlignment::Center)?
            .item_width(80.0)?
            .child(TextBlock::new()?.text("Wrap")?)?;
        assert_eq!(wrap.get_item_spacing()?, 8.0);
        assert_eq!(wrap.get_items_alignment()?, WrapPanelItemsAlignment::Center);

        let uniform = UniformGrid::new()?
            .rows(2)?
            .columns(3)?
            .first_column(1)?
            .row_spacing(6.0)?
            .column_spacing(8.0)?;
        assert_eq!(uniform.get_rows()?, 2);
        assert_eq!(uniform.get_columns()?, 3);

        let relative_child = TextBlock::new()?.text("Pinned")?;
        RelativePanel::set_align_left_with_panel(&relative_child, true)?;
        RelativePanel::set_align_top_with_panel(&relative_child, true)?;
        assert!(RelativePanel::get_align_left_with_panel(&relative_child)?);
        let relative = RelativePanel::new()?.child(relative_child)?;

        let viewbox = Viewbox::new()?
            .stretch(Stretch::Uniform)?
            .stretch_direction(StretchDirection::Both)?
            .child(Some(&TextBlock::new()?.text("Scaled")?))?;
        assert_eq!(viewbox.get_stretch()?, Stretch::Uniform);
        assert!(viewbox.get_child()?.is_some());

        let flex = FlexPanel::new()?
            .direction(FlexDirection::Column)?
            .justify_content(FlexJustifyContent::Center)?
            .align_items(FlexAlignItems::FlexStart)?
            .wrap(FlexWrap::Wrap)?
            .column_spacing(10.0)?
            .row_spacing(6.0)?
            .child(TextBlock::new()?.text("Flex")?)?;
        assert_eq!(flex.get_direction()?, FlexDirection::Column);
        assert_eq!(flex.get_justify_content()?, FlexJustifyContent::Center);

        let splitter = GridSplitter::new()?
            .resize_direction(GridResizeDirection::Columns)?
            .resize_behavior(GridResizeBehavior::PreviousAndNext)?
            .shows_preview(true)?
            .keyboard_increment(20.0)?;
        assert_eq!(
            splitter.get_resize_direction()?,
            GridResizeDirection::Columns
        );
        assert!(splitter.get_shows_preview()?);

        let repeat = RepeatButton::new()?.delay(400)?.interval(50)?;
        assert_eq!(repeat.get_delay()?, 400);
        let hyperlink = HyperlinkButton::new()?
            .navigate_uri("https://avaloniaui.net")?
            .visited(true)?;
        assert_eq!(
            hyperlink.get_navigate_uri()?.as_deref(),
            Some("https://avaloniaui.net")
        );
        let toggle_split = ToggleSplitButton::new()?.checked(true)?;
        assert!(toggle_split.get_is_checked()?);
        let context_menu = ContextMenu::new()?
            .horizontal_offset(8.0)?
            .placement(PlacementMode::Bottom)?;
        assert_eq!(context_menu.get_horizontal_offset()?, 8.0);
        let menu_flyout = MenuFlyout::new()?.item(MenuItem::new()?)?;
        assert_eq!(menu_flyout.items()?.len()?, 1);
        let split = SplitButton::new()?;
        let drop_down = DropDownButton::new()?;

        let panel = StackPanel::new()?
            .orientation(Orientation::Vertical)?
            .spacing(8.0)?
            .background(Brush::solid(Color::rgb(0x1E, 0x1E, 0x1E)))?
            .child(TextBlock::new()?.text("Hello from Rust")?)?
            .child(text_box)?
            .child(toggle)?
            .child(combo_box)?
            .child(list_box)?
            .child(radio_buttons)?
            .child(expander)?
            .child(hover_panel)?
            .child(progress)?
            .child(laid_out)?
            .child(tracks)?
            .child(image)?
            .child(tabs)?
            .child(tree)?
            .child(menu)?
            .child(split_view)?
            .child(date_picker)?
            .child(time_picker)?
            .child(wrap)?
            .child(uniform)?
            .child(relative)?
            .child(viewbox)?
            .child(flex)?
            .child(splitter)?
            .child(repeat)?
            .child(hyperlink)?
            .child(toggle_split)?
            .child(context_menu)?
            .child(split)?
            .child(drop_down)?
            .child(button)?;
        assert_eq!(panel.children()?.len()?, 31);
        assert_eq!(panel.get_orientation()?, Orientation::Vertical);
        assert_eq!(panel.get_spacing()?, 8.0);
        assert_eq!(
            panel.get_background()?,
            Some(Brush::solid(Color::rgb(0x1E, 0x1E, 0x1E)))
        );

        let window = Window::new()?
            .title("Avalonia Rust")?
            .can_resize(false)?
            .margin(Thickness::uniform(0.0))?
            .content(Some(&panel))?;
        assert!(!window.get_can_resize()?);
        assert_eq!(window.get_window_state()?, WindowState::Normal);
        window.set_can_resize(true)?;
        assert!(window.get_can_resize()?);
        scope.mount(window.clone())?;
        called_from_handler.store(true, Ordering::SeqCst);

        let context = scope.clone();
        let delay = scope.delay(Duration::from_millis(10))?;
        std::thread::spawn(move || {
            assert!(!context.check_access().unwrap());
            block_on(delay).unwrap();
            context
                .post(move || {
                    text_box_for_post.set_text("after").unwrap();
                    assert_eq!(
                        text_box_for_post.get_text().unwrap().as_deref(),
                        Some("after")
                    );
                    toggle_for_post.set_checked(Some(false)).unwrap();
                    assert_eq!(toggle_for_post.get_is_checked().unwrap(), Some(false));
                    radio_two_for_post.set_checked(Some(true)).unwrap();
                    assert_eq!(radio_one_for_post.get_is_checked().unwrap(), Some(false));
                    assert_eq!(radio_two_for_post.get_is_checked().unwrap(), Some(true));
                    combo_box_for_post.set_selected_index(1).unwrap();
                    assert_eq!(combo_box_for_post.get_selected_index().unwrap(), 1);
                    // An editable ComboBox rewrites its text from the selection, so flip it
                    // after the selection assertions rather than in the builder chain.
                    combo_box_for_post.set_editable(true).unwrap();
                    assert!(combo_box_for_post.get_is_editable().unwrap());
                    list_box_for_post.set_selected_index(1).unwrap();
                    assert_eq!(list_box_for_post.get_selected_index().unwrap(), 1);
                    list_box_for_post.select_all().unwrap();
                    list_box_for_post.unselect_all().unwrap();
                    posted_from_handler.store(true, Ordering::SeqCst);
                    window.close().unwrap();
                })
                .unwrap();
        });

        Ok(())
    })
    .unwrap();

    assert!(called.load(Ordering::SeqCst));
    assert!(posted.load(Ordering::SeqCst));
    assert!(text_changed.load(Ordering::SeqCst));
    assert!(toggle_changed.load(Ordering::SeqCst));
    assert!(combo_changed.load(Ordering::SeqCst));
    assert!(list_changed.load(Ordering::SeqCst));
    assert!(tree_collapsed.load(Ordering::SeqCst));
    assert!(menu_opened.load(Ordering::SeqCst));
}

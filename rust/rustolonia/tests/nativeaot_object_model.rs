use rustolonia::{
    tracks, AnyEl, App, AutoCompleteBox, Border, Brush, Button, CivilDate, CivilTime, ClickMode,
    Color, ComboBox, ComboBoxItem, Command, ContextMenu, CornerRadius, DatePicker, Decimal, Dock,
    DockPanel, DropDownButton, ExpandDirection, Expander, Flyout, FlyoutShowMode, FontWeight,
    GradientStop, Grid, GridResizeBehavior, GridResizeDirection, GridSplitter, HorizontalAlignment,
    HyperlinkButton, Image, ItemFilter, ItemSelector, Key, LinearGradientBrush, ListBox,
    ListBoxItem, Menu, MenuFlyout, MenuItem, MenuItemToggleType, NumericUpDown, Orientation, Paint,
    PlacementMode, PopupAnchor, PopupGravity, PopupPlacementCallback, PopupPlacementOutput,
    PopupPositionerConstraintAdjustment, ProgressBar, RadialGradientBrush, RadioButton,
    RelativePanel, RelativePoint, RelativeScalar, RelativeUnit, RepeatButton, ScrollViewer,
    SelectingItemsControlSelectionChangedArgs, SelectionMode, Shortcut, SplitButton, SplitView,
    SplitViewDisplayMode, SplitViewPanePlacement, SpreadMethod, StackPanel, Stretch,
    StretchDirection, Style, StyleSheet, TabControl, TabItem, TextAlignment, TextBlock, TextBox,
    TextFilter, TextSelector, ThemeVariant, Thickness, TimePicker, ToggleSplitButton, ToggleSwitch,
    ToolTip, TreeView, TreeViewItem, UniformGrid, Variant, Vector, VerticalAlignment, Viewbox,
    Window, WindowState, WrapPanel, WrapPanelItemsAlignment,
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
    if let Ok(path) = std::env::var("RUSTOLONIA_HOST_LIB") {
        return PathBuf::from(path);
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    #[cfg(target_os = "windows")]
    let candidates = [
        "host/bin/Release/net10.0/win-x64/publish/rustolonia_host.dll",
        "host/bin/Release/net10.0/win-arm64/publish/rustolonia_host.dll",
        "host/bin/Debug/net10.0/win-x64/publish/rustolonia_host.dll",
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        "rust/target/dotnet-linux-x64/publish/Avalonia.Host/release_linux-x64/librustolonia_host.so",
        "rust/target/dotnet-linux-arm64/publish/Avalonia.Host/release_linux-arm64/librustolonia_host.so",
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
                 RUSTOLONIA_HOST_LIB."
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
        let button = Button::el()
            .content("Close")
            .background(Brush::solid(Color::rgb(0x00, 0x7A, 0xCC)))
            .foreground(Brush::solid(Color::rgb(0xFF, 0xFF, 0xFF)))
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(CornerRadius::uniform(3.0))
            .font_size(14.0)
            .click_mode(ClickMode::Press)
            .default(true)
            .cancel(false)
            .horizontal_content_alignment(HorizontalAlignment::Center)
            .vertical_content_alignment(VerticalAlignment::Center)
            .build(scope)?;
        let classes = button.classes()?;
        classes.add("primary")?;
        assert!(classes.contains("primary")?);
        assert!(classes.remove_value("primary")?);
        Grid::set_row(&button, 1)?;
        assert_eq!(Grid::row(&button)?, 1);
        button.subscribe_click(|_| {})?.unsubscribe()?;
        button.on_click(scope, |_| {})?;
        let btn_cmd = Command::new(|_| Ok(()));
        button.set_command(Some(btn_cmd))?;
        assert!(button.command()?.is_some());
        button.set_command(Option::<Command>::None)?;
        assert!(button.command()?.is_none());
        assert_eq!(button.font_size()?, 14.0);
        assert_eq!(button.click_mode()?, ClickMode::Press);
        assert!(button.is_default()?);
        assert!(!button.is_cancel()?);
        // IsPressed is raised by Avalonia's input handling, so it reads back but never sets.
        assert!(!button.is_pressed()?);
        assert_eq!(
            button.horizontal_content_alignment()?,
            HorizontalAlignment::Center
        );
        assert_eq!(
            button.vertical_content_alignment()?,
            VerticalAlignment::Center
        );
        assert_eq!(
            button.background()?,
            Some(Paint::Solid(Brush::solid(Color::rgb(0x00, 0x7A, 0xCC))))
        );
        assert_eq!(
            button.foreground()?,
            Some(Paint::Solid(Brush::solid(Color::rgb(0xFF, 0xFF, 0xFF))))
        );
        assert_eq!(button.corner_radius()?, CornerRadius::uniform(3.0));

        let text_box = TextBox::el()
            .text("before")
            .on_text_changed(move |_| {
                text_changed_from_handler.store(true, Ordering::SeqCst);
            })
            .build(scope)?;
        DockPanel::set_dock(&text_box, Dock::Top)?;
        assert_eq!(DockPanel::dock(&text_box)?, Dock::Top);
        let text_box_for_post = text_box.clone();
        let dock_panel = DockPanel::new()?;
        dock_panel.children()?.add(&text_box)?;
        let text_box = ScrollViewer::new()?;
        text_box.set_content(&dock_panel)?;

        let toggle = ToggleSwitch::el()
            .content("ToggleSwitch")
            .on_content("On")
            .off_content("Off")
            .checked(Some(true))
            .three_state(true)
            .on_is_checked_changed(move |_| {
                toggle_changed_from_handler.store(true, Ordering::SeqCst);
            })
            .build(scope)?;
        assert!(toggle.is_three_state()?);
        let toggle_for_post = toggle.clone();

        let radio_one = RadioButton::el()
            .group_name("TestGroup")
            .content("One")
            .checked(Some(true))
            .build(scope)?;
        let radio_two = RadioButton::el()
            .group_name("TestGroup")
            .content("Two")
            .build(scope)?;
        let radio_one_for_post = radio_one.clone();
        let radio_two_for_post = radio_two.clone();
        let radio_buttons = StackPanel::el()
            .child(AnyEl::from_control(radio_one))
            .child(AnyEl::from_control(radio_two))
            .build(scope)?;

        let expander = Expander::el()
            .header("Header")
            .content("Content")
            .expand_direction(ExpandDirection::Down)
            .expanded(true)
            .build(scope)?;
        assert_eq!(expander.expand_direction()?, ExpandDirection::Down);
        assert!(expander.is_expanded()?);
        expander.subscribe_expanded(|_| {})?.unsubscribe()?;

        let combo_box = ComboBox::el()
            .placeholder_text("Pick")
            .max_drop_down_height(240.0)
            .item(ComboBoxItem::el().content("First"))
            .item(ComboBoxItem::el().content("Second"))
            .on_selection_changed(move |_| {
                combo_changed_from_handler.store(true, Ordering::SeqCst);
            })
            .build(scope)?;
        assert_eq!(combo_box.items()?.len()?, 2);
        assert!(!combo_box.is_editable()?);
        assert!(!combo_box.is_drop_down_open()?);
        assert_eq!(combo_box.max_drop_down_height()?, 240.0);
        let combo_box_for_post = combo_box.clone();

        let list_box = ListBox::el()
            .selection_mode(SelectionMode::MULTIPLE)
            .item(ListBoxItem::el().content("First"))
            .item(ListBoxItem::el().content("Second"))
            .on_selection_changed(
                move |args: &mut SelectingItemsControlSelectionChangedArgs| {
                    let _ = (&args.added_items, &args.removed_items);
                    list_changed_from_handler.store(true, Ordering::SeqCst);
                },
            )
            .build(scope)?;
        assert_eq!(list_box.items()?.len()?, 2);
        assert_eq!(list_box.selection_mode()?, SelectionMode::MULTIPLE);
        let list_box_for_post = list_box.clone();

        let hover_panel = Border::el().child("Hover").build(scope)?;
        hover_panel
            .subscribe_pointer_entered(|_| {})?
            .unsubscribe()?;
        hover_panel
            .subscribe_pointer_exited(|_| {})?
            .unsubscribe()?;

        let progress = ProgressBar::el()
            .minimum(0.0)
            .maximum(100.0)
            .value(40.0)
            .show_progress_text(true)
            .progress_text_format("{0:0}%")
            .build(scope)?;
        assert_eq!(progress.value()?, 40.0);
        assert!(progress.show_progress_text()?);

        let text_readout = TextBlock::el()
            .text("Laid out from Rust")
            .name("layout_readout")
            .margin(Thickness::symmetric(12.0, 4.0))
            .horizontal_alignment(HorizontalAlignment::Right)
            .vertical_alignment(VerticalAlignment::Bottom)
            .min_width(64.0)
            .min_height(16.0)
            .max_width(320.0)
            .max_height(96.0)
            .opacity(0.75)
            .font_size(18.0)
            .font_weight(FontWeight::Bold)
            .text_alignment(TextAlignment::Center)
            .padding(Thickness::symmetric(3.0, 5.0))
            .foreground(Brush::solid(Color::rgb(0xFF, 0xFF, 0xFF)))
            .build(scope)?;
        assert_eq!(text_readout.font_size()?, 18.0);
        assert_eq!(text_readout.font_weight()?, FontWeight::Bold);
        assert_eq!(text_readout.text_alignment()?, TextAlignment::Center);
        assert_eq!(text_readout.padding()?, Thickness::new(3.0, 5.0, 3.0, 5.0));
        assert_eq!(
            text_readout.foreground()?,
            Some(Paint::Solid(Brush::solid(Color::rgb(0xFF, 0xFF, 0xFF))))
        );

        let laid_out = Border::el()
            .padding(Thickness::uniform(6.0))
            .background(Brush::solid(Color::rgb(0x33, 0x66, 0x99)))
            .border_brush(Brush::new(Color::rgb(0xAA, 0xBB, 0xCC), 0.5))
            .border_thickness(Thickness::uniform(2.0))
            .corner_radius(CornerRadius::uniform(4.0))
            .child(AnyEl::from_control(text_readout))
            .build(scope)?;
        assert_eq!(laid_out.padding()?, Thickness::uniform(6.0));
        assert_eq!(
            laid_out.background()?,
            Some(Paint::Solid(Brush::solid(Color::rgb(0x33, 0x66, 0x99))))
        );
        assert_eq!(
            laid_out.border_brush()?,
            Some(Paint::Solid(Brush::new(Color::rgb(0xAA, 0xBB, 0xCC), 0.5)))
        );
        assert_eq!(laid_out.border_thickness()?, Thickness::uniform(2.0));
        assert_eq!(laid_out.corner_radius()?, CornerRadius::uniform(4.0));

        // LinearGradientBrush and RadialGradientBrush round-trip directly on Border::background.
        let linear_bg = LinearGradientBrush {
            opacity: 0.85,
            spread_method: SpreadMethod::Reflect,
            stops: vec![
                GradientStop::new(0.0, Color::rgb(0xFF, 0x00, 0x00)),
                GradientStop::new(1.0, Color::rgb(0x00, 0x00, 0xFF)),
            ],
            start_point: RelativePoint::new(0.0, 0.0, RelativeUnit::Relative),
            end_point: RelativePoint::new(1.0, 1.0, RelativeUnit::Relative),
        };
        laid_out.set_background(linear_bg.clone())?;
        assert_eq!(
            laid_out.background()?,
            Some(Paint::LinearGradient(linear_bg))
        );

        let radial_bg = RadialGradientBrush {
            opacity: 0.75,
            spread_method: SpreadMethod::Repeat,
            stops: vec![
                GradientStop::new(0.0, Color::rgb(0x10, 0x20, 0x30)),
                GradientStop::new(1.0, Color::rgb(0xAA, 0xBB, 0xCC)),
            ],
            center: RelativePoint::new(0.5, 0.5, RelativeUnit::Relative),
            gradient_origin: RelativePoint::new(0.25, 0.25, RelativeUnit::Relative),
            radius_x: RelativeScalar::new(0.5, RelativeUnit::Relative),
            radius_y: RelativeScalar::new(0.5, RelativeUnit::Relative),
        };
        laid_out.set_background(radial_bg.clone())?;
        assert_eq!(
            laid_out.background()?,
            Some(Paint::RadialGradient(radial_bg))
        );
        laid_out.set_background(Brush::solid(Color::rgb(0x33, 0x66, 0x99)))?;

        // Clearing a brush is a first-class state, not a transparent colour.
        laid_out.set_border_brush(Option::<Paint>::None)?;
        assert_eq!(laid_out.border_brush()?, None);
        laid_out.set_border_brush(Brush::new(Color::rgb(0xAA, 0xBB, 0xCC), 0.5))?;

        let readout = laid_out.child()?.expect("border child");
        assert_eq!(readout.margin()?, Thickness::new(12.0, 4.0, 12.0, 4.0));
        assert_eq!(readout.horizontal_alignment()?, HorizontalAlignment::Right);
        assert_eq!(readout.vertical_alignment()?, VerticalAlignment::Bottom);
        assert_eq!(readout.min_width()?, 64.0);
        assert_eq!(readout.max_height()?, 96.0);
        assert_eq!(readout.opacity()?, 0.75);
        assert_eq!(readout.name()?.as_deref(), Some("layout_readout"));
        assert!(readout.is_visible()?);
        readout.set_visible(false)?;
        assert!(!readout.is_visible()?);
        readout.set_visible(true)?;

        // Grid tracks round-trip as typed `GridTracks`; Avalonia's `1*` normalises to `Star(1.0)`
        // (`"*"` when formatted back to text), and writing `GridTracks` back is a fixed point.
        let grid_tracks = Grid::el()
            .column_definitions("*,Auto,120")
            .row_definitions(tracks![Auto, 2*])
            .column_spacing(4.0)
            .row_spacing(2.0)
            .build(scope)?;
        assert_eq!(grid_tracks.column_definitions()?, tracks![*, Auto, 120]);
        assert_eq!(grid_tracks.column_definitions()?.to_string(), "*,Auto,120");
        assert_eq!(grid_tracks.row_definitions()?, tracks![Auto, 2*]);
        assert_eq!(grid_tracks.row_definitions()?.to_string(), "Auto,2*");
        grid_tracks.set_column_definitions(grid_tracks.column_definitions()?)?;
        assert_eq!(grid_tracks.column_definitions()?, tracks![*, Auto, 120]);

        // Clearing the tracks yields an empty `GridTracks`.
        grid_tracks.set_row_definitions("")?;
        assert!(grid_tracks.row_definitions()?.is_empty());
        grid_tracks.set_row_definitions(tracks![Auto, 2*])?;

        let cell = TextBlock::el().text("Cell").build(scope)?;
        Grid::set_column(&cell, 1)?;
        Grid::set_row(&cell, 1)?;
        assert_eq!(Grid::column(&cell)?, 1);
        assert_eq!(Grid::row(&cell)?, 1);
        grid_tracks.children()?.add(&cell)?;

        // Image.Source crosses as the source string the host resolves into a bitmap, so the
        // getter hands back the exact path that was set rather than the bitmap's type name.
        let image_path = one_pixel_png_path();
        let image_path_text = image_path.to_string_lossy().into_owned();
        let image = Image::el()
            .source(image_path_text.clone())
            .stretch(Stretch::UniformToFill)
            .stretch_direction(StretchDirection::DownOnly)
            .build(scope)?;
        assert_eq!(image.source()?.as_deref(), Some(image_path_text.as_str()));
        assert_eq!(image.stretch()?, Stretch::UniformToFill);
        assert_eq!(image.stretch_direction()?, StretchDirection::DownOnly);
        // Clearing the source is a first-class state, not an empty path.
        image.set_source("")?;
        assert_eq!(image.source()?, None);
        image.set_source(&image_path_text)?;

        // ToolTip.Tip is an attached property that carries text and nothing else.
        ToolTip::set_tip(&image, "One pixel, loaded from Rust")?;
        ToolTip::set_show_delay(&image, 250)?;
        ToolTip::set_placement(&image, PlacementMode::Top)?;
        assert_eq!(
            ToolTip::tip(&image)?.as_deref(),
            Some("One pixel, loaded from Rust")
        );
        assert_eq!(ToolTip::show_delay(&image)?, 250);
        assert_eq!(ToolTip::placement(&image)?, PlacementMode::Top);

        let tabs = TabControl::el()
            .tab_strip_placement(Dock::Top)
            .item(TabItem::el().header("First").content("First page"))
            .item(TabItem::el().header("Second").content("Second page"))
            .selected_index(1)
            .build(scope)?;
        assert_eq!(tabs.items()?.len()?, 2);
        assert_eq!(tabs.tab_strip_placement()?, Dock::Top);
        assert_eq!(tabs.selected_index()?, 1);

        // A TreeViewItem is an ItemsControl, so children go into the inherited Items slot and
        // Level is maintained by the control rather than written from Rust.
        let leaf = TreeViewItem::el().header("Leaf");
        let branch = TreeViewItem::el()
            .header("Branch")
            .item(leaf)
            .expanded(true)
            .on_collapsed(move |_| {
                tree_collapsed_from_handler.store(true, Ordering::SeqCst);
            })
            .build(scope)?;
        assert!(branch.is_expanded()?);
        assert_eq!(branch.level()?, 0);
        branch.subscribe_expanded(|_| {})?.unsubscribe()?;
        let tree = TreeView::el()
            .selection_mode(SelectionMode::SINGLE)
            .auto_scroll_to_selected_item(false)
            .item(AnyEl::from_control(branch.clone()))
            .build(scope)?;
        assert_eq!(tree.items()?.len()?, 1);
        assert_eq!(tree.selection_mode()?, SelectionMode::SINGLE);
        assert!(!tree.auto_scroll_to_selected_item()?);
        tree.subscribe_selection_changed(|_| {})?.unsubscribe()?;
        tree.collapse_sub_tree_with_tree_view_item(&branch)?;
        assert!(!branch.is_expanded()?);
        tree.unselect_all()?;

        // Wave B. A Menu is an imperative ItemsControl, not the view-model NativeMenu: it opens
        // and closes through methods and its items are real controls.
        let save_item = MenuItem::el()
            .header("Save")
            .icon(Image::el())
            .toggle_type(MenuItemToggleType::CheckBox)
            .checked(true)
            .group_name("edits")
            .build(scope)?;
        assert!(save_item.is_checked()?);
        assert_eq!(save_item.toggle_type()?, MenuItemToggleType::CheckBox);
        assert_eq!(save_item.group_name()?.as_deref(), Some("edits"));
        save_item.subscribe_click(|_| {})?.unsubscribe()?;
        let file_menu_item = MenuItem::el()
            .header("File")
            .item(AnyEl::from_control(save_item))
            .build(scope)?;
        assert_eq!(file_menu_item.items()?.len()?, 1);

        let menu = Menu::el()
            .item(AnyEl::from_control(file_menu_item))
            .on_opened(move |_| {
                menu_opened_from_handler.store(true, Ordering::SeqCst);
            })
            .build(scope)?;
        assert!(!menu.is_open()?);
        menu.subscribe_closed(|_| {})?.unsubscribe()?;
        menu.open()?;
        assert!(menu.is_open()?);
        menu.close()?;
        assert!(!menu.is_open()?);

        // A flyout is an AvaloniaObject rather than a Control, so it is not a child of anything;
        // it reaches a control through show_at instead of through an attached property.
        let placement_cb = PopupPlacementCallback::new(|input| PopupPlacementOutput {
            offset: Vector::new(0.0, input.anchor_rect.height),
            anchor: PopupAnchor::TOP_LEFT,
            gravity: PopupGravity::BOTTOM_RIGHT,
            constraint_adjustment: PopupPositionerConstraintAdjustment::FLIP_X,
        });
        let flyout = Flyout::el()
            .content("Flyout body")
            .placement(PlacementMode::BottomEdgeAlignedLeft)
            .show_mode(FlyoutShowMode::Transient)
            .horizontal_offset(6.0)
            .custom_popup_placement_callback(placement_cb)
            .build(scope)?;
        assert_eq!(flyout.placement()?, PlacementMode::BottomEdgeAlignedLeft);
        assert_eq!(flyout.show_mode()?, FlyoutShowMode::Transient);
        assert!(!flyout.is_open()?);
        assert!(flyout.target()?.is_none());
        assert!(flyout.custom_popup_placement_callback()?.is_some());
        flyout.subscribe_opened(|_| {})?.unsubscribe()?;
        flyout
            .subscribe_closing(|arguments| arguments.cancel = false)?
            .unsubscribe()?;
        // Hiding a flyout that was never shown is a no-op rather than an error.
        flyout.hide()?;

        let split_view = SplitView::el()
            .display_mode(SplitViewDisplayMode::CompactOverlay)
            .pane_placement(SplitViewPanePlacement::Left)
            .open_pane_length(220.0)
            .compact_pane_length(48.0)
            .pane(StackPanel::el().child("Pane"))
            .pane_background(Brush::solid(Color::rgb(0x22, 0x22, 0x22)))
            .content("Body")
            .build(scope)?;
        split_view.subscribe_pane_opened(|_| {})?.unsubscribe()?;
        split_view.set_pane_open(true)?;
        assert!(split_view.is_pane_open()?);
        assert_eq!(
            split_view.display_mode()?,
            SplitViewDisplayMode::CompactOverlay
        );
        assert_eq!(split_view.open_pane_length()?, 220.0);
        assert!(split_view.pane()?.is_some());
        assert_eq!(
            split_view.pane_background()?,
            Some(Paint::Solid(Brush::solid(Color::rgb(0x22, 0x22, 0x22))))
        );

        // DatePicker, TimePicker, and NumericUpDown use typed CivilDate, CivilTime, and Decimal values.
        let min_date = CivilDate::new(2000, 1, 1)?;
        let max_date = CivilDate::new(2100, 12, 31)?;
        let chosen_date = CivilDate::new(2027, 1, 15)?;
        let date_picker = DatePicker::el()
            .min_year(min_date)
            .max_year(max_date)
            .selected_date(chosen_date)
            .month_format("MMMM")
            .build(scope)?;
        assert_eq!(date_picker.selected_date()?, Some(chosen_date));
        assert_eq!(date_picker.min_year()?, min_date);
        assert_eq!(date_picker.max_year()?, max_date);
        assert!("03/09/2026".parse::<CivilDate>().is_err());
        date_picker.set_selected_date(None)?;
        assert_eq!(date_picker.selected_date()?, None);
        date_picker.clear()?;

        let chosen_time = CivilTime::hm(17, 4).unwrap();
        let time_picker = TimePicker::el()
            .minute_increment(15)
            .clock_identifier("24HourClock")
            .selected_time(chosen_time)
            .build(scope)?;
        assert_eq!(
            time_picker.selected_time()?,
            Some(CivilTime::hms(17, 4, 0).unwrap())
        );
        assert!("PT8H15M".parse::<CivilTime>().is_err());
        time_picker.set_selected_time(None)?;
        assert_eq!(time_picker.selected_time()?, None);
        time_picker.clear()?;

        let numeric = NumericUpDown::el()
            .minimum(0)
            .maximum(100)
            .increment(Decimal::new(25, 1)?)
            .value(Decimal::new(125, 1)?)
            .build(scope)?;
        assert_eq!(numeric.minimum()?, Decimal::from(0));
        assert_eq!(numeric.maximum()?, Decimal::from(100));
        assert_eq!(numeric.increment()?, Decimal::new(25, 1)?);
        assert_eq!(numeric.value()?, Some(Decimal::new(125, 1)?));
        numeric.set_value(Option::<Decimal>::None)?;
        assert_eq!(numeric.value()?, None);

        let auto_complete = AutoCompleteBox::el()
            .placeholder_text("Search")
            .item_filter(ItemFilter::new(
                |search, item| matches!(item, Variant::Utf16(s) if s.contains(search)),
            ))
            .text_filter(TextFilter::new(|search, text| text.contains(search)))
            .item_selector(ItemSelector::new(|_, item| format!("{item:?}")))
            .text_selector(TextSelector::new(|_, text| text.to_owned()))
            .build(scope)?;
        assert!(auto_complete.item_filter()?.is_some());
        assert!(auto_complete.text_filter()?.is_some());
        assert!(auto_complete.item_selector()?.is_some());
        assert!(auto_complete.text_selector()?.is_some());

        // Wave C. Remaining layout panels: a WrapPanel carries spacing as doubles and
        // ItemsAlignment as a closed enum; RelativePanel's Align*WithPanel bools cross as
        // attached properties.
        let wrap = WrapPanel::el()
            .orientation(Orientation::Horizontal)
            .item_spacing(8.0)
            .line_spacing(4.0)
            .items_alignment(WrapPanelItemsAlignment::Center)
            .item_width(80.0)
            .child("Wrap")
            .build(scope)?;
        assert_eq!(wrap.item_spacing()?, 8.0);
        assert_eq!(wrap.items_alignment()?, WrapPanelItemsAlignment::Center);

        let uniform = UniformGrid::el()
            .rows(2)
            .columns(3)
            .first_column(1)
            .row_spacing(6.0)
            .column_spacing(8.0)
            .build(scope)?;
        assert_eq!(uniform.rows()?, 2);
        assert_eq!(uniform.columns()?, 3);

        let relative_child = TextBlock::el().text("Pinned").build(scope)?;
        RelativePanel::set_align_left_with_panel(&relative_child, true)?;
        RelativePanel::set_align_top_with_panel(&relative_child, true)?;
        assert!(RelativePanel::align_left_with_panel(&relative_child)?);
        let relative = RelativePanel::el()
            .child(AnyEl::from_control(relative_child))
            .build(scope)?;

        let viewbox = Viewbox::el()
            .stretch(Stretch::Uniform)
            .stretch_direction(StretchDirection::Both)
            .child("Scaled")
            .build(scope)?;
        assert_eq!(viewbox.stretch()?, Stretch::Uniform);
        assert!(viewbox.child()?.is_some());

        let splitter = GridSplitter::el()
            .resize_direction(GridResizeDirection::Columns)
            .resize_behavior(GridResizeBehavior::PreviousAndNext)
            .shows_preview(true)
            .keyboard_increment(20.0)
            .build(scope)?;
        assert_eq!(splitter.resize_direction()?, GridResizeDirection::Columns);
        assert!(splitter.shows_preview()?);

        let repeat = RepeatButton::el().delay(400).interval(50).build(scope)?;
        assert_eq!(repeat.delay()?, 400);
        let hyperlink = HyperlinkButton::el()
            .navigate_uri("https://avaloniaui.net")
            .visited(true)
            .build(scope)?;
        assert_eq!(
            hyperlink.navigate_uri()?.as_deref(),
            Some("https://avaloniaui.net")
        );
        let toggle_split = ToggleSplitButton::el().checked(true).build(scope)?;
        assert!(toggle_split.is_checked()?);
        let context_menu = ContextMenu::el()
            .horizontal_offset(8.0)
            .placement(PlacementMode::Bottom)
            .build(scope)?;
        assert_eq!(context_menu.horizontal_offset()?, 8.0);
        let menu_flyout = MenuFlyout::el().item(MenuItem::el()).build(scope)?;
        assert_eq!(menu_flyout.items()?.len()?, 1);
        let split = SplitButton::new()?;
        let drop_down = DropDownButton::new()?;

        let panel = StackPanel::el()
            .orientation(Orientation::Vertical)
            .spacing(8.0)
            .background(Brush::solid(Color::rgb(0x1E, 0x1E, 0x1E)))
            .child("Hello from Rust")
            .child(AnyEl::from_control(text_box))
            .child(AnyEl::from_control(toggle))
            .child(AnyEl::from_control(combo_box))
            .child(AnyEl::from_control(list_box))
            .child(AnyEl::from_control(radio_buttons))
            .child(AnyEl::from_control(expander))
            .child(AnyEl::from_control(hover_panel))
            .child(AnyEl::from_control(progress))
            .child(AnyEl::from_control(laid_out))
            .child(AnyEl::from_control(grid_tracks))
            .child(AnyEl::from_control(image))
            .child(AnyEl::from_control(tabs))
            .child(AnyEl::from_control(tree))
            .child(AnyEl::from_control(menu))
            .child(AnyEl::from_control(split_view))
            .child(AnyEl::from_control(date_picker))
            .child(AnyEl::from_control(time_picker))
            .child(AnyEl::from_control(numeric))
            .child(AnyEl::from_control(auto_complete))
            .child(AnyEl::from_control(wrap))
            .child(AnyEl::from_control(uniform))
            .child(AnyEl::from_control(relative))
            .child(AnyEl::from_control(viewbox))
            .child(AnyEl::from_control(splitter))
            .child(AnyEl::from_control(repeat))
            .child(AnyEl::from_control(hyperlink))
            .child(AnyEl::from_control(toggle_split))
            .child(AnyEl::from_control(context_menu))
            .child(AnyEl::from_control(split))
            .child(AnyEl::from_control(drop_down))
            .child(AnyEl::from_control(button))
            .build(scope)?;
        assert_eq!(panel.children()?.len()?, 32);

        // ControlList::insert places a control at an exact index and shifts subsequent children.
        let inserted_banner = TextBlock::el().text("Inserted banner").build(scope)?;
        panel.children()?.insert(1, &inserted_banner)?;
        assert_eq!(panel.children()?.len()?, 33);
        assert_eq!(panel.children()?.index_of(&inserted_banner)?, Some(1));
        assert!(panel.children()?.get(1)?.same_object(&inserted_banner)?);
        panel.children()?.remove(1)?;
        assert_eq!(panel.children()?.len()?, 32);

        // Scoped control styles register on the control's Styles collection.
        let style_handle = scope.apply_control_styles(
            &panel,
            StyleSheet::new().rule(
                Style::select("Button.primary:pointerover")
                    .background(Color::rgb(0x10, 0x6E, 0xBE))
                    .corner_radius(4.0),
            ),
        )?;
        assert!(style_handle > 0);

        assert_eq!(panel.orientation()?, Orientation::Vertical);
        assert_eq!(panel.spacing()?, 8.0);
        assert_eq!(
            panel.background()?,
            Some(Paint::Solid(Brush::solid(Color::rgb(0x1E, 0x1E, 0x1E))))
        );

        let window = Window::new()?;
        window.set_title("Avalonia Rust")?;
        window.set_can_resize(false)?;
        window.set_margin(Thickness::uniform(0.0))?;
        window.set_content(&panel)?;
        assert!(!window.can_resize()?);
        assert_eq!(window.window_state()?, WindowState::Normal);
        window.set_can_resize(true)?;
        assert!(window.can_resize()?);

        let shortcut_id = scope.bind_window_shortcut(
            &window,
            Shortcut::primary(Key::S),
            Command::new(|_| Ok(())),
            "save",
        )?;
        assert!(shortcut_id > 0);

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
                    assert_eq!(text_box_for_post.text().unwrap().as_deref(), Some("after"));
                    toggle_for_post.set_checked(Some(false)).unwrap();
                    assert_eq!(toggle_for_post.is_checked().unwrap(), Some(false));
                    radio_two_for_post.set_checked(Some(true)).unwrap();
                    assert_eq!(radio_one_for_post.is_checked().unwrap(), Some(false));
                    assert_eq!(radio_two_for_post.is_checked().unwrap(), Some(true));
                    combo_box_for_post.set_selected_index(1).unwrap();
                    assert_eq!(combo_box_for_post.selected_index().unwrap(), 1);
                    // An editable ComboBox rewrites its text from the selection, so flip it
                    // after the selection assertions rather than in the builder chain.
                    combo_box_for_post.set_editable(true).unwrap();
                    assert!(combo_box_for_post.is_editable().unwrap());
                    list_box_for_post.set_selected_index(1).unwrap();
                    assert_eq!(list_box_for_post.selected_index().unwrap(), 1);
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

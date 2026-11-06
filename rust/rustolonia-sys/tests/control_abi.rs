//! ABI guarantees for the control-completeness members.
//!
//! `ContentControl`, `Button`, `ToggleButton`, `ListBox` and `ComboBox` each grew slots in
//! place, and nano-COM vtables are flattened, so this pins the generated header slots every
//! interface at or below one of them now carries.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn completeness_members_are_published_on_the_type_that_declares_them() {
    for expected in [
        "*get_horizontal_content_alignment)(IAvnContentControl* self, int32_t* value)",
        "*set_horizontal_content_alignment)(IAvnContentControl* self, int32_t value)",
        "*set_vertical_content_alignment)(IAvnContentControl* self, int32_t value)",
        "*get_click_mode)(IAvnButton* self, int32_t* value)",
        "*set_click_mode)(IAvnButton* self, int32_t value)",
        "*set_is_default)(IAvnButton* self, int32_t value)",
        "*set_is_cancel)(IAvnButton* self, int32_t value)",
        "*set_command)(IAvnButton* self, IAvnCommand* value)",
        "*get_is_pressed)(IAvnButton* self, int32_t* value)",
        "*set_is_three_state)(IAvnToggleButton* self, int32_t value)",
        "*set_selection_mode)(IAvnListBox* self, int32_t value)",
        "*select_all)(IAvnListBox* self)",
        "*unselect_all)(IAvnListBox* self)",
        "*set_is_drop_down_open)(IAvnComboBox* self, int32_t value)",
        "*set_is_editable)(IAvnComboBox* self, int32_t value)",
        "*set_max_drop_down_height)(IAvnComboBox* self, double value)",
        "*set_size_to_content)(IAvnWindow* self, int32_t value)",
        "*set_show_activated)(IAvnWindow* self, int32_t value)",
        "*set_show_in_taskbar)(IAvnWindow* self, int32_t value)",
        "*set_can_minimize)(IAvnWindow* self, int32_t value)",
        "*set_can_maximize)(IAvnWindow* self, int32_t value)",
        "*set_window_startup_location)(IAvnWindow* self, int32_t value)",
        "*set_window_decorations)(IAvnWindow* self, int32_t value)",
        "*set_closing_behavior)(IAvnWindow* self, int32_t value)",
        "*hide)(IAvnWindow* self)",
        "*advise_closing)(IAvnWindow* self, IAvnWindowClosingHandler* handler, int64_t* subscription_id)",
        "*set_font_family)(IAvnTemplatedControl* self, const uint16_t* value)",
        "*set_font_style)(IAvnTemplatedControl* self, int32_t value)",
        "*set_font_weight)(IAvnTemplatedControl* self, int32_t value)",
        "*set_padding)(IAvnTemplatedControl* self, AvnThickness value)",
        "*set_font_family)(IAvnTextBlock* self, const uint16_t* value)",
        "*set_text_wrapping)(IAvnTextBlock* self, int32_t value)",
        "*set_text_trimming)(IAvnTextBlock* self, const uint16_t* value)",
        "*set_selected_text)(IAvnTextBox* self, const uint16_t* value)",
        "*select_all)(IAvnTextBox* self)",
        "*clear_selection)(IAvnTextBox* self)",
        "*set_inner_left_content)(IAvnTextBox* self, IAvnControl* value)",
        "*set_line_height)(IAvnTextBlock* self, double value)",
        "*set_baseline_offset)(IAvnTextBlock* self, double value)",
        "*scroll_to_line_with_int32)(IAvnTextBox* self, int32_t line_index)",
        "*select_all)(IAvnSelectableTextBlock* self)",
        "*set_selection_brush)(IAvnSelectableTextBlock* self, IAvnBrush* value)",
        "*next)(IAvnCarousel* self)",
        "*previous)(IAvnCarousel* self)",
        "*get_percentage)(IAvnProgressBar* self, double* value)",
        "*get_clip_to_bounds_radius)(IAvnBorder* self, AvnCornerRadius* value)",
        "*request_refresh)(IAvnRefreshContainer* self)",
        "*set_context_menu)(IAvnControl* self, IAvnContextMenu* value)",
        "*set_context_flyout)(IAvnControl* self, IAvnFlyoutBase* value)",
        "*get_is_loaded)(IAvnControl* self, int32_t* value)",
        "*get_line_count)(IAvnTextBox* self, int32_t* value)",
        "*set_password_char)(IAvnTextBox* self, uint16_t value)",
        "*set_flyout)(IAvnButton* self, IAvnFlyoutBase* value)",
        "*set_flyout)(IAvnSplitButton* self, IAvnFlyoutBase* value)",
        "*open)(IAvnMenuItem* self)",
        "*close)(IAvnMenuItem* self)",
        "*get_item_count)(IAvnItemsControl* self, int32_t* value)",
        "*scroll_into_view_with_int32)(IAvnItemsControl* self, int32_t index)",
        "*set_auto_scroll_to_selected_item)(IAvnSelectingItemsControl* self, int32_t value)",
        "*set_is_text_search_enabled)(IAvnSelectingItemsControl* self, int32_t value)",
        "*set_wrap_selection)(IAvnSelectingItemsControl* self, int32_t value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    // IsPressed is raised by Avalonia's own input handling, so the ABI publishes no setter.
    assert!(
        !HEADER.contains("*set_is_pressed)"),
        "IsPressed is read-only and must not publish a setter"
    );
}

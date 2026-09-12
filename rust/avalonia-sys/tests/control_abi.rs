//! ABI guarantees for the control-completeness members.
//!
//! `ContentControl`, `Button`, `ToggleButton`, `ListBox` and `ComboBox` each grew slots, and
//! nano-COM vtables are flattened, so every interface at or below one of them republishes
//! under a version 5 IID. This pins the generated header, the fresh IIDs, and the retired
//! ones that must never come back.

use avalonia_sys::{
    I_AVN_AVALONIA_OBJECT_IID, I_AVN_BORDER_IID, I_AVN_BUTTON_IID, I_AVN_COMBO_BOX_IID,
    I_AVN_CONTENT_CONTROL_IID, I_AVN_CONTROL_IID, I_AVN_LIST_BOX_IID, I_AVN_PANEL_IID,
    I_AVN_TEMPLATED_CONTROL_IID, I_AVN_TEXT_BLOCK_IID, I_AVN_TOGGLE_BUTTON_IID, I_AVN_WINDOW_IID,
};

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

#[test]
fn widened_interfaces_publish_abi_version_five() {
    for expected in [
        "#define I_AVN_CONTENT_CONTROL_ABI_VERSION 16",
        "#define I_AVN_HEADERED_CONTENT_CONTROL_ABI_VERSION 16",
        "#define I_AVN_EXPANDER_ABI_VERSION 16",
        "#define I_AVN_BUTTON_ABI_VERSION 19",
        "#define I_AVN_TOGGLE_BUTTON_ABI_VERSION 19",
        "#define I_AVN_CHECK_BOX_ABI_VERSION 19",
        "#define I_AVN_RADIO_BUTTON_ABI_VERSION 19",
        "#define I_AVN_TOGGLE_SWITCH_ABI_VERSION 19",
        "#define I_AVN_LIST_BOX_ABI_VERSION 19",
        "#define I_AVN_LIST_BOX_ITEM_ABI_VERSION 18",
        "#define I_AVN_COMBO_BOX_ABI_VERSION 20",
        "#define I_AVN_COMBO_BOX_ITEM_ABI_VERSION 18",
        "#define I_AVN_SCROLL_VIEWER_ABI_VERSION 18",
        "#define I_AVN_WINDOW_ABI_VERSION 21",
        "#define I_AVN_TEMPLATED_CONTROL_ABI_VERSION 15",
        "#define I_AVN_TEXT_BLOCK_ABI_VERSION 17",
        "#define I_AVN_TEXT_BOX_ABI_VERSION 21",
        "#define I_AVN_ITEMS_CONTROL_ABI_VERSION 18",
        "#define I_AVN_SELECTING_ITEMS_CONTROL_ABI_VERSION 18",
        "#define I_AVN_BORDER_ABI_VERSION 15",
        "#define I_AVN_PANEL_ABI_VERSION 14",
        "#define I_AVN_CONTROL_ABI_VERSION 11",
        "#define I_AVN_AVALONIA_OBJECT_ABI_VERSION 2",
        // The factory gained a creator per wave A control plus get_tool_tip_statics at version
        // 3, and a creator per constructible wave B type at version 4.
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 14",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn widened_interfaces_republish_under_fresh_iids() {
    // The version 4 IIDs are retired: reusing one for a longer vtable would let a stale
    // consumer call through slots the old contract never declared.
    for (name, retired, current) in [
        (
            "IAvnContentControl",
            "341FD6C2-31EA-572B-B53F-A8CFB57B3BF0",
            I_AVN_CONTENT_CONTROL_IID,
        ),
        (
            "IAvnButton",
            "9DE6B725-9BF0-5DEE-A947-DD87709DD52E",
            I_AVN_BUTTON_IID,
        ),
        (
            "IAvnToggleButton",
            "A68058CE-1067-5C24-A3C4-470105EBF433",
            I_AVN_TOGGLE_BUTTON_IID,
        ),
        (
            "IAvnListBox",
            "97204BC7-2150-5475-B853-9241B9B03781",
            I_AVN_LIST_BOX_IID,
        ),
        (
            "IAvnComboBox",
            "4EEFC042-03EE-5E79-AF4B-3A53F50C63B4",
            I_AVN_COMBO_BOX_IID,
        ),
        (
            "IAvnWindow",
            "0A480A4D-6DF1-5762-8661-F83519B0CC38",
            I_AVN_WINDOW_IID,
        ),
        (
            "IAvnWindow",
            "965CC1CE-DA6F-5CCB-900F-3150CA8DB605",
            I_AVN_WINDOW_IID,
        ),
        (
            "IAvnWindow",
            "F01ADFEE-98B3-5F6C-85B3-61121B1F7106",
            I_AVN_WINDOW_IID,
        ),
        (
            "IAvnWindow",
            "4E237A63-4083-5704-8B9E-1C6CAFC4172A",
            I_AVN_WINDOW_IID,
        ),
        (
            "IAvnContentControl",
            "2C4557A2-537C-5683-9E30-C3AE87D7614C",
            I_AVN_CONTENT_CONTROL_IID,
        ),
        (
            "IAvnButton",
            "6D86D2DB-4473-576B-8778-47C74AAF182D",
            I_AVN_BUTTON_IID,
        ),
        (
            "IAvnButton",
            "6145E298-C66D-5A68-8AD1-F829C9773DB5",
            I_AVN_BUTTON_IID,
        ),
        (
            "IAvnToggleButton",
            "ED70AAF8-5651-5844-BA09-078F9EB9B7D1",
            I_AVN_TOGGLE_BUTTON_IID,
        ),
        (
            "IAvnToggleButton",
            "587791B4-65DA-5D37-9E5B-C03B93115683",
            I_AVN_TOGGLE_BUTTON_IID,
        ),
        (
            "IAvnListBox",
            "EAD15413-53EB-5159-BE99-7BED7BF25651",
            I_AVN_LIST_BOX_IID,
        ),
        (
            "IAvnComboBox",
            "7334041F-D155-548C-BF70-4CFFB4F44021",
            I_AVN_COMBO_BOX_IID,
        ),
        (
            "IAvnComboBox",
            "1BFD4CC7-0C79-53D7-845D-CC6801697EAD",
            I_AVN_COMBO_BOX_IID,
        ),
        (
            "IAvnListBox",
            "887C3BB0-59E1-57DD-848F-366B137BA3D1",
            I_AVN_LIST_BOX_IID,
        ),
        (
            "IAvnComboBox",
            "ACF92675-F4CB-553B-9700-58360FEE0232",
            I_AVN_COMBO_BOX_IID,
        ),
        (
            "IAvnButton",
            "A47AB795-2D16-5F5D-914C-D9641520B7E9",
            I_AVN_BUTTON_IID,
        ),
        (
            "IAvnToggleButton",
            "4B6CC48D-00FF-569F-A965-FE8C6B86C502",
            I_AVN_TOGGLE_BUTTON_IID,
        ),
    ] {
        assert_ne!(format_iid(&current), retired, "{name} reused a retired IID");
    }

    // Wave M republished every TemplatedControl descendant under a fresh IID.
    for (name, expected, current) in [
        (
            "IAvnContentControl",
            "7312A513-1E46-5A98-9507-851220C7A465",
            I_AVN_CONTENT_CONTROL_IID,
        ),
        (
            "IAvnButton",
            "085861C8-F91D-5AFE-B57A-E601477FC2D2",
            I_AVN_BUTTON_IID,
        ),
        (
            "IAvnToggleButton",
            "AA447DC6-1470-599E-B0F5-27BF313DC7E7",
            I_AVN_TOGGLE_BUTTON_IID,
        ),
        (
            "IAvnListBox",
            "FD704B9F-4E33-516E-B48D-B1DC9AA44DCA",
            I_AVN_LIST_BOX_IID,
        ),
        (
            "IAvnComboBox",
            "1DA7FC0F-6D3F-5669-8644-030686AD4FD3",
            I_AVN_COMBO_BOX_IID,
        ),
        (
            "IAvnWindow",
            "9A01C107-BB12-5AFE-9DAA-C5AACEF2D879",
            I_AVN_WINDOW_IID,
        ),
    ] {
        assert_eq!(format_iid(&current), expected, "{name} changed its IID");
    }

    // Nothing that kept its vtable may change identity.
    assert_eq!(
        format_iid(&I_AVN_AVALONIA_OBJECT_IID),
        "FA7F2E03-0BFA-5422-840B-18AE1D9695C0"
    );
    assert_eq!(
        format_iid(&I_AVN_CONTROL_IID),
        "8B272500-09E4-535F-ABF1-14992E90E778"
    );
    assert_eq!(
        format_iid(&I_AVN_BORDER_IID),
        "430F4A36-8B46-5F3C-8E4D-BD11AA522969"
    );
    assert_eq!(
        format_iid(&I_AVN_PANEL_IID),
        "FE37643D-BDE5-511B-AB66-CB66B73F685F"
    );
    assert_eq!(
        format_iid(&I_AVN_TEMPLATED_CONTROL_IID),
        "1863F34C-F21F-5977-A305-5939D80CB8C5"
    );
    assert_eq!(
        format_iid(&I_AVN_TEXT_BLOCK_IID),
        "47E06BD1-10DC-5267-A55F-A5C3BB536285"
    );
}

fn format_iid(iid: &avalonia_sys::Guid) -> String {
    let mut out = format!("{:08X}-{:04X}-{:04X}-", iid.data1, iid.data2, iid.data3);
    for (index, byte) in iid.data4.iter().enumerate() {
        if index == 2 {
            out.push('-');
        }
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

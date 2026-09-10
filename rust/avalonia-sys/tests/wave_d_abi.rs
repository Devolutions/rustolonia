//! ABI guarantees for the wave D button family, `ContextMenu` and `MenuFlyout`.
//!
//! Wave D only *adds* interfaces. The factory gains a creator per constructible type and
//! moves from 5 to 6.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn repeat_button_and_hyperlink_are_buttons() {
    for expected in [
        "*set_delay)(IAvnRepeatButton* self, int32_t value)",
        "*set_interval)(IAvnRepeatButton* self, int32_t value)",
        "*set_is_visited)(IAvnHyperlinkButton* self, int32_t value)",
        "*get_navigate_uri)(IAvnHyperlinkButton* self, uint16_t** value)",
        "*set_navigate_uri)(IAvnHyperlinkButton* self, const uint16_t* value)",
        "#define I_AVN_REPEAT_BUTTON_ABI_VERSION 12",
        "#define I_AVN_HYPERLINK_BUTTON_ABI_VERSION 12",
        "#define I_AVN_DROP_DOWN_BUTTON_ABI_VERSION 12",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn split_button_carries_click_and_command() {
    for expected in [
        "*advise_click)(IAvnSplitButton* self, IAvnSplitButtonClickHandler* handler, int64_t* subscription_id)",
        "*set_is_checked)(IAvnToggleSplitButton* self, int32_t value)",
        "*set_command)(IAvnSplitButton* self, IAvnCommand* value)",
        "*set_command_parameter)(IAvnSplitButton* self, AvnVariant value)",
        "*set_command)(IAvnToggleSplitButton* self, IAvnCommand* value)",
        "*set_command_parameter)(IAvnToggleSplitButton* self, AvnVariant value)",
        "*create_split_button)(IAvnControlFactory* self, IAvnSplitButton** value)",
        "*create_toggle_split_button)(IAvnControlFactory* self, IAvnToggleSplitButton** value)",
        "*create_context_menu)(IAvnControlFactory* self, IAvnContextMenu** value)",
        "*create_menu_flyout)(IAvnControlFactory* self, IAvnMenuFlyout** value)",
        "#define I_AVN_SPLIT_BUTTON_ABI_VERSION 13",
        "#define I_AVN_TOGGLE_SPLIT_BUTTON_ABI_VERSION 13",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn menu_flyout_items_use_the_existing_item_list() {
    for expected in [
        "*get_items)(IAvnMenuFlyout* self, IAvnItemList** value)",
        "*set_placement)(IAvnContextMenu* self, int32_t value)",
        "*set_horizontal_offset)(IAvnContextMenu* self, double value)",
        "#define I_AVN_MENU_FLYOUT_ABI_VERSION 5",
        "#define I_AVN_CONTEXT_MENU_ABI_VERSION 16",
        "#define I_AVN_BUTTON_ABI_VERSION 17",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

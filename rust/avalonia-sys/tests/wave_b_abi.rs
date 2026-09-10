//! ABI guarantees for the wave B controls: the flyout trio, the imperative menu pair,
//! `SplitView`, `HeaderedSelectingItemsControl` and the two date/time pickers.
//!
//! Like wave A, wave B only *adds* interfaces. A flyout is an `AvaloniaObject` rather than a
//! `Control`, so `IAvnFlyoutBase` hangs directly off `IAvnAvaloniaObject` and inserts no slot
//! into anything that shipped; every interface published before keeps the exact IID it last
//! published, and the ten new ones publish at version 1. The one thing that moves is the
//! factory, which gains a creator per constructible wave B type and goes from 3 to 4.

use avalonia_sys::{
    I_AVN_AVALONIA_OBJECT_IID, I_AVN_CONTENT_CONTROL_IID, I_AVN_CONTROL_IID, I_AVN_DATE_PICKER_IID,
    I_AVN_FLYOUT_BASE_IID, I_AVN_FLYOUT_IID, I_AVN_HEADERED_ITEMS_CONTROL_IID,
    I_AVN_HEADERED_SELECTING_ITEMS_CONTROL_IID, I_AVN_ITEMS_CONTROL_IID, I_AVN_MENU_BASE_IID,
    I_AVN_MENU_IID, I_AVN_MENU_ITEM_IID, I_AVN_POPUP_FLYOUT_BASE_IID,
    I_AVN_SELECTING_ITEMS_CONTROL_IID, I_AVN_SPLIT_VIEW_IID, I_AVN_TEMPLATED_CONTROL_IID,
    I_AVN_TIME_PICKER_IID, I_AVN_TREE_VIEW_IID,
};

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn a_flyout_is_shown_through_a_method_rather_than_an_attached_property() {
    // The attached-property pipeline carries scalars and strings only, so there is no
    // `set_attached_flyout` and no `Button.flyout`. What crosses instead is `show_at`, which
    // takes any projected control and unwraps it back to the Avalonia object.
    for expected in [
        "*show_at_with_control)(IAvnFlyoutBase* self, IAvnControl* placement_target)",
        "*hide)(IAvnFlyoutBase* self)",
        "*get_target)(IAvnFlyoutBase* self, IAvnControl** value)",
        "*set_is_open)(IAvnFlyoutBase* self, int32_t value)",
        "*set_placement)(IAvnPopupFlyoutBase* self, int32_t value)",
        "*set_placement_anchor)(IAvnPopupFlyoutBase* self, int32_t value)",
        "*set_show_mode)(IAvnPopupFlyoutBase* self, int32_t value)",
        "*set_content)(IAvnFlyout* self, IAvnControl* value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    for forbidden in [
        "set_attached_flyout",
        "get_attached_flyout",
        "IAvnFlyoutBaseStatics",
    ] {
        assert!(
            !HEADER.contains(forbidden),
            "header must not declare `{forbidden}`"
        );
    }
}

#[test]
fn a_closing_flyout_carries_a_writable_cancel_field() {
    // The only wave B event with a payload. Cancel is in/out, exactly like KeyDown's Handled,
    // so a handler vetoes the close by writing it back rather than by returning a magic HRESULT.
    for expected in [
        "*invoke)(IAvnPopupFlyoutBaseClosingHandler* self, int32_t* cancel)",
        "*advise_closing)(IAvnPopupFlyoutBase* self, IAvnPopupFlyoutBaseClosingHandler* handler, int64_t* subscription_id)",
        "*advise_opening)(IAvnPopupFlyoutBase* self, IAvnPopupFlyoutBaseOpeningHandler* handler, int64_t* subscription_id)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn the_menu_pair_is_imperative_and_its_commands_cross() {
    for expected in [
        "*open)(IAvnMenuBase* self)",
        "*close)(IAvnMenuBase* self)",
        "*get_is_open)(IAvnMenuBase* self, int32_t* value)",
        "*set_toggle_type)(IAvnMenuItem* self, int32_t value)",
        "*set_is_checked)(IAvnMenuItem* self, int32_t value)",
        "*set_icon)(IAvnMenuItem* self, IAvnControl* value)",
        "*set_header)(IAvnHeaderedSelectingItemsControl* self, IAvnControl* value)",
        "*advise_click)(IAvnMenuItem* self, IAvnMenuItemClickHandler* handler, int64_t* subscription_id)",
        "*set_command)(IAvnMenuItem* self, IAvnCommand* value)",
        "*set_command_parameter)(IAvnMenuItem* self, AvnVariant value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    // MenuBase.IsOpen is read-only managed-side, so the ABI publishes no setter for it.
    assert!(
        !HEADER.contains("*set_is_open)(IAvnMenuBase* self"),
        "IAvnMenuBase must not publish a set_is_open"
    );
    // U27 projects the gestures as strings through KeyGesture's own round-trip, so the
    // setters exist; what stays out is the AvaloniaProperty plumbing.
    for expected in [
        "*set_hot_key)(IAvnMenuItem* self, const uint16_t* value)",
        "*set_input_gesture)(IAvnMenuItem* self, const uint16_t* value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn split_view_carries_its_pane_as_a_control_and_its_background_as_a_brush() {
    for expected in [
        "*set_is_pane_open)(IAvnSplitView* self, int32_t value)",
        "*set_display_mode)(IAvnSplitView* self, int32_t value)",
        "*set_pane_placement)(IAvnSplitView* self, int32_t value)",
        "*set_open_pane_length)(IAvnSplitView* self, double value)",
        "*set_compact_pane_length)(IAvnSplitView* self, double value)",
        "*set_pane)(IAvnSplitView* self, IAvnControl* value)",
        "*set_pane_background)(IAvnSplitView* self, IAvnBrush* value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn the_pickers_carry_dates_and_times_as_utf16_strings() {
    // `DateTimeOffset` and `TimeSpan` have no ABI shape in this projection, so they cross as
    // ISO-8601 text through a host-side converter — the same mechanism `Image.Source` uses. No
    // date struct is minted and no epoch integer is invented.
    for expected in [
        "*get_selected_date)(IAvnDatePicker* self, uint16_t** value)",
        "*set_selected_date)(IAvnDatePicker* self, const uint16_t* value)",
        "*set_min_year)(IAvnDatePicker* self, const uint16_t* value)",
        "*set_max_year)(IAvnDatePicker* self, const uint16_t* value)",
        "*clear)(IAvnDatePicker* self)",
        "*get_selected_time)(IAvnTimePicker* self, uint16_t** value)",
        "*set_selected_time)(IAvnTimePicker* self, const uint16_t* value)",
        "*set_clock_identifier)(IAvnTimePicker* self, const uint16_t* value)",
        "*clear)(IAvnTimePicker* self)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    // U23 introduced IAvnDateTimeList (dates as tick collections); the raw
    // AvnDateTime/AvnTimeSpan structs and the DateTimeOffset interface stay out.
    for forbidden in [
        "typedef struct AvnDateTime",
        "AvnTimeSpan",
        "IAvnDateTimeOffset",
    ] {
        assert!(
            !HEADER.contains(forbidden),
            "header must not declare `{forbidden}`"
        );
    }
}

#[test]
fn the_factory_gains_a_creator_per_constructible_wave_b_type() {
    for expected in [
        "*create_flyout)(IAvnControlFactory* self, IAvnFlyout** value)",
        "*create_menu)(IAvnControlFactory* self, IAvnMenu** value)",
        "*create_menu_item)(IAvnControlFactory* self, IAvnMenuItem** value)",
        "*create_split_view)(IAvnControlFactory* self, IAvnSplitView** value)",
        "*create_date_picker)(IAvnControlFactory* self, IAvnDatePicker** value)",
        "*create_time_picker)(IAvnControlFactory* self, IAvnTimePicker** value)",
        "*create_headered_selecting_items_control)(IAvnControlFactory* self, IAvnHeaderedSelectingItemsControl** value)",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    // The abstract bases are reachable by query_interface, never by construction.
    for forbidden in [
        "*create_flyout_base)",
        "*create_popup_flyout_base)",
        "*create_menu_base)",
    ] {
        assert!(
            !HEADER.contains(forbidden),
            "header must not declare `{forbidden}`"
        );
    }
}

#[test]
fn wave_b_interfaces_publish_abi_version_one_and_nothing_else_moved() {
    for expected in [
        "#define I_AVN_FLYOUT_BASE_ABI_VERSION 1",
        "#define I_AVN_POPUP_FLYOUT_BASE_ABI_VERSION 6",
        "#define I_AVN_FLYOUT_ABI_VERSION 4",
        "#define I_AVN_MENU_BASE_ABI_VERSION 13",
        "#define I_AVN_MENU_ABI_VERSION 13",
        "#define I_AVN_MENU_ITEM_ABI_VERSION 16",
        "#define I_AVN_HEADERED_SELECTING_ITEMS_CONTROL_ABI_VERSION 13",
        "#define I_AVN_SPLIT_VIEW_ABI_VERSION 10",
        "#define I_AVN_DATE_PICKER_ABI_VERSION 12",
        "#define I_AVN_TIME_PICKER_ABI_VERSION 12",
        "#define I_AVN_AVALONIA_OBJECT_ABI_VERSION 2",
        "#define I_AVN_CONTROL_ABI_VERSION 9",
        "#define I_AVN_ITEMS_CONTROL_ABI_VERSION 16",
        "#define I_AVN_SELECTING_ITEMS_CONTROL_ABI_VERSION 16",
        "#define I_AVN_TEMPLATED_CONTROL_ABI_VERSION 13",
        "#define I_AVN_CONTENT_CONTROL_ABI_VERSION 14",
        "#define I_AVN_HEADERED_ITEMS_CONTROL_ABI_VERSION 13",
        "#define I_AVN_TREE_VIEW_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn wave_b_iids_are_fresh_and_distinct_from_every_shipped_one() {
    // The interfaces the new types derive from must be byte-identical to what shipped, or a
    // consumer holding one would be calling through a contract that quietly changed. These are
    // the same literals wave A pinned, so a regression shows up here rather than at runtime.
    for (name, expected, current) in [
        (
            "IAvnAvaloniaObject",
            "FA7F2E03-0BFA-5422-840B-18AE1D9695C0",
            I_AVN_AVALONIA_OBJECT_IID,
        ),
        (
            "IAvnControl",
            "ABDFF81D-DB26-5BB7-9ACA-18353E6DB4D4",
            I_AVN_CONTROL_IID,
        ),
        (
            "IAvnItemsControl",
            "B656735C-EECA-50CC-A87D-B4362F7E045A",
            I_AVN_ITEMS_CONTROL_IID,
        ),
        (
            "IAvnSelectingItemsControl",
            "56020FA9-D4A4-52B6-B5FC-EAEB6FD563C2",
            I_AVN_SELECTING_ITEMS_CONTROL_IID,
        ),
        (
            "IAvnContentControl",
            "71FA5ADD-9F62-5918-84D4-4B300B8EA192",
            I_AVN_CONTENT_CONTROL_IID,
        ),
    ] {
        assert_eq!(format_iid(&current), expected, "{name} changed its IID");
    }

    let wave_b = [
        I_AVN_FLYOUT_BASE_IID,
        I_AVN_POPUP_FLYOUT_BASE_IID,
        I_AVN_FLYOUT_IID,
        I_AVN_MENU_BASE_IID,
        I_AVN_MENU_IID,
        I_AVN_MENU_ITEM_IID,
        I_AVN_HEADERED_SELECTING_ITEMS_CONTROL_IID,
        I_AVN_SPLIT_VIEW_IID,
        I_AVN_DATE_PICKER_IID,
        I_AVN_TIME_PICKER_IID,
    ];
    let shipped = [
        I_AVN_AVALONIA_OBJECT_IID,
        I_AVN_CONTROL_IID,
        I_AVN_ITEMS_CONTROL_IID,
        I_AVN_SELECTING_ITEMS_CONTROL_IID,
        I_AVN_CONTENT_CONTROL_IID,
        I_AVN_TEMPLATED_CONTROL_IID,
        I_AVN_HEADERED_ITEMS_CONTROL_IID,
        I_AVN_TREE_VIEW_IID,
    ];
    for new in wave_b {
        for old in shipped {
            assert_ne!(
                format_iid(&new),
                format_iid(&old),
                "a wave B interface reused a shipped IID"
            );
        }
    }
    for (index, left) in wave_b.iter().enumerate() {
        for right in &wave_b[index + 1..] {
            assert_ne!(
                format_iid(left),
                format_iid(right),
                "two wave B interfaces share an IID"
            );
        }
    }
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

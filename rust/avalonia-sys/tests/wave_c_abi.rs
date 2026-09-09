//! ABI guarantees for the wave C layout panels: `WrapPanel`, `UniformGrid`,
//! `RelativePanel`, `Viewbox`, `FlexPanel`, `Thumb` and `GridSplitter`.
//!
//! Wave C only *adds* interfaces. Nothing above or beside the new types gained a slot, so every
//! interface that shipped before keeps the exact IID it last published and the seven new ones
//! publish at version 1. The one thing that does move is the factory, which gains a creator per
//! constructible wave C type plus `get_relative_panel_statics`.

use avalonia_sys::{
    I_AVN_CONTROL_IID, I_AVN_FLEX_PANEL_IID, I_AVN_GRID_SPLITTER_IID, I_AVN_PANEL_IID,
    I_AVN_RELATIVE_PANEL_IID, I_AVN_TEMPLATED_CONTROL_IID, I_AVN_THUMB_IID, I_AVN_UNIFORM_GRID_IID,
    I_AVN_VIEWBOX_IID, I_AVN_WRAP_PANEL_IID,
};

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn wrap_panel_carries_spacing_orientation_and_item_size() {
    for expected in [
        "*set_item_spacing)(IAvnWrapPanel* self, double value)",
        "*set_line_spacing)(IAvnWrapPanel* self, double value)",
        "*set_orientation)(IAvnWrapPanel* self, int32_t value)",
        "*set_items_alignment)(IAvnWrapPanel* self, int32_t value)",
        "*set_item_width)(IAvnWrapPanel* self, double value)",
        "*set_item_height)(IAvnWrapPanel* self, double value)",
        "#define I_AVN_WRAP_PANEL_ABI_VERSION 8",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn relative_panel_exposes_bool_align_with_panel_attached_properties_only() {
    for expected in [
        "*set_align_left_with_panel)(IAvnRelativePanelStatics* self, IAvnControl* target, int32_t value)",
        "*set_align_top_with_panel)(IAvnRelativePanelStatics* self, IAvnControl* target, int32_t value)",
        "*get_relative_panel_statics)(IAvnControlFactory* self, IAvnRelativePanelStatics** value)",
        "#define I_AVN_RELATIVE_PANEL_STATICS_ABI_VERSION 1",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    for forbidden in [
        "set_above",
        "set_below",
        "set_left_of",
        "set_right_of",
        "set_align_left_with)",
        "set_order)",
        "set_grow)",
        "set_shrink)",
        "set_basis)",
    ] {
        assert!(
            !HEADER.contains(forbidden),
            "header must not declare `{forbidden}`"
        );
    }
}

#[test]
fn viewbox_child_is_a_control_and_grid_splitter_sits_on_thumb() {
    for expected in [
        "*set_child)(IAvnViewbox* self, IAvnControl* value)",
        "*set_stretch)(IAvnViewbox* self, int32_t value)",
        "*set_resize_direction)(IAvnGridSplitter* self, int32_t value)",
        "*set_shows_preview)(IAvnGridSplitter* self, int32_t value)",
        "*create_wrap_panel)(IAvnControlFactory* self, IAvnWrapPanel** value)",
        "*create_uniform_grid)(IAvnControlFactory* self, IAvnUniformGrid** value)",
        "*create_relative_panel)(IAvnControlFactory* self, IAvnRelativePanel** value)",
        "*create_viewbox)(IAvnControlFactory* self, IAvnViewbox** value)",
        "*create_flex_panel)(IAvnControlFactory* self, IAvnFlexPanel** value)",
        "*advise_drag_delta)(IAvnThumb* self, IAvnThumbDragDeltaHandler* handler, int64_t* subscription_id)",
        "*create_thumb)(IAvnControlFactory* self, IAvnThumb** value)",
        "*create_grid_splitter)(IAvnControlFactory* self, IAvnGridSplitter** value)",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    assert!(
        !HEADER.contains("set_preview_content"),
        "ITemplate has no ABI shape"
    );
}

#[test]
fn wave_c_interfaces_publish_abi_version_one_and_nothing_else_moved() {
    for expected in [
        "#define I_AVN_WRAP_PANEL_ABI_VERSION 8",
        "#define I_AVN_UNIFORM_GRID_ABI_VERSION 8",
        "#define I_AVN_RELATIVE_PANEL_ABI_VERSION 8",
        "#define I_AVN_VIEWBOX_ABI_VERSION 8",
        "#define I_AVN_FLEX_PANEL_ABI_VERSION 8",
        "#define I_AVN_THUMB_ABI_VERSION 10",
        "#define I_AVN_GRID_SPLITTER_ABI_VERSION 10",
        "#define I_AVN_AVALONIA_OBJECT_ABI_VERSION 2",
        "#define I_AVN_CONTROL_ABI_VERSION 8",
        "#define I_AVN_PANEL_ABI_VERSION 11",
        "#define I_AVN_TEMPLATED_CONTROL_ABI_VERSION 12",
        "#define I_AVN_CONTENT_CONTROL_ABI_VERSION 13",
        "#define I_AVN_FLYOUT_ABI_VERSION 4",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn wave_c_iids_are_fresh_and_distinct_from_every_shipped_one() {
    let new_iids = [
        I_AVN_WRAP_PANEL_IID,
        I_AVN_UNIFORM_GRID_IID,
        I_AVN_RELATIVE_PANEL_IID,
        I_AVN_VIEWBOX_IID,
        I_AVN_FLEX_PANEL_IID,
        I_AVN_THUMB_IID,
        I_AVN_GRID_SPLITTER_IID,
    ];
    let shipped = [
        I_AVN_CONTROL_IID,
        I_AVN_PANEL_IID,
        I_AVN_TEMPLATED_CONTROL_IID,
    ];
    for iid in new_iids {
        assert!(
            !shipped.contains(&iid),
            "a wave C IID collided with a shipped interface"
        );
    }
    for (index, left) in new_iids.iter().enumerate() {
        for right in &new_iids[index + 1..] {
            assert_ne!(left, right, "two wave C interfaces share an IID");
        }
    }
}

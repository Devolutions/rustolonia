//! ABI guarantees for the wave A controls: `Image`, the tab pair, the tree pair,
//! `HeaderedItemsControl` and the `ToolTip` attached properties.
//!
//! Wave A only *adds* interfaces; the factory gains a creator per new control plus
//! `get_tool_tip_statics`.

use rustolonia_sys::{
    I_AVN_AVALONIA_OBJECT_IID, I_AVN_CONTENT_CONTROL_IID, I_AVN_CONTROL_IID,
    I_AVN_HEADERED_CONTENT_CONTROL_IID, I_AVN_HEADERED_ITEMS_CONTROL_IID, I_AVN_IMAGE_IID,
    I_AVN_ITEMS_CONTROL_IID, I_AVN_SELECTING_ITEMS_CONTROL_IID, I_AVN_TAB_CONTROL_IID,
    I_AVN_TAB_ITEM_IID, I_AVN_TOOL_TIP_IID, I_AVN_TOOL_TIP_STATICS_IID, I_AVN_TREE_VIEW_IID,
    I_AVN_TREE_VIEW_ITEM_IID,
};

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn image_source_is_published_as_a_utf16_string_slot() {
    // Source crosses as the source string the host resolves into a bitmap, so it occupies an
    // ordinary string pair rather than a projected image interface.
    for expected in [
        "*get_source)(IAvnImage* self, uint16_t** value)",
        "*set_source)(IAvnImage* self, const uint16_t* value)",
        "*set_stretch)(IAvnImage* self, int32_t value)",
        "*set_stretch_direction)(IAvnImage* self, int32_t value)",
        "*set_blend_mode)(IAvnImage* self, int32_t value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    // No image object is minted, so nothing here carries IBrush-level reference-graph
    // complexity.
    for forbidden in ["IAvnImageSource", "IAvnBitmap", "IAvnImageList"] {
        assert!(
            !HEADER.contains(forbidden),
            "header must not declare `{forbidden}`"
        );
    }
}

#[test]
fn tool_tip_tip_is_a_string_attached_property_beside_the_scalar_ones() {
    for expected in [
        "*get_tip)(IAvnToolTipStatics* self, IAvnControl* target, uint16_t** value)",
        "*set_tip)(IAvnToolTipStatics* self, IAvnControl* target, const uint16_t* value)",
        "*set_show_delay)(IAvnToolTipStatics* self, IAvnControl* target, int32_t value)",
        "*set_placement)(IAvnToolTipStatics* self, IAvnControl* target, int32_t value)",
        "*set_horizontal_offset)(IAvnToolTipStatics* self, IAvnControl* target, double value)",
        // The factory hands the statics object out beside the ones it already published.
        "*get_tool_tip_statics)(IAvnControlFactory* self, IAvnToolTipStatics** value)",
        "*create_image)(IAvnControlFactory* self, IAvnImage** value)",
        "*create_tab_control)(IAvnControlFactory* self, IAvnTabControl** value)",
        "*create_tree_view_item)(IAvnControlFactory* self, IAvnTreeViewItem** value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn wave_a_iids_are_distinct_from_each_other_and_their_bases() {
    let wave_a = [
        I_AVN_IMAGE_IID,
        I_AVN_HEADERED_ITEMS_CONTROL_IID,
        I_AVN_TAB_CONTROL_IID,
        I_AVN_TAB_ITEM_IID,
        I_AVN_TREE_VIEW_IID,
        I_AVN_TREE_VIEW_ITEM_IID,
        I_AVN_TOOL_TIP_IID,
        I_AVN_TOOL_TIP_STATICS_IID,
    ];
    let shipped = [
        I_AVN_AVALONIA_OBJECT_IID,
        I_AVN_CONTROL_IID,
        I_AVN_ITEMS_CONTROL_IID,
        I_AVN_SELECTING_ITEMS_CONTROL_IID,
        I_AVN_CONTENT_CONTROL_IID,
        I_AVN_HEADERED_CONTENT_CONTROL_IID,
    ];
    for new in wave_a {
        for old in shipped {
            assert_ne!(
                format_iid(&new),
                format_iid(&old),
                "a wave A interface reused a shipped IID"
            );
        }
    }
    for (index, left) in wave_a.iter().enumerate() {
        for right in &wave_a[index + 1..] {
            assert_ne!(
                format_iid(left),
                format_iid(right),
                "two wave A interfaces share an IID"
            );
        }
    }
}

fn format_iid(iid: &rustolonia_sys::Guid) -> String {
    let mut out = format!("{:08X}-{:04X}-{:04X}-", iid.data1, iid.data2, iid.data3);
    for (index, byte) in iid.data4.iter().enumerate() {
        if index == 2 {
            out.push('-');
        }
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

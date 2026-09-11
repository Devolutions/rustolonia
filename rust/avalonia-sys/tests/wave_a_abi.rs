//! ABI guarantees for the wave A controls: `Image`, the tab pair, the tree pair,
//! `HeaderedItemsControl` and the `ToolTip` attached properties.
//!
//! Wave A only *adds* interfaces. Nothing above or beside the new types gained a slot, so every
//! interface that shipped before keeps the exact IID it last published and the seven new ones
//! publish at version 1. The one thing that does move is the factory, which gains a creator per
//! new control plus `get_tool_tip_statics`.

use avalonia_sys::{
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
        "#define I_AVN_IMAGE_ABI_VERSION 11",
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
        "#define I_AVN_TOOL_TIP_STATICS_ABI_VERSION 1",
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
fn wave_a_interfaces_publish_abi_version_one_and_nothing_else_moved() {
    for expected in [
        "#define I_AVN_IMAGE_ABI_VERSION 11",
        "#define I_AVN_HEADERED_ITEMS_CONTROL_ABI_VERSION 15",
        "#define I_AVN_TAB_CONTROL_ABI_VERSION 15",
        "#define I_AVN_TAB_ITEM_ABI_VERSION 14",
        "#define I_AVN_TREE_VIEW_ABI_VERSION 15",
        "#define I_AVN_TREE_VIEW_ITEM_ABI_VERSION 15",
        "#define I_AVN_TOOL_TIP_ABI_VERSION 12",
        // Every base the new interfaces sit on kept the version whose flattened vtable it
        // still matches, so no shipped consumer has to requery anything but the factory.
        "#define I_AVN_AVALONIA_OBJECT_ABI_VERSION 2",
        "#define I_AVN_CONTROL_ABI_VERSION 11",
        "#define I_AVN_ITEMS_CONTROL_ABI_VERSION 18",
        "#define I_AVN_SELECTING_ITEMS_CONTROL_ABI_VERSION 18",
        "#define I_AVN_CONTENT_CONTROL_ABI_VERSION 16",
        "#define I_AVN_HEADERED_CONTENT_CONTROL_ABI_VERSION 16",
        // Only the factory grew slots. Wave B moved it again, from 3 to 4, for its own
        // creators; every wave A interface still publishes at version 1.
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn wave_a_iids_are_fresh_and_distinct_from_every_shipped_one() {
    // The interfaces the new types derive from must be byte-identical to what shipped, or a
    // consumer holding one would be calling through a contract that quietly changed.
    for (name, expected, current) in [
        (
            "IAvnAvaloniaObject",
            "FA7F2E03-0BFA-5422-840B-18AE1D9695C0",
            I_AVN_AVALONIA_OBJECT_IID,
        ),
        (
            "IAvnControl",
            "8B272500-09E4-535F-ABF1-14992E90E778",
            I_AVN_CONTROL_IID,
        ),
        (
            "IAvnItemsControl",
            "C6061984-593E-5875-8956-4B5C4B6D886C",
            I_AVN_ITEMS_CONTROL_IID,
        ),
        (
            "IAvnSelectingItemsControl",
            "BF3CA6E3-C054-5059-AFE7-20E38D9B9DE8",
            I_AVN_SELECTING_ITEMS_CONTROL_IID,
        ),
        (
            "IAvnContentControl",
            "7312A513-1E46-5A98-9507-851220C7A465",
            I_AVN_CONTENT_CONTROL_IID,
        ),
        (
            "IAvnHeaderedContentControl",
            "0408F0EA-5BA0-54C6-A8E6-979328FBE88C",
            I_AVN_HEADERED_CONTENT_CONTROL_IID,
        ),
    ] {
        assert_eq!(format_iid(&current), expected, "{name} changed its IID");
    }

    // The pre-U17 IIDs of the items lineage are retired: ItemsControl and
    // SelectingItemsControl grew, and reusing those IIDs would let a stale
    // consumer call slots the old contract never declared.
    for (name, retired, current) in [
        (
            "IAvnItemsControl",
            "E61DDE0E-DE4F-5D63-B485-C9A8FB72537F",
            I_AVN_ITEMS_CONTROL_IID,
        ),
        (
            "IAvnSelectingItemsControl",
            "0381D38A-B15F-53A5-8D29-BADCE2D1173D",
            I_AVN_SELECTING_ITEMS_CONTROL_IID,
        ),
    ] {
        assert_ne!(format_iid(&current), retired, "{name} reused a retired IID");
    }

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

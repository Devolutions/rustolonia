//! ABI guarantees for the solid brush interface and the chrome members it unblocked.
//!
//! `Background`, `BorderBrush` and `Foreground` are the first members carried as an interface
//! pointer that is not a projected control, so this pins the generated header, the brush IID,
//! and the version bump that widening `Border`, `Panel`, `TemplatedControl` and `TextBlock`
//! forced on every interface below them.

use avalonia_sys::{
    I_AVN_AVALONIA_OBJECT_IID, I_AVN_BORDER_IID, I_AVN_BRUSH_IID, I_AVN_BUTTON_IID,
    I_AVN_CONTROL_IID, I_AVN_DECORATOR_IID, I_AVN_PANEL_IID, I_AVN_STYLED_ELEMENT_IID,
    I_AVN_TEXT_BLOCK_IID,
};

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn brush_publishes_a_read_only_colour_and_opacity_vtable() {
    for expected in [
        "typedef struct IAvnBrush IAvnBrush;",
        "*get_color)(IAvnBrush* self, AvnColor* value)",
        "*get_opacity)(IAvnBrush* self, double* value)",
        "#define I_AVN_BRUSH_VTABLE_SLOTS 5",
        "#define I_AVN_BRUSH_ABI_VERSION 1",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    // Read-only: the managed side hands out immutable brushes, so there is no setter to let a
    // caller mutate a brush another control is sharing.
    for forbidden in [
        "*set_color)(IAvnBrush*",
        "*set_opacity)(IAvnBrush*",
        "IAvnGradientBrush",
        "IAvnDrawingBrush",
        "IAvnVisualBrush",
    ] {
        assert!(
            !HEADER.contains(forbidden),
            "header must not declare `{forbidden}`"
        );
    }
}

#[test]
fn a_brush_is_minted_by_the_factory_rather_than_by_a_setter() {
    assert!(HEADER.contains(
        "*create_solid_color_brush)(IAvnControlFactory* self, AvnColor color, double opacity, IAvnBrush** value)"
    ));
}

#[test]
fn chrome_members_publish_brush_pointers_and_geometry_by_value() {
    for expected in [
        "*get_background)(IAvnBorder* self, IAvnBrush** value)",
        "*set_background)(IAvnBorder* self, IAvnBrush* value)",
        "*set_border_brush)(IAvnBorder* self, IAvnBrush* value)",
        "*set_border_thickness)(IAvnBorder* self, AvnThickness value)",
        "*set_corner_radius)(IAvnBorder* self, AvnCornerRadius value)",
        "*set_background)(IAvnPanel* self, IAvnBrush* value)",
        "*set_background)(IAvnTemplatedControl* self, IAvnBrush* value)",
        "*set_foreground)(IAvnTemplatedControl* self, IAvnBrush* value)",
        "*set_font_size)(IAvnTemplatedControl* self, double value)",
        "*set_foreground)(IAvnTextBlock* self, IAvnBrush* value)",
        "*set_font_size)(IAvnTextBlock* self, double value)",
        "*set_font_weight)(IAvnTextBlock* self, int32_t value)",
        "*set_text_alignment)(IAvnTextBlock* self, int32_t value)",
        "*set_padding)(IAvnTextBlock* self, AvnThickness value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn chrome_wave_interfaces_publish_abi_version_four() {
    for expected in [
        "#define I_AVN_BORDER_ABI_VERSION 15",
        "#define I_AVN_PANEL_ABI_VERSION 14",
        "#define I_AVN_TEMPLATED_CONTROL_ABI_VERSION 15",
        "#define I_AVN_TEXT_BLOCK_ABI_VERSION 17",
        // The factory grew create_solid_color_brush at version 2, a creator per wave A control
        // at version 3, and a creator per constructible wave B type at version 4.
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
        // Nothing was added to StyledElement, Control or Decorator, and none of their bases
        // moved, so their flattened vtables are byte-identical to version 3.
        "#define I_AVN_STYLED_ELEMENT_ABI_VERSION 6",
        "#define I_AVN_CONTROL_ABI_VERSION 11",
        "#define I_AVN_DECORATOR_ABI_VERSION 13",
        "#define I_AVN_AVALONIA_OBJECT_ABI_VERSION 2",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn widened_interfaces_republish_under_fresh_iids() {
    // The version 3 IIDs are retired: reusing one for a longer vtable would let a stale
    // consumer call through slots the old contract never declared.
    for (name, retired, current) in [
        (
            "IAvnBorder",
            "0A321CFC-85AB-5395-B39D-D6FF147BFF08",
            I_AVN_BORDER_IID,
        ),
        (
            "IAvnPanel",
            "873178A5-1556-51BD-BC8F-FD98F9BFDAEA",
            I_AVN_PANEL_IID,
        ),
        (
            "IAvnTextBlock",
            "48BE6862-9BA1-53AE-A197-41590E4E6A21",
            I_AVN_TEXT_BLOCK_IID,
        ),
        (
            "IAvnButton",
            "C50F6CD2-6A8C-5848-AB0D-ADA71B3E403E",
            I_AVN_BUTTON_IID,
        ),
    ] {
        assert_ne!(format_iid(&current), retired, "{name} reused a retired IID");
    }

    // The interfaces whose flattened vtable never moved keep the IIDs they already published.
    assert_eq!(
        format_iid(&I_AVN_AVALONIA_OBJECT_IID),
        "FA7F2E03-0BFA-5422-840B-18AE1D9695C0"
    );
    assert_eq!(
        format_iid(&I_AVN_STYLED_ELEMENT_IID),
        "383D0620-1A23-575B-B609-391608903CB1"
    );
    // Wave P grew Control (KeyUp/GotFocus/LostFocus), so Control and Decorator
    // republished under fresh IIDs.
    assert_eq!(
        format_iid(&I_AVN_CONTROL_IID),
        "8B272500-09E4-535F-ABF1-14992E90E778"
    );
    assert_eq!(
        format_iid(&I_AVN_DECORATOR_IID),
        "F8530439-ABA5-5B16-9C0B-2132D4EFD205"
    );

    // The brush is a brand new interface, so it starts at version 1 with its own IID.
    assert_eq!(
        format_iid(&I_AVN_BRUSH_IID),
        "FC7CCBAE-ED75-5C6D-8516-DF706E137ED3"
    );
    let declaration = HEADER
        .split("static const AvnGuid I_AVN_BRUSH_IID = {")
        .nth(1)
        .expect("header declares the brush IID");
    assert!(declaration.contains(&format!("0x{:08X},", I_AVN_BRUSH_IID.data1)));
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

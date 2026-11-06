//! ABI guarantees for the solid brush interface and the chrome members it unblocked.
//!
//! `Background`, `BorderBrush` and `Foreground` are the first members carried as an interface
//! pointer that is not a projected control, so this pins the generated header and the brush
//! vtable that `Border`, `Panel`, `TemplatedControl` and `TextBlock` consume.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn brush_publishes_a_read_only_colour_and_opacity_vtable() {
    for expected in [
        "typedef struct IAvnBrush IAvnBrush;",
        "*get_color)(IAvnBrush* self, AvnColor* value)",
        "*get_opacity)(IAvnBrush* self, double* value)",
        "#define I_AVN_BRUSH_VTABLE_SLOTS 5",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    // Read-only: the managed side hands out immutable brushes, so there is no setter to let a
    // caller mutate a brush another control is sharing.
    for forbidden in [
        "*set_color)(IAvnBrush*",
        "*set_opacity)(IAvnBrush*",
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

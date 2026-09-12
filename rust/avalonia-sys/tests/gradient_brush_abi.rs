//! ABI guarantees for the gradient brush capability interfaces.
//!
//! Gradients follow the sibling-interface pattern rather than widening `IAvnBrush`:
//! `IAvnGradientBrush` carries the members every gradient shares, and
//! `IAvnLinearGradientBrush`/`IAvnRadialGradientBrush` each derive from it and add their own
//! geometry. `IAvnBrush` itself keeps the IID and vtable it published with in the solid-brush
//! wave (see `brush_abi.rs`); only `IAvnControlFactory` republishes, to gain the two new
//! creator slots.

use avalonia_sys::{
    I_AVN_BRUSH_IID, I_AVN_GRADIENT_BRUSH_IID, I_AVN_LINEAR_GRADIENT_BRUSH_IID,
    I_AVN_RADIAL_GRADIENT_BRUSH_IID,
};

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn gradient_brush_publishes_the_shared_read_only_members() {
    for expected in [
        "typedef struct IAvnGradientBrush IAvnGradientBrush;",
        "*get_opacity)(IAvnGradientBrush* self, double* value)",
        "*get_spread_method)(IAvnGradientBrush* self, int32_t* value)",
        "*get_stop_count)(IAvnGradientBrush* self, int32_t* value)",
        "*get_stops)(IAvnGradientBrush* self, AvnGradientStopBuffer* value)",
        "#define I_AVN_GRADIENT_BRUSH_VTABLE_SLOTS 7",
        "#define I_AVN_GRADIENT_BRUSH_ABI_VERSION 1",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn linear_gradient_brush_adds_its_start_and_end_point() {
    for expected in [
        "typedef struct IAvnLinearGradientBrush IAvnLinearGradientBrush;",
        "*get_start_point)(IAvnLinearGradientBrush* self, AvnRelativePoint* value)",
        "*get_end_point)(IAvnLinearGradientBrush* self, AvnRelativePoint* value)",
        "#define I_AVN_LINEAR_GRADIENT_BRUSH_VTABLE_SLOTS 9",
        "#define I_AVN_LINEAR_GRADIENT_BRUSH_ABI_VERSION 1",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn radial_gradient_brush_adds_its_ellipse_geometry() {
    for expected in [
        "typedef struct IAvnRadialGradientBrush IAvnRadialGradientBrush;",
        "*get_center)(IAvnRadialGradientBrush* self, AvnRelativePoint* value)",
        "*get_gradient_origin)(IAvnRadialGradientBrush* self, AvnRelativePoint* value)",
        "*get_radius_x)(IAvnRadialGradientBrush* self, AvnRelativeScalar* value)",
        "*get_radius_y)(IAvnRadialGradientBrush* self, AvnRelativeScalar* value)",
        "#define I_AVN_RADIAL_GRADIENT_BRUSH_VTABLE_SLOTS 11",
        "#define I_AVN_RADIAL_GRADIENT_BRUSH_ABI_VERSION 1",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn gradients_are_minted_by_the_factory_and_carry_a_fixed_capacity_stop_buffer() {
    assert!(HEADER.contains(
        "*create_linear_gradient_brush)(IAvnControlFactory* self, double opacity, \
         int32_t spread_method, int32_t stop_count, AvnGradientStopBuffer stops, \
         AvnRelativePoint start_point, AvnRelativePoint end_point, \
         IAvnLinearGradientBrush** value)"
    ));
    assert!(HEADER.contains(
        "*create_radial_gradient_brush)(IAvnControlFactory* self, double opacity, \
         int32_t spread_method, int32_t stop_count, AvnGradientStopBuffer stops, \
         AvnRelativePoint center, AvnRelativePoint gradient_origin, \
         AvnRelativeScalar radius_x, AvnRelativeScalar radius_y, \
         IAvnRadialGradientBrush** value)"
    ));
    assert!(HEADER.contains("AvnGradientStop stops[8];"));
}

#[test]
fn gradient_interfaces_are_brand_new_and_distinct_from_each_other_and_from_brush() {
    let iids = [
        ("IAvnBrush", I_AVN_BRUSH_IID),
        ("IAvnGradientBrush", I_AVN_GRADIENT_BRUSH_IID),
        ("IAvnLinearGradientBrush", I_AVN_LINEAR_GRADIENT_BRUSH_IID),
        ("IAvnRadialGradientBrush", I_AVN_RADIAL_GRADIENT_BRUSH_IID),
    ];
    for (i, (name_a, iid_a)) in iids.iter().enumerate() {
        for (name_b, iid_b) in &iids[i + 1..] {
            assert_ne!(
                format_iid(iid_a),
                format_iid(iid_b),
                "{name_a} and {name_b} must not share an IID"
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

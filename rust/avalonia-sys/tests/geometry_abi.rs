//! Layout guarantees for the blittable geometry structs shared with the C header
//! (`avalonia-sys/include/avalonia-rust-abi.h`) and the managed COM layer.

use avalonia_sys::{AvnColor, AvnCornerRadius, AvnPoint, AvnRect, AvnSize, AvnThickness};
use std::mem::{align_of, size_of};

macro_rules! offset_of {
    ($ty:ty, $field:ident) => {{
        let value = <$ty>::default();
        let base = std::ptr::addr_of!(value) as usize;
        let field = std::ptr::addr_of!(value.$field) as usize;
        field - base
    }};
}

#[test]
fn geometry_structs_have_the_documented_sizes() {
    assert_eq!(size_of::<AvnThickness>(), 32);
    assert_eq!(size_of::<AvnCornerRadius>(), 32);
    assert_eq!(size_of::<AvnSize>(), 16);
    assert_eq!(size_of::<AvnPoint>(), 16);
    assert_eq!(size_of::<AvnRect>(), 32);
    assert_eq!(size_of::<AvnColor>(), 4);

    assert_eq!(align_of::<AvnThickness>(), align_of::<f64>());
    assert_eq!(align_of::<AvnColor>(), align_of::<u32>());
}

#[test]
fn geometry_fields_are_sequential() {
    assert_eq!(offset_of!(AvnThickness, left), 0);
    assert_eq!(offset_of!(AvnThickness, top), 8);
    assert_eq!(offset_of!(AvnThickness, right), 16);
    assert_eq!(offset_of!(AvnThickness, bottom), 24);

    assert_eq!(offset_of!(AvnCornerRadius, top_left), 0);
    assert_eq!(offset_of!(AvnCornerRadius, top_right), 8);
    assert_eq!(offset_of!(AvnCornerRadius, bottom_right), 16);
    assert_eq!(offset_of!(AvnCornerRadius, bottom_left), 24);

    assert_eq!(offset_of!(AvnSize, width), 0);
    assert_eq!(offset_of!(AvnSize, height), 8);

    assert_eq!(offset_of!(AvnPoint, x), 0);
    assert_eq!(offset_of!(AvnPoint, y), 8);

    assert_eq!(offset_of!(AvnRect, x), 0);
    assert_eq!(offset_of!(AvnRect, y), 8);
    assert_eq!(offset_of!(AvnRect, width), 16);
    assert_eq!(offset_of!(AvnRect, height), 24);

    assert_eq!(offset_of!(AvnColor, argb), 0);
}

#[test]
fn geometry_structs_round_trip_through_raw_bytes() {
    let thickness = AvnThickness {
        left: 1.0,
        top: 2.0,
        right: 3.0,
        bottom: 4.0,
    };
    let bytes: [u8; 32] = unsafe { std::mem::transmute(thickness) };
    let restored: AvnThickness = unsafe { std::mem::transmute(bytes) };
    assert_eq!(thickness, restored);

    let color = AvnColor { argb: 0x1234_5678 };
    let bytes: [u8; 4] = unsafe { std::mem::transmute(color) };
    assert_eq!(u32::from_ne_bytes(bytes), 0x1234_5678);
}

#[test]
fn generated_header_declares_the_same_structs() {
    let header = include_str!("../include/avalonia-rust-abi.h");

    for expected in [
        "typedef struct AvnThickness {",
        "    double left;",
        "    double top;",
        "    double right;",
        "    double bottom;",
        "} AvnThickness;",
        "typedef struct AvnCornerRadius {",
        "    double top_left;",
        "    double bottom_left;",
        "} AvnCornerRadius;",
        "typedef struct AvnSize {",
        "} AvnSize;",
        "typedef struct AvnPoint {",
        "} AvnPoint;",
        "typedef struct AvnRect {",
        "} AvnRect;",
        "typedef struct AvnColor {",
        "    uint32_t argb;",
        "} AvnColor;",
    ] {
        assert!(header.contains(expected), "header is missing `{expected}`");
    }
}

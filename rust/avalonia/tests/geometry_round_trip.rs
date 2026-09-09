//! Safe geometry newtypes round-trip through their `avalonia-sys` ABI structs.

use avalonia::{Color, CornerRadius, Point, Rect, Size, Thickness};
use avalonia_sys as sys;

#[test]
fn thickness_round_trips() {
    let value = Thickness::new(1.0, 2.5, -3.0, 4.25);
    let abi: sys::AvnThickness = value.into();

    assert_eq!(abi.left, 1.0);
    assert_eq!(abi.top, 2.5);
    assert_eq!(abi.right, -3.0);
    assert_eq!(abi.bottom, 4.25);
    assert_eq!(Thickness::from(abi), value);
}

#[test]
fn corner_radius_round_trips() {
    let value = CornerRadius::new(1.0, 2.0, 3.0, 4.0);
    let abi: sys::AvnCornerRadius = value.into();

    assert_eq!(abi.top_left, 1.0);
    assert_eq!(abi.top_right, 2.0);
    assert_eq!(abi.bottom_right, 3.0);
    assert_eq!(abi.bottom_left, 4.0);
    assert_eq!(CornerRadius::from(abi), value);
}

#[test]
fn size_and_point_round_trip() {
    let size = Size::new(640.0, 480.0);
    let abi: sys::AvnSize = size.into();
    assert_eq!((abi.width, abi.height), (640.0, 480.0));
    assert_eq!(Size::from(abi), size);

    let point = Point::new(-12.5, 7.25);
    let abi: sys::AvnPoint = point.into();
    assert_eq!((abi.x, abi.y), (-12.5, 7.25));
    assert_eq!(Point::from(abi), point);
}

#[test]
fn rect_round_trips() {
    let value = Rect::new(1.0, 2.0, 30.0, 40.0);
    let abi: sys::AvnRect = value.into();

    assert_eq!(abi.x, 1.0);
    assert_eq!(abi.y, 2.0);
    assert_eq!(abi.width, 30.0);
    assert_eq!(abi.height, 40.0);
    assert_eq!(Rect::from(abi), value);
}

#[test]
fn color_packs_argb_like_avalonia_media_color() {
    let value = Color::new(0x12, 0x34, 0x56, 0x78);
    let abi: sys::AvnColor = value.into();

    // Matches Avalonia.Media.Color.ToUInt32(): (A << 24) | (R << 16) | (G << 8) | B.
    assert_eq!(abi.argb, 0x1234_5678);
    assert_eq!(Color::from(abi), value);

    let opaque_red = Color::new(0xFF, 0xFF, 0x00, 0x00);
    assert_eq!(sys::AvnColor::from(opaque_red).argb, 0xFFFF_0000);
    assert_eq!(
        Color::from(sys::AvnColor { argb: 0x8000_FF00 }),
        Color::new(0x80, 0x00, 0xFF, 0x00)
    );
}

#[test]
fn defaults_are_zeroed() {
    assert_eq!(Thickness::default(), Thickness::new(0.0, 0.0, 0.0, 0.0));
    assert_eq!(Size::default(), Size::new(0.0, 0.0));
    assert_eq!(Color::default(), Color::new(0, 0, 0, 0));
    assert_eq!(sys::AvnRect::default().width, 0.0);
}

#[test]
fn thickness_helpers_match_the_explicit_constructor() {
    assert_eq!(Thickness::uniform(8.0), Thickness::new(8.0, 8.0, 8.0, 8.0));
    assert_eq!(
        Thickness::symmetric(12.0, 4.0),
        Thickness::new(12.0, 4.0, 12.0, 4.0)
    );
    assert_eq!(
        CornerRadius::uniform(6.0),
        CornerRadius::new(6.0, 6.0, 6.0, 6.0)
    );

    // The helpers are const, so they can seed constants in a consumer's layout tables.
    const PADDING: Thickness = Thickness::uniform(2.0);
    assert_eq!(sys::AvnThickness::from(PADDING).bottom, 2.0);
}

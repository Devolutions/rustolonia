//! The safe gradient brush types (`LinearGradientBrush`, `RadialGradientBrush`, `Paint`) and
//! their supporting value types (`GradientStop`, `SpreadMethod`, `RelativeUnit`,
//! `RelativePoint`, `RelativeScalar`).
//!
//! Like `Brush`, these are plain values in Rust; minting one across the ABI needs a live
//! host factory (`IAvnControlFactory::CreateLinearGradientBrush` /
//! `CreateRadialGradientBrush`). Neither shape is wired to a `Background`/`BorderBrush`/
//! `Foreground` control property in this wave, so there is no public route to a real ABI
//! round trip from outside the crate; these tests cover the value semantics that the ABI
//! marshalling (`to_abi`/`from_abi`) builds on.

use avalonia::{Color, GradientStop, LinearGradientBrush, Paint, RadialGradientBrush};
use avalonia::{RelativePoint, RelativeScalar, RelativeUnit, SpreadMethod};

#[test]
fn new_linear_gradient_is_fully_opaque_with_pad_spread() {
    let stops = vec![
        GradientStop::new(0.0, Color::rgb(0xFF, 0x00, 0x00)),
        GradientStop::new(1.0, Color::rgb(0x00, 0x00, 0xFF)),
    ];
    let start = RelativePoint::new(0.0, 0.0, RelativeUnit::Relative);
    let end = RelativePoint::new(1.0, 1.0, RelativeUnit::Relative);

    let brush = LinearGradientBrush::new(stops.clone(), start, end);

    assert_eq!(brush.opacity, 1.0);
    assert_eq!(brush.spread_method, SpreadMethod::Pad);
    assert_eq!(brush.stops, stops);
    assert_eq!(brush.start_point, start);
    assert_eq!(brush.end_point, end);
}

#[test]
fn new_radial_gradient_is_fully_opaque_with_pad_spread() {
    let stops = vec![GradientStop::new(0.5, Color::rgb(0x10, 0x20, 0x30))];
    let center = RelativePoint::new(0.5, 0.5, RelativeUnit::Relative);
    let origin = RelativePoint::new(0.5, 0.5, RelativeUnit::Relative);
    let radius_x = RelativeScalar::new(0.5, RelativeUnit::Relative);
    let radius_y = RelativeScalar::new(0.5, RelativeUnit::Relative);

    let brush = RadialGradientBrush::new(stops.clone(), center, origin, radius_x, radius_y);

    assert_eq!(brush.opacity, 1.0);
    assert_eq!(brush.spread_method, SpreadMethod::Pad);
    assert_eq!(brush.stops, stops);
    assert_eq!(brush.center, center);
    assert_eq!(brush.gradient_origin, origin);
    assert_eq!(brush.radius_x, radius_x);
    assert_eq!(brush.radius_y, radius_y);
}

#[test]
fn spread_method_round_trips_through_its_abi_ordinal() {
    assert_eq!(SpreadMethod::try_from(0).unwrap(), SpreadMethod::Pad);
    assert_eq!(SpreadMethod::try_from(1).unwrap(), SpreadMethod::Reflect);
    assert_eq!(SpreadMethod::try_from(2).unwrap(), SpreadMethod::Repeat);
    assert!(SpreadMethod::try_from(3).is_err());
}

#[test]
fn relative_unit_round_trips_through_its_abi_ordinal() {
    assert_eq!(RelativeUnit::try_from(0).unwrap(), RelativeUnit::Relative);
    assert_eq!(RelativeUnit::try_from(1).unwrap(), RelativeUnit::Absolute);
    assert!(RelativeUnit::try_from(2).is_err());
}

#[test]
fn a_colour_converts_into_a_solid_paint() {
    let paint: Paint = Color::rgb(0x10, 0x20, 0x30).into();

    assert_eq!(paint, Paint::Solid(Color::rgb(0x10, 0x20, 0x30).into()));
}

#[test]
fn a_gradient_brush_converts_into_the_matching_paint_variant() {
    let linear = LinearGradientBrush::new(
        vec![GradientStop::new(0.0, Color::rgb(0xFF, 0xFF, 0xFF))],
        RelativePoint::new(0.0, 0.0, RelativeUnit::Relative),
        RelativePoint::new(1.0, 0.0, RelativeUnit::Relative),
    );
    let radial = RadialGradientBrush::new(
        vec![GradientStop::new(0.0, Color::rgb(0x00, 0x00, 0x00))],
        RelativePoint::new(0.5, 0.5, RelativeUnit::Relative),
        RelativePoint::new(0.5, 0.5, RelativeUnit::Relative),
        RelativeScalar::new(0.5, RelativeUnit::Relative),
        RelativeScalar::new(0.5, RelativeUnit::Relative),
    );

    assert_eq!(Paint::from(linear.clone()), Paint::LinearGradient(linear));
    assert_eq!(Paint::from(radial.clone()), Paint::RadialGradient(radial));
}

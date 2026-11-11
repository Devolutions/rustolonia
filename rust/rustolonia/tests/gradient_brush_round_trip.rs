//! The safe gradient brush types (`LinearGradientBrush`, `RadialGradientBrush`, `Paint`) and
//! their supporting value types (`GradientStop`, `SpreadMethod`, `RelativeUnit`,
//! `RelativePoint`, `RelativeScalar`).
//!
//! Like `Brush`, these are plain values in Rust; minting one across the ABI needs a live
//! host factory (`IAvnControlFactory::CreateLinearGradientBrush` /
//! `CreateRadialGradientBrush`). These tests cover the pure-Rust value semantics and
//! `IntoValue<Option<Paint>>` conversions; `nativeaot_object_model` covers a real host
//! round trip on control brush properties.

use rustolonia::{
    Brush, Color, GradientStop, IntoValue, LinearGradientBrush, Paint, RadialGradientBrush,
};
use rustolonia::{RelativePoint, RelativeScalar, RelativeUnit, SpreadMethod};

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

    assert_eq!(
        Paint::from(linear.clone()),
        Paint::LinearGradient(linear.clone())
    );
    assert_eq!(
        Paint::from(radial.clone()),
        Paint::RadialGradient(radial.clone())
    );

    let from_color: Option<Paint> = Color::rgb(0x11, 0x22, 0x33).into_value();
    assert_eq!(
        from_color,
        Some(Paint::Solid(Brush::solid(Color::rgb(0x11, 0x22, 0x33))))
    );
    let from_rgb_tuple: Option<Paint> = (0x44_u8, 0x55_u8, 0x66_u8).into_value();
    assert_eq!(
        from_rgb_tuple,
        Some(Paint::Solid(Brush::solid(Color::rgb(0x44, 0x55, 0x66))))
    );
    let from_opt_brush: Option<Paint> = Some(Brush::new(Color::rgb(1, 2, 3), 0.5)).into_value();
    assert_eq!(
        from_opt_brush,
        Some(Paint::Solid(Brush::new(Color::rgb(1, 2, 3), 0.5)))
    );
    let cleared_brush: Option<Paint> = Option::<Brush>::None.into_value();
    assert_eq!(cleared_brush, None);
    let cleared_paint: Option<Paint> = Option::<Paint>::None.into_value();
    assert_eq!(cleared_paint, None);
    let from_linear: Option<Paint> = linear.clone().into_value();
    assert_eq!(from_linear, Some(Paint::LinearGradient(linear)));
    let from_radial: Option<Paint> = radial.clone().into_value();
    assert_eq!(from_radial, Some(Paint::RadialGradient(radial)));
}

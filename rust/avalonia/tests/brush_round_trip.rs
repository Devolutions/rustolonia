//! The safe `Brush` newtype and its colour helpers.
//!
//! `Brush` is a plain value in Rust; crossing the ABI turns it into an `IAvnBrush` minted by
//! the host factory, which needs a live UI context. These tests therefore cover the value
//! semantics; `nativeaot_object_model` covers a real host round trip.

use avalonia::{Brush, Color};
use avalonia_sys as sys;

#[test]
fn solid_is_fully_opaque() {
    let brush = Brush::solid(Color::rgb(0x33, 0x66, 0x99));

    assert_eq!(brush.color, Color::new(0xFF, 0x33, 0x66, 0x99));
    assert_eq!(brush.opacity, 1.0);
    assert_eq!(brush, Brush::new(Color::rgb(0x33, 0x66, 0x99), 1.0));
}

#[test]
fn a_colour_converts_into_a_solid_brush() {
    let brush: Brush = Color::rgb(0x10, 0x20, 0x30).into();

    assert_eq!(brush, Brush::solid(Color::rgb(0x10, 0x20, 0x30)));
}

#[test]
fn opacity_is_carried_alongside_the_colour() {
    let brush = Brush::new(Color::new(0x80, 0xAA, 0xBB, 0xCC), 0.25);

    assert_eq!(brush.opacity, 0.25);
    assert_eq!(sys::AvnColor::from(brush.color).argb, 0x80AA_BBCC);
}

#[test]
fn rgb_is_const_so_it_can_seed_a_palette() {
    const ACCENT: Brush = Brush::solid(Color::rgb(0x00, 0x7A, 0xCC));

    assert_eq!(ACCENT.color, Color::new(0xFF, 0x00, 0x7A, 0xCC));
    assert_eq!(sys::AvnColor::from(ACCENT.color).argb, 0xFF00_7ACC);
}

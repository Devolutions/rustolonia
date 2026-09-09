//! Variant conversions from Rust values. The UTF-16 paths across the ABI
//! need the host's allocator and are covered by the host COM tests; the
//! scalar tags map losslessly and are exercised end to end there.

use avalonia::Variant;

#[test]
fn variant_conversions_from_rust_values() {
    assert_eq!(Variant::from("text"), Variant::Utf16("text".to_string()));
    assert_eq!(
        Variant::from(String::from("text")),
        Variant::Utf16("text".to_string())
    );
    assert_eq!(Variant::from(7i32), Variant::I32(7));
    assert_eq!(Variant::from(7.5f64), Variant::F64(7.5));
    assert_eq!(Variant::from(true), Variant::Bool(true));
}

//! ABI guarantees for the layout members published on the widened control vtables.
//!
//! Layout slots were added to `IAvnStyledElement` and `IAvnControl`, whose flattened
//! vtables every projected control inherits, so this pins the generated header and the
//! by-value `AvnThickness` calling convention together.

use rustolonia_sys::AvnThickness;

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn control_publishes_the_layout_slots_with_geometry_by_value() {
    for expected in [
        "*get_margin)(IAvnControl* self, AvnThickness* value)",
        "*set_margin)(IAvnControl* self, AvnThickness value)",
        "*get_horizontal_alignment)(IAvnControl* self, int32_t* value)",
        "*set_horizontal_alignment)(IAvnControl* self, int32_t value)",
        "*set_vertical_alignment)(IAvnControl* self, int32_t value)",
        "*set_min_width)(IAvnControl* self, double value)",
        "*set_min_height)(IAvnControl* self, double value)",
        "*set_max_width)(IAvnControl* self, double value)",
        "*set_max_height)(IAvnControl* self, double value)",
        "*set_is_visible)(IAvnControl* self, int32_t value)",
        "*set_opacity)(IAvnControl* self, double value)",
        "*set_name)(IAvnStyledElement* self, const uint16_t* value)",
        "*set_padding)(IAvnDecorator* self, AvnThickness value)",
        "*set_can_resize)(IAvnWindow* self, int32_t value)",
        "*set_window_state)(IAvnWindow* self, int32_t value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn thickness_crosses_the_abi_by_value() {
    // A by-value slot only works because the struct is blittable and 32 bytes wide.
    let value = AvnThickness {
        left: 1.0,
        top: 2.0,
        right: 3.0,
        bottom: 4.0,
    };
    let slot: unsafe extern "system" fn(*mut u8, AvnThickness) -> i32 = passthrough;
    assert_eq!(unsafe { slot(std::ptr::null_mut(), value) }, 10);
}

unsafe extern "system" fn passthrough(_self: *mut u8, value: AvnThickness) -> i32 {
    (value.left + value.top + value.right + value.bottom) as i32
}

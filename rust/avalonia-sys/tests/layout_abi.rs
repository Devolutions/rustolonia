//! ABI guarantees for the layout members published on the widened control vtables.
//!
//! Layout slots were added to `IAvnStyledElement` and `IAvnControl`, whose flattened
//! vtables every projected control inherits, so this pins the generated header, the
//! generated IIDs, and the by-value `AvnThickness` calling convention together.

use avalonia_sys::{
    AvnThickness, I_AVN_AVALONIA_OBJECT_IID, I_AVN_BUTTON_IID, I_AVN_CONTROL_IID,
    I_AVN_DECORATOR_IID, I_AVN_STYLED_ELEMENT_IID, I_AVN_WINDOW_IID,
};

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
fn widened_interfaces_publish_their_layout_abi_version() {
    for expected in [
        // Nothing was added to StyledElement, Control or Decorator after the layout wave, so
        // their flattened vtables — and therefore their version 3 IIDs — still stand.
        "#define I_AVN_STYLED_ELEMENT_ABI_VERSION 6",
        "#define I_AVN_CONTROL_ABI_VERSION 8",
        "#define I_AVN_DECORATOR_ABI_VERSION 10",
        // Window and Button sit under ContentControl, which the completeness wave widened,
        // and the window overlay-chrome pass widened the leaf to 8 without disturbing
        // the content control or factory ABI.
        "#define I_AVN_WINDOW_ABI_VERSION 18",
        "#define I_AVN_BUTTON_ABI_VERSION 16",
        // AvaloniaObject projects no members, so its vtable never moved.
        "#define I_AVN_AVALONIA_OBJECT_ABI_VERSION 2",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn widened_interfaces_republish_under_fresh_iids() {
    // The version 2 IIDs are retired: reusing one for a longer vtable would let a stale
    // consumer call through slots the old contract never declared.
    for (name, retired, current) in [
        (
            "IAvnStyledElement",
            "FFC4634F-D15C-5549-B81E-4A6385E18383",
            I_AVN_STYLED_ELEMENT_IID,
        ),
        (
            "IAvnControl",
            "152B5D1F-7E08-56E0-B100-F779917510D3",
            I_AVN_CONTROL_IID,
        ),
        (
            "IAvnDecorator",
            "B893E635-46EE-58CC-A3A3-7522C6336C3F",
            I_AVN_DECORATOR_IID,
        ),
        (
            "IAvnWindow",
            "0EF99637-5CA4-5C5D-ACB7-546CBA814DD9",
            I_AVN_WINDOW_IID,
        ),
        (
            "IAvnButton",
            "476007AA-FC01-5943-A763-41589B33B7A7",
            I_AVN_BUTTON_IID,
        ),
    ] {
        assert_ne!(format_iid(&current), retired, "{name} reused a retired IID");
    }

    // The layout wave's own IIDs still stand for the interfaces it widened and nothing has
    // widened since.
    assert_eq!(
        format_iid(&I_AVN_STYLED_ELEMENT_IID),
        "383D0620-1A23-575B-B609-391608903CB1"
    );
    assert_eq!(
        format_iid(&I_AVN_CONTROL_IID),
        "06D79016-63D8-5035-B293-19969F0BB3C6"
    );
    assert_eq!(
        format_iid(&I_AVN_DECORATOR_IID),
        "57E09C6B-A22C-5F01-B65C-E54C3DD55E6C"
    );

    // AvaloniaObject gained nothing, so it keeps the IID it published at version 2.
    assert_eq!(
        format_iid(&I_AVN_AVALONIA_OBJECT_IID),
        "FA7F2E03-0BFA-5422-840B-18AE1D9695C0"
    );
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

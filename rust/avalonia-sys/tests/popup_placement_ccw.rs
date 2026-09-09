//! Round-trip tests for the Rust-side popup placement CCW: the host passes the
//! popup geometry through the invoke slots and reads the mutated placement
//! back through the out-parameters.

use avalonia_sys::popup_placement;

#[test]
fn popup_placement_round_trips_the_geometry() {
    let handle = popup_placement(|width, height, x, y, w, h| {
        assert_eq!(
            (width, height, x, y, w, h),
            (100.0, 50.0, 10.0, 20.0, 30.0, 40.0)
        );
        Ok(avalonia_sys::PopupPlacementResult {
            offset_x: 12.5,
            offset_y: -7.25,
            anchor: 4,
            gravity: 2,
            constraint_adjustment: 0,
        })
    });

    let ptr = handle.as_com_ptr();
    let placement = ptr
        .invoke(100.0, 50.0, 10.0, 20.0, 30.0, 40.0)
        .expect("invoke failed");
    assert_eq!(placement.offset_x, 12.5);
    assert_eq!(placement.offset_y, -7.25);
    assert_eq!(placement.anchor, 4);
    assert_eq!(placement.gravity, 2);
}

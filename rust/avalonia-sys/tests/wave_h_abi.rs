//! ABI guarantees for the wave H shape primitives.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn shapes_carry_brushes_points_and_path_data() {
    for expected in [
        "*set_fill)(IAvnShape* self, IAvnBrush* value)",
        "*set_stroke)(IAvnShape* self, IAvnBrush* value)",
        "*set_stroke_thickness)(IAvnShape* self, double value)",
        "*set_start_point)(IAvnLine* self, AvnPoint value)",
        "*set_data)(IAvnPath* self, const uint16_t* value)",
        "*set_radius_x)(IAvnRectangle* self, double value)",
        "*set_start_angle)(IAvnArc* self, double value)",
        "*create_rectangle)(IAvnControlFactory* self, IAvnRectangle** value)",
        "*create_ellipse)(IAvnControlFactory* self, IAvnEllipse** value)",
        "*create_line)(IAvnControlFactory* self, IAvnLine** value)",
        "*create_path)(IAvnControlFactory* self, IAvnPath** value)",
        "#define I_AVN_SHAPE_ABI_VERSION 8",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
    assert!(
        !HEADER.contains("*create_shape)"),
        "abstract Shape must not have a factory creator"
    );
    // U22: Points crosses as a UTF-16 "x,y x,y" list through the
    // AvnPointList converter rather than a projected point collection.
    for expected in [
        "*set_points)(IAvnPolygon* self, const uint16_t* value)",
        "*set_points)(IAvnPolyline* self, const uint16_t* value)",
        "*set_stroke_dash_array)(IAvnShape* self, const uint16_t* value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

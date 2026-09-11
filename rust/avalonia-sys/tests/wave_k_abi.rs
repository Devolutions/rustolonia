//! ABI guarantees for the wave K icons and TableView.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn icons_and_table_view_publish_at_version_one() {
    for expected in [
        "*set_data)(IAvnPathIcon* self, const uint16_t* value)",
        "*set_can_user_resize_columns)(IAvnTableView* self, int32_t value)",
        "*set_width)(IAvnTableViewColumn* self, const uint16_t* value)",
        "*create_path_icon)(IAvnControlFactory* self, IAvnPathIcon** value)",
        "*create_table_view)(IAvnControlFactory* self, IAvnTableView** value)",
        "*create_table_view_column)(IAvnControlFactory* self, IAvnTableViewColumn** value)",
        "#define I_AVN_PATH_ICON_ABI_VERSION 11",
        "#define I_AVN_TABLE_VIEW_ABI_VERSION 15",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
    assert!(
        !HEADER.contains("*create_icon_element)"),
        "abstract IconElement must not have a factory creator"
    );
}

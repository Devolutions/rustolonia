//! ABI guarantees for `Grid`'s column and row definitions.
//!
//! The definitions cross as the comma-separated length list that `ColumnDefinitions` and
//! `RowDefinitions` already parse and print, so they occupy ordinary UTF-16 string slots
//! rather than a projected collection of definition objects.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn definitions_are_published_as_utf16_string_slots_on_grid() {
    for expected in [
        "*get_column_definitions)(IAvnGrid* self, uint16_t** value)",
        "*set_column_definitions)(IAvnGrid* self, const uint16_t* value)",
        "*get_row_definitions)(IAvnGrid* self, uint16_t** value)",
        "*set_row_definitions)(IAvnGrid* self, const uint16_t* value)",
        "*set_column_spacing)(IAvnGrid* self, double value); /* slot 93 */",
        "*get_column_definitions)(IAvnGrid* self, uint16_t** value); /* slot 94 */",
        "#define I_AVN_GRID_VTABLE_SLOTS 103",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }

    #[test]
    fn grid_publishes_layout_and_child_collection_notifications() {
        for expected in [
            "*get_is_effectively_visible)(IAvnGrid* self, int32_t* value); /* slot 87 */",
            "*update_layout)(IAvnGrid* self); /* slot 98 */",
            "*advise_layout_updated)(IAvnGrid* self, IAvnGridLayoutUpdatedHandler* handler, int64_t* subscription_id); /* slot 99 */",
            "*unadvise_layout_updated)(IAvnGrid* self, int64_t subscription_id); /* slot 100 */",
            "*advise_children_changed)(IAvnGrid* self, IAvnGridChildrenChangedHandler* handler, int64_t* subscription_id); /* slot 101 */",
            "*unadvise_children_changed)(IAvnGrid* self, int64_t subscription_id); /* slot 102 */",
        ] {
            assert!(HEADER.contains(expected), "header is missing `{expected}`");
        }
    }

    // A definition list is a string, not a projected collection: no per-definition interface
    // is minted, so there is nothing here with IBrush-level reference-graph complexity.
    for forbidden in [
        "IAvnColumnDefinition",
        "IAvnRowDefinition",
        "IAvnColumnDefinitionList",
        "IAvnRowDefinitionList",
    ] {
        assert!(
            !HEADER.contains(forbidden),
            "header must not declare `{forbidden}`"
        );
    }
}

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
        // They are appended after the members Grid already published, so no existing slot moved.
        // (Waves P and Q grew Control above these slots.)
        "*set_column_spacing)(IAvnGrid* self, double value); /* slot 92 */",
        "*get_column_definitions)(IAvnGrid* self, uint16_t** value); /* slot 93 */",
        "#define I_AVN_GRID_VTABLE_SLOTS 97",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
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

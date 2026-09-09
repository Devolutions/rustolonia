//! ABI guarantees for the wave E input controls.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn numeric_up_down_decimals_are_utf16_strings() {
    for expected in [
        "*get_value)(IAvnNumericUpDown* self, uint16_t** value)",
        "*set_value)(IAvnNumericUpDown* self, const uint16_t* value)",
        "*set_minimum)(IAvnNumericUpDown* self, const uint16_t* value)",
        "*set_increment)(IAvnNumericUpDown* self, const uint16_t* value)",
        "*create_numeric_up_down)(IAvnControlFactory* self, IAvnNumericUpDown** value)",
        "#define I_AVN_NUMERIC_UP_DOWN_ABI_VERSION 10",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

#[test]
fn remaining_input_controls_publish_at_version_one() {
    for expected in [
        "*set_mask)(IAvnMaskedTextBox* self, const uint16_t* value)",
        "*set_selection_start)(IAvnSelectableTextBlock* self, int32_t value)",
        "*get_selected_text)(IAvnSelectableTextBlock* self, uint16_t** value)",
        "*set_filter_mode)(IAvnAutoCompleteBox* self, int32_t value)",
        "*set_allow_spin)(IAvnButtonSpinner* self, int32_t value)",
        "*create_button_spinner)(IAvnControlFactory* self, IAvnButtonSpinner** value)",
        "*create_auto_complete_box)(IAvnControlFactory* self, IAvnAutoCompleteBox** value)",
        "#define I_AVN_SPINNER_ABI_VERSION 9",
        "#define I_AVN_MASKED_TEXT_BOX_ABI_VERSION 15",
        "#define I_AVN_SELECTABLE_TEXT_BLOCK_ABI_VERSION 11",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
    assert!(
        !HEADER.contains("*create_spinner)"),
        "abstract Spinner must not have a factory creator"
    );
}

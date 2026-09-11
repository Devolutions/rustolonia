//! ABI guarantees for the wave J CommandBar family.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn command_bar_family_publishes_at_version_one() {
    for expected in [
        "*set_is_open)(IAvnCommandBar* self, int32_t value)",
        "*set_label)(IAvnCommandBarButton* self, const uint16_t* value)",
        "*set_number_of_pages)(IAvnPipsPager* self, int32_t value)",
        "*set_selected_page_index)(IAvnPipsPager* self, int32_t value)",
        "*create_command_bar)(IAvnControlFactory* self, IAvnCommandBar** value)",
        "*set_primary_commands)(IAvnCommandBar* self, IAvnControlList* value)",
        "*create_pips_pager)(IAvnControlFactory* self, IAvnPipsPager** value)",
        "*create_theme_variant_scope)(IAvnControlFactory* self, IAvnThemeVariantScope** value)",
        "#define I_AVN_COMMAND_BAR_ABI_VERSION 15",
        "#define I_AVN_PIPS_PAGER_ABI_VERSION 13",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

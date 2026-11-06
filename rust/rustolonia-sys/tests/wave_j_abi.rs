//! ABI guarantees for the wave J CommandBar family.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn command_bar_family_publishes_their_members() {
    for expected in [
        "*set_is_open)(IAvnCommandBar* self, int32_t value)",
        "*set_label)(IAvnCommandBarButton* self, const uint16_t* value)",
        "*set_number_of_pages)(IAvnPipsPager* self, int32_t value)",
        "*set_selected_page_index)(IAvnPipsPager* self, int32_t value)",
        "*create_command_bar)(IAvnControlFactory* self, IAvnCommandBar** value)",
        "*set_primary_commands)(IAvnCommandBar* self, IAvnControlList* value)",
        "*create_pips_pager)(IAvnControlFactory* self, IAvnPipsPager** value)",
        "*create_theme_variant_scope)(IAvnControlFactory* self, IAvnThemeVariantScope** value)",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}

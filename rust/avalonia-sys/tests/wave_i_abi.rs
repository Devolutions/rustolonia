//! ABI guarantees for the wave I overlay and notification controls.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn overlay_controls_publish_at_version_one() {
    for expected in [
        "*set_child)(IAvnPopup* self, IAvnControl* value)",
        "*set_is_open)(IAvnPopup* self, int32_t value)",
        "*set_placement_target)(IAvnPopup* self, IAvnControl* value)",
        "*set_placement_anchor)(IAvnPopup* self, int32_t value)",
        "*set_placement_rect)(IAvnPopup* self, AvnOptionalRect value)",
        "*open)(IAvnPopup* self)",
        "*close)(IAvnPopup* self)",
        "*set_tool_tip_text)(IAvnTrayIcon* self, const uint16_t* value)",
        "*set_command)(IAvnTrayIcon* self, IAvnCommand* value)",
        "*set_command_parameter)(IAvnTrayIcon* self, AvnVariant value)",
        "*set_icon)(IAvnTrayIcon* self, const uint16_t* value)",
        "*set_max_items)(IAvnWindowNotificationManager* self, int32_t value)",
        "*set_pull_direction)(IAvnRefreshContainer* self, int32_t value)",
        "*create_popup)(IAvnControlFactory* self, IAvnPopup** value)",
        "*create_tray_icon)(IAvnControlFactory* self, IAvnTrayIcon** value)",
        "#define I_AVN_POPUP_ABI_VERSION 11",
        "#define I_AVN_TRAY_ICON_ABI_VERSION 3",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
    for forbidden in ["*set_menu)(IAvnTrayIcon"] {
        assert!(
            !HEADER.contains(forbidden),
            "header must not declare `{forbidden}`"
        );
    }
}

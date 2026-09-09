//! ABI guarantees for the wave G content chrome.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn content_chrome_publishes_at_version_one() {
    for expected in [
        "*set_is_swipe_enabled)(IAvnCarousel* self, int32_t value)",
        "*set_viewport_fraction)(IAvnCarousel* self, double value)",
        "*get_is_swiping)(IAvnCarousel* self, int32_t* value)",
        "*set_is_transition_reversed)(IAvnTransitioningContentControl* self, int32_t value)",
        "*set_use_render_transform)(IAvnLayoutTransformControl* self, int32_t value)",
        "*create_carousel)(IAvnControlFactory* self, IAvnCarousel** value)",
        "*set_target)(IAvnLabel* self, IAvnControl* value)",
        "*create_label)(IAvnControlFactory* self, IAvnLabel** value)",
        "#define I_AVN_LABEL_ABI_VERSION 10",
        "*create_separator)(IAvnControlFactory* self, IAvnSeparator** value)",
        "*create_group_box)(IAvnControlFactory* self, IAvnGroupBox** value)",
        "*create_user_control)(IAvnControlFactory* self, IAvnUserControl** value)",
        "#define I_AVN_CAROUSEL_ABI_VERSION 13",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
    for forbidden in ["set_page_transition", "set_layout_transform"] {
        assert!(
            !HEADER.contains(forbidden),
            "header must not declare `{forbidden}`"
        );
    }
}

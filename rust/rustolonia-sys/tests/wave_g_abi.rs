//! ABI guarantees for the wave G content chrome.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn content_chrome_publishes_their_members() {
    for expected in [
        "*set_is_swipe_enabled)(IAvnCarousel* self, int32_t value)",
        "*set_viewport_fraction)(IAvnCarousel* self, double value)",
        "*get_is_swiping)(IAvnCarousel* self, int32_t* value)",
        "*set_is_transition_reversed)(IAvnTransitioningContentControl* self, int32_t value)",
        "*set_use_render_transform)(IAvnLayoutTransformControl* self, int32_t value)",
        "*create_carousel)(IAvnControlFactory* self, IAvnCarousel** value)",
        "*set_target)(IAvnLabel* self, IAvnControl* value)",
        "*create_label)(IAvnControlFactory* self, IAvnLabel** value)",
        "*create_separator)(IAvnControlFactory* self, IAvnSeparator** value)",
        "*create_group_box)(IAvnControlFactory* self, IAvnGroupBox** value)",
        "*create_user_control)(IAvnControlFactory* self, IAvnUserControl** value)",
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

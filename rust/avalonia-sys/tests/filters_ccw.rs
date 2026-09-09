//! Round-trip tests for the Rust-side AutoCompleteBox filter CCWs: the item
//! filter receives the item as a variant (freed after the call) and the text
//! filter receives both arguments as borrowed UTF-16 buffers.

use avalonia_sys::{item_filter, text_filter, AvnVariant};

#[test]
fn item_filter_invokes_the_closure() {
    let handle =
        item_filter(|_search: *const u16, item: AvnVariant| Ok(item.tag == AvnVariant::TAG_UTF16));

    let ptr = handle.as_com_ptr();
    let empty: [u16; 1] = [0];
    let text = AvnVariant {
        tag: AvnVariant::TAG_UTF16,
        utf16: empty.as_ptr() as *mut u16,
        ..Default::default()
    };
    assert!(ptr.invoke(empty.as_ptr(), text).expect("invoke failed"));
    assert!(!ptr
        .invoke(empty.as_ptr(), AvnVariant::default())
        .expect("invoke none failed"));
}

#[test]
fn text_filter_invokes_the_closure() {
    let handle = text_filter(|_search: *const u16, item: *const u16| Ok(item.is_null()));

    let ptr = handle.as_com_ptr();
    let empty: [u16; 1] = [0];
    assert!(ptr
        .invoke(empty.as_ptr(), std::ptr::null())
        .expect("invoke failed"));
    assert!(!ptr
        .invoke(empty.as_ptr(), empty.as_ptr())
        .expect("invoke item failed"));
}

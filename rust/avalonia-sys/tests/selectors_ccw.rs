//! Round-trip tests for the Rust-side AutoCompleteBox selector CCWs: the
//! item selector receives the item as a variant (freed after the call) and
//! both selectors return a host-allocated UTF-16 buffer, so without a loaded
//! host they report E_NOTIMPL on the allocation step.

use avalonia_sys::{item_selector, text_selector, AvnVariant};

#[test]
fn item_selector_reports_no_host_without_a_session() {
    let handle = item_selector(|_search: *const u16, _item: AvnVariant| Ok("text".to_string()));

    let ptr = handle.as_com_ptr();
    let empty: [u16; 1] = [0];
    // No host session means no allocator the host could free from, so the
    // string out-parameter cannot be filled.
    assert!(ptr.invoke(empty.as_ptr(), AvnVariant::default()).is_err());
}

#[test]
fn text_selector_reports_no_host_without_a_session() {
    let handle = text_selector(|_search: *const u16, _item: *const u16| Ok("text".to_string()));

    let ptr = handle.as_com_ptr();
    let empty: [u16; 1] = [0];
    assert!(ptr.invoke(empty.as_ptr(), empty.as_ptr()).is_err());
}

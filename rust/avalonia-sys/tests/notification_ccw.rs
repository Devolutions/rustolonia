//! Round-trip tests for the Rust-side `IAvnNotification` CCW: the getter slots
//! the host reads (Title, Message, Type, Expiration) round-trip through the
//! RCW. String getters report E_NOTIMPL without a loaded host (no allocator
//! the host could free from), so the unit test exercises the scalar slots and
//! the string error path.

use avalonia_sys::{notification, NotificationSpec};

#[test]
fn notification_round_trips_the_scalar_slots() {
    let spec = NotificationSpec {
        title: "Title".to_string(),
        message: "Message".to_string(),
        kind: 2, // Warning
        expiration_ticks: 500_0000,
        on_click: None,
        on_close: None,
    };
    let handle = notification(spec);
    let ptr = handle.as_com_ptr();

    assert_eq!(ptr.notification_type().expect("type failed"), 2);
    assert_eq!(ptr.expiration().expect("expiration failed"), 500_0000);

    // Without a host session there is no allocator for host-owned strings.
    assert!(ptr.title().is_err());
    assert!(ptr.message().is_err());
}

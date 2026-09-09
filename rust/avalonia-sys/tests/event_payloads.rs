//! U28: the nullable event payload wrappers round-trip their tick counts.
//! `AvnOptionalTimeSpan` and `AvnOptionalDateTime` carry the old/new pairs of
//! the picker value-change events across the ABI.

use avalonia_sys::{AvnOptionalDateTime, AvnOptionalTimeSpan};

#[test]
fn optional_time_span_round_trips_a_duration() {
    let span = AvnOptionalTimeSpan::from_duration(Some(core::time::Duration::from_secs(90)));
    assert_eq!(span.has_value, 1);
    assert_eq!(span.ticks, 900_000_000);

    let back = span.to_duration().expect("value lost");
    assert_eq!(back, core::time::Duration::from_secs(90));

    // The absent state round-trips too.
    let empty = AvnOptionalTimeSpan::from_duration(None);
    assert_eq!(empty.has_value, 0);
    assert!(empty.to_duration().is_none());
}

#[test]
fn optional_date_time_round_trips_a_system_time() {
    let now = std::time::SystemTime::now();
    let date = AvnOptionalDateTime::from_date_time(Some(now));
    assert_eq!(date.has_value, 1);

    let back = date.to_date_time().expect("value lost");
    let delta = match now.duration_since(back) {
        Ok(delta) => delta,
        Err(error) => error.duration(),
    };
    assert!(delta.as_millis() < 1, "round-trip drifted {delta:?}");

    let empty = AvnOptionalDateTime::from_date_time(None);
    assert_eq!(empty.has_value, 0);
    assert!(empty.to_date_time().is_none());
}

//! Round-trip tests for the Rust-side `IAvnDataTemplate` CCW: Match receives
//! the item as a variant, freed after the call; Build hands back a control
//! (the host supplies the factory, so this exercises the bookkeeping).

use avalonia_sys::{data_template, AvnVariant};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;

#[test]
fn data_template_matches_items() {
    let matches = Arc::new(AtomicI32::new(0));
    let builds = Arc::new(AtomicBool::new(false));

    let matches_for_template = Arc::clone(&matches);
    let builds_for_template = Arc::clone(&builds);
    let handle = data_template(
        move |data: AvnVariant| {
            matches_for_template.fetch_add(1, Ordering::SeqCst);
            Ok(data.tag == AvnVariant::TAG_NONE)
        },
        move || {
            builds_for_template.store(true, Ordering::SeqCst);
            Err(avalonia_sys::Error(avalonia_sys::E_POINTER))
        },
    );

    let ptr = handle.as_com_ptr();
    assert!(ptr
        .matches(AvnVariant::default())
        .expect("match none failed"));
    assert_eq!(matches.load(Ordering::SeqCst), 1);
    assert!(ptr.build().is_err());
    assert!(builds.load(Ordering::SeqCst));
}

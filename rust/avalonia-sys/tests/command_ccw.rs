//! Round-trip tests for the Rust-side `IAvnCommand` CCW: closures behind
//! execute/can_execute, and advise/notify/unadvise against a handler
//! implemented in Rust, without the NativeAOT host in the loop.

use avalonia_sys::{command, AvnVariant};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;

#[test]
fn command_executes_closures_and_reports_can_execute() {
    let executed = Arc::new(AtomicI32::new(0));
    let can_run = Arc::new(AtomicBool::new(true));

    let executed_for_command = Arc::clone(&executed);
    let can_run_for_command = Arc::clone(&can_run);
    let handle = command(
        move |parameter: AvnVariant| {
            assert_eq!(parameter.tag, AvnVariant::TAG_I32);
            assert_eq!(parameter.i32, 42);
            executed_for_command.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        move |parameter: AvnVariant| {
            assert_eq!(parameter.tag, AvnVariant::TAG_I32);
            Ok(can_run_for_command.load(Ordering::SeqCst))
        },
    );

    let ptr = handle.as_com_ptr();
    let parameter = AvnVariant {
        tag: AvnVariant::TAG_I32,
        i32: 42,
        ..Default::default()
    };
    assert!(ptr.can_execute(parameter).expect("can_execute failed"));
    ptr.execute(parameter).expect("execute failed");
    assert_eq!(executed.load(Ordering::SeqCst), 1);

    can_run.store(false, Ordering::SeqCst);
    assert!(!ptr.can_execute(parameter).expect("can_execute failed"));
}

#[test]
fn command_advise_notify_unadvise_round_trip() {
    let notifications = Arc::new(AtomicI32::new(0));

    let notifications_for_handler = Arc::clone(&notifications);
    let handler = avalonia_sys::command_can_execute_changed_handler(move || {
        notifications_for_handler.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });

    let handle = command(|_: AvnVariant| Ok(()), |_: AvnVariant| Ok(true));
    let ptr = handle.as_com_ptr();

    let subscription = ptr
        .advise_can_execute_changed(&handler)
        .expect("advise failed");
    assert_eq!(notifications.load(Ordering::SeqCst), 0);

    handle.notify();
    assert_eq!(notifications.load(Ordering::SeqCst), 1);

    handle.notify();
    assert_eq!(notifications.load(Ordering::SeqCst), 2);

    ptr.unadvise_can_execute_changed(subscription)
        .expect("unadvise failed");
    handle.notify();
    assert_eq!(notifications.load(Ordering::SeqCst), 2);
}

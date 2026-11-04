use rustolonia::{
    App, AppScope, Button, HorizontalAlignment, HostExports, Orientation, Result,
    SingleViewSession, StackPanel, TextBlock, Thickness, VerticalAlignment,
};
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

const E_FAIL: i32 = 0x8000_4005u32 as i32;
const E_POINTER: i32 = 0x8000_4003u32 as i32;

thread_local! {
    static SESSION: RefCell<Option<SingleViewSession>> = const { RefCell::new(None) };
}

fn build_view(scope: &AppScope) -> Result<StackPanel> {
    let count = Arc::new(AtomicU32::new(0));
    let label = TextBlock::new()?
        .text("Preparing...")?
        .font_size(28.0)?
        .horizontal_alignment(HorizontalAlignment::Center)?;
    let button = Button::new()?
        .content(Some(&TextBlock::new()?.text("Tap me")?))?
        .horizontal_alignment(HorizontalAlignment::Center)?;
    let button_handle = button.clone();
    let button = button.on_click(scope, {
        let label = label.clone();
        let count = count.clone();
        move |()| {
            let value = count.fetch_add(1, Ordering::Relaxed) + 1;
            let update = label.set_text(format!("Taps: {value}")).and_then(|_| {
                let caption = TextBlock::new()?.text("Tap again")?;
                button_handle.set_content(Some(&caption))
            });
            if let Err(error) = update {
                eprintln!("failed to update the counter: {error}");
            }
        }
    })?;
    let button = button.on_key_down(scope, {
        let label = label.clone();
        move |event| {
            if matches!(event.key_symbol.as_deref(), Some("r" | "R")) {
                count.store(0, Ordering::Relaxed);
                event.handled = true;
                if let Err(error) = label.set_text("Taps: 0") {
                    eprintln!("failed to reset the counter: {error}");
                }
            }
        }
    })?;

    let ready_label = label.clone();
    let view = StackPanel::new()?
        .orientation(Orientation::Vertical)?
        .horizontal_alignment(HorizontalAlignment::Center)?
        .vertical_alignment(VerticalAlignment::Center)?
        .margin(Thickness::uniform(24.0))?
        .child(
            TextBlock::new()?
                .text("Hello from Rustolonia on iOS")?
                .font_size(20.0)?,
        )?
        .child(label)?
        .child(button)?
        .child(
            TextBlock::new()?
                .text("Hardware keyboard: R resets")?
                .font_size(14.0)?
                .horizontal_alignment(HorizontalAlignment::Center)?,
        )?;
    scope.post(move || {
        if let Err(error) = ready_label.set_text("Taps: 0") {
            eprintln!("failed to initialize the counter: {error}");
        }
    })?;
    Ok(view)
}

#[cfg(feature = "smoke")]
fn build_smoke_view(scope: &AppScope, exports: &HostExports) -> Result<StackPanel> {
    let sentinel = rustolonia::Signal::new(17);
    let desktop = unsafe { App::from_in_process(exports)? };
    assert!(matches!(
        desktop.run(|_| Ok(())),
        Err(rustolonia::Error::Abi(error)) if error.0 == 0x8000_4001u32 as i32
    ));
    assert_eq!(sentinel.get(), 17);

    let duplicate = unsafe { App::from_in_process(exports)? };
    assert!(matches!(
        duplicate.start_single_view(|_| TextBlock::new()),
        Err(rustolonia::Error::UiContextAlreadyActive)
    ));
    assert_eq!(sentinel.get(), 17);

    let exports = *exports;
    let off_thread = std::thread::spawn(move || {
        let app = unsafe { App::from_in_process(&exports)? };
        app.start_single_view(|_| TextBlock::new()).map(|_| ())
    })
    .join()
    .expect("the off-thread startup check panicked");
    assert!(matches!(off_thread, Err(rustolonia::Error::NoUiContext)));
    build_view(scope)
}

/// Called once by the iOS host after Avalonia has initialized on the UI thread.
///
/// # Safety
///
/// `exports` must point to a complete, valid host export table whose functions
/// remain callable until the process exits.
#[no_mangle]
pub unsafe extern "C" fn rustolonia_ios_main(exports: *const HostExports) -> i32 {
    if exports.is_null() {
        return E_POINTER;
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        SESSION.with(|session| {
            if session.borrow().is_some() {
                return Err(rustolonia::Error::UiContextAlreadyActive);
            }
            let app = App::from_in_process(&*exports)?;
            #[cfg(feature = "smoke")]
            let running = app.start_single_view(|scope| build_smoke_view(scope, &*exports))?;
            #[cfg(not(feature = "smoke"))]
            let running = app.start_single_view(build_view)?;
            *session.borrow_mut() = Some(running);
            Ok(())
        })
    }));
    match result {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            eprintln!("failed to start the iOS application: {error}");
            E_FAIL
        }
        Err(_) => {
            eprintln!("the iOS application panicked during startup");
            E_FAIL
        }
    }
}

/// Called by the iOS app delegate on the UI thread during graceful termination.
#[no_mangle]
pub extern "C" fn rustolonia_ios_stop() -> i32 {
    match catch_unwind(AssertUnwindSafe(|| {
        SESSION.with(|session| match session.borrow_mut().take() {
            Some(running) => running.close(),
            None => Ok(()),
        })
    })) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            eprintln!("failed to stop the iOS application: {error}");
            E_FAIL
        }
        Err(_) => {
            eprintln!("the iOS application panicked during shutdown");
            E_FAIL
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_rejects_null_exports_and_shutdown_is_idempotent() {
        assert_eq!(unsafe { rustolonia_ios_main(std::ptr::null()) }, E_POINTER);
        assert_eq!(rustolonia_ios_stop(), 0);
        assert_eq!(rustolonia_ios_stop(), 0);
    }
}

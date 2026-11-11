#[cfg(feature = "smoke")]
use rustolonia::{App, HostExports};
use rustolonia::{
    rsx, AppScope, Button, HorizontalAlignment, Key, Orientation, Result, StackPanel, TextBlock,
    VerticalAlignment,
};

fn build_view(scope: &AppScope) -> Result<StackPanel> {
    let count = scope.signal(0_u32);
    let status = scope.signal(String::from("Preparing..."));
    let caption = scope.signal(String::from("Tap me"));

    let view = rsx! {
        StackPanel {
            orientation: Orientation::Vertical,
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Center,
            margin: 24.0,
            TextBlock {
                font_size: 20.0,
                "Hello from Rustolonia on iOS"
            }
            TextBlock {
                font_size: 28.0,
                horizontal_alignment: HorizontalAlignment::Center,
                text: status,
            }
            Button {
                horizontal_alignment: HorizontalAlignment::Center,
                on_click: move |_| {
                    count.update(|c| *c += 1);
                    status.set(format!("Taps: {}", count.get_untracked()));
                    caption.set(String::from("Tap again"));
                },
                on_key_down: move |event| {
                    if event.key == Key::R || matches!(event.key_symbol.as_deref(), Some("r" | "R")) {
                        count.set(0);
                        event.handled = true;
                        status.set(String::from("Taps: 0"));
                    }
                },
                TextBlock { text: caption }
            }
            TextBlock {
                font_size: 14.0,
                horizontal_alignment: HorizontalAlignment::Center,
                "Hardware keyboard: R resets"
            }
        }
    }
    .build(scope)?;
    scope.post({
        let writer = status.writer(scope);
        move || {
            let _ = writer.set(String::from("Taps: 0"));
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

#[cfg(not(feature = "smoke"))]
rustolonia::ios_application!(build_view);

#[cfg(feature = "smoke")]
/// # Safety
///
/// The iOS host must pass a valid export table on its UI thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustolonia_ios_main(exports: *const HostExports) -> i32 {
    unsafe { rustolonia::ios::start(exports, |scope| build_smoke_view(scope, &*exports)) }
}

#[cfg(feature = "smoke")]
#[unsafe(no_mangle)]
pub extern "C" fn rustolonia_ios_stop() -> i32 {
    rustolonia::ios::stop()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_rejects_null_exports_and_shutdown_is_idempotent() {
        assert_eq!(
            unsafe { rustolonia_ios_main(std::ptr::null()) },
            0x8000_4003u32 as i32
        );
        assert_eq!(rustolonia_ios_stop(), 0);
        assert_eq!(rustolonia_ios_stop(), 0);
    }
}

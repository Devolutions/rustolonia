#[cfg(target_os = "macos")]
fn main() {
    use rustolonia::{App, Button, MenuFlyout, MenuItem, Window};
    let host = rustolonia_sys::Host::load(rustolonia::discover_host_path().unwrap()).unwrap();
    let result = App::load_from_env().unwrap().run(|scope| {
        let protected = Button::new()?.tool_tip_tip("Protected tooltip")?;
        let normal = Button::new()?.tool_tip_tip("Normal tooltip")?;
        let window = Window::new()?.title("Protected")?.content(Some(&protected))?;
        scope.set_overlay_popups(&window, true)?;
        scope.mount(window)?;
        scope.mount(Window::new()?.title("Normal")?.content(Some(&normal))?)?;
        let baseline = visible_windows();
        for (target, overlay) in [(&protected, true), (&normal, false)] {
            target.set_tool_tip_is_open(true)?;
            assert!(target.get_tool_tip_is_open()?);
            let tooltip_windows = visible_windows();
            assert_eq!(tooltip_windows == baseline, overlay, "tooltip native-window count");
            target.set_tool_tip_is_open(false)?;
            let flyout = MenuFlyout::new()?;
            flyout.items()?.add(
                MenuItem::new()?.header(Some(&rustolonia::TextBlock::new()?.text("Menu entry")?))?,
            )?;
            flyout.show_at_with_control(target)?;
            assert!(flyout.get_is_open()?);
            let flyout_windows = visible_windows();
            assert_eq!(flyout_windows == baseline, overlay, "MenuFlyout native-window count");
            flyout.hide()?;
            println!("overlay={overlay}: baseline={baseline}, tooltip={tooltip_windows}, flyout={flyout_windows}");
        }
        let scope_to_shutdown = scope.clone();
        scope.post(move || scope_to_shutdown.shutdown().expect("shutdown popup fixture"))
    });
    if let Err(error) = result {
        eprintln!("{error}: {}", host.last_error().unwrap_or_default());
        std::process::exit(1);
    }
}

// Snapshot AppKit windows only on the process main thread; retain no native handles.
#[cfg(target_os = "macos")]
#[allow(clashing_extern_declarations)] // objc_msgSend requires each selector's exact ABI signature.
fn visible_windows() -> usize {
    use std::ffi::{c_char, c_void};
    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn send(receiver: *mut c_void, selector: *mut c_void) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn count(receiver: *mut c_void, selector: *mut c_void) -> usize;
        #[link_name = "objc_msgSend"]
        fn at(receiver: *mut c_void, selector: *mut c_void, index: usize) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn boolean(receiver: *mut c_void, selector: *mut c_void) -> bool;
    }
    unsafe {
        let app = send(
            objc_getClass(c"NSApplication".as_ptr()),
            sel_registerName(c"sharedApplication".as_ptr()),
        );
        let windows = send(app, sel_registerName(c"windows".as_ptr()));
        (0..count(windows, sel_registerName(c"count".as_ptr())))
            .filter(|index| {
                boolean(
                    at(
                        windows,
                        sel_registerName(c"objectAtIndex:".as_ptr()),
                        *index,
                    ),
                    sel_registerName(c"isVisible".as_ptr()),
                )
            })
            .count()
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("native_popup_host requires macOS");
    std::process::exit(2);
}

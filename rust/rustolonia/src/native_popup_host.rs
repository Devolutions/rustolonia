#[cfg(target_os = "macos")]
fn main() {
    use rustolonia::{App, Button, MenuFlyout, MenuItem, Window};
    let host = rustolonia_sys::Host::load(rustolonia::discover_host_path().unwrap()).unwrap();
    let result = App::load_from_env().unwrap().run(|scope| {
        if std::env::args().any(|arg| arg == "--gate") {
            return gate_probe(scope);
        }
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

#[cfg(target_os = "macos")]
fn gate_probe(scope: &rustolonia::AppScope) -> rustolonia::Result<()> {
    use rustolonia::{Button, MenuFlyout, MenuItem, TextBlock, Window};
    use rustolonia_sys::NativePopupEvent;
    let target = Button::new()?.tool_tip_tip("Gated tooltip")?;
    let window = Window::new()?.content(Some(&target))?;
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    scope.set_native_popup_gate(&window, {
        let events = events.clone();
        move |event| {
            events.lock().unwrap().push(event);
            Ok(())
        }
    })?;
    scope.mount(window)?;
    let normal = Button::new()?.tool_tip_tip("Ungated tooltip")?;
    scope.mount(Window::new()?.content(Some(&normal))?)?;
    let baseline = visible_windows();
    normal.set_tool_tip_is_open(true)?;
    assert_eq!(
        visible_windows(),
        baseline + 1,
        "normal owner is unaffected"
    );
    assert!(events.lock().unwrap().is_empty());
    normal.set_tool_tip_is_open(false)?;
    let latest = || {
        events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|event| match event {
                NativePopupEvent::Preparing {
                    token,
                    window_number,
                } => Some((*token, *window_number)),
                _ => None,
            })
            .expect("pre-show callback")
    };
    target.set_tool_tip_is_open(true)?;
    let (stale, number) = latest();
    assert!(number > 0);
    assert_eq!(
        visible_windows(),
        baseline,
        "pending tooltip must remain hidden"
    );
    target.set_tool_tip_is_open(false)?;
    assert!(
        scope.acknowledge_native_popup(stale).is_err(),
        "closed token rejected"
    );
    assert!(events
        .lock()
        .unwrap()
        .iter()
        .any(|event| matches!(event, NativePopupEvent::Closed { token } if *token == stale)));
    target.set_tool_tip_is_open(true)?;
    let (fresh, _) = latest();
    assert_ne!(fresh, stale, "recreated popup has fresh token");
    scope.acknowledge_native_popup(fresh)?;
    assert_eq!(
        visible_windows(),
        baseline + 1,
        "acknowledged tooltip retains native placement"
    );
    assert!(
        scope.acknowledge_native_popup(fresh).is_err(),
        "duplicate rejected"
    );
    target.set_tool_tip_is_open(false)?;
    let flyout = MenuFlyout::new()?;
    flyout
        .items()?
        .add(MenuItem::new()?.header(Some(&TextBlock::new()?.text("Gated menu")?))?)?;
    flyout.show_at_with_control(&target)?;
    assert_eq!(
        visible_windows(),
        baseline,
        "pending menu must remain hidden"
    );
    let (menu_token, _) = latest();
    flyout.hide()?;
    assert!(
        scope.acknowledge_native_popup(menu_token).is_err(),
        "closed menu token rejected"
    );
    flyout.show_at_with_control(&target)?;
    let (fresh_menu_token, _) = latest();
    assert_ne!(menu_token, fresh_menu_token);
    let scope_to_ack = scope.clone();
    scope.post(move || {
        scope_to_ack        .acknowledge_native_popup(fresh_menu_token).expect("posted asynchronous acknowledgement");
        assert_eq!(visible_windows(), baseline + 1);
        flyout.hide().expect("close acknowledged menu");
        println!("native gate: hidden before ack; native tooltip/menu after ack; stale and duplicate tokens rejected");
        scope_to_ack.shutdown().expect("shutdown gated fixture");
    })
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

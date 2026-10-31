#[cfg(target_os = "macos")]
fn main() {
    use rustolonia::{App, Button, MenuFlyout, MenuItem, Window};
    if std::env::args().any(|arg| arg == "--teardown") {
        teardown_probe().expect("auxiliary gate teardown fixture");
        return;
    }
    let host = rustolonia_sys::Host::load(rustolonia::discover_host_path().unwrap()).unwrap();
    let result = App::load_from_env().unwrap().run(|scope| {
        if std::env::args().any(|arg| arg == "--gate") {
            return gate_probe(scope);
        }
        if std::env::args().any(|arg| arg == "--placement") {
            return placement_probe(scope);
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
fn teardown_probe() -> Result<(), Box<dyn std::error::Error>> {
    use rustolonia::{AsControl, Button, MenuFlyout, MenuItem, PlacementMode, TextBlock, Window};
    use rustolonia_sys::NativePopupEvent;
    use std::sync::{Arc, Mutex};
    struct HandlerLifetime(Arc<std::sync::atomic::AtomicBool>);
    impl Drop for HandlerLifetime {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let args: Vec<_> = std::env::args().collect();
    let menu = args.iter().any(|arg| arg == "--menu");
    let revealed = args.iter().any(|arg| arg == "--revealed");
    let shutdown = args.iter().any(|arg| arg == "--shutdown");
    let automatic = args.iter().any(|arg| arg == "--automatic");
    let host = rustolonia_sys::Host::load(rustolonia::discover_host_path()?)?;
    let gate = host
        .activation_factory()?
        .create_application()?
        .popup_gate()?;
    let events = Arc::new(Mutex::new(Vec::new()));
    let owner_ref = Arc::new(Mutex::new(None));
    let app = rustolonia::App::load_from_env()?;
    app.run({
        let events = events.clone();
        let owner_ref = owner_ref.clone();
        move |scope| {
            let button = Button::new()?.tool_tip_tip("Teardown tooltip")?;
            button.set_tool_tip_placement(PlacementMode::Bottom)?;
            let owner = Window::new()?.content(Some(&button))?;
            let raw = owner.as_control()?.query_interface::<rustolonia_sys::IAvnWindow>()?;
            *owner_ref.lock().unwrap() = Some(raw.clone());
            let observed = events.clone();
            let released = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let lifetime = HandlerLifetime(released.clone());
            let handler = rustolonia_sys::native_popup_handler(move |event| {
                let _ = &lifetime;
                println!("callback: {event:?}");
                observed.lock().unwrap().push(event);
                Ok(())
            });
            gate.set(&raw, Some(&handler))?;
            drop(handler);
            if automatic {
                let gate_for_unload = gate.clone();
                let raw_for_unload = raw.clone();
                let events_for_unload = events.clone();
                let owner = owner.on_unloaded(scope, move |_| {
                    let events = events_for_unload.lock().unwrap();
                    assert!(events.iter().any(|event| matches!(event, NativePopupEvent::Closed { .. })),
                        "popup closes before owner Unloaded");
                    let clear = gate_for_unload.set(&raw_for_unload, None);
                    println!("owner Unloaded: clear={clear:?}");
                    clear.expect("clear gate during owner Unloaded");
                })?;
                scope.mount(owner.clone())?;
                scope.set_shutdown_mode(rustolonia::ShutdownMode::OnLastWindowClose)?;
                let scope = scope.clone();
                return scope.post(move || {
                    let flyout = MenuFlyout::new().unwrap();
                    flyout.items().unwrap().add(MenuItem::new().unwrap()
                        .header(Some(&TextBlock::new().unwrap().text("Entry").unwrap())).unwrap()).unwrap();
                    if menu { flyout.show_at_with_control(&button).unwrap(); }
                    else { button.set_tool_tip_is_open(true).unwrap(); }
                    let token = events.lock().unwrap().iter().find_map(|event| match event {
                        NativePopupEvent::Preparing { token, .. } => Some(*token), _ => None,
                    }).unwrap();
                    if revealed { gate.acknowledge(token).unwrap(); }
                    owner.close().unwrap();
                    println!("automatic last-window shutdown, no AppScope.shutdown call");
                });
            }
            scope.mount(owner.clone())?;
            scope.mount(Window::new()?.title("Keep dispatcher running")?)?;
            let scope = scope.clone();
            let dispatcher = scope.clone();
            dispatcher.post(move || {
                let flyout = MenuFlyout::new().expect("flyout");
                flyout.items().unwrap().add(
                    MenuItem::new().unwrap().header(Some(&TextBlock::new().unwrap().text("Entry").unwrap())).unwrap()
                ).unwrap();
                if menu {
                    flyout.show_at_with_control(&button).unwrap();
                } else {
                    button.set_tool_tip_is_open(true).unwrap();
                }
                let token = events.lock().unwrap().iter().find_map(|event| match event {
                    NativePopupEvent::Preparing { token, .. } => Some(*token),
                    _ => None,
                }).expect("preparing");
                let early_clear = gate.set(&raw, None);
                println!("clear with open popup: {early_clear:?}");
                assert!(early_clear.is_err(), "must reject clearing an open popup");
                if revealed {
                    gate.acknowledge(token).unwrap();
                }
                if shutdown {
                    scope.shutdown().unwrap();
                } else {
                    owner.close().unwrap();
                }
                let closed = events.lock().unwrap().iter().filter(|event| {
                    matches!(event, NativePopupEvent::Closed { token: value } if *value == token)
                }).count();
                let clear = gate.set(&raw, None);
                let stale = gate.acknowledge(token);
                println!("menu={menu}, revealed={revealed}, shutdown={shutdown}: closed={closed}, clear={clear:?}, stale={stale:?}");
                assert_eq!(closed, 1, "one Closed callback per popup");
                clear.expect("clear gate with closed owner projection");
                assert!(stale.is_err(), "closed token is invalid");
                drop(gate);
                drop(raw);
                drop(owner);
                drop(button);
                drop(flyout);
                println!("handler released before native-platform disposal: {}",
                    released.load(std::sync::atomic::Ordering::SeqCst));
                if !shutdown {
                    scope.shutdown().unwrap();
                }
            })
        }
    })?;
    let raw = owner_ref
        .lock()
        .unwrap()
        .take()
        .expect("owned window projection");
    println!("after App::run: events={:?}", events.lock().unwrap());
    drop(raw);
    drop(host);
    Ok(())
}

#[cfg(target_os = "macos")]
fn placement_probe(scope: &rustolonia::AppScope) -> rustolonia::Result<()> {
    use rustolonia::{Button, PlacementMode, Window};
    use rustolonia_sys::NativePopupEvent;
    let target = Button::new()?.tool_tip_tip("Placement tooltip")?;
    let window = Window::new()?
        .width(300.0)?
        .height(88.0)?
        .content(Some(&target))?;
    scope.mount(window.clone())?;
    let scope = scope.clone();
    let dispatcher = scope.clone();
    dispatcher.post(move || {
        let result = (|| -> rustolonia::Result<()> {
            let owner = scope.platform_handle(&window)?;
            let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let mut bottom_frame = None;
            for placement in [PlacementMode::Pointer, PlacementMode::Bottom] {
                target.set_tool_tip_placement(placement)?;
                let mut ungated = None;
                for gated in [false, true] {
                    if gated {
                        let events = events.clone();
                        scope.set_native_popup_gate(&window, move |event| {
                            events.lock().unwrap().push(event);
                            Ok(())
                        })?;
                    }
                    native_placement::move_owner(owner, -10000.0, -10000.0, 0.0);
                    native_placement::move_owner(owner, 400.0, 600.0, 0.0);
                    native_placement::move_owner(owner, 400.0, 600.0, 1.0);
                    let baseline = native_placement::visible_frames();
                    target.set_tool_tip_is_open(true)?;
                    if gated {
                        assert_eq!(native_placement::visible_frames().len(), baseline.len());
                        let token = events.lock().unwrap().iter().rev().find_map(|event| {
                            match event {
                                NativePopupEvent::Preparing { token, .. } => Some(*token),
                                _ => None,
                            }
                        }).expect("preparing token");
                        scope.acknowledge_native_popup(token)?;
                    }
                    let popup = native_placement::visible_frames().into_iter()
                        .find(|(number, _)| !baseline.iter().any(|(old, _)| old == number))
                        .expect("visible tooltip").1;
                    let owner_frame = native_placement::frame(owner);
                    println!("placement={placement:?}, gated={gated}: owner={owner_frame:?}, popup={popup:?}");
                    if let Some(expected) = ungated {
                        assert!(popup.approximately_eq(expected), "gate changed tooltip placement");
                    } else {
                        ungated = Some(popup);
                    }
                    if placement == PlacementMode::Bottom {
                        bottom_frame = Some(popup);
                        assert!(popup.x + popup.width >= owner_frame.x);
                        assert!(popup.x <= owner_frame.x + owner_frame.width);
                        assert!((popup.y + popup.height - owner_frame.y).abs() < 100.0,
                            "button-anchored tooltip must remain near moved owner");
                    } else {
                        assert!(popup.x < owner_frame.x - 100.0,
                            "pointer placement without pointer events is not button placement");
                    }
                    target.set_tool_tip_is_open(false)?;
                    if gated {
                        scope.clear_native_popup_gate(&window)?;
                    }
                }
            }
            let observed = events.clone();
            scope.set_native_popup_gate(&window, move |event| {
                observed.lock().unwrap().push(event);
                Ok(())
            })?;
            let baseline = native_placement::visible_frames();
            target.set_tool_tip_is_open(true)?;
            let token = events.lock().unwrap().iter().rev().find_map(|event| {
                match event {
                    NativePopupEvent::Preparing { token, .. } => Some(*token),
                    _ => None,
                }
            }).expect("posted acknowledgement token");
            let next_scope = scope.clone();
            scope.post(move || {
                next_scope.acknowledge_native_popup(token).expect("posted acknowledgement");
                let popup = native_placement::visible_frames().into_iter()
                    .find(|(number, _)| !baseline.iter().any(|(old, _)| old == number))
                    .expect("visible tooltip after posted acknowledgement").1;
                assert!(popup.approximately_eq(bottom_frame.expect("ungated bottom frame")),
                    "posted acknowledgement changed placement");
                println!("posted acknowledgement: popup={popup:?}");
                target.set_tool_tip_is_open(false).expect("close posted tooltip");
                next_scope.clear_native_popup_gate(&window).expect("clear placement gate");
                next_scope.shutdown().expect("shutdown placement fixture");
            })
        })();
        result.expect("native tooltip placement fixture");
    })
}

#[cfg(target_os = "macos")]
#[allow(clashing_extern_declarations)]
mod native_placement {
    use std::ffi::{c_char, c_void};

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct Frame {
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64,
    }

    impl Frame {
        pub fn approximately_eq(self, other: Self) -> bool {
            [
                self.x - other.x,
                self.y - other.y,
                self.width - other.width,
                self.height - other.height,
            ]
            .iter()
            .all(|difference| difference.abs() <= 1.0)
        }
    }

    #[repr(C)]
    struct Point {
        x: f64,
        y: f64,
    }

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
        #[link_name = "objc_msgSend"]
        fn set_point(receiver: *mut c_void, selector: *mut c_void, point: Point);
        #[link_name = "objc_msgSend"]
        fn set_double(receiver: *mut c_void, selector: *mut c_void, value: f64);
        #[cfg(target_arch = "aarch64")]
        #[link_name = "objc_msgSend"]
        fn get_frame(receiver: *mut c_void, selector: *mut c_void) -> Frame;
        #[cfg(target_arch = "x86_64")]
        #[link_name = "objc_msgSend_stret"]
        fn get_frame(out: *mut Frame, receiver: *mut c_void, selector: *mut c_void);
    }

    pub fn frame(handle: isize) -> Frame {
        unsafe {
            #[cfg(target_arch = "aarch64")]
            {
                get_frame(handle as *mut c_void, sel_registerName(c"frame".as_ptr()))
            }
            #[cfg(target_arch = "x86_64")]
            {
                let mut out = std::mem::MaybeUninit::uninit();
                get_frame(
                    out.as_mut_ptr(),
                    handle as *mut c_void,
                    sel_registerName(c"frame".as_ptr()),
                );
                out.assume_init()
            }
        }
    }

    pub fn move_owner(handle: isize, x: f64, y: f64, alpha: f64) {
        unsafe {
            let window = handle as *mut c_void;
            set_double(window, sel_registerName(c"setAlphaValue:".as_ptr()), alpha);
            set_point(
                window,
                sel_registerName(c"setFrameTopLeftPoint:".as_ptr()),
                Point { x, y },
            );
        }
    }

    pub fn visible_frames() -> Vec<(usize, Frame)> {
        unsafe {
            let app = send(
                objc_getClass(c"NSApplication".as_ptr()),
                sel_registerName(c"sharedApplication".as_ptr()),
            );
            let windows = send(app, sel_registerName(c"windows".as_ptr()));
            (0..count(windows, sel_registerName(c"count".as_ptr())))
                .filter_map(|index| {
                    let window = at(windows, sel_registerName(c"objectAtIndex:".as_ptr()), index);
                    boolean(window, sel_registerName(c"isVisible".as_ptr())).then(|| {
                        (
                            count(window, sel_registerName(c"windowNumber".as_ptr())),
                            frame(window as isize),
                        )
                    })
                })
                .collect()
        }
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

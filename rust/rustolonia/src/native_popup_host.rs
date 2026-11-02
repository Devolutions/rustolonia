#[cfg(target_os = "macos")]
fn main() {
    use rustolonia::{App, Button, MenuFlyout, MenuItem, Window};
    if std::env::args().any(|arg| arg == "--file-drag") {
        file_drag_probe().expect("file drag gesture fixture");
        return;
    }
    if std::env::args().any(|arg| arg == "--termination") {
        termination_probe().expect("termination coordination fixture");
        return;
    }
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
fn file_drag_probe() -> Result<(), Box<dyn std::error::Error>> {
    use rustolonia::{App, AsControl, Button, ShutdownMode, Window};
    use rustolonia_sys::{FileDragEvent, IAvnApplication12};
    use std::sync::{Arc, Mutex};
    let host = rustolonia_sys::Host::load(rustolonia::discover_host_path()?)?;
    let application = host
        .activation_factory()?
        .create_application()?
        .query_interface::<IAvnApplication12>()?;
    let mode = std::env::args()
        .skip(2)
        .next()
        .unwrap_or_else(|| "click".into());
    if mode.starts_with("native-") {
        native_placement::observe_file_drag();
    }
    App::load_from_env()?.run(move |scope| {
        scope.set_shutdown_mode(ShutdownMode::OnExplicitShutdown)?;
        let clicks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = clicks.clone();
        let button = Button::new()?.width(300.0)?.height(120.0)?.content(Some(&rustolonia::TextBlock::new()?.text("Drag files")?))?
            .on_click(scope, move |_| { count.fetch_add(1, std::sync::atomic::Ordering::SeqCst); })?;
        let owner = Window::new()?.width(300.0)?.height(120.0)?.content(Some(&button))?;
        scope.mount(owner.clone())?;
        let events = Arc::new(Mutex::new(Vec::new()));
        let observed = events.clone();
        let reentrant = application.clone();
        let raw = button.as_control()?;
        let reentrant_control = raw.clone();
        let handler = rustolonia_sys::file_drag_handler(move |event| {
            println!("file drag: {event:?}");
            assert!(reentrant.set_file_drag_handler(&reentrant_control, None).is_err(),
                "registration changes must be posted outside callbacks");
            if let FileDragEvent::Preparing { token } = event {
                assert!(reentrant.cancel_file_drag(token).is_err(), "callback must post completion");
            }
            observed.lock().unwrap().push(event);
            Ok(())
        });
        application.set_file_drag_handler(&raw, Some(&handler))?;
        let other = host.activation_factory()?.create_application()?.query_interface::<IAvnApplication12>()?;
        assert!(other.set_file_drag_handler(&raw, Some(&handler)).is_err(),
            "auxiliary applications must not register the same target twice");
        drop(handler);
        let window_handle = scope.platform_handle(&owner)?;
        let number = native_placement::window_number(window_handle);
        let trigger_scope = scope.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(350));
            trigger_scope.clone().post(move || {
            native_placement::post_mouse(number, 1, 50.0, 50.0);
            if mode == "click" {
                native_placement::post_mouse(number, 6, 52.0, 50.0);
                native_placement::post_mouse(number, 2, 52.0, 50.0);
            } else {
                native_placement::post_mouse(number, 6, 65.0, 50.0);
            }
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(350));
                let next = trigger_scope.clone();
                trigger_scope.post(move || {
                    if mode == "click" {
                        assert_eq!(clicks.load(std::sync::atomic::Ordering::SeqCst), 1);
                        assert!(events.lock().unwrap().is_empty());
                        println!("ordinary below-threshold click preserved");
                        application.set_file_drag_handler(&raw, None).unwrap();
                        next.shutdown().unwrap();
                        return;
                    }
                    let token = events.lock().unwrap().iter().find_map(|event| match event {
                        FileDragEvent::Preparing { token } => Some(*token), _ => None,
                    }).expect("real AppKit gesture crossed threshold");
                    assert_eq!(clicks.load(std::sync::atomic::Ordering::SeqCst), 0);
                    if mode.starts_with("native-") {
                        assert!(application.complete_file_drag(token, "[]").is_err());
                        assert!(application.complete_file_drag(token, "[\"https://invalid/file\"]").is_err());
                        let cwd = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
                        let json = format!("[\"file://{}/host/Avalonia.Host.csproj\",\"file://{}/rust/Cargo.toml\"]",
                            cwd.display(), cwd.display());
                        application.complete_file_drag(token, &json).unwrap();
                        assert!(application.complete_file_drag(token, &json).is_err());
                        assert!(application.cancel_file_drag(token).is_err());
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(350));
                            let end = next.clone();
                            next.post(move || {
                                native_placement::assert_file_drag();
                                if mode == "native-owner-close" {
                                    owner.close().unwrap();
                                    let completed = events.lock().unwrap().iter().any(|event| matches!(event,
                                        FileDragEvent::Completed { token: value, .. } if *value == token));
                                    assert_eq!(application.set_file_drag_handler(&raw, None).is_ok(), completed,
                                        "owner close cannot clear an outstanding native session");
                                }
                                native_placement::post_escape(number);
                                std::thread::spawn(move || {
                                    std::thread::sleep(std::time::Duration::from_millis(1200));
                                    let done = end.clone();
                                    end.post(move || {
                                        assert_eq!(events.lock().unwrap().iter().filter(|event| matches!(event,
                                            FileDragEvent::Completed { token: value, effect: 0, status: 0 } if *value == token)).count(), 1,
                                            "AppKit Escape must end the real session once");
                                        assert_eq!(clicks.load(std::sync::atomic::Ordering::SeqCst), 0);
                                        application.set_file_drag_handler(&raw, None).unwrap();
                                        println!("real AppKit multi-file copy-only session cancelled");
                                        done.shutdown().unwrap();
                                    }).unwrap();
                                });
                            }).unwrap();
                        });
                    } else if mode == "owner-close" {
                        owner.close().unwrap();
                        assert!(application.complete_file_drag(token, "[\"file:///tmp/missing\"]").is_err());
                        assert_eq!(events.lock().unwrap().iter().filter(|event| matches!(event,
                            FileDragEvent::Completed { token: value, effect: 0, .. } if *value == token)).count(), 1);
                        println!("owner close invalidated gesture");
                        next.shutdown().unwrap();
                    } else if mode == "cancel" {
                        application.cancel_file_drag(token).unwrap();
                        assert!(application.cancel_file_drag(token).is_err());
                        native_placement::post_mouse(number, 2, 65.0, 50.0);
                        let end = next.clone();
                        next.post(move || {
                            assert_eq!(clicks.load(std::sync::atomic::Ordering::SeqCst), 0);
                            println!("cancelled drag does not click");
                            native_placement::post_mouse(number, 1, 50.0, 50.0);
                            native_placement::post_mouse(number, 2, 50.0, 50.0);
                            assert_eq!(clicks.load(std::sync::atomic::Ordering::SeqCst), 1,
                                "ordinary clicks recover after cancelled drag");
                            end.shutdown().unwrap();
                        }).unwrap();
                    } else {
                        native_placement::post_mouse(number, 2, 65.0, 50.0);
                        let end = next.clone();
                        next.post(move || {
                            assert!(application.complete_file_drag(token, "[\"file:///tmp/missing\"]").is_err());
                            assert_eq!(clicks.load(std::sync::atomic::Ordering::SeqCst), 0);
                            assert_eq!(events.lock().unwrap().iter().filter(|event| matches!(event,
                                FileDragEvent::Completed { token: value, effect: 0, .. } if *value == token)).count(), 1);
                            println!("release before preparation invalidated gesture");
                            end.shutdown().unwrap();
                        }).unwrap();
                    }
                }).unwrap();
            });
            }).unwrap();
        });
        Ok(())
    })?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn termination_probe() -> Result<(), Box<dyn std::error::Error>> {
    use rustolonia::{App, Button, PlacementMode, ShutdownMode, Window};
    use rustolonia_sys::{IAvnApplication11, NativePopupEvent};
    use std::sync::{Arc, Mutex};
    let args: Vec<_> = std::env::args().collect();
    let native = args.iter().any(|arg| arg == "--native");
    let with_popup = args.iter().any(|arg| arg == "--popup");
    let os = args.iter().any(|arg| arg == "--os");
    let host = rustolonia_sys::Host::load(rustolonia::discover_host_path()?)?;
    let auxiliary = host.activation_factory()?.create_application()?;
    let gate = auxiliary.query_interface::<IAvnApplication11>()?;
    let popup_gate = auxiliary.popup_gate()?;
    drop(auxiliary);
    let requests = Arc::new(Mutex::new(Vec::new()));
    App::load_from_env()?.run(move |scope| {
        scope.set_shutdown_mode(if native {
            ShutdownMode::OnExplicitShutdown
        } else {
            ShutdownMode::OnLastWindowClose
        })?;
        native_placement::observe_termination();
        let events = requests.clone();
        let reentrant = gate.clone();
        let handler = rustolonia_sys::termination_handler(move |request| {
            println!("termination request: {request:?}");
            assert!(
                reentrant.complete_termination(request.token, true).is_err(),
                "completion inside native callback must be posted"
            );
            events.lock().unwrap().push(request);
            Ok(())
        });
        gate.set_termination_handler(Some(&handler))?;
        drop(handler);
        let button = Button::new()?.tool_tip_tip("Pending shutdown tooltip")?;
        button.set_tool_tip_placement(PlacementMode::Bottom)?;
        let owner = Window::new()?.content(Some(&button))?;
        if with_popup {
            use rustolonia::AsControl;
            let raw = owner
                .as_control()?
                .query_interface::<rustolonia_sys::IAvnWindow>()?;
            let handler = rustolonia_sys::native_popup_handler(move |event| {
                if matches!(event, NativePopupEvent::Closed { .. }) {
                    assert!(
                        native_placement::cleanup_done(),
                        "cleanup precedes popup Closed"
                    );
                    println!("termination popup Closed after cleanup");
                }
                Ok(())
            });
            popup_gate.set(&raw, Some(&handler))?;
            let owner = owner.on_unloaded(scope, move |_| {
                assert!(
                    native_placement::cleanup_done(),
                    "cleanup precedes owner Unloaded"
                );
                popup_gate
                    .set(&raw, None)
                    .expect("clear popup gate in owner Unloaded");
                println!("termination owner Unloaded after cleanup");
            })?;
            scope.mount(owner)?;
        }
        let scope = scope.clone();
        let next = scope.clone();
        scope.post(move || {
            if with_popup {
                button.set_tool_tip_is_open(true).unwrap();
            }
            native_placement::request_termination(native, native);
            native_placement::request_termination(native, true);
            let observed = requests.lock().unwrap();
            assert_eq!(
                observed.len(),
                2,
                "repeated requests notify with same token"
            );
            assert_eq!(observed[0].token, observed[1].token);
            let old = observed[0].token;
            drop(observed);
            assert!(gate.set_termination_handler(None).is_err());
            gate.complete_termination(old, false).unwrap();
            assert!(gate.complete_termination(old, false).is_err());
            println!("cancelled first request; application still running");
            if native {
                let cancel_gate = gate.clone();
                let cancel_requests = requests.clone();
                let cancel_scope = next.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    cancel_scope
                        .post(move || {
                            let token = cancel_requests.lock().unwrap().last().unwrap().token;
                            assert_ne!(token, old);
                            cancel_gate.complete_termination(token, false).unwrap();
                            println!(
                                "actual deferred AppKit request cancelled; cleanup not accepted"
                            );
                        })
                        .unwrap();
                });
                native_placement::request_termination(true, false);
                assert!(!native_placement::cleanup_done());
            }
            let request_scope = next.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let dispatcher = request_scope.clone();
                dispatcher
                    .post(move || {
                        let fresh = requests.lock().unwrap().last().copied().unwrap();
                        assert_ne!(fresh.token, old);
                        assert_eq!(fresh.flags & 1 != 0, native || os);
                        assert_eq!(fresh.flags & 2 != 0, os);
                        assert!(!native_placement::cleanup_done());
                        // Represents asynchronous engine finalization; no UI-thread wait.
                        native_placement::mark_cleanup_done();
                        println!(
                            "asynchronous cleanup complete before accepting token {}",
                            fresh.token
                        );
                        gate.complete_termination(fresh.token, true).unwrap();
                        assert!(gate.complete_termination(fresh.token, true).is_err());
                        println!("managed acceptance returned after window teardown");
                        drop(request_scope);
                    })
                    .unwrap();
            });
            if os {
                native_placement::send_logout_event();
            } else {
                native_placement::request_termination(native, false);
            }
        })
    })?;
    assert!(native_placement::cleanup_done());
    println!("App::run returned after cleanup");
    Ok(())
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
    use std::sync::atomic::{AtomicBool, Ordering};
    static CLEANUP_DONE: AtomicBool = AtomicBool::new(false);

    pub fn cleanup_done() -> bool {
        CLEANUP_DONE.load(Ordering::SeqCst)
    }
    pub fn mark_cleanup_done() {
        CLEANUP_DONE.store(true, Ordering::SeqCst);
    }

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
        #[link_name = "objc_msgSend"]
        fn mouse_event(
            receiver: *mut c_void,
            selector: *mut c_void,
            kind: usize,
            location: Point,
            modifiers: usize,
            timestamp: f64,
            number: isize,
            context: *mut c_void,
            event_number: isize,
            clicks: isize,
            pressure: f32,
        ) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn post_event(
            receiver: *mut c_void,
            selector: *mut c_void,
            event: *mut c_void,
            at_start: bool,
        );
        #[link_name = "objc_msgSend"]
        fn next_event(
            receiver: *mut c_void,
            selector: *mut c_void,
            mask: usize,
            date: *mut c_void,
            mode: *mut c_void,
            dequeue: bool,
        ) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn date_after(receiver: *mut c_void, selector: *mut c_void, seconds: f64) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn send_arg(receiver: *mut c_void, selector: *mut c_void, arg: *mut c_void) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn reply_code(receiver: *mut c_void, selector: *mut c_void, arg: *mut c_void) -> usize;
        fn objc_allocateClassPair(
            base: *mut c_void,
            name: *const c_char,
            extra: usize,
        ) -> *mut c_void;
        fn objc_registerClassPair(class: *mut c_void);
        fn class_addMethod(
            class: *mut c_void,
            selector: *mut c_void,
            imp: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void),
            types: *const c_char,
        ) -> bool;
        #[link_name = "objc_msgSend"]
        fn add_observer(
            center: *mut c_void,
            selector: *mut c_void,
            observer: *mut c_void,
            method: *mut c_void,
            name: *mut c_void,
            object: *mut c_void,
        );
        #[link_name = "objc_msgSend"]
        fn string(
            receiver: *mut c_void,
            selector: *mut c_void,
            value: *const c_char,
        ) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn pid_descriptor(receiver: *mut c_void, selector: *mut c_void, pid: i32) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn event_descriptor(
            receiver: *mut c_void,
            selector: *mut c_void,
            class: u32,
            id: u32,
            target: *mut c_void,
            return_id: i16,
            transaction: i32,
        ) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn enum_descriptor(receiver: *mut c_void, selector: *mut c_void, code: u32) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn set_parameter(
            receiver: *mut c_void,
            selector: *mut c_void,
            descriptor: *mut c_void,
            key: u32,
        );
        #[link_name = "objc_msgSend"]
        fn send_event(
            receiver: *mut c_void,
            selector: *mut c_void,
            options: usize,
            timeout: f64,
            error: *mut *mut c_void,
        ) -> *mut c_void;
        #[cfg(target_arch = "aarch64")]
        #[link_name = "objc_msgSend"]
        fn get_frame(receiver: *mut c_void, selector: *mut c_void) -> Frame;
        #[cfg(target_arch = "x86_64")]
        #[link_name = "objc_msgSend_stret"]
        fn get_frame(out: *mut Frame, receiver: *mut c_void, selector: *mut c_void);
    }

    pub fn window_number(handle: isize) -> usize {
        unsafe {
            count(
                handle as *mut c_void,
                sel_registerName(c"windowNumber".as_ptr()),
            )
        }
    }
    static ORIGINAL_DRAG: std::sync::atomic::AtomicPtr<c_void> =
        std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
    static DRAG_VALID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    unsafe extern "C" fn begin_drag(
        view: *mut c_void,
        selector: *mut c_void,
        items: *mut c_void,
        event: *mut c_void,
        source: *mut c_void,
    ) -> *mut c_void {
        unsafe extern "C" {
            #[link_name = "objc_msgSend"]
            fn index(receiver: *mut c_void, selector: *mut c_void, index: usize) -> *mut c_void;
            #[link_name = "objc_msgSend"]
            fn mask(
                receiver: *mut c_void,
                selector: *mut c_void,
                session: *mut c_void,
                context: usize,
            ) -> usize;
            #[link_name = "objc_msgSend"]
            fn read_objects(
                receiver: *mut c_void,
                selector: *mut c_void,
                classes: *mut c_void,
                options: *mut c_void,
            ) -> *mut c_void;
            #[link_name = "objc_msgSend"]
            fn add_object(receiver: *mut c_void, selector: *mut c_void, value: *mut c_void);
            #[link_name = "objc_msgSend"]
            fn write_objects(
                receiver: *mut c_void,
                selector: *mut c_void,
                value: *mut c_void,
            ) -> bool;
            #[link_name = "objc_msgSend"]
            fn action(receiver: *mut c_void, selector: *mut c_void);
        }
        let ty = string(
            objc_getClass(c"NSString".as_ptr()),
            sel_registerName(c"stringWithUTF8String:".as_ptr()),
            c"public.file-url".as_ptr(),
        );
        let mut valid = count(items, sel_registerName(c"count".as_ptr())) == 2
            && count(event, sel_registerName(c"type".as_ptr())) == 1;
        let writers = index(
            objc_getClass(c"NSMutableArray".as_ptr()),
            sel_registerName(c"arrayWithCapacity:".as_ptr()),
            2,
        );
        for i in 0..2 {
            let dragging_item = index(items, sel_registerName(c"objectAtIndex:".as_ptr()), i);
            let writer = send(dragging_item, sel_registerName(c"item".as_ptr()));
            let value = send_arg(
                writer,
                sel_registerName(c"pasteboardPropertyListForType:".as_ptr()),
                ty,
            );
            valid &= !value.is_null();
            add_object(writers, sel_registerName(c"addObject:".as_ptr()), writer);
        }
        let pasteboard = send(
            objc_getClass(c"NSPasteboard".as_ptr()),
            sel_registerName(c"pasteboardWithUniqueName".as_ptr()),
        );
        valid &= write_objects(
            pasteboard,
            sel_registerName(c"writeObjects:".as_ptr()),
            writers,
        );
        let classes = send_arg(
            objc_getClass(c"NSArray".as_ptr()),
            sel_registerName(c"arrayWithObject:".as_ptr()),
            objc_getClass(c"NSURL".as_ptr()),
        );
        let received = read_objects(
            pasteboard,
            sel_registerName(c"readObjectsForClasses:options:".as_ptr()),
            classes,
            std::ptr::null_mut(),
        );
        valid &= count(received, sel_registerName(c"count".as_ptr())) == 2;
        action(pasteboard, sel_registerName(c"releaseGlobally".as_ptr()));
        for context in [0, 1] {
            valid &= mask(
                source,
                sel_registerName(
                    c"draggingSession:sourceOperationMaskForDraggingContext:".as_ptr(),
                ),
                std::ptr::null_mut(),
                context,
            ) == 1;
        }
        DRAG_VALID.store(valid, std::sync::atomic::Ordering::SeqCst);
        let original: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut c_void,
            *mut c_void,
            *mut c_void,
        ) -> *mut c_void =
            std::mem::transmute(ORIGINAL_DRAG.load(std::sync::atomic::Ordering::SeqCst));
        original(view, selector, items, event, source)
    }
    pub fn observe_file_drag() {
        unsafe extern "C" {
            fn class_getInstanceMethod(class: *mut c_void, selector: *mut c_void) -> *mut c_void;
            fn method_setImplementation(
                method: *mut c_void,
                implementation: *mut c_void,
            ) -> *mut c_void;
        }
        unsafe {
            let method = class_getInstanceMethod(
                objc_getClass(c"NSView".as_ptr()),
                sel_registerName(c"beginDraggingSessionWithItems:event:source:".as_ptr()),
            );
            assert!(!method.is_null());
            ORIGINAL_DRAG.store(
                method_setImplementation(method, begin_drag as *mut c_void),
                std::sync::atomic::Ordering::SeqCst,
            );
        }
    }
    pub fn assert_file_drag() {
        assert!(
            DRAG_VALID.load(std::sync::atomic::Ordering::SeqCst),
            "two public.file-url writers, retained mouse-down, copy-only in both native contexts"
        );
        println!("native payload exposes two file URLs and Copy only; isolated NSPasteboard NSURL reader received both");
    }
    pub fn post_escape(number: usize) {
        unsafe extern "C" {
            #[link_name = "objc_msgSend"]
            fn key_event(
                receiver: *mut c_void,
                selector: *mut c_void,
                kind: usize,
                location: Point,
                modifiers: usize,
                time: f64,
                window: isize,
                context: *mut c_void,
                characters: *mut c_void,
                ignoring: *mut c_void,
                repeat: bool,
                keycode: u16,
            ) -> *mut c_void;
        }
        unsafe {
            let escape = string(
                objc_getClass(c"NSString".as_ptr()),
                sel_registerName(c"stringWithUTF8String:".as_ptr()),
                c"\x1b".as_ptr(),
            );
            let event = key_event(objc_getClass(c"NSEvent".as_ptr()),
                sel_registerName(c"keyEventWithType:location:modifierFlags:timestamp:windowNumber:context:characters:charactersIgnoringModifiers:isARepeat:keyCode:".as_ptr()),
                10, Point { x: 0.0, y: 0.0 }, 0, 2.0, number as isize, std::ptr::null_mut(), escape, escape, false, 53);
            let app = send(
                objc_getClass(c"NSApplication".as_ptr()),
                sel_registerName(c"sharedApplication".as_ptr()),
            );
            post_event(
                app,
                sel_registerName(c"postEvent:atStart:".as_ptr()),
                event,
                false,
            );
        }
    }
    pub fn post_mouse(number: usize, kind: usize, x: f64, y: f64) {
        unsafe {
            let event = mouse_event(objc_getClass(c"NSEvent".as_ptr()),
                sel_registerName(c"mouseEventWithType:location:modifierFlags:timestamp:windowNumber:context:eventNumber:clickCount:pressure:".as_ptr()),
                kind, Point { x, y }, 0, 1.0, number as isize, std::ptr::null_mut(), 0, 1, 1.0);
            assert!(!event.is_null());
            let app = send(
                objc_getClass(c"NSApplication".as_ptr()),
                sel_registerName(c"sharedApplication".as_ptr()),
            );
            post_event(
                app,
                sel_registerName(c"postEvent:atStart:".as_ptr()),
                event,
                false,
            );
            let date = date_after(
                objc_getClass(c"NSDate".as_ptr()),
                sel_registerName(c"dateWithTimeIntervalSinceNow:".as_ptr()),
                0.1,
            );
            let mode = string(
                objc_getClass(c"NSString".as_ptr()),
                sel_registerName(c"stringWithUTF8String:".as_ptr()),
                c"kCFRunLoopDefaultMode".as_ptr(),
            );
            let delivered = next_event(
                app,
                sel_registerName(c"nextEventMatchingMask:untilDate:inMode:dequeue:".as_ptr()),
                1 << kind,
                date,
                mode,
                true,
            );
            assert!(!delivered.is_null(), "queued AppKit gesture event");
            send_arg(app, sel_registerName(c"sendEvent:".as_ptr()), delivered);
        }
    }

    unsafe extern "C" fn will_terminate(_: *mut c_void, _: *mut c_void, _: *mut c_void) {
        if !cleanup_done() {
            eprintln!("WillTerminate before cleanup");
            std::process::exit(19);
        }
        println!("NSApplicationWillTerminateNotification after cleanup");
    }

    pub fn observe_termination() {
        unsafe {
            let class = objc_allocateClassPair(
                objc_getClass(c"NSObject".as_ptr()),
                c"RustoloniaTerminationFixtureObserver".as_ptr(),
                0,
            );
            assert!(!class.is_null());
            let method = sel_registerName(c"onTermination:".as_ptr());
            assert!(class_addMethod(
                class,
                method,
                will_terminate,
                c"v@:@".as_ptr()
            ));
            objc_registerClassPair(class);
            let observer = send(
                send(class, sel_registerName(c"alloc".as_ptr())),
                sel_registerName(c"init".as_ptr()),
            );
            let center = send(
                objc_getClass(c"NSNotificationCenter".as_ptr()),
                sel_registerName(c"defaultCenter".as_ptr()),
            );
            let name = string(
                objc_getClass(c"NSString".as_ptr()),
                sel_registerName(c"stringWithUTF8String:".as_ptr()),
                c"NSApplicationWillTerminateNotification".as_ptr(),
            );
            add_observer(
                center,
                sel_registerName(c"addObserver:selector:name:object:".as_ptr()),
                observer,
                method,
                name,
                std::ptr::null_mut(),
            );
        }
    }

    pub fn request_termination(native: bool, repeated: bool) {
        if !native {
            let trigger = rustolonia::Window::new().unwrap();
            trigger.show().unwrap();
            trigger.close().unwrap();
            return;
        }
        unsafe {
            let app = send(
                objc_getClass(c"NSApplication".as_ptr()),
                sel_registerName(c"sharedApplication".as_ptr()),
            );
            if native {
                if repeated {
                    let delegate = send(app, sel_registerName(c"delegate".as_ptr()));
                    assert_eq!(
                        reply_code(
                            delegate,
                            sel_registerName(c"applicationShouldTerminate:".as_ptr()),
                            app
                        ),
                        2,
                        "native delegate must return NSTerminateLater"
                    );
                } else {
                    send_arg(
                        app,
                        sel_registerName(c"terminate:".as_ptr()),
                        std::ptr::null_mut(),
                    );
                }
            }
        }
    }

    pub fn send_logout_event() {
        unsafe {
            let class = objc_getClass(c"NSAppleEventDescriptor".as_ptr());
            let target = pid_descriptor(
                class,
                sel_registerName(c"descriptorWithProcessIdentifier:".as_ptr()),
                std::process::id() as i32,
            );
            let event = event_descriptor(
                class,
                sel_registerName(
                    c"appleEventWithEventClass:eventID:targetDescriptor:returnID:transactionID:"
                        .as_ptr(),
                ),
                u32::from_be_bytes(*b"aevt"),
                u32::from_be_bytes(*b"quit"),
                target,
                -1,
                0,
            );
            let reason = enum_descriptor(
                class,
                sel_registerName(c"descriptorWithEnumCode:".as_ptr()),
                u32::from_be_bytes(*b"logo"),
            );
            set_parameter(
                event,
                sel_registerName(c"setParamDescriptor:forKeyword:".as_ptr()),
                reason,
                u32::from_be_bytes(*b"why?"),
            );
            let mut error = std::ptr::null_mut();
            send_event(
                event,
                sel_registerName(c"sendEventWithOptions:timeout:error:".as_ptr()),
                1,
                1.0,
                &mut error,
            );
            assert!(
                error.is_null(),
                "send self-directed logout-reason AppleEvent"
            );
        }
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

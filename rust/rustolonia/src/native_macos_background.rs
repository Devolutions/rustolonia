#[cfg(target_os = "macos")]
#[allow(clashing_extern_declarations)]
mod macos {
    use rustolonia::{App, Result, ShutdownMode, Window};
    use std::ffi::{c_char, c_void, CStr};
    use std::time::Duration;

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}
    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn get(object: *mut c_void, selector: *mut c_void) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn integer(object: *mut c_void, selector: *mut c_void) -> isize;
        #[link_name = "objc_msgSend"]
        fn process_id(object: *mut c_void, selector: *mut c_void) -> i32;
        #[link_name = "objc_msgSend"]
        fn boolean(object: *mut c_void, selector: *mut c_void) -> bool;
        #[link_name = "objc_msgSend"]
        fn running_application(object: *mut c_void, selector: *mut c_void, pid: i32)
            -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn activate(object: *mut c_void, selector: *mut c_void, options: usize) -> bool;
    }

    unsafe fn selector(name: &CStr) -> *mut c_void {
        sel_registerName(name.as_ptr())
    }

    unsafe fn application() -> *mut c_void {
        get(
            objc_getClass(c"NSApplication".as_ptr()),
            selector(c"sharedApplication"),
        )
    }

    unsafe fn frontmost_pid() -> i32 {
        let workspace = get(
            objc_getClass(c"NSWorkspace".as_ptr()),
            selector(c"sharedWorkspace"),
        );
        let frontmost = get(workspace, selector(c"frontmostApplication"));
        process_id(frontmost, selector(c"processIdentifier"))
    }

    unsafe fn restore_frontmost(pid: i32) {
        if frontmost_pid() == std::process::id() as i32 {
            let previous = running_application(
                objc_getClass(c"NSRunningApplication".as_ptr()),
                selector(c"runningApplicationWithProcessIdentifier:"),
                pid,
            );
            if !previous.is_null() {
                activate(previous, selector(c"activateWithOptions:"), 2);
            }
        }
    }

    unsafe fn assert_accessory(previous: i32) {
        assert_eq!(integer(application(), selector(c"activationPolicy")), 1);
        assert!(!boolean(application(), selector(c"isActive")));
        assert_eq!(
            frontmost_pid(),
            previous,
            "background launch changed the frontmost application"
        );
    }

    pub fn run() -> Result<()> {
        let mode = std::env::args()
            .nth(1)
            .expect("background, promote, or foreground");
        assert!(matches!(
            mode.as_str(),
            "background" | "promote" | "foreground"
        ));
        let previous = unsafe { frontmost_pid() };
        // The same agent bundle must start Accessory before the host runs.
        assert_eq!(
            unsafe { integer(application(), selector(c"activationPolicy")) },
            1
        );
        let app = App::load_from_env()?.with_error_handler(move |context, error| {
            eprintln!("{context}: {error}");
            unsafe { restore_frontmost(previous) };
            std::process::exit(1);
        });
        let app = if mode == "foreground" {
            app
        } else {
            app.with_macos_background()?
        };
        let result =
            app.without_startup_arguments().run(move |scope| {
                scope.set_shutdown_mode(ShutdownMode::OnExplicitShutdown)?;
                if mode != "foreground" {
                    unsafe { assert_accessory(previous) };
                }
                let window = Window::el()
                    .title("Rustolonia background regression")
                    .build(scope)?;
                scope.set_topmost(&window, true)?;
                scope.show_window(&window, mode == "foreground")?;
                let tray = scope.create_tray(|_| Ok(()))?;
                tray.set_icon(&std::env::args().nth(2).expect("tray image"))?;
                tray.set_visible(true)?;
                let scope = scope.clone();
                scope.clone().spawn_local(async move {
                for _ in 0..20 {
                    scope.delay(Duration::from_millis(10))?.await?;
                    if mode != "foreground" {
                        unsafe { assert_accessory(previous) };
                        assert!(window.is_visible()?);
                        assert!(unsafe { get(application(), selector(c"keyWindow")) }.is_null());
                    }
                }
                if mode == "promote" {
                    let off_thread = scope.clone();
                    assert!(std::thread::spawn(move || off_thread.promote_macos_to_foreground())
                        .join().unwrap().is_err());
                    unsafe { assert_accessory(previous) };
                    scope.promote_macos_to_foreground()?;
                    scope.show_window(&window, true)?;
                }
                if mode != "background" {
                    for _ in 0..200 {
                        if unsafe { frontmost_pid() } == std::process::id() as i32
                            && unsafe { boolean(application(), selector(c"isActive")) }
                        {
                            break;
                        }
                        scope.delay(Duration::from_millis(10))?.await?;
                    }
                    assert_eq!(unsafe { integer(application(), selector(c"activationPolicy")) }, 0);
                    assert_eq!(unsafe { frontmost_pid() }, std::process::id() as i32);
                    assert!(unsafe { boolean(application(), selector(c"isActive")) });
                }
                tray.set_visible(false)?;
                window.hide()?;
                unsafe { restore_frontmost(previous) };
                scope.delay(Duration::from_millis(100))?.await?;
                println!("{mode}: bundled launch, tray, window, and activation checks passed");
                scope.shutdown()?;
                Ok(())
            })?;
                Ok(())
            });
        unsafe { restore_frontmost(previous) };
        result
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    if let Err(error) = macos::run() {
        eprintln!("macOS background regression failed: {error}");
        std::process::exit(1);
    }
    #[cfg(not(target_os = "macos"))]
    eprintln!("native_macos_background requires macOS");
}

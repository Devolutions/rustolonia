#[cfg(target_os = "macos")]
#[allow(clashing_extern_declarations)] // objc_msgSend requires each selector's exact ABI signature.
mod macos {
    use rustolonia::{App, Result, Window};
    use std::ffi::{c_char, c_void, CStr};
    use std::time::Duration;

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn send(object: *mut c_void, selector: *mut c_void) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn index(object: *mut c_void, selector: *mut c_void, index: isize) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn count(object: *mut c_void, selector: *mut c_void) -> isize;
        #[link_name = "objc_msgSend"]
        fn action(object: *mut c_void, selector: *mut c_void);
        #[link_name = "objc_msgSend"]
        fn activate(object: *mut c_void, selector: *mut c_void, ignore_other_apps: bool);
    }

    unsafe fn get(object: *mut c_void, selector: &CStr) -> *mut c_void {
        assert!(!object.is_null(), "nil receiver for {selector:?}");
        send(object, sel_registerName(selector.as_ptr()))
    }

    unsafe fn text(object: *mut c_void) -> String {
        let utf8 = get(object, c"UTF8String").cast::<c_char>();
        assert!(!utf8.is_null());
        CStr::from_ptr(utf8).to_str().unwrap().to_owned()
    }

    unsafe fn check_name(expected: &str, expected_window: &str) {
        let process = get(objc_getClass(c"NSProcessInfo".as_ptr()), c"processInfo");
        assert_eq!(text(get(process, c"processName")), expected);
        let running = get(
            objc_getClass(c"NSRunningApplication".as_ptr()),
            c"currentApplication",
        );
        assert_eq!(text(get(running, c"localizedName")), expected);
        let app = get(
            objc_getClass(c"NSApplication".as_ptr()),
            c"sharedApplication",
        );
        assert_eq!(text(get(get(app, c"keyWindow"), c"title")), expected_window);
        let menu = get(app, c"mainMenu");
        action(menu, sel_registerName(c"update".as_ptr()));
        let first = index(menu, sel_registerName(c"itemAtIndex:".as_ptr()), 0);
        assert_eq!(text(get(first, c"title")), expected);
        let submenu = get(first, c"submenu");
        action(submenu, sel_registerName(c"update".as_ptr()));
        assert_eq!(text(get(submenu, c"title")), expected);
        let mut found_hide = false;
        let mut found_quit = false;
        for i in 0..count(submenu, sel_registerName(c"numberOfItems".as_ptr())) {
            let item = index(submenu, sel_registerName(c"itemAtIndex:".as_ptr()), i);
            let title = text(get(item, c"title"));
            assert!(!title.contains("Avalonia Application"), "{title}");
            if title == format!("Hide {expected}") {
                found_hide = true;
                assert!(!get(item, c"action").is_null());
                assert_eq!(text(get(item, c"keyEquivalent")), "h");
            }
            if title == "Quit" || title.starts_with("Quit ") {
                found_quit = true;
                assert!(!get(item, c"action").is_null());
                assert_eq!(text(get(item, c"keyEquivalent")), "q");
            }
        }
        assert!(found_hide && found_quit, "standard Hide/Quit items missing");
    }

    pub fn run() -> Result<()> {
        let name = std::env::args().nth(1).expect("application name argument");
        App::load_from_env()?
            .with_application_name(&name)?
            .without_startup_arguments()
            .run(move |scope| {
                let first = Window::el().title("First name-test window").build(scope)?;
                let second = Window::el().title("Second name-test window").build(scope)?;
                scope.mount(first.clone())?;
                scope.mount(second.clone())?;
                unsafe {
                    let app = get(
                        objc_getClass(c"NSApplication".as_ptr()),
                        c"sharedApplication",
                    );
                    activate(
                        app,
                        sel_registerName(c"activateIgnoringOtherApps:".as_ptr()),
                        true,
                    );
                }
                let scope = scope.clone();
                scope.clone().spawn(async move {
                    for (window, title) in [
                        (first.clone(), "First name-test window"),
                        (second, "Second name-test window"),
                        (first, "First name-test window"),
                    ] {
                        scope
                            .activate_window(&window)
                            .expect("activate test window");
                        scope
                            .delay(Duration::from_millis(300))
                            .unwrap()
                            .await
                            .unwrap();
                        unsafe { check_name(&name, title) };
                    }
                    println!("application name and menus preserved across window activation");
                    scope.shutdown().expect("shutdown name test");
                })
            })
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    if let Err(error) = macos::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("native_application_name is a macOS-only integration harness");
        std::process::exit(2);
    }
}

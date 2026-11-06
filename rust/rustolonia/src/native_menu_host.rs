//! macOS integration harness for code-first native menus: checks that a
//! Rust-built window menu and application menu reach `NSApp.mainMenu`, that a
//! menu click and a shortcut each reach the Rust handler exactly once.

#[cfg(target_os = "macos")]
#[allow(clashing_extern_declarations)] // objc_msgSend requires each selector's exact ABI signature.
mod macos {
    use rustolonia::{App, NativeMenuItem, Result, Window};
    use std::ffi::{c_char, c_void, CStr};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Point {
        x: f64,
        y: f64,
    }

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn send(object: *mut c_void, selector: *mut c_void) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn index(object: *mut c_void, selector: *mut c_void, index: isize) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn integer(object: *mut c_void, selector: *mut c_void) -> isize;
        #[link_name = "objc_msgSend"]
        fn action(object: *mut c_void, selector: *mut c_void);
        #[link_name = "objc_msgSend"]
        fn perform(object: *mut c_void, selector: *mut c_void, index: isize);
        #[link_name = "objc_msgSend"]
        fn with_object(object: *mut c_void, selector: *mut c_void, value: *mut c_void);
        #[link_name = "objc_msgSend"]
        fn activate(object: *mut c_void, selector: *mut c_void, ignore_other_apps: bool);
        #[link_name = "objc_msgSend"]
        fn string(object: *mut c_void, selector: *mut c_void, utf8: *const c_char) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        #[allow(clippy::too_many_arguments)]
        fn key_event(
            class: *mut c_void,
            selector: *mut c_void,
            kind: usize,
            location: Point,
            modifiers: usize,
            timestamp: f64,
            window_number: isize,
            context: *mut c_void,
            characters: *mut c_void,
            unmodified: *mut c_void,
            repeat: bool,
            key_code: u16,
        ) -> *mut c_void;
    }

    const OPEN: u32 = 11;
    const ABOUT: u32 = 1;
    const COMMAND_MASK: isize = 1 << 20;

    unsafe fn sel(name: &CStr) -> *mut c_void {
        sel_registerName(name.as_ptr())
    }

    unsafe fn get(object: *mut c_void, selector: &CStr) -> *mut c_void {
        assert!(!object.is_null(), "nil receiver for {selector:?}");
        send(object, sel(selector))
    }

    unsafe fn text(object: *mut c_void) -> String {
        let utf8 = get(object, c"UTF8String").cast::<c_char>();
        assert!(!utf8.is_null());
        CStr::from_ptr(utf8).to_str().unwrap().to_owned()
    }

    unsafe fn ns_app() -> *mut c_void {
        get(
            objc_getClass(c"NSApplication".as_ptr()),
            c"sharedApplication",
        )
    }

    unsafe fn items(menu: *mut c_void) -> Vec<(String, *mut c_void)> {
        action(menu, sel(c"update"));
        (0..integer(menu, sel(c"numberOfItems")))
            .map(|i| {
                let item = index(menu, sel(c"itemAtIndex:"), i);
                (text(get(item, c"title")), item)
            })
            .collect()
    }

    unsafe fn submenu_titled(menu: *mut c_void, title: &str) -> *mut c_void {
        let found = items(menu)
            .into_iter()
            .find(|(t, _)| t == title)
            .unwrap_or_else(|| panic!("no top-level {title:?} in {:?}", titles(menu)));
        get(found.1, c"submenu")
    }

    unsafe fn titles(menu: *mut c_void) -> Vec<String> {
        items(menu).into_iter().map(|(t, _)| t).collect()
    }

    /// Checks the menu bar and clicks File > Open through AppKit.
    unsafe fn check_menus() {
        let main = get(ns_app(), c"mainMenu");
        let application = get(index(main, sel(c"itemAtIndex:"), 0), c"submenu");
        let app_titles = titles(application);
        assert_eq!("About Menu Harness", app_titles[0], "{app_titles:?}");
        assert!(
            !app_titles.iter().any(|t| t == "About Avalonia"),
            "{app_titles:?}"
        );
        assert!(
            app_titles.iter().any(|t| t.starts_with("Quit")),
            "{app_titles:?}"
        );

        let file = submenu_titled(main, "File");
        let file_items = items(file);
        let (position, open) = file_items
            .iter()
            .enumerate()
            .find(|(_, (t, _))| t == "Open")
            .map(|(i, (_, item))| (i, *item))
            .unwrap_or_else(|| panic!("no Open in {:?}", titles(file)));
        assert_eq!("o", text(get(open, c"keyEquivalent")).to_lowercase());
        assert_eq!(
            COMMAND_MASK,
            integer(open, sel(c"keyEquivalentModifierMask")) & COMMAND_MASK
        );
        perform(
            file,
            sel(c"performActionForItemAtIndex:"),
            position as isize,
        );
    }

    /// Sends Cmd+O to the key window the way the system would.
    unsafe fn press_command_o() {
        let app = ns_app();
        let window = get(app, c"keyWindow");
        let number = integer(window, sel(c"windowNumber"));
        let ns_string = objc_getClass(c"NSString".as_ptr());
        let o = string(ns_string, sel(c"stringWithUTF8String:"), c"o".as_ptr());
        let event = key_event(
            objc_getClass(c"NSEvent".as_ptr()),
            sel(c"keyEventWithType:location:modifierFlags:timestamp:windowNumber:context:characters:charactersIgnoringModifiers:isARepeat:keyCode:"),
            10, // NSEventTypeKeyDown
            Point { x: 0.0, y: 0.0 },
            COMMAND_MASK as usize,
            0.0,
            number,
            std::ptr::null_mut(),
            o,
            o,
            false,
            31, // kVK_ANSI_O
        );
        assert!(!event.is_null());
        with_object(app, sel(c"sendEvent:"), event);
    }

    pub fn run() -> Result<()> {
        App::load_from_env()?
            .with_application_name("Menu Harness")?
            .without_startup_arguments()
            .run(move |scope| {
                let clicks = Arc::new(Mutex::new(Vec::new()));
                let menu = scope.create_native_menu({
                    let clicks = clicks.clone();
                    move |id| {
                        clicks.lock().unwrap().push(id);
                        Ok(())
                    }
                })?;
                menu.set_items(&[NativeMenuItem::submenu(
                    "File",
                    vec![
                        NativeMenuItem::command(10, "New"),
                        NativeMenuItem::command(OPEN, "Open").gesture("Primary+O"),
                    ],
                )])?;
                let app_menu = scope.create_native_menu(|_| Ok(()))?;
                app_menu.set_items(&[NativeMenuItem::command(ABOUT, "About Menu Harness")])?;
                scope.set_application_menu(Some(&app_menu))?;

                let window = Window::new()?.title("Native menu harness")?;
                scope.set_window_menu(&window, Some(&menu))?;
                scope.mount(window.clone())?;
                unsafe { activate(ns_app(), sel(c"activateIgnoringOtherApps:"), true) };

                let scope = scope.clone();
                scope.clone().spawn(async move {
                    let pause = || scope.delay(Duration::from_millis(400)).unwrap();
                    scope.activate_window(&window).expect("activate window");
                    pause().await.unwrap();
                    assert!(scope.is_native_menu_exported(&window).unwrap());
                    unsafe { check_menus() };
                    pause().await.unwrap();
                    assert_eq!(vec![OPEN], *clicks.lock().unwrap(), "menu click");
                    unsafe { press_command_o() };
                    pause().await.unwrap();
                    assert_eq!(vec![OPEN, OPEN], *clicks.lock().unwrap(), "shortcut");

                    scope.set_window_menu(&window, None).unwrap();
                    scope.set_application_menu(None).unwrap();
                    pause().await.unwrap();
                    unsafe {
                        let main = get(ns_app(), c"mainMenu");
                        let application = get(index(main, sel(c"itemAtIndex:"), 0), c"submenu");
                        assert_eq!("About Avalonia", titles(application)[0]);
                    }
                    println!("native menus exported and invoked");
                    scope.shutdown().expect("shutdown menu harness");
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
        eprintln!("native_menu_host is a macOS-only integration harness");
        std::process::exit(2);
    }
}

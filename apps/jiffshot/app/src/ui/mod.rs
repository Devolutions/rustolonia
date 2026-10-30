//! Every window, built from Rust with Rustolonia's code-first API: no AXAML,
//! no managed code. Windows bind to the signals of the view models in
//! [`crate::vm`] and send what the user does to their models.
//!
//! Event callbacks must be `Send`, but view models live on the UI thread:
//! callbacks reach the main and Capture windows' bindings through [`main`]
//! and [`capture`], and carry anything else in a [`UiOnly`].

pub mod canvas_view;
mod capture_view;
pub mod dialogs;
mod extras;
mod hud;
mod main_window;
mod overlay;
mod prefs_window;
mod selftest;
mod toast;
mod tray;

use crate::vm::{CaptureBinding, CaptureViewModel, MainBinding, MainViewModel, TrayBinding, TrayViewModel};
use rustolonia::{AppScope, Result, Window};
use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::thread::ThreadId;

thread_local! {
    static MAIN: RefCell<Option<MainBinding>> = const { RefCell::new(None) };
    static CAPTURE: RefCell<Option<CaptureBinding>> = const { RefCell::new(None) };
    static TRAY: RefCell<Option<TrayBinding>> = const { RefCell::new(None) };
    static MAIN_WINDOW: RefCell<Option<(AppScope, Window)>> = const { RefCell::new(None) };
    static CANVAS: RefCell<Option<canvas_view::CanvasView>> = const { RefCell::new(None) };
}

/// The editor window and its scope, once mounted. On the UI thread.
fn main_window() -> Option<Window> {
    MAIN_WINDOW.with(|m| m.borrow().as_ref().map(|(_, w)| w.clone()))
}

fn scope() -> Option<AppScope> {
    MAIN_WINDOW.with(|m| m.borrow().as_ref().map(|(s, _)| s.clone()))
}

/// The editor window's native handle (0 until first shown).
fn main_window_handle() -> Option<isize> {
    let (scope, window) = MAIN_WINDOW.with(|m| m.borrow().clone())?;
    scope.platform_handle(&window).ok().filter(|h| *h != 0)
}

/// The editor canvas, once the editor window is built. On the UI thread.
pub fn canvas() -> Option<canvas_view::CanvasView> {
    CANVAS.with(|c| c.borrow().clone())
}

fn set_canvas(canvas: canvas_view::CanvasView) {
    CANVAS.with(|c| *c.borrow_mut() = Some(canvas));
}

/// The editor window's binding. On the UI thread, once mounted.
pub fn main() -> MainBinding {
    MAIN.with(|m| m.borrow().clone()).expect("the main window is mounted")
}

/// The Capture window's binding. On the UI thread, once mounted.
pub fn capture() -> CaptureBinding {
    CAPTURE.with(|m| m.borrow().clone()).expect("the Capture window is mounted")
}

/// A UI-thread value carried by a `Send` callback. Using it on another thread
/// panics; dropping it on another thread (a callback released by the host's
/// finalizer) leaks it instead.
pub struct UiOnly<T> {
    value: ManuallyDrop<T>,
    thread: ThreadId,
}

// SAFETY: the value is only reached, and only dropped, on the thread that
// made it.
unsafe impl<T> Send for UiOnly<T> {}
unsafe impl<T> Sync for UiOnly<T> {}

impl<T> UiOnly<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: ManuallyDrop::new(value),
            thread: std::thread::current().id(),
        }
    }

    pub fn get(&self) -> &T {
        assert_eq!(self.thread, std::thread::current().id(), "UI value used off the UI thread");
        &self.value
    }
}

impl<T: Clone> Clone for UiOnly<T> {
    fn clone(&self) -> Self {
        Self::new(self.get().clone())
    }
}

impl<T> Drop for UiOnly<T> {
    fn drop(&mut self) {
        if self.thread == std::thread::current().id() {
            // SAFETY: dropped once, here.
            unsafe { ManuallyDrop::drop(&mut self.value) }
        }
    }
}

/// The program icon as a file, for window and tray icons (written once next
/// to the settings; the executable embeds the same image).
pub fn icon_file() -> String {
    static PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let dir = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Jiffshot");
        let path = dir.join("jiffshot.ico");
        let bytes = include_bytes!("../../../assets/jiffshot.ico");
        if std::fs::read(&path).ok().as_deref() != Some(&bytes[..]) {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(&path, bytes);
        }
        path.to_string_lossy().into_owned()
    })
    .clone()
}

/// Decodes `%XX` escapes (file URIs).
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 0 && i + 2 <= bytes.len() - 1 + 0 {
            if let Ok(byte) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Mounts the editor window, shown or hidden. It is the application's main
/// window, so it must be mounted first.
pub fn mount_main_window(scope: &AppScope, model: impl MainViewModel, show: bool) -> Result<()> {
    let binding = MainBinding::new(model)?;
    MAIN.with(|m| *m.borrow_mut() = Some(binding.clone()));
    let window = main_window::build(scope, &binding)?;
    MAIN_WINDOW.with(|m| *m.borrow_mut() = Some((scope.clone(), window.clone())));
    if show {
        scope.mount(window)?;
    } else {
        scope.mount_hidden(window)?;
    }
    // The windows the main view model asks for.
    toast::install(scope)?;
    extras::install(scope)?;
    prefs_window::install(scope)?;
    overlay::install(scope)?;
    hud::install(scope)?;
    // JIFFSHOT_UITEST=DIR: the editor drives itself (see `selftest`).
    if let Some(dir) = std::env::var_os("JIFFSHOT_UITEST").filter(|d| !d.is_empty()) {
        let test_scope = scope.clone();
        scope.post(move || selftest::start(&test_scope, std::path::Path::new(&dir)))?;
    }
    Ok(())
}

/// Mounts the Capture window, hidden.
pub fn mount_capture_window(scope: &AppScope, model: impl CaptureViewModel) -> Result<Window> {
    let binding = CaptureBinding::new(model)?;
    CAPTURE.with(|m| *m.borrow_mut() = Some(binding.clone()));
    let window = capture_view::build(scope, &binding)?;
    scope.mount_hidden(window.clone())?;
    Ok(window)
}

/// Shows the tray icon.
pub fn mount_tray(scope: &AppScope, model: impl TrayViewModel) -> Result<()> {
    let binding = tray::mount(scope, model)?;
    TRAY.with(|t| *t.borrow_mut() = Some(binding));
    Ok(())
}

/// Detaches every view model (the editor saves its work), once the
/// application has stopped running, however it stopped.
pub fn detach_all() {
    if let Some(main) = MAIN.with(|m| m.borrow_mut().take()) {
        main.model.run(|m| m.detach());
    }
    if let Some(capture) = CAPTURE.with(|c| c.borrow_mut().take()) {
        capture.model.run(|m| m.detach());
    }
    if let Some(tray) = TRAY.with(|t| t.borrow_mut().take()) {
        tray.model.run(|m| m.detach());
    }
}

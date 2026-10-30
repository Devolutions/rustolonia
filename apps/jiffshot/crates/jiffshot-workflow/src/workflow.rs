//! What happens after a capture: crop, optional cursor, library save,
//! clipboard and hand-off to the editor. The slow parts (PNG encoding, the
//! clipboard copy) run on a worker thread so the next hotkey is never blocked.

use crate::overlay::{Overlay, Selection};
use crate::settings::Settings;
use jiffshot_capture::library::{timestamp, CaptureKind, CaptureMeta, Library};
use jiffshot_capture::screen::{self, CursorSnapshot};
use jiffshot_capture::targets::process_name;
use jiffshot_capture::{clipboard, Frame, Rect};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct Capture {
    pub frame: Frame,
    pub meta: CaptureMeta,
    /// The pointer as its own image and where it goes (capture pixels),
    /// when it is kept as a separate object.
    pub cursor: Option<(Frame, (i32, i32))>,
    /// The top-level window it came from, to look up a browser's address.
    pub window: Option<isize>,
}

fn meta(kind: CaptureKind, rect: Rect, frame: &Frame, cursor_included: bool) -> CaptureMeta {
    CaptureMeta {
        kind,
        rect,
        width: frame.width,
        height: frame.height,
        captured_at: timestamp(),
        window_title: None,
        process: None,
        cursor_included,
        url: None,
        tags: Vec::new(),
        pinned: false,
    }
}

const BROWSERS: &[&str] = &["chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "vivaldi.exe"];

/// The address in a browser window's address bar (UI Automation: the
/// first edit box, which every major browser puts in its toolbar).
pub fn browser_url(hwnd: isize) -> Option<String> {
    use windows::Win32::Foundation::HWND as WHWND;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_EditControlTypeId, UIA_ValueValuePropertyId};
    // SAFETY: COM calls on this (worker) thread with valid arguments.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let window = automation.ElementFromHandle(WHWND(hwnd as *mut _)).ok()?;
        let edit = automation.CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(UIA_EditControlTypeId.0)).ok()?;
        let bar = window.FindFirst(TreeScope_Descendants, &edit).ok()?;
        let value = bar.GetCurrentPropertyValue(UIA_ValueValuePropertyId).ok()?;
        let text = value.to_string().trim().to_owned();
        if text.is_empty() || text.contains(' ') {
            return None;
        }
        Some(if text.contains("://") || text.starts_with("about:") { text } else { format!("https://{text}") })
    }
}

fn add_cursor(frame: &mut Frame, rect: Rect, cursor: Option<&CursorSnapshot>, include: bool) -> bool {
    match cursor {
        Some(cursor) if include && rect.contains(cursor.x, cursor.y) => cursor.draw_onto(frame, (rect.x, rect.y)).is_ok(),
        _ => false,
    }
}

/// Makes pixels outside `keep` (screen coordinates) transparent.
fn mask_frame(frame: &mut Frame, rect: Rect, keep: impl Fn(i32, i32) -> bool) {
    frame.alpha = true;
    let stride = frame.stride();
    for y in 0..frame.height as i32 {
        for x in 0..frame.width as i32 {
            let a = if keep(rect.x + x, rect.y + y) { 255 } else { 0 };
            frame.pixels[y as usize * stride + x as usize * 4 + 3] = a;
        }
    }
}

/// Even-odd point-in-polygon at a pixel center.
fn inside(points: &[(i32, i32)], x: i32, y: i32) -> bool {
    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
    let n = points.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + n - 1) % n]);
        let (ax, ay, bx, by) = (a.0 as f32, a.1 as f32, b.0 as f32, b.1 as f32);
        if (ay > py) != (by > py) && px < (bx - ax) * (py - ay) / (by - ay) + ax {
            c = !c;
        }
    }
    c
}

/// Crops the overlay's frozen snapshot to the selection.
/// `cursor_object`: keep the pointer as a separate image (for the editor)
/// instead of drawing it in.
pub fn from_overlay(overlay: &Overlay, selection: &Selection, cursor: Option<&CursorSnapshot>, include_cursor: bool, cursor_object: bool) -> Option<Capture> {
    let rect = selection.rect();
    let mut frame = overlay.snapshot.crop(rect.offset(-overlay.origin.0, -overlay.origin.1))?;
    let mut separate = None;
    let cursor_included = match cursor {
        Some(c) if include_cursor && cursor_object && rect.contains(c.x, c.y) => {
            separate = c.image().ok().map(|img| (img, (c.x - rect.x, c.y - rect.y)));
            separate.is_some()
        }
        _ => add_cursor(&mut frame, rect, cursor, include_cursor),
    };
    // Freehand and multi-area captures keep only their shapes.
    match selection {
        Selection::Freehand { points, .. } => {
            mask_frame(&mut frame, rect, |x, y| inside(points, x, y));
        }
        Selection::Multi { regions, .. } => {
            mask_frame(&mut frame, rect, |x, y| regions.iter().any(|r| r.contains(x, y)));
        }
        _ => {}
    }
    let kind = match selection {
        Selection::Freehand { .. } | Selection::Multi { .. } => CaptureKind::Region,
        Selection::Region(_) => CaptureKind::Region,
        Selection::Window { .. } => CaptureKind::Window,
        Selection::Monitor(_) => CaptureKind::Monitor,
        Selection::Desktop(_) => CaptureKind::Desktop,
    };
    let mut meta = meta(kind, rect, &frame, cursor_included);
    // The window it came from: the one chosen, or the one under the
    // middle of a region.
    let target = match selection {
        Selection::Window { target, .. } => Some(*target),
        _ => overlay.targets.hit(rect.x + rect.width / 2, rect.y + rect.height / 2, 0),
    };
    let mut window = None;
    if let Some(target) = target {
        let top = overlay.targets.top_level(target);
        meta.window_title = Some(top.title.clone()).filter(|t| !t.is_empty());
        meta.process = process_name(top.process_id);
        window = Some(top.hwnd);
    }
    Some(Capture { frame, meta, cursor: separate, window })
}

/// Wraps a frame made some other way (scrolling, panoramic, menus).
pub fn from_frame(kind: CaptureKind, rect: Rect, frame: Frame) -> Capture {
    let meta = meta(kind, rect, &frame, false);
    Capture { frame, meta, cursor: None, window: None }
}

/// Grabs a screen rectangle immediately (full screen and repeat-region
/// hotkeys).
pub fn immediate(kind: CaptureKind, rect: Rect, include_cursor: bool) -> std::io::Result<Capture> {
    let cursor = include_cursor.then(CursorSnapshot::take).flatten();
    let mut frame = screen::grab(rect)?;
    let cursor_included = add_cursor(&mut frame, rect, cursor.as_ref(), include_cursor);
    let meta = meta(kind, rect, &frame, cursor_included);
    Ok(Capture { frame, meta, cursor: None, window: None })
}

/// Applies effects (by the editor's names) to a capture.
pub fn apply_effects(frame: &mut Frame, names: &[String]) {
    if names.is_empty() {
        return;
    }
    let mut rgba = Vec::with_capacity(frame.pixels.len());
    for p in frame.pixels.chunks_exact(4) {
        rgba.extend_from_slice(&[p[2], p[1], p[0], if frame.alpha { p[3] } else { 255 }]);
    }
    let Some(pm) = tiny_skia::IntSize::from_wh(frame.width, frame.height).and_then(|s| tiny_skia::Pixmap::from_vec(rgba, s)) else { return };
    let mut doc = jiffshot_doc::Document::new(pm);
    for name in names {
        if let Some(effect) = jiffshot_doc::effects::by_name(name, jiffshot_doc::Rgba::BLACK) {
            jiffshot_doc::effects::apply(&mut doc, effect);
        }
    }
    let flat = doc.flatten();
    let mut out = Frame::new(flat.width(), flat.height());
    out.alpha = flat.pixels().iter().any(|p| p.alpha() < 255);
    for (d, p) in out.pixels.chunks_exact_mut(4).zip(flat.pixels()) {
        let c = p.demultiply();
        d.copy_from_slice(&[c.blue(), c.green(), c.red(), c.alpha()]);
    }
    *frame = out;
}

/// Finds the editor: the configured path, then next to this executable, then
/// the development build under `app/artifacts`.
pub fn find_editor(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured {
        return path.exists().then(|| path.to_path_buf());
    }
    if let Some(path) = std::env::var_os("JIFFSHOT_EDITOR").map(PathBuf::from) {
        if path.exists() {
            return Some(path);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?.to_path_buf();
    let beside = dir.join("jiffshot.exe");
    if beside.exists() {
        return Some(beside);
    }
    for _ in 0..5 {
        let dev = dir.join("app").join("artifacts").join("win-x64").join("jiffshot.exe");
        if dev.exists() {
            return Some(dev);
        }
        dir = dir.parent()?.to_path_buf();
    }
    None
}

/// Starts `program file` without inheriting any of our handles.
/// `std::process::Command` always enables handle inheritance, which would let
/// the editor hold our caller's pipes (or the tray's) open for its lifetime.
fn launch_detached(program: &Path, file: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW};
    let mut command_line = jiffshot_capture::wide(&format!("\"{}\" \"{}\"", program.display(), file.display()));
    let program = jiffshot_capture::wide(&program.to_string_lossy());
    // SAFETY: NUL-terminated strings, a mutable command-line buffer as
    // CreateProcessW requires, and both returned handles closed.
    unsafe {
        // Explicitly empty standard handles, so a console-subsystem editor
        // build cannot pick up our caller's console or pipes either.
        let mut startup: STARTUPINFOW = std::mem::zeroed();
        startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        startup.dwFlags = windows_sys::Win32::System::Threading::STARTF_USESTDHANDLES;
        let mut info: PROCESS_INFORMATION = std::mem::zeroed();
        let ok = CreateProcessW(
            program.as_ptr(),
            command_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            0,
            std::ptr::null(),
            std::ptr::null(),
            &startup,
            &mut info,
        );
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        CloseHandle(info.hThread);
        CloseHandle(info.hProcess);
    }
    Ok(())
}

pub struct Outcome {
    pub message: String,
    pub is_error: bool,
    /// True when the editor took over, so no notification is needed.
    pub opened_editor: bool,
}

/// Saves, copies and opens a capture according to `settings`.
pub fn finish(mut capture: Capture, settings: &Settings, owner: isize) -> Outcome {
    let started = Instant::now();
    apply_effects(&mut capture.frame, &settings.capture_effects);
    if let (Some(hwnd), Some(process)) = (capture.window, capture.meta.process.as_deref()) {
        if BROWSERS.iter().any(|b| process.eq_ignore_ascii_case(b)) {
            capture.meta.url = browser_url(hwnd);
        }
    }
    let library = Library::new(jiffshot_capture::config::library_root());
    let (w, h) = (capture.frame.width, capture.frame.height);
    let path = match library.save(&capture.frame, &capture.meta) {
        Ok(path) => path,
        Err(error) => {
            return Outcome { message: format!("Could not save the capture: {error}"), is_error: true, opened_editor: false }
        }
    };
    if let Some((image, (x, y))) = &capture.cursor {
        // The editor adds these as a movable pointer object.
        let stem = path.with_extension("");
        let _ = image.save_png(&PathBuf::from(format!("{}.cursor.png", stem.display())));
        let _ = std::fs::write(format!("{}.cursor.json", stem.display()), format!("{{\"x\":{x},\"y\":{y}}}"));
    }
    let saved = started.elapsed();
    let mut notes = Vec::new();
    let mut is_error = false;
    if settings.copy_to_clipboard {
        match clipboard::copy(&capture.frame, owner) {
            Ok(()) => notes.push("copied to the clipboard".to_owned()),
            Err(error) => {
                is_error = true;
                notes.push(format!("clipboard failed: {error}"));
            }
        }
    }
    let mut opened_editor = false;
    if settings.open_in_editor && crate::hooks().is_some() {
        opened_editor = open_editor(settings, &path);
    } else if settings.open_in_editor && jiffshot_capture::ipc::send_to_editor(&path) {
        opened_editor = true;
    } else if settings.open_in_editor {
        match find_editor(settings.editor.as_deref()) {
            Some(editor) => match launch_detached(&editor, &path) {
                Ok(()) => opened_editor = true,
                Err(error) => {
                    is_error = true;
                    notes.push(format!("editor failed to start: {error}"));
                }
            },
            None => notes.push("editor not found".to_owned()),
        }
    }
    match share(&settings.share, &path, owner) {
        Ok(Some(note)) => notes.push(note),
        Ok(None) => {}
        Err(error) => {
            is_error = true;
            notes.push(format!("share failed: {error}"));
        }
    }
    let file = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let mut message = format!("{w} × {h} saved as {file}");
    if !notes.is_empty() {
        message = format!("{message}; {}", notes.join("; "));
    }
    crate::log(&format!(
        "capture {w}x{h} saved in {:.0} ms, finished in {:.0} ms: {message}",
        saved.as_secs_f64() * 1000.0,
        started.elapsed().as_secs_f64() * 1000.0
    ));
    Outcome { message, is_error, opened_editor }
}

/// A tiny-skia image (premultiplied RGBA) as a capture frame (BGRA).
pub fn frame_from_pixmap(pm: &tiny_skia::Pixmap) -> Frame {
    let mut frame = Frame::new(pm.width(), pm.height());
    frame.alpha = pm.pixels().iter().any(|p| p.alpha() < 255);
    for (d, p) in frame.pixels.chunks_exact_mut(4).zip(pm.pixels()) {
        let c = p.demultiply();
        d.copy_from_slice(&[c.blue(), c.green(), c.red(), c.alpha()]);
    }
    frame
}

/// Starts the editor with `path` (when no editor is running to take it).
/// Opens `path` in the editor: in this process when Jiffshot's app runs it,
/// otherwise in a running Jiffshot or a new one.
pub fn open_editor(settings: &Settings, path: &Path) -> bool {
    if let Some(hooks) = crate::hooks() {
        return (hooks.open_in_editor)(path);
    }
    jiffshot_capture::ipc::send_to_editor(path) || find_editor(settings.editor.as_deref()).is_some_and(|e| launch_detached(&e, path).is_ok())
}

/// After a recording: opens it in the editor (or says where it is).
pub fn finish_video(path: &Path, _rect: Rect, settings: &Settings, owner: isize) -> Outcome {
    let name = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let mut opened_editor = false;
    if settings.open_in_editor {
        opened_editor = open_editor(settings, path);
    }
    let mut message = format!("Recording saved as {name}");
    let mut is_error = false;
    match share(&settings.video_share, path, owner) {
        Ok(Some(note)) => message = format!("{message}; {note}"),
        Ok(None) => {}
        Err(error) => {
            is_error = true;
            message = format!("{message}; share failed: {error}");
        }
    }
    Outcome { message, is_error, opened_editor }
}

/// Share destinations for captures, as (label, setting) pairs.
pub const SHARES: &[(&str, &str)] = &[
    ("None", "none"),
    ("Word", "word"),
    ("PowerPoint", "powerpoint"),
    ("Excel", "excel"),
    ("OneNote", "onenote"),
    ("Email", "email"),
    ("File…", "file"),
    ("Program…", "program"),
    ("Clipboard", "clipboard"),
    ("Printer", "print"),
];
/// The ones that take a video.
pub const VIDEO_SHARES: &[(&str, &str)] = &[("None", "none"), ("PowerPoint", "powerpoint"), ("Email", "email"), ("File…", "file"), ("Program…", "program")];

/// Sends a saved capture to `to` (a [`SHARES`] setting); says what it did.
pub fn share(to: &str, path: &Path, owner: isize) -> Result<Option<String>, String> {
    use jiffshot_capture::share;
    let office = |exe: &str, app: &str| share::open_in(owner, exe, path).map(|_| Some(format!("opened in {app}")));
    match to {
        "" | "none" => Ok(None),
        "file" => match save_copy_dialog(owner, path) {
            Some(target) => std::fs::copy(path, &target).map(|_| Some(format!("copied to {}", target.display()))).map_err(|e| e.to_string()),
            None => Ok(None),
        },
        "email" => {
            let subject = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            share::email(owner, path, &subject).map(|_| Some("opened a new message".to_owned()))
        }
        "word" => office("winword.exe", "Word"),
        "powerpoint" => office("powerpnt.exe", "PowerPoint"),
        "excel" => office("excel.exe", "Excel"),
        "onenote" => office("onenote.exe", "OneNote"),
        "paint" => office("mspaint.exe", "Paint"),
        "clipboard" => {
            let image = jiffshot_doc::io::load_image(path).map_err(|e| e.to_string())?;
            jiffshot_capture::clipboard::copy(&frame_from_pixmap(&image), owner).map(|_| Some("copied to the clipboard".to_owned())).map_err(|e| e.to_string())
        }
        "program" => share::open_with(owner, path).map(|_| None),
        "print" => share::print(owner, path).map(|_| Some("sent to the printer".to_owned())),
        other => Err(format!("unknown share destination {other}")),
    }
}

/// Asks where to save a copy of `path` (same type).
fn save_copy_dialog(owner: isize, path: &Path) -> Option<PathBuf> {
    use jiffshot_capture::{from_wide, wide};
    use windows_sys::Win32::UI::Controls::Dialogs::{GetSaveFileNameW, OFN_OVERWRITEPROMPT, OPENFILENAMEW};
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "png".into());
    let mut buf = vec![0u16; 1024];
    let name = wide(&path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    buf[..name.len().min(1023)].copy_from_slice(&name[..name.len().min(1023)]);
    let filter: Vec<u16> = format!("{} file (*.{ext}) *.{ext}  ", ext.to_uppercase()).encode_utf16().collect();
    let def = wide(&ext);
    // SAFETY: a plain C struct; zero is valid for every field.
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner as _;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = buf.as_mut_ptr();
    ofn.nMaxFile = buf.len() as u32;
    ofn.lpstrDefExt = def.as_ptr();
    ofn.Flags = OFN_OVERWRITEPROMPT;
    // SAFETY: every pointer in `ofn` outlives the modal call.
    let ok = unsafe { GetSaveFileNameW(&mut ofn) };
    (ok != 0).then(|| PathBuf::from(from_wide(&buf)))
}

//! On-screen furniture for captures that take time (scrolling, panoramic,
//! video): an outline around the captured area, drawn just outside it so it
//! never ends up in the capture, and a small control panel with buttons.
//!
//! The app draws both (Avalonia windows, left out of screen captures); this
//! module keeps what is on screen and sends all of it, as JSON, through
//! [`crate::Hooks::hud`] after every change. The panel's buttons answer
//! through [`answer`]; a toolbar's (the recording toolbar, [`run_toolbar`])
//! send named actions through [`act`].

use jiffshot_capture::Rect;
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT, VK_ESCAPE};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, PostQuitMessage, TranslateMessage, MSG, PM_REMOVE,
    QS_ALLINPUT, WM_HOTKEY, WM_QUIT,
};

/// A control panel next to `area` (desktop pixels).
#[derive(Clone, Debug, PartialEq, Serialize)]
struct PanelView {
    id: u64,
    area: [i32; 4],
    status: String,
    buttons: Vec<String>,
    /// A toolbar's state instead of a status and buttons (see
    /// [`run_toolbar`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    toolbar: Option<Value>,
}

/// An outline around `area` (desktop pixels), outside it.
#[derive(Clone, Debug, PartialEq, Serialize)]
struct BorderView {
    id: u64,
    area: [i32; 4],
}

/// Everything on screen, as the app receives it.
#[derive(Default, Serialize)]
struct Hud {
    panels: Vec<PanelView>,
    borders: Vec<BorderView>,
}

static HUD: Mutex<Hud> = Mutex::new(Hud { panels: Vec::new(), borders: Vec::new() });
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn xywh(r: Rect) -> [i32; 4] {
    [r.x, r.y, r.width, r.height]
}

/// Applies `f` and, when it changed anything, sends the new state. The lock
/// is held while sending, so the app sees changes in order.
fn change(f: impl FnOnce(&mut Hud) -> bool) {
    let mut hud = HUD.lock().unwrap_or_else(|e| e.into_inner());
    if f(&mut hud) {
        if let (Some(hooks), Ok(json)) = (crate::hooks(), serde_json::to_string(&*hud)) {
            (hooks.hud)(json);
        }
    }
}

/// A red outline just outside `rect`; click-through, removed on drop. Safe
/// to use from any thread.
pub struct Border {
    id: u64,
}

impl Border {
    pub fn show(rect: Rect) -> Border {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        change(|hud| {
            hud.borders.push(BorderView { id, area: xywh(rect) });
            true
        });
        Border { id }
    }
}

impl Drop for Border {
    fn drop(&mut self) {
        let id = self.id;
        change(|hud| {
            let before = hud.borders.len();
            hud.borders.retain(|b| b.id != id);
            hud.borders.len() != before
        });
    }
}

/// A key that presses a panel button while the panel shows, wherever the
/// focus is: `MOD_*` modifiers, a virtual key, and the button's index.
#[derive(Clone, Copy)]
pub struct Key {
    pub modifiers: u32,
    pub vk: u16,
    pub button: usize,
}

/// Esc, then the panel's own keys: thread hotkey ids from here, a block
/// of `KEYS_PER_PANEL` per open panel.
const HOTKEY_BASE: i32 = 0x5350;
const KEYS_PER_PANEL: usize = 8;

/// A panel or toolbar running on this thread.
struct Open {
    id: u64,
    answer: Option<Option<usize>>,
    /// A toolbar's actions, waiting to be handled.
    actions: Vec<String>,
}

thread_local! {
    /// The panels running on this thread, innermost last.
    static OPEN: RefCell<Vec<Open>> = const { RefCell::new(Vec::new()) };
}

fn set_answer(id: Option<u64>, result: Option<usize>) {
    OPEN.with(|open| {
        let mut open = open.borrow_mut();
        let panel = match id {
            Some(id) => open.iter_mut().find(|p| p.id == id),
            None => open.last_mut(),
        };
        if let Some(panel) = panel {
            panel.answer.get_or_insert(result);
        }
    });
}

fn push_action(id: u64, action: String) {
    OPEN.with(|open| {
        if let Some(panel) = open.borrow_mut().iter_mut().find(|p| p.id == id) {
            panel.actions.push(action);
        }
    });
}

/// A toolbar button of panel `id` sent `action`; from the app, on any thread.
pub fn act(id: u64, action: String) {
    crate::on_capture_thread(move || push_action(id, action));
}

/// A button of panel `id` was pressed (`None`: the panel was closed); from
/// the app, on any thread.
pub fn answer(id: u64, button: Option<usize>) {
    crate::on_capture_thread(move || set_answer(Some(id), button));
}

/// Ends the running panel as if button `result` (or Esc, for `None`) was
/// pressed; for use from its `tick`.
pub fn finish_panel(result: Option<usize>) {
    set_answer(None, result);
}

/// Shows a panel next to `area` and waits for an answer while this thread's
/// messages keep flowing, calling `tick` every `tick_ms` (its return value,
/// if any, becomes the status text). Returns the index of the button
/// pressed, or `None` for Esc.
pub fn run_panel(area: Rect, status: &str, buttons: &[&str], tick_ms: u32, tick: impl FnMut() -> Option<String>) -> Option<usize> {
    run_panel_with_keys(area, status, buttons, &[], tick_ms, tick)
}

/// [`run_panel`], with `keys` that press buttons too.
pub fn run_panel_with_keys(
    area: Rect,
    status: &str,
    buttons: &[&str],
    keys: &[Key],
    tick_ms: u32,
    mut tick: impl FnMut() -> Option<String>,
) -> Option<usize> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let depth = open(id);
    change(|hud| {
        hud.panels.push(PanelView { id, area: xywh(area), status: status.to_owned(), buttons: buttons.iter().map(|b| b.to_string()).collect(), toolbar: None });
        true
    });
    let base = HOTKEY_BASE + (depth * KEYS_PER_PANEL) as i32;
    let keys = &keys[..keys.len().min(KEYS_PER_PANEL - 1)];
    // SAFETY: thread hotkeys (no window), released below; a key some other
    // program holds just stays unregistered.
    unsafe {
        RegisterHotKey(std::ptr::null_mut(), base, MOD_NOREPEAT, VK_ESCAPE as u32);
        for (i, key) in keys.iter().enumerate() {
            RegisterHotKey(std::ptr::null_mut(), base + 1 + i as i32, key.modifiers | MOD_NOREPEAT, key.vk as u32);
        }
    }
    let every = Duration::from_millis(tick_ms.max(1) as u64);
    let mut next = Instant::now() + every;
    let mut quit = false;
    let result = loop {
        if let Some(result) = OPEN.with(|open| open.borrow().iter().find(|p| p.id == id).and_then(|p| p.answer)) {
            break result;
        }
        if quit {
            break None;
        }
        let now = Instant::now();
        if now >= next {
            if let Some(status) = tick() {
                change(|hud| match hud.panels.iter_mut().find(|p| p.id == id) {
                    Some(p) if p.status != status => {
                        p.status = status;
                        true
                    }
                    _ => false,
                });
            }
            next = Instant::now() + every;
            continue;
        }
        let wait = (next - now).as_millis().clamp(1, u32::MAX as u128) as u32;
        // SAFETY: a plain wait and message pump for this thread.
        unsafe {
            MsgWaitForMultipleObjects(0, std::ptr::null(), 0, wait, QS_ALLINPUT);
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    quit = true;
                    break;
                }
                if msg.message == WM_HOTKEY && msg.hwnd.is_null() {
                    let index = msg.wParam as i32 - base;
                    if index == 0 {
                        set_answer(Some(id), None);
                    } else if index > 0 && (index as usize) <= keys.len() {
                        set_answer(Some(id), Some(keys[index as usize - 1].button));
                    }
                    continue;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    };
    // SAFETY: the hotkeys registered above, on this thread.
    unsafe {
        for i in 0..=keys.len() as i32 {
            UnregisterHotKey(std::ptr::null_mut(), base + i);
        }
        if quit {
            PostQuitMessage(0);
        }
    }
    close(id);
    result
}

/// Registers panel `id` on this thread; returns how many are open under it.
fn open(id: u64) -> usize {
    OPEN.with(|open| {
        let mut open = open.borrow_mut();
        open.push(Open { id, answer: None, actions: Vec::new() });
        open.len() - 1
    })
}

fn close(id: u64) {
    OPEN.with(|open| open.borrow_mut().retain(|p| p.id != id));
    change(|hud| {
        hud.panels.retain(|p| p.id != id);
        true
    });
}

/// Shows a toolbar below `area` (the recording toolbar) and runs until an
/// action ends it, with this thread's messages flowing. Its buttons send
/// named actions; `act` handles each and returns true for one that ends the
/// toolbar. `keys` send actions too, from anywhere (`MOD_*`, virtual key,
/// action); Esc sends "cancel". `state` (JSON, drawn by the app) is taken
/// again every `tick_ms` and after each action; a call of [`finish_panel`]
/// from it ends the toolbar with "stop". Returns the action that ended it.
pub fn run_toolbar(
    area: Rect,
    keys: &[(u32, u16, &'static str)],
    tick_ms: u32,
    mut state: impl FnMut() -> Value,
    mut act: impl FnMut(&str) -> bool,
) -> String {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let depth = open(id);
    let first = state();
    change(|hud| {
        hud.panels.push(PanelView { id, area: xywh(area), status: String::new(), buttons: Vec::new(), toolbar: Some(first) });
        true
    });
    let base = HOTKEY_BASE + (depth * KEYS_PER_PANEL) as i32;
    let keys = &keys[..keys.len().min(KEYS_PER_PANEL - 1)];
    // SAFETY: as in `run_panel_with_keys`.
    unsafe {
        RegisterHotKey(std::ptr::null_mut(), base, MOD_NOREPEAT, VK_ESCAPE as u32);
        for (i, (modifiers, vk, _)) in keys.iter().enumerate() {
            RegisterHotKey(std::ptr::null_mut(), base + 1 + i as i32, modifiers | MOD_NOREPEAT, *vk as u32);
        }
    }
    let publish = |value: Value| {
        change(|hud| match hud.panels.iter_mut().find(|p| p.id == id) {
            Some(p) if p.toolbar.as_ref() != Some(&value) => {
                p.toolbar = Some(value);
                true
            }
            _ => false,
        })
    };
    let every = Duration::from_millis(tick_ms.max(1) as u64);
    let mut next = Instant::now() + every;
    let mut quit = false;
    let ended = 'run: loop {
        let actions = OPEN.with(|open| open.borrow_mut().iter_mut().find(|p| p.id == id).map(|p| std::mem::take(&mut p.actions)).unwrap_or_default());
        for action in &actions {
            if act(action) {
                break 'run action.clone();
            }
        }
        if !actions.is_empty() {
            publish(state());
        }
        if OPEN.with(|open| open.borrow().iter().find(|p| p.id == id).is_some_and(|p| p.answer.is_some())) {
            break "stop".to_owned();
        }
        if quit {
            break "cancel".to_owned();
        }
        let now = Instant::now();
        if now >= next {
            publish(state());
            next = Instant::now() + every;
            continue;
        }
        let wait = (next - now).as_millis().clamp(1, u32::MAX as u128) as u32;
        // SAFETY: a plain wait and message pump for this thread.
        unsafe {
            MsgWaitForMultipleObjects(0, std::ptr::null(), 0, wait, QS_ALLINPUT);
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    quit = true;
                    break;
                }
                if msg.message == WM_HOTKEY && msg.hwnd.is_null() {
                    let index = msg.wParam as i32 - base;
                    if index == 0 {
                        push_action(id, "cancel".into());
                    } else if index > 0 && (index as usize) <= keys.len() {
                        push_action(id, keys[index as usize - 1].2.into());
                    }
                    continue;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    };
    // SAFETY: the hotkeys registered above, on this thread.
    unsafe {
        for i in 0..=keys.len() as i32 {
            UnregisterHotKey(std::ptr::null_mut(), base + i);
        }
        if quit {
            PostQuitMessage(0);
        }
    }
    close(id);
    ended
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tick_can_finish_the_panel_and_the_state_is_cleared() {
        let mut ticks = 0;
        let result = run_panel(Rect::new(10, 20, 300, 200), "Counting", &["Start now", "Cancel"], 5, || {
            ticks += 1;
            if ticks == 3 {
                finish_panel(Some(1));
            }
            Some(format!("tick {ticks}"))
        });
        assert_eq!(result, Some(1));
        assert_eq!(ticks, 3);
        let hud = HUD.lock().unwrap();
        assert!(hud.panels.iter().all(|p| p.status != "tick 3"));
    }

    #[test]
    fn a_toolbar_handles_actions_until_one_ends_it() {
        let mut seen = Vec::new();
        let mut ticks = 0;
        let ended = run_toolbar(
            Rect::new(0, 0, 100, 100),
            &[],
            5,
            || {
                ticks += 1;
                // The app's buttons, as they would arrive.
                if ticks == 2 {
                    let id = OPEN.with(|o| o.borrow().last().map(|p| p.id)).unwrap();
                    push_action(id, "toggle:mic".into());
                    push_action(id, "stop".into());
                    push_action(id, "never".into());
                }
                serde_json::json!({ "ticks": ticks })
            },
            |action| {
                seen.push(action.to_owned());
                action == "stop"
            },
        );
        assert_eq!(ended, "stop");
        assert_eq!(seen, ["toggle:mic", "stop"]);
        // (Other tests' panels may be up at the same time.)
        assert!(HUD.lock().unwrap().panels.iter().all(|p| p.toolbar.is_none()));
    }

    #[test]
    fn borders_come_and_go_and_serialize_their_area() {
        let border = Border::show(Rect::new(1, 2, 3, 4));
        let json = serde_json::to_string(&*HUD.lock().unwrap()).unwrap();
        assert!(json.contains(&format!("{{\"id\":{},\"area\":[1,2,3,4]}}", border.id)), "{json}");
        let id = border.id;
        drop(border);
        assert!(HUD.lock().unwrap().borders.iter().all(|b| b.id != id));
    }
}

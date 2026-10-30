//! The selection overlay (the `overlay` signal), one window per monitor, and
//! Screen Draw (the `screen_draw` signal).
//!
//! The overlay's windows are made ahead of time (at startup, see
//! [`install`]) and only hidden natively between captures, so putting the
//! overlay up is loading the new screenshot, letting it render, and a
//! native show, not a full Avalonia show. They are taken down as soon as the
//! selection is made, and again when the capture side says so. The capture
//! side owns the selection; the windows send it their input and draw what
//! it says (see [`overlay_api`]).

pub(super) mod overlay_api;
mod overlay_view;

use super::hud::native;
use jiffshot_capture::Rect;
use overlay_api as api;
use overlay_view::{MonitorWindow, OverlayMonitor, Scene};
use rustolonia::pixel_surface::{Cursor, Interpolation, MouseButton, PixelFormat, Placement, PointerKind, SurfaceInput};
use rustolonia::{AppScope, Brush, Color, Effect, PixelSurface, Result, Window, WindowDecorations};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// How long a freshly loaded screenshot gets to render before the windows
/// show, so the last capture's never flashes. The host has no "frame
/// rendered" notification; a timer this short completes on a later
/// dispatcher turn, measured at 20–30 ms after the load (about a frame).
const FRAME_WAIT: Duration = Duration::from_millis(1);

/// Screen Draw's fading marks are re-rendered this often.
const FADE_TICK: Duration = Duration::from_millis(40);

/// Screen Draw's background: barely there, so the pointer stays on it.
const DRAW_BACKGROUND: u32 = 0x0100_0000;

thread_local! {
    static HOST: RefCell<Host> = RefCell::new(Host::default());
    static REDRAW_POSTED: Cell<bool> = const { Cell::new(false) };
    static DRAW: RefCell<Option<ScreenDraw>> = const { RefCell::new(None) };
}

/// One showing of the overlay: the capture side's session, its monitors and
/// what they show.
struct Session {
    id: u64,
    monitors: Vec<OverlayMonitor>,
    scene: Scene,
}

impl Session {
    /// Sends input (desktop pixels) and takes the new state; true once the
    /// selection is made.
    fn input(&mut self, kind: i32, x: f64, y: f64, value: i32, mods: i32) -> bool {
        self.scene.state = api::input(self.id, kind, x, y, value, mods);
        self.done()
    }

    fn done(&self) -> bool {
        self.scene.state.has(api::FLAG_DONE)
    }

    /// Fetches the texts, and the points when their counts changed.
    fn refresh(&mut self) {
        let (id, st) = (self.id, self.scene.state);
        self.scene.title = api::text(id, api::TEXT_TITLE);
        self.scene.size = api::text(id, api::TEXT_SIZE);
        if self.scene.hint.is_empty() {
            self.scene.hint = api::text(id, api::TEXT_HINT);
        }
        let path_len = st.path_count.max(0) as usize * 2;
        if self.scene.path.len() != path_len {
            self.scene.path = api::points(id, api::POINTS_PATH, path_len);
        }
        let regions_len = st.region_count.max(0) as usize * 4;
        if self.scene.regions.len() != regions_len {
            self.scene.regions = api::points(id, api::POINTS_REGIONS, regions_len);
        }
    }
}

/// The overlay's windows and the session showing, on the UI thread.
#[derive(Default)]
struct Host {
    scope: Option<AppScope>,
    windows: Vec<MonitorWindow>,
    session: Option<Session>,
}

impl Host {
    fn scope(&self) -> Option<AppScope> {
        self.scope.clone()
    }

    /// The window over `monitor`, made (and shown once, hidden) if needed.
    fn window_for(&mut self, scope: &AppScope, monitor: OverlayMonitor) -> Result<usize> {
        if let Some(i) = self.windows.iter().position(|w| w.rect == monitor.rect) {
            return Ok(i);
        }
        let rect = monitor.rect;
        let window = MonitorWindow::new(scope, monitor, move |input| Ok(on_input(rect, input)))?;
        // Shown once, cloaked so nothing flashes, then hidden natively:
        // Avalonia keeps it as a live window from then on. Shown on its own
        // screen, it takes that screen's scale now rather than on the first
        // capture.
        let early = window.hwnd(&scope);
        native::cloak(early, true);
        native::place(early, rect, false);
        window.window.show()?;
        let hwnd = window.hwnd(&scope);
        if early == 0 {
            native::cloak(hwnd, true);
        }
        native::place(hwnd, rect, true);
        scope.post(move || {
            native::place(hwnd, rect, false);
            native::hide(hwnd);
            native::cloak(hwnd, false);
        })?;
        self.windows.push(window);
        Ok(self.windows.len() - 1)
    }

    /// Draws the session (if any) on every window.
    fn draw(&mut self) {
        let Some(session) = self.session.as_mut() else { return };
        session.refresh();
        for window in &mut self.windows {
            if session.monitors.iter().any(|m| m.rect == window.rect) {
                if let Err(error) = window.draw(&session.scene) {
                    eprintln!("overlay: drawing failed: {error:?}");
                }
            }
        }
    }

    /// Input on the window over `rect`; returns whether it was handled and
    /// whether the selection is now made.
    fn input(&mut self, rect: Rect, input: &SurfaceInput) -> (bool, bool) {
        let Some(window) = self.windows.iter_mut().find(|w| w.rect == rect) else {
            return (false, false);
        };
        if let SurfaceInput::Resize(metrics) = input {
            // Windows moves and resizes a window whose screen scale
            // changed; put it back (after Avalonia has applied the size
            // Windows suggests).
            let changed = window.reported_scaling != 0.0 && window.reported_scaling != metrics.scaling;
            window.reported_scaling = metrics.scaling;
            if changed {
                if let Some(scope) = &self.scope {
                    let hwnd = window.hwnd(scope);
                    let _ = scope.post(move || native::place(hwnd, rect, false));
                }
                schedule_redraw(self.scope.as_ref());
            }
            return (false, false);
        }
        let Some(session) = self.session.as_mut().filter(|s| !s.done()) else {
            return (false, false);
        };
        let (kind, x, y, value, mods) = match input {
            SurfaceInput::Pointer(p) => {
                let (x, y) = window.desktop(p.x, p.y);
                let mods = api::modifiers(p.modifiers);
                match p.kind {
                    PointerKind::Pressed if p.button == MouseButton::Right => (api::CANCEL, x, y, 0, 0),
                    PointerKind::Pressed if p.button == MouseButton::Left => (api::PRESS, x, y, 0, mods),
                    PointerKind::Moved => (api::MOVE, x, y, 0, mods),
                    PointerKind::Released if p.button == MouseButton::Left => (api::RELEASE, x, y, 0, mods),
                    PointerKind::Wheel if p.delta_y != 0.0 => (api::WHEEL, x, y, if p.delta_y > 0.0 { 120 } else { -120 }, 0),
                    PointerKind::Pressed | PointerKind::Released => return (true, false),
                    _ => return (false, false),
                }
            }
            // Keys go to the window that has the focus, at the pointer.
            SurfaceInput::Key { down, key, modifiers } => {
                let vk = if *down { api::virtual_key(*key) } else { 0 };
                let [cx, cy] = session.scene.state.cursor;
                (
                    if vk != 0 { api::KEY } else { api::REFRESH },
                    cx as f64,
                    cy as f64,
                    vk,
                    api::modifiers(*modifiers),
                )
            }
            _ => return (false, false),
        };
        if session.input(kind, x, y, value, mods) {
            return (true, true);
        }
        schedule_redraw(self.scope.as_ref());
        (true, false)
    }
}

/// Redraws every window once the input queued now is in: the pointer can
/// report many moves per frame.
fn schedule_redraw(scope: Option<&AppScope>) {
    if REDRAW_POSTED.with(|p| p.replace(true)) {
        return;
    }
    let posted = scope.is_some_and(|scope| scope.post(redraw).is_ok());
    if !posted {
        REDRAW_POSTED.with(|p| p.set(false));
    }
}

fn redraw() {
    REDRAW_POSTED.with(|p| p.set(false));
    let busy = HOST.with(|h| match h.try_borrow_mut() {
        Ok(mut host) => {
            host.draw();
            None
        }
        Err(_) => h.try_borrow().ok().and_then(|host| host.scope()),
    });
    if let Some(scope) = busy {
        schedule_redraw(Some(&scope));
    }
}

/// The input surface's handler for the window over `rect`.
fn on_input(rect: Rect, input: &SurfaceInput) -> bool {
    let (handled, done) = HOST.with(|h| match h.try_borrow_mut() {
        Ok(mut host) => host.input(rect, input),
        Err(_) => (false, false),
    });
    if done {
        // The selection is made: hide at once.
        end();
    }
    handled
}

/// The announcement: `{"id","origin","size","cursor":[x,y],"monitors":[{"rect":[x,y,w,h],"scale","primary"}]}`.
fn parse_announcement(json: &str) -> Option<(u64, Vec<OverlayMonitor>, (i32, i32))> {
    let root: Value = serde_json::from_str(json).ok()?;
    let id = root.get("id")?.as_u64()?;
    let int = |v: &Value| v.as_i64().map(|v| v as i32);
    let monitors = root
        .get("monitors")?
        .as_array()?
        .iter()
        .filter_map(|m| {
            let r = m.get("rect")?.as_array()?;
            let [x, y, w, h] = [r.first()?, r.get(1)?, r.get(2)?, r.get(3)?].map(int);
            Some(OverlayMonitor {
                rect: Rect::new(x?, y?, w?, h?),
                scale: m.get("scale").and_then(Value::as_f64).unwrap_or(1.0),
            })
        })
        .filter(|m| !m.rect.is_empty())
        .collect();
    let cursor = root.get("cursor")?.as_array()?;
    Some((id, monitors, (int(cursor.first()?)?, int(cursor.get(1)?)?)))
}

/// The announcement JSON, or empty to take the overlay down.
fn apply(json: &str) {
    end();
    if json.is_empty() {
        return;
    }
    let Some((id, monitors, cursor)) = parse_announcement(json) else {
        eprintln!("overlay: a bad announcement");
        return;
    };
    let shown = HOST.with(|h| {
        let Ok(mut host) = h.try_borrow_mut() else { return None };
        let scope = host.scope()?;
        let mut session = Session {
            id,
            monitors: monitors.clone(),
            scene: Scene::default(),
        };
        session.scene.help = api::text(id, api::TEXT_HELP).split('\n').map(str::to_owned).collect();
        session.scene.guides = api::points(id, api::POINTS_GUIDES, 16);
        if session.input(api::REFRESH, cursor.0 as f64, cursor.1 as f64, 0, 0) {
            return None;
        }
        for &monitor in &monitors {
            let i = match host.window_for(&scope, monitor) {
                Ok(i) => i,
                Err(error) => {
                    eprintln!("overlay: no window for {:?}: {error:?}", monitor.rect);
                    continue;
                }
            };
            match host.windows[i].load(id) {
                Ok(true) => {}
                Ok(false) => return None,
                Err(error) => eprintln!("overlay: loading the screenshot failed: {error:?}"),
            }
        }
        // Windows of monitors that are gone.
        host.windows.retain(|w| {
            let keep = monitors.iter().any(|m| m.rect == w.rect);
            if !keep {
                w.close();
            }
            keep
        });
        host.session = Some(session);
        host.draw();
        Some(scope)
    });
    // Show once the new screenshot is drawn, so the last one never flashes.
    if let Some(scope) = shown {
        let waiter = scope.clone();
        // Scope tasks are polled on the UI thread.
        let started = scope.spawn(async move {
            if let Ok(delay) = waiter.delay(FRAME_WAIT) {
                let _ = delay.await;
            }
            show(id, cursor);
        });
        if started.is_err() {
            show(id, cursor);
        }
    }
}

/// Puts session `id`'s windows on screen, the one under the pointer with
/// the keyboard.
fn show(id: u64, cursor: (i32, i32)) {
    let Some((windows, focus)) = HOST.with(|h| {
        let host = h.try_borrow().ok()?;
        let session = host.session.as_ref().filter(|s| s.id == id && !s.done())?;
        let scope = host.scope()?;
        let mut windows = Vec::new();
        let mut focus = None;
        for w in host.windows.iter().filter(|w| session.monitors.iter().any(|m| m.rect == w.rect)) {
            let hwnd = w.hwnd(&scope);
            windows.push((hwnd, w.rect));
            if w.rect.contains(cursor.0, cursor.1) {
                focus = Some((hwnd, w.input.clone()));
            }
        }
        Some((windows, focus))
    }) else {
        return;
    };
    for (hwnd, rect) in windows {
        native::place(hwnd, rect, true);
    }
    if let Some((hwnd, input)) = focus {
        native::activate(hwnd);
        let _ = input.focus();
    }
    api::shown(id);
}

/// Takes the overlay down (the windows stay, hidden).
fn end() {
    let hwnds = HOST.with(|h| {
        let Ok(mut host) = h.try_borrow_mut() else { return Vec::new() };
        if host.session.take().is_none() {
            return Vec::new();
        }
        let Some(scope) = host.scope() else { return Vec::new() };
        host.windows
            .iter_mut()
            .map(|w| {
                let _ = w.unload();
                w.hwnd(&scope)
            })
            .collect()
    });
    for hwnd in hwnds {
        native::hide(hwnd);
    }
}

/// Makes (or keeps) a hidden window per screen, so the first capture is
/// quick too.
fn prepare() {
    let monitors = jiffshot_capture::screen::monitors();
    HOST.with(|h| {
        let Ok(mut host) = h.try_borrow_mut() else { return };
        let Some(scope) = host.scope() else { return };
        for m in monitors {
            if let Err(error) = host.window_for(
                &scope,
                OverlayMonitor {
                    rect: m.rect,
                    scale: m.scale() as f64,
                },
            ) {
                eprintln!("overlay: no window for {:?}: {error:?}", m.rect);
            }
        }
    });
}

// ---- Screen Draw ------------------------------------------------------------------

/// Screen Draw: a see-through window over the area being recorded, showing
/// the marks the capture side renders and sending it the pointer and keys.
/// It is left out of captures; the recorder draws the marks into the video.
struct ScreenDraw {
    id: u64,
    area: Rect,
    window: Window,
    surface: PixelSurface,
    /// The marks, BGRA, the area's size.
    pixels: Vec<u8>,
    /// Cleared when the window goes, which ends the fade ticks.
    alive: Arc<AtomicBool>,
}

impl ScreenDraw {
    /// Copies the marks onto the window.
    fn refresh(&mut self) {
        let (w, h) = (self.area.width.max(1) as u32, self.area.height.max(1) as u32);
        self.pixels.resize(w as usize * h as usize * 4, 0);
        if !api::draw_copy(self.id, &mut self.pixels, w, h) {
            return;
        }
        let shown = self
            .surface
            .set_layer(0, w, h, PixelFormat::Bgra8)
            .and_then(|_| self.surface.write_layer(0, 0, 0, w, h, &self.pixels, w as usize * 4))
            .and_then(|_| self.surface.place_layer(0, Placement::Fill, Interpolation::None, 1.0));
        if let Err(error) = shown {
            eprintln!("screen draw: {error:?}");
        }
    }

    fn close(&self) {
        self.alive.store(false, Ordering::Release);
        let _ = self.surface.detach();
        let _ = self.window.close();
    }
}

/// Screen Draw's input; true when handled.
fn on_draw_input(id: u64, input: &SurfaceInput) -> bool {
    let mut cancel = false;
    let handled = DRAW.with(|d| {
        let Ok(mut current) = d.try_borrow_mut() else { return false };
        let Some(draw) = current.as_mut().filter(|d| d.id == id) else { return false };
        let changed = match input {
            SurfaceInput::Pointer(p) => {
                let s = draw.surface.metrics().map(|m| m.scaling).unwrap_or(1.0);
                let (x, y) = (p.x * s, p.y * s);
                match p.kind {
                    PointerKind::Pressed if p.button == MouseButton::Right => {
                        cancel = true;
                        api::draw_input(id, api::CANCEL, x, y, 0)
                    }
                    PointerKind::Pressed => api::draw_input(id, api::PRESS, x, y, 0),
                    PointerKind::Moved => api::draw_input(id, api::MOVE, x, y, 0),
                    PointerKind::Released if p.button == MouseButton::Left => api::draw_input(id, api::RELEASE, x, y, 0),
                    _ => return false,
                }
            }
            SurfaceInput::Key { down: true, key, .. } => match api::virtual_key(*key) {
                0 => return false,
                vk => {
                    api::draw_input(id, api::KEY, 0.0, 0.0, vk);
                    true
                }
            },
            _ => return false,
        };
        if changed && !cancel {
            draw.refresh();
        }
        true
    });
    if cancel {
        // Drawing ended (the capture side announces it too).
        close_draw();
    }
    handled
}

/// Input for the overlay showing (desktop pixels), as its windows would send
/// it, for the UI self-test; the new state, or `None` with no overlay.
pub(super) fn feed(kind: i32, x: f64, y: f64, value: i32, mods: i32) -> Option<api::OverlayState> {
    let (state, scope) = HOST.with(|h| {
        let mut host = h.try_borrow_mut().ok()?;
        let scope = host.scope();
        let session = host.session.as_mut()?;
        session.input(kind, x, y, value, mods);
        session.refresh();
        Some((session.scene.state, scope))
    })?;
    schedule_redraw(scope.as_ref());
    Some(state)
}

/// Input for Screen Draw (area pixels), as its window would send it, for
/// the UI self-test.
pub(super) fn feed_draw(kind: i32, x: f64, y: f64, value: i32) {
    DRAW.with(|d| {
        if let Ok(mut current) = d.try_borrow_mut() {
            if let Some(draw) = current.as_mut() {
                if api::draw_input(draw.id, kind, x, y, value) || kind == api::KEY {
                    draw.refresh();
                }
            }
        }
    });
}

fn close_draw() {
    if let Some(draw) = DRAW.with(|d| d.try_borrow_mut().ok().and_then(|mut d| d.take())) {
        draw.close();
    }
}

/// `{"id","area":[x,y,w,h]}` shows Screen Draw; empty ends it.
fn apply_draw(scope: &AppScope, json: &str) {
    close_draw();
    if json.is_empty() {
        return;
    }
    let parsed = serde_json::from_str::<Value>(json).ok().and_then(|root| {
        let id = root.get("id")?.as_u64()?;
        let a = root.get("area")?.as_array()?;
        let v = |i: usize| a.get(i).and_then(Value::as_i64).map(|v| v as i32);
        Some((id, Rect::new(v(0)?, v(1)?, v(2)?, v(3)?)))
    });
    let Some((id, area)) = parsed.filter(|(_, a)| !a.is_empty()) else {
        eprintln!("screen draw: a bad announcement");
        return;
    };
    if let Err(error) = open_draw(scope, id, area) {
        eprintln!("screen draw: {error:?}");
    }
}

fn open_draw(scope: &AppScope, id: u64, area: Rect) -> Result<()> {
    let surface = scope.create_pixel_surface(move |input| Ok(on_draw_input(id, input)))?;
    surface.set_background(Some(DRAW_BACKGROUND))?;
    surface.set_cursor(Cursor::Cross)?;
    let (_, scale) = native::screen_of(area);
    let window = Window::new()?
        .title("Jiffshot Screen Draw")?
        .window_decorations(WindowDecorations::None)?
        .show_in_taskbar(false)?
        .can_resize(false)?
        .width(area.width as f64 / scale)?
        .height(area.height as f64 / scale)?
        .position((area.x, area.y))?
        .background(Brush::solid(Color::new(0, 0, 0, 0)))?
        .content(Some(surface.control()))?;
    scope.set_transparent(&window, true)?;
    scope.set_topmost(&window, true)?;
    window.show()?;
    let hwnd = native::handle(scope, &window);
    native::place(hwnd, area, false);
    native::exclude_from_capture(scope, &window);
    let alive = Arc::new(AtomicBool::new(true));
    let mut draw = ScreenDraw {
        id,
        area,
        window,
        surface: surface.clone(),
        pixels: Vec::new(),
        alive: alive.clone(),
    };
    draw.refresh();
    DRAW.with(|d| {
        if let Ok(mut d) = d.try_borrow_mut() {
            *d = Some(draw);
        }
    });
    native::activate(hwnd);
    surface.focus()?;
    // Re-render fading marks while the window is up.
    let ticker = scope.clone();
    scope.spawn(async move {
        while alive.load(Ordering::Acquire) {
            match ticker.delay(FADE_TICK) {
                Ok(delay) => {
                    let _ = delay.await;
                }
                Err(_) => break,
            }
            let _ = ticker.post(move || {
                DRAW.with(|d| {
                    if let Ok(mut d) = d.try_borrow_mut() {
                        if let Some(draw) = d.as_mut().filter(|d| d.id == id) {
                            if api::draw_tick(id) {
                                draw.refresh();
                            }
                        }
                    }
                })
            });
        }
    })
}

/// Starts following the main view model's signals. Called once, on the UI
/// thread, after the main window is mounted.
pub fn install(scope: &AppScope) -> Result<()> {
    HOST.with(|h| h.borrow_mut().scope = Some(scope.clone()));
    let sink = crate::ui::main().sink;
    // Straight away (this is the UI thread): the overlay should come up as
    // fast as it can.
    Effect::new(move || {
        let json = sink.overlay.get();
        apply(&json);
        Ok(())
    })?;
    let draw_scope = scope.clone();
    Effect::new(move || {
        let json = sink.screen_draw.get();
        let scope = draw_scope.clone();
        draw_scope.post(move || apply_draw(&scope, &json))
    })?;
    // The overlay's windows, made while idle so the first capture is quick.
    if std::env::var_os("JIFFSHOT_UITEST").is_none() {
        scope.post(prepare)?;
    }
    Ok(())
}

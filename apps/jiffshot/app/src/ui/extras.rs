//! The Capture Widget (the `widget` signal) and Grab Text's result window
//! (the `grab_text` signal, "serial\x1ftext").
//!
//! Both windows are built the first time they are asked for and then shown
//! and hidden, instead of being closed and made again.

use super::capture_view::parts::{self, post, rgb, Subscriptions, GLYPHS, SIZE_TO_CONTENT};
use rustolonia::{
    AnyEl, AppScope, Border, Button, Color, Dock, DockPanel, Effect, HorizontalAlignment, NodeRef, Orientation, Result, Signal, StackPanel, TextBlock,
    TextBox, TextWrapping, Window, WindowDecorations, WindowStartupLocation,
};
use std::cell::RefCell;
use std::time::Duration;

/// How long the widget stays open once the pointer has left it.
const COLLAPSE_AFTER: Duration = Duration::from_millis(400);

/// The widget's buttons: the mode sent as "widget:MODE", glyph, tooltip.
const MODES: [(&str, &str, &str); 4] =
    [("image", "\u{E722}", "Image"), ("video", "\u{E714}", "Video"), ("scrolling", "\u{E74B}", "Scrolling"), ("text", "\u{E8D2}", "Grab text")];

const STRIP: Color = rgb(0x3D8BFF);

/// What the windows need. UI thread only.
struct Extras {
    scope: AppScope,
    widget: Option<Widget>,
    grab_text: Option<GrabText>,
}

/// The Capture Widget: a thin strip at the top of the primary screen that
/// opens into capture buttons when the pointer comes near, and closes again
/// when it leaves. It never takes the focus.
struct Widget {
    window: Window,
    expanded: Signal<bool>,
    dark: Signal<bool>,
    /// Bumped each time the pointer comes or goes, so that a collapse timer
    /// started before is ignored.
    hover: u64,
    prepared: bool,
    _subs: Subscriptions,
}

/// Grab Text's result: the recognized text, editable, with Copy all.
struct GrabText {
    window: Window,
    text: TextBox,
}

thread_local! {
    static EXTRAS: RefCell<Option<Extras>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut Extras) -> R) -> Option<R> {
    EXTRAS.with(|e| e.borrow_mut().as_mut().map(f))
}

/// Starts following the main view model's signals. Called once, on the UI
/// thread, after the main window is mounted.
pub fn install(scope: &AppScope) -> Result<()> {
    let sink = super::main().sink;
    EXTRAS.with(|e| *e.borrow_mut() = Some(Extras { scope: scope.clone(), widget: None, grab_text: None }));
    let s = scope.clone();
    Effect::new(move || {
        let on = sink.widget.get();
        post(&s, "Capture Widget", move || set_widget(on));
        Ok(())
    })?;
    let s = scope.clone();
    Effect::new(move || {
        if let Some(text) = parts::payload(&sink.grab_text.get()).map(str::to_owned) {
            post(&s, "Grab Text", move || show_grab_text(text));
        }
        Ok(())
    })?;
    Ok(())
}

// ---- the Capture Widget -----------------------------------------------------

/// Shows or hides the widget.
fn set_widget(on: bool) -> Result<()> {
    let Some(scope) = with(|e| e.scope.clone()) else { return Ok(()) };
    if !on {
        if let Some(window) = with(|e| e.widget.as_ref().map(|w| w.window.clone())).flatten() {
            collapse();
            window.hide()?;
        }
        return Ok(());
    }
    if with(|e| e.widget.is_none()) == Some(true) {
        let widget = build_widget(&scope)?;
        with(|e| e.widget = Some(widget));
    }
    let Some((window, dark, prepare)) =
        with(|e| e.widget.as_mut().map(|w| (w.window.clone(), w.dark, !std::mem::replace(&mut w.prepared, true)))).flatten()
    else {
        return Ok(());
    };
    dark.set_if_changed(parts::dark_theme(&scope));
    if window.get_is_visible()? {
        return Ok(());
    }
    // Windows keeps top-level windows at least ~38 px high, and Avalonia
    // restyles the window as it shows it: a popup before and after.
    if let Some(hwnd) = parts::hwnd(&scope, &window) {
        parts::make_popup(hwnd);
    }
    // Shown off screen first: its size is only known once it is shown.
    window.set_position((-32000, -32000))?;
    window.show()?;
    if prepare {
        parts::make_tool_window(&scope, &window);
    }
    if let Some(hwnd) = parts::hwnd(&scope, &window) {
        parts::make_popup(hwnd);
    }
    place_widget(&scope, &window);
    Ok(())
}

/// Centered at the top of the primary screen.
fn place_widget(scope: &AppScope, window: &Window) {
    let Some(hwnd) = parts::hwnd(scope, window) else { return };
    let Some((width, _)) = parts::outer_size(hwnd) else { return };
    parts::move_topmost(hwnd, (parts::primary_screen_width() - width) / 2, 0);
}

fn collapse() {
    let expanded = with(|e| {
        e.widget.as_mut().map(|widget| {
            widget.hover += 1;
            widget.expanded
        })
    });
    // Outside the borrow: the window reacts to it.
    if let Some(expanded) = expanded.flatten() {
        expanded.set_if_changed(false);
    }
}

/// The pointer came in (open now) or left (close soon).
fn hovered(inside: bool) {
    let Some((scope, hover, expanded)) = with(|e| {
        e.widget.as_mut().map(|widget| {
            widget.hover += 1;
            (e.scope.clone(), widget.hover, widget.expanded)
        })
    })
    .flatten() else {
        return;
    };
    if inside {
        expanded.set_if_changed(true);
    } else {
        parts::after(&scope, COLLAPSE_AFTER, move || {
            if with(|e| e.widget.as_ref().is_some_and(|w| w.hover == hover)) == Some(true) {
                collapse();
            }
        });
    }
}

fn build_widget(scope: &AppScope) -> Result<Widget> {
    let expanded = scope.signal(false);
    let dark = scope.signal(parts::dark_theme(scope));
    let subs = Subscriptions::default();
    let buttons = StackPanel::el()
        .orientation(Orientation::Horizontal)
        .spacing(2.0)
        .margin((6.0, 4.0))
        .visible(expanded)
        .children_iter(MODES.iter().map(|&(mode, glyph, tip)| {
            let button = Button::el()
                .background(parts::TRANSPARENT)
                .padding((10.0, 6.0))
                .tool_tip_tip(tip)
                .content(TextBlock::el().text(glyph).font_family(GLYPHS).font_size(16.0));
            parts::on_click(button, &subs, move |_| {
                collapse();
                super::main().model.run(|m| m.run(format!("widget:{mode}")));
            })
        }));
    let strip = Border::el().width(90.0).height(6.0).background(STRIP).corner_radius((0.0, 0.0, 3.0, 3.0));
    let resize_scope = scope.clone();
    let window = Window::el()
        .title("Jiffshot Capture Widget")
        .window_decorations(WindowDecorations::None)
        .show_activated(false)
        .show_in_taskbar(false)
        .can_resize(false)
        .size_to_content(SIZE_TO_CONTENT)
        .background(move || if dark.get() { rgb(0x2B2D31) } else { rgb(0xFFFFFF) })
        .on_pointer_entered(|_| hovered(true))
        .on_pointer_exited(|_| hovered(false))
        // Opening and closing changes its width: keep it centered.
        .on_size_changed(move |_| {
            if let Some(window) = with(|e| e.widget.as_ref().map(|w| w.window.clone())).flatten() {
                if window.get_is_visible().unwrap_or(false) {
                    place_widget(&resize_scope, &window);
                }
            }
        })
        // It follows the setting: only Jiffshot's Exit ends it.
        .on_closing(|e| e.cancel = !parts::closing_for_good(e.close_reason))
        .content(StackPanel::el().child(buttons).child(strip))
        .build(scope)?;
    scope.set_topmost(&window, true)?;
    Ok(Widget { window, expanded, dark, hover: 0, prepared: false, _subs: subs })
}

// ---- Grab Text ----------------------------------------------------------------

/// Shows the recognized text, in front.
fn show_grab_text(text: String) -> Result<()> {
    let Some(scope) = with(|e| e.scope.clone()) else { return Ok(()) };
    if with(|e| e.grab_text.is_none()) == Some(true) {
        let window = build_grab_text(&scope)?;
        with(|e| e.grab_text = Some(window));
    }
    let Some((window, text_box)) = with(|e| e.grab_text.as_ref().map(|g| (g.window.clone(), g.text.clone()))).flatten() else { return Ok(()) };
    text_box.set_text(text)?;
    window.show()?;
    scope.set_topmost(&window, true)?;
    if let Some(hwnd) = parts::hwnd(&scope, &window) {
        crate::winutil::bring_forward(hwnd as isize);
    }
    Ok(())
}

fn build_grab_text(scope: &AppScope) -> Result<GrabText> {
    let text = TextBox::el()
        .accepts_return(true)
        .text_wrapping(TextWrapping::Wrap)
        .font_family("Consolas, Cascadia Mono, monospace")
        .build(scope)?;
    let window_ref = NodeRef::new();
    let copy_scope = scope.clone();
    let closing_scope = scope.clone();
    let source = text.clone();
    let copy = Button::el().content("Copy all").min_width(90.0).horizontal_content_alignment(HorizontalAlignment::Center).on_click(move |_| {
        let Some(window) = window_ref.get() else { return };
        let all = source.get_text().ok().flatten().unwrap_or_default();
        let copied = copy_scope.clipboard_set_text(&window, all).and_then(|operation| {
            copy_scope.spawn(async move {
                let _ = operation.await;
            })
        });
        if let Err(error) = copied {
            eprintln!("Grab Text: {error:?}");
        }
    });
    let close = Button::el().content("Close").min_width(90.0).cancel(true).horizontal_content_alignment(HorizontalAlignment::Center).on_click(move |_| {
        if let Some(window) = window_ref.get() {
            let _ = window.hide();
        }
    });
    let window = Window::el()
        .title("Grab Text")
        .icon(super::icon_file())
        .width(560.0)
        .height(420.0)
        .window_startup_location(WindowStartupLocation::CenterScreen)
        .on_closing(move |e| {
            // Kept for the next text.
            if !parts::closing_for_good(e.close_reason) {
                e.cancel = true;
                post(&closing_scope, "Grab Text", move || match window_ref.get() {
                    Some(window) => window.hide(),
                    None => Ok(()),
                });
            }
        })
        .content(
            DockPanel::el()
                .margin(14.0)
                .child(
                    StackPanel::el()
                        .orientation(Orientation::Horizontal)
                        .spacing(8.0)
                        .horizontal_alignment(HorizontalAlignment::Right)
                        .margin((0.0, 10.0, 0.0, 0.0))
                        .dock_panel_dock(Dock::Bottom)
                        .child(copy)
                        .child(close),
                )
                .child(AnyEl::from_control(text.clone())),
        )
        .node_ref(window_ref)
        .build(scope)?;
    Ok(GrabText { window, text })
}

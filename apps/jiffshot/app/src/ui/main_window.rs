//! The editor window:
//! the menu bar and toolbar on top, the library, status, narration and video
//! bars along the bottom, the properties and Effects panels on the right and
//! the canvas in the middle. The parts are built in the `main_*` modules.
//!
//! What the main view model asks of other windows (the Capture Widget, Grab
//! Text, toasts, Preferences, the overlay, Screen Draw and the capture HUD)
//! is handled by those windows' `install` functions, not here.

mod main_bars;
mod main_common;
mod main_library;
mod main_menu;
mod main_panel;
mod main_tools;

use super::canvas_view::{CanvasView, FLAG_CHANGED};
use self::main_bars::{narration_bar, status_bar, video_bar, ZoomView};
use self::main_common::{call, rule, run, solid, Ctx, Theme, Tone, RED, TRANSPARENT, WHITE};
use self::main_library::library_panel;
use self::main_menu::{create_flyout, menu_bar, share_flyout};
use self::main_panel::side_bar;
use self::main_tools::{effects_panel, tool_bar};
use super::UiOnly;
use crate::vm::{MainBinding, MainViewModelSink};
use rustolonia::{
    AnyEl, AppScope, Border, Button, Dock, DockPanel, DragDropEffects, Effect, El, FileDropEvent, FlyoutBase, FontWeight, Key, MenuFlyout, NodeRef, Orientation, Result, StackPanel, TextBox, ThemeVariant, ToggleButton, Upcast,
    VerticalAlignment, Window,
};
use std::cell::Cell;

/// `WindowCloseReason.ApplicationShutdown` and `OSShutdown`.
const CLOSE_APPLICATION_SHUTDOWN: i32 = 3;
const CLOSE_OS_SHUTDOWN: i32 = 4;

/// Builds the editor window for `binding` (not yet shown).
pub fn build(scope: &AppScope, binding: &MainBinding) -> Result<Window> {
    let sink = binding.sink;
    let canvas = CanvasView::new(scope)?;
    canvas.follow_theme(sink.theme)?;
    super::set_canvas(canvas.clone());
    let ctx = Ctx {
        scope: scope.clone(),
        sink,
        theme: Theme::new(),
        canvas: canvas.clone(),
        window_height: rustolonia::Signal::new(960.0),
        text_editor: NodeRef::new(),
    };
    let zoom = ZoomView::new();
    let theme = ctx.theme;
    let window_height = ctx.window_height;
    let window = Window::el()
        .title(sink.title)
        .width(1500.0)
        .height(960.0)
        .min_width(900.0)
        .min_height(600.0)
        .icon(super::icon_file())
        .background(theme.brush(Tone::Window))
        .on_size_changed(move |args| window_height.set_if_changed(args.new_size.height))
        .content(
            DockPanel::el()
                .child(menu_bar(&ctx).dock_panel_dock(Dock::Top))
                .child(top_bar(&ctx))
                .child(library_panel(&ctx))
                .child(status_bar(&ctx, zoom))
                .child(narration_bar(sink))
                .child(video_bar(&ctx))
                .child(side_bar(&ctx))
                .child(effects_panel(&ctx))
                .child(AnyEl::from_control(canvas.control().clone())),
        )
        .build(scope)?;
    follow_theme(scope, &window, sink, theme)?;
    wire_canvas(&ctx, zoom)?;
    wire_window(&ctx, &window)?;
    Ok(window)
}

/// The Editor and Library tabs, Capture, Create ▾, the tool palette, the
/// Effects toggle, Copy All and Share ▾.
fn top_bar(ctx: &Ctx) -> El<Border> {
    let (s, theme) = (ctx.sink, ctx.theme);
    let expanded = s.library_expanded;
    let tab = move |label: &str, library: bool, tip: &str| {
        let on = move || expanded.get() == library;
        Button::el()
            .content(label.to_owned())
            .tool_tip_tip(tip.to_owned())
            .padding((12.0, 5.0))
            .corner_radius(6.0)
            .background(move || if on() { theme.get(Tone::Well) } else { solid(TRANSPARENT) })
            .font_weight(move || if on() { FontWeight::DemiBold } else { FontWeight::Normal })
            .command(main_common::act(move || expanded.set_if_changed(library)))
    };
    let left = StackPanel::el()
        .dock_panel_dock(Dock::Left)
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .vertical_alignment(VerticalAlignment::Center)
        .child(StackPanel::el().orientation(Orientation::Horizontal).spacing(2.0).child(tab("Editor", false, "Edit the current capture")).child(tab("Library", true, "Browse all captures (Ctrl+L)")))
        .child(rule(theme, (4.0, 6.0)))
        .child(
            Button::el()
                .content("● Capture")
                .tool_tip_tip("Open the Capture window")
                .background(solid(RED))
                .foreground(solid(WHITE))
                .command(main_common::command(s.open_capture_window_enabled, |m| m.open_capture_window())),
        )
        .child(with_flyout(Button::el().content("Create ▾").tool_tip_tip("An image from a template, combined images, or a video from images"), create_flyout(ctx)))
        .child(rule(theme, (4.0, 6.0)));
    let right = StackPanel::el()
        .dock_panel_dock(Dock::Right)
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .vertical_alignment(VerticalAlignment::Center)
        .child(rule(theme, (4.0, 6.0)))
        .child(ToggleButton::el().content("✦ Effects").checked(s.show_effects.two_way()).tool_tip_tip("Show or hide the Effects panel"))
        .child(Button::el().content("Copy All").tool_tip_tip("Copy the whole image (Ctrl+Shift+C)").command(main_common::command(s.copy_image_enabled, |m| m.copy_image())))
        .child(with_flyout(Button::el().content("Share ▾").tool_tip_tip("Send the capture to another app, a file or a printer"), share_flyout(ctx)));
    Border::el()
        .dock_panel_dock(Dock::Top)
        .background(theme.brush(Tone::Panel))
        .padding((8.0, 4.0))
        .border_brush(theme.brush(Tone::Border))
        .border_thickness((0.0, 0.0, 0.0, 1.0))
        .child(DockPanel::el().child(left).child(right).child(tool_bar(ctx)))
}

/// A button that opens `flyout` when clicked.
fn with_flyout(button: El<Button>, flyout: El<MenuFlyout>) -> El<Button> {
    button.with_op(move |scope, button| {
        let flyout: FlyoutBase = flyout.build(scope)?.upcast();
        button.set_flyout(Some(&flyout))
    })
}

/// The one color theme, for every window: `theme` ("Dark", "Light" or
/// "System") sets the application's theme variant, and the window's own
/// colors follow the variant it ends up with.
fn follow_theme(scope: &AppScope, window: &Window, sink: MainViewModelSink, theme: Theme) -> Result<()> {
    let app = scope.clone();
    Effect::new(move || {
        let variant = match sink.theme.get().as_str() {
            "Light" => ThemeVariant::Light,
            "System" => ThemeVariant::Default,
            _ => ThemeVariant::Dark,
        };
        app.set_requested_theme_variant(variant)?;
        theme.dark.set_if_changed(app.actual_theme_variant()? == ThemeVariant::Dark);
        Ok(())
    })?;
    // The system's theme can change under "System".
    let app = scope.clone();
    window.clone().on_actual_theme_variant_changed(scope, move |_| {
        if let Ok(variant) = app.actual_theme_variant() {
            theme.dark.set_if_changed(variant == ThemeVariant::Dark);
        }
    })?;
    Ok(())
}

/// The canvas follows the model (frames, zoom requests, the library view)
/// and reports edits, double-clicks, Escape, zoom changes and dropped files.
fn wire_canvas(ctx: &Ctx, zoom: ZoomView) -> Result<()> {
    let (s, canvas) = (ctx.sink, ctx.canvas.clone());
    let refresh = canvas.clone();
    Effect::new(move || {
        s.frame_id.get();
        refresh.refresh();
        Ok(())
    })?;
    let zoomer = ctx.canvas_handle();
    let scope = ctx.scope.clone();
    Effect::new(move || {
        let request = s.zoom_request.get();
        let verb = request.split(':').next().unwrap_or_default().to_owned();
        if !verb.is_empty() {
            // After layout, so the canvas has its new size.
            let canvas = zoomer.clone();
            scope.post(move || canvas.get().zoom_to(&verb))?;
        }
        Ok(())
    })?;
    let control = canvas.control().clone();
    Effect::new(move || control.set_visible(!s.library_expanded.get()))?;
    canvas.on_changed(|flags| {
        if flags & FLAG_CHANGED != 0 {
            call(|m| m.canvas_changed());
        }
    });
    let (scope, editor) = (ctx.scope.clone(), ctx.text_editor);
    canvas.on_edit_requested(move || {
        run("finish");
        focus_text(&scope, editor);
    });
    canvas.on_cancel_requested(|| run("deselect"));
    canvas.on_zoom_changed(move |z| zoom.show(z));
    // Files dropped on the canvas go where they were dropped; the drop
    // carries no position, so that is the middle of the image.
    let dropped_on = ctx.canvas_handle();
    ctx.scope.on_file_drop(canvas.control(), DragDropEffects::COPY, move |event| {
        let FileDropEvent::Drop { items, .. } = event else { return };
        let paths: Vec<String> = items.iter().filter_map(|item| item.local_path()).map(|p| p.to_string_lossy().into_owned()).collect();
        if paths.is_empty() {
            return;
        }
        let (w, h) = dropped_on.get().document_size();
        run(format!("drop:{},{}|{}", f64::from(w) / 2.0, f64::from(h) / 2.0, paths.join("|")));
    })?;
    // The properties' text box takes focus when the model asks.
    let (scope, editor, first) = (ctx.scope.clone(), ctx.text_editor, Cell::new(true));
    Effect::new(move || {
        s.focus_text.get();
        if !first.replace(false) {
            focus_text(&scope, editor);
        }
        Ok(())
    })?;
    Ok(())
}

/// Focuses the properties' text box and selects its text, once the panel
/// shows it.
fn focus_text(scope: &AppScope, editor: NodeRef<TextBox>) {
    let _ = scope.post(move || {
        let Some(editor) = editor.get() else { return };
        if editor.get_is_visible().unwrap_or(false) {
            let _ = editor.focus_with_navigation_method_and_key_modifiers(0, 0);
            let _ = editor.select_all();
        }
    });
}

/// Escape deselects; closing the window hides it (Jiffshot keeps running in
/// the tray) except when Windows or the application is shutting down.
fn wire_window(ctx: &Ctx, window: &Window) -> Result<()> {
    window.clone().on_key_down(&ctx.scope, |args| {
        if !args.handled && args.key == Key::Escape as i32 {
            run("deselect");
            args.handled = true;
        }
    })?;
    // The UI self-test's window really closes, which ends it.
    if std::env::var_os("JIFFSHOT_UITEST").is_none() {
        let (scope, hidden) = (ctx.scope.clone(), UiOnly::new(window.clone()));
        window.clone().on_closing(&ctx.scope, move |args| {
            if matches!(args.close_reason, CLOSE_APPLICATION_SHUTDOWN | CLOSE_OS_SHUTDOWN) {
                return;
            }
            args.cancel = true;
            let window = hidden.clone();
            let _ = scope.post(move || {
                let _ = window.get().hide();
                run("hidden");
            });
        })?;
    }
    Ok(())
}

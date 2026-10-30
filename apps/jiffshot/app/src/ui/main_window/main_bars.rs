//! The bars along the bottom: the status bar with undo, redo and the zoom
//! controls, the narration bar, and the video controls.

use super::main_common::{bar, choice_box, command, one_way_toggle, rule, solid, time_text, Ctx, Tone, RED, WHITE};
use crate::vm::MainViewModelSink;
use rustolonia::{Border, Button, CheckBox, Dock, DockPanel, El, FontWeight, Orientation, Signal, Slider, StackPanel, TextAlignment, TextBlock, ToggleButton, VerticalAlignment};

/// The zoom slider's range, in percent.
const ZOOM_MIN: f64 = 5.0;
const ZOOM_MAX: f64 = 800.0;

/// The zoom as the window shows it: the slider's percent and the label.
#[derive(Clone, Copy)]
pub struct ZoomView {
    pub percent: Signal<f64>,
    pub text: Signal<String>,
}

impl ZoomView {
    pub fn new() -> Self {
        Self { percent: Signal::new(100.0), text: Signal::new("100%".to_owned()) }
    }

    /// The canvas zoomed to `zoom` (1 = 100%).
    pub fn show(self, zoom: f64) {
        self.text.set_if_changed(format!("{:.0}%", zoom * 100.0));
        self.percent.set_if_changed((zoom * 100.0).clamp(ZOOM_MIN, ZOOM_MAX));
    }
}

/// The status bar.
pub fn status_bar(ctx: &Ctx, zoom: ZoomView) -> El<Border> {
    let (s, theme) = (ctx.sink, ctx.theme);
    let canvas = ctx.canvas_handle();
    let zoom_slider = Slider::el()
        .width(110.0)
        .minimum(ZOOM_MIN)
        .maximum(ZOOM_MAX)
        .vertical_alignment(VerticalAlignment::Center)
        .tool_tip_tip("Zoom")
        .value(zoom.percent.two_way())
        .on_value_changed(move |args| {
            // Only the user's moves: the canvas's own zoom comes back here too.
            let canvas = canvas.get();
            if (args.new_value - (canvas.zoom() * 100.0).clamp(ZOOM_MIN, ZOOM_MAX)).abs() > 0.01 {
                canvas.set_zoom(args.new_value / 100.0);
            }
        });
    let zoom_bar = StackPanel::el()
        .dock_panel_dock(Dock::Right)
        .orientation(Orientation::Horizontal)
        .spacing(2.0)
        .vertical_alignment(VerticalAlignment::Center)
        .visible(move || !s.library_expanded.get())
        .child(bar(s.document_info).tool_tip_tip("Image size; click to fit it in the window").command(command(s.zoom_fit_enabled, |m| m.zoom_fit())))
        .child(rule(theme, (6.0, 4.0)))
        .child(bar("−").tool_tip_tip("Zoom out (Ctrl+−)").command(command(s.zoom_out_enabled, |m| m.zoom_out())))
        .child(zoom_slider)
        .child(bar("+").tool_tip_tip("Zoom in (Ctrl++)").command(command(s.zoom_in_enabled, |m| m.zoom_in())))
        .child(TextBlock::el().text(zoom.text).vertical_alignment(VerticalAlignment::Center).width(44.0).text_alignment(TextAlignment::Center).font_size(12.0))
        .child(bar("Fit").tool_tip_tip("Fit to window (Ctrl+Alt+0)").command(command(s.zoom_fit_enabled, |m| m.zoom_fit())))
        .child(bar("Actual size").tool_tip_tip("100% (Ctrl+0)").command(command(s.zoom_actual_enabled, |m| m.zoom_actual())))
        .child(rule(theme, (6.0, 4.0)))
        .child(
            ToggleButton::el()
                .checked(s.show_tray.two_way())
                .content(TextBlock::el().text(move || if s.show_tray.get() { "Hide Recent ▾" } else { "Show Recent ▴" }))
                .padding((8.0, 2.0))
                .min_height(0.0)
                .font_size(12.0)
                .tool_tip_tip("Show or hide the Recent Captures tray"),
        );
    let history = StackPanel::el()
        .dock_panel_dock(Dock::Left)
        .orientation(Orientation::Horizontal)
        .spacing(2.0)
        .vertical_alignment(VerticalAlignment::Center)
        .child(bar("↶").font_size(14.0).tool_tip_tip("Undo (Ctrl+Z)").command(command(s.undo_enabled, |m| m.undo())))
        .child(bar("↷").font_size(14.0).tool_tip_tip("Redo (Ctrl+Y)").command(command(s.redo_enabled, |m| m.redo())));
    Border::el()
        .dock_panel_dock(Dock::Bottom)
        .background(theme.brush(Tone::Panel))
        .padding((6.0, 3.0))
        .border_brush(theme.brush(Tone::Border))
        .border_thickness((0.0, 1.0, 0.0, 0.0))
        .child(
            DockPanel::el()
                .child(zoom_bar)
                .child(history)
                .child(TextBlock::el().text(s.status).margin((10.0, 0.0)).vertical_alignment(VerticalAlignment::Center).font_size(12.0).foreground(theme.brush(Tone::Subtle)).text_trimming("CharacterEllipsis")),
        )
}

/// Shown while a narrated video is being recorded.
pub fn narration_bar(sink: MainViewModelSink) -> El<Border> {
    Border::el().dock_panel_dock(Dock::Bottom).background(solid(RED)).padding((10.0, 6.0)).visible(sink.narrating).child(
        StackPanel::el()
            .orientation(Orientation::Horizontal)
            .spacing(10.0)
            .child(TextBlock::el().text("● Recording narration").foreground(solid(WHITE)).font_weight(FontWeight::DemiBold).vertical_alignment(VerticalAlignment::Center))
            .child(Button::el().content("Next picture ▶").command(command(sink.narrate_next_enabled, |m| m.narrate_next())))
            .child(Button::el().content("Finish").command(command(sink.narrate_finish_enabled, |m| m.narrate_finish()))),
    )
}

/// Playback, trimming and GIF export, while a video is open.
pub fn video_bar(ctx: &Ctx) -> El<Border> {
    let (s, theme) = (ctx.sink, ctx.theme);
    let muted = move || s.muted.get();
    let gif = StackPanel::el()
        .dock_panel_dock(Dock::Bottom)
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .margin((0.0, 6.0, 0.0, 0.0))
        .child(TextBlock::el().text("GIF").vertical_alignment(VerticalAlignment::Center).foreground(theme.brush(Tone::Muted)))
        .child(Button::el().content("Export GIF…").command(command(s.export_gif_enabled, |m| m.export_gif())))
        .child(
            choice_box(theme, || ["Screen recording", "High motion", "Small file", "Custom"].map(String::from).to_vec(), s.gif_preset)
                .vertical_alignment(VerticalAlignment::Center)
                .min_width(150.0)
                .tool_tip_tip("GIF frame rate and size"),
        )
        .child(CheckBox::el().content("Loop").checked(s.gif_loop.two_way()).vertical_alignment(VerticalAlignment::Center))
        .child(CheckBox::el().content("Fade out").checked(s.gif_fade.two_way()).vertical_alignment(VerticalAlignment::Center))
        .child(CheckBox::el().content("Dither").checked(s.gif_dither.two_way()).vertical_alignment(VerticalAlignment::Center))
        .child(TextBlock::el().text("Playback").vertical_alignment(VerticalAlignment::Center).margin((12.0, 0.0, 0.0, 0.0)).foreground(theme.brush(Tone::Muted)))
        .child(one_way_toggle(muted, || super::main_common::call(|m| m.toggle_mute())).content("🔇").tool_tip_tip("Mute playback"))
        .child(Slider::el().width(90.0).minimum(0.0).maximum(1.0).value(s.video_volume.two_way()).vertical_alignment(VerticalAlignment::Center).tool_tip_tip("Playback volume"));
    let playback = StackPanel::el()
        .dock_panel_dock(Dock::Left)
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .child(Button::el().width(40.0).content(TextBlock::el().text(move || if s.playing.get() { "⏸" } else { "▶" })).command(command(s.play_pause_enabled, |m| m.play_pause())))
        .child(TextBlock::el().vertical_alignment(VerticalAlignment::Center).width(60.0).text(move || time_text(s.video_position.get())));
    let editing = StackPanel::el()
        .dock_panel_dock(Dock::Right)
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .child(TextBlock::el().vertical_alignment(VerticalAlignment::Center).foreground(theme.brush(Tone::Muted)).text(move || time_text(s.video_duration.get())))
        .child(Button::el().content("Set start").tool_tip_tip("Trim everything before the playhead").command(command(s.set_trim_start_enabled, |m| m.set_trim_start())))
        .child(Button::el().content("Set end").tool_tip_tip("Trim everything after the playhead").command(command(s.set_trim_end_enabled, |m| m.set_trim_end())))
        .child(Button::el().content("◀|").tool_tip_tip("Previous frame").command(command(s.prev_frame_enabled, |m| m.prev_frame())))
        .child(Button::el().content("|▶").tool_tip_tip("Next frame").command(command(s.next_frame_enabled, |m| m.next_frame())))
        .child(Button::el().content("Cut selection…").tool_tip_tip("Save the video without the part between start and end").command(command(s.cut_selection_enabled, |m| m.cut_selection())))
        .child(Button::el().content("Save selection…").tool_tip_tip("Save the part between start and end as a new MP4").command(command(s.export_video_enabled, |m| m.export_video())))
        .child(Button::el().content("Edit frame").tool_tip_tip("Open the current frame as an image").command(command(s.frame_to_image_enabled, |m| m.frame_to_image())));
    let timeline = StackPanel::el()
        .margin((10.0, 0.0))
        .vertical_alignment(VerticalAlignment::Center)
        .child(Slider::el().minimum(0.0).maximum(s.video_duration).value(s.video_position.two_way()))
        .child(TextBlock::el().text(s.video_info).font_size(11.0).foreground(theme.brush(Tone::Muted)));
    Border::el()
        .dock_panel_dock(Dock::Bottom)
        .background(theme.brush(Tone::Panel))
        .padding((10.0, 6.0))
        .border_brush(theme.brush(Tone::Border))
        .border_thickness((0.0, 1.0, 0.0, 0.0))
        .visible(move || s.mode.with(|m| m == "Video"))
        .child(DockPanel::el().child(gif).child(playback).child(editing).child(timeline))
}

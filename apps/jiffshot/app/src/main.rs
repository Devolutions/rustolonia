//! Jiffshot: one process, every window on Rustolonia's code-first API. This
//! crate owns the editor's state (`app.rs`) and builds its windows (`ui`);
//! the document model, rendering and file formats live in `jiffshot-doc`,
//! capture in `jiffshot-capture` and the capture side's flows in
//! `jiffshot-workflow`.

// A GUI program in every build, not a console one.
#![windows_subsystem = "windows"]

mod app;
mod canvas;
mod capture_window;
mod library;
mod ocr;
mod settings;
mod share;
mod trace;
mod prefs;
mod tray;
mod ui;
mod video;
mod vm;
mod vm_support;
mod winutil;

use app::act;
use rustolonia::{App, Result};
use vm::{MainViewModel, MainViewModelSink};
use jiffshot_doc::editor::Tool;
use jiffshot_doc::object::{Cap, Dash, VAlign};
use jiffshot_doc::text::Align;
use jiffshot_doc::{Document, Editor, Rgba};

/// The window's view model; every call is forwarded to [`app::App`].
struct Model;

fn color(value: &str) -> Option<Rgba> {
    Rgba::parse(value)
}

fn cap(value: &str) -> Cap {
    Cap::from_name(value)
}

macro_rules! command {
    ($name:ident, |$app:ident| $body:expr) => {
        fn $name(&mut self) -> Result<()> {
            act(|$app| {
                $body;
            })
        }
    };
}

macro_rules! tool {
    ($name:ident, $tool:expr) => {
        fn $name(&mut self) -> Result<()> {
            act(|app| app.set_tool($tool))
        }
    };
}

impl MainViewModel for Model {
    fn attach(&mut self, sink: MainViewModelSink) -> Result<()> {
        app::with(|app| app.attach(sink)).unwrap_or(Ok(()))
    }

    fn detach(&mut self) -> Result<()> {
        app::with(|app| app.detach());
        Ok(())
    }

    fn set_stroke_color(&mut self, value: String) -> Result<()> {
        let Some(c) = color(&value) else { return Ok(()) };
        act(|app| app.style_edit("stroke", move |s| s.stroke = c))
    }

    fn set_fill_color(&mut self, value: String) -> Result<()> {
        let Some(c) = color(&value) else { return Ok(()) };
        act(|app| app.style_edit("fill", move |s| s.fill = c))
    }

    fn set_text_color(&mut self, value: String) -> Result<()> {
        let Some(c) = color(&value) else { return Ok(()) };
        act(|app| app.style_edit("textcolor", move |s| s.text_color = c))
    }

    fn set_stroke_width(&mut self, value: f64) -> Result<()> {
        act(|app| app.style_edit("width", move |s| s.width = value.clamp(0.0, 200.0) as f32))
    }

    fn set_font_size(&mut self, value: f64) -> Result<()> {
        let size = value.clamp(4.0, 400.0) as f32;
        act(|app| {
            app.style_edit("size", move |s| s.font_size = size);
            canvas::with_editor(|e| e.resize_selected_step(size));
        })
    }

    fn set_font_family(&mut self, value: String) -> Result<()> {
        if value.is_empty() {
            return Ok(());
        }
        act(|app| app.style_edit("font", move |s| s.font = value.clone()))
    }

    fn set_bold(&mut self, value: bool) -> Result<()> {
        act(|app| app.style_edit("bold", move |s| s.bold = value))
    }

    fn set_italic(&mut self, value: bool) -> Result<()> {
        act(|app| app.style_edit("italic", move |s| s.italic = value))
    }

    fn set_shadow(&mut self, value: bool) -> Result<()> {
        act(|app| app.style_edit("shadow", move |s| s.shadow = value))
    }

    fn set_opacity(&mut self, value: f64) -> Result<()> {
        act(|app| app.style_edit("opacity", move |s| s.opacity = (value / 100.0).clamp(0.05, 1.0) as f32))
    }

    fn set_text(&mut self, value: String) -> Result<()> {
        act(|app| app.set_text(&value))
    }

    fn set_variant(&mut self, value: String) -> Result<()> {
        act(|app| app.set_variant(&value))
    }

    fn set_amount(&mut self, value: f64) -> Result<()> {
        act(|app| app.set_amount(value))
    }

    fn set_align(&mut self, value: String) -> Result<()> {
        let align = match value.as_str() {
            "Center" => Align::Center,
            "Right" => Align::Right,
            _ => Align::Left,
        };
        act(|app| app.style_edit("align", move |s| s.align = align))
    }

    fn set_dash(&mut self, value: String) -> Result<()> {
        let dash = match value.as_str() {
            "Dash" => Dash::Dash,
            "Dot" => Dash::Dot,
            _ => Dash::Solid,
        };
        act(|app| app.style_edit("dash", move |s| s.dash = dash))
    }

    fn set_start_cap(&mut self, value: String) -> Result<()> {
        let c = cap(&value);
        act(|app| app.style_edit("startcap", move |s| s.start_cap = c))
    }

    fn set_end_cap(&mut self, value: String) -> Result<()> {
        let c = cap(&value);
        act(|app| app.style_edit("endcap", move |s| s.end_cap = c))
    }

    fn set_action(&mut self, _value: String) -> Result<()> {
        Ok(())
    }

    fn set_video_position(&mut self, value: f64) -> Result<()> {
        act(|app| video::seek(app, value))
    }

    fn set_rotation(&mut self, value: f64) -> Result<()> {
        act(|app| app.set_rotation(value))
    }

    fn set_head_size(&mut self, value: f64) -> Result<()> {
        act(|app| app.style_edit("head", move |s| s.head_size = value.clamp(0.3, 4.0) as f32))
    }

    fn set_shadow_angle(&mut self, value: f64) -> Result<()> {
        act(|app| app.style_edit("shadowangle", move |s| s.shadow_angle = value as f32))
    }

    fn set_shadow_distance(&mut self, value: f64) -> Result<()> {
        act(|app| app.style_edit("shadowdistance", move |s| s.shadow_distance = value.clamp(0.0, 60.0) as f32))
    }

    fn set_shadow_blur(&mut self, value: f64) -> Result<()> {
        act(|app| app.style_edit("shadowblur", move |s| s.shadow_blur = value.clamp(0.0, 40.0) as f32))
    }

    fn set_shadow_color(&mut self, value: String) -> Result<()> {
        let Some(c) = color(&value) else { return Ok(()) };
        act(|app| app.style_edit("shadowcolor", move |s| s.shadow_color = c))
    }

    fn set_outline_color(&mut self, value: String) -> Result<()> {
        let Some(c) = color(&value) else { return Ok(()) };
        act(|app| app.style_edit("outline", move |s| s.outline = c))
    }

    fn set_outline_width(&mut self, value: f64) -> Result<()> {
        act(|app| app.style_edit("outlinewidth", move |s| s.outline_width = value.clamp(0.0, 20.0) as f32))
    }

    fn set_underline(&mut self, value: bool) -> Result<()> {
        act(|app| app.style_edit("underline", move |s| s.underline = value))
    }

    fn set_strike(&mut self, value: bool) -> Result<()> {
        act(|app| app.style_edit("strike", move |s| s.strike = value))
    }

    fn set_v_align(&mut self, value: String) -> Result<()> {
        let v = match value.as_str() {
            "Middle" => VAlign::Middle,
            "Bottom" => VAlign::Bottom,
            _ => VAlign::Top,
        };
        act(|app| app.style_edit("valign", move |s| s.valign = v))
    }

    fn set_padding(&mut self, value: f64) -> Result<()> {
        act(|app| app.style_edit("padding", move |s| s.padding = value.clamp(-1.0, 100.0) as f32))
    }

    fn set_fill2(&mut self, value: String) -> Result<()> {
        let Some(c) = color(&value) else { return Ok(()) };
        act(|app| app.style_edit("fill2", move |s| s.fill2 = c))
    }

    fn set_gradient_radial(&mut self, value: bool) -> Result<()> {
        act(|app| app.style_edit("radial", move |s| s.gradient_radial = value))
    }

    fn set_gradient_angle(&mut self, value: f64) -> Result<()> {
        act(|app| app.style_edit("gradangle", move |s| s.gradient_angle = value as f32))
    }

    fn set_canvas_color(&mut self, value: String) -> Result<()> {
        let Some(c) = color(&value) else { return Ok(()) };
        act(|_app| {
            canvas::with_editor(|e| e.canvas_color = c);
        })
    }

    command!(expand_canvas, |app| app.document_expand());
    command!(cycle_theme, |app| app.cycle_theme());
    command!(prev_frame, |app| app.video_action("prev"));
    command!(next_frame, |app| app.video_action("next"));
    command!(cut_selection, |app| app.video_action("cut"));
    command!(silent_copy, |app| app.video_action("silent"));
    command!(combine_videos, |app| app.video_action("combine"));
    command!(video_from_images, |app| app.video_action("slideshow"));
    command!(toggle_mute, |app| app.video_action("mute"));
    command!(toggle_library_view, |app| app.library_view(|v| v.expanded = !v.expanded));
    command!(combine_selected, |app| app.combine_selected());
    command!(clear_library_selection, |app| app.clear_library_selection());
    command!(show_closed, |app| app.show_closed());
    command!(reset_toolbar, |app| app.run("toolbar:reset"));
    command!(open_capture_window, |app| app.open_capture_window());
    command!(auto_trim, |app| app.auto_trim());
    command!(tool_crop_menu, |app| app.set_tool(jiffshot_doc::Tool::Crop));

    fn open_recent_file(&mut self, value: String) -> Result<()> {
        act(|app| app.open_recent(&value))
    }
    command!(narrate_images, |app| app.video_action("narrate"));
    command!(narrate_next, |app| app.video_action("narrate-next"));
    command!(narrate_finish, |app| app.video_action("narrate-finish"));

    fn set_template_theme(&mut self, value: String) -> Result<()> {
        act(|app| app.template_prefs(|t| t.theme = value))
    }

    fn set_template_captions(&mut self, value: bool) -> Result<()> {
        act(|app| app.template_prefs(|t| t.captions = value))
    }

    fn set_template_numbers(&mut self, value: bool) -> Result<()> {
        act(|app| app.template_prefs(|t| t.numbers = value))
    }

    fn set_template_footer(&mut self, value: String) -> Result<()> {
        act(|app| app.template_prefs(|t| t.footer = value))
    }

    fn set_library_search(&mut self, value: String) -> Result<()> {
        act(|app| app.library_view(|v| v.search = value))
    }

    fn set_library_filter(&mut self, value: String) -> Result<()> {
        if value.is_empty() {
            return Ok(());
        }
        act(|app| app.library_view(|v| v.filter = value))
    }

    fn set_library_sort(&mut self, value: String) -> Result<()> {
        if value.is_empty() {
            return Ok(());
        }
        act(|app| app.library_view(|v| v.sort = value))
    }

    fn set_thumb_size(&mut self, value: f64) -> Result<()> {
        act(|app| app.library_view(|v| v.thumb = value.clamp(60.0, 240.0)))
    }

    fn set_show_side_bar(&mut self, value: bool) -> Result<()> {
        act(|app| app.layout(|l| l.side_bar = value))
    }

    fn set_show_effects(&mut self, value: bool) -> Result<()> {
        act(|app| app.layout(|l| l.effects = value))
    }

    fn set_show_tray(&mut self, value: bool) -> Result<()> {
        act(|app| app.layout(|l| l.tray = value))
    }

    fn set_tool_labels(&mut self, value: bool) -> Result<()> {
        act(|app| app.layout(|l| l.labels = value))
    }

    fn set_library_expanded(&mut self, value: bool) -> Result<()> {
        act(|app| app.library_view(|v| v.expanded = value))
    }

    fn set_gif_preset(&mut self, value: String) -> Result<()> {
        if !settings::GIF_PRESETS.contains(&value.as_str()) {
            return Ok(());
        }
        act(|app| app.video_prefs(|p| p.gif_preset = value))
    }

    fn set_gif_loop(&mut self, value: bool) -> Result<()> {
        act(|app| app.video_prefs(|p| p.gif_loop = value))
    }

    fn set_gif_fade(&mut self, value: bool) -> Result<()> {
        act(|app| app.video_prefs(|p| p.gif_fade = value))
    }

    fn set_gif_dither(&mut self, value: bool) -> Result<()> {
        act(|app| app.video_prefs(|p| p.gif_dither = value))
    }

    fn set_video_volume(&mut self, value: f64) -> Result<()> {
        act(|app| {
            app.video_prefs(|p| {
                p.volume = value.clamp(0.0, 1.0);
                p.muted = false;
            })
        })
    }

    fn set_background_fill(&mut self, value: String) -> Result<()> {
        act(|app| app.set_background_fill(&value))
    }

    command!(select_all, |_app| canvas::with_editor(|e| e.select_all()));
    command!(deselect_all, |_app| canvas::with_editor(|e| e.set_selection(Vec::new())));
    command!(invert_selection, |_app| canvas::with_editor(|e| e.invert_selection()));
    command!(group, |_app| canvas::with_editor(|e| e.group_selected()));
    command!(ungroup, |_app| canvas::with_editor(|e| e.ungroup_selected()));
    command!(flatten_selected, |_app| canvas::with_editor(|e| e.flatten_selected()));
    command!(smart_redact, |app| app.smart_redact());
    command!(new_from_clipboard, |app| app.new_from_clipboard());
    command!(print, |app| app.print());
    command!(close_file, |app| app.close_file());
    command!(duplicate_file, |app| app.duplicate_file());
    command!(delete_file, |app| app.delete_file());
    tool!(tool_magic_wand, Tool::MagicWand);
    tool!(tool_eraser, Tool::Eraser);

    command!(new_image, |app| app.new_image());
    command!(open, |app| app.open());
    command!(save, |app| app.save());
    command!(save_as, |app| app.save_as());
    command!(export, |app| app.export());
    command!(copy_image, |app| app.copy_image());
    command!(paste, |app| app.paste());
    command!(new_capture, |app| app.new_capture());
    command!(combine_images, |app| app.combine_images());
    command!(open_library_folder, |app| app.open_library_folder());
    command!(exit, |app| app.exit());
    command!(undo, |_app| canvas::with_editor(|e| e.undo()));
    command!(redo, |_app| canvas::with_editor(|e| e.redo()));
    command!(copy, |app| app.copy());
    command!(cut, |app| app.cut());
    command!(delete, |_app| canvas::with_editor(|e| e.delete_selected()));
    command!(duplicate, |_app| canvas::with_editor(|e| e.duplicate_selected()));
    command!(bring_to_front, |_app| canvas::with_editor(|e| e.arrange(2)));
    command!(bring_forward, |_app| canvas::with_editor(|e| e.arrange(1)));
    command!(send_backward, |_app| canvas::with_editor(|e| e.arrange(-1)));
    command!(send_to_back, |_app| canvas::with_editor(|e| e.arrange(-2)));
    command!(save_quick_style, |app| app.save_quick_style());
    command!(rotate_left, |app| app.document_op("Rotate left", |d| {
        d.rotate(false);
        true
    }));
    command!(rotate_right, |app| app.document_op("Rotate right", |d| {
        d.rotate(true);
        true
    }));
    command!(flip_horizontal, |app| app.document_op("Flip horizontal", |d| {
        d.flip(true);
        true
    }));
    command!(flip_vertical, |app| app.document_op("Flip vertical", |d| {
        d.flip(false);
        true
    }));
    command!(resize50, |app| app.resize(0.5));
    command!(resize200, |app| app.resize(2.0));
    command!(flatten, |app| app.document_op("Flatten", |d| {
        let any = !d.objects.is_empty();
        d.flatten_objects();
        any
    }));
    command!(zoom_in, |app| app.run("zoom:in"));
    command!(zoom_out, |app| app.run("zoom:out"));
    command!(zoom_fit, |app| app.run("zoom:fit"));
    command!(zoom_actual, |app| app.run("zoom:actual"));
    command!(grab_text, |app| app.grab_text());
    command!(simplify, |app| app.simplify());
    command!(play_pause, |app| app.video_action("play"));
    command!(set_trim_start, |app| app.video_action("start"));
    command!(set_trim_end, |app| app.video_action("end"));
    command!(export_video, |app| app.video_action("export-mp4"));
    command!(export_gif, |app| app.video_action("export-gif"));
    command!(frame_to_image, |app| app.video_action("frame"));
    command!(record_video, |app| app.video_action("record"));
    command!(canvas_changed, |_app| ());
    tool!(tool_select, Tool::Select);
    tool!(tool_arrow, Tool::Arrow);
    tool!(tool_line, Tool::Line);
    tool!(tool_shape, Tool::Shape);
    tool!(tool_text, Tool::Text);
    tool!(tool_callout, Tool::Callout);
    tool!(tool_step, Tool::Step);
    tool!(tool_pen, Tool::Pen);
    tool!(tool_highlight, Tool::Highlight);
    tool!(tool_blur, Tool::Blur);
    tool!(tool_magnify, Tool::Magnify);
    tool!(tool_spotlight, Tool::Spotlight);
    tool!(tool_stamp, Tool::Stamp);
    tool!(tool_fill, Tool::Fill);
    tool!(tool_crop, Tool::Crop);
    tool!(tool_cut_out, Tool::CutOut);
    tool!(tool_selection, Tool::Selection);
    tool!(tool_eyedropper, Tool::Eyedropper);
    tool!(tool_smart_move, Tool::SmartMove);
    tool!(tool_text_replace, Tool::TextReplace);

    fn run(&mut self, value: String) -> Result<()> {
        act(|app| app.run(&value))
    }
}

fn main() -> rustolonia::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Command-line tools and self-tests run and exit without the app.
    if let Some(code) = jiffshot_workflow::run_command(&args) {
        std::process::exit(code);
    }
    let files: Vec<std::path::PathBuf> = args.iter().filter(|a| !a.starts_with('-')).map(std::path::PathBuf::from).collect();
    let capture = jiffshot_workflow::capture_flag(&args);
    // The UI self-test drives the editor alone: no tray, hotkeys or
    // single-instance claim, and closing its window ends it.
    let tray = std::env::var_os("JIFFSHOT_UITEST").is_none();
    // One Jiffshot per session: a second launch hands its file or capture
    // to the running one and exits.
    if tray && !jiffshot_workflow::claim_instance() {
        match files.first().and_then(|f| std::path::absolute(f).ok()) {
            Some(file) => {
                jiffshot_capture::ipc::send_to_editor(&file);
            }
            None => jiffshot_workflow::forward_to_running(capture),
        }
        return Ok(());
    }
    let show_editor = !tray || !files.is_empty();
    jiffshot_capture::screen::enable_per_monitor_dpi();
    let mut editor = Editor::new(Document::blank(1280, 720, Rgba::WHITE));
    editor.analyzer = Some(Box::new(ocr::Recognizer));
    *canvas::EDITOR.lock() = Some(editor);
    let result = App::load_from_env()?.run(move |scope| {
        let opening = scope.activation_items()?.iter().find_map(|item| item.local_path().map(|p| p.to_path_buf()));
        vm_support::install(scope);
        app::install(scope.clone(), opening);
        // A second editor hands its file to this one (see the capture
        // tool's hand-off); the pipe belongs to the first editor.
        let _ = jiffshot_capture::ipc::serve(|path| app::post(move |app| app.incoming(path)));
        if tray {
            // The tray keeps Jiffshot running with the editor closed.
            scope.set_shutdown_mode(rustolonia::ShutdownMode::OnExplicitShutdown)?;
        }
        ui::mount_main_window(scope, Model, show_editor)?;
        if let Some(window) = scope.main_window() {
            winutil::set_main_hwnd(scope.platform_handle(&window).unwrap_or(0));
        }
        // The Capture window, hidden until asked for (the UI self-test
        // drives it too).
        let window = ui::mount_capture_window(scope, capture_window::CaptureModel)?;
        capture_window::set_window(window);
        if tray {
            let hooks = jiffshot_workflow::Hooks {
                open_in_editor: Box::new(|path| {
                    let path = path.to_path_buf();
                    app::post(move |app| app.incoming(path));
                    true
                }),
                show_editor: Box::new(|| app::post(|app| app.show_editor())),
                quit: Box::new(|| app::post(|app| app.exit())),
                notify: Box::new(|title, text, problem| {
                    let (title, text) = (title.to_owned(), text.to_owned());
                    app::post(move |app| app.toast(&title, &text, problem));
                }),
                state: Box::new(|state| {
                    app::post(move |app| {
                        app.set_widget(state.show_widget);
                        capture_window::refresh(&state);
                        tray::publish(state);
                    })
                }),
                show_capture_window: Box::new(|| app::post(|app| app.show_capture_window())),
                show_text: Box::new(|text| app::post(move |app| app.show_grab_text(&text))),
                hud: Box::new(app::post_hud),
                overlay: Box::new(|json| app::post_quiet(move |app| app.set_overlay(&json))),
                screen_draw: Box::new(|json| app::post_quiet(move |app| app.set_screen_draw(&json))),
            };
            if let Err(error) = jiffshot_workflow::start_engine(capture, hooks) {
                eprintln!("the capture side did not start: {error}");
            }
            ui::mount_tray(scope, tray::TrayModel)?;
        }
        Ok(())
    });
    ui::detach_all();
    result
}

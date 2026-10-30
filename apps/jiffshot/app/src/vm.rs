//! The windows' view models: what the Rust side publishes (the `*Sink`
//! types) and what the windows send back (the `*ViewModel` traits).
//!
//! Each published value is a [`Signal`] the views in [`crate::ui`] bind to.
//! A sink can be used from any thread; off the UI thread its change is
//! posted there. A writable property's signal is watched: when a control
//! changes it (a two-way binding), the model's setter runs. Values the model
//! publishes itself are not sent back (see [`crate::vm_support`]).
//!
//! The model code (`app.rs`, `library.rs`, `tray.rs`, `capture_window.rs`)
//! uses these names; keep them when adding members.

use crate::vm_support::{publish, publish_update, watch, Element, ElementSink, ModelCell};
use rustolonia::{Result, Signal};


// ---- MainViewModel -----------------------------------------------------

pub trait MainViewModel: Send + 'static {
    fn attach(&mut self, sink: MainViewModelSink) -> Result<()>;
    fn detach(&mut self) -> Result<()>;
    fn set_stroke_color(&mut self, value: String) -> Result<()>;
    fn set_fill_color(&mut self, value: String) -> Result<()>;
    fn set_text_color(&mut self, value: String) -> Result<()>;
    fn set_stroke_width(&mut self, value: f64) -> Result<()>;
    fn set_font_size(&mut self, value: f64) -> Result<()>;
    fn set_font_family(&mut self, value: String) -> Result<()>;
    fn set_bold(&mut self, value: bool) -> Result<()>;
    fn set_italic(&mut self, value: bool) -> Result<()>;
    fn set_shadow(&mut self, value: bool) -> Result<()>;
    fn set_opacity(&mut self, value: f64) -> Result<()>;
    fn set_text(&mut self, value: String) -> Result<()>;
    fn set_variant(&mut self, value: String) -> Result<()>;
    fn set_amount(&mut self, value: f64) -> Result<()>;
    fn set_align(&mut self, value: String) -> Result<()>;
    fn set_dash(&mut self, value: String) -> Result<()>;
    fn set_start_cap(&mut self, value: String) -> Result<()>;
    fn set_end_cap(&mut self, value: String) -> Result<()>;
    fn set_action(&mut self, value: String) -> Result<()>;
    fn set_video_position(&mut self, value: f64) -> Result<()>;
    fn set_rotation(&mut self, value: f64) -> Result<()>;
    fn set_background_fill(&mut self, value: String) -> Result<()>;
    fn set_head_size(&mut self, value: f64) -> Result<()>;
    fn set_shadow_angle(&mut self, value: f64) -> Result<()>;
    fn set_shadow_distance(&mut self, value: f64) -> Result<()>;
    fn set_shadow_blur(&mut self, value: f64) -> Result<()>;
    fn set_shadow_color(&mut self, value: String) -> Result<()>;
    fn set_outline_color(&mut self, value: String) -> Result<()>;
    fn set_outline_width(&mut self, value: f64) -> Result<()>;
    fn set_underline(&mut self, value: bool) -> Result<()>;
    fn set_strike(&mut self, value: bool) -> Result<()>;
    fn set_v_align(&mut self, value: String) -> Result<()>;
    fn set_padding(&mut self, value: f64) -> Result<()>;
    fn set_fill2(&mut self, value: String) -> Result<()>;
    fn set_gradient_radial(&mut self, value: bool) -> Result<()>;
    fn set_gradient_angle(&mut self, value: f64) -> Result<()>;
    fn set_canvas_color(&mut self, value: String) -> Result<()>;
    fn set_gif_preset(&mut self, value: String) -> Result<()>;
    fn set_gif_loop(&mut self, value: bool) -> Result<()>;
    fn set_gif_fade(&mut self, value: bool) -> Result<()>;
    fn set_gif_dither(&mut self, value: bool) -> Result<()>;
    fn set_video_volume(&mut self, value: f64) -> Result<()>;
    fn set_library_search(&mut self, value: String) -> Result<()>;
    fn set_library_filter(&mut self, value: String) -> Result<()>;
    fn set_library_sort(&mut self, value: String) -> Result<()>;
    fn set_thumb_size(&mut self, value: f64) -> Result<()>;
    fn set_library_expanded(&mut self, value: bool) -> Result<()>;
    fn set_template_theme(&mut self, value: String) -> Result<()>;
    fn set_template_captions(&mut self, value: bool) -> Result<()>;
    fn set_template_numbers(&mut self, value: bool) -> Result<()>;
    fn set_template_footer(&mut self, value: String) -> Result<()>;
    fn set_show_side_bar(&mut self, value: bool) -> Result<()>;
    fn set_show_effects(&mut self, value: bool) -> Result<()>;
    fn set_show_tray(&mut self, value: bool) -> Result<()>;
    fn set_tool_labels(&mut self, value: bool) -> Result<()>;
    fn new_image(&mut self) -> Result<()>;
    fn open(&mut self) -> Result<()>;
    fn save(&mut self) -> Result<()>;
    fn save_as(&mut self) -> Result<()>;
    fn export(&mut self) -> Result<()>;
    fn copy_image(&mut self) -> Result<()>;
    fn paste(&mut self) -> Result<()>;
    fn new_capture(&mut self) -> Result<()>;
    fn combine_images(&mut self) -> Result<()>;
    fn open_library_folder(&mut self) -> Result<()>;
    fn exit(&mut self) -> Result<()>;
    fn undo(&mut self) -> Result<()>;
    fn redo(&mut self) -> Result<()>;
    fn copy(&mut self) -> Result<()>;
    fn cut(&mut self) -> Result<()>;
    fn delete(&mut self) -> Result<()>;
    fn duplicate(&mut self) -> Result<()>;
    fn bring_to_front(&mut self) -> Result<()>;
    fn bring_forward(&mut self) -> Result<()>;
    fn send_backward(&mut self) -> Result<()>;
    fn send_to_back(&mut self) -> Result<()>;
    fn save_quick_style(&mut self) -> Result<()>;
    fn rotate_left(&mut self) -> Result<()>;
    fn rotate_right(&mut self) -> Result<()>;
    fn flip_horizontal(&mut self) -> Result<()>;
    fn flip_vertical(&mut self) -> Result<()>;
    fn resize50(&mut self) -> Result<()>;
    fn resize200(&mut self) -> Result<()>;
    fn flatten(&mut self) -> Result<()>;
    fn zoom_in(&mut self) -> Result<()>;
    fn zoom_out(&mut self) -> Result<()>;
    fn zoom_fit(&mut self) -> Result<()>;
    fn zoom_actual(&mut self) -> Result<()>;
    fn grab_text(&mut self) -> Result<()>;
    fn simplify(&mut self) -> Result<()>;
    fn play_pause(&mut self) -> Result<()>;
    fn set_trim_start(&mut self) -> Result<()>;
    fn set_trim_end(&mut self) -> Result<()>;
    fn export_video(&mut self) -> Result<()>;
    fn export_gif(&mut self) -> Result<()>;
    fn frame_to_image(&mut self) -> Result<()>;
    fn record_video(&mut self) -> Result<()>;
    fn canvas_changed(&mut self) -> Result<()>;
    fn select_all(&mut self) -> Result<()>;
    fn deselect_all(&mut self) -> Result<()>;
    fn invert_selection(&mut self) -> Result<()>;
    fn group(&mut self) -> Result<()>;
    fn ungroup(&mut self) -> Result<()>;
    fn flatten_selected(&mut self) -> Result<()>;
    fn smart_redact(&mut self) -> Result<()>;
    fn new_from_clipboard(&mut self) -> Result<()>;
    fn print(&mut self) -> Result<()>;
    fn close_file(&mut self) -> Result<()>;
    fn duplicate_file(&mut self) -> Result<()>;
    fn delete_file(&mut self) -> Result<()>;
    fn tool_magic_wand(&mut self) -> Result<()>;
    fn tool_eraser(&mut self) -> Result<()>;
    fn expand_canvas(&mut self) -> Result<()>;
    fn cycle_theme(&mut self) -> Result<()>;
    fn prev_frame(&mut self) -> Result<()>;
    fn next_frame(&mut self) -> Result<()>;
    fn cut_selection(&mut self) -> Result<()>;
    fn silent_copy(&mut self) -> Result<()>;
    fn combine_videos(&mut self) -> Result<()>;
    fn video_from_images(&mut self) -> Result<()>;
    fn toggle_mute(&mut self) -> Result<()>;
    fn toggle_library_view(&mut self) -> Result<()>;
    fn combine_selected(&mut self) -> Result<()>;
    fn clear_library_selection(&mut self) -> Result<()>;
    fn show_closed(&mut self) -> Result<()>;
    fn reset_toolbar(&mut self) -> Result<()>;
    fn narrate_images(&mut self) -> Result<()>;
    fn narrate_next(&mut self) -> Result<()>;
    fn narrate_finish(&mut self) -> Result<()>;
    fn open_capture_window(&mut self) -> Result<()>;
    fn open_recent_file(&mut self, value: String) -> Result<()>;
    fn auto_trim(&mut self) -> Result<()>;
    fn tool_crop_menu(&mut self) -> Result<()>;
    fn tool_select(&mut self) -> Result<()>;
    fn tool_arrow(&mut self) -> Result<()>;
    fn tool_line(&mut self) -> Result<()>;
    fn tool_shape(&mut self) -> Result<()>;
    fn tool_text(&mut self) -> Result<()>;
    fn tool_callout(&mut self) -> Result<()>;
    fn tool_step(&mut self) -> Result<()>;
    fn tool_pen(&mut self) -> Result<()>;
    fn tool_highlight(&mut self) -> Result<()>;
    fn tool_blur(&mut self) -> Result<()>;
    fn tool_magnify(&mut self) -> Result<()>;
    fn tool_spotlight(&mut self) -> Result<()>;
    fn tool_stamp(&mut self) -> Result<()>;
    fn tool_fill(&mut self) -> Result<()>;
    fn tool_crop(&mut self) -> Result<()>;
    fn tool_cut_out(&mut self) -> Result<()>;
    fn tool_selection(&mut self) -> Result<()>;
    fn tool_eyedropper(&mut self) -> Result<()>;
    fn tool_smart_move(&mut self) -> Result<()>;
    fn tool_text_replace(&mut self) -> Result<()>;
    fn run(&mut self, value: String) -> Result<()>;
}

#[derive(Clone, Copy, Debug)]
pub struct MainViewModelSink {
    pub title: Signal<String>,
    pub status: Signal<String>,
    pub tool: Signal<String>,
    pub frame_id: Signal<i64>,
    pub stroke_color: Signal<String>,
    pub fill_color: Signal<String>,
    pub text_color: Signal<String>,
    pub stroke_width: Signal<f64>,
    pub font_size: Signal<f64>,
    pub font_family: Signal<String>,
    pub bold: Signal<bool>,
    pub italic: Signal<bool>,
    pub shadow: Signal<bool>,
    pub opacity: Signal<f64>,
    pub text: Signal<String>,
    pub variant: Signal<String>,
    pub amount: Signal<f64>,
    pub align: Signal<String>,
    pub dash: Signal<String>,
    pub start_cap: Signal<String>,
    pub end_cap: Signal<String>,
    pub selection: Signal<String>,
    pub panel: Signal<String>,
    pub amount_label: Signal<String>,
    pub focus_text: Signal<i64>,
    pub zoom_request: Signal<String>,
    pub action: Signal<String>,
    pub mode: Signal<String>,
    pub video_position: Signal<f64>,
    pub video_duration: Signal<f64>,
    pub video_info: Signal<String>,
    pub trim_start: Signal<f64>,
    pub trim_end: Signal<f64>,
    pub playing: Signal<bool>,
    pub document_info: Signal<String>,
    pub rotation: Signal<f64>,
    pub library_info: Signal<String>,
    pub background_fill: Signal<String>,
    pub selection_count: Signal<i64>,
    pub head_size: Signal<f64>,
    pub shadow_angle: Signal<f64>,
    pub shadow_distance: Signal<f64>,
    pub shadow_blur: Signal<f64>,
    pub shadow_color: Signal<String>,
    pub outline_color: Signal<String>,
    pub outline_width: Signal<f64>,
    pub underline: Signal<bool>,
    pub strike: Signal<bool>,
    pub v_align: Signal<String>,
    pub padding: Signal<f64>,
    pub fill2: Signal<String>,
    pub gradient_radial: Signal<bool>,
    pub gradient_angle: Signal<f64>,
    pub canvas_color: Signal<String>,
    pub theme: Signal<String>,
    pub gif_preset: Signal<String>,
    pub gif_loop: Signal<bool>,
    pub gif_fade: Signal<bool>,
    pub gif_dither: Signal<bool>,
    pub video_volume: Signal<f64>,
    pub muted: Signal<bool>,
    pub library_search: Signal<String>,
    pub library_filter: Signal<String>,
    pub library_sort: Signal<String>,
    pub thumb_size: Signal<f64>,
    pub library_expanded: Signal<bool>,
    pub library_selected: Signal<i64>,
    pub template_theme: Signal<String>,
    pub template_captions: Signal<bool>,
    pub template_numbers: Signal<bool>,
    pub template_footer: Signal<String>,
    pub zone_fit: Signal<bool>,
    pub tool_order: Signal<String>,
    pub narrating: Signal<bool>,
    pub show_side_bar: Signal<bool>,
    pub show_effects: Signal<bool>,
    pub show_tray: Signal<bool>,
    pub tool_labels: Signal<bool>,
    pub toast: Signal<String>,
    pub grab_text: Signal<String>,
    pub widget: Signal<bool>,
    pub hud: Signal<String>,
    pub overlay: Signal<String>,
    pub screen_draw: Signal<String>,
    pub prefs: Signal<String>,
    pub variants: Signal<Vec<String>>,
    pub fonts: Signal<Vec<String>>,
    pub library: Signal<Vec<std::rc::Rc<LibraryItemItem>>>,
    pub quick_styles: Signal<Vec<std::rc::Rc<QuickStyleItem>>>,
    pub effects: Signal<Vec<String>>,
    pub templates: Signal<Vec<String>>,
    pub shares: Signal<Vec<String>>,
    pub library_filters: Signal<Vec<String>>,
    pub library_sorts: Signal<Vec<String>>,
    pub template_themes: Signal<Vec<String>>,
    pub recent_files: Signal<Vec<String>>,
    /// Whether `new_image` can run now.
    pub new_image_enabled: Signal<bool>,
    /// Whether `open` can run now.
    pub open_enabled: Signal<bool>,
    /// Whether `save` can run now.
    pub save_enabled: Signal<bool>,
    /// Whether `save_as` can run now.
    pub save_as_enabled: Signal<bool>,
    /// Whether `export` can run now.
    pub export_enabled: Signal<bool>,
    /// Whether `copy_image` can run now.
    pub copy_image_enabled: Signal<bool>,
    /// Whether `paste` can run now.
    pub paste_enabled: Signal<bool>,
    /// Whether `new_capture` can run now.
    pub new_capture_enabled: Signal<bool>,
    /// Whether `combine_images` can run now.
    pub combine_images_enabled: Signal<bool>,
    /// Whether `open_library_folder` can run now.
    pub open_library_folder_enabled: Signal<bool>,
    /// Whether `exit` can run now.
    pub exit_enabled: Signal<bool>,
    /// Whether `undo` can run now.
    pub undo_enabled: Signal<bool>,
    /// Whether `redo` can run now.
    pub redo_enabled: Signal<bool>,
    /// Whether `copy` can run now.
    pub copy_enabled: Signal<bool>,
    /// Whether `cut` can run now.
    pub cut_enabled: Signal<bool>,
    /// Whether `delete` can run now.
    pub delete_enabled: Signal<bool>,
    /// Whether `duplicate` can run now.
    pub duplicate_enabled: Signal<bool>,
    /// Whether `bring_to_front` can run now.
    pub bring_to_front_enabled: Signal<bool>,
    /// Whether `bring_forward` can run now.
    pub bring_forward_enabled: Signal<bool>,
    /// Whether `send_backward` can run now.
    pub send_backward_enabled: Signal<bool>,
    /// Whether `send_to_back` can run now.
    pub send_to_back_enabled: Signal<bool>,
    /// Whether `save_quick_style` can run now.
    pub save_quick_style_enabled: Signal<bool>,
    /// Whether `rotate_left` can run now.
    pub rotate_left_enabled: Signal<bool>,
    /// Whether `rotate_right` can run now.
    pub rotate_right_enabled: Signal<bool>,
    /// Whether `flip_horizontal` can run now.
    pub flip_horizontal_enabled: Signal<bool>,
    /// Whether `flip_vertical` can run now.
    pub flip_vertical_enabled: Signal<bool>,
    /// Whether `resize50` can run now.
    pub resize50_enabled: Signal<bool>,
    /// Whether `resize200` can run now.
    pub resize200_enabled: Signal<bool>,
    /// Whether `flatten` can run now.
    pub flatten_enabled: Signal<bool>,
    /// Whether `zoom_in` can run now.
    pub zoom_in_enabled: Signal<bool>,
    /// Whether `zoom_out` can run now.
    pub zoom_out_enabled: Signal<bool>,
    /// Whether `zoom_fit` can run now.
    pub zoom_fit_enabled: Signal<bool>,
    /// Whether `zoom_actual` can run now.
    pub zoom_actual_enabled: Signal<bool>,
    /// Whether `grab_text` can run now.
    pub grab_text_enabled: Signal<bool>,
    /// Whether `simplify` can run now.
    pub simplify_enabled: Signal<bool>,
    /// Whether `play_pause` can run now.
    pub play_pause_enabled: Signal<bool>,
    /// Whether `set_trim_start` can run now.
    pub set_trim_start_enabled: Signal<bool>,
    /// Whether `set_trim_end` can run now.
    pub set_trim_end_enabled: Signal<bool>,
    /// Whether `export_video` can run now.
    pub export_video_enabled: Signal<bool>,
    /// Whether `export_gif` can run now.
    pub export_gif_enabled: Signal<bool>,
    /// Whether `frame_to_image` can run now.
    pub frame_to_image_enabled: Signal<bool>,
    /// Whether `record_video` can run now.
    pub record_video_enabled: Signal<bool>,
    /// Whether `select_all` can run now.
    pub select_all_enabled: Signal<bool>,
    /// Whether `deselect_all` can run now.
    pub deselect_all_enabled: Signal<bool>,
    /// Whether `invert_selection` can run now.
    pub invert_selection_enabled: Signal<bool>,
    /// Whether `group` can run now.
    pub group_enabled: Signal<bool>,
    /// Whether `ungroup` can run now.
    pub ungroup_enabled: Signal<bool>,
    /// Whether `flatten_selected` can run now.
    pub flatten_selected_enabled: Signal<bool>,
    /// Whether `smart_redact` can run now.
    pub smart_redact_enabled: Signal<bool>,
    /// Whether `new_from_clipboard` can run now.
    pub new_from_clipboard_enabled: Signal<bool>,
    /// Whether `print` can run now.
    pub print_enabled: Signal<bool>,
    /// Whether `close_file` can run now.
    pub close_file_enabled: Signal<bool>,
    /// Whether `duplicate_file` can run now.
    pub duplicate_file_enabled: Signal<bool>,
    /// Whether `delete_file` can run now.
    pub delete_file_enabled: Signal<bool>,
    /// Whether `tool_magic_wand` can run now.
    pub tool_magic_wand_enabled: Signal<bool>,
    /// Whether `tool_eraser` can run now.
    pub tool_eraser_enabled: Signal<bool>,
    /// Whether `expand_canvas` can run now.
    pub expand_canvas_enabled: Signal<bool>,
    /// Whether `cycle_theme` can run now.
    pub cycle_theme_enabled: Signal<bool>,
    /// Whether `prev_frame` can run now.
    pub prev_frame_enabled: Signal<bool>,
    /// Whether `next_frame` can run now.
    pub next_frame_enabled: Signal<bool>,
    /// Whether `cut_selection` can run now.
    pub cut_selection_enabled: Signal<bool>,
    /// Whether `silent_copy` can run now.
    pub silent_copy_enabled: Signal<bool>,
    /// Whether `combine_videos` can run now.
    pub combine_videos_enabled: Signal<bool>,
    /// Whether `video_from_images` can run now.
    pub video_from_images_enabled: Signal<bool>,
    /// Whether `toggle_mute` can run now.
    pub toggle_mute_enabled: Signal<bool>,
    /// Whether `toggle_library_view` can run now.
    pub toggle_library_view_enabled: Signal<bool>,
    /// Whether `combine_selected` can run now.
    pub combine_selected_enabled: Signal<bool>,
    /// Whether `show_closed` can run now.
    pub show_closed_enabled: Signal<bool>,
    /// Whether `reset_toolbar` can run now.
    pub reset_toolbar_enabled: Signal<bool>,
    /// Whether `narrate_images` can run now.
    pub narrate_images_enabled: Signal<bool>,
    /// Whether `narrate_next` can run now.
    pub narrate_next_enabled: Signal<bool>,
    /// Whether `narrate_finish` can run now.
    pub narrate_finish_enabled: Signal<bool>,
    /// Whether `open_capture_window` can run now.
    pub open_capture_window_enabled: Signal<bool>,
    /// Whether `open_recent_file` can run now.
    pub open_recent_file_enabled: Signal<bool>,
    /// Whether `auto_trim` can run now.
    pub auto_trim_enabled: Signal<bool>,
    /// Whether `tool_crop_menu` can run now.
    pub tool_crop_menu_enabled: Signal<bool>,
    /// Whether `tool_select` can run now.
    pub tool_select_enabled: Signal<bool>,
    /// Whether `tool_arrow` can run now.
    pub tool_arrow_enabled: Signal<bool>,
    /// Whether `tool_line` can run now.
    pub tool_line_enabled: Signal<bool>,
    /// Whether `tool_shape` can run now.
    pub tool_shape_enabled: Signal<bool>,
    /// Whether `tool_text` can run now.
    pub tool_text_enabled: Signal<bool>,
    /// Whether `tool_callout` can run now.
    pub tool_callout_enabled: Signal<bool>,
    /// Whether `tool_step` can run now.
    pub tool_step_enabled: Signal<bool>,
    /// Whether `tool_pen` can run now.
    pub tool_pen_enabled: Signal<bool>,
    /// Whether `tool_highlight` can run now.
    pub tool_highlight_enabled: Signal<bool>,
    /// Whether `tool_blur` can run now.
    pub tool_blur_enabled: Signal<bool>,
    /// Whether `tool_magnify` can run now.
    pub tool_magnify_enabled: Signal<bool>,
    /// Whether `tool_spotlight` can run now.
    pub tool_spotlight_enabled: Signal<bool>,
    /// Whether `tool_stamp` can run now.
    pub tool_stamp_enabled: Signal<bool>,
    /// Whether `tool_fill` can run now.
    pub tool_fill_enabled: Signal<bool>,
    /// Whether `tool_crop` can run now.
    pub tool_crop_enabled: Signal<bool>,
    /// Whether `tool_cut_out` can run now.
    pub tool_cut_out_enabled: Signal<bool>,
    /// Whether `tool_selection` can run now.
    pub tool_selection_enabled: Signal<bool>,
    /// Whether `tool_eyedropper` can run now.
    pub tool_eyedropper_enabled: Signal<bool>,
    /// Whether `tool_smart_move` can run now.
    pub tool_smart_move_enabled: Signal<bool>,
    /// Whether `tool_text_replace` can run now.
    pub tool_text_replace_enabled: Signal<bool>,
}

impl MainViewModelSink {
    /// Creates the signals, on the UI thread.
    fn new() -> Self {
        Self {
            title: Signal::new(String::from("Jiffshot Editor")),
            status: Signal::new(String::from("Ready")),
            tool: Signal::new(String::from("Arrow")),
            frame_id: Signal::new(0),
            stroke_color: Signal::new(String::from("#FFE53935")),
            fill_color: Signal::new(String::from("#00000000")),
            text_color: Signal::new(String::from("#FF000000")),
            stroke_width: Signal::new(6.0),
            font_size: Signal::new(28.0),
            font_family: Signal::new(String::from("Segoe UI")),
            bold: Signal::new(false),
            italic: Signal::new(false),
            shadow: Signal::new(false),
            opacity: Signal::new(100.0),
            text: Signal::new(String::new()),
            variant: Signal::new(String::new()),
            amount: Signal::new(0.0),
            align: Signal::new(String::from("Left")),
            dash: Signal::new(String::from("Solid")),
            start_cap: Signal::new(String::from("None")),
            end_cap: Signal::new(String::from("None")),
            selection: Signal::new(String::new()),
            panel: Signal::new(String::new()),
            amount_label: Signal::new(String::new()),
            focus_text: Signal::new(0),
            zoom_request: Signal::new(String::new()),
            action: Signal::new(String::new()),
            mode: Signal::new(String::from("Image")),
            video_position: Signal::new(0.0),
            video_duration: Signal::new(0.0),
            video_info: Signal::new(String::new()),
            trim_start: Signal::new(0.0),
            trim_end: Signal::new(0.0),
            playing: Signal::new(false),
            document_info: Signal::new(String::new()),
            rotation: Signal::new(0.0),
            library_info: Signal::new(String::new()),
            background_fill: Signal::new(String::from("Auto-fill")),
            selection_count: Signal::new(0),
            head_size: Signal::new(1.0),
            shadow_angle: Signal::new(45.0),
            shadow_distance: Signal::new(5.0),
            shadow_blur: Signal::new(5.0),
            shadow_color: Signal::new(String::from("#73000000")),
            outline_color: Signal::new(String::from("#00000000")),
            outline_width: Signal::new(0.0),
            underline: Signal::new(false),
            strike: Signal::new(false),
            v_align: Signal::new(String::from("Top")),
            padding: Signal::new(-1.0),
            fill2: Signal::new(String::from("#00000000")),
            gradient_radial: Signal::new(false),
            gradient_angle: Signal::new(90.0),
            canvas_color: Signal::new(String::from("#FFFFFFFF")),
            theme: Signal::new(String::from("Dark")),
            gif_preset: Signal::new(String::from("Screen recording")),
            gif_loop: Signal::new(true),
            gif_fade: Signal::new(false),
            gif_dither: Signal::new(false),
            video_volume: Signal::new(1.0),
            muted: Signal::new(false),
            library_search: Signal::new(String::new()),
            library_filter: Signal::new(String::from("All")),
            library_sort: Signal::new(String::from("Newest")),
            thumb_size: Signal::new(120.0),
            library_expanded: Signal::new(false),
            library_selected: Signal::new(0),
            template_theme: Signal::new(String::from("Light")),
            template_captions: Signal::new(false),
            template_numbers: Signal::new(false),
            template_footer: Signal::new(String::new()),
            zone_fit: Signal::new(false),
            tool_order: Signal::new(String::new()),
            narrating: Signal::new(false),
            show_side_bar: Signal::new(true),
            show_effects: Signal::new(false),
            show_tray: Signal::new(true),
            tool_labels: Signal::new(true),
            toast: Signal::new(String::new()),
            grab_text: Signal::new(String::new()),
            widget: Signal::new(false),
            hud: Signal::new(String::new()),
            overlay: Signal::new(String::new()),
            screen_draw: Signal::new(String::new()),
            prefs: Signal::new(String::new()),
            variants: Signal::new(Vec::new()),
            fonts: Signal::new(Vec::new()),
            library: Signal::new(Vec::new()),
            quick_styles: Signal::new(Vec::new()),
            effects: Signal::new(Vec::new()),
            templates: Signal::new(Vec::new()),
            shares: Signal::new(Vec::new()),
            library_filters: Signal::new(Vec::new()),
            library_sorts: Signal::new(Vec::new()),
            template_themes: Signal::new(Vec::new()),
            recent_files: Signal::new(Vec::new()),
            new_image_enabled: Signal::new(true),
            open_enabled: Signal::new(true),
            save_enabled: Signal::new(true),
            save_as_enabled: Signal::new(true),
            export_enabled: Signal::new(true),
            copy_image_enabled: Signal::new(true),
            paste_enabled: Signal::new(true),
            new_capture_enabled: Signal::new(true),
            combine_images_enabled: Signal::new(true),
            open_library_folder_enabled: Signal::new(true),
            exit_enabled: Signal::new(true),
            undo_enabled: Signal::new(true),
            redo_enabled: Signal::new(true),
            copy_enabled: Signal::new(true),
            cut_enabled: Signal::new(true),
            delete_enabled: Signal::new(true),
            duplicate_enabled: Signal::new(true),
            bring_to_front_enabled: Signal::new(true),
            bring_forward_enabled: Signal::new(true),
            send_backward_enabled: Signal::new(true),
            send_to_back_enabled: Signal::new(true),
            save_quick_style_enabled: Signal::new(true),
            rotate_left_enabled: Signal::new(true),
            rotate_right_enabled: Signal::new(true),
            flip_horizontal_enabled: Signal::new(true),
            flip_vertical_enabled: Signal::new(true),
            resize50_enabled: Signal::new(true),
            resize200_enabled: Signal::new(true),
            flatten_enabled: Signal::new(true),
            zoom_in_enabled: Signal::new(true),
            zoom_out_enabled: Signal::new(true),
            zoom_fit_enabled: Signal::new(true),
            zoom_actual_enabled: Signal::new(true),
            grab_text_enabled: Signal::new(true),
            simplify_enabled: Signal::new(true),
            play_pause_enabled: Signal::new(true),
            set_trim_start_enabled: Signal::new(true),
            set_trim_end_enabled: Signal::new(true),
            export_video_enabled: Signal::new(true),
            export_gif_enabled: Signal::new(true),
            frame_to_image_enabled: Signal::new(true),
            record_video_enabled: Signal::new(true),
            select_all_enabled: Signal::new(true),
            deselect_all_enabled: Signal::new(true),
            invert_selection_enabled: Signal::new(true),
            group_enabled: Signal::new(true),
            ungroup_enabled: Signal::new(true),
            flatten_selected_enabled: Signal::new(true),
            smart_redact_enabled: Signal::new(true),
            new_from_clipboard_enabled: Signal::new(true),
            print_enabled: Signal::new(true),
            close_file_enabled: Signal::new(true),
            duplicate_file_enabled: Signal::new(true),
            delete_file_enabled: Signal::new(true),
            tool_magic_wand_enabled: Signal::new(true),
            tool_eraser_enabled: Signal::new(true),
            expand_canvas_enabled: Signal::new(true),
            cycle_theme_enabled: Signal::new(true),
            prev_frame_enabled: Signal::new(true),
            next_frame_enabled: Signal::new(true),
            cut_selection_enabled: Signal::new(true),
            silent_copy_enabled: Signal::new(true),
            combine_videos_enabled: Signal::new(true),
            video_from_images_enabled: Signal::new(true),
            toggle_mute_enabled: Signal::new(true),
            toggle_library_view_enabled: Signal::new(true),
            combine_selected_enabled: Signal::new(true),
            show_closed_enabled: Signal::new(true),
            reset_toolbar_enabled: Signal::new(true),
            narrate_images_enabled: Signal::new(true),
            narrate_next_enabled: Signal::new(true),
            narrate_finish_enabled: Signal::new(true),
            open_capture_window_enabled: Signal::new(true),
            open_recent_file_enabled: Signal::new(true),
            auto_trim_enabled: Signal::new(true),
            tool_crop_menu_enabled: Signal::new(true),
            tool_select_enabled: Signal::new(true),
            tool_arrow_enabled: Signal::new(true),
            tool_line_enabled: Signal::new(true),
            tool_shape_enabled: Signal::new(true),
            tool_text_enabled: Signal::new(true),
            tool_callout_enabled: Signal::new(true),
            tool_step_enabled: Signal::new(true),
            tool_pen_enabled: Signal::new(true),
            tool_highlight_enabled: Signal::new(true),
            tool_blur_enabled: Signal::new(true),
            tool_magnify_enabled: Signal::new(true),
            tool_spotlight_enabled: Signal::new(true),
            tool_stamp_enabled: Signal::new(true),
            tool_fill_enabled: Signal::new(true),
            tool_crop_enabled: Signal::new(true),
            tool_cut_out_enabled: Signal::new(true),
            tool_selection_enabled: Signal::new(true),
            tool_eyedropper_enabled: Signal::new(true),
            tool_smart_move_enabled: Signal::new(true),
            tool_text_replace_enabled: Signal::new(true),
        }
    }

    pub fn set_title(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.title, value.as_ref().to_owned())
    }
    pub fn set_status(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.status, value.as_ref().to_owned())
    }
    pub fn set_tool(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.tool, value.as_ref().to_owned())
    }
    pub fn set_frame_id(&self, value: i64) -> Result<()> {
        publish(self.frame_id, value)
    }
    pub fn set_stroke_color(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.stroke_color, value.as_ref().to_owned())
    }
    pub fn set_fill_color(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.fill_color, value.as_ref().to_owned())
    }
    pub fn set_text_color(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.text_color, value.as_ref().to_owned())
    }
    pub fn set_stroke_width(&self, value: f64) -> Result<()> {
        publish(self.stroke_width, value)
    }
    pub fn set_font_size(&self, value: f64) -> Result<()> {
        publish(self.font_size, value)
    }
    pub fn set_font_family(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.font_family, value.as_ref().to_owned())
    }
    pub fn set_bold(&self, value: bool) -> Result<()> {
        publish(self.bold, value)
    }
    pub fn set_italic(&self, value: bool) -> Result<()> {
        publish(self.italic, value)
    }
    pub fn set_shadow(&self, value: bool) -> Result<()> {
        publish(self.shadow, value)
    }
    pub fn set_opacity(&self, value: f64) -> Result<()> {
        publish(self.opacity, value)
    }
    pub fn set_text(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.text, value.as_ref().to_owned())
    }
    pub fn set_variant(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.variant, value.as_ref().to_owned())
    }
    pub fn set_amount(&self, value: f64) -> Result<()> {
        publish(self.amount, value)
    }
    pub fn set_align(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.align, value.as_ref().to_owned())
    }
    pub fn set_dash(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.dash, value.as_ref().to_owned())
    }
    pub fn set_start_cap(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.start_cap, value.as_ref().to_owned())
    }
    pub fn set_end_cap(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.end_cap, value.as_ref().to_owned())
    }
    pub fn set_selection(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.selection, value.as_ref().to_owned())
    }
    pub fn set_panel(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.panel, value.as_ref().to_owned())
    }
    pub fn set_amount_label(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.amount_label, value.as_ref().to_owned())
    }
    pub fn set_focus_text(&self, value: i64) -> Result<()> {
        publish(self.focus_text, value)
    }
    pub fn set_zoom_request(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.zoom_request, value.as_ref().to_owned())
    }
    pub fn set_mode(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.mode, value.as_ref().to_owned())
    }
    pub fn set_video_position(&self, value: f64) -> Result<()> {
        publish(self.video_position, value)
    }
    pub fn set_video_duration(&self, value: f64) -> Result<()> {
        publish(self.video_duration, value)
    }
    pub fn set_video_info(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.video_info, value.as_ref().to_owned())
    }
    pub fn set_trim_start(&self, value: f64) -> Result<()> {
        publish(self.trim_start, value)
    }
    pub fn set_trim_end(&self, value: f64) -> Result<()> {
        publish(self.trim_end, value)
    }
    pub fn set_playing(&self, value: bool) -> Result<()> {
        publish(self.playing, value)
    }
    pub fn set_document_info(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.document_info, value.as_ref().to_owned())
    }
    pub fn set_rotation(&self, value: f64) -> Result<()> {
        publish(self.rotation, value)
    }
    pub fn set_library_info(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.library_info, value.as_ref().to_owned())
    }
    pub fn set_background_fill(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.background_fill, value.as_ref().to_owned())
    }
    pub fn set_selection_count(&self, value: i64) -> Result<()> {
        publish(self.selection_count, value)
    }
    pub fn set_head_size(&self, value: f64) -> Result<()> {
        publish(self.head_size, value)
    }
    pub fn set_shadow_angle(&self, value: f64) -> Result<()> {
        publish(self.shadow_angle, value)
    }
    pub fn set_shadow_distance(&self, value: f64) -> Result<()> {
        publish(self.shadow_distance, value)
    }
    pub fn set_shadow_blur(&self, value: f64) -> Result<()> {
        publish(self.shadow_blur, value)
    }
    pub fn set_shadow_color(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.shadow_color, value.as_ref().to_owned())
    }
    pub fn set_outline_color(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.outline_color, value.as_ref().to_owned())
    }
    pub fn set_outline_width(&self, value: f64) -> Result<()> {
        publish(self.outline_width, value)
    }
    pub fn set_underline(&self, value: bool) -> Result<()> {
        publish(self.underline, value)
    }
    pub fn set_strike(&self, value: bool) -> Result<()> {
        publish(self.strike, value)
    }
    pub fn set_v_align(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.v_align, value.as_ref().to_owned())
    }
    pub fn set_padding(&self, value: f64) -> Result<()> {
        publish(self.padding, value)
    }
    pub fn set_fill2(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.fill2, value.as_ref().to_owned())
    }
    pub fn set_gradient_radial(&self, value: bool) -> Result<()> {
        publish(self.gradient_radial, value)
    }
    pub fn set_gradient_angle(&self, value: f64) -> Result<()> {
        publish(self.gradient_angle, value)
    }
    pub fn set_theme(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.theme, value.as_ref().to_owned())
    }
    pub fn set_gif_preset(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.gif_preset, value.as_ref().to_owned())
    }
    pub fn set_gif_loop(&self, value: bool) -> Result<()> {
        publish(self.gif_loop, value)
    }
    pub fn set_gif_fade(&self, value: bool) -> Result<()> {
        publish(self.gif_fade, value)
    }
    pub fn set_gif_dither(&self, value: bool) -> Result<()> {
        publish(self.gif_dither, value)
    }
    pub fn set_video_volume(&self, value: f64) -> Result<()> {
        publish(self.video_volume, value)
    }
    pub fn set_muted(&self, value: bool) -> Result<()> {
        publish(self.muted, value)
    }
    pub fn set_library_search(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.library_search, value.as_ref().to_owned())
    }
    pub fn set_library_filter(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.library_filter, value.as_ref().to_owned())
    }
    pub fn set_library_sort(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.library_sort, value.as_ref().to_owned())
    }
    pub fn set_thumb_size(&self, value: f64) -> Result<()> {
        publish(self.thumb_size, value)
    }
    pub fn set_library_expanded(&self, value: bool) -> Result<()> {
        publish(self.library_expanded, value)
    }
    pub fn set_library_selected(&self, value: i64) -> Result<()> {
        publish(self.library_selected, value)
    }
    pub fn set_template_theme(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.template_theme, value.as_ref().to_owned())
    }
    pub fn set_template_captions(&self, value: bool) -> Result<()> {
        publish(self.template_captions, value)
    }
    pub fn set_template_numbers(&self, value: bool) -> Result<()> {
        publish(self.template_numbers, value)
    }
    pub fn set_template_footer(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.template_footer, value.as_ref().to_owned())
    }
    pub fn set_zone_fit(&self, value: bool) -> Result<()> {
        publish(self.zone_fit, value)
    }
    pub fn set_tool_order(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.tool_order, value.as_ref().to_owned())
    }
    pub fn set_narrating(&self, value: bool) -> Result<()> {
        publish(self.narrating, value)
    }
    pub fn set_show_side_bar(&self, value: bool) -> Result<()> {
        publish(self.show_side_bar, value)
    }
    pub fn set_show_effects(&self, value: bool) -> Result<()> {
        publish(self.show_effects, value)
    }
    pub fn set_show_tray(&self, value: bool) -> Result<()> {
        publish(self.show_tray, value)
    }
    pub fn set_tool_labels(&self, value: bool) -> Result<()> {
        publish(self.tool_labels, value)
    }
    pub fn set_toast(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.toast, value.as_ref().to_owned())
    }
    pub fn set_grab_text(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.grab_text, value.as_ref().to_owned())
    }
    pub fn set_widget(&self, value: bool) -> Result<()> {
        publish(self.widget, value)
    }
    pub fn set_hud(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.hud, value.as_ref().to_owned())
    }
    pub fn set_overlay(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.overlay, value.as_ref().to_owned())
    }
    pub fn set_screen_draw(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.screen_draw, value.as_ref().to_owned())
    }
    pub fn set_prefs(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.prefs, value.as_ref().to_owned())
    }
    pub fn set_undo_enabled(&self, enabled: bool) -> Result<()> {
        publish(self.undo_enabled, enabled)
    }
    pub fn set_redo_enabled(&self, enabled: bool) -> Result<()> {
        publish(self.redo_enabled, enabled)
    }
    pub fn set_cut_enabled(&self, enabled: bool) -> Result<()> {
        publish(self.cut_enabled, enabled)
    }
    pub fn set_delete_enabled(&self, enabled: bool) -> Result<()> {
        publish(self.delete_enabled, enabled)
    }
    pub fn set_duplicate_enabled(&self, enabled: bool) -> Result<()> {
        publish(self.duplicate_enabled, enabled)
    }
    pub fn set_bring_to_front_enabled(&self, enabled: bool) -> Result<()> {
        publish(self.bring_to_front_enabled, enabled)
    }
    pub fn set_bring_forward_enabled(&self, enabled: bool) -> Result<()> {
        publish(self.bring_forward_enabled, enabled)
    }
    pub fn set_send_backward_enabled(&self, enabled: bool) -> Result<()> {
        publish(self.send_backward_enabled, enabled)
    }
    pub fn set_send_to_back_enabled(&self, enabled: bool) -> Result<()> {
        publish(self.send_to_back_enabled, enabled)
    }
    pub fn add_variants(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.variants, move |list| list.push(value))
    }
    pub fn clear_variants(&self) -> Result<()> {
        publish_update(self.variants, Vec::clear)
    }
    pub fn add_fonts(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.fonts, move |list| list.push(value))
    }
    pub fn add_library(&self, value: impl LibraryItemViewModel) -> Result<()> {
        Element::add(self.library, value, |m| Box::new(m) as Box<dyn LibraryItemViewModel>, |m, sink| m.attach(LibraryItemViewModelSink(sink)), |m| m.detach())
    }
    pub fn clear_library(&self) -> Result<()> {
        Element::clear(self.library)
    }
    pub fn add_quick_styles(&self, value: impl QuickStyleViewModel) -> Result<()> {
        Element::add(self.quick_styles, value, |m| Box::new(m) as Box<dyn QuickStyleViewModel>, |m, sink| m.attach(QuickStyleViewModelSink(sink)), |m| m.detach())
    }
    pub fn clear_quick_styles(&self) -> Result<()> {
        Element::clear(self.quick_styles)
    }
    pub fn add_effects(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.effects, move |list| list.push(value))
    }
    pub fn add_templates(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.templates, move |list| list.push(value))
    }
    pub fn add_shares(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.shares, move |list| list.push(value))
    }
    pub fn clear_shares(&self) -> Result<()> {
        publish_update(self.shares, Vec::clear)
    }
    pub fn add_library_filters(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.library_filters, move |list| list.push(value))
    }
    pub fn clear_library_filters(&self) -> Result<()> {
        publish_update(self.library_filters, Vec::clear)
    }
    pub fn add_library_sorts(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.library_sorts, move |list| list.push(value))
    }
    pub fn clear_library_sorts(&self) -> Result<()> {
        publish_update(self.library_sorts, Vec::clear)
    }
    pub fn add_template_themes(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.template_themes, move |list| list.push(value))
    }
    pub fn add_recent_files(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.recent_files, move |list| list.push(value))
    }
    pub fn clear_recent_files(&self) -> Result<()> {
        publish_update(self.recent_files, Vec::clear)
    }
}

/// A `MainViewModel` bound to its signals: what a window is built from.
#[derive(Clone)]
pub struct MainBinding {
    pub sink: MainViewModelSink,
    pub model: ModelCell<dyn MainViewModel>,
}

impl MainBinding {
    /// Creates the sink, watches the writable properties and attaches the
    /// model. On the UI thread.
    pub fn new(model: impl MainViewModel) -> Result<Self> {
        let sink = MainViewModelSink::new();
        let model: ModelCell<dyn MainViewModel> = ModelCell::new(Box::new(model));
        watch(sink.stroke_color, &model, |m, v| m.set_stroke_color(v))?;
        watch(sink.fill_color, &model, |m, v| m.set_fill_color(v))?;
        watch(sink.text_color, &model, |m, v| m.set_text_color(v))?;
        watch(sink.stroke_width, &model, |m, v| m.set_stroke_width(v))?;
        watch(sink.font_size, &model, |m, v| m.set_font_size(v))?;
        watch(sink.font_family, &model, |m, v| m.set_font_family(v))?;
        watch(sink.bold, &model, |m, v| m.set_bold(v))?;
        watch(sink.italic, &model, |m, v| m.set_italic(v))?;
        watch(sink.shadow, &model, |m, v| m.set_shadow(v))?;
        watch(sink.opacity, &model, |m, v| m.set_opacity(v))?;
        watch(sink.text, &model, |m, v| m.set_text(v))?;
        watch(sink.variant, &model, |m, v| m.set_variant(v))?;
        watch(sink.amount, &model, |m, v| m.set_amount(v))?;
        watch(sink.align, &model, |m, v| m.set_align(v))?;
        watch(sink.dash, &model, |m, v| m.set_dash(v))?;
        watch(sink.start_cap, &model, |m, v| m.set_start_cap(v))?;
        watch(sink.end_cap, &model, |m, v| m.set_end_cap(v))?;
        watch(sink.action, &model, |m, v| m.set_action(v))?;
        watch(sink.video_position, &model, |m, v| m.set_video_position(v))?;
        watch(sink.rotation, &model, |m, v| m.set_rotation(v))?;
        watch(sink.background_fill, &model, |m, v| m.set_background_fill(v))?;
        watch(sink.head_size, &model, |m, v| m.set_head_size(v))?;
        watch(sink.shadow_angle, &model, |m, v| m.set_shadow_angle(v))?;
        watch(sink.shadow_distance, &model, |m, v| m.set_shadow_distance(v))?;
        watch(sink.shadow_blur, &model, |m, v| m.set_shadow_blur(v))?;
        watch(sink.shadow_color, &model, |m, v| m.set_shadow_color(v))?;
        watch(sink.outline_color, &model, |m, v| m.set_outline_color(v))?;
        watch(sink.outline_width, &model, |m, v| m.set_outline_width(v))?;
        watch(sink.underline, &model, |m, v| m.set_underline(v))?;
        watch(sink.strike, &model, |m, v| m.set_strike(v))?;
        watch(sink.v_align, &model, |m, v| m.set_v_align(v))?;
        watch(sink.padding, &model, |m, v| m.set_padding(v))?;
        watch(sink.fill2, &model, |m, v| m.set_fill2(v))?;
        watch(sink.gradient_radial, &model, |m, v| m.set_gradient_radial(v))?;
        watch(sink.gradient_angle, &model, |m, v| m.set_gradient_angle(v))?;
        watch(sink.canvas_color, &model, |m, v| m.set_canvas_color(v))?;
        watch(sink.gif_preset, &model, |m, v| m.set_gif_preset(v))?;
        watch(sink.gif_loop, &model, |m, v| m.set_gif_loop(v))?;
        watch(sink.gif_fade, &model, |m, v| m.set_gif_fade(v))?;
        watch(sink.gif_dither, &model, |m, v| m.set_gif_dither(v))?;
        watch(sink.video_volume, &model, |m, v| m.set_video_volume(v))?;
        watch(sink.library_search, &model, |m, v| m.set_library_search(v))?;
        watch(sink.library_filter, &model, |m, v| m.set_library_filter(v))?;
        watch(sink.library_sort, &model, |m, v| m.set_library_sort(v))?;
        watch(sink.thumb_size, &model, |m, v| m.set_thumb_size(v))?;
        watch(sink.library_expanded, &model, |m, v| m.set_library_expanded(v))?;
        watch(sink.template_theme, &model, |m, v| m.set_template_theme(v))?;
        watch(sink.template_captions, &model, |m, v| m.set_template_captions(v))?;
        watch(sink.template_numbers, &model, |m, v| m.set_template_numbers(v))?;
        watch(sink.template_footer, &model, |m, v| m.set_template_footer(v))?;
        watch(sink.show_side_bar, &model, |m, v| m.set_show_side_bar(v))?;
        watch(sink.show_effects, &model, |m, v| m.set_show_effects(v))?;
        watch(sink.show_tray, &model, |m, v| m.set_show_tray(v))?;
        watch(sink.tool_labels, &model, |m, v| m.set_tool_labels(v))?;
        model.call(|m| m.attach(sink))?;
        Ok(Self { sink, model })
    }
}

// ---- LibraryItemViewModel ----------------------------------------------

pub trait LibraryItemViewModel: Send + 'static {
    fn attach(&mut self, sink: LibraryItemViewModelSink) -> Result<()>;
    fn detach(&mut self) -> Result<()>;
    fn open(&mut self) -> Result<()>;
    fn toggle_pin(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn delete(&mut self) -> Result<()>;
    fn rename(&mut self, value: String) -> Result<()>;
    fn set_tags(&mut self, value: String) -> Result<()>;
    fn toggle_select(&mut self) -> Result<()>;
    fn show_in_folder(&mut self) -> Result<()>;
}

/// A `LibraryItemViewModel`'s published values.
#[derive(Clone, Debug, PartialEq)]
pub struct LibraryItemFields {
    pub path: String,
    pub label: String,
    pub thumbnail: String,
    pub is_current: bool,
    pub kind: String,
    pub is_pinned: bool,
    pub tags: String,
    pub info: String,
    pub is_selected: bool,
    pub new_name: String,
    pub tag_text: String,
    pub unsaved: bool,
    pub thumb_height: f64,
}

impl Default for LibraryItemFields {
    fn default() -> Self {
        Self {
            path: String::new(),
            label: String::new(),
            thumbnail: String::new(),
            is_current: false,
            kind: String::from("Image"),
            is_pinned: false,
            tags: String::new(),
            info: String::new(),
            is_selected: false,
            new_name: String::new(),
            tag_text: String::new(),
            unsaved: false,
            thumb_height: 88.0,
        }
    }
}

/// One element of a `LibraryItemViewModel` collection: its values and its model.
pub type LibraryItemItem = Element<LibraryItemFields, dyn LibraryItemViewModel>;

#[derive(Clone, Copy, Debug)]
pub struct LibraryItemViewModelSink(ElementSink<LibraryItemFields>);

impl LibraryItemViewModelSink {
    pub fn set_path(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.path = value)
    }
    pub fn set_label(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.label = value)
    }
    pub fn set_thumbnail(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.thumbnail = value)
    }
    pub fn set_is_current(&self, value: bool) -> Result<()> {
        self.0.update(move |f| f.is_current = value)
    }
    pub fn set_kind(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.kind = value)
    }
    pub fn set_is_pinned(&self, value: bool) -> Result<()> {
        self.0.update(move |f| f.is_pinned = value)
    }
    pub fn set_tags(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.tags = value)
    }
    pub fn set_info(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.info = value)
    }
    pub fn set_is_selected(&self, value: bool) -> Result<()> {
        self.0.update(move |f| f.is_selected = value)
    }
    pub fn set_new_name(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.new_name = value)
    }
    pub fn set_tag_text(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.tag_text = value)
    }
    pub fn set_unsaved(&self, value: bool) -> Result<()> {
        self.0.update(move |f| f.unsaved = value)
    }
    pub fn set_thumb_height(&self, value: f64) -> Result<()> {
        self.0.update(move |f| f.thumb_height = value)
    }
}


// ---- QuickStyleViewModel -----------------------------------------------

pub trait QuickStyleViewModel: Send + 'static {
    fn attach(&mut self, sink: QuickStyleViewModelSink) -> Result<()>;
    fn detach(&mut self) -> Result<()>;
    fn apply(&mut self) -> Result<()>;
    fn delete(&mut self) -> Result<()>;
    fn move_earlier(&mut self) -> Result<()>;
    fn move_later(&mut self) -> Result<()>;
    fn toggle_favorite(&mut self) -> Result<()>;
}

/// A `QuickStyleViewModel`'s published values.
#[derive(Clone, Debug, PartialEq)]
pub struct QuickStyleFields {
    pub name: String,
    pub stroke: String,
    pub fill: String,
    pub text_color: String,
    pub width: f64,
    pub is_favorite: bool,
}

impl Default for QuickStyleFields {
    fn default() -> Self {
        Self {
            name: String::new(),
            stroke: String::from("#00000000"),
            fill: String::from("#00000000"),
            text_color: String::from("#00000000"),
            width: 0.0,
            is_favorite: false,
        }
    }
}

/// One element of a `QuickStyleViewModel` collection: its values and its model.
pub type QuickStyleItem = Element<QuickStyleFields, dyn QuickStyleViewModel>;

#[derive(Clone, Copy, Debug)]
pub struct QuickStyleViewModelSink(ElementSink<QuickStyleFields>);

impl QuickStyleViewModelSink {
    pub fn set_name(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.name = value)
    }
    pub fn set_stroke(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.stroke = value)
    }
    pub fn set_fill(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.fill = value)
    }
    pub fn set_text_color(&self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.0.update(move |f| f.text_color = value)
    }
    pub fn set_width(&self, value: f64) -> Result<()> {
        self.0.update(move |f| f.width = value)
    }
    pub fn set_is_favorite(&self, value: bool) -> Result<()> {
        self.0.update(move |f| f.is_favorite = value)
    }
}


// ---- TrayViewModel -----------------------------------------------------

pub trait TrayViewModel: Send + 'static {
    fn attach(&mut self, sink: TrayViewModelSink) -> Result<()>;
    fn detach(&mut self) -> Result<()>;
    fn set_copy_to_clipboard(&mut self, value: bool) -> Result<()>;
    fn set_open_in_editor(&mut self, value: bool) -> Result<()>;
    fn set_include_cursor(&mut self, value: bool) -> Result<()>;
    fn set_show_widget(&mut self, value: bool) -> Result<()>;
    fn set_run_at_startup(&mut self, value: bool) -> Result<()>;
    fn set_time_lapse(&mut self, value: bool) -> Result<()>;
    fn set_schedule(&mut self, value: bool) -> Result<()>;
    fn capture(&mut self) -> Result<()>;
    fn capture_delayed(&mut self) -> Result<()>;
    fn capture_screen(&mut self) -> Result<()>;
    fn repeat_last(&mut self) -> Result<()>;
    fn capture_window(&mut self) -> Result<()>;
    fn show_editor(&mut self) -> Result<()>;
    fn scrolling(&mut self) -> Result<()>;
    fn scrolling_horizontal(&mut self) -> Result<()>;
    fn scrolling_both(&mut self) -> Result<()>;
    fn panoramic(&mut self) -> Result<()>;
    fn freehand(&mut self) -> Result<()>;
    fn multiple_areas(&mut self) -> Result<()>;
    fn fixed_size(&mut self) -> Result<()>;
    fn menu_capture(&mut self) -> Result<()>;
    fn grab_text(&mut self) -> Result<()>;
    fn step_capture(&mut self) -> Result<()>;
    fn from_clipboard(&mut self) -> Result<()>;
    fn from_file(&mut self) -> Result<()>;
    fn record_video(&mut self) -> Result<()>;
    fn open_library_folder(&mut self) -> Result<()>;
    fn exit(&mut self) -> Result<()>;
    fn preferences(&mut self) -> Result<()>;
    fn open_recent_capture(&mut self, value: String) -> Result<()>;
}

#[derive(Clone, Copy, Debug)]
pub struct TrayViewModelSink {
    pub copy_to_clipboard: Signal<bool>,
    pub open_in_editor: Signal<bool>,
    pub include_cursor: Signal<bool>,
    pub show_widget: Signal<bool>,
    pub run_at_startup: Signal<bool>,
    pub time_lapse: Signal<bool>,
    pub schedule: Signal<bool>,
    pub can_repeat: Signal<bool>,
    pub recent_captures: Signal<Vec<String>>,
}

impl TrayViewModelSink {
    /// Creates the signals, on the UI thread.
    fn new() -> Self {
        Self {
            copy_to_clipboard: Signal::new(true),
            open_in_editor: Signal::new(true),
            include_cursor: Signal::new(false),
            show_widget: Signal::new(false),
            run_at_startup: Signal::new(false),
            time_lapse: Signal::new(false),
            schedule: Signal::new(false),
            can_repeat: Signal::new(false),
            recent_captures: Signal::new(Vec::new()),
        }
    }

    pub fn set_copy_to_clipboard(&self, value: bool) -> Result<()> {
        publish(self.copy_to_clipboard, value)
    }
    pub fn set_open_in_editor(&self, value: bool) -> Result<()> {
        publish(self.open_in_editor, value)
    }
    pub fn set_include_cursor(&self, value: bool) -> Result<()> {
        publish(self.include_cursor, value)
    }
    pub fn set_show_widget(&self, value: bool) -> Result<()> {
        publish(self.show_widget, value)
    }
    pub fn set_run_at_startup(&self, value: bool) -> Result<()> {
        publish(self.run_at_startup, value)
    }
    pub fn set_time_lapse(&self, value: bool) -> Result<()> {
        publish(self.time_lapse, value)
    }
    pub fn set_schedule(&self, value: bool) -> Result<()> {
        publish(self.schedule, value)
    }
    pub fn set_can_repeat(&self, value: bool) -> Result<()> {
        publish(self.can_repeat, value)
    }
    pub fn add_recent_captures(&self, value: impl AsRef<str>) -> Result<()> {
        let value = value.as_ref().to_owned();
        publish_update(self.recent_captures, move |list| list.push(value))
    }
    pub fn clear_recent_captures(&self) -> Result<()> {
        publish_update(self.recent_captures, Vec::clear)
    }
}

/// A `TrayViewModel` bound to its signals: what a window is built from.
#[derive(Clone)]
pub struct TrayBinding {
    pub sink: TrayViewModelSink,
    pub model: ModelCell<dyn TrayViewModel>,
}

impl TrayBinding {
    /// Creates the sink, watches the writable properties and attaches the
    /// model. On the UI thread.
    pub fn new(model: impl TrayViewModel) -> Result<Self> {
        let sink = TrayViewModelSink::new();
        let model: ModelCell<dyn TrayViewModel> = ModelCell::new(Box::new(model));
        watch(sink.copy_to_clipboard, &model, |m, v| m.set_copy_to_clipboard(v))?;
        watch(sink.open_in_editor, &model, |m, v| m.set_open_in_editor(v))?;
        watch(sink.include_cursor, &model, |m, v| m.set_include_cursor(v))?;
        watch(sink.show_widget, &model, |m, v| m.set_show_widget(v))?;
        watch(sink.run_at_startup, &model, |m, v| m.set_run_at_startup(v))?;
        watch(sink.time_lapse, &model, |m, v| m.set_time_lapse(v))?;
        watch(sink.schedule, &model, |m, v| m.set_schedule(v))?;
        model.call(|m| m.attach(sink))?;
        Ok(Self { sink, model })
    }
}

// ---- CaptureViewModel --------------------------------------------------

pub trait CaptureViewModel: Send + 'static {
    fn attach(&mut self, sink: CaptureViewModelSink) -> Result<()>;
    fn detach(&mut self) -> Result<()>;
    fn set_action(&mut self, value: String) -> Result<()>;
    fn run(&mut self, value: String) -> Result<()>;
}

#[derive(Clone, Copy, Debug)]
pub struct CaptureViewModelSink {
    pub view: Signal<String>,
    pub menu: Signal<String>,
    pub panel: Signal<String>,
    pub problem: Signal<String>,
    pub action: Signal<String>,
}

impl CaptureViewModelSink {
    /// Creates the signals, on the UI thread.
    fn new() -> Self {
        Self {
            view: Signal::new(String::from("{}")),
            menu: Signal::new(String::new()),
            panel: Signal::new(String::new()),
            problem: Signal::new(String::new()),
            action: Signal::new(String::new()),
        }
    }

    pub fn set_view(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.view, value.as_ref().to_owned())
    }
    pub fn set_menu(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.menu, value.as_ref().to_owned())
    }
    pub fn set_panel(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.panel, value.as_ref().to_owned())
    }
    pub fn set_problem(&self, value: impl AsRef<str>) -> Result<()> {
        publish(self.problem, value.as_ref().to_owned())
    }
}

/// A `CaptureViewModel` bound to its signals: what a window is built from.
#[derive(Clone)]
pub struct CaptureBinding {
    pub sink: CaptureViewModelSink,
    pub model: ModelCell<dyn CaptureViewModel>,
}

impl CaptureBinding {
    /// Creates the sink, watches the writable properties and attaches the
    /// model. On the UI thread.
    pub fn new(model: impl CaptureViewModel) -> Result<Self> {
        let sink = CaptureViewModelSink::new();
        let model: ModelCell<dyn CaptureViewModel> = ModelCell::new(Box::new(model));
        watch(sink.action, &model, |m, v| m.set_action(v))?;
        model.call(|m| m.attach(sink))?;
        Ok(Self { sink, model })
    }
}

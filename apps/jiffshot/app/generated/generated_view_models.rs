//! Generated from view-model.ir.json. Do not edit.

#![allow(dead_code)]

#[derive(Clone, Debug)]
pub struct MainViewModelSink(crate::view_model::ViewModelSink);

/// Declared capacity of the `MainViewModel` recent-file list.
pub const MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY: usize = 10;

impl MainViewModelSink {
    pub fn set_title(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_tool(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_canvas_api(&self, value: i64) -> crate::Result<()> { self.0.set_integer(4, value) }
    pub fn set_frame_id(&self, value: i64) -> crate::Result<()> { self.0.set_integer(5, value) }
    pub fn set_stroke_color(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(6, value) }
    pub fn set_fill_color(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(7, value) }
    pub fn set_text_color(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(8, value) }
    pub fn set_stroke_width(&self, value: f64) -> crate::Result<()> { self.0.set_double(9, value) }
    pub fn set_font_size(&self, value: f64) -> crate::Result<()> { self.0.set_double(10, value) }
    pub fn set_font_family(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(11, value) }
    pub fn set_bold(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(12, value) }
    pub fn set_italic(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(13, value) }
    pub fn set_shadow(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(14, value) }
    pub fn set_opacity(&self, value: f64) -> crate::Result<()> { self.0.set_double(15, value) }
    pub fn set_text(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(16, value) }
    pub fn set_variant(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(17, value) }
    pub fn set_amount(&self, value: f64) -> crate::Result<()> { self.0.set_double(18, value) }
    pub fn set_align(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(19, value) }
    pub fn set_dash(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(20, value) }
    pub fn set_start_cap(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(21, value) }
    pub fn set_end_cap(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(22, value) }
    pub fn set_selection(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(23, value) }
    pub fn set_panel(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(24, value) }
    pub fn set_amount_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(25, value) }
    pub fn set_focus_text(&self, value: i64) -> crate::Result<()> { self.0.set_integer(26, value) }
    pub fn set_zoom_request(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(27, value) }
    pub fn set_action(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(28, value) }
    pub fn set_mode(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(29, value) }
    pub fn set_video_position(&self, value: f64) -> crate::Result<()> { self.0.set_double(30, value) }
    pub fn set_video_duration(&self, value: f64) -> crate::Result<()> { self.0.set_double(31, value) }
    pub fn set_video_info(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(32, value) }
    pub fn set_trim_start(&self, value: f64) -> crate::Result<()> { self.0.set_double(33, value) }
    pub fn set_trim_end(&self, value: f64) -> crate::Result<()> { self.0.set_double(34, value) }
    pub fn set_playing(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(35, value) }
    pub fn set_document_info(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(36, value) }
    pub fn set_rotation(&self, value: f64) -> crate::Result<()> { self.0.set_double(37, value) }
    pub fn set_library_info(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(38, value) }
    pub fn set_background_fill(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(39, value) }
    pub fn set_selection_count(&self, value: i64) -> crate::Result<()> { self.0.set_integer(40, value) }
    pub fn set_head_size(&self, value: f64) -> crate::Result<()> { self.0.set_double(41, value) }
    pub fn set_shadow_angle(&self, value: f64) -> crate::Result<()> { self.0.set_double(42, value) }
    pub fn set_shadow_distance(&self, value: f64) -> crate::Result<()> { self.0.set_double(43, value) }
    pub fn set_shadow_blur(&self, value: f64) -> crate::Result<()> { self.0.set_double(44, value) }
    pub fn set_shadow_color(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(45, value) }
    pub fn set_outline_color(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(46, value) }
    pub fn set_outline_width(&self, value: f64) -> crate::Result<()> { self.0.set_double(47, value) }
    pub fn set_underline(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(48, value) }
    pub fn set_strike(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(49, value) }
    pub fn set_v_align(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(50, value) }
    pub fn set_padding(&self, value: f64) -> crate::Result<()> { self.0.set_double(51, value) }
    pub fn set_fill2(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(52, value) }
    pub fn set_gradient_radial(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(53, value) }
    pub fn set_gradient_angle(&self, value: f64) -> crate::Result<()> { self.0.set_double(54, value) }
    pub fn set_canvas_color(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(55, value) }
    pub fn set_theme(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(56, value) }
    pub fn set_gif_preset(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(57, value) }
    pub fn set_gif_loop(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(58, value) }
    pub fn set_gif_fade(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(59, value) }
    pub fn set_gif_dither(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(60, value) }
    pub fn set_video_volume(&self, value: f64) -> crate::Result<()> { self.0.set_double(61, value) }
    pub fn set_muted(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(62, value) }
    pub fn set_library_search(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(63, value) }
    pub fn set_library_filter(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(64, value) }
    pub fn set_library_sort(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(65, value) }
    pub fn set_thumb_size(&self, value: f64) -> crate::Result<()> { self.0.set_double(66, value) }
    pub fn set_library_expanded(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(67, value) }
    pub fn set_library_selected(&self, value: i64) -> crate::Result<()> { self.0.set_integer(68, value) }
    pub fn set_template_theme(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(69, value) }
    pub fn set_template_captions(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(70, value) }
    pub fn set_template_numbers(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(71, value) }
    pub fn set_template_footer(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(72, value) }
    pub fn set_zone_fit(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(73, value) }
    pub fn set_tool_order(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(74, value) }
    pub fn set_narrating(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(75, value) }
    pub fn set_show_side_bar(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(76, value) }
    pub fn set_show_effects(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(77, value) }
    pub fn set_show_tray(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(78, value) }
    pub fn set_tool_labels(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(79, value) }
    pub fn set_toast(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(80, value) }
    pub fn set_grab_text(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(81, value) }
    pub fn set_widget(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(82, value) }
    pub fn set_hud(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(83, value) }
    pub fn set_ui_api(&self, value: i64) -> crate::Result<()> { self.0.set_integer(84, value) }
    pub fn set_overlay(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(85, value) }
    pub fn set_screen_draw(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(86, value) }
    pub fn set_prefs(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(87, value) }
    pub fn add_variants(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(1, value) }
    pub fn insert_variants(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(1, index, value) }
    pub fn replace_variants(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(1, index, value) }
    pub fn add_fonts(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(2, value) }
    pub fn insert_fonts(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(2, index, value) }
    pub fn replace_fonts(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(2, index, value) }
    pub fn add_effects(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(5, value) }
    pub fn insert_effects(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(5, index, value) }
    pub fn replace_effects(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(5, index, value) }
    pub fn add_templates(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(6, value) }
    pub fn insert_templates(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(6, index, value) }
    pub fn replace_templates(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(6, index, value) }
    pub fn add_shares(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(7, value) }
    pub fn insert_shares(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(7, index, value) }
    pub fn replace_shares(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(7, index, value) }
    pub fn add_library_filters(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(8, value) }
    pub fn insert_library_filters(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(8, index, value) }
    pub fn replace_library_filters(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(8, index, value) }
    pub fn add_library_sorts(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(9, value) }
    pub fn insert_library_sorts(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(9, index, value) }
    pub fn replace_library_sorts(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(9, index, value) }
    pub fn add_template_themes(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(10, value) }
    pub fn insert_template_themes(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(10, index, value) }
    pub fn replace_template_themes(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(10, index, value) }
    pub fn add_recent_files(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(11, value) }
    pub fn insert_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(11, index, value) }
    pub fn replace_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(11, index, value) }
    pub fn add_library(&self, value: impl LibraryItemViewModel) -> crate::Result<()> { self.0.add_model(3, LibraryItemViewModelDispatch { model: value }) }
    pub fn insert_library(&self, index: i32, value: impl LibraryItemViewModel) -> crate::Result<()> { self.0.insert_model(3, index, LibraryItemViewModelDispatch { model: value }) }
    pub fn replace_library(&self, index: i32, value: impl LibraryItemViewModel) -> crate::Result<()> { self.0.replace_model(3, index, LibraryItemViewModelDispatch { model: value }) }
    pub fn add_quick_styles(&self, value: impl QuickStyleViewModel) -> crate::Result<()> { self.0.add_model(4, QuickStyleViewModelDispatch { model: value }) }
    pub fn insert_quick_styles(&self, index: i32, value: impl QuickStyleViewModel) -> crate::Result<()> { self.0.insert_model(4, index, QuickStyleViewModelDispatch { model: value }) }
    pub fn replace_quick_styles(&self, index: i32, value: impl QuickStyleViewModel) -> crate::Result<()> { self.0.replace_model(4, index, QuickStyleViewModelDispatch { model: value }) }
    pub fn remove_variants(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(1, index) }
    pub fn move_variants(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(1, from_index, to_index) }
    pub fn clear_variants(&self) -> crate::Result<()> { self.0.clear_string_collection(1) }
    pub fn remove_fonts(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(2, index) }
    pub fn move_fonts(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(2, from_index, to_index) }
    pub fn clear_fonts(&self) -> crate::Result<()> { self.0.clear_string_collection(2) }
    pub fn remove_effects(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(5, index) }
    pub fn move_effects(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(5, from_index, to_index) }
    pub fn clear_effects(&self) -> crate::Result<()> { self.0.clear_string_collection(5) }
    pub fn remove_templates(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(6, index) }
    pub fn move_templates(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(6, from_index, to_index) }
    pub fn clear_templates(&self) -> crate::Result<()> { self.0.clear_string_collection(6) }
    pub fn remove_shares(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(7, index) }
    pub fn move_shares(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(7, from_index, to_index) }
    pub fn clear_shares(&self) -> crate::Result<()> { self.0.clear_string_collection(7) }
    pub fn remove_library_filters(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(8, index) }
    pub fn move_library_filters(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(8, from_index, to_index) }
    pub fn clear_library_filters(&self) -> crate::Result<()> { self.0.clear_string_collection(8) }
    pub fn remove_library_sorts(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(9, index) }
    pub fn move_library_sorts(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(9, from_index, to_index) }
    pub fn clear_library_sorts(&self) -> crate::Result<()> { self.0.clear_string_collection(9) }
    pub fn remove_template_themes(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(10, index) }
    pub fn move_template_themes(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(10, from_index, to_index) }
    pub fn clear_template_themes(&self) -> crate::Result<()> { self.0.clear_string_collection(10) }
    pub fn remove_recent_files(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(11, index) }
    pub fn move_recent_files(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(11, from_index, to_index) }
    pub fn clear_recent_files(&self) -> crate::Result<()> { self.0.clear_string_collection(11) }
    pub fn remove_library(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(3, index) }
    pub fn move_library(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(3, from_index, to_index) }
    pub fn clear_library(&self) -> crate::Result<()> { self.0.clear_model_collection(3) }
    pub fn remove_quick_styles(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(4, index) }
    pub fn move_quick_styles(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(4, from_index, to_index) }
    pub fn clear_quick_styles(&self) -> crate::Result<()> { self.0.clear_model_collection(4) }
    pub fn set_new_image_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_open_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_save_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_save_as_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(4, enabled) }
    pub fn set_export_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(5, enabled) }
    pub fn set_copy_image_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(6, enabled) }
    pub fn set_paste_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(7, enabled) }
    pub fn set_new_capture_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(8, enabled) }
    pub fn set_combine_images_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(9, enabled) }
    pub fn set_open_library_folder_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(10, enabled) }
    pub fn set_exit_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(11, enabled) }
    pub fn set_undo_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(12, enabled) }
    pub fn set_redo_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(13, enabled) }
    pub fn set_copy_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(14, enabled) }
    pub fn set_cut_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(15, enabled) }
    pub fn set_delete_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(16, enabled) }
    pub fn set_duplicate_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(17, enabled) }
    pub fn set_bring_to_front_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(18, enabled) }
    pub fn set_bring_forward_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(19, enabled) }
    pub fn set_send_backward_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(20, enabled) }
    pub fn set_send_to_back_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(21, enabled) }
    pub fn set_save_quick_style_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(22, enabled) }
    pub fn set_rotate_left_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(23, enabled) }
    pub fn set_rotate_right_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(24, enabled) }
    pub fn set_flip_horizontal_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(25, enabled) }
    pub fn set_flip_vertical_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(26, enabled) }
    pub fn set_resize50_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(27, enabled) }
    pub fn set_resize200_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(28, enabled) }
    pub fn set_flatten_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(29, enabled) }
    pub fn set_zoom_in_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(30, enabled) }
    pub fn set_zoom_out_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(31, enabled) }
    pub fn set_zoom_fit_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(32, enabled) }
    pub fn set_zoom_actual_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(33, enabled) }
    pub fn set_grab_text_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(34, enabled) }
    pub fn set_simplify_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(35, enabled) }
    pub fn set_play_pause_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(36, enabled) }
    pub fn set_set_trim_start_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(37, enabled) }
    pub fn set_set_trim_end_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(38, enabled) }
    pub fn set_export_video_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(39, enabled) }
    pub fn set_export_gif_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(40, enabled) }
    pub fn set_frame_to_image_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(41, enabled) }
    pub fn set_record_video_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(42, enabled) }
    pub fn set_canvas_changed_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(43, enabled) }
    pub fn set_select_all_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(44, enabled) }
    pub fn set_deselect_all_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(45, enabled) }
    pub fn set_invert_selection_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(46, enabled) }
    pub fn set_group_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(47, enabled) }
    pub fn set_ungroup_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(48, enabled) }
    pub fn set_flatten_selected_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(49, enabled) }
    pub fn set_smart_redact_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(50, enabled) }
    pub fn set_new_from_clipboard_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(51, enabled) }
    pub fn set_print_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(52, enabled) }
    pub fn set_close_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(53, enabled) }
    pub fn set_duplicate_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(54, enabled) }
    pub fn set_delete_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(55, enabled) }
    pub fn set_tool_magic_wand_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(56, enabled) }
    pub fn set_tool_eraser_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(57, enabled) }
    pub fn set_expand_canvas_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(58, enabled) }
    pub fn set_cycle_theme_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(59, enabled) }
    pub fn set_prev_frame_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(60, enabled) }
    pub fn set_next_frame_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(61, enabled) }
    pub fn set_cut_selection_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(62, enabled) }
    pub fn set_silent_copy_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(63, enabled) }
    pub fn set_combine_videos_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(64, enabled) }
    pub fn set_video_from_images_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(65, enabled) }
    pub fn set_toggle_mute_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(66, enabled) }
    pub fn set_toggle_library_view_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(67, enabled) }
    pub fn set_combine_selected_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(68, enabled) }
    pub fn set_clear_library_selection_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(69, enabled) }
    pub fn set_show_closed_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(70, enabled) }
    pub fn set_reset_toolbar_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(71, enabled) }
    pub fn set_narrate_images_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(72, enabled) }
    pub fn set_narrate_next_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(73, enabled) }
    pub fn set_narrate_finish_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(74, enabled) }
    pub fn set_toggle_side_bar_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(75, enabled) }
    pub fn set_toggle_effects_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(76, enabled) }
    pub fn set_toggle_tray_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(77, enabled) }
    pub fn set_toggle_tool_labels_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(78, enabled) }
    pub fn set_open_capture_window_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(79, enabled) }
    pub fn set_open_recent_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(80, enabled) }
    pub fn set_auto_trim_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(81, enabled) }
    pub fn set_tool_crop_menu_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(82, enabled) }
    pub fn set_tool_select_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(83, enabled) }
    pub fn set_tool_arrow_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(84, enabled) }
    pub fn set_tool_line_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(85, enabled) }
    pub fn set_tool_shape_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(86, enabled) }
    pub fn set_tool_text_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(87, enabled) }
    pub fn set_tool_callout_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(88, enabled) }
    pub fn set_tool_step_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(89, enabled) }
    pub fn set_tool_pen_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(90, enabled) }
    pub fn set_tool_highlight_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(91, enabled) }
    pub fn set_tool_blur_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(92, enabled) }
    pub fn set_tool_magnify_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(93, enabled) }
    pub fn set_tool_spotlight_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(94, enabled) }
    pub fn set_tool_stamp_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(95, enabled) }
    pub fn set_tool_fill_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(96, enabled) }
    pub fn set_tool_crop_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(97, enabled) }
    pub fn set_tool_cut_out_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(98, enabled) }
    pub fn set_tool_selection_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(99, enabled) }
    pub fn set_tool_eyedropper_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(100, enabled) }
    pub fn set_tool_smart_move_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(101, enabled) }
    pub fn set_tool_text_replace_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(102, enabled) }
    pub fn set_run_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(103, enabled) }
    /// Publishes the most-recently-used storage URIs into `RecentFiles`.
    ///
    /// The list is bounded by its own capacity (10), so this replaces a
    /// handful of entries rather than a data set; the generated menu derives
    /// each header from the URI and passes the URI back as the command parameter.
    pub fn publish_recent_files(&self, recent: &crate::RecentFileList) -> crate::Result<()> {
        self.0.clear_string_collection(11)?;
        for uri in recent.entries() { self.0.add_string(11, uri)?; }
        Ok(())
    }
    pub fn set_title_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_tool_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_canvas_api_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_frame_id_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_stroke_color_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_fill_color_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_text_color_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_stroke_width_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_font_size_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    pub fn set_font_family_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(11, message) }
    pub fn set_bold_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(12, message) }
    pub fn set_italic_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(13, message) }
    pub fn set_shadow_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(14, message) }
    pub fn set_opacity_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(15, message) }
    pub fn set_text_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(16, message) }
    pub fn set_variant_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(17, message) }
    pub fn set_amount_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(18, message) }
    pub fn set_align_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(19, message) }
    pub fn set_dash_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(20, message) }
    pub fn set_start_cap_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(21, message) }
    pub fn set_end_cap_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(22, message) }
    pub fn set_selection_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(23, message) }
    pub fn set_panel_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(24, message) }
    pub fn set_amount_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(25, message) }
    pub fn set_focus_text_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(26, message) }
    pub fn set_zoom_request_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(27, message) }
    pub fn set_action_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(28, message) }
    pub fn set_mode_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(29, message) }
    pub fn set_video_position_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(30, message) }
    pub fn set_video_duration_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(31, message) }
    pub fn set_video_info_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(32, message) }
    pub fn set_trim_start_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(33, message) }
    pub fn set_trim_end_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(34, message) }
    pub fn set_playing_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(35, message) }
    pub fn set_document_info_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(36, message) }
    pub fn set_rotation_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(37, message) }
    pub fn set_library_info_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(38, message) }
    pub fn set_background_fill_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(39, message) }
    pub fn set_selection_count_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(40, message) }
    pub fn set_head_size_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(41, message) }
    pub fn set_shadow_angle_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(42, message) }
    pub fn set_shadow_distance_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(43, message) }
    pub fn set_shadow_blur_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(44, message) }
    pub fn set_shadow_color_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(45, message) }
    pub fn set_outline_color_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(46, message) }
    pub fn set_outline_width_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(47, message) }
    pub fn set_underline_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(48, message) }
    pub fn set_strike_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(49, message) }
    pub fn set_v_align_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(50, message) }
    pub fn set_padding_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(51, message) }
    pub fn set_fill2_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(52, message) }
    pub fn set_gradient_radial_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(53, message) }
    pub fn set_gradient_angle_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(54, message) }
    pub fn set_canvas_color_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(55, message) }
    pub fn set_theme_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(56, message) }
    pub fn set_gif_preset_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(57, message) }
    pub fn set_gif_loop_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(58, message) }
    pub fn set_gif_fade_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(59, message) }
    pub fn set_gif_dither_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(60, message) }
    pub fn set_video_volume_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(61, message) }
    pub fn set_muted_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(62, message) }
    pub fn set_library_search_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(63, message) }
    pub fn set_library_filter_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(64, message) }
    pub fn set_library_sort_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(65, message) }
    pub fn set_thumb_size_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(66, message) }
    pub fn set_library_expanded_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(67, message) }
    pub fn set_library_selected_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(68, message) }
    pub fn set_template_theme_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(69, message) }
    pub fn set_template_captions_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(70, message) }
    pub fn set_template_numbers_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(71, message) }
    pub fn set_template_footer_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(72, message) }
    pub fn set_zone_fit_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(73, message) }
    pub fn set_tool_order_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(74, message) }
    pub fn set_narrating_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(75, message) }
    pub fn set_show_side_bar_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(76, message) }
    pub fn set_show_effects_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(77, message) }
    pub fn set_show_tray_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(78, message) }
    pub fn set_tool_labels_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(79, message) }
    pub fn set_toast_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(80, message) }
    pub fn set_grab_text_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(81, message) }
    pub fn set_widget_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(82, message) }
    pub fn set_hud_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(83, message) }
    pub fn set_ui_api_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(84, message) }
    pub fn set_overlay_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(85, message) }
    pub fn set_screen_draw_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(86, message) }
    pub fn set_prefs_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(87, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> MainViewModelSinkBatch { MainViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: MainViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct MainViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl MainViewModelSinkBatch {
    pub fn set_title(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_title_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_title_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_status_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_tool(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_tool_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_tool_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_canvas_api(&mut self, value: i64) { self.0.push_integer(4, value); }
    pub fn set_canvas_api_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_canvas_api_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_frame_id(&mut self, value: i64) { self.0.push_integer(5, value); }
    pub fn set_frame_id_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_frame_id_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_stroke_color(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 6, 0, value); }
    pub fn set_stroke_color_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_stroke_color_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_fill_color(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 7, 0, value); }
    pub fn set_fill_color_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_fill_color_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_text_color(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 8, 0, value); }
    pub fn set_text_color_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_text_color_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_stroke_width(&mut self, value: f64) { self.0.push_double(9, value); }
    pub fn set_stroke_width_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_stroke_width_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_font_size(&mut self, value: f64) { self.0.push_double(10, value); }
    pub fn set_font_size_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_font_size_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_font_family(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 11, 0, value); }
    pub fn set_font_family_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 11, 0, message); }
    pub fn clear_font_family_error(&mut self) { self.0.push_clear_error(11); }
    pub fn set_bold(&mut self, value: bool) { self.0.push_boolean(3, 12, value); }
    pub fn set_bold_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 12, 0, message); }
    pub fn clear_bold_error(&mut self) { self.0.push_clear_error(12); }
    pub fn set_italic(&mut self, value: bool) { self.0.push_boolean(3, 13, value); }
    pub fn set_italic_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 13, 0, message); }
    pub fn clear_italic_error(&mut self) { self.0.push_clear_error(13); }
    pub fn set_shadow(&mut self, value: bool) { self.0.push_boolean(3, 14, value); }
    pub fn set_shadow_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 14, 0, message); }
    pub fn clear_shadow_error(&mut self) { self.0.push_clear_error(14); }
    pub fn set_opacity(&mut self, value: f64) { self.0.push_double(15, value); }
    pub fn set_opacity_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 15, 0, message); }
    pub fn clear_opacity_error(&mut self) { self.0.push_clear_error(15); }
    pub fn set_text(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 16, 0, value); }
    pub fn set_text_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 16, 0, message); }
    pub fn clear_text_error(&mut self) { self.0.push_clear_error(16); }
    pub fn set_variant(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 17, 0, value); }
    pub fn set_variant_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 17, 0, message); }
    pub fn clear_variant_error(&mut self) { self.0.push_clear_error(17); }
    pub fn set_amount(&mut self, value: f64) { self.0.push_double(18, value); }
    pub fn set_amount_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 18, 0, message); }
    pub fn clear_amount_error(&mut self) { self.0.push_clear_error(18); }
    pub fn set_align(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 19, 0, value); }
    pub fn set_align_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 19, 0, message); }
    pub fn clear_align_error(&mut self) { self.0.push_clear_error(19); }
    pub fn set_dash(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 20, 0, value); }
    pub fn set_dash_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 20, 0, message); }
    pub fn clear_dash_error(&mut self) { self.0.push_clear_error(20); }
    pub fn set_start_cap(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 21, 0, value); }
    pub fn set_start_cap_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 21, 0, message); }
    pub fn clear_start_cap_error(&mut self) { self.0.push_clear_error(21); }
    pub fn set_end_cap(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 22, 0, value); }
    pub fn set_end_cap_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 22, 0, message); }
    pub fn clear_end_cap_error(&mut self) { self.0.push_clear_error(22); }
    pub fn set_selection(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 23, 0, value); }
    pub fn set_selection_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 23, 0, message); }
    pub fn clear_selection_error(&mut self) { self.0.push_clear_error(23); }
    pub fn set_panel(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 24, 0, value); }
    pub fn set_panel_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 24, 0, message); }
    pub fn clear_panel_error(&mut self) { self.0.push_clear_error(24); }
    pub fn set_amount_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 25, 0, value); }
    pub fn set_amount_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 25, 0, message); }
    pub fn clear_amount_label_error(&mut self) { self.0.push_clear_error(25); }
    pub fn set_focus_text(&mut self, value: i64) { self.0.push_integer(26, value); }
    pub fn set_focus_text_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 26, 0, message); }
    pub fn clear_focus_text_error(&mut self) { self.0.push_clear_error(26); }
    pub fn set_zoom_request(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 27, 0, value); }
    pub fn set_zoom_request_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 27, 0, message); }
    pub fn clear_zoom_request_error(&mut self) { self.0.push_clear_error(27); }
    pub fn set_action(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 28, 0, value); }
    pub fn set_action_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 28, 0, message); }
    pub fn clear_action_error(&mut self) { self.0.push_clear_error(28); }
    pub fn set_mode(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 29, 0, value); }
    pub fn set_mode_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 29, 0, message); }
    pub fn clear_mode_error(&mut self) { self.0.push_clear_error(29); }
    pub fn set_video_position(&mut self, value: f64) { self.0.push_double(30, value); }
    pub fn set_video_position_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 30, 0, message); }
    pub fn clear_video_position_error(&mut self) { self.0.push_clear_error(30); }
    pub fn set_video_duration(&mut self, value: f64) { self.0.push_double(31, value); }
    pub fn set_video_duration_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 31, 0, message); }
    pub fn clear_video_duration_error(&mut self) { self.0.push_clear_error(31); }
    pub fn set_video_info(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 32, 0, value); }
    pub fn set_video_info_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 32, 0, message); }
    pub fn clear_video_info_error(&mut self) { self.0.push_clear_error(32); }
    pub fn set_trim_start(&mut self, value: f64) { self.0.push_double(33, value); }
    pub fn set_trim_start_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 33, 0, message); }
    pub fn clear_trim_start_error(&mut self) { self.0.push_clear_error(33); }
    pub fn set_trim_end(&mut self, value: f64) { self.0.push_double(34, value); }
    pub fn set_trim_end_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 34, 0, message); }
    pub fn clear_trim_end_error(&mut self) { self.0.push_clear_error(34); }
    pub fn set_playing(&mut self, value: bool) { self.0.push_boolean(3, 35, value); }
    pub fn set_playing_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 35, 0, message); }
    pub fn clear_playing_error(&mut self) { self.0.push_clear_error(35); }
    pub fn set_document_info(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 36, 0, value); }
    pub fn set_document_info_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 36, 0, message); }
    pub fn clear_document_info_error(&mut self) { self.0.push_clear_error(36); }
    pub fn set_rotation(&mut self, value: f64) { self.0.push_double(37, value); }
    pub fn set_rotation_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 37, 0, message); }
    pub fn clear_rotation_error(&mut self) { self.0.push_clear_error(37); }
    pub fn set_library_info(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 38, 0, value); }
    pub fn set_library_info_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 38, 0, message); }
    pub fn clear_library_info_error(&mut self) { self.0.push_clear_error(38); }
    pub fn set_background_fill(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 39, 0, value); }
    pub fn set_background_fill_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 39, 0, message); }
    pub fn clear_background_fill_error(&mut self) { self.0.push_clear_error(39); }
    pub fn set_selection_count(&mut self, value: i64) { self.0.push_integer(40, value); }
    pub fn set_selection_count_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 40, 0, message); }
    pub fn clear_selection_count_error(&mut self) { self.0.push_clear_error(40); }
    pub fn set_head_size(&mut self, value: f64) { self.0.push_double(41, value); }
    pub fn set_head_size_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 41, 0, message); }
    pub fn clear_head_size_error(&mut self) { self.0.push_clear_error(41); }
    pub fn set_shadow_angle(&mut self, value: f64) { self.0.push_double(42, value); }
    pub fn set_shadow_angle_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 42, 0, message); }
    pub fn clear_shadow_angle_error(&mut self) { self.0.push_clear_error(42); }
    pub fn set_shadow_distance(&mut self, value: f64) { self.0.push_double(43, value); }
    pub fn set_shadow_distance_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 43, 0, message); }
    pub fn clear_shadow_distance_error(&mut self) { self.0.push_clear_error(43); }
    pub fn set_shadow_blur(&mut self, value: f64) { self.0.push_double(44, value); }
    pub fn set_shadow_blur_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 44, 0, message); }
    pub fn clear_shadow_blur_error(&mut self) { self.0.push_clear_error(44); }
    pub fn set_shadow_color(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 45, 0, value); }
    pub fn set_shadow_color_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 45, 0, message); }
    pub fn clear_shadow_color_error(&mut self) { self.0.push_clear_error(45); }
    pub fn set_outline_color(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 46, 0, value); }
    pub fn set_outline_color_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 46, 0, message); }
    pub fn clear_outline_color_error(&mut self) { self.0.push_clear_error(46); }
    pub fn set_outline_width(&mut self, value: f64) { self.0.push_double(47, value); }
    pub fn set_outline_width_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 47, 0, message); }
    pub fn clear_outline_width_error(&mut self) { self.0.push_clear_error(47); }
    pub fn set_underline(&mut self, value: bool) { self.0.push_boolean(3, 48, value); }
    pub fn set_underline_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 48, 0, message); }
    pub fn clear_underline_error(&mut self) { self.0.push_clear_error(48); }
    pub fn set_strike(&mut self, value: bool) { self.0.push_boolean(3, 49, value); }
    pub fn set_strike_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 49, 0, message); }
    pub fn clear_strike_error(&mut self) { self.0.push_clear_error(49); }
    pub fn set_v_align(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 50, 0, value); }
    pub fn set_v_align_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 50, 0, message); }
    pub fn clear_v_align_error(&mut self) { self.0.push_clear_error(50); }
    pub fn set_padding(&mut self, value: f64) { self.0.push_double(51, value); }
    pub fn set_padding_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 51, 0, message); }
    pub fn clear_padding_error(&mut self) { self.0.push_clear_error(51); }
    pub fn set_fill2(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 52, 0, value); }
    pub fn set_fill2_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 52, 0, message); }
    pub fn clear_fill2_error(&mut self) { self.0.push_clear_error(52); }
    pub fn set_gradient_radial(&mut self, value: bool) { self.0.push_boolean(3, 53, value); }
    pub fn set_gradient_radial_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 53, 0, message); }
    pub fn clear_gradient_radial_error(&mut self) { self.0.push_clear_error(53); }
    pub fn set_gradient_angle(&mut self, value: f64) { self.0.push_double(54, value); }
    pub fn set_gradient_angle_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 54, 0, message); }
    pub fn clear_gradient_angle_error(&mut self) { self.0.push_clear_error(54); }
    pub fn set_canvas_color(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 55, 0, value); }
    pub fn set_canvas_color_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 55, 0, message); }
    pub fn clear_canvas_color_error(&mut self) { self.0.push_clear_error(55); }
    pub fn set_theme(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 56, 0, value); }
    pub fn set_theme_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 56, 0, message); }
    pub fn clear_theme_error(&mut self) { self.0.push_clear_error(56); }
    pub fn set_gif_preset(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 57, 0, value); }
    pub fn set_gif_preset_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 57, 0, message); }
    pub fn clear_gif_preset_error(&mut self) { self.0.push_clear_error(57); }
    pub fn set_gif_loop(&mut self, value: bool) { self.0.push_boolean(3, 58, value); }
    pub fn set_gif_loop_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 58, 0, message); }
    pub fn clear_gif_loop_error(&mut self) { self.0.push_clear_error(58); }
    pub fn set_gif_fade(&mut self, value: bool) { self.0.push_boolean(3, 59, value); }
    pub fn set_gif_fade_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 59, 0, message); }
    pub fn clear_gif_fade_error(&mut self) { self.0.push_clear_error(59); }
    pub fn set_gif_dither(&mut self, value: bool) { self.0.push_boolean(3, 60, value); }
    pub fn set_gif_dither_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 60, 0, message); }
    pub fn clear_gif_dither_error(&mut self) { self.0.push_clear_error(60); }
    pub fn set_video_volume(&mut self, value: f64) { self.0.push_double(61, value); }
    pub fn set_video_volume_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 61, 0, message); }
    pub fn clear_video_volume_error(&mut self) { self.0.push_clear_error(61); }
    pub fn set_muted(&mut self, value: bool) { self.0.push_boolean(3, 62, value); }
    pub fn set_muted_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 62, 0, message); }
    pub fn clear_muted_error(&mut self) { self.0.push_clear_error(62); }
    pub fn set_library_search(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 63, 0, value); }
    pub fn set_library_search_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 63, 0, message); }
    pub fn clear_library_search_error(&mut self) { self.0.push_clear_error(63); }
    pub fn set_library_filter(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 64, 0, value); }
    pub fn set_library_filter_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 64, 0, message); }
    pub fn clear_library_filter_error(&mut self) { self.0.push_clear_error(64); }
    pub fn set_library_sort(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 65, 0, value); }
    pub fn set_library_sort_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 65, 0, message); }
    pub fn clear_library_sort_error(&mut self) { self.0.push_clear_error(65); }
    pub fn set_thumb_size(&mut self, value: f64) { self.0.push_double(66, value); }
    pub fn set_thumb_size_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 66, 0, message); }
    pub fn clear_thumb_size_error(&mut self) { self.0.push_clear_error(66); }
    pub fn set_library_expanded(&mut self, value: bool) { self.0.push_boolean(3, 67, value); }
    pub fn set_library_expanded_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 67, 0, message); }
    pub fn clear_library_expanded_error(&mut self) { self.0.push_clear_error(67); }
    pub fn set_library_selected(&mut self, value: i64) { self.0.push_integer(68, value); }
    pub fn set_library_selected_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 68, 0, message); }
    pub fn clear_library_selected_error(&mut self) { self.0.push_clear_error(68); }
    pub fn set_template_theme(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 69, 0, value); }
    pub fn set_template_theme_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 69, 0, message); }
    pub fn clear_template_theme_error(&mut self) { self.0.push_clear_error(69); }
    pub fn set_template_captions(&mut self, value: bool) { self.0.push_boolean(3, 70, value); }
    pub fn set_template_captions_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 70, 0, message); }
    pub fn clear_template_captions_error(&mut self) { self.0.push_clear_error(70); }
    pub fn set_template_numbers(&mut self, value: bool) { self.0.push_boolean(3, 71, value); }
    pub fn set_template_numbers_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 71, 0, message); }
    pub fn clear_template_numbers_error(&mut self) { self.0.push_clear_error(71); }
    pub fn set_template_footer(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 72, 0, value); }
    pub fn set_template_footer_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 72, 0, message); }
    pub fn clear_template_footer_error(&mut self) { self.0.push_clear_error(72); }
    pub fn set_zone_fit(&mut self, value: bool) { self.0.push_boolean(3, 73, value); }
    pub fn set_zone_fit_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 73, 0, message); }
    pub fn clear_zone_fit_error(&mut self) { self.0.push_clear_error(73); }
    pub fn set_tool_order(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 74, 0, value); }
    pub fn set_tool_order_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 74, 0, message); }
    pub fn clear_tool_order_error(&mut self) { self.0.push_clear_error(74); }
    pub fn set_narrating(&mut self, value: bool) { self.0.push_boolean(3, 75, value); }
    pub fn set_narrating_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 75, 0, message); }
    pub fn clear_narrating_error(&mut self) { self.0.push_clear_error(75); }
    pub fn set_show_side_bar(&mut self, value: bool) { self.0.push_boolean(3, 76, value); }
    pub fn set_show_side_bar_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 76, 0, message); }
    pub fn clear_show_side_bar_error(&mut self) { self.0.push_clear_error(76); }
    pub fn set_show_effects(&mut self, value: bool) { self.0.push_boolean(3, 77, value); }
    pub fn set_show_effects_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 77, 0, message); }
    pub fn clear_show_effects_error(&mut self) { self.0.push_clear_error(77); }
    pub fn set_show_tray(&mut self, value: bool) { self.0.push_boolean(3, 78, value); }
    pub fn set_show_tray_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 78, 0, message); }
    pub fn clear_show_tray_error(&mut self) { self.0.push_clear_error(78); }
    pub fn set_tool_labels(&mut self, value: bool) { self.0.push_boolean(3, 79, value); }
    pub fn set_tool_labels_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 79, 0, message); }
    pub fn clear_tool_labels_error(&mut self) { self.0.push_clear_error(79); }
    pub fn set_toast(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 80, 0, value); }
    pub fn set_toast_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 80, 0, message); }
    pub fn clear_toast_error(&mut self) { self.0.push_clear_error(80); }
    pub fn set_grab_text(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 81, 0, value); }
    pub fn set_grab_text_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 81, 0, message); }
    pub fn clear_grab_text_error(&mut self) { self.0.push_clear_error(81); }
    pub fn set_widget(&mut self, value: bool) { self.0.push_boolean(3, 82, value); }
    pub fn set_widget_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 82, 0, message); }
    pub fn clear_widget_error(&mut self) { self.0.push_clear_error(82); }
    pub fn set_hud(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 83, 0, value); }
    pub fn set_hud_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 83, 0, message); }
    pub fn clear_hud_error(&mut self) { self.0.push_clear_error(83); }
    pub fn set_ui_api(&mut self, value: i64) { self.0.push_integer(84, value); }
    pub fn set_ui_api_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 84, 0, message); }
    pub fn clear_ui_api_error(&mut self) { self.0.push_clear_error(84); }
    pub fn set_overlay(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 85, 0, value); }
    pub fn set_overlay_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 85, 0, message); }
    pub fn clear_overlay_error(&mut self) { self.0.push_clear_error(85); }
    pub fn set_screen_draw(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 86, 0, value); }
    pub fn set_screen_draw_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 86, 0, message); }
    pub fn clear_screen_draw_error(&mut self) { self.0.push_clear_error(86); }
    pub fn set_prefs(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 87, 0, value); }
    pub fn set_prefs_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 87, 0, message); }
    pub fn clear_prefs_error(&mut self) { self.0.push_clear_error(87); }
    pub fn add_variants(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 1, 0, value); }
    pub fn insert_variants(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 1, index, value); }
    pub fn replace_variants(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 1, index, value); }
    pub fn replace_variants_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(1, values); }
    pub fn remove_variants(&mut self, index: i32) { self.0.push_indices(13, 1, index, 0); }
    pub fn move_variants(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 1, from_index, to_index); }
    pub fn clear_variants(&mut self) { self.0.push_indices(19, 1, 0, 0); }
    pub fn add_fonts(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 2, 0, value); }
    pub fn insert_fonts(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 2, index, value); }
    pub fn replace_fonts(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 2, index, value); }
    pub fn replace_fonts_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(2, values); }
    pub fn remove_fonts(&mut self, index: i32) { self.0.push_indices(13, 2, index, 0); }
    pub fn move_fonts(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 2, from_index, to_index); }
    pub fn clear_fonts(&mut self) { self.0.push_indices(19, 2, 0, 0); }
    pub fn add_effects(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 5, 0, value); }
    pub fn insert_effects(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 5, index, value); }
    pub fn replace_effects(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 5, index, value); }
    pub fn replace_effects_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(5, values); }
    pub fn remove_effects(&mut self, index: i32) { self.0.push_indices(13, 5, index, 0); }
    pub fn move_effects(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 5, from_index, to_index); }
    pub fn clear_effects(&mut self) { self.0.push_indices(19, 5, 0, 0); }
    pub fn add_templates(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 6, 0, value); }
    pub fn insert_templates(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 6, index, value); }
    pub fn replace_templates(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 6, index, value); }
    pub fn replace_templates_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(6, values); }
    pub fn remove_templates(&mut self, index: i32) { self.0.push_indices(13, 6, index, 0); }
    pub fn move_templates(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 6, from_index, to_index); }
    pub fn clear_templates(&mut self) { self.0.push_indices(19, 6, 0, 0); }
    pub fn add_shares(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 7, 0, value); }
    pub fn insert_shares(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 7, index, value); }
    pub fn replace_shares(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 7, index, value); }
    pub fn replace_shares_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(7, values); }
    pub fn remove_shares(&mut self, index: i32) { self.0.push_indices(13, 7, index, 0); }
    pub fn move_shares(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 7, from_index, to_index); }
    pub fn clear_shares(&mut self) { self.0.push_indices(19, 7, 0, 0); }
    pub fn add_library_filters(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 8, 0, value); }
    pub fn insert_library_filters(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 8, index, value); }
    pub fn replace_library_filters(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 8, index, value); }
    pub fn replace_library_filters_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(8, values); }
    pub fn remove_library_filters(&mut self, index: i32) { self.0.push_indices(13, 8, index, 0); }
    pub fn move_library_filters(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 8, from_index, to_index); }
    pub fn clear_library_filters(&mut self) { self.0.push_indices(19, 8, 0, 0); }
    pub fn add_library_sorts(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 9, 0, value); }
    pub fn insert_library_sorts(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 9, index, value); }
    pub fn replace_library_sorts(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 9, index, value); }
    pub fn replace_library_sorts_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(9, values); }
    pub fn remove_library_sorts(&mut self, index: i32) { self.0.push_indices(13, 9, index, 0); }
    pub fn move_library_sorts(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 9, from_index, to_index); }
    pub fn clear_library_sorts(&mut self) { self.0.push_indices(19, 9, 0, 0); }
    pub fn add_template_themes(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 10, 0, value); }
    pub fn insert_template_themes(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 10, index, value); }
    pub fn replace_template_themes(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 10, index, value); }
    pub fn replace_template_themes_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(10, values); }
    pub fn remove_template_themes(&mut self, index: i32) { self.0.push_indices(13, 10, index, 0); }
    pub fn move_template_themes(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 10, from_index, to_index); }
    pub fn clear_template_themes(&mut self) { self.0.push_indices(19, 10, 0, 0); }
    pub fn add_recent_files(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 11, 0, value); }
    pub fn insert_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 11, index, value); }
    pub fn replace_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 11, index, value); }
    pub fn replace_recent_files_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(11, values); }
    pub fn remove_recent_files(&mut self, index: i32) { self.0.push_indices(13, 11, index, 0); }
    pub fn move_recent_files(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 11, from_index, to_index); }
    pub fn clear_recent_files(&mut self) { self.0.push_indices(19, 11, 0, 0); }
    pub fn add_library(&mut self, value: impl LibraryItemViewModel) { self.0.push_model(8, 3, 0, LibraryItemViewModelDispatch { model: value }); }
    pub fn insert_library(&mut self, index: i32, value: impl LibraryItemViewModel) { self.0.push_model(10, 3, index, LibraryItemViewModelDispatch { model: value }); }
    pub fn replace_library(&mut self, index: i32, value: impl LibraryItemViewModel) { self.0.push_model(12, 3, index, LibraryItemViewModelDispatch { model: value }); }
    pub fn replace_library_snapshot<M: LibraryItemViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(3, values.into_iter().map(|value| LibraryItemViewModelDispatch { model: value })); }
    pub fn remove_library(&mut self, index: i32) { self.0.push_model_indices(13, 3, index, 0); }
    pub fn move_library(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 3, from_index, to_index); }
    pub fn clear_library(&mut self) { self.0.push_model_clear(3); }
    pub fn add_quick_styles(&mut self, value: impl QuickStyleViewModel) { self.0.push_model(8, 4, 0, QuickStyleViewModelDispatch { model: value }); }
    pub fn insert_quick_styles(&mut self, index: i32, value: impl QuickStyleViewModel) { self.0.push_model(10, 4, index, QuickStyleViewModelDispatch { model: value }); }
    pub fn replace_quick_styles(&mut self, index: i32, value: impl QuickStyleViewModel) { self.0.push_model(12, 4, index, QuickStyleViewModelDispatch { model: value }); }
    pub fn replace_quick_styles_snapshot<M: QuickStyleViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(4, values.into_iter().map(|value| QuickStyleViewModelDispatch { model: value })); }
    pub fn remove_quick_styles(&mut self, index: i32) { self.0.push_model_indices(13, 4, index, 0); }
    pub fn move_quick_styles(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 4, from_index, to_index); }
    pub fn clear_quick_styles(&mut self) { self.0.push_model_clear(4); }
    pub fn set_new_image_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_open_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_save_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
    pub fn set_save_as_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 4, enabled); }
    pub fn set_export_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 5, enabled); }
    pub fn set_copy_image_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 6, enabled); }
    pub fn set_paste_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 7, enabled); }
    pub fn set_new_capture_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 8, enabled); }
    pub fn set_combine_images_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 9, enabled); }
    pub fn set_open_library_folder_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 10, enabled); }
    pub fn set_exit_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 11, enabled); }
    pub fn set_undo_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 12, enabled); }
    pub fn set_redo_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 13, enabled); }
    pub fn set_copy_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 14, enabled); }
    pub fn set_cut_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 15, enabled); }
    pub fn set_delete_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 16, enabled); }
    pub fn set_duplicate_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 17, enabled); }
    pub fn set_bring_to_front_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 18, enabled); }
    pub fn set_bring_forward_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 19, enabled); }
    pub fn set_send_backward_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 20, enabled); }
    pub fn set_send_to_back_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 21, enabled); }
    pub fn set_save_quick_style_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 22, enabled); }
    pub fn set_rotate_left_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 23, enabled); }
    pub fn set_rotate_right_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 24, enabled); }
    pub fn set_flip_horizontal_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 25, enabled); }
    pub fn set_flip_vertical_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 26, enabled); }
    pub fn set_resize50_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 27, enabled); }
    pub fn set_resize200_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 28, enabled); }
    pub fn set_flatten_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 29, enabled); }
    pub fn set_zoom_in_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 30, enabled); }
    pub fn set_zoom_out_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 31, enabled); }
    pub fn set_zoom_fit_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 32, enabled); }
    pub fn set_zoom_actual_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 33, enabled); }
    pub fn set_grab_text_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 34, enabled); }
    pub fn set_simplify_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 35, enabled); }
    pub fn set_play_pause_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 36, enabled); }
    pub fn set_set_trim_start_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 37, enabled); }
    pub fn set_set_trim_end_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 38, enabled); }
    pub fn set_export_video_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 39, enabled); }
    pub fn set_export_gif_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 40, enabled); }
    pub fn set_frame_to_image_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 41, enabled); }
    pub fn set_record_video_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 42, enabled); }
    pub fn set_canvas_changed_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 43, enabled); }
    pub fn set_select_all_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 44, enabled); }
    pub fn set_deselect_all_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 45, enabled); }
    pub fn set_invert_selection_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 46, enabled); }
    pub fn set_group_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 47, enabled); }
    pub fn set_ungroup_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 48, enabled); }
    pub fn set_flatten_selected_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 49, enabled); }
    pub fn set_smart_redact_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 50, enabled); }
    pub fn set_new_from_clipboard_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 51, enabled); }
    pub fn set_print_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 52, enabled); }
    pub fn set_close_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 53, enabled); }
    pub fn set_duplicate_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 54, enabled); }
    pub fn set_delete_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 55, enabled); }
    pub fn set_tool_magic_wand_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 56, enabled); }
    pub fn set_tool_eraser_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 57, enabled); }
    pub fn set_expand_canvas_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 58, enabled); }
    pub fn set_cycle_theme_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 59, enabled); }
    pub fn set_prev_frame_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 60, enabled); }
    pub fn set_next_frame_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 61, enabled); }
    pub fn set_cut_selection_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 62, enabled); }
    pub fn set_silent_copy_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 63, enabled); }
    pub fn set_combine_videos_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 64, enabled); }
    pub fn set_video_from_images_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 65, enabled); }
    pub fn set_toggle_mute_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 66, enabled); }
    pub fn set_toggle_library_view_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 67, enabled); }
    pub fn set_combine_selected_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 68, enabled); }
    pub fn set_clear_library_selection_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 69, enabled); }
    pub fn set_show_closed_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 70, enabled); }
    pub fn set_reset_toolbar_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 71, enabled); }
    pub fn set_narrate_images_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 72, enabled); }
    pub fn set_narrate_next_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 73, enabled); }
    pub fn set_narrate_finish_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 74, enabled); }
    pub fn set_toggle_side_bar_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 75, enabled); }
    pub fn set_toggle_effects_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 76, enabled); }
    pub fn set_toggle_tray_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 77, enabled); }
    pub fn set_toggle_tool_labels_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 78, enabled); }
    pub fn set_open_capture_window_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 79, enabled); }
    pub fn set_open_recent_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 80, enabled); }
    pub fn set_auto_trim_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 81, enabled); }
    pub fn set_tool_crop_menu_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 82, enabled); }
    pub fn set_tool_select_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 83, enabled); }
    pub fn set_tool_arrow_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 84, enabled); }
    pub fn set_tool_line_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 85, enabled); }
    pub fn set_tool_shape_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 86, enabled); }
    pub fn set_tool_text_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 87, enabled); }
    pub fn set_tool_callout_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 88, enabled); }
    pub fn set_tool_step_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 89, enabled); }
    pub fn set_tool_pen_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 90, enabled); }
    pub fn set_tool_highlight_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 91, enabled); }
    pub fn set_tool_blur_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 92, enabled); }
    pub fn set_tool_magnify_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 93, enabled); }
    pub fn set_tool_spotlight_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 94, enabled); }
    pub fn set_tool_stamp_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 95, enabled); }
    pub fn set_tool_fill_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 96, enabled); }
    pub fn set_tool_crop_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 97, enabled); }
    pub fn set_tool_cut_out_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 98, enabled); }
    pub fn set_tool_selection_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 99, enabled); }
    pub fn set_tool_eyedropper_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 100, enabled); }
    pub fn set_tool_smart_move_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 101, enabled); }
    pub fn set_tool_text_replace_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 102, enabled); }
    pub fn set_run_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 103, enabled); }
    /// Stages the most-recently-used storage URIs as one `RecentFiles` snapshot.
    pub fn set_recent_files(&mut self, recent: &crate::RecentFileList) { self.0.push_string_snapshot(11, recent.entries()); }
}

pub trait MainViewModel: Send + 'static {
    fn attach(&mut self, sink: MainViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_stroke_color(&mut self, value: String) -> crate::Result<()>;
    fn set_fill_color(&mut self, value: String) -> crate::Result<()>;
    fn set_text_color(&mut self, value: String) -> crate::Result<()>;
    fn set_stroke_width(&mut self, value: f64) -> crate::Result<()>;
    fn set_font_size(&mut self, value: f64) -> crate::Result<()>;
    fn set_font_family(&mut self, value: String) -> crate::Result<()>;
    fn set_bold(&mut self, value: bool) -> crate::Result<()>;
    fn set_italic(&mut self, value: bool) -> crate::Result<()>;
    fn set_shadow(&mut self, value: bool) -> crate::Result<()>;
    fn set_opacity(&mut self, value: f64) -> crate::Result<()>;
    fn set_text(&mut self, value: String) -> crate::Result<()>;
    fn set_variant(&mut self, value: String) -> crate::Result<()>;
    fn set_amount(&mut self, value: f64) -> crate::Result<()>;
    fn set_align(&mut self, value: String) -> crate::Result<()>;
    fn set_dash(&mut self, value: String) -> crate::Result<()>;
    fn set_start_cap(&mut self, value: String) -> crate::Result<()>;
    fn set_end_cap(&mut self, value: String) -> crate::Result<()>;
    fn set_action(&mut self, value: String) -> crate::Result<()>;
    fn set_video_position(&mut self, value: f64) -> crate::Result<()>;
    fn set_rotation(&mut self, value: f64) -> crate::Result<()>;
    fn set_background_fill(&mut self, value: String) -> crate::Result<()>;
    fn set_head_size(&mut self, value: f64) -> crate::Result<()>;
    fn set_shadow_angle(&mut self, value: f64) -> crate::Result<()>;
    fn set_shadow_distance(&mut self, value: f64) -> crate::Result<()>;
    fn set_shadow_blur(&mut self, value: f64) -> crate::Result<()>;
    fn set_shadow_color(&mut self, value: String) -> crate::Result<()>;
    fn set_outline_color(&mut self, value: String) -> crate::Result<()>;
    fn set_outline_width(&mut self, value: f64) -> crate::Result<()>;
    fn set_underline(&mut self, value: bool) -> crate::Result<()>;
    fn set_strike(&mut self, value: bool) -> crate::Result<()>;
    fn set_v_align(&mut self, value: String) -> crate::Result<()>;
    fn set_padding(&mut self, value: f64) -> crate::Result<()>;
    fn set_fill2(&mut self, value: String) -> crate::Result<()>;
    fn set_gradient_radial(&mut self, value: bool) -> crate::Result<()>;
    fn set_gradient_angle(&mut self, value: f64) -> crate::Result<()>;
    fn set_canvas_color(&mut self, value: String) -> crate::Result<()>;
    fn set_gif_preset(&mut self, value: String) -> crate::Result<()>;
    fn set_gif_loop(&mut self, value: bool) -> crate::Result<()>;
    fn set_gif_fade(&mut self, value: bool) -> crate::Result<()>;
    fn set_gif_dither(&mut self, value: bool) -> crate::Result<()>;
    fn set_video_volume(&mut self, value: f64) -> crate::Result<()>;
    fn set_library_search(&mut self, value: String) -> crate::Result<()>;
    fn set_library_filter(&mut self, value: String) -> crate::Result<()>;
    fn set_library_sort(&mut self, value: String) -> crate::Result<()>;
    fn set_thumb_size(&mut self, value: f64) -> crate::Result<()>;
    fn set_library_expanded(&mut self, value: bool) -> crate::Result<()>;
    fn set_template_theme(&mut self, value: String) -> crate::Result<()>;
    fn set_template_captions(&mut self, value: bool) -> crate::Result<()>;
    fn set_template_numbers(&mut self, value: bool) -> crate::Result<()>;
    fn set_template_footer(&mut self, value: String) -> crate::Result<()>;
    fn set_show_side_bar(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_effects(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_tray(&mut self, value: bool) -> crate::Result<()>;
    fn set_tool_labels(&mut self, value: bool) -> crate::Result<()>;
    fn new_image(&mut self) -> crate::Result<()>;
    fn open(&mut self) -> crate::Result<()>;
    fn save(&mut self) -> crate::Result<()>;
    fn save_as(&mut self) -> crate::Result<()>;
    fn export(&mut self) -> crate::Result<()>;
    fn copy_image(&mut self) -> crate::Result<()>;
    fn paste(&mut self) -> crate::Result<()>;
    fn new_capture(&mut self) -> crate::Result<()>;
    fn combine_images(&mut self) -> crate::Result<()>;
    fn open_library_folder(&mut self) -> crate::Result<()>;
    fn exit(&mut self) -> crate::Result<()>;
    fn undo(&mut self) -> crate::Result<()>;
    fn redo(&mut self) -> crate::Result<()>;
    fn copy(&mut self) -> crate::Result<()>;
    fn cut(&mut self) -> crate::Result<()>;
    fn delete(&mut self) -> crate::Result<()>;
    fn duplicate(&mut self) -> crate::Result<()>;
    fn bring_to_front(&mut self) -> crate::Result<()>;
    fn bring_forward(&mut self) -> crate::Result<()>;
    fn send_backward(&mut self) -> crate::Result<()>;
    fn send_to_back(&mut self) -> crate::Result<()>;
    fn save_quick_style(&mut self) -> crate::Result<()>;
    fn rotate_left(&mut self) -> crate::Result<()>;
    fn rotate_right(&mut self) -> crate::Result<()>;
    fn flip_horizontal(&mut self) -> crate::Result<()>;
    fn flip_vertical(&mut self) -> crate::Result<()>;
    fn resize50(&mut self) -> crate::Result<()>;
    fn resize200(&mut self) -> crate::Result<()>;
    fn flatten(&mut self) -> crate::Result<()>;
    fn zoom_in(&mut self) -> crate::Result<()>;
    fn zoom_out(&mut self) -> crate::Result<()>;
    fn zoom_fit(&mut self) -> crate::Result<()>;
    fn zoom_actual(&mut self) -> crate::Result<()>;
    fn grab_text(&mut self) -> crate::Result<()>;
    fn simplify(&mut self) -> crate::Result<()>;
    fn play_pause(&mut self) -> crate::Result<()>;
    fn set_trim_start(&mut self) -> crate::Result<()>;
    fn set_trim_end(&mut self) -> crate::Result<()>;
    fn export_video(&mut self) -> crate::Result<()>;
    fn export_gif(&mut self) -> crate::Result<()>;
    fn frame_to_image(&mut self) -> crate::Result<()>;
    fn record_video(&mut self) -> crate::Result<()>;
    fn canvas_changed(&mut self) -> crate::Result<()>;
    fn select_all(&mut self) -> crate::Result<()>;
    fn deselect_all(&mut self) -> crate::Result<()>;
    fn invert_selection(&mut self) -> crate::Result<()>;
    fn group(&mut self) -> crate::Result<()>;
    fn ungroup(&mut self) -> crate::Result<()>;
    fn flatten_selected(&mut self) -> crate::Result<()>;
    fn smart_redact(&mut self) -> crate::Result<()>;
    fn new_from_clipboard(&mut self) -> crate::Result<()>;
    fn print(&mut self) -> crate::Result<()>;
    fn close_file(&mut self) -> crate::Result<()>;
    fn duplicate_file(&mut self) -> crate::Result<()>;
    fn delete_file(&mut self) -> crate::Result<()>;
    fn tool_magic_wand(&mut self) -> crate::Result<()>;
    fn tool_eraser(&mut self) -> crate::Result<()>;
    fn expand_canvas(&mut self) -> crate::Result<()>;
    fn cycle_theme(&mut self) -> crate::Result<()>;
    fn prev_frame(&mut self) -> crate::Result<()>;
    fn next_frame(&mut self) -> crate::Result<()>;
    fn cut_selection(&mut self) -> crate::Result<()>;
    fn silent_copy(&mut self) -> crate::Result<()>;
    fn combine_videos(&mut self) -> crate::Result<()>;
    fn video_from_images(&mut self) -> crate::Result<()>;
    fn toggle_mute(&mut self) -> crate::Result<()>;
    fn toggle_library_view(&mut self) -> crate::Result<()>;
    fn combine_selected(&mut self) -> crate::Result<()>;
    fn clear_library_selection(&mut self) -> crate::Result<()>;
    fn show_closed(&mut self) -> crate::Result<()>;
    fn reset_toolbar(&mut self) -> crate::Result<()>;
    fn narrate_images(&mut self) -> crate::Result<()>;
    fn narrate_next(&mut self) -> crate::Result<()>;
    fn narrate_finish(&mut self) -> crate::Result<()>;
    fn toggle_side_bar(&mut self) -> crate::Result<()>;
    fn toggle_effects(&mut self) -> crate::Result<()>;
    fn toggle_tray(&mut self) -> crate::Result<()>;
    fn toggle_tool_labels(&mut self) -> crate::Result<()>;
    fn open_capture_window(&mut self) -> crate::Result<()>;
    fn open_recent_file(&mut self, value: String) -> crate::Result<()>;
    fn auto_trim(&mut self) -> crate::Result<()>;
    fn tool_crop_menu(&mut self) -> crate::Result<()>;
    fn tool_select(&mut self) -> crate::Result<()>;
    fn tool_arrow(&mut self) -> crate::Result<()>;
    fn tool_line(&mut self) -> crate::Result<()>;
    fn tool_shape(&mut self) -> crate::Result<()>;
    fn tool_text(&mut self) -> crate::Result<()>;
    fn tool_callout(&mut self) -> crate::Result<()>;
    fn tool_step(&mut self) -> crate::Result<()>;
    fn tool_pen(&mut self) -> crate::Result<()>;
    fn tool_highlight(&mut self) -> crate::Result<()>;
    fn tool_blur(&mut self) -> crate::Result<()>;
    fn tool_magnify(&mut self) -> crate::Result<()>;
    fn tool_spotlight(&mut self) -> crate::Result<()>;
    fn tool_stamp(&mut self) -> crate::Result<()>;
    fn tool_fill(&mut self) -> crate::Result<()>;
    fn tool_crop(&mut self) -> crate::Result<()>;
    fn tool_cut_out(&mut self) -> crate::Result<()>;
    fn tool_selection(&mut self) -> crate::Result<()>;
    fn tool_eyedropper(&mut self) -> crate::Result<()>;
    fn tool_smart_move(&mut self) -> crate::Result<()>;
    fn tool_text_replace(&mut self) -> crate::Result<()>;
    fn run(&mut self, value: String) -> crate::Result<()>;
}

struct MainViewModelDispatch<T: MainViewModel> { model: T }

impl<T: MainViewModel> crate::view_model::DynamicViewModel for MainViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(MainViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, value: String) -> crate::Result<()> {
        match property_id {
            6 => self.model.set_stroke_color(value),
            7 => self.model.set_fill_color(value),
            8 => self.model.set_text_color(value),
            11 => self.model.set_font_family(value),
            16 => self.model.set_text(value),
            17 => self.model.set_variant(value),
            19 => self.model.set_align(value),
            20 => self.model.set_dash(value),
            21 => self.model.set_start_cap(value),
            22 => self.model.set_end_cap(value),
            28 => self.model.set_action(value),
            39 => self.model.set_background_fill(value),
            45 => self.model.set_shadow_color(value),
            46 => self.model.set_outline_color(value),
            50 => self.model.set_v_align(value),
            52 => self.model.set_fill2(value),
            55 => self.model.set_canvas_color(value),
            57 => self.model.set_gif_preset(value),
            63 => self.model.set_library_search(value),
            64 => self.model.set_library_filter(value),
            65 => self.model.set_library_sort(value),
            69 => self.model.set_template_theme(value),
            72 => self.model.set_template_footer(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, value: bool) -> crate::Result<()> {
        match property_id {
            12 => self.model.set_bold(value),
            13 => self.model.set_italic(value),
            14 => self.model.set_shadow(value),
            48 => self.model.set_underline(value),
            49 => self.model.set_strike(value),
            53 => self.model.set_gradient_radial(value),
            58 => self.model.set_gif_loop(value),
            59 => self.model.set_gif_fade(value),
            60 => self.model.set_gif_dither(value),
            67 => self.model.set_library_expanded(value),
            70 => self.model.set_template_captions(value),
            71 => self.model.set_template_numbers(value),
            76 => self.model.set_show_side_bar(value),
            77 => self.model.set_show_effects(value),
            78 => self.model.set_show_tray(value),
            79 => self.model.set_tool_labels(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_double(&mut self, property_id: i32, value: f64) -> crate::Result<()> {
        match property_id {
            9 => self.model.set_stroke_width(value),
            10 => self.model.set_font_size(value),
            15 => self.model.set_opacity(value),
            18 => self.model.set_amount(value),
            30 => self.model.set_video_position(value),
            37 => self.model.set_rotation(value),
            41 => self.model.set_head_size(value),
            42 => self.model.set_shadow_angle(value),
            43 => self.model.set_shadow_distance(value),
            44 => self.model.set_shadow_blur(value),
            47 => self.model.set_outline_width(value),
            51 => self.model.set_padding(value),
            54 => self.model.set_gradient_angle(value),
            61 => self.model.set_video_volume(value),
            66 => self.model.set_thumb_size(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.new_image(),
            2 => self.model.open(),
            3 => self.model.save(),
            4 => self.model.save_as(),
            5 => self.model.export(),
            6 => self.model.copy_image(),
            7 => self.model.paste(),
            8 => self.model.new_capture(),
            9 => self.model.combine_images(),
            10 => self.model.open_library_folder(),
            11 => self.model.exit(),
            12 => self.model.undo(),
            13 => self.model.redo(),
            14 => self.model.copy(),
            15 => self.model.cut(),
            16 => self.model.delete(),
            17 => self.model.duplicate(),
            18 => self.model.bring_to_front(),
            19 => self.model.bring_forward(),
            20 => self.model.send_backward(),
            21 => self.model.send_to_back(),
            22 => self.model.save_quick_style(),
            23 => self.model.rotate_left(),
            24 => self.model.rotate_right(),
            25 => self.model.flip_horizontal(),
            26 => self.model.flip_vertical(),
            27 => self.model.resize50(),
            28 => self.model.resize200(),
            29 => self.model.flatten(),
            30 => self.model.zoom_in(),
            31 => self.model.zoom_out(),
            32 => self.model.zoom_fit(),
            33 => self.model.zoom_actual(),
            34 => self.model.grab_text(),
            35 => self.model.simplify(),
            36 => self.model.play_pause(),
            37 => self.model.set_trim_start(),
            38 => self.model.set_trim_end(),
            39 => self.model.export_video(),
            40 => self.model.export_gif(),
            41 => self.model.frame_to_image(),
            42 => self.model.record_video(),
            43 => self.model.canvas_changed(),
            44 => self.model.select_all(),
            45 => self.model.deselect_all(),
            46 => self.model.invert_selection(),
            47 => self.model.group(),
            48 => self.model.ungroup(),
            49 => self.model.flatten_selected(),
            50 => self.model.smart_redact(),
            51 => self.model.new_from_clipboard(),
            52 => self.model.print(),
            53 => self.model.close_file(),
            54 => self.model.duplicate_file(),
            55 => self.model.delete_file(),
            56 => self.model.tool_magic_wand(),
            57 => self.model.tool_eraser(),
            58 => self.model.expand_canvas(),
            59 => self.model.cycle_theme(),
            60 => self.model.prev_frame(),
            61 => self.model.next_frame(),
            62 => self.model.cut_selection(),
            63 => self.model.silent_copy(),
            64 => self.model.combine_videos(),
            65 => self.model.video_from_images(),
            66 => self.model.toggle_mute(),
            67 => self.model.toggle_library_view(),
            68 => self.model.combine_selected(),
            69 => self.model.clear_library_selection(),
            70 => self.model.show_closed(),
            71 => self.model.reset_toolbar(),
            72 => self.model.narrate_images(),
            73 => self.model.narrate_next(),
            74 => self.model.narrate_finish(),
            75 => self.model.toggle_side_bar(),
            76 => self.model.toggle_effects(),
            77 => self.model.toggle_tray(),
            78 => self.model.toggle_tool_labels(),
            79 => self.model.open_capture_window(),
            80 => self.model.open_recent_file(parameter.unwrap_or_default()),
            81 => self.model.auto_trim(),
            82 => self.model.tool_crop_menu(),
            83 => self.model.tool_select(),
            84 => self.model.tool_arrow(),
            85 => self.model.tool_line(),
            86 => self.model.tool_shape(),
            87 => self.model.tool_text(),
            88 => self.model.tool_callout(),
            89 => self.model.tool_step(),
            90 => self.model.tool_pen(),
            91 => self.model.tool_highlight(),
            92 => self.model.tool_blur(),
            93 => self.model.tool_magnify(),
            94 => self.model.tool_spotlight(),
            95 => self.model.tool_stamp(),
            96 => self.model.tool_fill(),
            97 => self.model.tool_crop(),
            98 => self.model.tool_cut_out(),
            99 => self.model.tool_selection(),
            100 => self.model.tool_eyedropper(),
            101 => self.model.tool_smart_move(),
            102 => self.model.tool_text_replace(),
            103 => self.model.run(parameter.unwrap_or_default()),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}

pub fn mount_main_window(scope: &crate::AppScope, model: impl MainViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model(1, MainViewModelDispatch { model }) }
pub fn mount_main_window_hidden(scope: &crate::AppScope, model: impl MainViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model_hidden(1, MainViewModelDispatch { model }).map(drop) }

#[derive(Clone, Debug)]
pub struct LibraryItemViewModelSink(crate::view_model::ViewModelSink);

impl LibraryItemViewModelSink {
    pub fn set_path(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_thumbnail(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_is_current(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(4, value) }
    pub fn set_kind(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(5, value) }
    pub fn set_is_pinned(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(6, value) }
    pub fn set_tags(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(7, value) }
    pub fn set_info(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(8, value) }
    pub fn set_is_selected(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(9, value) }
    pub fn set_new_name(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(10, value) }
    pub fn set_tag_text(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(11, value) }
    pub fn set_unsaved(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(12, value) }
    pub fn set_thumb_height(&self, value: f64) -> crate::Result<()> { self.0.set_double(13, value) }
    pub fn set_open_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_toggle_pin_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_close_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_delete_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(4, enabled) }
    pub fn set_rename_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(5, enabled) }
    pub fn set_set_tags_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(6, enabled) }
    pub fn set_toggle_select_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(7, enabled) }
    pub fn set_show_in_folder_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(8, enabled) }
    pub fn set_path_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_thumbnail_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_is_current_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_kind_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_is_pinned_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_tags_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_info_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_is_selected_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_new_name_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    pub fn set_tag_text_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(11, message) }
    pub fn set_unsaved_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(12, message) }
    pub fn set_thumb_height_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(13, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> LibraryItemViewModelSinkBatch { LibraryItemViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: LibraryItemViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct LibraryItemViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl LibraryItemViewModelSinkBatch {
    pub fn set_path(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_path_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_path_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_label_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_thumbnail(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_thumbnail_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_thumbnail_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_is_current(&mut self, value: bool) { self.0.push_boolean(3, 4, value); }
    pub fn set_is_current_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_is_current_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_kind(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 5, 0, value); }
    pub fn set_kind_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_kind_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_is_pinned(&mut self, value: bool) { self.0.push_boolean(3, 6, value); }
    pub fn set_is_pinned_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_is_pinned_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_tags(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 7, 0, value); }
    pub fn set_tags_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_tags_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_info(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 8, 0, value); }
    pub fn set_info_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_info_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_is_selected(&mut self, value: bool) { self.0.push_boolean(3, 9, value); }
    pub fn set_is_selected_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_is_selected_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_new_name(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 10, 0, value); }
    pub fn set_new_name_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_new_name_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_tag_text(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 11, 0, value); }
    pub fn set_tag_text_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 11, 0, message); }
    pub fn clear_tag_text_error(&mut self) { self.0.push_clear_error(11); }
    pub fn set_unsaved(&mut self, value: bool) { self.0.push_boolean(3, 12, value); }
    pub fn set_unsaved_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 12, 0, message); }
    pub fn clear_unsaved_error(&mut self) { self.0.push_clear_error(12); }
    pub fn set_thumb_height(&mut self, value: f64) { self.0.push_double(13, value); }
    pub fn set_thumb_height_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 13, 0, message); }
    pub fn clear_thumb_height_error(&mut self) { self.0.push_clear_error(13); }
    pub fn set_open_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_toggle_pin_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_close_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
    pub fn set_delete_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 4, enabled); }
    pub fn set_rename_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 5, enabled); }
    pub fn set_set_tags_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 6, enabled); }
    pub fn set_toggle_select_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 7, enabled); }
    pub fn set_show_in_folder_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 8, enabled); }
}

pub trait LibraryItemViewModel: Send + 'static {
    fn attach(&mut self, sink: LibraryItemViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_new_name(&mut self, value: String) -> crate::Result<()>;
    fn set_tag_text(&mut self, value: String) -> crate::Result<()>;
    fn open(&mut self) -> crate::Result<()>;
    fn toggle_pin(&mut self) -> crate::Result<()>;
    fn close(&mut self) -> crate::Result<()>;
    fn delete(&mut self) -> crate::Result<()>;
    fn rename(&mut self, value: String) -> crate::Result<()>;
    fn set_tags(&mut self, value: String) -> crate::Result<()>;
    fn toggle_select(&mut self) -> crate::Result<()>;
    fn show_in_folder(&mut self) -> crate::Result<()>;
}

struct LibraryItemViewModelDispatch<T: LibraryItemViewModel> { model: T }

impl<T: LibraryItemViewModel> crate::view_model::DynamicViewModel for LibraryItemViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(LibraryItemViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, value: String) -> crate::Result<()> {
        match property_id {
            10 => self.model.set_new_name(value),
            11 => self.model.set_tag_text(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, _value: bool) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.open(),
            2 => self.model.toggle_pin(),
            3 => self.model.close(),
            4 => self.model.delete(),
            5 => self.model.rename(parameter.unwrap_or_default()),
            6 => self.model.set_tags(parameter.unwrap_or_default()),
            7 => self.model.toggle_select(),
            8 => self.model.show_in_folder(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}


#[derive(Clone, Debug)]
pub struct QuickStyleViewModelSink(crate::view_model::ViewModelSink);

impl QuickStyleViewModelSink {
    pub fn set_name(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_stroke(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_fill(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_text_color(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(4, value) }
    pub fn set_width(&self, value: f64) -> crate::Result<()> { self.0.set_double(5, value) }
    pub fn set_is_favorite(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(6, value) }
    pub fn set_apply_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_delete_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_move_earlier_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_move_later_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(4, enabled) }
    pub fn set_toggle_favorite_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(5, enabled) }
    pub fn set_name_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_stroke_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_fill_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_text_color_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_width_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_is_favorite_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> QuickStyleViewModelSinkBatch { QuickStyleViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: QuickStyleViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct QuickStyleViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl QuickStyleViewModelSinkBatch {
    pub fn set_name(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_name_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_name_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_stroke(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_stroke_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_stroke_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_fill(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_fill_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_fill_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_text_color(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 4, 0, value); }
    pub fn set_text_color_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_text_color_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_width(&mut self, value: f64) { self.0.push_double(5, value); }
    pub fn set_width_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_width_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_is_favorite(&mut self, value: bool) { self.0.push_boolean(3, 6, value); }
    pub fn set_is_favorite_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_is_favorite_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_apply_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_delete_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_move_earlier_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
    pub fn set_move_later_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 4, enabled); }
    pub fn set_toggle_favorite_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 5, enabled); }
}

pub trait QuickStyleViewModel: Send + 'static {
    fn attach(&mut self, sink: QuickStyleViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn apply(&mut self) -> crate::Result<()>;
    fn delete(&mut self) -> crate::Result<()>;
    fn move_earlier(&mut self) -> crate::Result<()>;
    fn move_later(&mut self) -> crate::Result<()>;
    fn toggle_favorite(&mut self) -> crate::Result<()>;
}

struct QuickStyleViewModelDispatch<T: QuickStyleViewModel> { model: T }

impl<T: QuickStyleViewModel> crate::view_model::DynamicViewModel for QuickStyleViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(QuickStyleViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, _value: String) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, _value: bool) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.apply(),
            2 => self.model.delete(),
            3 => self.model.move_earlier(),
            4 => self.model.move_later(),
            5 => self.model.toggle_favorite(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}


#[derive(Clone, Debug)]
pub struct TrayViewModelSink(crate::view_model::ViewModelSink);

/// Declared capacity of the `TrayViewModel` recent-file list.
pub const TRAY_VIEW_MODEL_RECENT_CAPTURES_CAPACITY: usize = 10;

impl TrayViewModelSink {
    pub fn set_copy_to_clipboard(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(1, value) }
    pub fn set_open_in_editor(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(2, value) }
    pub fn set_include_cursor(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(3, value) }
    pub fn set_show_widget(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(4, value) }
    pub fn set_run_at_startup(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(5, value) }
    pub fn set_time_lapse(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(6, value) }
    pub fn set_schedule(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(7, value) }
    pub fn set_can_repeat(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(8, value) }
    pub fn add_recent_captures(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(1, value) }
    pub fn insert_recent_captures(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(1, index, value) }
    pub fn replace_recent_captures(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(1, index, value) }
    pub fn remove_recent_captures(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(1, index) }
    pub fn move_recent_captures(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(1, from_index, to_index) }
    pub fn clear_recent_captures(&self) -> crate::Result<()> { self.0.clear_string_collection(1) }
    pub fn set_capture_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_capture_delayed_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_capture_screen_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_repeat_last_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(4, enabled) }
    pub fn set_capture_window_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(5, enabled) }
    pub fn set_show_editor_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(6, enabled) }
    pub fn set_scrolling_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(7, enabled) }
    pub fn set_scrolling_horizontal_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(8, enabled) }
    pub fn set_scrolling_both_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(9, enabled) }
    pub fn set_panoramic_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(10, enabled) }
    pub fn set_freehand_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(11, enabled) }
    pub fn set_multiple_areas_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(12, enabled) }
    pub fn set_fixed_size_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(13, enabled) }
    pub fn set_menu_capture_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(14, enabled) }
    pub fn set_grab_text_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(15, enabled) }
    pub fn set_step_capture_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(16, enabled) }
    pub fn set_from_clipboard_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(17, enabled) }
    pub fn set_from_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(18, enabled) }
    pub fn set_record_video_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(19, enabled) }
    pub fn set_open_library_folder_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(20, enabled) }
    pub fn set_exit_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(21, enabled) }
    pub fn set_preferences_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(22, enabled) }
    pub fn set_open_recent_capture_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(23, enabled) }
    /// Publishes the most-recently-used storage URIs into `RecentCaptures`.
    ///
    /// The list is bounded by its own capacity (10), so this replaces a
    /// handful of entries rather than a data set; the generated menu derives
    /// each header from the URI and passes the URI back as the command parameter.
    pub fn publish_recent_captures(&self, recent: &crate::RecentFileList) -> crate::Result<()> {
        self.0.clear_string_collection(1)?;
        for uri in recent.entries() { self.0.add_string(1, uri)?; }
        Ok(())
    }
    pub fn set_copy_to_clipboard_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_open_in_editor_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_include_cursor_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_show_widget_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_run_at_startup_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_time_lapse_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_schedule_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_can_repeat_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> TrayViewModelSinkBatch { TrayViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: TrayViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct TrayViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl TrayViewModelSinkBatch {
    pub fn set_copy_to_clipboard(&mut self, value: bool) { self.0.push_boolean(3, 1, value); }
    pub fn set_copy_to_clipboard_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_copy_to_clipboard_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_open_in_editor(&mut self, value: bool) { self.0.push_boolean(3, 2, value); }
    pub fn set_open_in_editor_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_open_in_editor_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_include_cursor(&mut self, value: bool) { self.0.push_boolean(3, 3, value); }
    pub fn set_include_cursor_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_include_cursor_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_show_widget(&mut self, value: bool) { self.0.push_boolean(3, 4, value); }
    pub fn set_show_widget_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_show_widget_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_run_at_startup(&mut self, value: bool) { self.0.push_boolean(3, 5, value); }
    pub fn set_run_at_startup_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_run_at_startup_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_time_lapse(&mut self, value: bool) { self.0.push_boolean(3, 6, value); }
    pub fn set_time_lapse_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_time_lapse_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_schedule(&mut self, value: bool) { self.0.push_boolean(3, 7, value); }
    pub fn set_schedule_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_schedule_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_can_repeat(&mut self, value: bool) { self.0.push_boolean(3, 8, value); }
    pub fn set_can_repeat_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_can_repeat_error(&mut self) { self.0.push_clear_error(8); }
    pub fn add_recent_captures(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 1, 0, value); }
    pub fn insert_recent_captures(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 1, index, value); }
    pub fn replace_recent_captures(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 1, index, value); }
    pub fn replace_recent_captures_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(1, values); }
    pub fn remove_recent_captures(&mut self, index: i32) { self.0.push_indices(13, 1, index, 0); }
    pub fn move_recent_captures(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 1, from_index, to_index); }
    pub fn clear_recent_captures(&mut self) { self.0.push_indices(19, 1, 0, 0); }
    pub fn set_capture_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_capture_delayed_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_capture_screen_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
    pub fn set_repeat_last_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 4, enabled); }
    pub fn set_capture_window_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 5, enabled); }
    pub fn set_show_editor_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 6, enabled); }
    pub fn set_scrolling_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 7, enabled); }
    pub fn set_scrolling_horizontal_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 8, enabled); }
    pub fn set_scrolling_both_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 9, enabled); }
    pub fn set_panoramic_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 10, enabled); }
    pub fn set_freehand_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 11, enabled); }
    pub fn set_multiple_areas_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 12, enabled); }
    pub fn set_fixed_size_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 13, enabled); }
    pub fn set_menu_capture_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 14, enabled); }
    pub fn set_grab_text_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 15, enabled); }
    pub fn set_step_capture_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 16, enabled); }
    pub fn set_from_clipboard_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 17, enabled); }
    pub fn set_from_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 18, enabled); }
    pub fn set_record_video_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 19, enabled); }
    pub fn set_open_library_folder_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 20, enabled); }
    pub fn set_exit_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 21, enabled); }
    pub fn set_preferences_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 22, enabled); }
    pub fn set_open_recent_capture_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 23, enabled); }
    /// Stages the most-recently-used storage URIs as one `RecentCaptures` snapshot.
    pub fn set_recent_captures(&mut self, recent: &crate::RecentFileList) { self.0.push_string_snapshot(1, recent.entries()); }
}

pub trait TrayViewModel: Send + 'static {
    fn attach(&mut self, sink: TrayViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_copy_to_clipboard(&mut self, value: bool) -> crate::Result<()>;
    fn set_open_in_editor(&mut self, value: bool) -> crate::Result<()>;
    fn set_include_cursor(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_widget(&mut self, value: bool) -> crate::Result<()>;
    fn set_run_at_startup(&mut self, value: bool) -> crate::Result<()>;
    fn set_time_lapse(&mut self, value: bool) -> crate::Result<()>;
    fn set_schedule(&mut self, value: bool) -> crate::Result<()>;
    fn capture(&mut self) -> crate::Result<()>;
    fn capture_delayed(&mut self) -> crate::Result<()>;
    fn capture_screen(&mut self) -> crate::Result<()>;
    fn repeat_last(&mut self) -> crate::Result<()>;
    fn capture_window(&mut self) -> crate::Result<()>;
    fn show_editor(&mut self) -> crate::Result<()>;
    fn scrolling(&mut self) -> crate::Result<()>;
    fn scrolling_horizontal(&mut self) -> crate::Result<()>;
    fn scrolling_both(&mut self) -> crate::Result<()>;
    fn panoramic(&mut self) -> crate::Result<()>;
    fn freehand(&mut self) -> crate::Result<()>;
    fn multiple_areas(&mut self) -> crate::Result<()>;
    fn fixed_size(&mut self) -> crate::Result<()>;
    fn menu_capture(&mut self) -> crate::Result<()>;
    fn grab_text(&mut self) -> crate::Result<()>;
    fn step_capture(&mut self) -> crate::Result<()>;
    fn from_clipboard(&mut self) -> crate::Result<()>;
    fn from_file(&mut self) -> crate::Result<()>;
    fn record_video(&mut self) -> crate::Result<()>;
    fn open_library_folder(&mut self) -> crate::Result<()>;
    fn exit(&mut self) -> crate::Result<()>;
    fn preferences(&mut self) -> crate::Result<()>;
    fn open_recent_capture(&mut self, value: String) -> crate::Result<()>;
}

struct TrayViewModelDispatch<T: TrayViewModel> { model: T }

impl<T: TrayViewModel> crate::view_model::DynamicViewModel for TrayViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(TrayViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, _value: String) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, value: bool) -> crate::Result<()> {
        match property_id {
            1 => self.model.set_copy_to_clipboard(value),
            2 => self.model.set_open_in_editor(value),
            3 => self.model.set_include_cursor(value),
            4 => self.model.set_show_widget(value),
            5 => self.model.set_run_at_startup(value),
            6 => self.model.set_time_lapse(value),
            7 => self.model.set_schedule(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.capture(),
            2 => self.model.capture_delayed(),
            3 => self.model.capture_screen(),
            4 => self.model.repeat_last(),
            5 => self.model.capture_window(),
            6 => self.model.show_editor(),
            7 => self.model.scrolling(),
            8 => self.model.scrolling_horizontal(),
            9 => self.model.scrolling_both(),
            10 => self.model.panoramic(),
            11 => self.model.freehand(),
            12 => self.model.multiple_areas(),
            13 => self.model.fixed_size(),
            14 => self.model.menu_capture(),
            15 => self.model.grab_text(),
            16 => self.model.step_capture(),
            17 => self.model.from_clipboard(),
            18 => self.model.from_file(),
            19 => self.model.record_video(),
            20 => self.model.open_library_folder(),
            21 => self.model.exit(),
            22 => self.model.preferences(),
            23 => self.model.open_recent_capture(parameter.unwrap_or_default()),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}

pub fn mount_tray(scope: &crate::AppScope, model: impl TrayViewModel) -> crate::Result<()> { scope.mount_dynamic_tray(1, TrayViewModelDispatch { model }) }

#[derive(Clone, Debug)]
pub struct CaptureViewModelSink(crate::view_model::ViewModelSink);

impl CaptureViewModelSink {
    pub fn set_view(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_menu(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_panel(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_problem(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(4, value) }
    pub fn set_action(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(5, value) }
    pub fn set_run_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_view_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_menu_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_panel_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_problem_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_action_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> CaptureViewModelSinkBatch { CaptureViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: CaptureViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct CaptureViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl CaptureViewModelSinkBatch {
    pub fn set_view(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_view_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_view_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_menu(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_menu_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_menu_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_panel(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_panel_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_panel_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_problem(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 4, 0, value); }
    pub fn set_problem_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_problem_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_action(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 5, 0, value); }
    pub fn set_action_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_action_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_run_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
}

pub trait CaptureViewModel: Send + 'static {
    fn attach(&mut self, sink: CaptureViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_action(&mut self, value: String) -> crate::Result<()>;
    fn run(&mut self, value: String) -> crate::Result<()>;
}

struct CaptureViewModelDispatch<T: CaptureViewModel> { model: T }

impl<T: CaptureViewModel> crate::view_model::DynamicViewModel for CaptureViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(CaptureViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, value: String) -> crate::Result<()> {
        match property_id {
            5 => self.model.set_action(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, _value: bool) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.run(parameter.unwrap_or_default()),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}

pub fn mount_capture_window(scope: &crate::AppScope, model: impl CaptureViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model(2, CaptureViewModelDispatch { model }) }
pub fn mount_capture_window_hidden(scope: &crate::AppScope, model: impl CaptureViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model_hidden(2, CaptureViewModelDispatch { model }).map(drop) }

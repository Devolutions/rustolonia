//! Generated from view-model.ir.json. Do not edit.

#![allow(dead_code)]

#[repr(i64)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
}

impl std::convert::TryFrom<i64> for Priority {
    type Error = ();
    fn try_from(value: i64) -> std::result::Result<Self, ()> {
        match value {
            0 => Ok(Self::Low),
            1 => Ok(Self::Normal),
            2 => Ok(Self::High),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SampleViewModelSink(crate::view_model::ViewModelSink);

/// Declared capacity of the `SampleViewModel` recent-file list.
pub const SAMPLE_VIEW_MODEL_RECENT_FILES_CAPACITY: usize = 8;

impl SampleViewModelSink {
    pub fn set_name(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_count(&self, value: i64) -> crate::Result<()> { self.0.set_integer(2, value) }
    pub fn set_new_item(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(4, value) }
    pub fn set_nickname(&self, value: Option<impl AsRef<str>>) -> crate::Result<()> { match value { Some(value) => self.0.set_string(5, value), None => self.0.set_null(5) } }
    pub fn set_priority(&self, value: Priority) -> crate::Result<()> { self.0.set_integer(6, value as i64) }
    pub fn set_address<M: AddressViewModel>(&self, value: Option<M>) -> crate::Result<()> { self.0.set_model(7, value.map(|model| AddressViewModelDispatch { model })) }
    pub fn set_new_task_title(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(8, value) }
    pub fn set_selected_trace_index(&self, value: i64) -> crate::Result<()> { self.0.set_integer(9, value) }
    pub fn set_selected_trace_key(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(10, value) }
    pub fn set_trace_sort_direction(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(11, value) }
    pub fn set_file_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(12, value) }
    pub fn set_drop_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(13, value) }
    pub fn set_activation_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(14, value) }
    pub fn set_log_window_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(15, value) }
    pub fn set_show_trace_details(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(16, value) }
    pub fn set_clipboard_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(17, value) }
    pub fn add_items(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(1, value) }
    pub fn insert_items(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(1, index, value) }
    pub fn replace_items(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(1, index, value) }
    pub fn add_selected_files(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(4, value) }
    pub fn insert_selected_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(4, index, value) }
    pub fn replace_selected_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(4, index, value) }
    pub fn add_recent_files(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(7, value) }
    pub fn insert_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(7, index, value) }
    pub fn replace_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(7, index, value) }
    pub fn add_tasks(&self, value: impl TaskItemViewModel) -> crate::Result<()> { self.0.add_model(2, TaskItemViewModelDispatch { model: value }) }
    pub fn insert_tasks(&self, index: i32, value: impl TaskItemViewModel) -> crate::Result<()> { self.0.insert_model(2, index, TaskItemViewModelDispatch { model: value }) }
    pub fn replace_tasks(&self, index: i32, value: impl TaskItemViewModel) -> crate::Result<()> { self.0.replace_model(2, index, TaskItemViewModelDispatch { model: value }) }
    pub fn add_trace_rows(&self, value: impl TraceRowViewModel) -> crate::Result<()> { self.0.add_model(3, TraceRowViewModelDispatch { model: value }) }
    pub fn insert_trace_rows(&self, index: i32, value: impl TraceRowViewModel) -> crate::Result<()> { self.0.insert_model(3, index, TraceRowViewModelDispatch { model: value }) }
    pub fn replace_trace_rows(&self, index: i32, value: impl TraceRowViewModel) -> crate::Result<()> { self.0.replace_model(3, index, TraceRowViewModelDispatch { model: value }) }
    pub fn add_log_tree(&self, value: impl LogNodeViewModel) -> crate::Result<()> { self.0.add_model(6, LogNodeViewModelDispatch { model: value }) }
    pub fn insert_log_tree(&self, index: i32, value: impl LogNodeViewModel) -> crate::Result<()> { self.0.insert_model(6, index, LogNodeViewModelDispatch { model: value }) }
    pub fn replace_log_tree(&self, index: i32, value: impl LogNodeViewModel) -> crate::Result<()> { self.0.replace_model(6, index, LogNodeViewModelDispatch { model: value }) }
    pub fn add_core_loads(&self, value: f64) -> crate::Result<()> { self.0.add_double(8, value) }
    pub fn insert_core_loads(&self, index: i32, value: f64) -> crate::Result<()> { self.0.insert_double(8, index, value) }
    pub fn replace_core_loads(&self, index: i32, value: f64) -> crate::Result<()> { self.0.replace_double(8, index, value) }
    pub fn add_core_ticks(&self, value: i64) -> crate::Result<()> { self.0.add_integer(9, value) }
    pub fn insert_core_ticks(&self, index: i32, value: i64) -> crate::Result<()> { self.0.insert_integer(9, index, value) }
    pub fn replace_core_ticks(&self, index: i32, value: i64) -> crate::Result<()> { self.0.replace_integer(9, index, value) }
    /// True when the attached host implements the scalar-number sink capability.
    /// The reflectable (dynamic-binding) adapter deliberately does not.
    pub fn supports_number_collections(&self) -> bool { self.0.supports_number_collections() }
    pub fn remove_items(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(1, index) }
    pub fn move_items(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(1, from_index, to_index) }
    pub fn clear_items(&self) -> crate::Result<()> { self.0.clear_string_collection(1) }
    pub fn remove_selected_files(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(4, index) }
    pub fn move_selected_files(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(4, from_index, to_index) }
    pub fn clear_selected_files(&self) -> crate::Result<()> { self.0.clear_string_collection(4) }
    pub fn remove_recent_files(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(7, index) }
    pub fn move_recent_files(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(7, from_index, to_index) }
    pub fn clear_recent_files(&self) -> crate::Result<()> { self.0.clear_string_collection(7) }
    pub fn remove_tasks(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(2, index) }
    pub fn move_tasks(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(2, from_index, to_index) }
    pub fn clear_tasks(&self) -> crate::Result<()> { self.0.clear_model_collection(2) }
    pub fn remove_trace_rows(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(3, index) }
    pub fn move_trace_rows(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(3, from_index, to_index) }
    pub fn clear_trace_rows(&self) -> crate::Result<()> { self.0.clear_model_collection(3) }
    pub fn remove_log_tree(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(6, index) }
    pub fn move_log_tree(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(6, from_index, to_index) }
    pub fn clear_log_tree(&self) -> crate::Result<()> { self.0.clear_model_collection(6) }
    pub fn remove_core_loads(&self, index: i32) -> crate::Result<()> { self.0.remove_number_at(8, index) }
    pub fn move_core_loads(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_number_item(8, from_index, to_index) }
    pub fn clear_core_loads(&self) -> crate::Result<()> { self.0.clear_number_collection(8) }
    pub fn remove_core_ticks(&self, index: i32) -> crate::Result<()> { self.0.remove_number_at(9, index) }
    pub fn move_core_ticks(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_number_item(9, from_index, to_index) }
    pub fn clear_core_ticks(&self) -> crate::Result<()> { self.0.clear_number_collection(9) }
    pub fn set_increment_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_add_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_save_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_clear_nickname_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(4, enabled) }
    pub fn set_toggle_address_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(5, enabled) }
    pub fn set_add_task_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(6, enabled) }
    pub fn set_remove_first_task_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(7, enabled) }
    pub fn set_shuffle_tasks_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(8, enabled) }
    pub fn set_clear_tasks_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(9, enabled) }
    pub fn set_sort_trace_rows_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(10, enabled) }
    pub fn set_open_files_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(11, enabled) }
    pub fn set_open_folder_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(12, enabled) }
    pub fn set_save_export_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(13, enabled) }
    pub fn set_refresh_log_window_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(14, enabled) }
    pub fn set_open_recent_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(15, enabled) }
    pub fn set_copy_selected_row_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(16, enabled) }
    pub fn set_cut_selected_row_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(17, enabled) }
    pub fn set_paste_from_clipboard_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(18, enabled) }
    pub fn set_clear_clipboard_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(19, enabled) }
    pub fn set_exit_application_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(20, enabled) }
    /// Publishes the most-recently-used storage URIs into `RecentFiles`.
    ///
    /// The list is bounded by its own capacity (8), so this replaces a
    /// handful of entries rather than a data set; the generated menu derives
    /// each header from the URI and passes the URI back as the command parameter.
    pub fn publish_recent_files(&self, recent: &crate::RecentFileList) -> crate::Result<()> {
        self.0.clear_string_collection(7)?;
        for uri in recent.entries() { self.0.add_string(7, uri)?; }
        Ok(())
    }
    pub fn set_name_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_count_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_new_item_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_nickname_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_priority_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_new_task_title_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_selected_trace_index_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_selected_trace_key_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    pub fn set_trace_sort_direction_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(11, message) }
    pub fn set_file_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(12, message) }
    pub fn set_drop_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(13, message) }
    pub fn set_activation_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(14, message) }
    pub fn set_log_window_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(15, message) }
    pub fn set_show_trace_details_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(16, message) }
    pub fn set_clipboard_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(17, message) }
    /// True when the attached host implements the stage 30 sink capability.
    /// The reflectable (dynamic-binding) adapter deliberately does not.
    pub fn supports_richer_shapes(&self) -> bool { self.0.supports_richer_shapes() }
    pub fn set_severity_counts(&self, key: impl Into<crate::MapKey>, value: i64) -> crate::Result<()> { self.0.map_set_integer(1, key.into(), value) }
    pub fn remove_severity_counts(&self, key: impl Into<crate::MapKey>) -> crate::Result<()> { self.0.map_remove(1, key.into()) }
    pub fn clear_severity_counts(&self) -> crate::Result<()> { self.0.map_clear(1) }
    pub fn set_source_details(&self, key: impl Into<crate::MapKey>, value: impl TraceEventViewModel) -> crate::Result<()> { self.0.map_set_model(2, key.into(), TraceEventViewModelDispatch { model: value }) }
    pub fn remove_source_details(&self, key: impl Into<crate::MapKey>) -> crate::Result<()> { self.0.map_remove(2, key.into()) }
    pub fn clear_source_details(&self) -> crate::Result<()> { self.0.map_clear(2) }
    pub fn set_save_progress(&self, value: Option<f64>, message: Option<&str>) -> crate::Result<()> { self.0.set_command_progress(3, value, message) }
    pub fn set_save_running(&self, running: bool) -> crate::Result<()> { self.0.set_command_running(3, running) }
    /// Claims the single terminal transition of one `Save` invocation.
    /// Returns false when success, failure or cancellation already claimed it.
    pub fn claim_save_completion(&self, token: &crate::CancellationToken) -> bool { self.0.claim_completion(3, token) }
    pub fn set_save_result(&self, value: impl SaveReportViewModel) -> crate::Result<()> { self.0.set_command_result(3, Some(SaveReportViewModelDispatch { model: value })) }
    pub fn clear_save_result(&self) -> crate::Result<()> { self.0.clear_command_result(3) }
    /// Republishes `LogWindow`'s dataset identity, invalidating every realized page.
    pub fn reset_log_window(&self, generation: i64, total_count: i64) -> crate::Result<()> { self.0.publish_range_reset(5, generation, total_count) }
    /// Starts a page for `LogWindow` at the currently published generation.
    pub fn log_window_page(&self, offset: i64) -> Option<crate::RangeBatch> { self.0.range_batch(5, offset) }
    pub fn push_log_window_row(&self, page: &mut crate::RangeBatch, value: impl TraceRowViewModel) { self.0.push_range_model(page, TraceRowViewModelDispatch { model: value }); }
    pub fn publish_log_window_page(&self, page: crate::RangeBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.publish_range(page) }
    /// Re-requests realized `LogWindow` pages at the current generation (live values, no adapter churn).
    pub fn refresh_log_window(&self) -> crate::Result<()> { self.0.publish_range_invalidate(5) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> SampleViewModelSinkBatch { SampleViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: SampleViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct SampleViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl SampleViewModelSinkBatch {
    pub fn set_name(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_name_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_name_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_count(&mut self, value: i64) { self.0.push_integer(2, value); }
    pub fn set_count_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_count_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_new_item(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_new_item_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_new_item_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 4, 0, value); }
    pub fn set_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_status_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_nickname(&mut self, value: Option<impl AsRef<str>>) { match value { Some(value) => self.0.push_string(1, 5, 0, value), None => self.0.push_null(5) } }
    pub fn set_nickname_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_nickname_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_priority(&mut self, value: Priority) { self.0.push_integer(6, value as i64); }
    pub fn set_priority_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_priority_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_address(&mut self, value: impl AddressViewModel) { self.0.push_model(6, 7, 0, AddressViewModelDispatch { model: value }); }
    pub fn clear_address(&mut self) { self.0.push_model_null(7); }
    pub fn set_address_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_address_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_new_task_title(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 8, 0, value); }
    pub fn set_new_task_title_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_new_task_title_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_selected_trace_index(&mut self, value: i64) { self.0.push_integer(9, value); }
    pub fn set_selected_trace_index_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_selected_trace_index_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_selected_trace_key(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 10, 0, value); }
    pub fn set_selected_trace_key_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_selected_trace_key_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_trace_sort_direction(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 11, 0, value); }
    pub fn set_trace_sort_direction_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 11, 0, message); }
    pub fn clear_trace_sort_direction_error(&mut self) { self.0.push_clear_error(11); }
    pub fn set_file_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 12, 0, value); }
    pub fn set_file_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 12, 0, message); }
    pub fn clear_file_status_error(&mut self) { self.0.push_clear_error(12); }
    pub fn set_drop_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 13, 0, value); }
    pub fn set_drop_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 13, 0, message); }
    pub fn clear_drop_status_error(&mut self) { self.0.push_clear_error(13); }
    pub fn set_activation_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 14, 0, value); }
    pub fn set_activation_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 14, 0, message); }
    pub fn clear_activation_status_error(&mut self) { self.0.push_clear_error(14); }
    pub fn set_log_window_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 15, 0, value); }
    pub fn set_log_window_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 15, 0, message); }
    pub fn clear_log_window_status_error(&mut self) { self.0.push_clear_error(15); }
    pub fn set_show_trace_details(&mut self, value: bool) { self.0.push_boolean(3, 16, value); }
    pub fn set_show_trace_details_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 16, 0, message); }
    pub fn clear_show_trace_details_error(&mut self) { self.0.push_clear_error(16); }
    pub fn set_clipboard_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 17, 0, value); }
    pub fn set_clipboard_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 17, 0, message); }
    pub fn clear_clipboard_status_error(&mut self) { self.0.push_clear_error(17); }
    pub fn add_items(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 1, 0, value); }
    pub fn insert_items(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 1, index, value); }
    pub fn replace_items(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 1, index, value); }
    pub fn replace_items_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(1, values); }
    pub fn remove_items(&mut self, index: i32) { self.0.push_indices(13, 1, index, 0); }
    pub fn move_items(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 1, from_index, to_index); }
    pub fn clear_items(&mut self) { self.0.push_indices(19, 1, 0, 0); }
    pub fn add_selected_files(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 4, 0, value); }
    pub fn insert_selected_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 4, index, value); }
    pub fn replace_selected_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 4, index, value); }
    pub fn replace_selected_files_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(4, values); }
    pub fn remove_selected_files(&mut self, index: i32) { self.0.push_indices(13, 4, index, 0); }
    pub fn move_selected_files(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 4, from_index, to_index); }
    pub fn clear_selected_files(&mut self) { self.0.push_indices(19, 4, 0, 0); }
    pub fn add_recent_files(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 7, 0, value); }
    pub fn insert_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 7, index, value); }
    pub fn replace_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 7, index, value); }
    pub fn replace_recent_files_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(7, values); }
    pub fn remove_recent_files(&mut self, index: i32) { self.0.push_indices(13, 7, index, 0); }
    pub fn move_recent_files(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 7, from_index, to_index); }
    pub fn clear_recent_files(&mut self) { self.0.push_indices(19, 7, 0, 0); }
    pub fn add_tasks(&mut self, value: impl TaskItemViewModel) { self.0.push_model(8, 2, 0, TaskItemViewModelDispatch { model: value }); }
    pub fn insert_tasks(&mut self, index: i32, value: impl TaskItemViewModel) { self.0.push_model(10, 2, index, TaskItemViewModelDispatch { model: value }); }
    pub fn replace_tasks(&mut self, index: i32, value: impl TaskItemViewModel) { self.0.push_model(12, 2, index, TaskItemViewModelDispatch { model: value }); }
    pub fn replace_tasks_snapshot<M: TaskItemViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(2, values.into_iter().map(|value| TaskItemViewModelDispatch { model: value })); }
    pub fn remove_tasks(&mut self, index: i32) { self.0.push_model_indices(13, 2, index, 0); }
    pub fn move_tasks(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 2, from_index, to_index); }
    pub fn clear_tasks(&mut self) { self.0.push_model_clear(2); }
    pub fn add_trace_rows(&mut self, value: impl TraceRowViewModel) { self.0.push_model(8, 3, 0, TraceRowViewModelDispatch { model: value }); }
    pub fn insert_trace_rows(&mut self, index: i32, value: impl TraceRowViewModel) { self.0.push_model(10, 3, index, TraceRowViewModelDispatch { model: value }); }
    pub fn replace_trace_rows(&mut self, index: i32, value: impl TraceRowViewModel) { self.0.push_model(12, 3, index, TraceRowViewModelDispatch { model: value }); }
    pub fn replace_trace_rows_snapshot<M: TraceRowViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(3, values.into_iter().map(|value| TraceRowViewModelDispatch { model: value })); }
    pub fn remove_trace_rows(&mut self, index: i32) { self.0.push_model_indices(13, 3, index, 0); }
    pub fn move_trace_rows(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 3, from_index, to_index); }
    pub fn clear_trace_rows(&mut self) { self.0.push_model_clear(3); }
    pub fn add_log_tree(&mut self, value: impl LogNodeViewModel) { self.0.push_model(8, 6, 0, LogNodeViewModelDispatch { model: value }); }
    pub fn insert_log_tree(&mut self, index: i32, value: impl LogNodeViewModel) { self.0.push_model(10, 6, index, LogNodeViewModelDispatch { model: value }); }
    pub fn replace_log_tree(&mut self, index: i32, value: impl LogNodeViewModel) { self.0.push_model(12, 6, index, LogNodeViewModelDispatch { model: value }); }
    pub fn replace_log_tree_snapshot<M: LogNodeViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(6, values.into_iter().map(|value| LogNodeViewModelDispatch { model: value })); }
    pub fn remove_log_tree(&mut self, index: i32) { self.0.push_model_indices(13, 6, index, 0); }
    pub fn move_log_tree(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 6, from_index, to_index); }
    pub fn clear_log_tree(&mut self) { self.0.push_model_clear(6); }
    pub fn set_increment_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_add_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_save_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
    pub fn set_clear_nickname_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 4, enabled); }
    pub fn set_toggle_address_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 5, enabled); }
    pub fn set_add_task_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 6, enabled); }
    pub fn set_remove_first_task_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 7, enabled); }
    pub fn set_shuffle_tasks_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 8, enabled); }
    pub fn set_clear_tasks_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 9, enabled); }
    pub fn set_sort_trace_rows_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 10, enabled); }
    pub fn set_open_files_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 11, enabled); }
    pub fn set_open_folder_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 12, enabled); }
    pub fn set_save_export_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 13, enabled); }
    pub fn set_refresh_log_window_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 14, enabled); }
    pub fn set_open_recent_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 15, enabled); }
    pub fn set_copy_selected_row_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 16, enabled); }
    pub fn set_cut_selected_row_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 17, enabled); }
    pub fn set_paste_from_clipboard_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 18, enabled); }
    pub fn set_clear_clipboard_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 19, enabled); }
    pub fn set_exit_application_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 20, enabled); }
    /// Stages the most-recently-used storage URIs as one `RecentFiles` snapshot.
    pub fn set_recent_files(&mut self, recent: &crate::RecentFileList) { self.0.push_string_snapshot(7, recent.entries()); }
}

pub trait SampleViewModel: Send + 'static {
    fn attach(&mut self, sink: SampleViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_name(&mut self, value: String) -> crate::Result<()>;
    fn set_new_item(&mut self, value: String) -> crate::Result<()>;
    fn set_nickname(&mut self, value: String) -> crate::Result<()>;
    fn set_priority(&mut self, value: Priority) -> crate::Result<()>;
    fn set_new_task_title(&mut self, value: String) -> crate::Result<()>;
    fn set_selected_trace_index(&mut self, value: i64) -> crate::Result<()>;
    fn set_selected_trace_key(&mut self, value: String) -> crate::Result<()>;
    fn set_show_trace_details(&mut self, value: bool) -> crate::Result<()>;
    fn increment(&mut self) -> crate::Result<()>;
    fn add(&mut self, value: String) -> crate::Result<()>;
    fn save(&mut self, token: crate::CancellationToken) -> crate::Result<()>;
    fn clear_nickname(&mut self) -> crate::Result<()>;
    fn toggle_address(&mut self) -> crate::Result<()>;
    fn add_task(&mut self, value: String) -> crate::Result<()>;
    fn remove_first_task(&mut self) -> crate::Result<()>;
    fn shuffle_tasks(&mut self) -> crate::Result<()>;
    fn clear_tasks(&mut self) -> crate::Result<()>;
    fn sort_trace_rows(&mut self, value: String) -> crate::Result<()>;
    fn open_files(&mut self) -> crate::Result<()>;
    fn open_folder(&mut self) -> crate::Result<()>;
    fn save_export(&mut self) -> crate::Result<()>;
    fn refresh_log_window(&mut self) -> crate::Result<()>;
    fn open_recent_file(&mut self, value: String) -> crate::Result<()>;
    fn copy_selected_row(&mut self) -> crate::Result<()>;
    fn cut_selected_row(&mut self) -> crate::Result<()>;
    fn paste_from_clipboard(&mut self) -> crate::Result<()>;
    fn clear_clipboard(&mut self) -> crate::Result<()>;
    fn exit_application(&mut self) -> crate::Result<()>;
    /// Realizes one page of `LogWindow`. Called on the runtime's dedicated
    /// range thread, never on the UI thread, so it may take as long as the dataset needs.
    fn request_log_window_range(&mut self, request: crate::RangeRequest) -> crate::Result<()>;
}

struct SampleViewModelDispatch<T: SampleViewModel> { model: T }

impl<T: SampleViewModel> crate::view_model::DynamicViewModel for SampleViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(SampleViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, value: String) -> crate::Result<()> {
        match property_id {
            1 => self.model.set_name(value),
            3 => self.model.set_new_item(value),
            5 => self.model.set_nickname(value),
            8 => self.model.set_new_task_title(value),
            10 => self.model.set_selected_trace_key(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_integer(&mut self, property_id: i32, value: i64) -> crate::Result<()> {
        match property_id {
            6 => self.model.set_priority(Priority::try_from(value).map_err(|_| crate::Error::InvalidViewModelMember { kind: "property", id: property_id })?),
            9 => self.model.set_selected_trace_index(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_boolean(&mut self, property_id: i32, value: bool) -> crate::Result<()> {
        match property_id {
            16 => self.model.set_show_trace_details(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.increment(),
            2 => self.model.add(parameter.unwrap_or_default()),
            4 => self.model.clear_nickname(),
            5 => self.model.toggle_address(),
            6 => self.model.add_task(parameter.unwrap_or_default()),
            7 => self.model.remove_first_task(),
            8 => self.model.shuffle_tasks(),
            9 => self.model.clear_tasks(),
            10 => self.model.sort_trace_rows(parameter.unwrap_or_default()),
            14 => self.model.refresh_log_window(),
            15 => self.model.open_recent_file(parameter.unwrap_or_default()),
            20 => self.model.exit_application(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            3 => self.model.save(crate::CancellationToken::none()),
            11 => self.model.open_files(),
            12 => self.model.open_folder(),
            13 => self.model.save_export(),
            16 => self.model.copy_selected_row(),
            17 => self.model.cut_selected_row(),
            18 => self.model.paste_from_clipboard(),
            19 => self.model.clear_clipboard(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async_tracked(&mut self, command_id: i32, parameter: Option<String>, token: crate::CancellationToken) -> crate::Result<()> {
        match command_id {
            3 => self.model.save(token),
            _ => self.begin_async(command_id, parameter),
        }
    }
    fn request_range(&mut self, request: crate::RangeRequest) -> crate::Result<()> {
        match request.collection_id {
            5 => self.model.request_log_window_range(request),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "collection", id: request.collection_id }),
        }
    }
}

pub fn mount_rust_vm_window(scope: &crate::AppScope, model: impl SampleViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model(1, SampleViewModelDispatch { model }) }
pub fn mount_rust_vm_window_with_converters<C: ValueConverters>(
    scope: &crate::AppScope,
        model: impl SampleViewModel,
        converters: C,
    ) -> crate::Result<()> {
        register_value_converters(scope, converters)?;
        mount_rust_vm_window(scope, model)
    }
pub fn mount_rust_dynamic_vm_window(scope: &crate::AppScope, model: impl SampleViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model(2, SampleViewModelDispatch { model }) }
pub fn mount_rust_dynamic_vm_window_with_converters<C: ValueConverters>(
    scope: &crate::AppScope,
        model: impl SampleViewModel,
        converters: C,
    ) -> crate::Result<()> {
        register_value_converters(scope, converters)?;
        mount_rust_dynamic_vm_window(scope, model)
    }

#[derive(Clone, Debug)]
pub struct AddressViewModelSink(crate::view_model::ViewModelSink);

impl AddressViewModelSink {
    pub fn set_street(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_city(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_street_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_city_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> AddressViewModelSinkBatch { AddressViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: AddressViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct AddressViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl AddressViewModelSinkBatch {
    pub fn set_street(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_street_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_street_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_city(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_city_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_city_error(&mut self) { self.0.push_clear_error(2); }
}

pub trait AddressViewModel: Send + 'static {
    fn attach(&mut self, sink: AddressViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_street(&mut self, value: String) -> crate::Result<()>;
    fn set_city(&mut self, value: String) -> crate::Result<()>;
}

struct AddressViewModelDispatch<T: AddressViewModel> { model: T }

impl<T: AddressViewModel> crate::view_model::DynamicViewModel for AddressViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(AddressViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, value: String) -> crate::Result<()> {
        match property_id {
            1 => self.model.set_street(value),
            2 => self.model.set_city(value),
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
    fn execute(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}


#[derive(Clone, Debug)]
pub struct TaskItemViewModelSink(crate::view_model::ViewModelSink);

impl TaskItemViewModelSink {
    pub fn set_title(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_done(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(2, value) }
    pub fn set_title_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_done_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> TaskItemViewModelSinkBatch { TaskItemViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: TaskItemViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct TaskItemViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl TaskItemViewModelSinkBatch {
    pub fn set_title(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_title_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_title_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_done(&mut self, value: bool) { self.0.push_boolean(3, 2, value); }
    pub fn set_done_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_done_error(&mut self) { self.0.push_clear_error(2); }
}

pub trait TaskItemViewModel: Send + 'static {
    fn attach(&mut self, sink: TaskItemViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_done(&mut self, value: bool) -> crate::Result<()>;
}

struct TaskItemViewModelDispatch<T: TaskItemViewModel> { model: T }

impl<T: TaskItemViewModel> crate::view_model::DynamicViewModel for TaskItemViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(TaskItemViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, _value: String) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, value: bool) -> crate::Result<()> {
        match property_id {
            2 => self.model.set_done(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}


#[derive(Clone, Debug)]
pub struct TraceRowViewModelSink(crate::view_model::ViewModelSink);

impl TraceRowViewModelSink {
    pub fn set_timestamp(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_severity(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_message(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_event<M: TraceEventViewModel>(&self, value: Option<M>) -> crate::Result<()> { self.0.set_model(4, value.map(|model| TraceEventViewModelDispatch { model })) }
    pub fn set_timestamp_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_severity_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_message_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> TraceRowViewModelSinkBatch { TraceRowViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: TraceRowViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct TraceRowViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl TraceRowViewModelSinkBatch {
    pub fn set_timestamp(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_timestamp_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_timestamp_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_severity(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_severity_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_severity_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_message(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_message_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_message_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_event(&mut self, value: impl TraceEventViewModel) { self.0.push_model(6, 4, 0, TraceEventViewModelDispatch { model: value }); }
    pub fn clear_event(&mut self) { self.0.push_model_null(4); }
    pub fn set_event_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_event_error(&mut self) { self.0.push_clear_error(4); }
}

pub trait TraceRowViewModel: Send + 'static {
    fn attach(&mut self, sink: TraceRowViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
}

struct TraceRowViewModelDispatch<T: TraceRowViewModel> { model: T }

impl<T: TraceRowViewModel> crate::view_model::DynamicViewModel for TraceRowViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(TraceRowViewModelSink(sink)) }
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
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}


#[derive(Clone, Debug)]
pub struct TraceEventViewModelSink(crate::view_model::ViewModelSink);

impl TraceEventViewModelSink {
    pub fn set_id(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_source(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_id_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_source_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> TraceEventViewModelSinkBatch { TraceEventViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: TraceEventViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct TraceEventViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl TraceEventViewModelSinkBatch {
    pub fn set_id(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_id_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_id_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_source(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_source_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_source_error(&mut self) { self.0.push_clear_error(2); }
}

pub trait TraceEventViewModel: Send + 'static {
    fn attach(&mut self, sink: TraceEventViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
}

struct TraceEventViewModelDispatch<T: TraceEventViewModel> { model: T }

impl<T: TraceEventViewModel> crate::view_model::DynamicViewModel for TraceEventViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(TraceEventViewModelSink(sink)) }
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
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}


#[derive(Clone, Debug)]
pub struct LogNodeViewModelSink(crate::view_model::ViewModelSink);

impl LogNodeViewModelSink {
    pub fn set_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_detail(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_has_children(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(3, value) }
    pub fn add_children(&self, value: impl LogNodeViewModel) -> crate::Result<()> { self.0.add_model(1, LogNodeViewModelDispatch { model: value }) }
    pub fn insert_children(&self, index: i32, value: impl LogNodeViewModel) -> crate::Result<()> { self.0.insert_model(1, index, LogNodeViewModelDispatch { model: value }) }
    pub fn replace_children(&self, index: i32, value: impl LogNodeViewModel) -> crate::Result<()> { self.0.replace_model(1, index, LogNodeViewModelDispatch { model: value }) }
    pub fn remove_children(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(1, index) }
    pub fn move_children(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(1, from_index, to_index) }
    pub fn clear_children(&self) -> crate::Result<()> { self.0.clear_model_collection(1) }
    pub fn set_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_detail_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_has_children_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> LogNodeViewModelSinkBatch { LogNodeViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: LogNodeViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct LogNodeViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl LogNodeViewModelSinkBatch {
    pub fn set_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_label_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_detail(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_detail_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_detail_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_has_children(&mut self, value: bool) { self.0.push_boolean(3, 3, value); }
    pub fn set_has_children_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_has_children_error(&mut self) { self.0.push_clear_error(3); }
    pub fn add_children(&mut self, value: impl LogNodeViewModel) { self.0.push_model(8, 1, 0, LogNodeViewModelDispatch { model: value }); }
    pub fn insert_children(&mut self, index: i32, value: impl LogNodeViewModel) { self.0.push_model(10, 1, index, LogNodeViewModelDispatch { model: value }); }
    pub fn replace_children(&mut self, index: i32, value: impl LogNodeViewModel) { self.0.push_model(12, 1, index, LogNodeViewModelDispatch { model: value }); }
    pub fn replace_children_snapshot<M: LogNodeViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(1, values.into_iter().map(|value| LogNodeViewModelDispatch { model: value })); }
    pub fn remove_children(&mut self, index: i32) { self.0.push_model_indices(13, 1, index, 0); }
    pub fn move_children(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 1, from_index, to_index); }
    pub fn clear_children(&mut self) { self.0.push_model_clear(1); }
}

pub trait LogNodeViewModel: Send + 'static {
    fn attach(&mut self, sink: LogNodeViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
}

struct LogNodeViewModelDispatch<T: LogNodeViewModel> { model: T }

impl<T: LogNodeViewModel> crate::view_model::DynamicViewModel for LogNodeViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(LogNodeViewModelSink(sink)) }
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
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}


#[derive(Clone, Debug)]
pub struct SaveReportViewModelSink(crate::view_model::ViewModelSink);

impl SaveReportViewModelSink {
    pub fn set_destination(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_bytes(&self, value: i64) -> crate::Result<()> { self.0.set_integer(2, value) }
    pub fn set_succeeded(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(3, value) }
    pub fn set_destination_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_bytes_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_succeeded_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> SaveReportViewModelSinkBatch { SaveReportViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: SaveReportViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct SaveReportViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl SaveReportViewModelSinkBatch {
    pub fn set_destination(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_destination_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_destination_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_bytes(&mut self, value: i64) { self.0.push_integer(2, value); }
    pub fn set_bytes_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_bytes_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_succeeded(&mut self, value: bool) { self.0.push_boolean(3, 3, value); }
    pub fn set_succeeded_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_succeeded_error(&mut self) { self.0.push_clear_error(3); }
}

pub trait SaveReportViewModel: Send + 'static {
    fn attach(&mut self, sink: SaveReportViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
}

struct SaveReportViewModelDispatch<T: SaveReportViewModel> { model: T }

impl<T: SaveReportViewModel> crate::view_model::DynamicViewModel for SaveReportViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(SaveReportViewModelSink(sink)) }
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
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}


pub trait ValueConverters: Send + Sync + 'static {
    fn count_to_label(&self, value: i64) -> String;
}

struct ValueConvertersDispatch<T: ValueConverters> { converters: T }

impl<T: ValueConverters> crate::value_converter::ValueConverterDispatch for ValueConvertersDispatch<T> {
    fn convert(
        &self,
        converter_id: i32,
        direction: crate::ConversionDirection,
        value: crate::ScalarValue,
        _parameter: crate::ScalarValue,
        _target_kind: crate::ScalarKind,
        _culture: &str,
    ) -> crate::Result<crate::ScalarValue> {
        use crate::{ConversionDirection, ScalarValue};
        match (converter_id, direction) {
            (1, ConversionDirection::Convert) => {
                let value = match value { ScalarValue::Int64(value) => value, ScalarValue::Unset => return Ok(ScalarValue::Unset), ScalarValue::DoNothing => return Ok(ScalarValue::DoNothing), ScalarValue::Null => return Ok(ScalarValue::Null), _ => return Err(crate::Error::InvalidViewModelMember { kind: "converter-value", id: 1 }) };
                Ok(ScalarValue::String(self.converters.count_to_label(value)))
            }
            _ => Err(crate::Error::InvalidViewModelMember { kind: "converter", id: converter_id }),
        }
    }
}

pub fn register_value_converters(scope: &crate::AppScope, converters: impl ValueConverters) -> crate::Result<()> {
    scope.register_value_converter_dispatch(ValueConvertersDispatch { converters })
}

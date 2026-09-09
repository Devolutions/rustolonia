//! Generated from view-model.ir.json. Do not edit.

#![allow(dead_code)]

#[derive(Clone, Debug)]
pub struct MainViewModelSink(crate::view_model::ViewModelSink);

/// Declared capacity of the `MainViewModel` recent-file list.
pub const MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY: usize = 5;

impl MainViewModelSink {
    pub fn set_title(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_archive_name(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_archive_kind(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(4, value) }
    pub fn set_entry_count_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(5, value) }
    pub fn set_filter_text(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(6, value) }
    pub fn set_selected_index(&self, value: i64) -> crate::Result<()> { self.0.set_integer(7, value) }
    pub fn set_selected_key(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(8, value) }
    pub fn set_sort_direction(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(9, value) }
    pub fn set_is_loading(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(10, value) }
    pub fn set_can_extract_selected(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(11, value) }
    pub fn set_can_extract_all(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(12, value) }
    pub fn set_selected_count_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(13, value) }
    pub fn set_current_path(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(14, value) }
    pub fn set_can_go_up(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(15, value) }
    pub fn set_totals_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(16, value) }
    pub fn set_open_after_extract(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(17, value) }
    pub fn set_can_test(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(18, value) }
    pub fn add_recent_files(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(1, value) }
    pub fn insert_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(1, index, value) }
    pub fn replace_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(1, index, value) }
    pub fn add_entries(&self, value: impl EntryRowViewModel) -> crate::Result<()> { self.0.add_model(2, EntryRowViewModelDispatch { model: value }) }
    pub fn insert_entries(&self, index: i32, value: impl EntryRowViewModel) -> crate::Result<()> { self.0.insert_model(2, index, EntryRowViewModelDispatch { model: value }) }
    pub fn replace_entries(&self, index: i32, value: impl EntryRowViewModel) -> crate::Result<()> { self.0.replace_model(2, index, EntryRowViewModelDispatch { model: value }) }
    pub fn remove_recent_files(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(1, index) }
    pub fn move_recent_files(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(1, from_index, to_index) }
    pub fn clear_recent_files(&self) -> crate::Result<()> { self.0.clear_string_collection(1) }
    pub fn remove_entries(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(2, index) }
    pub fn move_entries(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(2, from_index, to_index) }
    pub fn clear_entries(&self) -> crate::Result<()> { self.0.clear_model_collection(2) }
    pub fn set_open_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_extract_selected_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_extract_all_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_select_all_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(4, enabled) }
    pub fn set_clear_selection_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(5, enabled) }
    pub fn set_sort_entries_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(6, enabled) }
    pub fn set_open_recent_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(7, enabled) }
    pub fn set_exit_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(8, enabled) }
    pub fn set_go_up_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(9, enabled) }
    pub fn set_open_item_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(10, enabled) }
    pub fn set_extract_here_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(11, enabled) }
    pub fn set_test_archive_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(12, enabled) }
    pub fn set_new_archive_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(13, enabled) }
    pub fn set_invert_selection_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(14, enabled) }
    pub fn set_copy_path_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(15, enabled) }
    /// Publishes the most-recently-used storage URIs into `RecentFiles`.
    ///
    /// The list is bounded by its own capacity (5), so this replaces a
    /// handful of entries rather than a data set; the generated menu derives
    /// each header from the URI and passes the URI back as the command parameter.
    pub fn publish_recent_files(&self, recent: &crate::RecentFileList) -> crate::Result<()> {
        self.0.clear_string_collection(1)?;
        for uri in recent.entries() { self.0.add_string(1, uri)?; }
        Ok(())
    }
    pub fn set_title_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_archive_name_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_archive_kind_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_entry_count_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_filter_text_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_selected_index_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_selected_key_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_sort_direction_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_is_loading_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    pub fn set_can_extract_selected_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(11, message) }
    pub fn set_can_extract_all_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(12, message) }
    pub fn set_selected_count_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(13, message) }
    pub fn set_current_path_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(14, message) }
    pub fn set_can_go_up_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(15, message) }
    pub fn set_totals_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(16, message) }
    pub fn set_open_after_extract_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(17, message) }
    pub fn set_can_test_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(18, message) }
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
    pub fn set_archive_name(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_archive_name_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_archive_name_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_archive_kind(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 4, 0, value); }
    pub fn set_archive_kind_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_archive_kind_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_entry_count_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 5, 0, value); }
    pub fn set_entry_count_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_entry_count_label_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_filter_text(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 6, 0, value); }
    pub fn set_filter_text_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_filter_text_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_selected_index(&mut self, value: i64) { self.0.push_integer(7, value); }
    pub fn set_selected_index_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_selected_index_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_selected_key(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 8, 0, value); }
    pub fn set_selected_key_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_selected_key_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_sort_direction(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 9, 0, value); }
    pub fn set_sort_direction_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_sort_direction_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_is_loading(&mut self, value: bool) { self.0.push_boolean(3, 10, value); }
    pub fn set_is_loading_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_is_loading_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_can_extract_selected(&mut self, value: bool) { self.0.push_boolean(3, 11, value); }
    pub fn set_can_extract_selected_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 11, 0, message); }
    pub fn clear_can_extract_selected_error(&mut self) { self.0.push_clear_error(11); }
    pub fn set_can_extract_all(&mut self, value: bool) { self.0.push_boolean(3, 12, value); }
    pub fn set_can_extract_all_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 12, 0, message); }
    pub fn clear_can_extract_all_error(&mut self) { self.0.push_clear_error(12); }
    pub fn set_selected_count_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 13, 0, value); }
    pub fn set_selected_count_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 13, 0, message); }
    pub fn clear_selected_count_label_error(&mut self) { self.0.push_clear_error(13); }
    pub fn set_current_path(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 14, 0, value); }
    pub fn set_current_path_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 14, 0, message); }
    pub fn clear_current_path_error(&mut self) { self.0.push_clear_error(14); }
    pub fn set_can_go_up(&mut self, value: bool) { self.0.push_boolean(3, 15, value); }
    pub fn set_can_go_up_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 15, 0, message); }
    pub fn clear_can_go_up_error(&mut self) { self.0.push_clear_error(15); }
    pub fn set_totals_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 16, 0, value); }
    pub fn set_totals_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 16, 0, message); }
    pub fn clear_totals_label_error(&mut self) { self.0.push_clear_error(16); }
    pub fn set_open_after_extract(&mut self, value: bool) { self.0.push_boolean(3, 17, value); }
    pub fn set_open_after_extract_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 17, 0, message); }
    pub fn clear_open_after_extract_error(&mut self) { self.0.push_clear_error(17); }
    pub fn set_can_test(&mut self, value: bool) { self.0.push_boolean(3, 18, value); }
    pub fn set_can_test_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 18, 0, message); }
    pub fn clear_can_test_error(&mut self) { self.0.push_clear_error(18); }
    pub fn add_recent_files(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 1, 0, value); }
    pub fn insert_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 1, index, value); }
    pub fn replace_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 1, index, value); }
    pub fn replace_recent_files_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(1, values); }
    pub fn remove_recent_files(&mut self, index: i32) { self.0.push_indices(13, 1, index, 0); }
    pub fn move_recent_files(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 1, from_index, to_index); }
    pub fn clear_recent_files(&mut self) { self.0.push_indices(19, 1, 0, 0); }
    pub fn add_entries(&mut self, value: impl EntryRowViewModel) { self.0.push_model(8, 2, 0, EntryRowViewModelDispatch { model: value }); }
    pub fn insert_entries(&mut self, index: i32, value: impl EntryRowViewModel) { self.0.push_model(10, 2, index, EntryRowViewModelDispatch { model: value }); }
    pub fn replace_entries(&mut self, index: i32, value: impl EntryRowViewModel) { self.0.push_model(12, 2, index, EntryRowViewModelDispatch { model: value }); }
    pub fn replace_entries_snapshot<M: EntryRowViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(2, values.into_iter().map(|value| EntryRowViewModelDispatch { model: value })); }
    pub fn remove_entries(&mut self, index: i32) { self.0.push_model_indices(13, 2, index, 0); }
    pub fn move_entries(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 2, from_index, to_index); }
    pub fn clear_entries(&mut self) { self.0.push_model_clear(2); }
    pub fn set_open_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_extract_selected_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_extract_all_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
    pub fn set_select_all_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 4, enabled); }
    pub fn set_clear_selection_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 5, enabled); }
    pub fn set_sort_entries_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 6, enabled); }
    pub fn set_open_recent_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 7, enabled); }
    pub fn set_exit_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 8, enabled); }
    pub fn set_go_up_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 9, enabled); }
    pub fn set_open_item_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 10, enabled); }
    pub fn set_extract_here_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 11, enabled); }
    pub fn set_test_archive_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 12, enabled); }
    pub fn set_new_archive_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 13, enabled); }
    pub fn set_invert_selection_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 14, enabled); }
    pub fn set_copy_path_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 15, enabled); }
    /// Stages the most-recently-used storage URIs as one `RecentFiles` snapshot.
    pub fn set_recent_files(&mut self, recent: &crate::RecentFileList) { self.0.push_string_snapshot(1, recent.entries()); }
}

pub trait MainViewModel: Send + 'static {
    fn attach(&mut self, sink: MainViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_filter_text(&mut self, value: String) -> crate::Result<()>;
    fn set_selected_index(&mut self, value: i64) -> crate::Result<()>;
    fn set_selected_key(&mut self, value: String) -> crate::Result<()>;
    fn set_open_after_extract(&mut self, value: bool) -> crate::Result<()>;
    fn open_file(&mut self) -> crate::Result<()>;
    fn extract_selected(&mut self) -> crate::Result<()>;
    fn extract_all(&mut self) -> crate::Result<()>;
    fn select_all(&mut self) -> crate::Result<()>;
    fn clear_selection(&mut self) -> crate::Result<()>;
    fn sort_entries(&mut self, value: String) -> crate::Result<()>;
    fn open_recent_file(&mut self, value: String) -> crate::Result<()>;
    fn exit(&mut self) -> crate::Result<()>;
    fn go_up(&mut self) -> crate::Result<()>;
    fn open_item(&mut self) -> crate::Result<()>;
    fn extract_here(&mut self) -> crate::Result<()>;
    fn test_archive(&mut self) -> crate::Result<()>;
    fn new_archive(&mut self) -> crate::Result<()>;
    fn invert_selection(&mut self) -> crate::Result<()>;
    fn copy_path(&mut self) -> crate::Result<()>;
}

struct MainViewModelDispatch<T: MainViewModel> { model: T }

impl<T: MainViewModel> crate::view_model::DynamicViewModel for MainViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(MainViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, value: String) -> crate::Result<()> {
        match property_id {
            6 => self.model.set_filter_text(value),
            8 => self.model.set_selected_key(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_integer(&mut self, property_id: i32, value: i64) -> crate::Result<()> {
        match property_id {
            7 => self.model.set_selected_index(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_boolean(&mut self, property_id: i32, value: bool) -> crate::Result<()> {
        match property_id {
            17 => self.model.set_open_after_extract(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            4 => self.model.select_all(),
            5 => self.model.clear_selection(),
            6 => self.model.sort_entries(parameter.unwrap_or_default()),
            7 => self.model.open_recent_file(parameter.unwrap_or_default()),
            8 => self.model.exit(),
            9 => self.model.go_up(),
            14 => self.model.invert_selection(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.open_file(),
            2 => self.model.extract_selected(),
            3 => self.model.extract_all(),
            10 => self.model.open_item(),
            11 => self.model.extract_here(),
            12 => self.model.test_archive(),
            13 => self.model.new_archive(),
            15 => self.model.copy_path(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
}

pub fn mount_main_window(scope: &crate::AppScope, model: impl MainViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model(1, MainViewModelDispatch { model }) }

#[derive(Clone, Debug)]
pub struct EntryRowViewModelSink(crate::view_model::ViewModelSink);

impl EntryRowViewModelSink {
    pub fn set_key(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_path(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_name(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_kind(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(4, value) }
    pub fn set_size_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(5, value) }
    pub fn set_packed_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(6, value) }
    pub fn set_modified(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(7, value) }
    pub fn set_is_selected(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(8, value) }
    pub fn set_crc_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(9, value) }
    pub fn set_ratio_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(10, value) }
    pub fn set_open_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_key_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_path_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_name_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_kind_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_size_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_packed_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_modified_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_is_selected_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_crc_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_ratio_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> EntryRowViewModelSinkBatch { EntryRowViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: EntryRowViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct EntryRowViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl EntryRowViewModelSinkBatch {
    pub fn set_key(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_key_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_key_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_path(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_path_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_path_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_name(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_name_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_name_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_kind(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 4, 0, value); }
    pub fn set_kind_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_kind_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_size_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 5, 0, value); }
    pub fn set_size_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_size_label_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_packed_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 6, 0, value); }
    pub fn set_packed_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_packed_label_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_modified(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 7, 0, value); }
    pub fn set_modified_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_modified_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_is_selected(&mut self, value: bool) { self.0.push_boolean(3, 8, value); }
    pub fn set_is_selected_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_is_selected_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_crc_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 9, 0, value); }
    pub fn set_crc_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_crc_label_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_ratio_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 10, 0, value); }
    pub fn set_ratio_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_ratio_label_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_open_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
}

pub trait EntryRowViewModel: Send + 'static {
    fn attach(&mut self, sink: EntryRowViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_is_selected(&mut self, value: bool) -> crate::Result<()>;
    fn open(&mut self) -> crate::Result<()>;
}

struct EntryRowViewModelDispatch<T: EntryRowViewModel> { model: T }

impl<T: EntryRowViewModel> crate::view_model::DynamicViewModel for EntryRowViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(EntryRowViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, _value: String) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, value: bool) -> crate::Result<()> {
        match property_id {
            8 => self.model.set_is_selected(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.open(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}

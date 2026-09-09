//! Generated from view-model.ir.json. Do not edit.

#![allow(dead_code)]

#[derive(Clone, Debug)]
pub struct MainViewModelSink(crate::view_model::ViewModelSink);

/// Declared capacity of the `MainViewModel` recent-file list.
pub const MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY: usize = 5;

impl MainViewModelSink {
    pub fn set_title(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_document_name(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_page_image(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(4, value) }
    pub fn set_page_text(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(5, value) }
    pub fn set_page_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(6, value) }
    pub fn set_can_go_previous(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(7, value) }
    pub fn set_can_go_next(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(8, value) }
    pub fn set_is_loading(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(9, value) }
    pub fn set_search_text(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(10, value) }
    pub fn set_search_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(11, value) }
    pub fn set_search_match_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(12, value) }
    pub fn set_can_go_previous_match(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(13, value) }
    pub fn set_can_go_next_match(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(14, value) }
    pub fn set_outline_visible(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(15, value) }
    pub fn set_has_outline(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(16, value) }
    pub fn set_outline_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(17, value) }
    pub fn add_recent_files(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(1, value) }
    pub fn insert_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(1, index, value) }
    pub fn replace_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(1, index, value) }
    pub fn add_outline(&self, value: impl OutlineItemViewModel) -> crate::Result<()> { self.0.add_model(2, OutlineItemViewModelDispatch { model: value }) }
    pub fn insert_outline(&self, index: i32, value: impl OutlineItemViewModel) -> crate::Result<()> { self.0.insert_model(2, index, OutlineItemViewModelDispatch { model: value }) }
    pub fn replace_outline(&self, index: i32, value: impl OutlineItemViewModel) -> crate::Result<()> { self.0.replace_model(2, index, OutlineItemViewModelDispatch { model: value }) }
    pub fn remove_recent_files(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(1, index) }
    pub fn move_recent_files(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(1, from_index, to_index) }
    pub fn clear_recent_files(&self) -> crate::Result<()> { self.0.clear_string_collection(1) }
    pub fn remove_outline(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(2, index) }
    pub fn move_outline(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(2, from_index, to_index) }
    pub fn clear_outline(&self) -> crate::Result<()> { self.0.clear_model_collection(2) }
    pub fn set_open_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_previous_page_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_next_page_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_open_recent_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(4, enabled) }
    pub fn set_exit_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(5, enabled) }
    pub fn set_search_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(6, enabled) }
    pub fn set_previous_match_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(7, enabled) }
    pub fn set_next_match_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(8, enabled) }
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
    pub fn set_document_name_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_page_image_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_page_text_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_page_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_can_go_previous_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_can_go_next_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_is_loading_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_search_text_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    pub fn set_search_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(11, message) }
    pub fn set_search_match_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(12, message) }
    pub fn set_can_go_previous_match_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(13, message) }
    pub fn set_can_go_next_match_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(14, message) }
    pub fn set_outline_visible_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(15, message) }
    pub fn set_has_outline_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(16, message) }
    pub fn set_outline_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(17, message) }
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
    pub fn set_document_name(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_document_name_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_document_name_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_page_image(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 4, 0, value); }
    pub fn set_page_image_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_page_image_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_page_text(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 5, 0, value); }
    pub fn set_page_text_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_page_text_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_page_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 6, 0, value); }
    pub fn set_page_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_page_label_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_can_go_previous(&mut self, value: bool) { self.0.push_boolean(3, 7, value); }
    pub fn set_can_go_previous_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_can_go_previous_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_can_go_next(&mut self, value: bool) { self.0.push_boolean(3, 8, value); }
    pub fn set_can_go_next_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_can_go_next_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_is_loading(&mut self, value: bool) { self.0.push_boolean(3, 9, value); }
    pub fn set_is_loading_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_is_loading_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_search_text(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 10, 0, value); }
    pub fn set_search_text_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_search_text_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_search_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 11, 0, value); }
    pub fn set_search_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 11, 0, message); }
    pub fn clear_search_status_error(&mut self) { self.0.push_clear_error(11); }
    pub fn set_search_match_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 12, 0, value); }
    pub fn set_search_match_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 12, 0, message); }
    pub fn clear_search_match_label_error(&mut self) { self.0.push_clear_error(12); }
    pub fn set_can_go_previous_match(&mut self, value: bool) { self.0.push_boolean(3, 13, value); }
    pub fn set_can_go_previous_match_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 13, 0, message); }
    pub fn clear_can_go_previous_match_error(&mut self) { self.0.push_clear_error(13); }
    pub fn set_can_go_next_match(&mut self, value: bool) { self.0.push_boolean(3, 14, value); }
    pub fn set_can_go_next_match_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 14, 0, message); }
    pub fn clear_can_go_next_match_error(&mut self) { self.0.push_clear_error(14); }
    pub fn set_outline_visible(&mut self, value: bool) { self.0.push_boolean(3, 15, value); }
    pub fn set_outline_visible_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 15, 0, message); }
    pub fn clear_outline_visible_error(&mut self) { self.0.push_clear_error(15); }
    pub fn set_has_outline(&mut self, value: bool) { self.0.push_boolean(3, 16, value); }
    pub fn set_has_outline_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 16, 0, message); }
    pub fn clear_has_outline_error(&mut self) { self.0.push_clear_error(16); }
    pub fn set_outline_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 17, 0, value); }
    pub fn set_outline_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 17, 0, message); }
    pub fn clear_outline_status_error(&mut self) { self.0.push_clear_error(17); }
    pub fn add_recent_files(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 1, 0, value); }
    pub fn insert_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 1, index, value); }
    pub fn replace_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 1, index, value); }
    pub fn replace_recent_files_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(1, values); }
    pub fn remove_recent_files(&mut self, index: i32) { self.0.push_indices(13, 1, index, 0); }
    pub fn move_recent_files(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 1, from_index, to_index); }
    pub fn clear_recent_files(&mut self) { self.0.push_indices(19, 1, 0, 0); }
    pub fn add_outline(&mut self, value: impl OutlineItemViewModel) { self.0.push_model(8, 2, 0, OutlineItemViewModelDispatch { model: value }); }
    pub fn insert_outline(&mut self, index: i32, value: impl OutlineItemViewModel) { self.0.push_model(10, 2, index, OutlineItemViewModelDispatch { model: value }); }
    pub fn replace_outline(&mut self, index: i32, value: impl OutlineItemViewModel) { self.0.push_model(12, 2, index, OutlineItemViewModelDispatch { model: value }); }
    pub fn replace_outline_snapshot<M: OutlineItemViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(2, values.into_iter().map(|value| OutlineItemViewModelDispatch { model: value })); }
    pub fn remove_outline(&mut self, index: i32) { self.0.push_model_indices(13, 2, index, 0); }
    pub fn move_outline(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 2, from_index, to_index); }
    pub fn clear_outline(&mut self) { self.0.push_model_clear(2); }
    pub fn set_open_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_previous_page_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_next_page_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
    pub fn set_open_recent_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 4, enabled); }
    pub fn set_exit_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 5, enabled); }
    pub fn set_search_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 6, enabled); }
    pub fn set_previous_match_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 7, enabled); }
    pub fn set_next_match_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 8, enabled); }
    /// Stages the most-recently-used storage URIs as one `RecentFiles` snapshot.
    pub fn set_recent_files(&mut self, recent: &crate::RecentFileList) { self.0.push_string_snapshot(1, recent.entries()); }
}

pub trait MainViewModel: Send + 'static {
    fn attach(&mut self, sink: MainViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_search_text(&mut self, value: String) -> crate::Result<()>;
    fn set_outline_visible(&mut self, value: bool) -> crate::Result<()>;
    fn open_file(&mut self) -> crate::Result<()>;
    fn previous_page(&mut self) -> crate::Result<()>;
    fn next_page(&mut self) -> crate::Result<()>;
    fn open_recent_file(&mut self, value: String) -> crate::Result<()>;
    fn exit(&mut self) -> crate::Result<()>;
    fn search(&mut self) -> crate::Result<()>;
    fn previous_match(&mut self) -> crate::Result<()>;
    fn next_match(&mut self) -> crate::Result<()>;
}

struct MainViewModelDispatch<T: MainViewModel> { model: T }

impl<T: MainViewModel> crate::view_model::DynamicViewModel for MainViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(MainViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, value: String) -> crate::Result<()> {
        match property_id {
            10 => self.model.set_search_text(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, value: bool) -> crate::Result<()> {
        match property_id {
            15 => self.model.set_outline_visible(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            2 => self.model.previous_page(),
            3 => self.model.next_page(),
            4 => self.model.open_recent_file(parameter.unwrap_or_default()),
            5 => self.model.exit(),
            7 => self.model.previous_match(),
            8 => self.model.next_match(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.open_file(),
            6 => self.model.search(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
}

pub fn mount_main_window(scope: &crate::AppScope, model: impl MainViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model(1, MainViewModelDispatch { model }) }

#[derive(Clone, Debug)]
pub struct OutlineItemViewModelSink(crate::view_model::ViewModelSink);

impl OutlineItemViewModelSink {
    pub fn set_title(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_page_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_has_children(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(3, value) }
    pub fn add_children(&self, value: impl OutlineItemViewModel) -> crate::Result<()> { self.0.add_model(1, OutlineItemViewModelDispatch { model: value }) }
    pub fn insert_children(&self, index: i32, value: impl OutlineItemViewModel) -> crate::Result<()> { self.0.insert_model(1, index, OutlineItemViewModelDispatch { model: value }) }
    pub fn replace_children(&self, index: i32, value: impl OutlineItemViewModel) -> crate::Result<()> { self.0.replace_model(1, index, OutlineItemViewModelDispatch { model: value }) }
    pub fn remove_children(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(1, index) }
    pub fn move_children(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(1, from_index, to_index) }
    pub fn clear_children(&self) -> crate::Result<()> { self.0.clear_model_collection(1) }
    pub fn set_go_to_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_title_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_page_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_has_children_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> OutlineItemViewModelSinkBatch { OutlineItemViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: OutlineItemViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct OutlineItemViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl OutlineItemViewModelSinkBatch {
    pub fn set_title(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_title_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_title_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_page_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_page_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_page_label_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_has_children(&mut self, value: bool) { self.0.push_boolean(3, 3, value); }
    pub fn set_has_children_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_has_children_error(&mut self) { self.0.push_clear_error(3); }
    pub fn add_children(&mut self, value: impl OutlineItemViewModel) { self.0.push_model(8, 1, 0, OutlineItemViewModelDispatch { model: value }); }
    pub fn insert_children(&mut self, index: i32, value: impl OutlineItemViewModel) { self.0.push_model(10, 1, index, OutlineItemViewModelDispatch { model: value }); }
    pub fn replace_children(&mut self, index: i32, value: impl OutlineItemViewModel) { self.0.push_model(12, 1, index, OutlineItemViewModelDispatch { model: value }); }
    pub fn replace_children_snapshot<M: OutlineItemViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(1, values.into_iter().map(|value| OutlineItemViewModelDispatch { model: value })); }
    pub fn remove_children(&mut self, index: i32) { self.0.push_model_indices(13, 1, index, 0); }
    pub fn move_children(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 1, from_index, to_index); }
    pub fn clear_children(&mut self) { self.0.push_model_clear(1); }
    pub fn set_go_to_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
}

pub trait OutlineItemViewModel: Send + 'static {
    fn attach(&mut self, sink: OutlineItemViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn go_to(&mut self) -> crate::Result<()>;
}

struct OutlineItemViewModelDispatch<T: OutlineItemViewModel> { model: T }

impl<T: OutlineItemViewModel> crate::view_model::DynamicViewModel for OutlineItemViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(OutlineItemViewModelSink(sink)) }
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
            1 => self.model.go_to(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}

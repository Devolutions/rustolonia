//! Generated from view-model.ir.json. Do not edit.

#![allow(dead_code)]

#[derive(Clone, Debug)]
pub struct MainViewModelSink(crate::view_model::ViewModelSink);

/// Declared capacity of the `MainViewModel` recent-file list.
pub const MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY: usize = 8;

impl MainViewModelSink {
    pub fn set_title(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_file_count_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_expected_hash(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(4, value) }
    pub fn set_compare_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(5, value) }
    pub fn set_include_md5(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(6, value) }
    pub fn set_include_sha1(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(7, value) }
    pub fn set_include_sha256(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(8, value) }
    pub fn set_include_sha512(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(9, value) }
    pub fn set_include_blake3(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(10, value) }
    pub fn set_is_busy(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(11, value) }
    pub fn set_can_clear(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(12, value) }
    pub fn set_can_copy_report(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(13, value) }
    pub fn set_primary_digest(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(14, value) }
    pub fn set_empty_visible(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(15, value) }
    pub fn add_recent_files(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.add_string(1, value) }
    pub fn insert_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.insert_string(1, index, value) }
    pub fn replace_recent_files(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> { self.0.replace_string(1, index, value) }
    pub fn add_files(&self, value: impl FileRowViewModel) -> crate::Result<()> { self.0.add_model(2, FileRowViewModelDispatch { model: value }) }
    pub fn insert_files(&self, index: i32, value: impl FileRowViewModel) -> crate::Result<()> { self.0.insert_model(2, index, FileRowViewModelDispatch { model: value }) }
    pub fn replace_files(&self, index: i32, value: impl FileRowViewModel) -> crate::Result<()> { self.0.replace_model(2, index, FileRowViewModelDispatch { model: value }) }
    pub fn remove_recent_files(&self, index: i32) -> crate::Result<()> { self.0.remove_string_at(1, index) }
    pub fn move_recent_files(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_string_item(1, from_index, to_index) }
    pub fn clear_recent_files(&self) -> crate::Result<()> { self.0.clear_string_collection(1) }
    pub fn remove_files(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(2, index) }
    pub fn move_files(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(2, from_index, to_index) }
    pub fn clear_files(&self) -> crate::Result<()> { self.0.clear_model_collection(2) }
    pub fn set_open_files_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_clear_files_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_copy_report_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_open_recent_file_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(4, enabled) }
    pub fn set_exit_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(5, enabled) }
    /// Publishes the most-recently-used storage URIs into `RecentFiles`.
    ///
    /// The list is bounded by its own capacity (8), so this replaces a
    /// handful of entries rather than a data set; the generated menu derives
    /// each header from the URI and passes the URI back as the command parameter.
    pub fn publish_recent_files(&self, recent: &crate::RecentFileList) -> crate::Result<()> {
        self.0.clear_string_collection(1)?;
        for uri in recent.entries() { self.0.add_string(1, uri)?; }
        Ok(())
    }
    pub fn set_title_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_file_count_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_expected_hash_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_compare_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_include_md5_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_include_sha1_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_include_sha256_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_include_sha512_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_include_blake3_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    pub fn set_is_busy_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(11, message) }
    pub fn set_can_clear_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(12, message) }
    pub fn set_can_copy_report_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(13, message) }
    pub fn set_primary_digest_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(14, message) }
    pub fn set_empty_visible_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(15, message) }
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
    pub fn set_file_count_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_file_count_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_file_count_label_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_expected_hash(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 4, 0, value); }
    pub fn set_expected_hash_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_expected_hash_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_compare_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 5, 0, value); }
    pub fn set_compare_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_compare_status_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_include_md5(&mut self, value: bool) { self.0.push_boolean(3, 6, value); }
    pub fn set_include_md5_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_include_md5_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_include_sha1(&mut self, value: bool) { self.0.push_boolean(3, 7, value); }
    pub fn set_include_sha1_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_include_sha1_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_include_sha256(&mut self, value: bool) { self.0.push_boolean(3, 8, value); }
    pub fn set_include_sha256_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_include_sha256_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_include_sha512(&mut self, value: bool) { self.0.push_boolean(3, 9, value); }
    pub fn set_include_sha512_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_include_sha512_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_include_blake3(&mut self, value: bool) { self.0.push_boolean(3, 10, value); }
    pub fn set_include_blake3_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_include_blake3_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_is_busy(&mut self, value: bool) { self.0.push_boolean(3, 11, value); }
    pub fn set_is_busy_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 11, 0, message); }
    pub fn clear_is_busy_error(&mut self) { self.0.push_clear_error(11); }
    pub fn set_can_clear(&mut self, value: bool) { self.0.push_boolean(3, 12, value); }
    pub fn set_can_clear_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 12, 0, message); }
    pub fn clear_can_clear_error(&mut self) { self.0.push_clear_error(12); }
    pub fn set_can_copy_report(&mut self, value: bool) { self.0.push_boolean(3, 13, value); }
    pub fn set_can_copy_report_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 13, 0, message); }
    pub fn clear_can_copy_report_error(&mut self) { self.0.push_clear_error(13); }
    pub fn set_primary_digest(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 14, 0, value); }
    pub fn set_primary_digest_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 14, 0, message); }
    pub fn clear_primary_digest_error(&mut self) { self.0.push_clear_error(14); }
    pub fn set_empty_visible(&mut self, value: bool) { self.0.push_boolean(3, 15, value); }
    pub fn set_empty_visible_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 15, 0, message); }
    pub fn clear_empty_visible_error(&mut self) { self.0.push_clear_error(15); }
    pub fn add_recent_files(&mut self, value: impl AsRef<str>) { self.0.push_string(7, 1, 0, value); }
    pub fn insert_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(9, 1, index, value); }
    pub fn replace_recent_files(&mut self, index: i32, value: impl AsRef<str>) { self.0.push_string(11, 1, index, value); }
    pub fn replace_recent_files_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) { self.0.push_string_snapshot(1, values); }
    pub fn remove_recent_files(&mut self, index: i32) { self.0.push_indices(13, 1, index, 0); }
    pub fn move_recent_files(&mut self, from_index: i32, to_index: i32) { self.0.push_indices(14, 1, from_index, to_index); }
    pub fn clear_recent_files(&mut self) { self.0.push_indices(19, 1, 0, 0); }
    pub fn add_files(&mut self, value: impl FileRowViewModel) { self.0.push_model(8, 2, 0, FileRowViewModelDispatch { model: value }); }
    pub fn insert_files(&mut self, index: i32, value: impl FileRowViewModel) { self.0.push_model(10, 2, index, FileRowViewModelDispatch { model: value }); }
    pub fn replace_files(&mut self, index: i32, value: impl FileRowViewModel) { self.0.push_model(12, 2, index, FileRowViewModelDispatch { model: value }); }
    pub fn replace_files_snapshot<M: FileRowViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(2, values.into_iter().map(|value| FileRowViewModelDispatch { model: value })); }
    pub fn remove_files(&mut self, index: i32) { self.0.push_model_indices(13, 2, index, 0); }
    pub fn move_files(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 2, from_index, to_index); }
    pub fn clear_files(&mut self) { self.0.push_model_clear(2); }
    pub fn set_open_files_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_clear_files_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_copy_report_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
    pub fn set_open_recent_file_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 4, enabled); }
    pub fn set_exit_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 5, enabled); }
    /// Stages the most-recently-used storage URIs as one `RecentFiles` snapshot.
    pub fn set_recent_files(&mut self, recent: &crate::RecentFileList) { self.0.push_string_snapshot(1, recent.entries()); }
}

pub trait MainViewModel: Send + 'static {
    fn attach(&mut self, sink: MainViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_expected_hash(&mut self, value: String) -> crate::Result<()>;
    fn set_include_md5(&mut self, value: bool) -> crate::Result<()>;
    fn set_include_sha1(&mut self, value: bool) -> crate::Result<()>;
    fn set_include_sha256(&mut self, value: bool) -> crate::Result<()>;
    fn set_include_sha512(&mut self, value: bool) -> crate::Result<()>;
    fn set_include_blake3(&mut self, value: bool) -> crate::Result<()>;
    fn open_files(&mut self) -> crate::Result<()>;
    fn clear_files(&mut self) -> crate::Result<()>;
    fn copy_report(&mut self) -> crate::Result<()>;
    fn open_recent_file(&mut self, value: String) -> crate::Result<()>;
    fn exit(&mut self) -> crate::Result<()>;
}

struct MainViewModelDispatch<T: MainViewModel> { model: T }

impl<T: MainViewModel> crate::view_model::DynamicViewModel for MainViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(MainViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, value: String) -> crate::Result<()> {
        match property_id {
            4 => self.model.set_expected_hash(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, value: bool) -> crate::Result<()> {
        match property_id {
            6 => self.model.set_include_md5(value),
            7 => self.model.set_include_sha1(value),
            8 => self.model.set_include_sha256(value),
            9 => self.model.set_include_sha512(value),
            10 => self.model.set_include_blake3(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            2 => self.model.clear_files(),
            4 => self.model.open_recent_file(parameter.unwrap_or_default()),
            5 => self.model.exit(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.open_files(),
            3 => self.model.copy_report(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
}

pub fn mount_main_window(scope: &crate::AppScope, model: impl MainViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model(1, MainViewModelDispatch { model }) }

#[derive(Clone, Debug)]
pub struct FileRowViewModelSink(crate::view_model::ViewModelSink);

impl FileRowViewModelSink {
    pub fn set_name(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_file_path(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(2, value) }
    pub fn set_size_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(3, value) }
    pub fn set_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(4, value) }
    pub fn set_match_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(5, value) }
    pub fn set_md5(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(6, value) }
    pub fn set_sha1(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(7, value) }
    pub fn set_sha256(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(8, value) }
    pub fn set_sha512(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(9, value) }
    pub fn set_blake3(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(10, value) }
    pub fn set_show_md5(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(11, value) }
    pub fn set_show_sha1(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(12, value) }
    pub fn set_show_sha256(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(13, value) }
    pub fn set_show_sha512(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(14, value) }
    pub fn set_show_blake3(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(15, value) }
    pub fn set_can_copy_sha256(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(16, value) }
    pub fn set_copy_sha256_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_copy_row_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_remove_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_name_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_file_path_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_size_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_match_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_md5_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_sha1_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_sha256_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_sha512_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_blake3_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    pub fn set_show_md5_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(11, message) }
    pub fn set_show_sha1_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(12, message) }
    pub fn set_show_sha256_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(13, message) }
    pub fn set_show_sha512_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(14, message) }
    pub fn set_show_blake3_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(15, message) }
    pub fn set_can_copy_sha256_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(16, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> FileRowViewModelSinkBatch { FileRowViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: FileRowViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct FileRowViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl FileRowViewModelSinkBatch {
    pub fn set_name(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_name_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_name_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_file_path(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 2, 0, value); }
    pub fn set_file_path_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_file_path_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_size_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 3, 0, value); }
    pub fn set_size_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_size_label_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 4, 0, value); }
    pub fn set_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_status_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_match_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 5, 0, value); }
    pub fn set_match_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_match_label_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_md5(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 6, 0, value); }
    pub fn set_md5_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_md5_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_sha1(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 7, 0, value); }
    pub fn set_sha1_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_sha1_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_sha256(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 8, 0, value); }
    pub fn set_sha256_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_sha256_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_sha512(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 9, 0, value); }
    pub fn set_sha512_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_sha512_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_blake3(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 10, 0, value); }
    pub fn set_blake3_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_blake3_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_show_md5(&mut self, value: bool) { self.0.push_boolean(3, 11, value); }
    pub fn set_show_md5_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 11, 0, message); }
    pub fn clear_show_md5_error(&mut self) { self.0.push_clear_error(11); }
    pub fn set_show_sha1(&mut self, value: bool) { self.0.push_boolean(3, 12, value); }
    pub fn set_show_sha1_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 12, 0, message); }
    pub fn clear_show_sha1_error(&mut self) { self.0.push_clear_error(12); }
    pub fn set_show_sha256(&mut self, value: bool) { self.0.push_boolean(3, 13, value); }
    pub fn set_show_sha256_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 13, 0, message); }
    pub fn clear_show_sha256_error(&mut self) { self.0.push_clear_error(13); }
    pub fn set_show_sha512(&mut self, value: bool) { self.0.push_boolean(3, 14, value); }
    pub fn set_show_sha512_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 14, 0, message); }
    pub fn clear_show_sha512_error(&mut self) { self.0.push_clear_error(14); }
    pub fn set_show_blake3(&mut self, value: bool) { self.0.push_boolean(3, 15, value); }
    pub fn set_show_blake3_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 15, 0, message); }
    pub fn clear_show_blake3_error(&mut self) { self.0.push_clear_error(15); }
    pub fn set_can_copy_sha256(&mut self, value: bool) { self.0.push_boolean(3, 16, value); }
    pub fn set_can_copy_sha256_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 16, 0, message); }
    pub fn clear_can_copy_sha256_error(&mut self) { self.0.push_clear_error(16); }
    pub fn set_copy_sha256_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_copy_row_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_remove_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
}

pub trait FileRowViewModel: Send + 'static {
    fn attach(&mut self, sink: FileRowViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn copy_sha256(&mut self) -> crate::Result<()>;
    fn copy_row(&mut self) -> crate::Result<()>;
    fn remove(&mut self) -> crate::Result<()>;
}

struct FileRowViewModelDispatch<T: FileRowViewModel> { model: T }

impl<T: FileRowViewModel> crate::view_model::DynamicViewModel for FileRowViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(FileRowViewModelSink(sink)) }
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
            3 => self.model.remove(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.copy_sha256(),
            2 => self.model.copy_row(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
}

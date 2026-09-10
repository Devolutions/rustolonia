//! A zip/tar explorer built as an external Rustolonia consumer.
//!
//! Rust owns archive listing, selection and extraction. The generated
//! view-model bridge exposes that state to the compiled Avalonia presentation.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

pub use avalonia::{
    AppScope, CancellationToken, ClipboardData, ConversionDirection, Error, MapKey, RangeBatch,
    RangeRequest, RecentFileList, Result, ScalarKind, ScalarValue,
};
pub mod view_model {
    pub use avalonia::view_model::{
        BatchCompletion, DynamicViewModel, ViewModelBatch, ViewModelSink,
    };
}
pub mod value_converter {
    pub use avalonia::value_converter::ValueConverterDispatch;
}

#[path = "../generated/generated_view_models.rs"]
mod generated_view_models;
mod archive;
mod fs_pane;
mod panes;

use archive::{
    archive_totals, create_archive_from_folder, extract_entries, is_nested_archive, list_entries,
    smart_extract_destination, ArchiveEntry, ArchiveKind, ExtractReport,
};
use panes::{Pane, Side};
use avalonia::{
    ActivationEvent, App, DragDropEffects, FileDropEvent, FileTypeFilter, FolderPickerOptions,
    OpenFilePickerOptions, PickerOutcome, SaveFilePickerOptions, Window,
};
use generated_view_models::{
    mount_main_window, EntryRowViewModel, EntryRowViewModelSink, MainViewModel, MainViewModelSink,
    MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc, Mutex,
};

const APP_TITLE: &str = "Archive";
static NEXT_LOAD_GENERATION: AtomicI64 = AtomicI64::new(0);
static NEXT_EXTRACT_GENERATION: AtomicI64 = AtomicI64::new(0);

struct Shared {
    sink: Option<MainViewModelSink>,
    scope: Option<AppScope>,
    window: Option<Window>,
    left: Pane,
    right: Pane,
    active: Side,
    status: String,
    loading: bool,
    recent: RecentFileList,
    load_generation: i64,
    extract_generation: i64,
    filter_text: String,
    selected_index: i64,
    selected_key: String,
    open_after_extract: bool,
}

impl Shared {
    fn new(status: String, recent: RecentFileList) -> Self {
        Self {
            sink: None,
            scope: None,
            window: None,
            left: Pane::home(),
            right: Pane::folder(fs_pane::downloads_dir()),
            active: Side::Left,
            status,
            loading: false,
            recent,
            load_generation: 0,
            extract_generation: 0,
            filter_text: String::new(),
            selected_index: -1,
            selected_key: String::new(),
            open_after_extract: true,
        }
    }

    fn pane(&self, side: Side) -> &Pane {
        match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }

    fn pane_mut(&mut self, side: Side) -> &mut Pane {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    fn active_pane(&self) -> &Pane {
        self.pane(self.active)
    }

    fn active_pane_mut(&mut self) -> &mut Pane {
        self.pane_mut(self.active)
    }
}

struct EntryRowModel {
    entry: ArchiveEntry,
    selected: bool,
    side: Side,
    shared: Arc<Mutex<Shared>>,
}

impl EntryRowViewModel for EntryRowModel {
    fn attach(&mut self, sink: EntryRowViewModelSink) -> Result<()> {
        sink.set_key(&self.entry.path)?;
        sink.set_path(&self.entry.path)?;
        sink.set_name(&self.entry.name)?;
        sink.set_kind(self.entry.kind_label())?;
        sink.set_size_label(self.entry.size_label())?;
        sink.set_packed_label(self.entry.packed_label())?;
        sink.set_modified(&self.entry.modified)?;
        sink.set_crc_label(self.entry.crc_label())?;
        sink.set_ratio_label(self.entry.ratio_label())?;
        sink.set_is_selected(self.selected)?;
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        Ok(())
    }

    fn open(&mut self) -> Result<()> {
        open_entry(&self.shared, self.side, &self.entry)
    }

    fn set_is_selected(&mut self, value: bool) -> Result<()> {
        self.selected = value;
        let (sink, label, can_extract, can_delete, can_copy) = {
            let mut shared = self.shared.lock().expect("shared state lock poisoned");
            shared.active = self.side;
            let pane = shared.pane_mut(self.side);
            if value {
                pane.selected_paths.insert(self.entry.path.clone());
            } else {
                pane.selected_paths.remove(&self.entry.path);
            }
            let label = selected_count_label(pane.selected_paths.len());
            let can_extract = can_extract_selected(&shared);
            let can_delete = can_delete_selected(&shared);
            let can_copy = can_copy_to_other(&shared);
            (shared.sink.clone(), label, can_extract, can_delete, can_copy)
        };
        if let Some(sink) = sink {
            sink.set_selected_count_label(label)?;
            sink.set_can_extract_selected(can_extract)?;
            sink.set_extract_selected_enabled(can_extract)?;
            sink.set_can_delete(can_delete)?;
            sink.set_can_copy_to_other(can_copy)?;
            sink.set_left_active(self.side == Side::Left)?;
            sink.set_right_active(self.side == Side::Right)?;
        }
        Ok(())
    }
}

struct Model {
    shared: Arc<Mutex<Shared>>,
}

impl Model {
    fn new(shared: Arc<Mutex<Shared>>) -> Self {
        Self { shared }
    }

    fn shared(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().expect("shared state lock poisoned")
    }
}

impl MainViewModel for Model {
    fn attach(&mut self, sink: MainViewModelSink) -> Result<()> {
        {
            let mut shared = self.shared();
            shared.sink = Some(sink.clone());
        }
        publish_state(&self.shared)?;
        let recent = self.shared().recent.clone();
        sink.publish_recent_files(&recent)?;
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        self.shared().sink = None;
        Ok(())
    }

    fn open_file(&mut self) -> Result<()> {
        let (scope, window, shared) = {
            let shared = self.shared();
            let (Some(scope), Some(window)) = (shared.scope.clone(), shared.window.clone()) else {
                return Ok(());
            };
            (scope, window, self.shared.clone())
        };
        let operation = scope.open_file_picker(
            &window,
            &OpenFilePickerOptions::new()
                .title("Open an archive")
                .allow_multiple(false)
                .file_type(archive_filter())
                .file_type(
                    FileTypeFilter::new("ZIP archives")
                        .with_extension("zip")
                        .with_mime_type("application/zip")
                        .with_apple_uniform_type_identifier("public.zip-archive"),
                )
                .file_type(
                    FileTypeFilter::new("TAR archives")
                        .with_extension("tar")
                        .with_mime_type("application/x-tar")
                        .with_apple_uniform_type_identifier("public.tar-archive"),
                )
                .file_type(
                    FileTypeFilter::new("Gzip TAR archives")
                        .with_extension("tgz")
                        .with_pattern("*.tar.gz")
                        .with_mime_type("application/gzip")
                        .with_apple_uniform_type_identifier("org.gnu.gnu-zip-tar-archive"),
                ),
        )?;
        scope.spawn(async move {
            match operation.await {
                Ok(PickerOutcome::Selected(items)) => {
                    let Some(path) = items.iter().find_map(|item| item.local_path()) else {
                        let _ = set_status(
                            &shared,
                            "The selected archive is not available as a local file.",
                        );
                        return;
                    };
                    let path = path.to_path_buf();
                    let name = display_name(&path);
                    let _ = start_loading(&shared, path, format!("Opening {name}..."));
                }
                Ok(PickerOutcome::Cancelled) => {}
                Err(error) => {
                    let _ = set_status(&shared, format!("Open dialog failed: {error}"));
                }
            }
        })
    }

    fn extract_selected(&mut self) -> Result<()> {
        start_extract_picker(&self.shared, false)
    }

    fn extract_all(&mut self) -> Result<()> {
        start_extract_picker(&self.shared, true)
    }

    fn select_all(&mut self) -> Result<()> {
        {
            let mut shared = self.shared();
            let filter = shared.filter_text.clone();
            let active = shared.active;
            let paths: Vec<String> = shared
                .pane(active)
                .visible(&filter)
                .into_iter()
                .map(|entry| entry.path)
                .collect();
            shared.pane_mut(active).selected_paths = paths.into_iter().collect();
        }
        publish_state(&self.shared)
    }

    fn clear_selection(&mut self) -> Result<()> {
        {
            let mut shared = self.shared();
            shared.active_pane_mut().clear_selection();
        }
        publish_state(&self.shared)
    }

    fn sort_entries(&mut self, value: String) -> Result<()> {
        {
            let mut shared = self.shared();
            let column = value.split(':').next().unwrap_or(&value).to_string();
            let pane = shared.active_pane_mut();
            if pane.sort_column == column {
                pane.sort_descending = !pane.sort_descending;
            } else {
                pane.sort_column = column;
                pane.sort_descending = false;
            }
        }
        publish_state(&self.shared)
    }

    fn go_up(&mut self) -> Result<()> {
        {
            let mut shared = self.shared();
            shared.active_pane_mut().go_up();
        }
        publish_state(&self.shared)
    }

    fn open_item(&mut self) -> Result<()> {
        let (side, entry) = {
            let shared = self.shared();
            let pane = shared.active_pane();
            let key = if pane.selected_key.is_empty() {
                return Ok(());
            } else {
                pane.selected_key.clone()
            };
            let entry = pane
                .visible(&shared.filter_text)
                .into_iter()
                .find(|entry| entry.path == key);
            (shared.active, entry)
        };
        let Some(entry) = entry else {
            return Ok(());
        };
        open_entry(&self.shared, side, &entry)
    }

    fn extract_here(&mut self) -> Result<()> {
        let (archive_path, dest, names) = {
            let shared = self.shared();
            let Some(path) = shared.active_pane().archive_path().map(Path::to_path_buf) else {
                return Ok(());
            };
            let entries = shared.active_pane().archive_entries().unwrap_or(&[]).to_vec();
            let parent = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let dest = smart_extract_destination(&parent, &archive_stem(&path), &entries);
            let names = extract_names(&shared, shared.active_pane().selected_paths.is_empty());
            (path, dest, names)
        };
        if names.is_empty() {
            return set_status(&self.shared, "Nothing to extract.");
        }
        start_extract(&self.shared, archive_path, dest, names, true)
    }

    fn test_archive(&mut self) -> Result<()> {
        let path = {
            let shared = self.shared();
            let Some(path) = shared.active_pane().archive_path().map(Path::to_path_buf) else {
                return Ok(());
            };
            path
        };
        let generation = {
            let mut shared = self.shared();
            shared.extract_generation = NEXT_EXTRACT_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
            shared.status = "Testing archive...".to_string();
            shared.loading = true;
            shared.extract_generation
        };
        let _ = publish_state(&self.shared);
        let shared = self.shared.clone();
        std::thread::Builder::new()
            .name("archive-test".to_string())
            .spawn(move || match archive::test_archive(&path) {
                Ok(report) => {
                    let _ = set_worker_status(&shared, report.summary(), generation);
                }
                Err(error) => {
                    let _ = set_worker_status(&shared, format!("Test failed: {error}"), generation);
                }
            })
            .map(|_| ())
            .map_err(|error| Error::Load(format!("Unable to start test worker: {error}")))
    }

    fn new_archive(&mut self) -> Result<()> {
        let (scope, window, shared) = {
            let shared = self.shared();
            let (Some(scope), Some(window)) = (shared.scope.clone(), shared.window.clone()) else {
                return Ok(());
            };
            (scope, window, self.shared.clone())
        };
        let folder_op = scope.open_folder_picker(
            &window,
            &FolderPickerOptions::new().title("Folder to compress"),
        )?;
        scope.spawn(async move {
            let folder = match folder_op.await {
                Ok(PickerOutcome::Selected(items)) => {
                    match items.iter().find_map(|item| item.local_path()) {
                        Some(path) => path.to_path_buf(),
                        None => {
                            let _ = set_status(&shared, "The folder is not a local path.");
                            return;
                        }
                    }
                }
                Ok(PickerOutcome::Cancelled) => return,
                Err(error) => {
                    let _ = set_status(&shared, format!("Folder picker failed: {error}"));
                    return;
                }
            };
            let default_name = folder
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("archive");
            let save_op = match {
                let state = shared.lock().expect("shared state lock poisoned");
                match (state.scope.clone(), state.window.clone()) {
                    (Some(scope), Some(window)) => scope.save_file_picker(
                        &window,
                        &SaveFilePickerOptions::new()
                            .title("Save archive")
                            .suggested_file_name(format!("{default_name}.zip"))
                            .default_extension("zip")
                            .file_type(
                                FileTypeFilter::new("ZIP archive")
                                    .with_extension("zip")
                                    .with_mime_type("application/zip"),
                            )
                            .file_type(
                                FileTypeFilter::new("Gzip TAR archive")
                                    .with_extension("tgz")
                                    .with_pattern("*.tar.gz")
                                    .with_mime_type("application/gzip"),
                            )
                            .file_type(
                                FileTypeFilter::new("7-Zip archive")
                                    .with_extension("7z")
                                    .with_mime_type("application/x-7z-compressed"),
                            ),
                    ),
                    _ => return,
                }
            } {
                Ok(operation) => operation,
                Err(error) => {
                    let _ = set_status(&shared, format!("Save dialog failed: {error}"));
                    return;
                }
            };
            match save_op.await {
                Ok(PickerOutcome::Selected(items)) => {
                    let Some(path) = items.iter().find_map(|item| item.local_path()) else {
                        let _ = set_status(&shared, "The save path is not a local file.");
                        return;
                    };
                    let _ = start_create(&shared, folder, path.to_path_buf());
                }
                Ok(PickerOutcome::Cancelled) => {}
                Err(error) => {
                    let _ = set_status(&shared, format!("Save dialog failed: {error}"));
                }
            }
        })
    }

    fn invert_selection(&mut self) -> Result<()> {
        {
            let mut shared = self.shared();
            let filter = shared.filter_text.clone();
            let active = shared.active;
            let visible: Vec<String> = shared
                .pane(active)
                .visible(&filter)
                .into_iter()
                .map(|entry| entry.path)
                .collect();
            let pane = shared.pane_mut(active);
            for path in visible {
                if !pane.selected_paths.remove(&path) {
                    pane.selected_paths.insert(path);
                }
            }
        }
        publish_state(&self.shared)
    }

    fn copy_path(&mut self) -> Result<()> {
        let (scope, window, text) = {
            let shared = self.shared();
            let (Some(scope), Some(window)) = (shared.scope.clone(), shared.window.clone()) else {
                return Ok(());
            };
            let pane = shared.active_pane();
            let text = if !pane.selected_paths.is_empty() {
                let mut names: Vec<_> = pane.selected_paths.iter().cloned().collect();
                names.sort();
                names.join("\n")
            } else if !pane.selected_key.is_empty() {
                pane.selected_key.clone()
            } else {
                return Ok(());
            };
            (scope, window, text)
        };
        let operation = scope.clipboard_write(&window, &ClipboardData::text(text))?;
        let shared = self.shared.clone();
        scope.spawn(async move {
            let status = match operation.await {
                Ok(()) => "Copied path(s) to the clipboard".to_string(),
                Err(error) => format!("Clipboard write failed: {error}"),
            };
            let _ = set_status(&shared, status);
        })
    }

    fn open_folder(&mut self) -> Result<()> {
        let (scope, window, shared) = {
            let shared = self.shared();
            let (Some(scope), Some(window)) = (shared.scope.clone(), shared.window.clone()) else {
                return Ok(());
            };
            (scope, window, self.shared.clone())
        };
        let operation = scope.open_folder_picker(
            &window,
            &FolderPickerOptions::new().title("Open folder"),
        )?;
        scope.spawn(async move {
            match operation.await {
                Ok(PickerOutcome::Selected(items)) => {
                    let Some(path) = items.iter().find_map(|item| item.local_path()) else {
                        return;
                    };
                    {
                        let mut state = shared.lock().expect("shared state lock poisoned");
                        state.active_pane_mut().open_folder(path.to_path_buf());
                        state.status = format!("Opened {}", path.display());
                    }
                    let _ = publish_state(&shared);
                }
                Ok(PickerOutcome::Cancelled) => {}
                Err(error) => {
                    let _ = set_status(&shared, format!("Folder picker failed: {error}"));
                }
            }
        })
    }

    fn activate_left(&mut self) -> Result<()> {
        self.shared().active = Side::Left;
        publish_state(&self.shared)
    }

    fn activate_right(&mut self) -> Result<()> {
        self.shared().active = Side::Right;
        publish_state(&self.shared)
    }

    fn delete_selected(&mut self) -> Result<()> {
        let paths = {
            let shared = self.shared();
            if !matches!(shared.active_pane().location, panes::Location::Folder(_)) {
                return set_status(&self.shared, "Delete is only available in folders.");
            }
            extract_names(&shared, false)
        };
        if paths.is_empty() {
            return set_status(&self.shared, "Select files to delete.");
        }
        for path in &paths {
            let target = PathBuf::from(path);
            let result = if target.is_dir() {
                std::fs::remove_dir_all(&target)
            } else {
                std::fs::remove_file(&target)
            };
            if let Err(error) = result {
                return set_status(&self.shared, format!("Delete failed: {error}"));
            }
        }
        {
            let mut shared = self.shared();
            shared.active_pane_mut().clear_selection();
            shared.status = format!("Deleted {} item(s)", paths.len());
        }
        publish_state(&self.shared)
    }

    fn copy_to_other(&mut self) -> Result<()> {
        let (from_archive, dest, names, folder_sources) = {
            let shared = self.shared();
            let dest = match shared.pane(shared.active.other()).copy_destination() {
                Some(path) => path,
                None => return set_status(&self.shared, "The other pane has nowhere to copy to."),
            };
            let names = extract_names(&shared, false);
            if names.is_empty() {
                return set_status(&self.shared, "Select items to copy.");
            }
            match shared.active_pane().archive_path() {
                Some(path) => (Some(path.to_path_buf()), dest, names, Vec::new()),
                None => (None, dest, Vec::new(), names),
            }
        };
        if let Some(archive_path) = from_archive {
            return start_extract(&self.shared, archive_path, dest, names, false);
        }
        for name in &folder_sources {
            let source = PathBuf::from(name);
            let file_name = source.file_name().unwrap_or_default();
            let target = dest.join(file_name);
            let result = if source.is_dir() {
                copy_dir_all(&source, &target)
            } else {
                std::fs::copy(&source, &target).map(|_| ())
            };
            if let Err(error) = result {
                return set_status(&self.shared, format!("Copy failed: {error}"));
            }
        }
        set_status(
            &self.shared,
            format!("Copied {} item(s) to {}", folder_sources.len(), dest.display()),
        )
    }

    fn open_computer(&mut self) -> Result<()> {
        {
            let mut shared = self.shared();
            *shared.active_pane_mut() = Pane::computer();
            shared.status = "Computer".to_string();
        }
        publish_state(&self.shared)
    }

    fn open_home(&mut self) -> Result<()> {
        {
            let mut shared = self.shared();
            *shared.active_pane_mut() = Pane::home();
            shared.status = format!("Opened {}", fs_pane::home_dir().display());
        }
        publish_state(&self.shared)
    }

    fn set_open_after_extract(&mut self, value: bool) -> Result<()> {
        self.shared().open_after_extract = value;
        Ok(())
    }

    fn set_filter_text(&mut self, value: String) -> Result<()> {
        {
            let mut shared = self.shared();
            shared.filter_text = value;
        }
        publish_state(&self.shared)
    }

    fn set_selected_index(&mut self, value: i64) -> Result<()> {
        self.shared().selected_index = value;
        Ok(())
    }

    fn set_selected_key(&mut self, value: String) -> Result<()> {
        self.shared().selected_key = value;
        Ok(())
    }

    fn set_left_selected_index(&mut self, value: i64) -> Result<()> {
        let mut shared = self.shared();
        shared.active = Side::Left;
        shared.left.selected_index = value;
        Ok(())
    }

    fn set_left_selected_key(&mut self, value: String) -> Result<()> {
        let mut shared = self.shared();
        shared.active = Side::Left;
        shared.left.selected_key = value;
        Ok(())
    }

    fn set_right_selected_index(&mut self, value: i64) -> Result<()> {
        let mut shared = self.shared();
        shared.active = Side::Right;
        shared.right.selected_index = value;
        Ok(())
    }

    fn set_right_selected_key(&mut self, value: String) -> Result<()> {
        let mut shared = self.shared();
        shared.active = Side::Right;
        shared.right.selected_key = value;
        Ok(())
    }

    fn open_recent_file(&mut self, value: String) -> Result<()> {
        let path = PathBuf::from(value);
        let name = display_name(&path);
        start_loading(&self.shared, path, format!("Opening {name}..."))
    }

    fn exit(&mut self) -> Result<()> {
        let scope = self.shared().scope.clone();
        match scope {
            Some(scope) => scope.shutdown(),
            None => Ok(()),
        }
    }
}

fn archive_filter() -> FileTypeFilter {
    FileTypeFilter::new("Archives")
        .with_extension("zip")
        .with_extension("tar")
        .with_extension("tgz")
        .with_extension("7z")
        .with_extension("cab")
        .with_extension("gz")
        .with_extension("bz2")
        .with_extension("xz")
        .with_extension("zst")
        .with_extension("lz4")
        .with_pattern("*.tar.gz")
        .with_pattern("*.tar.bz2")
        .with_pattern("*.tar.xz")
        .with_pattern("*.tar.zst")
        .with_mime_type("application/zip")
        .with_mime_type("application/x-tar")
        .with_mime_type("application/gzip")
        .with_apple_uniform_type_identifier("public.zip-archive")
        .with_apple_uniform_type_identifier("public.tar-archive")
}

fn sort_label(pane: &Pane) -> String {
    format!(
        "{} {}",
        pane.sort_column,
        if pane.sort_descending {
            "Descending"
        } else {
            "Ascending"
        }
    )
}

fn totals_label(entries: &[ArchiveEntry]) -> String {
    let (files, size, packed) = archive_totals(entries);
    match packed {
        Some(packed) => format!(
            "{files} files · {} → {}",
            format_size_ref(size),
            format_size_ref(packed)
        ),
        None => format!("{files} files · {}", format_size_ref(size)),
    }
}

fn format_size_ref(bytes: u64) -> String {
    archive::format_size(bytes)
}

fn archive_stem(path: &Path) -> String {
    let name = display_name(path);
    name.trim_end_matches(".tar.gz")
        .trim_end_matches(".tgz")
        .trim_end_matches(".tar")
        .trim_end_matches(".zip")
        .trim_end_matches(".ZIP")
        .trim_end_matches(".7z")
        .trim_end_matches(".gz")
        .to_string()
}

fn selected_count_label(count: usize) -> String {
    if count == 1 {
        "1 selected".to_string()
    } else {
        format!("{count} selected")
    }
}

fn entry_count_label(visible: usize, total: usize) -> String {
    if visible == total {
        if total == 1 {
            "1 entry".to_string()
        } else {
            format!("{total} entries")
        }
    } else {
        format!("{visible} of {total} entries")
    }
}

fn can_extract_selected(shared: &Shared) -> bool {
    if shared.loading || !shared.active_pane().is_archive() {
        return false;
    }
    let pane = shared.active_pane();
    !pane.selected_paths.is_empty() || !pane.selected_key.is_empty()
}

fn can_delete_selected(shared: &Shared) -> bool {
    !shared.loading
        && matches!(shared.active_pane().location, panes::Location::Folder(_))
        && (!shared.active_pane().selected_paths.is_empty()
            || !shared.active_pane().selected_key.is_empty())
}

fn can_copy_to_other(shared: &Shared) -> bool {
    if shared.loading {
        return false;
    }
    shared.pane(shared.active.other()).copy_destination().is_some()
        && (!shared.active_pane().selected_paths.is_empty()
            || !shared.active_pane().selected_key.is_empty())
}

fn extract_names(shared: &Shared, all: bool) -> Vec<String> {
    let pane = shared.active_pane();
    if all {
        return pane
            .archive_entries()
            .unwrap_or(&[])
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.path.clone())
            .collect();
    }
    if !pane.selected_paths.is_empty() {
        return pane.selected_paths.iter().cloned().collect();
    }
    if pane.selected_key.is_empty() {
        Vec::new()
    } else {
        vec![pane.selected_key.clone()]
    }
}

fn row_models(shared: &Arc<Mutex<Shared>>, side: Side) -> Vec<EntryRowModel> {
    let guard = shared.lock().expect("shared state lock poisoned");
    let pane = guard.pane(side);
    pane.visible(&guard.filter_text)
        .into_iter()
        .map(|entry| EntryRowModel {
            selected: pane.selected_paths.contains(&entry.path),
            entry,
            side,
            shared: shared.clone(),
        })
        .collect()
}

fn publish_state(shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let left_rows = row_models(shared, Side::Left);
    let right_rows = row_models(shared, Side::Right);
    let snapshot = {
        let shared = shared.lock().expect("shared state lock poisoned");
        let left_visible = left_rows.len();
        let right_visible = right_rows.len();
        let active = shared.active_pane();
        let can_archive = active.is_archive() && !shared.loading;
        PublishSnapshot {
            sink: shared.sink.clone(),
            status: shared.status.clone(),
            loading: shared.loading,
            archive_name: active.path_label(),
            archive_kind: active.kind_label(),
            entry_count: entry_count_label(left_visible + right_visible, left_visible + right_visible),
            selected_count: selected_count_label(active.selected_paths.len()),
            can_selected: can_extract_selected(&shared),
            can_all: can_archive,
            can_test: can_archive,
            can_go_up: active.can_go_up() && !shared.loading,
            can_delete: can_delete_selected(&shared),
            can_copy: can_copy_to_other(&shared),
            selected_index: shared.selected_index,
            selected_key: shared.selected_key.clone(),
            recent: shared.recent.clone(),
            generation: shared.load_generation,
            current_path: active.path_label(),
            totals: active
                .archive_entries()
                .map(totals_label)
                .unwrap_or_else(|| format!("{left_visible} + {right_visible} items")),
            sort: sort_label(active),
            open_after: shared.open_after_extract,
            left_path: shared.left.path_label(),
            right_path: shared.right.path_label(),
            left_kind: shared.left.kind_label(),
            right_kind: shared.right.kind_label(),
            left_can_go_up: shared.left.can_go_up() && !shared.loading,
            right_can_go_up: shared.right.can_go_up() && !shared.loading,
            left_active: shared.active == Side::Left,
            right_active: shared.active == Side::Right,
            left_selected_index: shared.left.selected_index,
            left_selected_key: shared.left.selected_key.clone(),
            right_selected_index: shared.right.selected_index,
            right_selected_key: shared.right.selected_key.clone(),
        }
    };
    let Some(sink) = snapshot.sink else {
        return Ok(());
    };
    let mut batch = sink.batch(snapshot.generation);
    batch.set_title(APP_TITLE);
    batch.set_status(snapshot.status);
    batch.set_archive_name(snapshot.archive_name);
    batch.set_archive_kind(snapshot.archive_kind);
    batch.set_entry_count_label(snapshot.entry_count);
    batch.set_selected_count_label(snapshot.selected_count);
    batch.set_is_loading(snapshot.loading);
    batch.set_can_extract_selected(snapshot.can_selected);
    batch.set_can_extract_all(snapshot.can_all);
    batch.set_can_test(snapshot.can_test);
    batch.set_can_go_up(snapshot.can_go_up);
    batch.set_can_delete(snapshot.can_delete);
    batch.set_can_copy_to_other(snapshot.can_copy);
    batch.set_extract_selected_enabled(snapshot.can_selected);
    batch.set_extract_all_enabled(snapshot.can_all);
    batch.set_extract_here_enabled(snapshot.can_all);
    batch.set_test_archive_enabled(snapshot.can_test);
    batch.set_go_up_enabled(snapshot.can_go_up);
    batch.set_open_item_enabled(!snapshot.loading);
    batch.set_delete_selected_enabled(snapshot.can_delete);
    batch.set_copy_to_other_enabled(snapshot.can_copy);
    batch.set_current_path(snapshot.current_path);
    batch.set_totals_label(snapshot.totals);
    batch.set_sort_direction(snapshot.sort);
    batch.set_open_after_extract(snapshot.open_after);
    batch.set_recent_files(&snapshot.recent);
    batch.set_left_path(snapshot.left_path);
    batch.set_right_path(snapshot.right_path);
    batch.set_left_kind(snapshot.left_kind);
    batch.set_right_kind(snapshot.right_kind);
    batch.set_left_can_go_up(snapshot.left_can_go_up);
    batch.set_right_can_go_up(snapshot.right_can_go_up);
    batch.set_left_active(snapshot.left_active);
    batch.set_right_active(snapshot.right_active);
    batch.replace_left_entries_snapshot(left_rows);
    batch.replace_right_entries_snapshot(right_rows);
    batch.set_selected_index(snapshot.selected_index);
    batch.set_selected_key(snapshot.selected_key);
    batch.set_left_selected_index(snapshot.left_selected_index);
    batch.set_left_selected_key(snapshot.left_selected_key);
    batch.set_right_selected_index(snapshot.right_selected_index);
    batch.set_right_selected_key(snapshot.right_selected_key);
    sink.submit_batch(batch).map(|_| ())?;
    Ok(())
}

struct PublishSnapshot {
    sink: Option<MainViewModelSink>,
    status: String,
    loading: bool,
    archive_name: String,
    archive_kind: String,
    entry_count: String,
    selected_count: String,
    can_selected: bool,
    can_all: bool,
    can_test: bool,
    can_go_up: bool,
    can_delete: bool,
    can_copy: bool,
    selected_index: i64,
    selected_key: String,
    recent: RecentFileList,
    generation: i64,
    current_path: String,
    totals: String,
    sort: String,
    open_after: bool,
    left_path: String,
    right_path: String,
    left_kind: String,
    right_kind: String,
    left_can_go_up: bool,
    right_can_go_up: bool,
    left_active: bool,
    right_active: bool,
    left_selected_index: i64,
    left_selected_key: String,
    right_selected_index: i64,
    right_selected_key: String,
}

fn set_status(shared: &Arc<Mutex<Shared>>, status: impl Into<String>) -> Result<()> {
    {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.status = status.into();
        shared.loading = false;
    }
    publish_state(shared)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("archive")
        .to_string()
}

fn start_loading(shared: &Arc<Mutex<Shared>>, path: PathBuf, status: String) -> Result<()> {
    let (generation, sink) = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.load_generation = NEXT_LOAD_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        shared.status = status.clone();
        shared.loading = true;
        (shared.load_generation, shared.sink.clone())
    };
    let worker_shared = shared.clone();
    let worker_path = path;
    std::thread::Builder::new()
        .name("archive-loader".to_string())
        .spawn(move || {
            let result = list_entries(&worker_path);
            match result {
                Ok((kind, entries)) => {
                    if let Err(error) =
                        apply_archive(&worker_shared, worker_path, kind, entries, generation)
                    {
                        eprintln!("Archive explorer update failed: {error}");
                    }
                }
                Err(error) => {
                    if let Err(update_error) =
                        set_load_error(&worker_shared, &worker_path, error, generation)
                    {
                        eprintln!("Archive explorer error update failed: {update_error}");
                    }
                }
            }
        })
        .map_err(|error| Error::Load(format!("Unable to start archive worker: {error}")))?;
    if let Some(sink) = sink {
        sink.set_status(status)?;
        sink.set_is_loading(true)?;
        sink.set_can_extract_selected(false)?;
        sink.set_can_extract_all(false)?;
        sink.set_extract_selected_enabled(false)?;
        sink.set_extract_all_enabled(false)?;
        sink.set_extract_here_enabled(false)?;
        sink.set_test_archive_enabled(false)?;
    }
    Ok(())
}

fn apply_archive(
    shared: &Arc<Mutex<Shared>>,
    path: PathBuf,
    kind: ArchiveKind,
    entries: Vec<ArchiveEntry>,
    generation: i64,
) -> Result<()> {
    {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        if shared.load_generation != generation {
            return Ok(());
        }
        let name = display_name(&path);
        let files = entries.iter().filter(|entry| !entry.is_dir).count();
        shared.status = format!(
            "Loaded {name} · {files} file{}",
            if files == 1 { "" } else { "s" }
        );
        shared.loading = false;
        shared.recent.push(path.to_string_lossy().to_string());
        shared.selected_index = if entries.is_empty() { -1 } else { 0 };
        shared.selected_key = entries
            .first()
            .map(|entry| entry.path.clone())
            .unwrap_or_default();
        shared.active_pane_mut().open_archive(path, kind, entries);
    }
    publish_state(shared)
}

fn set_load_error(
    shared: &Arc<Mutex<Shared>>,
    path: &Path,
    error: String,
    generation: i64,
) -> Result<()> {
    {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        if shared.load_generation != generation {
            return Ok(());
        }
        shared.status = format!("Unable to open {}: {error}", display_name(path));
        shared.loading = false;
        shared.active_pane_mut().clear_selection();
        shared.selected_index = -1;
        shared.selected_key.clear();
    }
    publish_state(shared)
}

fn start_extract_picker(shared: &Arc<Mutex<Shared>>, all: bool) -> Result<()> {
    let names = {
        let state = shared.lock().expect("shared state lock poisoned");
        extract_names(&state, all)
    };
    if names.is_empty() {
        return set_status(shared, "Select at least one entry to extract.");
    }
    let (scope, window, archive_path) = {
        let state = shared.lock().expect("shared state lock poisoned");
        let (Some(scope), Some(window), Some(archive_path)) = (
            state.scope.clone(),
            state.window.clone(),
            state.active_pane().archive_path().map(Path::to_path_buf),
        ) else {
            return Ok(());
        };
        (scope, window, archive_path)
    };
    let operation = scope.open_folder_picker(
        &window,
        &FolderPickerOptions::new().title("Extract to folder"),
    )?;
    let worker_shared = shared.clone();
    scope.spawn(async move {
        match operation.await {
            Ok(PickerOutcome::Selected(items)) => {
                let Some(path) = items.iter().find_map(|item| item.local_path()) else {
                    let _ = set_status(
                        &worker_shared,
                        "The selected folder is not available as a local path.",
                    );
                    return;
                };
                let dest = {
                    let state = worker_shared.lock().expect("shared state lock poisoned");
                    match state.active_pane().archive_path() {
                        Some(archive_path) => smart_extract_destination(
                            path,
                            &archive_stem(archive_path),
                            state.active_pane().archive_entries().unwrap_or(&[]),
                        ),
                        None => path.to_path_buf(),
                    }
                };
                let _ = start_extract(&worker_shared, archive_path, dest, names, true);
            }
            Ok(PickerOutcome::Cancelled) => {}
            Err(error) => {
                let _ = set_status(&worker_shared, format!("Extract dialog failed: {error}"));
            }
        }
    })
}

fn start_extract(
    shared: &Arc<Mutex<Shared>>,
    archive_path: PathBuf,
    destination: PathBuf,
    names: Vec<String>,
    reveal: bool,
) -> Result<()> {
    let generation = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.extract_generation = NEXT_EXTRACT_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        shared.status = format!("Extracting {} item(s)...", names.len());
        shared.loading = true;
        shared.extract_generation
    };
    let _ = publish_state(shared);
    let worker_shared = shared.clone();
    std::thread::Builder::new()
        .name("archive-extract".to_string())
        .spawn(move || match extract_entries(&archive_path, &destination, &names) {
            Ok(report) => {
                if let Err(error) = apply_extract(&worker_shared, report, generation, reveal) {
                    eprintln!("Archive explorer extract update failed: {error}");
                }
            }
            Err(error) => {
                if let Err(update_error) = set_extract_error(&worker_shared, error, generation) {
                    eprintln!("Archive explorer extract error update failed: {update_error}");
                }
            }
        })
        .map(|_| ())
        .map_err(|error| Error::Load(format!("Unable to start extract worker: {error}")))
}

fn apply_extract(
    shared: &Arc<Mutex<Shared>>,
    report: ExtractReport,
    generation: i64,
    reveal: bool,
) -> Result<()> {
    let (open_after, dest) = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        if shared.extract_generation != generation {
            return Ok(());
        }
        shared.loading = false;
        shared.status = format!(
            "Extracted {} file{} to {}",
            report.extracted,
            if report.extracted == 1 { "" } else { "s" },
            report.destination.display()
        );
        (
            reveal && shared.open_after_extract,
            report.destination.clone(),
        )
    };
    if open_after {
        let _ = open_in_shell(&dest);
    }
    publish_state(shared)
}

fn set_extract_error(shared: &Arc<Mutex<Shared>>, error: String, generation: i64) -> Result<()> {
    {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        if shared.extract_generation != generation {
            return Ok(());
        }
        shared.loading = false;
        shared.status = format!("Extract failed: {error}");
    }
    publish_state(shared)
}

fn open_dropped_or_activated(shared: &Arc<Mutex<Shared>>, path: PathBuf) -> Result<()> {
    let name = display_name(&path);
    start_loading(shared, path, format!("Opening {name}..."))
}

fn open_entry(shared: &Arc<Mutex<Shared>>, side: Side, entry: &ArchiveEntry) -> Result<()> {
    if entry.is_dir {
        {
            let mut shared = shared.lock().expect("shared state lock poisoned");
            shared.active = side;
            shared.pane_mut(side).enter_dir(entry.path.clone());
        }
        return publish_state(shared);
    }
    let folder_path = PathBuf::from(&entry.path);
    let in_archive = {
        let shared = shared.lock().expect("shared state lock poisoned");
        shared.pane(side).is_archive()
    };
    if !in_archive {
        if folder_path.is_dir() {
            {
                let mut shared = shared.lock().expect("shared state lock poisoned");
                shared.active = side;
                shared.pane_mut(side).open_folder(folder_path);
            }
            return publish_state(shared);
        }
        if archive::detect_kind(&folder_path).is_ok() {
            return start_loading(shared, folder_path, format!("Opening {}...", entry.name));
        }
        return match open_in_shell(&folder_path) {
            Ok(()) => set_status(shared, format!("Opened {}", entry.name)),
            Err(error) => set_status(shared, error),
        };
    }
    let archive_path = {
        let shared = shared.lock().expect("shared state lock poisoned");
        let Some(path) = shared.pane(side).archive_path().map(Path::to_path_buf) else {
            return Ok(());
        };
        path
    };
    let temp = std::env::temp_dir().join(format!(
        "rustolonia-archive-open-{}-{}",
        std::process::id(),
        NEXT_EXTRACT_GENERATION.fetch_add(1, Ordering::Relaxed)
    ));
    extract_entries(&archive_path, &temp, &[entry.path.clone()]).map_err(Error::Load)?;
    let extracted = temp.join(entry.path.replace('/', std::path::MAIN_SEPARATOR_STR));
    if is_nested_archive(&entry.path) {
        return start_loading(shared, extracted, format!("Opening {}...", entry.name));
    }
    match open_in_shell(&extracted) {
        Ok(()) => set_status(shared, format!("Opened {}", entry.name)),
        Err(error) => set_status(shared, error),
    }
}

fn set_worker_status(
    shared: &Arc<Mutex<Shared>>,
    status: String,
    generation: i64,
) -> Result<()> {
    {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        if shared.extract_generation != generation {
            return Ok(());
        }
        shared.loading = false;
        shared.status = status;
    }
    publish_state(shared)
}

fn start_create(shared: &Arc<Mutex<Shared>>, folder: PathBuf, destination: PathBuf) -> Result<()> {
    let kind = archive::detect_kind(&destination).unwrap_or(ArchiveKind::Zip);
    let generation = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.extract_generation = NEXT_EXTRACT_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        shared.status = format!("Creating {}...", display_name(&destination));
        shared.loading = true;
        shared.extract_generation
    };
    let _ = publish_state(shared);
    let worker = shared.clone();
    std::thread::Builder::new()
        .name("archive-create".to_string())
        .spawn(move || match create_archive_from_folder(&folder, &destination, kind) {
            Ok(()) => {
                let name = display_name(&destination);
                let _ = start_loading(&worker, destination, format!("Opening {name}..."));
            }
            Err(error) => {
                let _ = set_worker_status(&worker, format!("Create failed: {error}"), generation);
            }
        })
        .map(|_| ())
        .map_err(|error| Error::Load(format!("Unable to start create worker: {error}")))
}

fn copy_dir_all(source: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn open_in_shell(path: &Path) -> std::result::Result<(), String> {
    let result = {
        #[cfg(windows)]
        {
            if path.is_dir() {
                Command::new("explorer").arg(path).spawn()
            } else {
                Command::new("cmd")
                    .args(["/C", "start", "", &path.to_string_lossy()])
                    .spawn()
            }
        }
        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg(path).spawn()
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            Command::new("xdg-open").arg(path).spawn()
        }
    };
    result
        .map(|_| ())
        .map_err(|error| format!("Unable to open {}: {error}", path.display()))
}

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let startup = scope.activation_items()?;
        let mut recent = RecentFileList::with_capacity(MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY);
        let startup_path = startup.iter().find_map(|item| item.local_path()).map(Path::to_path_buf);
        let status = if let Some(path) = startup_path.as_ref() {
            recent.push(path.to_string_lossy().to_string());
            format!("Opening {}...", display_name(path))
        } else {
            "Ready.".to_string()
        };
        let shared = Arc::new(Mutex::new(Shared::new(status.clone(), recent)));
        mount_main_window(scope, Model::new(shared.clone()))?;

        let mut state = shared.lock().expect("shared state lock poisoned");
        state.scope = Some(scope.clone());
        state.window = scope.main_window();
        drop(state);

        let activation_shared = shared.clone();
        scope.on_activation(move |event| {
            if let ActivationEvent::Files(items) = &event {
                if let Some(path) = items.iter().find_map(|item| item.local_path()) {
                    let _ = open_dropped_or_activated(&activation_shared, path.to_path_buf());
                }
            }
        })?;
        if let Some(window) = scope.main_window() {
            let drop_shared = shared.clone();
            scope.on_file_drop(&window, DragDropEffects::COPY, move |event| {
                if let FileDropEvent::Drop { items, .. } = event {
                    if let Some(path) = items.iter().find_map(|item| item.local_path()) {
                        let _ = open_dropped_or_activated(&drop_shared, path.to_path_buf());
                    }
                }
            })?;
        }
        if let Some(path) = startup_path {
            start_loading(&shared, path, status)?;
        } else {
            publish_state(&shared)?;
        }
        Ok(())
    })
}

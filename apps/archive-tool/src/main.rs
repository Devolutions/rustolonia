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

use archive::{
    archive_totals, children_of, create_archive_from_folder, extract_entries, is_nested_archive,
    list_entries, parent_dir, smart_extract_destination, ArchiveEntry, ArchiveKind,
    ExtractReport,
};
use avalonia::{
    ActivationEvent, App, DragDropEffects, FileDropEvent, FileTypeFilter, FolderPickerOptions,
    OpenFilePickerOptions, PickerOutcome, SaveFilePickerOptions, Window,
};
use generated_view_models::{
    mount_main_window, EntryRowViewModel, EntryRowViewModelSink, MainViewModel, MainViewModelSink,
    MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY,
};
use std::cmp::Ordering as CmpOrdering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc, Mutex,
};

const APP_TITLE: &str = "Archive Explorer";
static NEXT_LOAD_GENERATION: AtomicI64 = AtomicI64::new(0);
static NEXT_EXTRACT_GENERATION: AtomicI64 = AtomicI64::new(0);

struct ArchiveState {
    path: PathBuf,
    name: String,
    kind: ArchiveKind,
    entries: Vec<ArchiveEntry>,
}

struct Shared {
    sink: Option<MainViewModelSink>,
    scope: Option<AppScope>,
    window: Option<Window>,
    archive: Option<ArchiveState>,
    status: String,
    loading: bool,
    recent: RecentFileList,
    load_generation: i64,
    extract_generation: i64,
    filter_text: String,
    selected_paths: HashSet<String>,
    selected_index: i64,
    selected_key: String,
    current_dir: String,
    sort_column: String,
    sort_descending: bool,
    open_after_extract: bool,
}

impl Shared {
    fn new(status: String, recent: RecentFileList) -> Self {
        Self {
            sink: None,
            scope: None,
            window: None,
            archive: None,
            status,
            loading: false,
            recent,
            load_generation: 0,
            extract_generation: 0,
            filter_text: String::new(),
            selected_paths: HashSet::new(),
            selected_index: -1,
            selected_key: String::new(),
            current_dir: String::new(),
            sort_column: "Name".to_string(),
            sort_descending: false,
            open_after_extract: true,
        }
    }
}

struct EntryRowModel {
    entry: ArchiveEntry,
    selected: bool,
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
        open_entry(&self.shared, &self.entry)
    }

    fn set_is_selected(&mut self, value: bool) -> Result<()> {
        self.selected = value;
        let (sink, label, can_extract) = {
            let mut shared = self.shared.lock().expect("shared state lock poisoned");
            if value {
                shared.selected_paths.insert(self.entry.path.clone());
            } else {
                shared.selected_paths.remove(&self.entry.path);
            }
            let label = selected_count_label(shared.selected_paths.len());
            let can_extract = can_extract_selected(&shared);
            (shared.sink.clone(), label, can_extract)
        };
        if let Some(sink) = sink {
            sink.set_selected_count_label(label)?;
            sink.set_can_extract_selected(can_extract)?;
            sink.set_extract_selected_enabled(can_extract)?;
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
            let Some(archive) = shared.archive.as_ref() else {
                return Ok(());
            };
            let paths: Vec<String> = visible_entries(&shared, archive)
                .into_iter()
                .map(|entry| entry.path.clone())
                .collect();
            shared.selected_paths = paths.into_iter().collect();
        }
        publish_state(&self.shared)
    }

    fn clear_selection(&mut self) -> Result<()> {
        {
            let mut shared = self.shared();
            shared.selected_paths.clear();
        }
        publish_state(&self.shared)
    }

    fn sort_entries(&mut self, value: String) -> Result<()> {
        {
            let mut shared = self.shared();
            let column = value.split(':').next().unwrap_or(&value).to_string();
            if shared.sort_column == column {
                shared.sort_descending = !shared.sort_descending;
            } else {
                shared.sort_column = column;
                shared.sort_descending = false;
            }
        }
        publish_state(&self.shared)
    }

    fn go_up(&mut self) -> Result<()> {
        {
            let mut shared = self.shared();
            if shared.current_dir.is_empty() {
                return Ok(());
            }
            shared.current_dir = parent_dir(&shared.current_dir);
            shared.selected_paths.clear();
            shared.selected_index = -1;
            shared.selected_key.clear();
        }
        publish_state(&self.shared)
    }

    fn open_item(&mut self) -> Result<()> {
        let entry = {
            let shared = self.shared();
            let Some(archive) = shared.archive.as_ref() else {
                return Ok(());
            };
            let key = if shared.selected_key.is_empty() {
                return Ok(());
            } else {
                shared.selected_key.clone()
            };
            archive
                .entries
                .iter()
                .find(|entry| entry.path == key)
                .cloned()
        };
        let Some(entry) = entry else {
            return Ok(());
        };
        open_entry(&self.shared, &entry)
    }

    fn extract_here(&mut self) -> Result<()> {
        let (archive_path, dest, names) = {
            let shared = self.shared();
            let Some(archive) = shared.archive.as_ref() else {
                return Ok(());
            };
            let parent = archive
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let dest = smart_extract_destination(&parent, &archive_stem(&archive.path), &archive.entries);
            let names = extract_names(&shared, shared.selected_paths.is_empty());
            (archive.path.clone(), dest, names)
        };
        if names.is_empty() {
            return set_status(&self.shared, "Nothing to extract.");
        }
        start_extract(&self.shared, archive_path, dest, names, true)
    }

    fn test_archive(&mut self) -> Result<()> {
        let path = {
            let shared = self.shared();
            let Some(archive) = shared.archive.as_ref() else {
                return Ok(());
            };
            archive.path.clone()
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
            let Some(archive) = shared.archive.as_ref() else {
                return Ok(());
            };
            let visible: Vec<String> = visible_entries(&shared, archive)
                .into_iter()
                .map(|entry| entry.path.clone())
                .collect();
            for path in visible {
                if !shared.selected_paths.remove(&path) {
                    shared.selected_paths.insert(path);
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
            let text = if !shared.selected_paths.is_empty() {
                let mut names: Vec<_> = shared.selected_paths.iter().cloned().collect();
                names.sort();
                names.join("\n")
            } else if !shared.selected_key.is_empty() {
                shared.selected_key.clone()
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
        .with_pattern("*.tar.gz")
        .with_mime_type("application/zip")
        .with_mime_type("application/x-tar")
        .with_mime_type("application/gzip")
        .with_apple_uniform_type_identifier("public.zip-archive")
        .with_apple_uniform_type_identifier("public.tar-archive")
}

fn visible_entries<'a>(shared: &'a Shared, archive: &'a ArchiveState) -> Vec<&'a ArchiveEntry> {
    let needle = shared.filter_text.trim().to_ascii_lowercase();
    let mut entries = if needle.is_empty() {
        children_of(&archive.entries, &shared.current_dir)
    } else {
        archive
            .entries
            .iter()
            .filter(|entry| {
                (shared.current_dir.is_empty()
                    || entry.path == shared.current_dir
                    || entry.path.starts_with(&format!("{}/", shared.current_dir)))
                    && entry.path.to_ascii_lowercase().contains(&needle)
            })
            .collect()
    };
    entries.sort_by(|left, right| compare_entries(left, right, &shared.sort_column, shared.sort_descending));
    entries
}

fn compare_entries(
    left: &ArchiveEntry,
    right: &ArchiveEntry,
    column: &str,
    descending: bool,
) -> CmpOrdering {
    let folder_order = right.is_dir.cmp(&left.is_dir);
    if folder_order != CmpOrdering::Equal {
        return folder_order;
    }
    let ordering = match column {
        "Kind" => left.kind_label().cmp(right.kind_label()),
        "Size" => left.size.cmp(&right.size),
        "Packed" => left
            .compressed_size
            .unwrap_or(0)
            .cmp(&right.compressed_size.unwrap_or(0)),
        "Ratio" => left.ratio_label().cmp(&right.ratio_label()),
        "Modified" => left.modified.cmp(&right.modified),
        "Crc" => left.crc.cmp(&right.crc),
        _ => left.name.to_ascii_lowercase().cmp(&right.name.to_ascii_lowercase()),
    };
    if descending {
        ordering.reverse()
    } else {
        ordering
    }
}

fn current_path_label(shared: &Shared) -> String {
    match shared.archive.as_ref() {
        None => String::new(),
        Some(archive) if shared.current_dir.is_empty() => archive.name.clone(),
        Some(archive) => format!("{} / {}", archive.name, shared.current_dir.replace('/', " / ")),
    }
}

fn sort_label(shared: &Shared) -> String {
    format!(
        "{} {}",
        shared.sort_column,
        if shared.sort_descending {
            "Descending"
        } else {
            "Ascending"
        }
    )
}

fn totals_label(archive: &ArchiveState) -> String {
    let (files, size, packed) = archive_totals(&archive.entries);
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
    if shared.loading || shared.archive.is_none() {
        return false;
    }
    if !shared.selected_paths.is_empty() {
        return true;
    }
    !shared.selected_key.is_empty()
}

fn extract_names(shared: &Shared, all: bool) -> Vec<String> {
    let Some(archive) = shared.archive.as_ref() else {
        return Vec::new();
    };
    if all {
        return archive
            .entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.path.clone())
            .collect();
    }
    if !shared.selected_paths.is_empty() {
        return shared.selected_paths.iter().cloned().collect();
    }
    if shared.selected_key.is_empty() {
        Vec::new()
    } else {
        vec![shared.selected_key.clone()]
    }
}

fn row_models(shared: &Arc<Mutex<Shared>>) -> Vec<EntryRowModel> {
    let guard = shared.lock().expect("shared state lock poisoned");
    let Some(archive) = guard.archive.as_ref() else {
        return Vec::new();
    };
    visible_entries(&guard, archive)
        .into_iter()
        .map(|entry| EntryRowModel {
            selected: guard.selected_paths.contains(&entry.path),
            entry: entry.clone(),
            shared: shared.clone(),
        })
        .collect()
}

fn publish_state(shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let rows = row_models(shared);
    let (
        sink,
        status,
        loading,
        archive_name,
        archive_kind,
        entry_count,
        selected_count,
        can_selected,
        can_all,
        can_test,
        can_go_up,
        selected_index,
        selected_key,
        recent,
        generation,
        current_path,
        totals,
        sort,
        open_after,
    ) = {
        let shared = shared.lock().expect("shared state lock poisoned");
        let total = shared
            .archive
            .as_ref()
            .map(|archive| archive.entries.len())
            .unwrap_or(0);
        let visible = rows.len();
        (
            shared.sink.clone(),
            shared.status.clone(),
            shared.loading,
            shared
                .archive
                .as_ref()
                .map(|archive| archive.name.clone())
                .unwrap_or_default(),
            shared
                .archive
                .as_ref()
                .map(|archive| archive.kind.label().to_string())
                .unwrap_or_default(),
            if shared.archive.is_some() {
                entry_count_label(visible, total)
            } else {
                "No archive loaded".to_string()
            },
            selected_count_label(shared.selected_paths.len()),
            can_extract_selected(&shared),
            shared.archive.is_some() && !shared.loading,
            shared.archive.is_some() && !shared.loading,
            !shared.current_dir.is_empty() && !shared.loading,
            shared.selected_index,
            shared.selected_key.clone(),
            shared.recent.clone(),
            shared.load_generation,
            current_path_label(&shared),
            shared
                .archive
                .as_ref()
                .map(totals_label)
                .unwrap_or_default(),
            sort_label(&shared),
            shared.open_after_extract,
        )
    };
    let Some(sink) = sink else {
        return Ok(());
    };
    let mut batch = sink.batch(generation);
    batch.set_title(APP_TITLE);
    batch.set_status(status);
    batch.set_archive_name(archive_name);
    batch.set_archive_kind(archive_kind);
    batch.set_entry_count_label(entry_count);
    batch.set_selected_count_label(selected_count);
    batch.set_is_loading(loading);
    batch.set_can_extract_selected(can_selected);
    batch.set_can_extract_all(can_all);
    batch.set_can_test(can_test);
    batch.set_can_go_up(can_go_up);
    batch.set_extract_selected_enabled(can_selected);
    batch.set_extract_all_enabled(can_all);
    batch.set_extract_here_enabled(can_all);
    batch.set_test_archive_enabled(can_test);
    batch.set_go_up_enabled(can_go_up);
    batch.set_open_item_enabled(can_all);
    batch.set_current_path(current_path);
    batch.set_totals_label(totals);
    batch.set_sort_direction(sort);
    batch.set_open_after_extract(open_after);
    batch.set_recent_files(&recent);
    batch.replace_entries_snapshot(rows);
    batch.set_selected_index(selected_index);
    batch.set_selected_key(selected_key);
    sink.submit_batch(batch).map(|_| ())?;
    Ok(())
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
    let generation = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.load_generation = NEXT_LOAD_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        shared.status = status.clone();
        shared.loading = true;
        shared.load_generation
    };
    let _ = publish_state(shared);
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
        .map(|_| ())
        .map_err(|error| Error::Load(format!("Unable to start archive worker: {error}")))
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
        shared.selected_paths.clear();
        shared.current_dir.clear();
        shared.selected_index = if entries.is_empty() { -1 } else { 0 };
        shared.selected_key = children_of(&entries, "")
            .first()
            .map(|entry| entry.path.clone())
            .unwrap_or_default();
        shared.archive = Some(ArchiveState {
            path,
            name,
            kind,
            entries,
        });
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
        shared.archive = None;
        shared.selected_paths.clear();
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
        let (Some(scope), Some(window), Some(archive)) = (
            state.scope.clone(),
            state.window.clone(),
            state.archive.as_ref(),
        ) else {
            return Ok(());
        };
        (scope, window, archive.path.clone())
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
                    match state.archive.as_ref() {
                        Some(archive) => smart_extract_destination(
                            path,
                            &archive_stem(&archive.path),
                            &archive.entries,
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

fn open_entry(shared: &Arc<Mutex<Shared>>, entry: &ArchiveEntry) -> Result<()> {
    if entry.is_dir {
        {
            let mut shared = shared.lock().expect("shared state lock poisoned");
            shared.current_dir = entry.path.clone();
            shared.selected_paths.clear();
            shared.selected_index = -1;
            shared.selected_key.clear();
        }
        return publish_state(shared);
    }
    let archive_path = {
        let shared = shared.lock().expect("shared state lock poisoned");
        let Some(archive) = shared.archive.as_ref() else {
            return Ok(());
        };
        archive.path.clone()
    };
    let temp = std::env::temp_dir().join(format!(
        "rustolonia-archive-open-{}-{}",
        std::process::id(),
        NEXT_EXTRACT_GENERATION.fetch_add(1, Ordering::Relaxed)
    ));
    extract_entries(&archive_path, &temp, &[entry.path.clone()])
        .map_err(Error::Load)?;
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
            "Open an archive to get started.".to_string()
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

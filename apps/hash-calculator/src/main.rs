//! A file hash calculator built as an external Rustolonia consumer.
//!
//! Rust owns hashing, comparison and the file list. The generated view-model
//! bridge exposes that state to the compiled Avalonia presentation.
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
mod hash;

use avalonia::{
    ActivationEvent, App, FileTypeFilter, OpenFilePickerOptions, PickerOutcome, Window,
};
use generated_view_models::{
    mount_main_window, FileRowViewModel, FileRowViewModelSink, MainViewModel, MainViewModelSink,
    MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY,
};
use hash::{
    format_row_report, format_size, hash_file, match_label, write_sample_file, FileHashes,
    HashOptions, SAMPLE_FILE_NAME,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const APP_TITLE: &str = "Hash Calculator";
const STATUS_WAITING: &str = "Waiting...";
const STATUS_HASHING: &str = "Hashing...";
const STATUS_HASHED: &str = "Hashed";

struct HashFile {
    id: u64,
    path: PathBuf,
    name: String,
    size_label: String,
    status: String,
    hashes: FileHashes,
    match_label: String,
    sink: Option<FileRowViewModelSink>,
}

struct Shared {
    sink: Option<MainViewModelSink>,
    scope: Option<AppScope>,
    window: Option<Window>,
    files: Vec<HashFile>,
    recent: RecentFileList,
    expected_hash: String,
    options: HashOptions,
    status: String,
    hash_generation: i64,
    next_id: u64,
    worker_running: bool,
}

impl Shared {
    fn new(status: String, recent: RecentFileList) -> Self {
        Self {
            sink: None,
            scope: None,
            window: None,
            files: Vec::new(),
            recent,
            expected_hash: String::new(),
            options: HashOptions::default(),
            status,
            hash_generation: 0,
            next_id: 1,
            worker_running: false,
        }
    }
}

struct FileRowModel {
    id: u64,
    shared: Arc<Mutex<Shared>>,
}

impl FileRowViewModel for FileRowModel {
    fn attach(&mut self, sink: FileRowViewModelSink) -> Result<()> {
        let mut shared = self.shared.lock().expect("shared state lock poisoned");
        let options = shared.options;
        if let Some(file) = shared.files.iter_mut().find(|file| file.id == self.id) {
            publish_file_row(&sink, file, options)?;
            file.sink = Some(sink);
        }
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        let mut shared = self.shared.lock().expect("shared state lock poisoned");
        if let Some(file) = shared.files.iter_mut().find(|file| file.id == self.id) {
            file.sink = None;
        }
        Ok(())
    }

    fn copy_sha256(&mut self) -> Result<()> {
        let digest = {
            let shared = self.shared.lock().expect("shared state lock poisoned");
            shared
                .files
                .iter()
                .find(|file| file.id == self.id)
                .and_then(|file| file.hashes.sha256.clone())
        };
        let Some(digest) = digest else {
            return Ok(());
        };
        copy_text(&self.shared, digest, "Copied SHA-256 to the clipboard")
    }

    fn copy_row(&mut self) -> Result<()> {
        let report = {
            let shared = self.shared.lock().expect("shared state lock poisoned");
            shared.files.iter().find(|file| file.id == self.id).map(|file| {
                format_row_report(
                    &file.name,
                    &file.path.to_string_lossy(),
                    &file.size_label,
                    &file.hashes,
                )
            })
        };
        let Some(report) = report else {
            return Ok(());
        };
        copy_text(&self.shared, report, "Copied file hashes to the clipboard")
    }

    fn remove(&mut self) -> Result<()> {
        let id = self.id;
        let shared = self.shared.clone();
        let Some(scope) = self
            .shared
            .lock()
            .expect("shared state lock poisoned")
            .scope
            .clone()
        else {
            return Ok(());
        };
        // Defer so we do not destroy the row whose Remove command is still running.
        scope.post(move || {
            if let Err(error) = remove_file(&shared, id) {
                eprintln!("Hash calculator remove failed: {error}");
            }
        })
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
            shared.sink = Some(sink);
        }
        publish_main(&self.shared)?;
        republish_files(&self.shared)
    }

    fn detach(&mut self) -> Result<()> {
        self.shared().sink = None;
        Ok(())
    }

    fn open_files(&mut self) -> Result<()> {
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
                .title("Open files to hash")
                .allow_multiple(true)
                .file_type(FileTypeFilter::new("All files").with_pattern("*.*")),
        )?;
        scope.spawn(async move {
            match operation.await {
                Ok(PickerOutcome::Selected(items)) => {
                    let paths: Vec<PathBuf> = items
                        .iter()
                        .filter_map(|item| item.local_path().map(Path::to_path_buf))
                        .collect();
                    if paths.is_empty() {
                        let _ = set_status(&shared, "The selection is not available as local files.");
                        return;
                    }
                    if let Err(error) = add_paths(&shared, &paths, true) {
                        eprintln!("Hash calculator open failed: {error}");
                    }
                }
                Ok(PickerOutcome::Cancelled) => {}
                Err(error) => {
                    let _ = set_status(&shared, format!("Open dialog failed: {error}"));
                }
            }
        })
    }

    fn clear_files(&mut self) -> Result<()> {
        {
            let mut shared = self.shared();
            shared.files.clear();
            shared.hash_generation += 1;
            shared.status = "File list cleared".to_string();
        }
        republish_files(&self.shared)?;
        publish_main(&self.shared)
    }

    fn copy_report(&mut self) -> Result<()> {
        let report = {
            let shared = self.shared();
            if shared.files.is_empty() {
                return Ok(());
            }
            shared
                .files
                .iter()
                .map(|file| {
                    format_row_report(
                        &file.name,
                        &file.path.to_string_lossy(),
                        &file.size_label,
                        &file.hashes,
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        };
        copy_text(&self.shared, report, "Copied hash report to the clipboard")
    }

    fn set_expected_hash(&mut self, value: String) -> Result<()> {
        {
            let mut shared = self.shared();
            shared.expected_hash = value;
            refresh_matches(&mut shared);
        }
        publish_rows(&self.shared)?;
        publish_main(&self.shared)
    }

    fn set_include_md5(&mut self, value: bool) -> Result<()> {
        self.set_option(|options| options.md5 = value)
    }

    fn set_include_sha1(&mut self, value: bool) -> Result<()> {
        self.set_option(|options| options.sha1 = value)
    }

    fn set_include_sha256(&mut self, value: bool) -> Result<()> {
        self.set_option(|options| options.sha256 = value)
    }

    fn set_include_sha512(&mut self, value: bool) -> Result<()> {
        self.set_option(|options| options.sha512 = value)
    }

    fn set_include_blake3(&mut self, value: bool) -> Result<()> {
        self.set_option(|options| options.blake3 = value)
    }

    fn open_recent_file(&mut self, value: String) -> Result<()> {
        add_paths(&self.shared, &[PathBuf::from(value)], true)
    }

    fn exit(&mut self) -> Result<()> {
        match self.shared().scope.clone() {
            Some(scope) => scope.shutdown(),
            None => Ok(()),
        }
    }
}

impl Model {
    fn set_option(&mut self, update: impl FnOnce(&mut HashOptions)) -> Result<()> {
        {
            let mut shared = self.shared();
            let previous = shared.options;
            update(&mut shared.options);
            if !shared.options.any() {
                shared.options = previous;
            } else if shared.options != previous {
                let options = shared.options;
                for file in &mut shared.files {
                    file.hashes.clear_disabled(options);
                    if file.hashes.missing(options) && !file.status.starts_with("Failed") {
                        file.status = STATUS_WAITING.to_string();
                    }
                }
                refresh_matches(&mut shared);
                shared.hash_generation += 1;
            }
        }
        publish_rows(&self.shared)?;
        publish_main(&self.shared)?;
        start_worker(&self.shared)
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("file")
        .to_string()
}

fn digest_text(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "—".to_string())
}

fn file_count_label(count: usize) -> String {
    match count {
        0 => "No files".to_string(),
        1 => "1 file".to_string(),
        count => format!("{count} files"),
    }
}

fn compare_status(shared: &Shared) -> String {
    if hash::normalize_digest(&shared.expected_hash).is_empty() {
        return "Paste a digest to compare".to_string();
    }
    let hashed = shared
        .files
        .iter()
        .filter(|file| file.hashes.has_any())
        .count();
    if hashed == 0 {
        return "Waiting for hashes...".to_string();
    }
    let matches = shared
        .files
        .iter()
        .filter(|file| file.hashes.matched_algorithm(&shared.expected_hash).is_some())
        .count();
    if matches == 0 {
        "No files match".to_string()
    } else if matches == hashed {
        format!("All {hashed} files match")
    } else {
        format!("{matches} of {hashed} files match")
    }
}

fn overall_status(shared: &Shared) -> String {
    if shared.files.is_empty() {
        return shared.status.clone();
    }
    let hashing = shared
        .files
        .iter()
        .filter(|file| file.status == STATUS_HASHING || file.status == STATUS_WAITING)
        .count();
    let failed = shared
        .files
        .iter()
        .filter(|file| file.status.starts_with("Failed"))
        .count();
    let hashed = shared
        .files
        .iter()
        .filter(|file| file.status == STATUS_HASHED)
        .count();
    if hashing > 0 {
        if let Some(current) = shared
            .files
            .iter()
            .find(|file| file.status == STATUS_HASHING)
        {
            return format!("Hashing {}...", current.name);
        }
        return format!(
            "Waiting to hash {hashing} file{}...",
            if hashing == 1 { "" } else { "s" }
        );
    }
    match (hashed, failed) {
        (count, 0) if count == shared.files.len() => {
            format!("{count} file{} hashed", if count == 1 { "" } else { "s" })
        }
        (hashed, failed) if failed > 0 => format!("{hashed} hashed, {failed} failed"),
        _ => shared.status.clone(),
    }
}

fn primary_digest(shared: &Shared) -> String {
    shared
        .files
        .first()
        .and_then(|file| file.hashes.sha256.clone())
        .unwrap_or_default()
}

fn is_busy(shared: &Shared) -> bool {
    shared.worker_running
        || shared
            .files
            .iter()
            .any(|file| file.status == STATUS_HASHING || file.status == STATUS_WAITING)
}

fn refresh_matches(shared: &mut Shared) {
    let expected = shared.expected_hash.clone();
    for file in &mut shared.files {
        file.match_label = match_label(&expected, &file.hashes);
    }
}

fn publish_file_row(
    sink: &FileRowViewModelSink,
    file: &HashFile,
    options: HashOptions,
) -> Result<()> {
    sink.set_name(&file.name)?;
    sink.set_file_path(file.path.to_string_lossy().as_ref())?;
    sink.set_size_label(&file.size_label)?;
    sink.set_status(&file.status)?;
    sink.set_match_label(&file.match_label)?;
    sink.set_md5(digest_text(&file.hashes.md5))?;
    sink.set_sha1(digest_text(&file.hashes.sha1))?;
    sink.set_sha256(digest_text(&file.hashes.sha256))?;
    sink.set_sha512(digest_text(&file.hashes.sha512))?;
    sink.set_blake3(digest_text(&file.hashes.blake3))?;
    sink.set_show_md5(options.md5)?;
    sink.set_show_sha1(options.sha1)?;
    sink.set_show_sha256(options.sha256)?;
    sink.set_show_sha512(options.sha512)?;
    sink.set_show_blake3(options.blake3)?;
    sink.set_can_copy_sha256(file.hashes.sha256.is_some())?;
    Ok(())
}

fn publish_rows(shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let shared = shared.lock().expect("shared state lock poisoned");
    let options = shared.options;
    for file in &shared.files {
        if let Some(sink) = file.sink.as_ref() {
            publish_file_row(sink, file, options)?;
        }
    }
    Ok(())
}

fn remove_file(shared: &Arc<Mutex<Shared>>, id: u64) -> Result<()> {
    let (index, sink) = {
        let mut state = shared.lock().expect("shared state lock poisoned");
        let index = state.files.iter().position(|file| file.id == id);
        if let Some(index) = index {
            state.files.remove(index);
            state.hash_generation += 1;
        }
        (index, state.sink.clone())
    };
    let Some(index) = index else {
        return Ok(());
    };
    if let Some(sink) = sink {
        sink.remove_files(index as i32)?;
    }
    publish_main(shared)?;
    start_worker(shared)
}

fn republish_files(shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let (sink, ids) = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        for file in &mut shared.files {
            file.sink = None;
        }
        (
            shared.sink.clone(),
            shared.files.iter().map(|file| file.id).collect::<Vec<_>>(),
        )
    };
    let Some(sink) = sink else {
        return Ok(());
    };
    sink.clear_files()?;
    for id in ids {
        sink.add_files(FileRowModel {
            id,
            shared: shared.clone(),
        })?;
    }
    Ok(())
}

fn publish_main(shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let snapshot = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.status = overall_status(&shared);
        (
            shared.sink.clone(),
            shared.status.clone(),
            file_count_label(shared.files.len()),
            compare_status(&shared),
            shared.options,
            is_busy(&shared),
            !shared.files.is_empty(),
            shared.files.iter().any(|file| file.hashes.has_any()),
            primary_digest(&shared),
            shared.files.is_empty(),
            shared.recent.clone(),
        )
    };
    let Some(sink) = snapshot.0 else {
        return Ok(());
    };
    sink.set_title(APP_TITLE)?;
    sink.set_status(&snapshot.1)?;
    sink.set_file_count_label(&snapshot.2)?;
    sink.set_compare_status(&snapshot.3)?;
    sink.set_include_md5(snapshot.4.md5)?;
    sink.set_include_sha1(snapshot.4.sha1)?;
    sink.set_include_sha256(snapshot.4.sha256)?;
    sink.set_include_sha512(snapshot.4.sha512)?;
    sink.set_include_blake3(snapshot.4.blake3)?;
    sink.set_is_busy(snapshot.5)?;
    sink.set_can_clear(snapshot.6)?;
    sink.set_can_copy_report(snapshot.7)?;
    sink.set_primary_digest(&snapshot.8)?;
    sink.set_empty_visible(snapshot.9)?;
    sink.publish_recent_files(&snapshot.10)?;
    Ok(())
}

fn set_status(shared: &Arc<Mutex<Shared>>, status: impl Into<String>) -> Result<()> {
    {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.status = status.into();
    }
    publish_main(shared)
}

fn copy_text(shared: &Arc<Mutex<Shared>>, text: String, ok: &str) -> Result<()> {
    let (scope, window, sink) = {
        let shared = shared.lock().expect("shared state lock poisoned");
        let (Some(scope), Some(window), Some(sink)) = (
            shared.scope.clone(),
            shared.window.clone(),
            shared.sink.clone(),
        ) else {
            return Ok(());
        };
        (scope, window, sink)
    };
    let operation = scope.clipboard_write(&window, &ClipboardData::text(text))?;
    let ok = ok.to_string();
    scope.spawn(async move {
        let status = match operation.await {
            Ok(()) => ok,
            Err(error) => format!("Clipboard write failed: {error}"),
        };
        let _ = sink.set_status(status);
    })
}

fn add_paths(shared: &Arc<Mutex<Shared>>, paths: &[PathBuf], remember: bool) -> Result<()> {
    let mut added = 0;
    {
        let mut state = shared.lock().expect("shared state lock poisoned");
        for path in paths {
            if state.files.iter().any(|file| file.path == *path) {
                continue;
            }
            let metadata = match std::fs::metadata(path) {
                Ok(metadata) if metadata.is_file() => metadata,
                Ok(_) => {
                    state.status = format!("{} is not a file", display_name(path));
                    continue;
                }
                Err(error) => {
                    state.status = format!("Unable to read {}: {error}", display_name(path));
                    continue;
                }
            };
            let id = state.next_id;
            state.next_id += 1;
            let expected = state.expected_hash.clone();
            state.files.push(HashFile {
                id,
                path: path.clone(),
                name: display_name(path),
                size_label: format_size(metadata.len()),
                status: STATUS_WAITING.to_string(),
                hashes: FileHashes::default(),
                match_label: match_label(&expected, &FileHashes::default()),
                sink: None,
            });
            if remember {
                state.recent.push(path.to_string_lossy().to_string());
            }
            added += 1;
        }
        if added > 0 {
            state.hash_generation += 1;
        }
    }
    if added == 0 {
        return publish_main(shared);
    }
    republish_files(shared)?;
    publish_main(shared)?;
    start_worker(shared)
}

fn start_worker(shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let generation = {
        let mut state = shared.lock().expect("shared state lock poisoned");
        if state.worker_running {
            return Ok(());
        }
        if !state
            .files
            .iter()
            .any(|file| file.hashes.missing(state.options) && !file.status.starts_with("Failed"))
        {
            return Ok(());
        }
        state.worker_running = true;
        state.hash_generation
    };
    let worker_shared = shared.clone();
    std::thread::Builder::new()
        .name("hash-worker".to_string())
        .spawn(move || {
            loop {
                let next = {
                    let mut state = worker_shared.lock().expect("shared state lock poisoned");
                    if state.hash_generation != generation {
                        state.worker_running = false;
                        drop(state);
                        let _ = start_worker(&worker_shared);
                        return;
                    }
                    let options = state.options;
                    let index = state.files.iter().position(|file| {
                        file.hashes.missing(options) && !file.status.starts_with("Failed")
                    });
                    match index {
                        Some(index) => {
                            state.files[index].status = STATUS_HASHING.to_string();
                            Some((
                                state.files[index].id,
                                state.files[index].path.clone(),
                                options,
                            ))
                        }
                        None => {
                            state.worker_running = false;
                            None
                        }
                    }
                };
                let Some((id, path, options)) = next else {
                    let _ = publish_rows(&worker_shared);
                    let _ = publish_main(&worker_shared);
                    return;
                };
                let _ = publish_rows(&worker_shared);
                let _ = publish_main(&worker_shared);
                let result = hash_file(&path, options);
                {
                    let mut state = worker_shared.lock().expect("shared state lock poisoned");
                    if state.hash_generation != generation {
                        state.worker_running = false;
                        drop(state);
                        let _ = start_worker(&worker_shared);
                        return;
                    }
                    let expected = state.expected_hash.clone();
                    if let Some(file) = state.files.iter_mut().find(|file| file.id == id) {
                        match result {
                            Ok(hashes) => {
                                file.hashes = hashes;
                                file.status = STATUS_HASHED.to_string();
                                file.match_label = match_label(&expected, &file.hashes);
                            }
                            Err(error) => {
                                file.status = format!("Failed: {error}");
                            }
                        }
                    }
                }
                let _ = publish_rows(&worker_shared);
                let _ = publish_main(&worker_shared);
            }
        })
        .map(|_| ())
        .map_err(|error| Error::Load(format!("Unable to start hash worker: {error}")))
}

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let startup = scope.activation_items()?;
        let mut recent = RecentFileList::with_capacity(MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY);
        let startup_paths: Vec<PathBuf> = startup
            .iter()
            .filter_map(|item| item.local_path().map(Path::to_path_buf))
            .collect();
        let (initial_paths, create_sample, status) = if startup_paths.is_empty() {
            (
                vec![std::env::temp_dir().join(SAMPLE_FILE_NAME)],
                true,
                "Preparing sample file...".to_string(),
            )
        } else {
            for path in &startup_paths {
                recent.push(path.to_string_lossy().to_string());
            }
            (startup_paths, false, "Opening files...".to_string())
        };
        let shared = Arc::new(Mutex::new(Shared::new(status, recent)));
        mount_main_window(scope, Model::new(shared.clone()))?;
        {
            let mut state = shared.lock().expect("shared state lock poisoned");
            state.scope = Some(scope.clone());
            state.window = scope.main_window();
        }

        let activation_shared = shared.clone();
        scope.on_activation(move |event| {
            if let ActivationEvent::Files(items) = &event {
                let paths: Vec<PathBuf> = items
                    .iter()
                    .filter_map(|item| item.local_path().map(Path::to_path_buf))
                    .collect();
                if !paths.is_empty() {
                    let _ = add_paths(&activation_shared, &paths, true);
                }
            }
        })?;

        if create_sample {
            if let Some(path) = initial_paths.first() {
                if let Err(error) = write_sample_file(path) {
                    set_status(&shared, format!("Unable to create sample file: {error}"))?;
                    return Ok(());
                }
            }
        }
        add_paths(&shared, &initial_paths, false)?;
        Ok(())
    })
}

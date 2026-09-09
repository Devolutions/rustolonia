//! A small PDF viewer built as an external Rustolonia consumer.
//!
//! Rust owns PDF loading, page rendering and navigation state. The generated
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
mod pdf;

use avalonia::{
    ActivationEvent, App, FileTypeFilter, OpenFilePickerOptions, PickerOutcome, Window,
};
use base64::Engine;
use generated_view_models::{
    mount_main_window, MainViewModel, MainViewModelSink, MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY,
};
use pdf::{
    create_sample_pdf, load_page, render_page, search_document, DocumentState, PdfHandle, SearchHit,
};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc, Mutex,
};

const APP_TITLE: &str = "PDF Viewer";
static NEXT_LOAD_GENERATION: AtomicI64 = AtomicI64::new(0);
static NEXT_SEARCH_GENERATION: AtomicI64 = AtomicI64::new(0);

struct Shared {
    sink: Option<MainViewModelSink>,
    scope: Option<AppScope>,
    window: Option<Window>,
    document: Option<DocumentState>,
    status: String,
    loading: bool,
    recent: RecentFileList,
    load_generation: i64,
    search_text: String,
    search_status: String,
    search_match_label: String,
    can_go_previous_match: bool,
    can_go_next_match: bool,
    search_results: Vec<SearchHit>,
    search_index: usize,
    search_generation: i64,
}

impl Shared {
    fn new(status: String, recent: RecentFileList) -> Self {
        Self {
            sink: None,
            scope: None,
            window: None,
            status,
            loading: true,
            recent,
            document: None,
            load_generation: 0,
            search_text: String::new(),
            search_status: "Type a word to search".to_string(),
            search_match_label: String::new(),
            can_go_previous_match: false,
            can_go_next_match: false,
            search_results: Vec::new(),
            search_index: 0,
            search_generation: 0,
        }
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
        sink.publish_recent_files(&recent)
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
                .title("Open a PDF")
                .allow_multiple(false)
                .file_type(
                    FileTypeFilter::new("PDF documents")
                        .with_extension("pdf")
                        .with_mime_type("application/pdf"),
                ),
        )?;

        scope.spawn(async move {
            match operation.await {
                Ok(PickerOutcome::Selected(items)) => {
                    let Some(path) = items.iter().find_map(|item| item.local_path()) else {
                        let _ = set_status(
                            &shared,
                            "The selected PDF is not available as a local file.",
                        );
                        return;
                    };
                    let path = path.to_path_buf();
                    let name = display_name(&path);
                    let _ =
                        start_loading(&shared, path, 0, None, format!("Opening {name}..."), false);
                }
                Ok(PickerOutcome::Cancelled) => {}
                Err(error) => {
                    let _ = set_status(&shared, format!("Open dialog failed: {error}"));
                }
            }
        })
    }

    fn previous_page(&mut self) -> Result<()> {
        self.navigate(-1)
    }

    fn next_page(&mut self) -> Result<()> {
        self.navigate(1)
    }

    fn set_search_text(&mut self, value: String) -> Result<()> {
        let is_empty = value.trim().is_empty();
        {
            let mut shared = self.shared();
            shared.search_text = value;
        }
        if is_empty {
            clear_search_state(&self.shared)?;
        }
        Ok(())
    }

    fn search(&mut self) -> Result<()> {
        let (query, pdf) = {
            let shared = self.shared();
            (
                shared.search_text.trim().to_string(),
                shared
                    .document
                    .as_ref()
                    .map(|document| document.pdf.clone()),
            )
        };
        if query.is_empty() {
            return clear_search_state(&self.shared);
        }
        let Some(pdf) = pdf else {
            return publish_search_state(
                &self.shared,
                "Load a PDF before searching".to_string(),
                String::new(),
                false,
                false,
            );
        };
        start_search(&self.shared, pdf, query)
    }

    fn previous_match(&mut self) -> Result<()> {
        self.move_match(-1)
    }

    fn next_match(&mut self) -> Result<()> {
        self.move_match(1)
    }

    fn open_recent_file(&mut self, value: String) -> Result<()> {
        let path = PathBuf::from(value);
        let name = display_name(&path);
        start_loading(
            &self.shared,
            path,
            0,
            None,
            format!("Opening {name}..."),
            false,
        )
    }

    fn exit(&mut self) -> Result<()> {
        let scope = self.shared().scope.clone();
        match scope {
            Some(scope) => scope.shutdown(),
            None => Ok(()),
        }
    }
}

impl Model {
    fn navigate(&mut self, delta: isize) -> Result<()> {
        let (path, current_page, pdf) = {
            let shared = self.shared();
            let Some(document) = shared.document.as_ref() else {
                return Ok(());
            };
            (
                document.path.clone(),
                document.page,
                Some(document.pdf.clone()),
            )
        };
        let target = if delta.is_negative() {
            current_page.saturating_sub(delta.unsigned_abs())
        } else {
            current_page.saturating_add(delta as usize)
        };
        start_loading(
            &self.shared,
            path,
            target,
            pdf,
            format!("Rendering page {}...", target + 1),
            false,
        )
    }

    fn move_match(&mut self, delta: isize) -> Result<()> {
        let (hit, path, pdf, index, total) = {
            let mut shared = self.shared();
            if shared.search_results.is_empty() {
                return Ok(());
            }
            let total = shared.search_results.len();
            let current = shared.search_index as isize;
            let next = (current + delta).rem_euclid(total as isize) as usize;
            shared.search_index = next;
            let hit = shared.search_results[next].clone();
            let Some(document) = shared.document.as_ref() else {
                return Ok(());
            };
            (
                hit,
                document.path.clone(),
                document.pdf.clone(),
                next,
                total,
            )
        };
        publish_search_state(
            &self.shared,
            format!(
                "Match on page {}: {}",
                hit.page + 1,
                compact_match_text(&hit.text)
            ),
            format!("{} of {}", index + 1, total),
            total > 1,
            total > 1,
        )?;
        start_loading(
            &self.shared,
            path,
            hit.page,
            Some(pdf),
            format!("Seeking to match on page {}...", hit.page + 1),
            false,
        )
    }
}

fn publish_state(shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let (sink, document, status, loading) = {
        let shared = shared.lock().expect("shared state lock poisoned");
        (
            shared.sink.clone(),
            shared.document.clone(),
            shared.status.clone(),
            shared.loading,
        )
    };
    let Some(sink) = sink else {
        return Ok(());
    };

    sink.set_title(APP_TITLE)?;
    sink.set_status(status)?;
    sink.set_is_loading(loading)?;
    if let Some(document) = document {
        sink.set_document_name(document.name)?;
        sink.set_page_image(base64::engine::general_purpose::STANDARD.encode(document.image))?;
        sink.set_page_text(document.text)?;
        sink.set_page_label(format!(
            "Page {} of {}",
            document.page + 1,
            document.page_count
        ))?;
        sink.set_can_go_previous(document.page > 0)?;
        sink.set_can_go_next(document.page + 1 < document.page_count)?;
    } else {
        sink.set_document_name("")?;
        sink.set_page_image("")?;
        sink.set_page_text("")?;
        sink.set_page_label("No page loaded")?;
        sink.set_can_go_previous(false)?;
        sink.set_can_go_next(false)?;
    }
    Ok(())
}

fn set_status(shared: &Arc<Mutex<Shared>>, status: impl Into<String>) -> Result<()> {
    let (sink, status) = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.status = status.into();
        shared.loading = false;
        (shared.sink.clone(), shared.status.clone())
    };
    if let Some(sink) = sink {
        sink.set_status(status)?;
        sink.set_is_loading(false)?;
    }
    Ok(())
}

fn compact_match_text(text: &str) -> String {
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut compact = single_line.chars().take(72).collect::<String>();
    if single_line.chars().count() > 72 {
        compact.push_str("...");
    }
    compact
}

fn publish_search_state(
    shared: &Arc<Mutex<Shared>>,
    status: String,
    match_label: String,
    can_go_previous: bool,
    can_go_next: bool,
) -> Result<()> {
    let generation = NEXT_LOAD_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let sink = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.search_status = status.clone();
        shared.search_match_label = match_label.clone();
        shared.can_go_previous_match = can_go_previous;
        shared.can_go_next_match = can_go_next;
        shared.sink.clone()
    };
    if let Some(sink) = sink {
        let mut batch = sink.batch(generation);
        batch.set_search_status(status);
        batch.set_search_match_label(match_label);
        batch.set_can_go_previous_match(can_go_previous);
        batch.set_can_go_next_match(can_go_next);
        sink.submit_batch(batch).map(|_| ())?;
    }
    Ok(())
}

fn clear_search_state(shared: &Arc<Mutex<Shared>>) -> Result<()> {
    NEXT_SEARCH_GENERATION.fetch_add(1, Ordering::Relaxed);
    let mut shared_state = shared.lock().expect("shared state lock poisoned");
    shared_state.search_results.clear();
    shared_state.search_index = 0;
    drop(shared_state);
    publish_search_state(
        shared,
        "Type a word to search".to_string(),
        String::new(),
        false,
        false,
    )
}

fn start_search(shared: &Arc<Mutex<Shared>>, pdf: PdfHandle, query: String) -> Result<()> {
    let search_generation = NEXT_SEARCH_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    {
        let mut shared_state = shared.lock().expect("shared state lock poisoned");
        shared_state.search_generation = search_generation;
        shared_state.search_results.clear();
        shared_state.search_index = 0;
    }
    publish_search_state(
        shared,
        "Searching document...".to_string(),
        String::new(),
        false,
        false,
    )?;

    let worker_shared = shared.clone();
    std::thread::Builder::new()
        .name("pdf-search".to_string())
        .spawn(move || match search_document(&pdf, &query) {
            Ok(results) => {
                if let Err(error) =
                    apply_search_results(&worker_shared, query, results, search_generation)
                {
                    eprintln!("PDF viewer search update failed: {error}");
                }
            }
            Err(error) => {
                if let Err(update_error) =
                    set_search_error(&worker_shared, search_generation, error)
                {
                    eprintln!("PDF viewer search error update failed: {update_error}");
                }
            }
        })
        .map(|_| ())
        .map_err(|error| Error::Load(format!("Unable to start PDF search worker: {error}")))
}

fn apply_search_results(
    shared: &Arc<Mutex<Shared>>,
    query: String,
    results: Vec<SearchHit>,
    search_generation: i64,
) -> Result<()> {
    let (hit, path, pdf, count) = {
        let mut shared_state = shared.lock().expect("shared state lock poisoned");
        if shared_state.search_generation != search_generation {
            return Ok(());
        }
        let count = results.len();
        shared_state.search_results = results;
        shared_state.search_index = 0;
        let Some(document) = shared_state.document.as_ref() else {
            return Ok(());
        };
        (
            shared_state.search_results.first().cloned(),
            document.path.clone(),
            document.pdf.clone(),
            count,
        )
    };

    if count == 0 {
        return publish_search_state(
            shared,
            format!("No matches for \"{}\"", compact_match_text(&query)),
            "0 matches".to_string(),
            false,
            false,
        );
    }

    let hit = hit.expect("a non-empty search result set has a first hit");
    publish_search_state(
        shared,
        format!(
            "Found {count} matches for \"{}\"",
            compact_match_text(&query)
        ),
        format!("1 of {count}"),
        count > 1,
        count > 1,
    )?;
    start_loading(
        shared,
        path,
        hit.page,
        Some(pdf),
        format!("Seeking to match on page {}...", hit.page + 1),
        false,
    )
}

fn set_search_error(
    shared: &Arc<Mutex<Shared>>,
    search_generation: i64,
    error: String,
) -> Result<()> {
    {
        let shared_state = shared.lock().expect("shared state lock poisoned");
        if shared_state.search_generation != search_generation {
            return Ok(());
        }
    }
    publish_search_state(
        shared,
        format!("Search failed: {error}"),
        String::new(),
        false,
        false,
    )
}

fn set_error(
    shared: &Arc<Mutex<Shared>>,
    path: &Path,
    error: String,
    generation: i64,
) -> Result<()> {
    let message = format!("Unable to open {}: {error}", path.display());
    let sink = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        if shared.load_generation != generation {
            return Ok(());
        }
        shared.status = message.clone();
        shared.loading = false;
        shared.sink.clone()
    };
    if let Some(sink) = sink {
        let mut batch = sink.batch(generation);
        batch.set_status(message);
        batch.set_is_loading(false);
        sink.submit_batch(batch).map(|_| ())?;
    }
    Ok(())
}

fn apply_document(
    shared: &Arc<Mutex<Shared>>,
    document: DocumentState,
    generation: i64,
) -> Result<()> {
    let image = base64::engine::general_purpose::STANDARD.encode(&document.image);
    let text = document.text.clone();
    let name = document.name.clone();
    let page_label = format!("Page {} of {}", document.page + 1, document.page_count);
    let can_go_previous = document.page > 0;
    let can_go_next = document.page + 1 < document.page_count;
    let (sink, recent, status) = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        if shared.load_generation != generation {
            return Ok(());
        }
        shared.status = format!(
            "Loaded {} with {} page{}",
            document.name,
            document.page_count,
            if document.page_count == 1 { "" } else { "s" }
        );
        shared.loading = false;
        shared
            .recent
            .push(document.path.to_string_lossy().to_string());
        shared.document = Some(document);
        (
            shared.sink.clone(),
            shared.recent.clone(),
            shared.status.clone(),
        )
    };
    if let Some(sink) = sink {
        let mut batch = sink.batch(generation);
        batch.set_title(APP_TITLE);
        batch.set_status(status);
        batch.set_document_name(name);
        batch.set_page_image(image);
        batch.set_page_text(text);
        batch.set_page_label(page_label);
        batch.set_can_go_previous(can_go_previous);
        batch.set_can_go_next(can_go_next);
        batch.set_is_loading(false);
        batch.set_recent_files(&recent);
        sink.submit_batch(batch).map(|_| ())?;
    }
    Ok(())
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("document.pdf")
        .to_string()
}

fn set_loading_status(shared: &Arc<Mutex<Shared>>, status: String) -> Result<i64> {
    let (sink, generation) = {
        let mut shared = shared.lock().expect("shared state lock poisoned");
        shared.load_generation = NEXT_LOAD_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        shared.status = status.clone();
        shared.loading = true;
        (shared.sink.clone(), shared.load_generation)
    };
    if let Some(sink) = sink {
        sink.set_status(status)?;
        sink.set_is_loading(true)?;
    }
    Ok(generation)
}

fn start_loading(
    shared: &Arc<Mutex<Shared>>,
    path: PathBuf,
    page: usize,
    pdf: Option<PdfHandle>,
    status: String,
    create_sample: bool,
) -> Result<()> {
    let generation = set_loading_status(shared, status)?;
    let worker_shared = shared.clone();
    let worker_path = path.clone();
    std::thread::Builder::new()
        .name("pdf-loader".to_string())
        .spawn(move || {
            let result = if create_sample {
                create_sample_pdf(&worker_path).and_then(|_| load_page(&worker_path, page))
            } else if let Some(pdf) = pdf {
                render_page(pdf, &worker_path, page)
            } else {
                load_page(&worker_path, page)
            };
            match result {
                Ok(document) => {
                    if let Err(error) = apply_document(&worker_shared, document, generation) {
                        eprintln!("PDF viewer update failed: {error}");
                    }
                }
                Err(error) => {
                    if let Err(update_error) =
                        set_error(&worker_shared, &worker_path, error, generation)
                    {
                        eprintln!("PDF viewer error update failed: {update_error}");
                    }
                }
            }
        })
        .map(|_| ())
        .map_err(|error| Error::Load(format!("Unable to start PDF worker: {error}")))
}

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let startup = scope.activation_items()?;
        let mut recent = RecentFileList::with_capacity(MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY);
        let (path, create_sample, status) =
            if let Some(path) = startup.iter().find_map(|item| item.local_path()) {
                let path = path.to_path_buf();
                (
                    path.clone(),
                    false,
                    format!("Opening {}...", display_name(&path)),
                )
            } else {
                (
                    std::env::temp_dir().join("rustolonia-pdf-viewer-sample.pdf"),
                    true,
                    "Preparing sample PDF...".to_string(),
                )
            };
        if !create_sample {
            recent.push(path.to_string_lossy().to_string());
        }
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
                    let path = path.to_path_buf();
                    let name = display_name(&path);
                    let _ = start_loading(
                        &activation_shared,
                        path,
                        0,
                        None,
                        format!("Opening {name}..."),
                        false,
                    );
                }
            }
        })?;
        start_loading(&shared, path, 0, None, status, create_sample)?;
        Ok(())
    })
}

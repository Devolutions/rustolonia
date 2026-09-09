//! Desktop file integration for the flagship Rust VM + AXAML sample.
//!
//! Rust owns the state and the file operations; the managed compiled AXAML only
//! presents them. The pickers are tied to the window the host created for the
//! mounted view model, and every result is published back through the generated
//! `SampleViewModelSink`.
//!
//! Automation note: nothing here opens a dialog on its own. The open/save
//! commands are only reachable by clicking their buttons, so UI automation can
//! exercise the drop target, the activation status and every other part of the
//! sample without a modal dialog appearing. The picker flows themselves are
//! covered by the managed fake-storage-provider tests in
//! `tests/Avalonia.Host.Tests/Desktop/DesktopFilePickerTests.cs`.
//!
//! The drop subscription targets the mounted window rather than the status
//! panel, because Rust owns state and not presentation: it has no handle to a
//! control declared inside the compiled AXAML. Dropping anywhere in the window
//! is therefore accepted, and the panel reports what happened.

use avalonia::{
    ActivationEvent, AppScope, ClipboardData, DragDropEffects, FileDropEvent, FileTypeFilter,
    FolderPickerOptions, OpenFilePickerOptions, PickerOutcome, RecentFileList,
    SaveFilePickerOptions, StorageItem, Window,
};
use avalonia_sample::{SampleViewModelSink, SAMPLE_VIEW_MODEL_RECENT_FILES_CAPACITY};
use std::sync::{Arc, Mutex};

/// Conservative accepted effects for the sample's drop target. The host answers
/// the platform with the intersection of this mask and the allowed effects, so
/// the sample never claims a move it does not perform.
pub const ACCEPTED_DROP_EFFECTS: DragDropEffects = DragDropEffects::COPY;

/// Everything the sample needs to run a picker: the application scope, the
/// window the picker is parented to, and the sink to publish results through.
///
/// The three parts arrive at different times. The sink arrives when the model
/// attaches (inside the mount call); the scope and window arrive right after
/// mounting, because the host creates the window from the managed view
/// registry. Picker commands are no-ops until all three are present.
#[derive(Default)]
pub struct DesktopFiles {
    state: Mutex<DesktopFilesState>,
}

struct DesktopFilesState {
    scope: Option<AppScope>,
    window: Option<Window>,
    sink: Option<SampleViewModelSink>,
    /// Stage 31: Rust owns the most-recently-used list. Entries are stage 29
    /// storage URIs, so the same value identifies a picked file, a dropped
    /// file and an "open with" activation, and the generated menu derives its
    /// header from it.
    recent: RecentFileList,
}

impl Default for DesktopFilesState {
    fn default() -> Self {
        Self {
            scope: None,
            window: None,
            sink: None,
            recent: RecentFileList::with_capacity(SAMPLE_VIEW_MODEL_RECENT_FILES_CAPACITY),
        }
    }
}

struct Ready {
    scope: AppScope,
    window: Window,
    sink: SampleViewModelSink,
}

/// Which picker a command runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickerKind {
    OpenFiles,
    OpenFolder,
    SaveExport,
}

impl DesktopFiles {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Records the model's sink; called from `SampleViewModel::attach`/`detach`.
    pub fn set_sink(&self, sink: Option<SampleViewModelSink>) {
        self.lock().sink = sink;
    }

    /// Records the mounted window and application scope.
    pub fn attach(&self, scope: &AppScope, window: Window) {
        let mut state = self.lock();
        state.scope = Some(scope.clone());
        state.window = Some(window);
    }

    /// Publishes the ordered startup activation items ("open with" at launch).
    pub fn publish_activation_items(&self, items: &[StorageItem]) {
        let Some(ready) = self.ready() else {
            return;
        };
        let _ = publish_items(&ready.sink, items, "startup");
        self.remember(items);
        let _ = ready.sink.set_activation_status(activation_summary(items));
    }

    /// Publishes a post-startup activation (macOS "open with" while running,
    /// protocol activation, dock reopen).
    pub fn publish_activation_event(&self, event: &ActivationEvent) {
        let Some(ready) = self.ready() else {
            return;
        };
        let items = event.items();
        if !items.is_empty() {
            let _ = publish_items(&ready.sink, items, "activation");
            self.remember(items);
        }
        let _ = ready
            .sink
            .set_activation_status(format!("{event:?}: {}", activation_summary(items)));
    }

    /// Publishes one incoming drag notification.
    pub fn publish_drop_event(&self, event: &FileDropEvent) {
        let Some(ready) = self.ready() else {
            return;
        };
        let status = match event {
            FileDropEvent::Enter { items, .. } => format!(
                "Drag entered with {} item(s), accepted: {}",
                items.len(),
                event.accepted()
            ),
            // Over fires continuously; it would only churn the status text.
            FileDropEvent::Over { .. } => return,
            FileDropEvent::Leave => "Drag left the panel".to_string(),
            FileDropEvent::Drop { items, .. } => {
                let _ = publish_items(&ready.sink, items, "drop");
                self.remember(items);
                format!("Dropped {} item(s)", items.len())
            }
        };
        let _ = ready.sink.set_drop_status(status);
    }

    pub fn open_files(self: &Arc<Self>) -> avalonia::Result<()> {
        self.start(PickerKind::OpenFiles)
    }
    pub fn open_folder(self: &Arc<Self>) -> avalonia::Result<()> {
        self.start(PickerKind::OpenFolder)
    }

    pub fn save_export(self: &Arc<Self>) -> avalonia::Result<()> {
        self.start(PickerKind::SaveExport)
    }

    fn start(self: &Arc<Self>, kind: PickerKind) -> avalonia::Result<()> {
        let Some(ready) = self.ready() else {
            return Ok(());
        };
        let scope = &ready.scope;
        let window = &ready.window;
        let operation = match kind {
            PickerKind::OpenFiles => scope.open_file_picker(
                window,
                &OpenFilePickerOptions::new()
                    .title("Open files")
                    .allow_multiple(true)
                    .file_type(log_file_type())
                    .file_type(FileTypeFilter::new("All files").with_pattern("*.*")),
            )?,
            PickerKind::OpenFolder => scope
                .open_folder_picker(window, &FolderPickerOptions::new().title("Open folder"))?,
            PickerKind::SaveExport => scope.save_file_picker(
                window,
                &SaveFilePickerOptions::new()
                    .title("Export trace")
                    .suggested_file_name("trace-export.log")
                    .default_extension("log")
                    .show_overwrite_prompt(true)
                    .file_type(log_file_type()),
            )?,
        };

        // Async command semantics: disable the three file commands while a
        // dialog is open so the sample cannot stack modal pickers.
        set_commands_enabled(&ready.sink, false);
        let _ = ready
            .sink
            .set_file_status(format!("{kind:?} picker open..."));

        let sink = ready.sink.clone();
        let handle = self.clone();
        ready.scope.spawn(async move {
            let outcome = operation.await;
            let status = match &outcome {
                Ok(PickerOutcome::Cancelled) => format!("{kind:?} cancelled"),
                Ok(PickerOutcome::Selected(items)) => {
                    let _ = publish_items(&sink, items, "picker");
                    handle.remember(items);
                    format!("{kind:?} selected {} item(s)", items.len())
                }
                // Cancellation is not an error; anything reaching here really is
                // one, including an operation aborted at shutdown.
                Err(error) => format!("{kind:?} failed: {error}"),
            };
            let _ = sink.set_file_status(status);
            set_commands_enabled(&sink, true);
        })
    }

    /// Stage 31 command surface: recent files and the clipboard.
    ///
    /// Every clipboard operation is asynchronous, because a platform clipboard
    /// read can block for as long as the owning application takes to render the
    /// requested format. The future is awaited on the application scope's
    /// executor and the outcome is published back through the same sink, so the
    /// UI thread is never blocked by a menu command.
    pub fn remember(&self, items: &[StorageItem]) {
        let Some(ready) = self.ready() else {
            return;
        };
        let changed = {
            let mut state = self.lock();
            state.recent.extend_items(items).is_changed()
        };
        if !changed {
            return;
        }

        let state = self.lock();
        let _ = ready.sink.publish_recent_files(&state.recent);
    }

    pub fn open_recent(&self, uri: String) -> avalonia::Result<()> {
        let Some(ready) = self.ready() else {
            return Ok(());
        };
        let changed = {
            let mut state = self.lock();
            state.recent.push(uri.clone()).is_changed()
        };
        if changed {
            let state = self.lock();
            ready.sink.publish_recent_files(&state.recent)?;
        }

        ready.sink.set_file_status(format!("Reopened {uri}"))
    }

    pub fn copy_text(self: &Arc<Self>, text: String, cut: bool) -> avalonia::Result<()> {
        let Some(ready) = self.ready() else {
            return Ok(());
        };
        let verb = if cut { "Cut" } else { "Copied" };
        let length = text.chars().count();
        let operation = ready
            .scope
            .clipboard_write(&ready.window, &ClipboardData::text(text))?;
        let sink = ready.sink.clone();
        ready.scope.spawn(async move {
            let status = match operation.await {
                Ok(()) => format!("{verb} {length} character(s) to the clipboard"),
                Err(error) => format!("Clipboard write failed: {error}"),
            };
            let _ = sink.set_clipboard_status(status);
        })
    }

    pub fn paste(self: &Arc<Self>) -> avalonia::Result<()> {
        let Some(ready) = self.ready() else {
            return Ok(());
        };
        let text = ready.scope.clipboard_get_text(&ready.window)?;
        let files = ready.scope.clipboard_read_files(&ready.window)?;
        let sink = ready.sink.clone();
        let handle = self.clone();
        ready.scope.spawn(async move {
            let status = match text.await {
                Ok(Some(value)) => format!("Pasted {} character(s)", value.chars().count()),
                Ok(None) => "Clipboard has no text".to_string(),
                Err(error) => format!("Clipboard read failed: {error}"),
            };
            // File entries are optional: a clipboard with no files completes
            // successfully with an empty list, which is not a failure.
            let status = match files.await {
                Ok(items) if !items.is_empty() => {
                    let _ = publish_items(&sink, &items, "clipboard");
                    handle.remember(&items);
                    format!("{status}; {} file(s) on the clipboard", items.len())
                }
                Ok(_) => status,
                Err(error) => format!("{status}; file read failed: {error}"),
            };
            let _ = sink.set_clipboard_status(status);
        })
    }

    pub fn clear_clipboard(self: &Arc<Self>) -> avalonia::Result<()> {
        let Some(ready) = self.ready() else {
            return Ok(());
        };
        let operation = ready.scope.clipboard_clear(&ready.window)?;
        let sink = ready.sink.clone();
        ready.scope.spawn(async move {
            let status = match operation.await {
                Ok(()) => "Clipboard cleared".to_string(),
                Err(error) => format!("Clipboard clear failed: {error}"),
            };
            let _ = sink.set_clipboard_status(status);
        })
    }

    pub fn exit(&self) -> avalonia::Result<()> {
        let Some(ready) = self.ready() else {
            return Ok(());
        };
        ready.scope.shutdown()
    }

    fn ready(&self) -> Option<Ready> {
        let state = self.lock();
        Some(Ready {
            scope: state.scope.clone()?,
            window: state.window.clone()?,
            sink: state.sink.clone()?,
        })
    }
    fn lock(&self) -> std::sync::MutexGuard<'_, DesktopFilesState> {
        self.state.lock().expect("desktop file state lock poisoned")
    }
}

/// One filter that works on every desktop platform: glob patterns for
/// Windows/Linux, a MIME type for Linux, and a uniform type identifier for
/// Apple platforms.
fn log_file_type() -> FileTypeFilter {
    FileTypeFilter::new("Log files")
        .with_extension("log")
        .with_mime_type("text/plain")
        .with_apple_uniform_type_identifier("public.plain-text")
}

fn set_commands_enabled(sink: &SampleViewModelSink, enabled: bool) {
    let _ = sink.set_open_files_enabled(enabled);
    let _ = sink.set_open_folder_enabled(enabled);
    let _ = sink.set_save_export_enabled(enabled);
}

/// Publishes selected/dropped/activated items, showing the URI whenever the
/// platform has no local path for the item.
fn publish_items(
    sink: &SampleViewModelSink,
    items: &[StorageItem],
    origin: &str,
) -> avalonia::Result<()> {
    for item in items {
        let location = item
            .local_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| item.uri().to_string());
        let kind = if item.is_folder() { "folder" } else { "file" };
        sink.add_selected_files(format!("[{origin}] {kind}: {location}"))?;
    }
    Ok(())
}

fn activation_summary(items: &[StorageItem]) -> String {
    if items.is_empty() {
        return "No startup files".to_string();
    }
    format!("{} activation item(s)", items.len())
}

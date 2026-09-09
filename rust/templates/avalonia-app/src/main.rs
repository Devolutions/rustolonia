//! This crate owns Rust state; `generated/generated_view_models.rs` is emitted
//! from `view-model.ir.json` by the repository-owned consumer build tool.

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

use avalonia::{ActivationEvent, App, StorageItem, Window};
use generated_view_models::{
    mount_main_window, MainViewModel, MainViewModelSink, MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY,
};
use std::sync::{Arc, Mutex};

/// State the application scope and the mounted window are published into after
/// mounting, so a menu command can reach them. The sink arrives first, while
/// the model attaches inside the mount call.
#[derive(Default)]
struct Shared {
    sink: Option<MainViewModelSink>,
    scope: Option<AppScope>,
    window: Option<Window>,
    title: String,
}

struct Model {
    shared: Arc<Mutex<Shared>>,
    /// Rust owns the most-recently-used list; the generated File menu projects
    /// it and hands the chosen URI back as a command parameter.
    recent: RecentFileList,
    title: String,
}

impl Model {
    fn new(shared: Arc<Mutex<Shared>>, startup: &[StorageItem]) -> Self {
        let mut recent = RecentFileList::with_capacity(MAIN_VIEW_MODEL_RECENT_FILES_CAPACITY);
        recent.extend_items(startup);
        Self {
            shared,
            recent,
            title: describe(startup),
        }
    }

    fn shared(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().expect("shared state lock poisoned")
    }
}

impl MainViewModel for Model {
    fn attach(&mut self, sink: MainViewModelSink) -> Result<()> {
        sink.set_title(&self.title)?;
        sink.publish_recent_files(&self.recent)?;
        let mut shared = self.shared();
        shared.title = self.title.clone();
        shared.sink = Some(sink);
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        self.shared().sink = None;
        Ok(())
    }

    /// Copies the current title. Clipboard operations are asynchronous, so the
    /// UI thread is never blocked waiting on the platform.
    fn copy_title(&mut self) -> Result<()> {
        let (sink, scope, window, title) = {
            let shared = self.shared();
            let (Some(sink), Some(scope), Some(window)) = (
                shared.sink.clone(),
                shared.scope.clone(),
                shared.window.clone(),
            ) else {
                return Ok(());
            };
            (sink, scope, window, shared.title.clone())
        };
        let operation = scope.clipboard_write(&window, &ClipboardData::text(title))?;
        scope.spawn(async move {
            let status = match operation.await {
                Ok(()) => "Copied the title to the clipboard".to_string(),
                Err(error) => format!("Clipboard write failed: {error}"),
            };
            let _ = sink.set_status(status);
        })
    }

    /// Invoked by the generated recent-files submenu with the chosen URI.
    fn open_recent_file(&mut self, value: String) -> Result<()> {
        let changed = self.recent.push(value.clone()).is_changed();
        let shared = self.shared();
        let Some(sink) = shared.sink.as_ref() else {
            return Ok(());
        };
        if changed {
            sink.publish_recent_files(&self.recent)?;
        }
        sink.set_status(format!("Reopened {value}"))
    }

    fn exit(&mut self) -> Result<()> {
        let scope = self.shared().scope.clone();
        match scope {
            Some(scope) => scope.shutdown(),
            None => Ok(()),
        }
    }
}

/// Describes the documents the desktop shell launched this application with.
///
/// `App::run` forwards this process's arguments to the managed desktop
/// lifetime, so a registered file association (see `file-associations/`)
/// reaches Rust here with no extra wiring. `uri()` is always available;
/// `local_path()` is `None` for non-local documents.
fn describe(items: &[StorageItem]) -> String {
    if items.is_empty() {
        return "Hello from an external Rust consumer".to_string();
    }
    let names: Vec<&str> = items.iter().map(StorageItem::name).collect();
    format!("Opened {}: {}", items.len(), names.join(", "))
}

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let startup = scope.activation_items()?;
        mount_main_window(scope, Model::new(shared.clone(), &startup))?;

        {
            let mut state = shared.lock().expect("shared state lock poisoned");
            state.scope = Some(scope.clone());
            state.window = scope.main_window();
        }

        // Later "open with" activations, where the desktop lifetime supports
        // them (macOS while the application is already running).
        let activation_shared = shared.clone();
        scope.on_activation(move |event| {
            if let ActivationEvent::Files(items) = &event {
                let mut state = activation_shared
                    .lock()
                    .expect("shared state lock poisoned");
                state.title = describe(items);
                if let Some(sink) = state.sink.as_ref() {
                    let _ = sink.set_title(&state.title);
                }
            }
        })
    })
}

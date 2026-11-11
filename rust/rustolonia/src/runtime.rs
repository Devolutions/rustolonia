use crate::async_runtime::{decode_none, decode_string, TaskScope};
use crate::reactive::{on_scope_cleanup, RawSignalId, UiCallbackHandle};
use crate::{AsyncOperation, Error, Result, Signal, Window};
use rustolonia_sys as sys;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Return-type polymorphism for UI event handlers, allowing callbacks to either
/// return `()` or propagate `rustolonia::Result<()>` via `?`.
pub trait IntoEventResult {
    fn into_event_result(self) -> Result<()>;
}

impl IntoEventResult for () {
    fn into_event_result(self) -> Result<()> {
        Ok(())
    }
}

impl IntoEventResult for Result<()> {
    fn into_event_result(self) -> Result<()> {
        self
    }
}

type UiErrorHandler = Arc<dyn Fn(&str, &Error) + Send + Sync>;

thread_local! {
    static UI_ERROR_HANDLER: RefCell<Option<UiErrorHandler>> = const { RefCell::new(None) };
}

pub(crate) fn report_ui_error(context: &str, error: &Error) {
    let custom = UI_ERROR_HANDLER.with(|slot| slot.borrow().clone());
    if let Some(handler) = custom {
        handler(context, error);
    } else {
        eprintln!("{context}: {error}");
    }
}

/// Installs an error handler for UI callbacks/effects for the duration of `run`.
pub fn with_ui_error_handler<R>(
    handler: impl Fn(&str, &Error) + Send + Sync + 'static,
    run: impl FnOnce() -> R,
) -> R {
    let handler: UiErrorHandler = Arc::new(handler);
    let previous = UI_ERROR_HANDLER.with(|slot| slot.borrow_mut().replace(handler));
    struct RestoreGuard(Option<UiErrorHandler>);
    impl Drop for RestoreGuard {
        fn drop(&mut self) {
            let prev = self.0.take();
            UI_ERROR_HANDLER.with(|slot| {
                *slot.borrow_mut() = prev;
            });
        }
    }
    let _guard = RestoreGuard(previous);
    run()
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeVariant {
    Default = 0,
    Light = 1,
    Dark = 2,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResourceValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Double(f64),
    String(String),
    Color(u32),
}

#[derive(Clone)]
pub struct LocalizationResources {
    state: Arc<LocalizationState>,
}

struct LocalizationState {
    default_culture: String,
    current_culture: RawSignalId,
    catalog: Mutex<LocalizationCatalog>,
}

#[derive(Default)]
struct LocalizationCatalog {
    values: HashMap<(String, String), String>,
    signals: Vec<(String, RawSignalId)>,
    revision: u64,
}

impl LocalizationResources {
    /// Creates a Rust-owned string catalog with a required fallback culture.
    ///
    /// Read and update the catalog on the Avalonia UI thread, like other
    /// [`Signal`] values.
    pub fn new(default_culture: impl Into<String>) -> Result<Self> {
        let default_culture = default_culture.into();
        if default_culture.trim().is_empty() {
            return Err(Error::InvalidResourceCatalog(
                "localization default culture cannot be empty".to_string(),
            ));
        }
        Ok(Self {
            state: Arc::new(LocalizationState {
                current_culture: Signal::new(default_culture.clone()).raw_id(),
                default_culture,
                catalog: Mutex::new(LocalizationCatalog::default()),
            }),
        })
    }

    /// Adds or replaces a localized string and refreshes existing string signals.
    pub fn insert(
        &self,
        culture: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<()> {
        let culture = culture.into();
        let key = key.into();
        if culture.trim().is_empty() || key.trim().is_empty() {
            return Err(Error::InvalidResourceCatalog(
                "localization culture and key cannot be empty".to_string(),
            ));
        }
        let current_culture =
            Signal::<String>::try_from_raw(self.state.current_culture)?.get_untracked();
        let (revision, updates) = {
            let mut catalog = lock_localization_catalog(&self.state)?;
            catalog.values.insert((culture, key), value.into());
            let updates = localized_updates(&self.state, &catalog, &current_culture)?;
            catalog.revision = catalog.revision.wrapping_add(1);
            (catalog.revision, updates)
        };
        self.apply_updates(revision, updates)
    }

    /// Changes the active culture; missing translations fall back to the default culture.
    pub fn set_culture(&self, culture: impl Into<String>) -> Result<()> {
        let culture = culture.into();
        if culture.trim().is_empty() {
            return Err(Error::InvalidResourceCatalog(
                "localization culture cannot be empty".to_string(),
            ));
        }
        let current_sig = Signal::<String>::try_from_raw(self.state.current_culture)?;
        let (revision, updates) = {
            let mut catalog = lock_localization_catalog(&self.state)?;
            let updates = localized_updates(&self.state, &catalog, &culture)?;
            catalog.revision = catalog.revision.wrapping_add(1);
            (catalog.revision, updates)
        };
        current_sig.set_if_changed(culture);
        self.apply_updates(revision, updates)
    }

    /// Returns the active culture identifier.
    pub fn culture(&self) -> String {
        Signal::<String>::try_from_raw(self.state.current_culture)
            .expect("LocalizationResources::culture must be called on the UI thread")
            .get()
    }

    /// Creates a reactive string signal, returning an error if the key has no
    /// value in either the active or default culture.
    pub fn string_signal(&self, key: impl Into<String>) -> Result<Signal<String>> {
        let key = key.into();
        if key.trim().is_empty() {
            return Err(Error::InvalidResourceCatalog(
                "localization key cannot be empty".to_string(),
            ));
        }
        let culture = Signal::<String>::try_from_raw(self.state.current_culture)?.get_untracked();
        let mut catalog = lock_localization_catalog(&self.state)?;
        let initial = localized_value(&self.state, &catalog, &key, &culture)?;
        let signal = Signal::new(initial);
        let raw_signal = signal.raw_id();
        catalog.signals.push((key, raw_signal));
        drop(catalog);
        let state = Arc::downgrade(&self.state);
        on_scope_cleanup(move || {
            if let Some(state) = state.upgrade() {
                let mut catalog = match state.catalog.lock() {
                    Ok(catalog) => catalog,
                    Err(poisoned) => {
                        eprintln!("localization catalog lock is poisoned during cleanup");
                        poisoned.into_inner()
                    }
                };
                catalog
                    .signals
                    .retain(|(_, registered)| *registered != raw_signal);
            }
            if let Ok(sig) = Signal::<String>::try_from_raw(raw_signal) {
                sig.dispose_value();
            }
        });
        Ok(signal)
    }

    fn apply_updates(&self, revision: u64, updates: Vec<(RawSignalId, String)>) -> Result<()> {
        for (raw_signal, value) in updates {
            if lock_localization_catalog(&self.state)?.revision != revision {
                break;
            }
            let signal = Signal::<String>::try_from_raw(raw_signal)?;
            if signal.is_alive() {
                signal.set_if_changed(value);
            }
        }
        Ok(())
    }
}

fn lock_localization_catalog(
    state: &LocalizationState,
) -> Result<std::sync::MutexGuard<'_, LocalizationCatalog>> {
    state.catalog.lock().map_err(|_| {
        Error::InvalidResourceCatalog("localization catalog lock is poisoned".to_string())
    })
}

fn localized_value(
    state: &LocalizationState,
    catalog: &LocalizationCatalog,
    key: &str,
    culture: &str,
) -> Result<String> {
    catalog
        .values
        .get(&(culture.to_string(), key.to_string()))
        .or_else(|| {
            catalog
                .values
                .get(&(state.default_culture.clone(), key.to_string()))
        })
        .cloned()
        .ok_or_else(|| Error::MissingResource {
            key: key.to_string(),
            culture: culture.to_string(),
        })
}

fn localized_updates(
    state: &LocalizationState,
    catalog: &LocalizationCatalog,
    culture: &str,
) -> Result<Vec<(RawSignalId, String)>> {
    catalog
        .signals
        .iter()
        .map(|(key, signal)| {
            localized_value(state, catalog, key, culture).map(|value| (*signal, value))
        })
        .collect()
}

thread_local! {
    static FACTORY: RefCell<Option<sys::ComPtr<sys::IAvnControlFactory>>> = const { RefCell::new(None) };
    static VIEW_BUILD_DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub(crate) fn with_view_subscription_scope<T>(build: impl FnOnce() -> T) -> T {
    VIEW_BUILD_DEPTH.with(|depth| depth.set(depth.get() + 1));
    struct BuildGuard;
    impl Drop for BuildGuard {
        fn drop(&mut self) {
            VIEW_BUILD_DEPTH.with(|depth| depth.set(depth.get() - 1));
        }
    }
    let _guard = BuildGuard;
    build()
}

pub trait AsControl {
    fn as_control(&self) -> Result<sys::ComPtr<sys::IAvnControl>>;
}

impl<T: AsControl + ?Sized> AsControl for &T {
    fn as_control(&self) -> Result<sys::ComPtr<sys::IAvnControl>> {
        (**self).as_control()
    }
}

/// Static upcast along the projected Avalonia single-inheritance hierarchy.
///
/// # Safety
/// `Self` and `Target` must be `#[repr(transparent)]` wrappers over `ComPtr`
/// interfaces where `Self`'s vtable is a prefix-extension of `Target`'s vtable.
pub unsafe trait Upcast<Target>: Sized {
    fn upcast(&self) -> Target;
    fn upcast_ref(&self) -> &Target;
}

unsafe impl<T: Clone> Upcast<T> for T {
    fn upcast(&self) -> T {
        self.clone()
    }

    fn upcast_ref(&self) -> &T {
        self
    }
}

/// Safe runtime downcast from a projected base class (such as `Control` or
/// `AvaloniaObject`) to a more specific derived class via `QueryInterface`.
pub unsafe trait Downcast<Target>: Sized {
    fn is_instance(&self) -> bool;
    fn downcast(&self) -> Option<Target>;
}

unsafe impl<T: Clone> Downcast<T> for T {
    fn is_instance(&self) -> bool {
        true
    }

    fn downcast(&self) -> Option<T> {
        Some(self.clone())
    }
}

/// A lightweight identity-comparable wrapper for non-owning element references
/// (such as `PlacementTarget` or `Label::target`), comparing handles by their
/// host `object_id`.
#[derive(Clone, Debug)]
pub struct NonOwningRef<T> {
    object_id: i64,
    target: T,
}

impl<T> NonOwningRef<T> {
    pub fn from_object_id(object_id: i64, target: T) -> Self {
        Self { object_id, target }
    }

    pub fn object_id(&self) -> i64 {
        self.object_id
    }

    pub fn get(&self) -> &T {
        &self.target
    }
}

impl<T> PartialEq for NonOwningRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.object_id == other.object_id
    }
}

impl<T> Eq for NonOwningRef<T> {}

pub struct EventSubscription {
    unsubscribe: Option<Box<dyn Fn() -> sys::Result<()> + Send>>,
    _thread_affinity: PhantomData<Rc<()>>,
}

impl EventSubscription {
    pub(crate) fn new(unsubscribe: impl Fn() -> sys::Result<()> + Send + 'static) -> Self {
        Self {
            unsubscribe: Some(Box::new(unsubscribe)),
            _thread_affinity: PhantomData,
        }
    }

    pub fn unsubscribe(&mut self) -> Result<()> {
        if let Some(unsubscribe) = self.unsubscribe.as_ref() {
            unsubscribe()?;
            self.unsubscribe = None;
        }
        Ok(())
    }

    fn into_persistent(mut self) -> PersistentSubscription {
        PersistentSubscription {
            unsubscribe: self.unsubscribe.take(),
        }
    }
}

struct PersistentSubscription {
    unsubscribe: Option<Box<dyn Fn() -> sys::Result<()> + Send>>,
}

impl Drop for PersistentSubscription {
    fn drop(&mut self) {
        if let Some(unsubscribe) = self.unsubscribe.as_ref() {
            let _ = unsubscribe();
        }
    }
}

impl fmt::Debug for EventSubscription {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EventSubscription")
            .field("active", &self.unsubscribe.is_some())
            .finish()
    }
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        if let Some(unsubscribe) = self.unsubscribe.as_ref() {
            let _ = unsubscribe();
        }
    }
}

pub struct App {
    application: sys::ComPtr<sys::IAvnApplication>,
    controls: sys::ComPtr<sys::IAvnControlFactory>,
    dispatcher: sys::ComPtr<sys::IAvnDispatcher>,
    startup_arguments: Option<Vec<String>>,
    error_handler: Option<UiErrorHandler>,
    _host: sys::Host,
}

/// A `Clone + Send + Sync` sender that delivers messages from background threads
/// to a UI-thread handler registered with [`AppScope::channel`].
pub struct UiSender<M: Send + 'static> {
    tx: mpsc::Sender<M>,
    scheduled: Arc<AtomicBool>,
    context: AppContext,
    drain: UiCallbackHandle<Box<dyn FnMut()>>,
}

impl<M: Send + 'static> Clone for UiSender<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            scheduled: self.scheduled.clone(),
            context: self.context.clone(),
            drain: self.drain.clone(),
        }
    }
}

impl<M: Send + 'static> fmt::Debug for UiSender<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UiSender").finish_non_exhaustive()
    }
}

impl<M: Send + 'static> UiSender<M> {
    /// Enqueues `message` and schedules a coalesced drain on the UI dispatcher.
    pub fn send(&self, message: M) -> Result<()> {
        self.tx
            .send(message)
            .map_err(|_| Error::Binding("UI channel receiver has been closed".to_string()))?;
        if !self.scheduled.swap(true, Ordering::AcqRel) {
            let drain = self.drain.clone();
            let scheduled = self.scheduled.clone();
            if let Err(err) = self.context.post(move || {
                let _ = drain.with(|cb| cb());
            }) {
                scheduled.store(false, Ordering::Release);
                return Err(err);
            }
        }
        Ok(())
    }
}

/// A running single-view application owned by the platform's UI thread.
///
/// Keep this value alive until the platform is finished with the view. Dropping
/// it detaches the view and releases the application scope on the same thread.
pub struct SingleViewSession {
    _app: App,
    scope: AppScope,
    single_view: sys::ComPtr<sys::IAvnApplication>,
    mounted: bool,
    active: bool,
    _thread_affinity: PhantomData<Rc<()>>,
}

impl SingleViewSession {
    /// Detaches the main view and releases the Rust application scope.
    pub fn close(mut self) -> Result<()> {
        self.cleanup()
    }

    fn cleanup(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }
        let result = if self.mounted {
            self.single_view.set_main_view(None).map_err(Error::from)
        } else {
            Ok(())
        };
        self.scope.clear();
        FACTORY.with(|current| {
            current.replace(None);
        });
        self.active = false;
        result
    }
}

impl Drop for SingleViewSession {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!("failed to detach the Avalonia single view: {error}");
        }
    }
}

#[derive(Clone)]
pub struct AppScope {
    context: AppContext,
    state: Arc<AppScopeState>,
}

struct AppScopeState {
    subscriptions: Mutex<Vec<PersistentSubscription>>,
    objects: Mutex<Vec<Box<dyn Any + Send>>>,
    tasks: Arc<TaskScope>,
    windows: Mutex<Vec<Window>>,
}

impl AppScope {
    fn new(context: AppContext) -> Self {
        Self {
            context,
            state: Arc::new(AppScopeState {
                subscriptions: Mutex::new(Vec::new()),
                objects: Mutex::new(Vec::new()),
                tasks: Arc::new(TaskScope::default()),
                windows: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn mount(&self, window: Window) -> Result<()> {
        if crate::reactive::use_context::<Window>().is_none() {
            crate::reactive::provide_context(window.clone());
        }
        window.raw.show()?;
        self.record_window(window);
        Ok(())
    }

    /// Sets the root control of a single-view application lifetime, such as
    /// the browser or direct-display Linux host where Avalonia cannot create windows. Desktop hosts
    /// reject this with `E_NOTIMPL`; use [`AppScope::mount`] there.
    pub fn set_main_view(&self, view: &impl AsControl) -> Result<()> {
        let view = view.as_control()?;
        Ok(self.context.application.set_main_view(Some(&view))?)
    }

    pub(crate) fn record_window(&self, window: Window) {
        self.state
            .windows
            .lock()
            .expect("application window scope lock poisoned")
            .push(window);
    }

    /// Every window mounted through [`AppScope::mount`], in mount order.
    ///
    /// Desktop file pickers are tied to a window, so a Rust view model that owns
    /// state but not presentation still needs a handle to the window the host
    /// created for it.
    pub fn windows(&self) -> Vec<Window> {
        self.state
            .windows
            .lock()
            .expect("application window scope lock poisoned")
            .clone()
    }

    /// The first window mounted through [`AppScope::mount`].
    pub fn main_window(&self) -> Option<Window> {
        self.state
            .windows
            .lock()
            .expect("application window scope lock poisoned")
            .first()
            .cloned()
    }

    pub fn delay(&self, duration: Duration) -> Result<AsyncOperation<()>> {
        let milliseconds =
            i32::try_from(duration.as_millis()).map_err(|_| Error::InvalidAsyncValue)?;
        let application = self.context.application.clone();
        AsyncOperation::start(
            application.clone(),
            move |completion| application.start_delay(milliseconds, completion),
            decode_none,
        )
    }

    pub fn clipboard_set_text(
        &self,
        window: &Window,
        text: impl AsRef<str>,
    ) -> Result<AsyncOperation<()>> {
        let text: Vec<u16> = text.as_ref().encode_utf16().chain(Some(0)).collect();
        let window = window.raw.clone();
        let application = self.context.application.clone();
        AsyncOperation::start(
            application.clone(),
            move |completion| application.start_clipboard_set_text(&window, &text, completion),
            decode_none,
        )
    }

    pub fn clipboard_get_text(&self, window: &Window) -> Result<AsyncOperation<Option<String>>> {
        let window = window.raw.clone();
        let application = self.context.application.clone();
        AsyncOperation::start(
            application.clone(),
            move |completion| application.start_clipboard_get_text(&window, completion),
            decode_string,
        )
    }

    /// Shows `dialog` modally over `window`, completing with the dialog result the host
    /// converted (`None` when the dialog returned no result). Dropping the returned
    /// operation closes the dialog.
    pub fn show_dialog(
        &self,
        window: &Window,
        dialog: &Window,
    ) -> Result<AsyncOperation<Option<String>>> {
        let owner = window.raw.clone();
        let dialog = dialog.raw.clone();
        let application = self.context.application.clone();
        let dialogs = application.clone();
        AsyncOperation::start(
            application,
            move |completion| {
                dialogs.start_show_dialog(Some(&owner), Some(&dialog), Some(completion))
            },
            decode_string,
        )
    }

    pub(crate) fn clone_context(&self) -> AppContext {
        self.context.clone()
    }

    /// Creates a `Send + Sync + Clone` [`UiSender<M>`] that delivers background
    /// messages in batches to a UI-thread `handler` (which may freely capture
    /// `!Send` [`Signal`]s, `Rc`, or `RefCell` state).
    pub fn channel<M, R>(&self, mut handler: impl FnMut(M) -> R + 'static) -> UiSender<M>
    where
        M: Send + 'static,
        R: IntoEventResult,
    {
        let (tx, rx) = mpsc::channel::<M>();
        let scheduled = Arc::new(AtomicBool::new(false));
        let scheduled_for_drain = scheduled.clone();
        let drain_fn: Box<dyn FnMut()> = Box::new(move || loop {
            scheduled_for_drain.store(false, Ordering::Release);
            let mut processed = false;
            while let Ok(msg) = rx.try_recv() {
                processed = true;
                if let Err(err) = handler(msg).into_event_result() {
                    report_ui_error("UI channel handler failed", &err);
                }
            }
            if !processed || !scheduled_for_drain.load(Ordering::Acquire) {
                break;
            }
        });
        let drain = UiCallbackHandle::new(drain_fn);
        self.retain_object(drain.clone());
        UiSender {
            tx,
            scheduled,
            context: self.context.clone(),
            drain,
        }
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> Result<()> {
        self.state
            .tasks
            .spawn(self.context.dispatcher.clone(), future)
    }

    /// Spawns a `!Send` UI-thread future polled on the Avalonia dispatcher.
    ///
    /// The future may freely capture [`Signal`]s, `Rc`, `RefCell`, and controls,
    /// and may return either `()` or `rustolonia::Result<()>` (via `?`).
    pub fn spawn_local<R: IntoEventResult + 'static>(
        &self,
        future: impl Future<Output = R> + 'static,
    ) -> Result<()> {
        self.state
            .tasks
            .spawn_local(self.context.dispatcher.clone(), async move {
                if let Err(err) = future.await.into_event_result() {
                    report_ui_error("UI task failed", &err);
                }
            })
    }

    /// Runs `work` on a background OS thread and returns a future resolving on
    /// the polling executor (such as [`AppScope::spawn_local`]).
    pub fn run_blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T> + Send + 'static,
    ) -> crate::async_runtime::BlockingOperation<T> {
        crate::async_runtime::BlockingOperation::spawn(work)
    }

    /// Runs `work` on a background OS thread and delivers its result to
    /// `on_complete` on the UI thread.
    pub fn spawn_blocking<T, R>(
        &self,
        work: impl FnOnce() -> Result<T> + Send + 'static,
        mut on_complete: impl FnMut(Result<T>) -> R + 'static,
    ) -> Result<()>
    where
        T: Send + 'static,
        R: IntoEventResult + 'static,
    {
        let op = self.run_blocking(work);
        self.spawn_local(async move {
            let result = op.await;
            on_complete(result)
        })
    }

    /// Resolves the enclosing [`Window`] from reactive scope context, falling
    /// back to [`AppScope::main_window`].
    pub fn resolve_window(&self) -> Result<Window> {
        crate::reactive::use_context::<Window>()
            .or_else(|| self.main_window())
            .ok_or_else(|| {
                Error::Binding("no active Window in scope or mounted on AppScope".to_string())
            })
    }

    /// Single-`Result` async sleep on the host timer.
    pub async fn delay_async(&self, duration: Duration) -> Result<()> {
        self.delay(duration)?.await
    }

    /// Reads clipboard text using the enclosing or main [`Window`].
    pub async fn clipboard_text(&self) -> Result<Option<String>> {
        let window = self.resolve_window()?;
        self.clipboard_get_text(&window)?.await
    }

    /// Reads clipboard text using an explicit `window`.
    pub async fn clipboard_text_on(&self, window: &Window) -> Result<Option<String>> {
        self.clipboard_get_text(window)?.await
    }

    /// Writes clipboard text using the enclosing or main [`Window`].
    pub async fn set_clipboard_text(&self, text: impl AsRef<str>) -> Result<()> {
        let window = self.resolve_window()?;
        self.clipboard_set_text(&window, text)?.await
    }

    /// Writes clipboard text using an explicit `window`.
    pub async fn set_clipboard_text_on(
        &self,
        window: &Window,
        text: impl AsRef<str>,
    ) -> Result<()> {
        self.clipboard_set_text(window, text)?.await
    }

    /// Opens the file picker attached to the enclosing or main [`Window`].
    pub async fn open_files(
        &self,
        options: &crate::OpenFilePickerOptions,
    ) -> Result<crate::PickerOutcome> {
        let window = self.resolve_window()?;
        self.open_file_picker(&window, options)?.await
    }

    /// Opens the file picker attached to an explicit `window`.
    pub async fn open_files_on(
        &self,
        window: &Window,
        options: &crate::OpenFilePickerOptions,
    ) -> Result<crate::PickerOutcome> {
        self.open_file_picker(window, options)?.await
    }

    /// Opens the folder picker attached to the enclosing or main [`Window`].
    pub async fn pick_folder(
        &self,
        options: &crate::FolderPickerOptions,
    ) -> Result<crate::PickerOutcome> {
        let window = self.resolve_window()?;
        self.open_folder_picker(&window, options)?.await
    }

    /// Opens the folder picker attached to an explicit `window`.
    pub async fn pick_folder_on(
        &self,
        window: &Window,
        options: &crate::FolderPickerOptions,
    ) -> Result<crate::PickerOutcome> {
        self.open_folder_picker(window, options)?.await
    }

    /// Opens the save-file picker attached to the enclosing or main [`Window`].
    pub async fn save_file(
        &self,
        options: &crate::SaveFilePickerOptions,
    ) -> Result<crate::PickerOutcome> {
        let window = self.resolve_window()?;
        self.save_file_picker(&window, options)?.await
    }

    /// Opens the save-file picker attached to an explicit `window`.
    pub async fn save_file_on(
        &self,
        window: &Window,
        options: &crate::SaveFilePickerOptions,
    ) -> Result<crate::PickerOutcome> {
        self.save_file_picker(window, options)?.await
    }

    /// Shows `dialog` modally over the enclosing or main [`Window`].
    pub async fn show_modal(&self, dialog: &Window) -> Result<Option<String>> {
        let owner = self.resolve_window()?;
        self.show_dialog(&owner, dialog)?.await
    }

    /// Shows `dialog` modally over an explicit `owner` window.
    pub async fn show_modal_on(&self, owner: &Window, dialog: &Window) -> Result<Option<String>> {
        self.show_dialog(owner, dialog)?.await
    }

    /// Retains an event subscription until the current reactive view is disposed,
    /// or until the application scope ends when called outside a view build.
    pub fn retain_subscription(&self, subscription: EventSubscription) {
        if VIEW_BUILD_DEPTH.with(|depth| depth.get() > 0) {
            on_scope_cleanup(move || {
                let mut subscription = subscription;
                if let Err(error) = subscription.unsubscribe() {
                    eprintln!("failed to unsubscribe a disposed view event: {error}");
                }
            });
            return;
        }
        self.state
            .subscriptions
            .lock()
            .expect("application subscription scope lock poisoned")
            .push(subscription.into_persistent());
    }

    pub(crate) fn retain_object(&self, value: impl Any + Send) {
        self.state
            .objects
            .lock()
            .expect("application object scope lock poisoned")
            .push(Box::new(value));
    }

    /// Keeps localized resource signals active for this application lifetime.
    pub fn retain_localization_resources(&self, resources: LocalizationResources) {
        self.retain_object(resources);
    }

    /// Keeps Rust-owned resource signals active for this application lifetime.
    pub fn retain_resource_scope(&self, resources: crate::ResourceScope) {
        self.retain_object(resources);
    }

    pub(crate) fn application(&self) -> &sys::ComPtr<sys::IAvnApplication> {
        &self.context.application
    }

    // Single-view hosts (browser, Android) keep the session for the process lifetime.
    #[cfg_attr(any(target_os = "emscripten", target_os = "android"), allow(dead_code))]
    fn clear(&self) {
        crate::reactive::clear_reactive_runtime();
        self.state.tasks.clear();
        self.state
            .subscriptions
            .lock()
            .expect("application subscription scope lock poisoned")
            .clear();
        self.state
            .objects
            .lock()
            .expect("application object scope lock poisoned")
            .clear();
        self.state
            .windows
            .lock()
            .expect("application window scope lock poisoned")
            .clear();
        crate::value_converter::clear_value_converter_provider(&self.context.application);
    }
}

impl Deref for AppScope {
    type Target = AppContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl fmt::Debug for AppScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("AppScope").finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct AppContext {
    application: sys::ComPtr<sys::IAvnApplication>,
    dispatcher: sys::ComPtr<sys::IAvnDispatcher>,
}

impl AppContext {
    pub fn check_access(&self) -> Result<bool> {
        Ok(self.dispatcher.check_access()?)
    }

    pub(crate) fn application(&self) -> &sys::ComPtr<sys::IAvnApplication> {
        &self.application
    }

    pub fn post(&self, callback: impl FnOnce() + Send + 'static) -> Result<()> {
        let action = sys::action(move || {
            callback();
            Ok(())
        });
        Ok(self.dispatcher.post(&action)?)
    }

    pub fn shutdown(&self) -> Result<()> {
        Ok(self.application.shutdown()?)
    }

    pub fn requested_theme_variant(&self) -> Result<ThemeVariant> {
        theme_variant(self.application.requested_theme_variant()?)
    }

    pub fn set_requested_theme_variant(&self, value: ThemeVariant) -> Result<()> {
        Ok(self.application.set_requested_theme_variant(value as i32)?)
    }

    pub fn actual_theme_variant(&self) -> Result<ThemeVariant> {
        theme_variant(self.application.actual_theme_variant()?)
    }

    pub fn find_resource(
        &self,
        key: impl AsRef<str>,
        theme: ThemeVariant,
    ) -> Result<Option<ResourceValue>> {
        let key: Vec<u16> = key.as_ref().encode_utf16().chain(Some(0)).collect();
        self.application
            .try_get_resource(&key, theme as i32)?
            .map(resource_value)
            .transpose()
    }
}

/// Environment variable that explicitly overrides native host discovery.
/// Set by `rust/build.ps1`/`rust/build.sh` for the workspace test suite, and
/// always takes priority over the adjacent-executable lookup performed by
/// [`discover_host_path`].
pub const HOST_NATIVE_LIB_ENV_VAR: &str = "RUSTOLONIA_HOST_LIB";

#[cfg(target_os = "windows")]
const HOST_FILE_NAME: &str = "rustolonia_host.dll";
#[cfg(target_os = "linux")]
const HOST_FILE_NAME: &str = "librustolonia_host.so";
#[cfg(target_os = "macos")]
const HOST_FILE_NAME: &str = "librustolonia_host.dylib";
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
const HOST_FILE_NAME: &str = "rustolonia_host";

fn adjacent_host_path(directory: &Path) -> Option<PathBuf> {
    let candidate = directory.join(HOST_FILE_NAME);
    candidate.exists().then_some(candidate)
}

/// Directories searched for the host, in order: the executable's directory,
/// on macOS the app bundle's `Contents/Frameworks`, then the directory
/// `rustolonia-sys`'s build script staged the host in (`dev-host-path`).
fn host_search_dirs(exe_dir: Option<&Path>, build_host_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(exe_dir) = exe_dir {
        dirs.push(exe_dir.to_path_buf());
        if cfg!(target_os = "macos") {
            dirs.push(exe_dir.join("..").join("Frameworks"));
        }
    }
    dirs.extend(build_host_dir.map(Path::to_path_buf));
    dirs
}

fn find_host(dirs: &[PathBuf]) -> std::result::Result<PathBuf, String> {
    if let Some(found) = dirs.iter().find_map(|dir| adjacent_host_path(dir)) {
        return Ok(found);
    }
    let searched = if dirs.is_empty() {
        "no directories".to_owned()
    } else {
        dirs.iter()
            .map(|dir| format!("'{}'", dir.display()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Err(format!(
        "{HOST_NATIVE_LIB_ENV_VAR} is not set and no {HOST_FILE_NAME} was found (searched \
         {searched}); set {HOST_NATIVE_LIB_ENV_VAR} to override, or ship {HOST_FILE_NAME} \
         beside this executable (see rust/PRODUCTIZATION.md#host-discovery)"
    ))
}

/// Locates the native Avalonia host library the same way [`App::load_from_env`]
/// does, without loading it.
///
/// `RUSTOLONIA_HOST_LIB` ([`HOST_NATIVE_LIB_ENV_VAR`]) is an explicit override
/// and always wins when set to a non-empty value (an empty value counts as
/// unset) -- even to a path that does not exist yet, so
/// [`sys::Host::load`] can surface a precise loader error instead of this
/// function silently falling back. Otherwise the platform host library
/// (`rustolonia_host.dll` / `.so` / `.dylib`) is searched for:
///
/// 1. next to the running executable (the layout `rust/package.ps1` and
///    `rust/package.sh` produce);
/// 2. on macOS, in the app bundle's `Contents/Frameworks`;
/// 3. in the directory where `rustolonia-sys` staged the host at build time,
///    when the `dev-host-path` feature is enabled, so `cargo run` works
///    without copying files.
///
/// See `rust/PRODUCTIZATION.md#host-discovery`.
pub fn discover_host_path() -> Result<PathBuf> {
    if cfg!(target_os = "ios") {
        return Err(Error::Load(
            "iOS applications must use the in-process host supplied by the iOS app".into(),
        ));
    }
    if let Some(value) = std::env::var_os(HOST_NATIVE_LIB_ENV_VAR).filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(value));
    }
    let exe = std::env::current_exe().ok();
    let dirs = host_search_dirs(
        exe.as_deref().and_then(Path::parent),
        sys::BUILD_HOST_DIR.map(Path::new),
    );
    find_host(&dirs).map_err(Error::Load)
}

impl App {
    /// Loads the native Avalonia host discovered by [`discover_host_path`]:
    /// `RUSTOLONIA_HOST_LIB` if set, otherwise the platform host library next
    /// to this executable.
    ///
    /// In browser builds (`wasm32-unknown-emscripten`) and with the
    /// `static-host` feature the host is statically linked into the app, so
    /// no discovery takes place.
    pub fn load_from_env() -> Result<Self> {
        #[cfg(target_os = "emscripten")]
        return Self::load(Path::new(""));
        #[cfg(not(target_os = "emscripten"))]
        if sys::STATIC_HOST {
            Self::load_static()
        } else {
            Self::load(discover_host_path()?)
        }
    }

    /// Binds the host linked into this binary by the `static-host` feature.
    #[cfg(not(target_os = "emscripten"))]
    pub fn load_static() -> Result<Self> {
        let host = sys::Host::load_static().map_err(|error| Error::Load(error.to_string()))?;
        Self::from_host(host)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        if cfg!(target_os = "ios") {
            return Err(Error::Load(
                "iOS applications must use the in-process host supplied by the iOS app".into(),
            ));
        }
        let host = sys::Host::load(path).map_err(|error| Error::Load(error.to_string()))?;
        Self::from_host(host)
    }

    /// Connects to the managed host embedded in the current process.
    ///
    /// # Safety
    ///
    /// The functions in `exports` must have the declared ABI and remain
    /// callable until the process exits. The managed host must not be unloaded.
    pub unsafe fn from_in_process(exports: &sys::HostExports) -> Result<Self> {
        let host =
            sys::Host::from_in_process(exports).map_err(|error| Error::Load(error.to_string()))?;
        Self::from_host(host)
    }

    fn from_host(host: sys::Host) -> Result<Self> {
        let activation = host.activation_factory()?;
        let application = activation.create_application()?;
        let controls = activation.create_control_factory()?;
        let dispatcher = activation.create_dispatcher()?;
        Ok(Self {
            application,
            controls,
            dispatcher,
            startup_arguments: None,
            error_handler: None,
            _host: host,
        })
    }

    /// Installs a custom handler for errors returned by UI event callbacks,
    /// reactive effects, two-way bindings, and commands.
    ///
    /// When unset, errors are printed to `stderr`.
    pub fn with_error_handler(
        mut self,
        handler: impl Fn(&str, &Error) + Send + Sync + 'static,
    ) -> Self {
        self.error_handler = Some(Arc::new(handler));
        self
    }

    /// Creates the root control for an already-initialized single-view
    /// platform such as iOS. The platform owns the event loop; unlike
    /// [`App::run`], this returns as soon as the root is mounted.
    ///
    /// Call this on the platform UI thread and retain the returned session.
    pub fn start_single_view<T: AsControl>(
        self,
        callback: impl FnOnce(&AppScope) -> Result<T>,
    ) -> Result<SingleViewSession> {
        if !self.dispatcher.check_access()? {
            return Err(Error::NoUiContext);
        }
        let single_view = self.application.clone();
        let context = AppContext {
            application: self.application.clone(),
            dispatcher: self.dispatcher.clone(),
        };
        let scope = AppScope::new(context);
        let installed = FACTORY.with(|current| {
            let mut current = current.borrow_mut();
            if current.is_some() {
                false
            } else {
                *current = Some(self.controls.clone());
                true
            }
        });
        if !installed {
            return Err(Error::UiContextAlreadyActive);
        }
        let mut session = SingleViewSession {
            _app: self,
            scope,
            single_view,
            mounted: false,
            active: true,
            _thread_affinity: PhantomData,
        };
        let view = callback(&session.scope)?;
        session
            .single_view
            .set_main_view(Some(&view.as_control()?))?;
        session.mounted = true;
        Ok(session)
    }

    /// Overrides the startup/open-with arguments handed to the managed desktop
    /// lifetime.
    ///
    /// A Rust executable owns `argv`, so the host cannot see it. By default
    /// [`App::run`] forwards this process's own arguments, which is what makes
    /// an "open with" launch work with no extra wiring; call this to supply a
    /// different list (a single-instance relay, a test harness, an embedder).
    pub fn with_startup_arguments(
        mut self,
        arguments: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.startup_arguments = Some(arguments.into_iter().map(Into::into).collect());
        self
    }

    /// Sets Avalonia's application name before initialization.
    ///
    /// On macOS this supplies the running-process name and default application
    /// menu title, including the Hide item. It does not set window titles or
    /// replace bundle metadata. Call before [`App::run`], not in its callback.
    ///
    /// Empty/whitespace-only names and embedded NULs return `E_INVALIDARG`.
    /// iOS and Android, where Avalonia is initialized before Rust runs, return
    /// `E_NOTIMPL`.
    pub fn with_application_name(self, name: impl AsRef<str>) -> Result<Self> {
        let name = encode_application_name(name.as_ref())?;
        self.application.set_application_name(Some(&name))?;
        Ok(self)
    }

    /// Starts with no startup arguments at all.
    pub fn without_startup_arguments(mut self) -> Self {
        self.startup_arguments = Some(Vec::new());
        self
    }

    /// Starts the application and calls `callback` once the UI is ready.
    ///
    /// On desktop and direct-display Linux this blocks until the application shuts down. In the
    /// browser and on Android the host owns the UI loop instead: `run`
    /// returns once the application has started (on Android after `callback`
    /// has run; in the browser the callback runs later on the page's event
    /// loop), and the application stays alive for the rest of the process
    /// or page. On iOS, use [`App::start_single_view`] instead.
    pub fn run(
        self,
        callback: impl FnOnce(&AppScope) -> Result<()> + Send + 'static,
    ) -> Result<()> {
        if cfg!(target_os = "ios") {
            return Err(sys::Error(sys::E_NOTIMPL).into());
        }

        let controls = self.controls.clone();
        let context = AppContext {
            application: self.application.clone(),
            dispatcher: self.dispatcher.clone(),
        };
        let scope = AppScope::new(context);
        let cleanup_scope = scope.clone();
        let handler = sys::app_handler(move || callback(&scope).map_err(to_abi_error));
        let arguments = self
            .startup_arguments
            .clone()
            .unwrap_or_else(process_startup_arguments);
        publish_startup_arguments(&self.application, &arguments)?;
        // Single-view hosts own the UI loop: in the browser the JavaScript event
        // loop, on Android the activity's main thread. Their Run starts the
        // application and returns immediately. The control factory, scope and
        // host stay alive for the page or process lifetime: the startup callback
        // releases its scope clone once it returns, so this one keeps the
        // retained subscriptions and objects registered.
        let error_handler = self.error_handler.clone();
        #[cfg(any(target_os = "emscripten", target_os = "android"))]
        {
            if let Some(handler) = error_handler {
                UI_ERROR_HANDLER.with(|slot| *slot.borrow_mut() = Some(handler));
            }
            FACTORY.with(|current| current.replace(Some(controls)));
            let result = self.application.run(&handler);
            std::mem::forget(cleanup_scope);
            std::mem::forget(handler);
            std::mem::forget(self);
            Ok(result?)
        }
        #[cfg(not(any(target_os = "emscripten", target_os = "android")))]
        FACTORY.with(|current| {
            let previous_err = UI_ERROR_HANDLER.with(|slot| slot.borrow_mut().clone());
            if let Some(handler) = error_handler {
                UI_ERROR_HANDLER.with(|slot| *slot.borrow_mut() = Some(handler));
            }
            let previous = current.replace(Some(controls));
            let result = self.application.run(&handler);
            cleanup_scope.clear();
            current.replace(previous);
            UI_ERROR_HANDLER.with(|slot| *slot.borrow_mut() = previous_err);
            Ok(result?)
        })
    }
}

/// This process's arguments, minus the executable name.
///
/// `std::env::args` panics on an argument that is not valid Unicode, and a
/// document path is exactly where that happens: a Unix filename is an arbitrary
/// byte sequence, so a shell "open with" can hand this process one. Losing the
/// invalid part of a path is a far better outcome than panicking before the
/// application has even started, so the bytes are converted lossily instead.
fn process_startup_arguments() -> Vec<String> {
    // An Android process is forked from the zygote; its argv is not the
    // application's. Intents are the activation mechanism there.
    if cfg!(target_os = "android") {
        return Vec::new();
    }
    std::env::args_os()
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

fn encode_application_name(name: &str) -> Result<Vec<u16>> {
    if name.trim().is_empty() || name.contains('\0') {
        return Err(sys::Error(sys::E_INVALIDARG).into());
    }
    Ok(name.encode_utf16().chain(Some(0)).collect())
}

/// Publishes the startup arguments before the managed lifetime starts.
fn publish_startup_arguments(
    application: &sys::ComPtr<sys::IAvnApplication>,
    arguments: &[String],
) -> Result<()> {
    application.clear_startup_arguments()?;
    for argument in arguments {
        let encoded: Vec<u16> = argument.encode_utf16().chain(Some(0)).collect();
        application.add_startup_argument(Some(&encoded))?;
    }
    Ok(())
}

pub(crate) fn with_factory<T>(
    callback: impl FnOnce(&sys::ComPtr<sys::IAvnControlFactory>) -> sys::Result<T>,
) -> Result<T> {
    FACTORY.with(|factory| {
        let factory = factory.borrow();
        let factory = factory.as_ref().ok_or(Error::NoUiContext)?;
        Ok(callback(factory)?)
    })
}

fn to_abi_error(error: Error) -> sys::Error {
    match error {
        Error::Abi(error) => error,
        Error::Load(_)
        | Error::NoUiContext
        | Error::UiContextAlreadyActive
        | Error::InvalidEnumValue(_)
        | Error::InvalidAsyncValue
        | Error::InvalidResourceCatalog(_)
        | Error::MissingResource { .. }
        | Error::MissingScopedResource { .. }
        | Error::ResourceTypeMismatch { .. }
        | Error::InvalidResourceScope(_)
        | Error::Binding(_)
        | Error::ReadOnlyBindingPath
        | Error::DuplicateItemKey
        | Error::CollectionUpdateRollback { .. }
        | Error::ControlSlotNotMounted
        | Error::ControlSlotAlreadyMounted
        | Error::InvalidViewRegistry(_)
        | Error::InvalidViewModelMember { .. }
        | Error::InvalidValue(_)
        | Error::Async { .. } => sys::Error(sys::E_FAIL),
    }
}

fn theme_variant(value: i32) -> Result<ThemeVariant> {
    match value {
        0 => Ok(ThemeVariant::Default),
        1 => Ok(ThemeVariant::Light),
        2 => Ok(ThemeVariant::Dark),
        value => Err(Error::InvalidEnumValue(value)),
    }
}

fn resource_value(value: sys::ComPtr<sys::IAvnResourceValue>) -> Result<ResourceValue> {
    Ok(match value.kind()? {
        0 => ResourceValue::Null,
        1 => ResourceValue::Boolean(value.boolean()?),
        2 => ResourceValue::Integer(value.integer()?),
        3 => ResourceValue::Double(value.double()?),
        4 => ResourceValue::String(unsafe {
            sys::take_utf16(value.string()?).ok_or(Error::Abi(sys::Error(sys::E_POINTER)))?
        }),
        5 => ResourceValue::Color(value.color()?),
        kind => return Err(Error::InvalidEnumValue(kind)),
    })
}

#[cfg(test)]
mod localization_tests {
    use super::*;

    #[test]
    fn localized_signals_follow_culture_and_fall_back_to_default() {
        let resources = LocalizationResources::new("en-US").unwrap();
        resources.insert("en-US", "greeting", "Hello").unwrap();
        resources.insert("fr-FR", "greeting", "Bonjour").unwrap();
        let greeting = resources.string_signal("greeting").unwrap();
        assert_eq!(greeting.get(), "Hello");

        resources.set_culture("fr-FR").unwrap();
        assert_eq!(greeting.get(), "Bonjour");

        resources.set_culture("de-DE").unwrap();
        assert_eq!(greeting.get(), "Hello");
    }

    #[test]
    fn localized_signals_update_when_catalog_entries_change() {
        let resources = LocalizationResources::new("en-US").unwrap();
        resources.insert("en-US", "greeting", "Hello").unwrap();
        let greeting = resources.string_signal("greeting").unwrap();

        resources.insert("en-US", "greeting", "Hi").unwrap();
        assert_eq!(greeting.get(), "Hi");
    }

    #[test]
    fn missing_localized_strings_are_reported() {
        let resources = LocalizationResources::new("en-US").unwrap();
        let error = resources
            .string_signal("missing")
            .expect_err("missing keys must not silently become empty strings");
        assert!(matches!(
            error,
            Error::MissingResource { key, culture } if key == "missing" && culture == "en-US"
        ));
    }

    #[test]
    fn culture_changes_with_missing_translations_leave_the_previous_values_intact() {
        let resources = LocalizationResources::new("en-US").unwrap();
        resources.set_culture("fr-FR").unwrap();
        resources.insert("fr-FR", "greeting", "Bonjour").unwrap();
        let greeting = resources.string_signal("greeting").unwrap();

        assert!(matches!(
            resources.set_culture("de-DE"),
            Err(Error::MissingResource { key, culture })
                if key == "greeting" && culture == "de-DE"
        ));
        assert_eq!(resources.culture(), "fr-FR");
        assert_eq!(greeting.get(), "Bonjour");

        resources.insert("en-US", "greeting", "Hello").unwrap();
        resources.set_culture("de-DE").unwrap();
        assert_eq!(greeting.get(), "Hello");
    }

    #[test]
    fn dropping_the_last_catalog_owner_releases_its_state() {
        let resources = LocalizationResources::new("en-US").unwrap();
        resources.insert("en-US", "greeting", "Hello").unwrap();
        let greeting = resources.string_signal("greeting").unwrap();
        let state = Arc::downgrade(&resources.state);

        drop(resources);
        assert!(state.upgrade().is_none());
        assert_eq!(greeting.get(), "Hello");
    }

    #[test]
    fn disposing_a_view_removes_its_localization_subscription() {
        let resources = LocalizationResources::new("en-US").unwrap();
        resources.insert("en-US", "greeting", "Hello").unwrap();
        let scope = crate::ReactiveScope::new();
        let _greeting = scope.run(|| resources.string_signal("greeting").unwrap());
        assert_eq!(resources.state.catalog.lock().unwrap().signals.len(), 1);

        scope.dispose();
        assert!(resources.state.catalog.lock().unwrap().signals.is_empty());
        resources.insert("en-US", "greeting", "Hi").unwrap();
    }

    #[test]
    fn disposing_a_view_during_catalog_updates_skips_its_pending_signal() {
        let resources = LocalizationResources::new("en-US").unwrap();
        resources.insert("en-US", "greeting", "Hello").unwrap();
        let trigger = resources.string_signal("greeting").unwrap();
        let view = crate::ReactiveScope::new();
        let _view_signal = view.run(|| resources.string_signal("greeting").unwrap());
        let survivor = resources.string_signal("greeting").unwrap();
        let effect = crate::Effect::new(move || {
            if trigger.get() == "Hi" {
                view.dispose();
            }
            Ok(())
        })
        .unwrap();

        resources.insert("en-US", "greeting", "Hi").unwrap();
        assert_eq!(survivor.get(), "Hi");
        assert_eq!(resources.state.catalog.lock().unwrap().signals.len(), 2);
        effect.dispose();
    }

    #[test]
    fn reentrant_catalog_updates_refresh_all_registered_strings() {
        let resources = LocalizationResources::new("en-US").unwrap();
        resources.insert("en-US", "first", "First").unwrap();
        resources.insert("en-US", "second", "Second").unwrap();
        resources.insert("fr-FR", "first", "Premier").unwrap();
        resources.insert("fr-FR", "second", "Deuxieme").unwrap();
        let first = resources.string_signal("first").unwrap();
        let second = resources.string_signal("second").unwrap();
        let callback_resources = resources.clone();
        let effect = crate::Effect::new(move || {
            if callback_resources.culture() == "fr-FR" {
                callback_resources.insert("fr-FR", "first", "Un")?;
            }
            Ok(())
        })
        .unwrap();

        resources.set_culture("fr-FR").unwrap();
        assert_eq!(first.get(), "Un");
        assert_eq!(second.get(), "Deuxieme");
        effect.dispose();
    }
}

#[cfg(test)]
mod application_name_tests {
    use super::*;

    #[test]
    fn encodes_unicode_without_trimming() {
        let name = " Jiffshot \u{1f4f7} ";
        let encoded = encode_application_name(name).unwrap();
        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(
            String::from_utf16(&encoded[..encoded.len() - 1]).unwrap(),
            name
        );
    }

    #[test]
    fn rejects_empty_whitespace_and_embedded_nul_names() {
        for name in ["", " \t\n", "\u{2003}", "Jiff\0shot", "Jiffshot\0"] {
            assert!(matches!(
                encode_application_name(name),
                Err(Error::Abi(sys::Error(sys::E_INVALIDARG)))
            ));
        }
    }
}

#[cfg(test)]
mod host_discovery_tests {
    use super::*;
    use std::sync::Mutex;

    // `discover_host_path` reads/writes process-wide environment state, and
    // Rust tests in one binary run on multiple threads by default, so every
    // test that touches `HOST_NATIVE_LIB_ENV_VAR` must hold this lock for its
    // whole duration.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvVarGuard {
        previous: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set(value: &str) -> Self {
            let previous = std::env::var_os(HOST_NATIVE_LIB_ENV_VAR);
            std::env::set_var(HOST_NATIVE_LIB_ENV_VAR, value);
            Self { previous }
        }

        fn unset() -> Self {
            let previous = std::env::var_os(HOST_NATIVE_LIB_ENV_VAR);
            std::env::remove_var(HOST_NATIVE_LIB_ENV_VAR);
            Self { previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var(HOST_NATIVE_LIB_ENV_VAR, value),
                None => std::env::remove_var(HOST_NATIVE_LIB_ENV_VAR),
            }
        }
    }

    #[test]
    fn env_override_wins_even_when_the_path_does_not_exist() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let _guard = EnvVarGuard::set("/definitely/not/a/real/rustolonia/host");
        let resolved = discover_host_path().expect("explicit override must always resolve");
        assert_eq!(
            resolved,
            PathBuf::from("/definitely/not/a/real/rustolonia/host")
        );
    }

    #[test]
    fn adjacent_lookup_finds_the_host_file_beside_a_directory() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let _guard = EnvVarGuard::unset();
        let directory = std::env::temp_dir().join(format!(
            "avalonia-host-discovery-present-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory).expect("create scratch directory");
        let host_path = directory.join(HOST_FILE_NAME);
        std::fs::write(&host_path, b"stub").expect("write stub host file");

        let found = adjacent_host_path(&directory);

        std::fs::remove_dir_all(&directory).ok();
        assert_eq!(found, Some(host_path));
    }

    #[test]
    fn adjacent_lookup_returns_none_when_the_host_file_is_missing() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let directory = std::env::temp_dir().join(format!(
            "avalonia-host-discovery-missing-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory).expect("create scratch directory");

        let found = adjacent_host_path(&directory);

        std::fs::remove_dir_all(&directory).ok();
        assert_eq!(found, None);
    }

    #[test]
    fn missing_env_and_missing_adjacent_host_reports_both_mechanisms() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let directory = std::env::temp_dir().join(format!(
            "avalonia-host-discovery-none-{}-{}",
            std::process::id(),
            line!()
        ));
        let dirs = host_search_dirs(Some(&directory), Some(&directory.join("build")));
        let message = find_host(&dirs).expect_err("nothing should resolve");
        assert!(message.contains(HOST_NATIVE_LIB_ENV_VAR));
        assert!(message.contains(HOST_FILE_NAME));
        for dir in &dirs {
            assert!(message.contains(&dir.display().to_string()), "{message}");
        }
    }

    #[test]
    fn search_order_is_exe_dir_then_bundle_then_build_dir() {
        let exe_dir = Path::new("exe");
        let build_dir = Path::new("build");
        let mut expected = vec![PathBuf::from("exe")];
        if cfg!(target_os = "macos") {
            expected.push(Path::new("exe").join("..").join("Frameworks"));
        }
        expected.push(PathBuf::from("build"));
        assert_eq!(host_search_dirs(Some(exe_dir), Some(build_dir)), expected);
        assert_eq!(
            host_search_dirs(None, Some(build_dir)),
            vec![PathBuf::from("build")]
        );
        assert!(host_search_dirs(None, None).is_empty());
    }

    #[test]
    fn the_first_directory_containing_the_host_wins() {
        let root = std::env::temp_dir().join(format!(
            "avalonia-host-discovery-order-{}-{}",
            std::process::id(),
            line!()
        ));
        let (exe_dir, build_dir) = (root.join("exe"), root.join("build"));
        std::fs::create_dir_all(&exe_dir).expect("create exe directory");
        std::fs::create_dir_all(&build_dir).expect("create build directory");
        std::fs::write(build_dir.join(HOST_FILE_NAME), b"stub").expect("write build host");
        let dirs = host_search_dirs(Some(&exe_dir), Some(&build_dir));

        let from_build = find_host(&dirs);
        std::fs::write(exe_dir.join(HOST_FILE_NAME), b"stub").expect("write exe host");
        let from_exe = find_host(&dirs);

        std::fs::remove_dir_all(&root).ok();
        assert_eq!(from_build, Ok(build_dir.join(HOST_FILE_NAME)));
        assert_eq!(from_exe, Ok(exe_dir.join(HOST_FILE_NAME)));
    }
}

use crate::async_runtime::{decode_none, decode_string, TaskScope};
use crate::{AsyncOperation, Error, Result, Window};
use rustolonia_sys as sys;
use std::any::Any;
use std::cell::RefCell;
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

thread_local! {
    static FACTORY: RefCell<Option<sys::ComPtr<sys::IAvnControlFactory>>> = const { RefCell::new(None) };
}

pub trait AsControl {
    fn as_control(&self) -> Result<sys::ComPtr<sys::IAvnControl>>;
}

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
    _host: sys::Host,
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
        window.raw.show()?;
        self.state
            .windows
            .lock()
            .expect("application window scope lock poisoned")
            .push(window);
        Ok(())
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
        let dialogs = application.dialogs()?;
        AsyncOperation::start(
            application,
            move |completion| {
                dialogs.start_show_dialog(Some(&owner), Some(&dialog), Some(completion))
            },
            decode_string,
        )
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> Result<()> {
        self.state
            .tasks
            .spawn(self.context.dispatcher.clone(), future)
    }

    pub(crate) fn retain_subscription(&self, subscription: EventSubscription) {
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

    pub(crate) fn application(&self) -> &sys::ComPtr<sys::IAvnApplication> {
        &self.context.application
    }

    fn clear(&self) {
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
    pub fn load_from_env() -> Result<Self> {
        Self::load(discover_host_path()?)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let host = sys::Host::load(path).map_err(|error| Error::Load(error.to_string()))?;
        let activation = host.activation_factory()?;
        let application = activation.create_application()?;
        let controls = activation.create_control_factory()?;
        let dispatcher = activation.create_dispatcher()?;
        Ok(Self {
            application,
            controls,
            dispatcher,
            startup_arguments: None,
            _host: host,
        })
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

    /// Starts with no startup arguments at all.
    pub fn without_startup_arguments(mut self) -> Self {
        self.startup_arguments = Some(Vec::new());
        self
    }

    pub fn run(
        self,
        callback: impl FnOnce(&AppScope) -> Result<()> + Send + 'static,
    ) -> Result<()> {
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
        FACTORY.with(|current| {
            let previous = current.replace(Some(controls));
            let result = self.application.run(&handler);
            cleanup_scope.clear();
            current.replace(previous);
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
    std::env::args_os()
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

/// Publishes the startup arguments through the separately versioned desktop
/// file integration capability, before the managed lifetime starts.
fn publish_startup_arguments(
    application: &sys::ComPtr<sys::IAvnApplication>,
    arguments: &[String],
) -> Result<()> {
    let desktop = application.desktop_files()?;
    desktop.clear_startup_arguments()?;
    for argument in arguments {
        let encoded: Vec<u16> = argument.encode_utf16().chain(Some(0)).collect();
        desktop.add_startup_argument(Some(&encoded))?;
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
        | Error::InvalidEnumValue(_)
        | Error::InvalidAsyncValue
        | Error::InvalidViewModelMember { .. }
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

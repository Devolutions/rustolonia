//! Raw nano-COM bindings for the Avalonia NativeAOT host.
//!
//! IR-generated interfaces with handwritten ownership, host-loading, and
//! callback infrastructure.

/// Borrows an already terminated argument, or owns its terminated copy for
/// the duration of a synchronous ABI call. Embedded NULs retain ABI semantics.
pub(crate) fn terminated_utf16(value: &[u16]) -> std::borrow::Cow<'_, [u16]> {
    if value.contains(&0) {
        std::borrow::Cow::Borrowed(value)
    } else {
        std::borrow::Cow::Owned(value.iter().copied().chain(Some(0)).collect())
    }
}

mod app_handler;
mod application;
mod async_completion;
mod async_populator;
mod clipboard;
mod com;
mod command;
mod data_template;
mod dialogs;
mod dispatcher;
mod echo;
mod event_callback;
mod factory;
mod filters;
mod notification;
mod popup_placement;
mod selectors;
#[rustfmt::skip]
mod generated;
mod guid;
mod hresult;
mod rust_vm;
mod storage;
mod value_converter;

pub use app_handler::{app_handler, IAvnAppHandler};
pub use application::{IAvnApplication, IAvnResourceValue};
pub use async_completion::{async_completion, AsyncCompletionArgs, IAvnAsyncCompletion};
pub use async_populator::{async_populator, AsyncPopulator, PopulateCompletion};
pub use clipboard::{IAvnApplication4, IAvnClipboardData, IAVN_APPLICATION4_METHOD_COUNT};
pub use com::{ComInterface, ComPtr, IUnknown};
pub use command::{command, Command};
pub use data_template::{data_template, DataTemplate};
pub use dialogs::IAvnApplication5;
pub use dispatcher::{action, IAvnAction, IAvnDispatcher};
pub use echo::IAvnEcho;
pub use factory::IAvnActivationFactory;
pub use filters::{item_filter, text_filter, ItemFilter, TextFilter};
pub use generated::*;
pub use guid::Guid;
pub use hresult::{
    Error, Result, AVN_E_FIXTURE, AVN_E_NONSOLIDBRUSH, E_FAIL, E_INVALIDARG, E_NOINTERFACE,
    E_NOTIMPL, E_POINTER, S_OK,
};
pub use notification::{notification, Notification, NotificationSpec};
pub use popup_placement::{popup_placement, PopupPlacement, PopupPlacementResult};
pub use rust_vm::{
    rust_view_model, rust_view_model_with_control, rust_vm_range_batch, rust_vm_update_batch,
    IAvnRustRangeSource, IAvnRustViewModel, IAvnRustViewModel2, IAvnRustVmRangeBatch,
    IAvnRustVmSink, IAvnRustVmSink2, IAvnRustVmSink3, IAvnRustVmSink4, IAvnRustVmSink5,
    IAvnRustVmUpdateBatch, IAvnRustVmUpdateBatch2, IAvnRustVmUpdateOperation, MapKey,
    RustViewModelBeginTracked, RustViewModelCallbacks, RustViewModelControlCallbacks,
    RustVmBatchCompletion, RustVmBatchOwnershipCommit, RustVmDroppedRange, RustVmRangeItem,
    RustVmUpdate, RUST_VM_RANGE_FILL, RUST_VM_RANGE_INVALIDATE, RUST_VM_RANGE_RESET,
};
pub use selectors::{item_selector, text_selector, ItemSelector, TextSelector};
pub use storage::{
    activation_handler, file_drop_handler, storage_completion, ActivationArgs, FileDropArgs,
    IAvnActivationHandler, IAvnApplication3, IAvnFileDropHandler, IAvnFilePickerOptions,
    IAvnStorageCompletion, IAvnStorageItem, IAvnStorageItemList, StorageCompletionArgs,
    StorageItemData,
};
pub use value_converter::{
    rust_value_converter_provider, ConversionDirection, ConvertFn, ConverterAbiError,
    IAvnRustValueConverterProvider, ScalarKind, ScalarValue,
};

use libloading::Library;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::path::Path;
use std::ptr;
use std::sync::{Mutex, OnceLock};

type GetActivationFactoryFn = unsafe extern "C" fn(*mut *mut c_void) -> i32;
type FreeFn = unsafe extern "C" fn(*mut c_void);
type AllocUtf16Fn = unsafe extern "C" fn(i32) -> *mut u16;
type GetLastErrorFn = unsafe extern "C" fn(*mut *mut u16) -> i32;
type GetHostInfoFn = unsafe extern "C" fn(*mut HostInfoNative) -> i32;

/// SHA-256 of `include/avalonia-rust-abi.h`, computed by `build.rs`. A host
/// is only accepted if it reports this exact fingerprint.
pub const ABI_FINGERPRINT: &str = env!("RUSTOLONIA_ABI_FINGERPRINT");

#[repr(C)]
struct HostInfoNative {
    struct_size: u32,
    reserved: u32,
    version: *const std::ffi::c_char,
    abi_fingerprint: *const std::ffi::c_char,
}

/// Version information reported by a loaded host via `avn_get_host_info`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostInfo {
    /// The rustolonia release the host was built from.
    pub version: String,
    /// SHA-256 of the ABI header the host was built against.
    pub abi_fingerprint: String,
}

static FREE: OnceLock<FreeFn> = OnceLock::new();
static ALLOC_UTF16: OnceLock<AllocUtf16Fn> = OnceLock::new();
static HOST_EXPORTS: Mutex<()> = Mutex::new(());

/// Failure while loading a native Avalonia host.
#[derive(Debug)]
pub enum HostLoadError {
    Library(libloading::Error),
    IncompatibleAllocator,
    /// The host was built against a different ABI than this crate. `found`
    /// is `None` for hosts that predate `avn_get_host_info`.
    IncompatibleHost {
        expected: &'static str,
        found: Option<HostInfo>,
    },
}

impl std::fmt::Display for HostLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Library(error) => error.fmt(formatter),
            Self::IncompatibleAllocator => formatter.write_str(
                "Avalonia Host exports a different UTF-16 allocator than the already loaded host",
            ),
            Self::IncompatibleHost { expected, found } => match found {
                Some(found) => write!(
                    formatter,
                    "rustolonia host {} was built for ABI {}, but rustolonia-sys {} expects ABI {}; \
                     use the host released with rustolonia-sys {}",
                    found.version,
                    found.abi_fingerprint,
                    env!("CARGO_PKG_VERSION"),
                    expected,
                    env!("CARGO_PKG_VERSION"),
                ),
                None => write!(
                    formatter,
                    "rustolonia host does not export avn_get_host_info, so it predates \
                     rustolonia-sys {}; use the host released with this version",
                    env!("CARGO_PKG_VERSION"),
                ),
            },
        }
    }
}

/// Accepts `found` only if its ABI fingerprint is exactly `expected`.
fn check_host_compatibility(
    expected: &'static str,
    found: Option<HostInfo>,
) -> std::result::Result<HostInfo, HostLoadError> {
    match found {
        Some(info) if info.abi_fingerprint == expected => Ok(info),
        found => Err(HostLoadError::IncompatibleHost { expected, found }),
    }
}

/// Calls `avn_get_host_info`. Returns `None` if the call fails.
unsafe fn query_host_info(get_host_info: GetHostInfoFn) -> Option<HostInfo> {
    let mut native = HostInfoNative {
        struct_size: std::mem::size_of::<HostInfoNative>() as u32,
        reserved: 0,
        version: ptr::null(),
        abi_fingerprint: ptr::null(),
    };
    if get_host_info(&mut native) < 0 {
        return None;
    }
    let read = |value: *const std::ffi::c_char| {
        (!value.is_null()).then(|| {
            std::ffi::CStr::from_ptr(value)
                .to_string_lossy()
                .into_owned()
        })
    };
    Some(HostInfo {
        version: read(native.version).unwrap_or_default(),
        abi_fingerprint: read(native.abi_fingerprint).unwrap_or_default(),
    })
}

impl std::error::Error for HostLoadError {}

impl From<libloading::Error> for HostLoadError {
    fn from(error: libloading::Error) -> Self {
        Self::Library(error)
    }
}

/// Takes ownership of a host-allocated null-terminated UTF-16 string.
///
/// # Safety
/// `ptr` must be null or allocated by the loaded Avalonia host.
pub unsafe fn take_utf16(ptr: *mut u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let value = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
    FREE.get()
        .expect("Avalonia Host must be loaded before reading ABI strings")(ptr.cast());
    Some(value)
}

pub(crate) unsafe fn clone_utf16(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    Some(String::from_utf16_lossy(std::slice::from_raw_parts(
        ptr, len,
    )))
}

/// Allocates a host-owned, null-terminated UTF-16 buffer (via
/// `avn_alloc_utf16`, the same allocator `avn_free` releases) and copies
/// `value` into it. Returns `None` if the host rejects the allocation.
/// Rust code must use this — not its own allocator — whenever it hands an
/// owned string back across the ABI, so the managed caller can free it
/// without a cross-allocator mismatch.
///
/// # Panics
/// Panics if called before a `Host` has been loaded.
pub(crate) fn alloc_utf16(value: &str) -> Option<*mut u16> {
    let alloc = ALLOC_UTF16
        .get()
        .expect("Avalonia Host must be loaded before allocating ABI strings");
    let units: Vec<u16> = value.encode_utf16().collect();
    let length = i32::try_from(units.len()).ok()?;
    unsafe {
        let buffer = alloc(length);
        if buffer.is_null() {
            return None;
        }
        ptr::copy_nonoverlapping(units.as_ptr(), buffer, units.len());
        *buffer.add(units.len()) = 0;
        Some(buffer)
    }
}

/// Allocates a host-owned, null-terminated UTF-16 buffer of `length` UTF-16
/// units (not including the terminator) via `avn_alloc_utf16`. The caller
/// writes the units and releases the buffer with [`free_utf16`].
///
/// # Panics
/// Panics if called before a `Host` has been loaded.
pub fn alloc_utf16_raw(length: i32) -> Option<*mut u16> {
    let alloc = ALLOC_UTF16
        .get()
        .expect("Avalonia Host must be loaded before allocating ABI strings");
    if length < 0 {
        return None;
    }
    unsafe {
        let buffer = alloc(length);
        if buffer.is_null() {
            None
        } else {
            *buffer = 0;
            Some(buffer)
        }
    }
}

/// Releases a host-owned UTF-16 buffer allocated by `avn_alloc_utf16`
/// (including via [`alloc_utf16_raw`]).
///
/// # Safety
/// `ptr` must be null or allocated by the loaded Avalonia host, and must not
/// be freed twice.
pub unsafe fn free_utf16(ptr: *mut u16) {
    if !ptr.is_null() {
        FREE.get()
            .expect("Avalonia Host must be loaded before freeing ABI strings")(ptr.cast());
    }
}

/// Frees a host-owned UTF-16 buffer when a host has been loaded, and does
/// nothing otherwise. CCWs use this for payload slots whose strings the host
/// allocates: outside a host session (unit tests) there is no allocator to
/// answer to.
pub(crate) unsafe fn free_utf16_if_host(ptr: *mut u16) {
    if !ptr.is_null() && FREE.get().is_some() {
        free_utf16(ptr);
    }
}

/// The host's raw UTF-16 allocator (length in UTF-16 units, including the
/// terminator the caller writes), when a host session has been loaded. CCWs
/// use this for strings the host will later release; without a host there is
/// no allocator, and callers surface E_NOTIMPL instead.
pub(crate) fn alloc_utf16_provider() -> Option<AllocUtf16Fn> {
    ALLOC_UTF16.get().copied()
}

pub struct Host {
    // Successful hosts deliberately remain loaded for the process lifetime:
    // FREE/ALLOC_UTF16 are process-wide ABI callbacks and may service strings
    // retained by Rust after an individual Host value is dropped.
    _lib: ManuallyDrop<Library>,
    _dependencies: ManuallyDrop<Vec<Library>>,
    get_activation_factory: GetActivationFactoryFn,
    free: FreeFn,
    get_last_error: GetLastErrorFn,
    info: HostInfo,
}

impl Host {
    pub fn load(path: impl AsRef<Path>) -> std::result::Result<Self, HostLoadError> {
        unsafe {
            let path = path.as_ref();
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            let directory = path.parent().unwrap_or_else(|| Path::new("."));
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            let mut dependencies = Vec::new();
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            let dependencies: Vec<Library> = Vec::new();
            #[cfg(target_os = "windows")]
            for name in ["libSkiaSharp.dll", "libHarfBuzzSharp.dll"] {
                let dependency = directory.join(name);
                if dependency.exists() {
                    dependencies.push(Library::new(dependency)?);
                }
            }
            #[cfg(target_os = "macos")]
            for name in ["libSkiaSharp.dylib", "libHarfBuzzSharp.dylib"] {
                let dependency = directory.join(name);
                if dependency.exists() {
                    dependencies.push(Library::new(dependency)?);
                }
            }
            let lib = Library::new(path)?;
            let found = match lib.get::<GetHostInfoFn>(b"avn_get_host_info\0") {
                Ok(get_host_info) => query_host_info(*get_host_info),
                Err(_) => None,
            };
            let info = check_host_compatibility(ABI_FINGERPRINT, found)?;
            let get_activation_factory =
                *lib.get::<GetActivationFactoryFn>(b"avn_get_activation_factory\0")?;
            let free = *lib.get::<FreeFn>(b"avn_free\0")?;
            let alloc_utf16 = *lib.get::<AllocUtf16Fn>(b"avn_alloc_utf16\0")?;
            let get_last_error = *lib.get::<GetLastErrorFn>(b"avn_get_last_error\0")?;
            let _exports = HOST_EXPORTS
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if FREE
                .get()
                .is_some_and(|published| *published as usize != free as usize)
                || ALLOC_UTF16
                    .get()
                    .is_some_and(|published| *published as usize != alloc_utf16 as usize)
            {
                return Err(HostLoadError::IncompatibleAllocator);
            }
            if FREE.get().is_none() {
                FREE.set(free)
                    .expect("host allocator publication must be serialized");
            }
            if ALLOC_UTF16.get().is_none() {
                ALLOC_UTF16
                    .set(alloc_utf16)
                    .expect("host allocator publication must be serialized");
            }
            Ok(Self {
                _lib: ManuallyDrop::new(lib),
                _dependencies: ManuallyDrop::new(dependencies),
                get_activation_factory,
                free,
                get_last_error,
                info,
            })
        }
    }

    /// Version and ABI fingerprint reported by this host.
    pub fn info(&self) -> &HostInfo {
        &self.info
    }

    pub fn activation_factory(&self) -> Result<ComPtr<IAvnActivationFactory>> {
        unsafe {
            let mut unk = std::ptr::null_mut();
            hresult::check((self.get_activation_factory)(&mut unk))?;
            let unk = ComPtr::<IUnknown>::from_raw(unk.cast()).ok_or(Error(hresult::E_POINTER))?;
            unk.query_interface()
        }
    }

    /// # Safety
    ///
    /// `ptr` must be null or an allocation returned by this host, and it must
    /// not have been freed previously.
    pub unsafe fn free(&self, ptr: *mut c_void) {
        if !ptr.is_null() {
            (self.free)(ptr);
        }
    }

    pub fn last_error(&self) -> Option<String> {
        unsafe {
            let mut value = std::ptr::null_mut();
            if (self.get_last_error)(&mut value) < 0 || value.is_null() {
                return None;
            }
            let mut len = 0;
            while *value.add(len) != 0 {
                len += 1;
            }
            let result = String::from_utf16_lossy(std::slice::from_raw_parts(value, len));
            (self.free)(value.cast());
            Some(result)
        }
    }
}

#[cfg(test)]
mod host_info_tests {
    use super::*;

    fn info(fingerprint: &str) -> HostInfo {
        HostInfo {
            version: "0.1.0".into(),
            abi_fingerprint: fingerprint.into(),
        }
    }

    #[test]
    fn abi_fingerprint_is_lowercase_sha256() {
        assert_eq!(ABI_FINGERPRINT.len(), 64);
        assert!(ABI_FINGERPRINT
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    }

    #[test]
    fn matching_fingerprint_is_accepted() {
        let accepted = check_host_compatibility("abc", Some(info("abc"))).unwrap();
        assert_eq!(accepted, info("abc"));
    }

    #[test]
    fn mismatched_fingerprint_is_rejected() {
        let error = check_host_compatibility("abc", Some(info("def"))).unwrap_err();
        assert!(matches!(
            &error,
            HostLoadError::IncompatibleHost { expected: "abc", found: Some(found) } if found.abi_fingerprint == "def"
        ));
        let message = error.to_string();
        assert!(message.contains("ABI def"), "{message}");
        assert!(message.contains("expects ABI abc"), "{message}");
    }

    #[test]
    fn missing_host_info_is_rejected() {
        let error = check_host_compatibility("abc", None).unwrap_err();
        assert!(matches!(
            error,
            HostLoadError::IncompatibleHost { found: None, .. }
        ));
        assert!(error.to_string().contains("avn_get_host_info"));
    }

    unsafe extern "C" fn fake_host_info(info: *mut HostInfoNative) -> i32 {
        if (*info).struct_size < std::mem::size_of::<HostInfoNative>() as u32 {
            return hresult::E_INVALIDARG;
        }
        (*info).version = c"1.2.3".as_ptr();
        (*info).abi_fingerprint = c"feed".as_ptr();
        0
    }

    unsafe extern "C" fn failing_host_info(_: *mut HostInfoNative) -> i32 {
        hresult::E_POINTER
    }

    #[test]
    fn query_reads_utf8_strings() {
        let found = unsafe { query_host_info(fake_host_info) }.unwrap();
        assert_eq!(
            found,
            HostInfo {
                version: "1.2.3".into(),
                abi_fingerprint: "feed".into()
            }
        );
        assert!(unsafe { query_host_info(failing_host_info) }.is_none());
    }
}
extern crate self as rustolonia;

#[rustfmt::skip]
mod generated;
pub mod android;
mod async_runtime;
pub mod callbacks;
pub mod clipboard;
pub mod data_shapes;
pub mod ios;
pub mod menu;
pub mod native_menu;
pub mod pixel_surface;
pub mod reactive;
mod resource_scope;
mod runtime;
pub mod shell;
pub mod storage;
pub mod style;
pub mod value_converter;
pub mod values;
pub mod view;
pub mod view_model;

/// Escape hatch for low-level `rustolonia-sys` COM handles (`HostExports`, `ComPtr`).
pub mod raw {
    pub use rustolonia_sys::{ComPtr, HostExports};
}
#[doc(hidden)]
pub use rustolonia_sys::HostExports;

pub use async_runtime::{AsyncOperation, AsyncValue, BlockingOperation};
pub use callbacks::{
    AsyncPopulator, IntoContent, ItemFilter, ItemSelector, Notification, PopulateRequest,
    PopupPlacementCallback, PopupPlacementInput, PopupPlacementOutput, TextFilter, TextSelector,
};
pub use clipboard::{ClipboardCapabilities, ClipboardData, ClipboardFilesOperation};
pub use data_shapes::{CancellationToken, MapKey, RangeBatch, RangeRequest};
pub use generated::*;
pub use menu::{RecentFileList, RecentFilesChange};
pub use native_menu::{
    MenuBar, MenuBuilder, MenuItemState, NativeMenu, NativeMenuItem, NativeMenuItemKind,
};
pub use pixel_surface::{PixelSurface, SurfaceInput};
pub use reactive::{
    provide_context, use_context, Effect, Memo, NodeRef, PathTwoWay, PropertyPath, ReactiveScope,
    Resource, ResourceState, Signal, SignalWriter, TwoWay,
};
pub use resource_scope::ResourceScope;
pub use runtime::{
    discover_host_path, with_ui_error_handler, App, AppContext, AppScope, AsControl, Downcast,
    EventSubscription, IntoEventResult, LocalizationResources, NonOwningRef, ResourceValue,
    SingleViewSession, ThemeVariant, UiSender, Upcast, HOST_NATIVE_LIB_ENV_VAR,
};
pub use rustolonia_macros::{color, component, fmt, lens, rsx, tracks, Lens};
pub use shell::{
    NativePopupEvent, NativeWindowHandle, NativeWindowHandleKind, ShellTray, ShutdownMode,
    TrayEvent, TrayMenuItem,
};
pub use storage::{
    ActivationEvent, DragDropEffects, FileDropEvent, FileTypeFilter, FolderPickerOptions,
    OpenFilePickerOptions, PickerOutcome, SaveFilePickerOptions, StorageCapabilities, StorageItem,
    StorageItemKind, StorageLocation, StoragePickerOperation, WellKnownFolder,
};
pub use style::{BoxShadow, BoxShadows, Shortcut, Style, StyleSheet};
pub use value_converter::{
    ConversionDirection, ConvertedBinding, ConverterTarget, RegisteredValueConverter, ScalarKind,
    ScalarPath, ScalarValue, ValueConverterDispatch,
};
pub use values::{CivilDate, CivilTime, Decimal, GridLength, GridTrack, GridTracks};
pub use view::{
    data_template_reactive, theme_brush, use_theme, use_window, AnyEl, Command, ControlSlot, El,
    For, IntoChildEl, IntoCommandProp, IntoContentEl, IntoProp, IntoValue, ItemNameScope,
    ItemsView, KeyedListReconciler, PanelChildSlot, PropBinding, Show, TryIntoGridTracks,
    ViewRegistry,
};

impl Variant {
    /// Copies a callback parameter whose UTF-16 storage is owned by the ABI
    /// thunk and must remain alive until that thunk returns.
    unsafe fn from_borrowed_abi(value: rustolonia_sys::AvnVariant) -> rustolonia_sys::Result<Self> {
        use rustolonia_sys::AvnVariant;
        match value.tag {
            AvnVariant::TAG_NONE => Ok(Self::None),
            AvnVariant::TAG_UTF16 => rustolonia_sys::clone_utf16(value.utf16)
                .map(Self::Utf16)
                .ok_or(rustolonia_sys::Error(rustolonia_sys::E_POINTER)),
            AvnVariant::TAG_I32 => Ok(Self::I32(value.i32)),
            AvnVariant::TAG_F64 => Ok(Self::F64(value.f64)),
            AvnVariant::TAG_BOOL => Ok(Self::Bool(value.i32 != 0)),
            _ => Err(rustolonia_sys::Error(rustolonia_sys::E_INVALIDARG)),
        }
    }
}

impl VariantAbi {
    /// Transfers the argument to an ABI method that consumes its UTF-16 payload.
    fn into_owned_abi(self) -> rustolonia_sys::AvnVariant {
        let value = *self;
        std::mem::forget(self);
        value
    }
}

/// Builds a data template from UI-thread Rust closures, wrapping the sys-level
/// CCW in the safe `DataTemplate` type the template setters accept.
pub fn data_template(
    matches: impl FnMut(Variant) -> bool + 'static,
    build: impl FnMut() -> Result<Control> + 'static,
) -> Result<DataTemplate> {
    let matches_cb = crate::reactive::UiCallbackHandle::new(matches);
    let build_cb = crate::reactive::UiCallbackHandle::new(build);
    let sys_template = rustolonia_sys::data_template(
        move |data: rustolonia_sys::AvnVariant| {
            let variant = unsafe { Variant::from_borrowed_abi(data)? };
            matches_cb
                .with(|cb| cb(variant))
                .map_err(|_| rustolonia_sys::Error(rustolonia_sys::E_FAIL))
        },
        move || {
            let control = build_cb
                .with(|cb| cb())
                .map_err(|_| rustolonia_sys::Error(rustolonia_sys::E_FAIL))?
                .map_err(|error| match error {
                    crate::Error::Abi(error) => error,
                    other => {
                        crate::runtime::report_ui_error("data template build failed", &other);
                        rustolonia_sys::Error(rustolonia_sys::E_FAIL)
                    }
                })?;
            Ok(control.raw.clone())
        },
    );
    Ok(DataTemplate::from_raw(sys_template.as_com_ptr().clone()))
}

#[derive(Debug)]
pub enum Error {
    Abi(rustolonia_sys::Error),
    Load(String),
    NoUiContext,
    UiContextAlreadyActive,
    InvalidEnumValue(i32),
    InvalidAsyncValue,
    InvalidResourceCatalog(String),
    MissingResource {
        key: String,
        culture: String,
    },
    MissingScopedResource {
        key: String,
        theme: ThemeVariant,
    },
    ResourceTypeMismatch {
        key: String,
        expected: &'static str,
        actual: &'static str,
    },
    InvalidResourceScope(String),
    Binding(String),
    ReadOnlyBindingPath,
    DuplicateItemKey,
    CollectionUpdateRollback {
        update: Box<Error>,
        rollback: Box<Error>,
    },
    ControlSlotNotMounted,
    ControlSlotAlreadyMounted,
    InvalidViewRegistry(String),
    InvalidViewModelMember {
        kind: &'static str,
        id: i32,
    },
    InvalidValue(String),
    Async {
        hresult: i32,
        message: String,
    },
}

impl From<rustolonia_sys::Error> for Error {
    fn from(value: rustolonia_sys::Error) -> Self {
        Self::Abi(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Abi(error) => error.fmt(formatter),
            Self::Load(error) => write!(formatter, "failed to load Avalonia host: {error}"),
            Self::NoUiContext => {
                formatter.write_str("Avalonia controls must be created on an active UI thread")
            }
            Self::UiContextAlreadyActive => {
                formatter.write_str("an Avalonia UI context is already active on this thread")
            }
            Self::InvalidEnumValue(value) => {
                write!(formatter, "invalid projected enum value {value}")
            }
            Self::InvalidAsyncValue => formatter.write_str("invalid asynchronous result value"),
            Self::InvalidResourceCatalog(error) => {
                write!(formatter, "invalid localization resource catalog: {error}")
            }
            Self::MissingResource { key, culture } => {
                write!(
                    formatter,
                    "missing localized string '{key}' for culture '{culture}'"
                )
            }
            Self::MissingScopedResource { key, theme } => {
                write!(formatter, "missing resource '{key}' for theme {theme:?}")
            }
            Self::ResourceTypeMismatch {
                key,
                expected,
                actual,
            } => write!(
                formatter,
                "resource '{key}' has type {actual}, but {expected} was requested"
            ),
            Self::InvalidResourceScope(error) => {
                write!(formatter, "invalid resource scope: {error}")
            }
            Self::Binding(error) => write!(formatter, "binding failed: {error}"),
            Self::ReadOnlyBindingPath => {
                formatter.write_str("a two-way binding requires a writable property path")
            }
            Self::DuplicateItemKey => {
                formatter.write_str("a keyed item list contains duplicate keys")
            }
            Self::CollectionUpdateRollback { update, rollback } => write!(
                formatter,
                "failed to update a control collection: {update}; restoring it also failed: {rollback}"
            ),
            Self::ControlSlotNotMounted => {
                formatter.write_str("the control slot has not been mounted")
            }
            Self::ControlSlotAlreadyMounted => {
                formatter.write_str("the control slot is already mounted")
            }
            Self::InvalidViewRegistry(error) => {
                write!(formatter, "invalid view registry: {error}")
            }
            Self::InvalidViewModelMember { kind, id } => {
                write!(formatter, "invalid view-model {kind} ID {id}")
            }
            Self::InvalidValue(error) => write!(formatter, "invalid value: {error}"),
            Self::Async { hresult, message } => {
                write!(
                    formatter,
                    "asynchronous operation failed (0x{hresult:08X}): {message}"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod variant_tests {
    use super::*;

    #[test]
    fn callback_variant_copies_borrowed_utf16_without_taking_ownership() {
        let text = "端口-😀";
        let mut units = text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let variant = rustolonia_sys::AvnVariant {
            tag: rustolonia_sys::AvnVariant::TAG_UTF16,
            utf16: units.as_mut_ptr(),
            ..Default::default()
        };
        for _ in 0..10 {
            assert_eq!(
                unsafe { Variant::from_borrowed_abi(variant) }.unwrap(),
                Variant::Utf16(text.to_string())
            );
        }
        assert_eq!(String::from_utf16(&units[..units.len() - 1]).unwrap(), text);

        let invalid = rustolonia_sys::AvnVariant {
            tag: rustolonia_sys::AvnVariant::TAG_UTF16,
            utf16: std::ptr::null_mut(),
            ..Default::default()
        };
        assert!(unsafe { Variant::from_borrowed_abi(invalid) }.is_err());
    }
}

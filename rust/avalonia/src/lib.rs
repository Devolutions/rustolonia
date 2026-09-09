#[rustfmt::skip]
mod generated;
mod async_runtime;
pub mod clipboard;
pub mod data_shapes;
pub mod menu;
mod runtime;
pub mod storage;
pub mod value_converter;
pub mod view_model;

pub use async_runtime::{AsyncOperation, AsyncValue};
pub use clipboard::{ClipboardCapabilities, ClipboardData, ClipboardFilesOperation};
pub use data_shapes::{CancellationToken, MapKey, RangeBatch, RangeRequest};
pub use generated::*;
pub use menu::{RecentFileList, RecentFilesChange};
pub use runtime::{
    discover_host_path, App, AppContext, AppScope, AsControl, EventSubscription, ResourceValue,
    ThemeVariant, HOST_NATIVE_LIB_ENV_VAR,
};
pub use storage::{
    ActivationEvent, DragDropEffects, FileDropEvent, FileTypeFilter, FolderPickerOptions,
    OpenFilePickerOptions, PickerOutcome, SaveFilePickerOptions, StorageCapabilities, StorageItem,
    StorageItemKind, StorageLocation, StoragePickerOperation, WellKnownFolder,
};
pub use value_converter::{ConversionDirection, ScalarKind, ScalarValue};

/// Builds a data template from Rust closures, wrapping the sys-level CCW in
/// the safe `DataTemplate` type the template setters accept.
pub fn data_template(
    matches: impl FnMut(Variant) -> bool + Send + 'static,
    build: impl FnMut() -> Result<Control> + Send + 'static,
) -> Result<DataTemplate> {
    let mut matches = matches;
    let mut build = build;
    let sys_template = avalonia_sys::data_template(
        move |data: avalonia_sys::AvnVariant| Ok(matches(Variant::from_abi(data))),
        move || {
            let control = build().map_err(|error| match error {
                crate::Error::Abi(error) => error,
                _ => avalonia_sys::Error(avalonia_sys::E_FAIL),
            })?;
            Ok(control.raw.clone())
        },
    );
    Ok(DataTemplate::from_raw(sys_template.as_com_ptr().clone()))
}

#[derive(Debug)]
pub enum Error {
    Abi(avalonia_sys::Error),
    Load(String),
    NoUiContext,
    InvalidEnumValue(i32),
    InvalidAsyncValue,
    InvalidViewModelMember { kind: &'static str, id: i32 },
    Async { hresult: i32, message: String },
}

impl From<avalonia_sys::Error> for Error {
    fn from(value: avalonia_sys::Error) -> Self {
        Self::Abi(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Abi(error) => error.fmt(formatter),
            Self::Load(error) => write!(formatter, "failed to load Avalonia host: {error}"),
            Self::NoUiContext => {
                formatter.write_str("Avalonia controls must be created inside App::run")
            }
            Self::InvalidEnumValue(value) => {
                write!(formatter, "invalid projected enum value {value}")
            }
            Self::InvalidAsyncValue => formatter.write_str("invalid asynchronous result value"),
            Self::InvalidViewModelMember { kind, id } => {
                write!(formatter, "invalid view-model {kind} ID {id}")
            }
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

//! Platform-neutral desktop file integration for Rust consumers.
//!
//! Everything here goes through Avalonia's own abstractions
//! (`TopLevel.StorageProvider`, `IStorageFile`/`IStorageFolder`, the drag-and-drop
//! routed events and the desktop lifetime activation arguments) over the
//! separately versioned stage 29 capability interface. No raw platform dialog
//! API is involved, and no application-specific host method is introduced.
//!
//! See `rust/DESKTOP_FILES.md` for the design, the drag-effect negotiation rule
//! and the file-association packaging metadata.

use crate::async_runtime::{AsyncFailure, CompletionSlot};
use crate::runtime::{AsControl, EventSubscription};
use crate::{AppContext, AppScope, Error, Result, Window};
use avalonia_sys as sys;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// Whether a storage item is a file or a folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageItemKind {
    File,
    Folder,
}

/// One selected, dropped or activated storage item.
///
/// `uri` is always present. `local_path` is `None` whenever the platform has no
/// filesystem path for the item, which is normal for Android `content:` URIs,
/// browser handles and other non-local providers, so consumers must never assume
/// a path exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageItem {
    kind: StorageItemKind,
    name: String,
    uri: String,
    local_path: Option<PathBuf>,
}

impl StorageItem {
    pub fn kind(&self) -> StorageItemKind {
        self.kind
    }

    pub fn is_folder(&self) -> bool {
        self.kind == StorageItemKind::Folder
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The absolute URI of the item. Always available.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// The local filesystem path, when the platform provides one.
    pub fn local_path(&self) -> Option<&Path> {
        self.local_path.as_deref()
    }

    pub(crate) fn from_abi(data: sys::StorageItemData) -> Result<Self> {
        let kind = match data.kind {
            0 => StorageItemKind::File,
            1 => StorageItemKind::Folder,
            value => return Err(Error::InvalidEnumValue(value)),
        };
        Ok(Self {
            kind,
            name: data.name.unwrap_or_default(),
            uri: data.uri.ok_or(Error::InvalidAsyncValue)?,
            local_path: data.local_path.map(PathBuf::from),
        })
    }

    pub(crate) fn from_abi_list(items: Vec<sys::StorageItemData>) -> Result<Vec<Self>> {
        items.into_iter().map(Self::from_abi).collect()
    }
}

/// A file type shown by a picker.
///
/// Every list is optional and platform-specific: `patterns` are used on Windows,
/// Linux and the browser, `mime_types` on Linux, Android and the browser, and
/// `apple_uniform_type_identifiers` on Apple platforms. Supplying all three is
/// what makes one filter work everywhere.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileTypeFilter {
    pub name: String,
    pub patterns: Vec<String>,
    pub mime_types: Vec<String>,
    pub apple_uniform_type_identifiers: Vec<String>,
}

impl FileTypeFilter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Adds a glob pattern such as `*.log`.
    pub fn with_pattern(mut self, value: impl Into<String>) -> Self {
        self.patterns.push(value.into());
        self
    }

    pub fn with_mime_type(mut self, value: impl Into<String>) -> Self {
        self.mime_types.push(value.into());
        self
    }

    pub fn with_apple_uniform_type_identifier(mut self, value: impl Into<String>) -> Self {
        self.apple_uniform_type_identifiers.push(value.into());
        self
    }

    /// Adds one bare extension (`"log"` or `".log"`) as a glob pattern.
    pub fn with_extension(self, value: impl AsRef<str>) -> Self {
        let value = value.as_ref().trim_start_matches('.');
        self.with_pattern(format!("*.{value}"))
    }
}

/// A well-known user folder a picker can start in.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WellKnownFolder {
    Desktop = 0,
    Documents = 1,
    Downloads = 2,
    Music = 3,
    Pictures = 4,
    Videos = 5,
}

/// Where a picker should start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageLocation {
    /// A local filesystem path; when it names a file, its parent folder is used.
    Path(PathBuf),
    /// An absolute URI, including non-`file` schemes on platforms that use them.
    Uri(String),
    WellKnown(WellKnownFolder),
}

impl From<WellKnownFolder> for StorageLocation {
    fn from(value: WellKnownFolder) -> Self {
        Self::WellKnown(value)
    }
}

impl From<PathBuf> for StorageLocation {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<&Path> for StorageLocation {
    fn from(value: &Path) -> Self {
        Self::Path(value.to_path_buf())
    }
}

/// Options for the multi-select open-file picker.
#[derive(Clone, Debug, Default)]
pub struct OpenFilePickerOptions {
    pub title: Option<String>,
    pub allow_multiple: bool,
    pub suggested_file_name: Option<String>,
    pub suggested_start_location: Option<StorageLocation>,
    pub file_types: Vec<FileTypeFilter>,
    /// Index into `file_types` preselected when the dialog opens.
    pub suggested_file_type: Option<usize>,
}

impl OpenFilePickerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = Some(value.into());
        self
    }

    pub fn allow_multiple(mut self, value: bool) -> Self {
        self.allow_multiple = value;
        self
    }

    pub fn suggested_file_name(mut self, value: impl Into<String>) -> Self {
        self.suggested_file_name = Some(value.into());
        self
    }

    pub fn start_in(mut self, value: impl Into<StorageLocation>) -> Self {
        self.suggested_start_location = Some(value.into());
        self
    }

    pub fn file_type(mut self, value: FileTypeFilter) -> Self {
        self.file_types.push(value);
        self
    }

    pub fn suggested_file_type(mut self, index: usize) -> Self {
        self.suggested_file_type = Some(index);
        self
    }
}

/// Options for the folder picker.
///
/// `allow_multiple` is honoured only where the platform folder picker supports
/// multi-selection; elsewhere a single folder is returned.
#[derive(Clone, Debug, Default)]
pub struct FolderPickerOptions {
    pub title: Option<String>,
    pub allow_multiple: bool,
    pub suggested_start_location: Option<StorageLocation>,
}

impl FolderPickerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = Some(value.into());
        self
    }

    pub fn allow_multiple(mut self, value: bool) -> Self {
        self.allow_multiple = value;
        self
    }

    pub fn start_in(mut self, value: impl Into<StorageLocation>) -> Self {
        self.suggested_start_location = Some(value.into());
        self
    }
}

/// Options for the save/export picker.
#[derive(Clone, Debug, Default)]
pub struct SaveFilePickerOptions {
    pub title: Option<String>,
    pub suggested_file_name: Option<String>,
    pub suggested_start_location: Option<StorageLocation>,
    pub default_extension: Option<String>,
    /// `None` keeps the platform default.
    pub show_overwrite_prompt: Option<bool>,
    pub file_types: Vec<FileTypeFilter>,
    pub suggested_file_type: Option<usize>,
}

impl SaveFilePickerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = Some(value.into());
        self
    }

    pub fn suggested_file_name(mut self, value: impl Into<String>) -> Self {
        self.suggested_file_name = Some(value.into());
        self
    }

    pub fn start_in(mut self, value: impl Into<StorageLocation>) -> Self {
        self.suggested_start_location = Some(value.into());
        self
    }

    pub fn default_extension(mut self, value: impl Into<String>) -> Self {
        self.default_extension = Some(value.into());
        self
    }

    pub fn show_overwrite_prompt(mut self, value: bool) -> Self {
        self.show_overwrite_prompt = Some(value);
        self
    }

    pub fn file_type(mut self, value: FileTypeFilter) -> Self {
        self.file_types.push(value);
        self
    }

    pub fn suggested_file_type(mut self, index: usize) -> Self {
        self.suggested_file_type = Some(index);
        self
    }
}

/// The result of a picker.
///
/// Dismissing a dialog is [`PickerOutcome::Cancelled`], never an error; a real
/// failure (or an aborted operation) surfaces as `Err` from the future.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickerOutcome {
    Selected(Vec<StorageItem>),
    Cancelled,
}

impl PickerOutcome {
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    /// The selected items, or an empty slice when the picker was cancelled.
    pub fn items(&self) -> &[StorageItem] {
        match self {
            Self::Selected(items) => items,
            Self::Cancelled => &[],
        }
    }

    /// The first selected item, if any. Convenient for save/export.
    pub fn first(&self) -> Option<&StorageItem> {
        self.items().first()
    }

    pub fn into_items(self) -> Vec<StorageItem> {
        match self {
            Self::Selected(items) => items,
            Self::Cancelled => Vec::new(),
        }
    }
}

/// Which pickers the window's top-level actually supports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StorageCapabilities {
    pub can_open: bool,
    pub can_save: bool,
    pub can_pick_folder: bool,
}

impl StorageCapabilities {
    fn from_bits(bits: i32) -> Self {
        Self {
            can_open: bits & 1 != 0,
            can_save: bits & 2 != 0,
            can_pick_folder: bits & 4 != 0,
        }
    }
}

/// Drag-and-drop effect mask, mirroring `Avalonia.Input.DragDropEffects`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DragDropEffects(i32);

impl DragDropEffects {
    pub const NONE: Self = Self(0);
    pub const COPY: Self = Self(1);
    pub const MOVE: Self = Self(2);
    pub const LINK: Self = Self(4);

    pub fn bits(self) -> i32 {
        self.0
    }

    pub fn from_bits_truncate(bits: i32) -> Self {
        Self(bits & 0b111)
    }

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for DragDropEffects {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for DragDropEffects {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// One incoming drag notification.
///
/// The host answers the platform synchronously with the intersection of the
/// allowed effects and the conservative mask supplied at subscription time, then
/// delivers this notification asynchronously, so a Rust handler can never block
/// the platform drag loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDropEvent {
    Enter {
        allowed_effects: DragDropEffects,
        effective_effects: DragDropEffects,
        items: Vec<StorageItem>,
    },
    Over {
        allowed_effects: DragDropEffects,
        effective_effects: DragDropEffects,
        items: Vec<StorageItem>,
    },
    Leave,
    Drop {
        allowed_effects: DragDropEffects,
        effective_effects: DragDropEffects,
        items: Vec<StorageItem>,
    },
}

impl FileDropEvent {
    /// The dropped/hovered items; empty for [`FileDropEvent::Leave`].
    pub fn items(&self) -> &[StorageItem] {
        match self {
            Self::Enter { items, .. } | Self::Over { items, .. } | Self::Drop { items, .. } => {
                items
            }
            Self::Leave => &[],
        }
    }

    /// The effect the host reported back to the platform.
    pub fn effective_effects(&self) -> DragDropEffects {
        match self {
            Self::Enter {
                effective_effects, ..
            }
            | Self::Over {
                effective_effects, ..
            }
            | Self::Drop {
                effective_effects, ..
            } => *effective_effects,
            Self::Leave => DragDropEffects::NONE,
        }
    }

    /// True when the host accepted the drag, i.e. the negotiated effect is not
    /// empty.
    pub fn accepted(&self) -> bool {
        !self.effective_effects().is_empty()
    }

    pub(crate) fn from_abi(arguments: &mut sys::FileDropArgs) -> Result<Self> {
        let allowed_effects = DragDropEffects::from_bits_truncate(arguments.allowed_effects);
        let effective_effects = DragDropEffects::from_bits_truncate(arguments.effective_effects);
        let items = StorageItem::from_abi_list(std::mem::take(&mut arguments.items))?;
        Ok(match arguments.kind {
            0 => Self::Enter {
                allowed_effects,
                effective_effects,
                items,
            },
            1 => Self::Over {
                allowed_effects,
                effective_effects,
                items,
            },
            2 => Self::Leave,
            3 => Self::Drop {
                allowed_effects,
                effective_effects,
                items,
            },
            kind => return Err(Error::InvalidEnumValue(kind)),
        })
    }
}

/// How the application was activated after startup.
///
/// Only some platforms raise these (macOS "open with" while already running,
/// protocol activation, dock reopen). Where the desktop lifetime has no
/// activation feature the subscription stays valid and simply never fires, so
/// consumers need no platform branches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivationEvent {
    Files(Vec<StorageItem>),
    OpenUri(Vec<StorageItem>),
    Reopen,
    Background,
    Other(i32),
}

impl ActivationEvent {
    pub fn items(&self) -> &[StorageItem] {
        match self {
            Self::Files(items) | Self::OpenUri(items) => items,
            _ => &[],
        }
    }

    pub(crate) fn from_abi(arguments: &mut sys::ActivationArgs) -> Result<Self> {
        let items = StorageItem::from_abi_list(std::mem::take(&mut arguments.items))?;
        Ok(match arguments.kind {
            10 => Self::Files(items),
            20 => Self::OpenUri(items),
            30 => Self::Reopen,
            40 => Self::Background,
            kind => Self::Other(kind),
        })
    }
}

/// A pending picker operation.
///
/// Like [`crate::AsyncOperation`] it is executor-neutral: it stores its single
/// completion under a mutex, wakes the registered waker, and never requires a
/// particular runtime. Dropping it while pending cancels the host operation
/// through the same operation registry the rest of the async ABI uses; the
/// completion object stays alive until the host releases it, so a late
/// completion never touches freed memory.
pub struct StoragePickerOperation {
    application: sys::ComPtr<sys::IAvnApplication>,
    operation_id: i64,
    _completion: sys::ComPtr<sys::IAvnStorageCompletion>,
    state: Arc<Mutex<CompletionSlot<PickerOutcome>>>,
}

impl fmt::Debug for StoragePickerOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoragePickerOperation")
            .field("operation_id", &self.operation_id)
            .finish_non_exhaustive()
    }
}

impl Future for StoragePickerOperation {
    type Output = Result<PickerOutcome>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        CompletionSlot::poll(&self.state, context)
    }
}

impl Drop for StoragePickerOperation {
    fn drop(&mut self) {
        if CompletionSlot::is_pending(&self.state) {
            let _ = self.application.cancel_async_operation(self.operation_id);
        }
    }
}

fn utf16(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn apply_common(
    options: &sys::ComPtr<sys::IAvnFilePickerOptions>,
    title: Option<&String>,
    suggested_file_name: Option<&String>,
    start: Option<&StorageLocation>,
) -> Result<()> {
    if let Some(title) = title {
        options.set_title(Some(&utf16(title)))?;
    }
    if let Some(name) = suggested_file_name {
        options.set_suggested_file_name(Some(&utf16(name)))?;
    }
    match start {
        Some(StorageLocation::Path(path)) => {
            options.set_suggested_start_location(Some(&utf16(&path.to_string_lossy())))?;
        }
        Some(StorageLocation::Uri(uri)) => {
            options.set_suggested_start_location(Some(&utf16(uri)))?;
        }
        Some(StorageLocation::WellKnown(folder)) => {
            options.set_suggested_start_well_known_folder(*folder as i32)?;
        }
        None => {}
    }
    Ok(())
}

fn apply_file_types(
    options: &sys::ComPtr<sys::IAvnFilePickerOptions>,
    file_types: &[FileTypeFilter],
    suggested: Option<usize>,
) -> Result<()> {
    for file_type in file_types {
        let index = options.add_file_type(Some(&utf16(&file_type.name)))?;
        for pattern in &file_type.patterns {
            options.add_file_type_pattern(index, &utf16(pattern))?;
        }
        for mime in &file_type.mime_types {
            options.add_file_type_mime_type(index, &utf16(mime))?;
        }
        for uti in &file_type.apple_uniform_type_identifiers {
            options.add_file_type_apple_uniform_type_identifier(index, &utf16(uti))?;
        }
    }
    if let Some(index) = suggested {
        let index = i32::try_from(index).map_err(|_| Error::InvalidAsyncValue)?;
        options.set_suggested_file_type_index(index)?;
    }
    Ok(())
}

fn decode_picker_completion(
    arguments: &mut sys::StorageCompletionArgs,
) -> std::result::Result<PickerOutcome, AsyncFailure> {
    if arguments.hresult < 0 {
        return Err(AsyncFailure {
            hresult: arguments.hresult,
            message: arguments
                .error
                .clone()
                .unwrap_or_else(|| format!("picker failed: 0x{:08X}", arguments.hresult)),
        });
    }
    if arguments.outcome == 1 {
        return Ok(PickerOutcome::Cancelled);
    }
    match StorageItem::from_abi_list(std::mem::take(&mut arguments.items)) {
        Ok(items) => Ok(PickerOutcome::Selected(items)),
        Err(error) => Err(AsyncFailure {
            hresult: sys::E_FAIL,
            message: error.to_string(),
        }),
    }
}

impl AppContext {
    /// Queries the stage 29 desktop file integration capability.
    fn desktop_files(&self) -> Result<sys::ComPtr<sys::IAvnApplication3>> {
        Ok(self.application().desktop_files()?)
    }

    /// Reports which pickers this window's top-level supports.
    pub fn storage_capabilities(&self, window: &Window) -> Result<StorageCapabilities> {
        let bits = self
            .desktop_files()?
            .storage_capabilities(Some(&window.raw))?;
        Ok(StorageCapabilities::from_bits(bits))
    }

    /// Opens the file picker, tied to `window`.
    pub fn open_file_picker(
        &self,
        window: &Window,
        options: &OpenFilePickerOptions,
    ) -> Result<StoragePickerOperation> {
        let desktop = self.desktop_files()?;
        let abi = desktop.create_picker_options()?;
        apply_common(
            &abi,
            options.title.as_ref(),
            options.suggested_file_name.as_ref(),
            options.suggested_start_location.as_ref(),
        )?;
        abi.set_allow_multiple(options.allow_multiple)?;
        apply_file_types(&abi, &options.file_types, options.suggested_file_type)?;
        self.start_picker(window, &abi, move |window, options, completion| {
            desktop.start_open_file_picker(Some(window), Some(options), Some(completion))
        })
    }

    /// Opens the folder picker, tied to `window`.
    pub fn open_folder_picker(
        &self,
        window: &Window,
        options: &FolderPickerOptions,
    ) -> Result<StoragePickerOperation> {
        let desktop = self.desktop_files()?;
        let abi = desktop.create_picker_options()?;
        apply_common(
            &abi,
            options.title.as_ref(),
            None,
            options.suggested_start_location.as_ref(),
        )?;
        abi.set_allow_multiple(options.allow_multiple)?;
        self.start_picker(window, &abi, move |window, options, completion| {
            desktop.start_open_folder_picker(Some(window), Some(options), Some(completion))
        })
    }

    /// Opens the save/export picker, tied to `window`.
    ///
    /// The result carries the picked name, URI and optional local path; it never
    /// forces managed stream IO, so writing stays entirely in Rust.
    pub fn save_file_picker(
        &self,
        window: &Window,
        options: &SaveFilePickerOptions,
    ) -> Result<StoragePickerOperation> {
        let desktop = self.desktop_files()?;
        let abi = desktop.create_picker_options()?;
        apply_common(
            &abi,
            options.title.as_ref(),
            options.suggested_file_name.as_ref(),
            options.suggested_start_location.as_ref(),
        )?;
        if let Some(extension) = &options.default_extension {
            abi.set_default_extension(Some(&utf16(extension)))?;
        }
        abi.set_show_overwrite_prompt(match options.show_overwrite_prompt {
            None => -1,
            Some(false) => 0,
            Some(true) => 1,
        })?;
        apply_file_types(&abi, &options.file_types, options.suggested_file_type)?;
        self.start_picker(window, &abi, move |window, options, completion| {
            desktop.start_save_file_picker(Some(window), Some(options), Some(completion))
        })
    }

    /// The verbatim startup arguments this process passed to the host.
    pub fn startup_arguments(&self) -> Result<Vec<String>> {
        let desktop = self.desktop_files()?;
        let count = desktop.startup_argument_count()?;
        let mut arguments = Vec::with_capacity(count.max(0) as usize);
        for index in 0..count {
            arguments.push(desktop.startup_argument(index)?.unwrap_or_default());
        }
        Ok(arguments)
    }

    /// The normalized, de-duplicated, order-preserving open-with items derived
    /// from the startup arguments.
    pub fn activation_items(&self) -> Result<Vec<StorageItem>> {
        StorageItem::from_abi_list(self.desktop_files()?.activation_items()?)
    }

    /// Subscribes to incoming file/folder drags on a control or window.
    ///
    /// `accepted_effects` is the conservative mask the host uses to answer the
    /// platform synchronously; Rust is never called inside the drag loop.
    pub fn subscribe_file_drop(
        &self,
        target: &impl AsControl,
        accepted_effects: DragDropEffects,
        callback: impl FnMut(FileDropEvent) + Send + 'static,
    ) -> Result<EventSubscription> {
        let target = target.as_control()?;
        let desktop = self.desktop_files()?;
        let mut callback = callback;
        let handler = sys::file_drop_handler(move |arguments| {
            let event =
                FileDropEvent::from_abi(arguments).map_err(|_| sys::Error(sys::E_INVALIDARG))?;
            callback(event);
            Ok(())
        });
        let subscription_id =
            desktop.subscribe_file_drop(Some(&target), accepted_effects.bits(), Some(&handler))?;
        Ok(EventSubscription::new(move || {
            desktop.unsubscribe_file_drop(subscription_id)
        }))
    }

    /// Subscribes to post-startup activation.
    pub fn subscribe_activation(
        &self,
        callback: impl FnMut(ActivationEvent) + Send + 'static,
    ) -> Result<EventSubscription> {
        let desktop = self.desktop_files()?;
        let mut callback = callback;
        let handler = sys::activation_handler(move |arguments| {
            let event =
                ActivationEvent::from_abi(arguments).map_err(|_| sys::Error(sys::E_INVALIDARG))?;
            callback(event);
            Ok(())
        });
        let subscription_id = desktop.advise_activation(Some(&handler))?;
        Ok(EventSubscription::new(move || {
            desktop.unadvise_activation(subscription_id)
        }))
    }

    fn start_picker(
        &self,
        window: &Window,
        options: &sys::ComPtr<sys::IAvnFilePickerOptions>,
        start: impl FnOnce(
            &sys::ComPtr<sys::IAvnWindow>,
            &sys::ComPtr<sys::IAvnFilePickerOptions>,
            &sys::ComPtr<sys::IAvnStorageCompletion>,
        ) -> sys::Result<i64>,
    ) -> Result<StoragePickerOperation> {
        let state = Arc::new(Mutex::new(CompletionSlot::default()));
        let completion_state = state.clone();
        let completion = sys::storage_completion(move |arguments| {
            CompletionSlot::publish(&completion_state, decode_picker_completion(arguments))
        });
        let operation_id = start(&window.raw, options, &completion)?;
        Ok(StoragePickerOperation {
            application: self.application().clone(),
            operation_id,
            _completion: completion,
            state,
        })
    }
}

impl AppScope {
    /// Subscribes to incoming file drags and keeps the subscription alive for
    /// the whole application scope.
    pub fn on_file_drop(
        &self,
        target: &impl AsControl,
        accepted_effects: DragDropEffects,
        callback: impl FnMut(FileDropEvent) + Send + 'static,
    ) -> Result<()> {
        let subscription = self.subscribe_file_drop(target, accepted_effects, callback)?;
        self.retain_subscription(subscription);
        Ok(())
    }

    /// Subscribes to post-startup activation for the whole application scope.
    pub fn on_activation(
        &self,
        callback: impl FnMut(ActivationEvent) + Send + 'static,
    ) -> Result<()> {
        let subscription = self.subscribe_activation(callback)?;
        self.retain_subscription(subscription);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: i32, uri: &str, local: Option<&str>) -> sys::StorageItemData {
        sys::StorageItemData {
            kind,
            name: Some("name".to_string()),
            uri: Some(uri.to_string()),
            local_path: local.map(str::to_string),
        }
    }

    #[test]
    fn local_and_non_local_items_round_trip() {
        let local = StorageItem::from_abi(item(0, "file:///tmp/a.txt", Some("/tmp/a.txt")))
            .expect("local item");
        assert_eq!(StorageItemKind::File, local.kind());
        assert_eq!("file:///tmp/a.txt", local.uri());
        assert_eq!(Some(Path::new("/tmp/a.txt")), local.local_path());

        let remote = StorageItem::from_abi(item(1, "content://media/documents/7", None))
            .expect("non-local item");
        assert!(remote.is_folder());
        assert_eq!("content://media/documents/7", remote.uri());
        assert_eq!(None, remote.local_path());
    }

    #[test]
    fn an_item_without_a_uri_is_rejected() {
        let data = sys::StorageItemData {
            kind: 0,
            name: Some("name".to_string()),
            uri: None,
            local_path: None,
        };
        assert!(matches!(
            StorageItem::from_abi(data),
            Err(Error::InvalidAsyncValue)
        ));
    }

    #[test]
    fn an_unknown_item_kind_is_rejected() {
        assert!(matches!(
            StorageItem::from_abi(item(7, "file:///tmp/a", None)),
            Err(Error::InvalidEnumValue(7))
        ));
    }

    #[test]
    fn a_cancelled_picker_is_not_an_error() {
        let mut arguments = sys::StorageCompletionArgs {
            operation_id: 1,
            hresult: 0,
            outcome: 1,
            items: Vec::new(),
            error: None,
        };
        let outcome = decode_picker_completion(&mut arguments).expect("cancel is not an error");
        assert!(outcome.is_cancelled());
        assert!(outcome.items().is_empty());
    }

    #[test]
    fn a_failed_picker_reports_the_host_error() {
        let mut arguments = sys::StorageCompletionArgs {
            operation_id: 1,
            hresult: sys::E_FAIL,
            outcome: 0,
            items: Vec::new(),
            error: Some("no storage provider".to_string()),
        };
        let failure = decode_picker_completion(&mut arguments).expect_err("failure");
        assert_eq!(sys::E_FAIL, failure.hresult);
        assert_eq!("no storage provider", failure.message);
    }

    #[test]
    fn a_successful_picker_carries_ordered_items() {
        let mut arguments = sys::StorageCompletionArgs {
            operation_id: 1,
            hresult: 0,
            outcome: 0,
            items: vec![
                item(0, "file:///tmp/a.txt", Some("/tmp/a.txt")),
                item(0, "file:///tmp/b.txt", Some("/tmp/b.txt")),
            ],
            error: None,
        };
        let outcome = decode_picker_completion(&mut arguments).expect("selection");
        let uris: Vec<_> = outcome.items().iter().map(StorageItem::uri).collect();
        assert_eq!(vec!["file:///tmp/a.txt", "file:///tmp/b.txt"], uris);
    }

    #[test]
    fn drop_events_decode_by_kind() {
        for (kind, expect_items) in [(0, true), (1, true), (2, false), (3, true)] {
            let mut arguments = sys::FileDropArgs {
                subscription_id: 3,
                kind,
                allowed_effects: 3,
                effective_effects: 1,
                items: vec![item(0, "file:///tmp/a.txt", Some("/tmp/a.txt"))],
            };
            let event = FileDropEvent::from_abi(&mut arguments).expect("event");
            assert_eq!(expect_items, !event.items().is_empty());
            assert_eq!(expect_items, event.accepted());
        }
    }

    #[test]
    fn an_unknown_drop_kind_is_rejected() {
        let mut arguments = sys::FileDropArgs {
            subscription_id: 3,
            kind: 9,
            allowed_effects: 0,
            effective_effects: 0,
            items: Vec::new(),
        };
        assert!(matches!(
            FileDropEvent::from_abi(&mut arguments),
            Err(Error::InvalidEnumValue(9))
        ));
    }

    #[test]
    fn activation_events_decode_by_kind() {
        let mut files = sys::ActivationArgs {
            kind: 10,
            items: vec![item(0, "file:///tmp/a.txt", Some("/tmp/a.txt"))],
        };
        assert!(matches!(
            ActivationEvent::from_abi(&mut files),
            Ok(ActivationEvent::Files(items)) if items.len() == 1));

        let mut uri = sys::ActivationArgs {
            kind: 20,
            items: vec![item(0, "myapp://open/7", None)],
        };
        assert!(matches!(
            ActivationEvent::from_abi(&mut uri),
            Ok(ActivationEvent::OpenUri(items)) if items[0].local_path().is_none()));

        let mut reopen = sys::ActivationArgs {
            kind: 30,
            items: Vec::new(),
        };
        assert_eq!(
            ActivationEvent::Reopen,
            ActivationEvent::from_abi(&mut reopen).expect("reopen")
        );

        let mut unknown = sys::ActivationArgs {
            kind: 99,
            items: Vec::new(),
        };
        assert_eq!(
            ActivationEvent::Other(99),
            ActivationEvent::from_abi(&mut unknown).expect("unknown kinds stay forward compatible")
        );
    }

    #[test]
    fn effect_masks_intersect_like_the_managed_side() {
        let allowed = DragDropEffects::COPY | DragDropEffects::MOVE;
        assert!(allowed.contains(DragDropEffects::COPY));
        assert!(!allowed.contains(DragDropEffects::LINK));
        assert_eq!(
            DragDropEffects::COPY,
            allowed & (DragDropEffects::COPY | DragDropEffects::LINK)
        );
        assert!((allowed & DragDropEffects::LINK).is_empty());
        assert_eq!(
            DragDropEffects::COPY,
            DragDropEffects::from_bits_truncate(0b1000_0001)
        );
    }

    #[test]
    fn file_type_filters_build_cross_platform_metadata() {
        let filter = FileTypeFilter::new("Log files")
            .with_extension(".log")
            .with_pattern("*.txt")
            .with_mime_type("text/plain")
            .with_apple_uniform_type_identifier("public.plain-text");
        assert_eq!(vec!["*.log", "*.txt"], filter.patterns);
        assert_eq!(vec!["text/plain"], filter.mime_types);
        assert_eq!(
            vec!["public.plain-text"],
            filter.apple_uniform_type_identifiers
        );
    }
}

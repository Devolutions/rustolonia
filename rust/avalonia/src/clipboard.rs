//! Stage 31 clipboard commands for Rust consumers.
//!
//! Everything here goes through Avalonia's own clipboard abstraction over the
//! separately versioned `IAvnApplication4` capability. Plain-text write and
//! read still use the frozen `IAvnApplication` methods; this module adds
//! clearing, multi-format writes and reading file entries back as the same
//! immutable [`StorageItem`] snapshots stage 29 produces.
//!
//! Every operation is asynchronous. A clipboard read can block for as long as
//! the owning application takes to render the requested format, so a synchronous
//! clipboard API would be a UI-thread hazard; these futures are executor-neutral
//! and cancel their host operation when dropped, exactly like the stage 29
//! pickers.

use crate::async_runtime::{AsyncFailure, CompletionSlot};
use crate::storage::StorageItem;
use crate::{AppContext, AsyncOperation, Error, Result, Window};
use avalonia_sys as sys;
use std::fmt;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// What the window's top-level clipboard supports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClipboardCapabilities {
    bits: i32,
}

impl ClipboardCapabilities {
    const AVAILABLE: i32 = 1;
    const FILES: i32 = 2;

    pub(crate) fn from_bits(bits: i32) -> Self {
        Self { bits }
    }

    /// Whether the top-level has a clipboard at all.
    pub fn is_available(self) -> bool {
        self.bits & Self::AVAILABLE != 0
    }

    /// Whether file entries can be resolved, which additionally requires a
    /// storage provider on the same top-level.
    pub fn supports_files(self) -> bool {
        self.bits & Self::FILES != 0
    }
}

/// One clipboard payload: optional text plus optional file entries.
///
/// File entries are storage URIs (or absolute local paths), which is exactly
/// what a stage 29 [`StorageItem`] guarantees, so a picked, dropped or
/// recent-file item can be copied without any extra plumbing. An entry the
/// platform cannot resolve is dropped by the host rather than failing the copy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClipboardData {
    text: Option<String>,
    files: Vec<String>,
}

impl ClipboardData {
    pub fn new() -> Self {
        Self::default()
    }

    /// A text-only payload.
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            text: Some(value.into()),
            files: Vec::new(),
        }
    }

    pub fn with_text(mut self, value: impl Into<String>) -> Self {
        self.text = Some(value.into());
        self
    }

    /// Adds one file entry from a storage URI or an absolute local path.
    pub fn with_file_uri(mut self, value: impl Into<String>) -> Self {
        self.files.push(value.into());
        self
    }

    /// Adds one file entry from a [`StorageItem`], using its guaranteed URI.
    pub fn with_item(mut self, item: &StorageItem) -> Self {
        self.files.push(item.uri().to_string());
        self
    }

    /// Adds one file entry from a local path.
    pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
        self.files
            .push(path.as_ref().to_string_lossy().into_owned());
        self
    }

    pub fn text_value(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn file_uris(&self) -> &[String] {
        &self.files
    }

    /// Whether this payload would write nothing. An empty write is rejected by
    /// the host with `E_INVALIDARG`, because "write nothing" is
    /// [`AppContext::clipboard_clear`], not a write.
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.files.is_empty()
    }
}

/// A pending clipboard file read.
///
/// Executor-neutral like every other host-started operation: it stores its one
/// completion under a mutex, wakes the registered waker, and cancels the host
/// operation if dropped while still pending.
pub struct ClipboardFilesOperation {
    application: sys::ComPtr<sys::IAvnApplication>,
    operation_id: i64,
    _completion: sys::ComPtr<sys::IAvnStorageCompletion>,
    state: Arc<Mutex<CompletionSlot<Vec<StorageItem>>>>,
}

impl fmt::Debug for ClipboardFilesOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClipboardFilesOperation")
            .field("operation_id", &self.operation_id)
            .finish_non_exhaustive()
    }
}

impl Future for ClipboardFilesOperation {
    type Output = Result<Vec<StorageItem>>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        CompletionSlot::poll(&self.state, context)
    }
}

impl Drop for ClipboardFilesOperation {
    fn drop(&mut self) {
        if CompletionSlot::is_pending(&self.state) {
            let _ = self.application.cancel_async_operation(self.operation_id);
        }
    }
}

fn utf16(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn decode_files(
    arguments: &mut sys::StorageCompletionArgs,
) -> std::result::Result<Vec<StorageItem>, AsyncFailure> {
    if arguments.hresult < 0 {
        return Err(AsyncFailure {
            hresult: arguments.hresult,
            message: arguments
                .error
                .clone()
                .unwrap_or_else(|| format!("clipboard read failed: 0x{:08X}", arguments.hresult)),
        });
    }
    StorageItem::from_abi_list(std::mem::take(&mut arguments.items)).map_err(|error| AsyncFailure {
        hresult: sys::E_FAIL,
        message: error.to_string(),
    })
}

impl AppContext {
    /// Queries the stage 31 clipboard command capability.
    fn clipboard_capability(&self) -> Result<sys::ComPtr<sys::IAvnApplication4>> {
        Ok(self.application().clipboard()?)
    }

    /// Reports what this window's clipboard supports.
    pub fn clipboard_capabilities(&self, window: &Window) -> Result<ClipboardCapabilities> {
        let bits = self
            .clipboard_capability()?
            .clipboard_capabilities(Some(&window.raw))?;
        Ok(ClipboardCapabilities::from_bits(bits))
    }

    /// Writes text and/or file entries to the clipboard.
    pub fn clipboard_write(
        &self,
        window: &Window,
        data: &ClipboardData,
    ) -> Result<AsyncOperation<()>> {
        if data.is_empty() {
            return Err(Error::InvalidAsyncValue);
        }
        let capability = self.clipboard_capability()?;
        let payload = capability.create_clipboard_data()?;
        if let Some(text) = data.text_value() {
            payload.set_text(Some(&utf16(text)))?;
        }
        for uri in data.file_uris() {
            payload.add_file_uri(Some(&utf16(uri)))?;
        }
        let window = window.raw.clone();
        let application = self.application().clone();
        AsyncOperation::start(
            application,
            move |completion| {
                capability.start_clipboard_write(Some(&window), Some(&payload), Some(completion))
            },
            crate::async_runtime::decode_none,
        )
    }

    /// Clears the clipboard.
    pub fn clipboard_clear(&self, window: &Window) -> Result<AsyncOperation<()>> {
        let capability = self.clipboard_capability()?;
        let window = window.raw.clone();
        let application = self.application().clone();
        AsyncOperation::start(
            application,
            move |completion| capability.start_clipboard_clear(Some(&window), Some(completion)),
            crate::async_runtime::decode_none,
        )
    }

    /// Reads file entries from the clipboard.
    ///
    /// A clipboard that carries no files completes successfully with an empty
    /// list; only a real platform failure is an error.
    pub fn clipboard_read_files(&self, window: &Window) -> Result<ClipboardFilesOperation> {
        let capability = self.clipboard_capability()?;
        let state = Arc::new(Mutex::new(CompletionSlot::default()));
        let completion_state = state.clone();
        let completion = sys::storage_completion(move |arguments| {
            CompletionSlot::publish(&completion_state, decode_files(arguments))
        });
        let operation_id =
            capability.start_clipboard_read_files(Some(&window.raw), Some(&completion))?;
        Ok(ClipboardFilesOperation {
            application: self.application().clone(),
            operation_id,
            _completion: completion,
            state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_payload_is_reported_as_empty() {
        assert!(ClipboardData::new().is_empty());
        assert!(!ClipboardData::text("hello").is_empty());
        assert!(!ClipboardData::new()
            .with_file_uri("file:///tmp/a.txt")
            .is_empty());
    }

    #[test]
    fn a_payload_keeps_text_and_file_order() {
        let data = ClipboardData::new()
            .with_text("copied")
            .with_file_uri("file:///tmp/a.txt")
            .with_file_uri("file:///tmp/b.txt");
        assert_eq!(Some("copied"), data.text_value());
        assert_eq!(
            &[
                "file:///tmp/a.txt".to_string(),
                "file:///tmp/b.txt".to_string()
            ],
            data.file_uris()
        );
    }

    #[test]
    fn capabilities_decode_independent_flags() {
        assert!(!ClipboardCapabilities::from_bits(0).is_available());
        assert!(ClipboardCapabilities::from_bits(1).is_available());
        assert!(!ClipboardCapabilities::from_bits(1).supports_files());
        assert!(ClipboardCapabilities::from_bits(3).supports_files());
    }
}

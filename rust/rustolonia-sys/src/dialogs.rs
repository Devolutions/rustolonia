//! Modal dialog activation.
//!
//! `IAvnApplication::StartShowDialog` completes through the shared async
//! operation registry (`IAvnAsyncCompletion`), so a dialog completes exactly
//! once and a dropped `AsyncOperation` cancels it by closing the dialog.

use crate::application::IAvnApplication;
use crate::async_completion::IAvnAsyncCompletion;
use crate::com::{ComInterface, ComPtr};
use crate::generated::IAvnWindow;
use crate::hresult::{self, Result};
use std::ptr;

fn optional_raw<T: ComInterface>(value: Option<&ComPtr<T>>) -> *mut T {
    value.map_or(ptr::null_mut(), ComPtr::as_raw)
}

impl ComPtr<IAvnApplication> {
    /// Shows `dialog` modally over `owner`, completing through the shared async
    /// operation registry. The completion's string value is the dialog result the
    /// host converted (`None` when the dialog returned no result).
    pub fn start_show_dialog(
        &self,
        owner: Option<&ComPtr<IAvnWindow>>,
        dialog: Option<&ComPtr<IAvnWindow>>,
        completion: Option<&ComPtr<IAvnAsyncCompletion>>,
    ) -> Result<i64> {
        unsafe {
            let mut operation_id = 0;
            let hr = (self.vtbl().start_show_dialog)(
                self.as_raw(),
                optional_raw(owner),
                optional_raw(dialog),
                optional_raw(completion),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }
}

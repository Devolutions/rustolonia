//! Stage 32 modal dialog activation ABI.
//!
//! `IAvnApplication5` is a separately versioned capability queried from
//! `IAvnApplication`; nothing here is added to an already published vtable. The
//! dialog completion rides the shared async operation registry
//! (`IAvnAsyncCompletion`), so a dialog completes exactly once and a dropped
//! `AsyncOperation` cancels it by closing the dialog.

use crate::application::IAvnApplication;
use crate::async_completion::IAvnAsyncCompletion;
use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::IAvnWindow;
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::ptr;

const IAVN_APPLICATION5_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x70],
};

#[repr(C)]
struct IAvnApplication5Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    start_show_dialog: unsafe extern "system" fn(
        *mut IAvnApplication5,
        *mut IAvnWindow,
        *mut IAvnWindow,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
}

#[repr(C)]
pub struct IAvnApplication5 {
    vtbl: *const IAvnApplication5Vtbl,
}

unsafe impl ComInterface for IAvnApplication5 {
    const IID: Guid = IAVN_APPLICATION5_IID;
}

fn optional_raw<T: ComInterface>(value: Option<&ComPtr<T>>) -> *mut T {
    value.map_or(ptr::null_mut(), ComPtr::as_raw)
}

impl ComPtr<IAvnApplication5> {
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
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().start_show_dialog)(
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

impl ComPtr<IAvnApplication> {
    /// Queries the separately versioned modal dialog capability.
    pub fn dialogs(&self) -> Result<ComPtr<IAvnApplication5>> {
        self.query_interface::<IAvnApplication5>()
    }
}

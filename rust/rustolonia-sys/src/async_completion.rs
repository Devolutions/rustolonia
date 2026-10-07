use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::guid::Guid;
use crate::hresult::Result;
use std::ffi::c_void;

const IAVN_ASYNC_COMPLETION_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x17],
};

#[derive(Debug)]
pub struct AsyncCompletionArgs {
    pub operation_id: i64,
    pub hresult: i32,
    pub value_kind: i32,
    pub integer_value: i64,
    pub double_value: f64,
    pub string_value: Option<String>,
    pub error: Option<String>,
}

#[repr(C)]
struct IAvnAsyncCompletionVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    complete: unsafe extern "system" fn(
        *mut IAvnAsyncCompletion,
        i64,
        i32,
        i32,
        i64,
        f64,
        *const u16,
        *const u16,
    ) -> i32,
}

#[repr(C)]
pub struct IAvnAsyncCompletion {
    vtbl: *const IAvnAsyncCompletionVtbl,
}

unsafe impl ComInterface for IAvnAsyncCompletion {
    const IID: Guid = IAVN_ASYNC_COMPLETION_IID;
}

static ASYNC_COMPLETION_VTBL: IAvnAsyncCompletionVtbl = IAvnAsyncCompletionVtbl {
    query_interface,
    add_ref,
    release,
    complete,
};

pub fn async_completion(
    callback: impl FnMut(&mut AsyncCompletionArgs) -> Result<()> + Send + 'static,
) -> ComPtr<IAvnAsyncCompletion> {
    crate::event_callback::create(
        IAvnAsyncCompletion {
            vtbl: &ASYNC_COMPLETION_VTBL,
        },
        callback,
    )
}

unsafe extern "system" fn query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnAsyncCompletion, AsyncCompletionArgs>(
        this, iid, result,
    )
}

unsafe extern "system" fn add_ref(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnAsyncCompletion, AsyncCompletionArgs>(this)
}

unsafe extern "system" fn release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnAsyncCompletion, AsyncCompletionArgs>(this)
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn complete(
    this: *mut IAvnAsyncCompletion,
    operation_id: i64,
    hresult: i32,
    value_kind: i32,
    integer_value: i64,
    double_value: f64,
    string_value: *const u16,
    error: *const u16,
) -> i32 {
    let mut arguments = AsyncCompletionArgs {
        operation_id,
        hresult,
        value_kind,
        integer_value,
        double_value,
        string_value: crate::clone_utf16(string_value),
        error: crate::clone_utf16(error),
    };
    crate::event_callback::invoke::<IAvnAsyncCompletion, AsyncCompletionArgs>(this, &mut arguments)
}

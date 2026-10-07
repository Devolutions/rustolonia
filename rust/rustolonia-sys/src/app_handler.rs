use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Mutex;

const IAVN_APP_HANDLER_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x13],
};

#[repr(C)]
struct IAvnAppHandlerVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    on_started: unsafe extern "system" fn(*mut IAvnAppHandler) -> i32,
}

#[repr(C)]
pub struct IAvnAppHandler {
    vtbl: *const IAvnAppHandlerVtbl,
}

unsafe impl ComInterface for IAvnAppHandler {
    const IID: Guid = IAVN_APP_HANDLER_IID;
}

type StartedCallback = Box<dyn FnOnce() -> Result<()> + Send>;

#[repr(C)]
struct AppHandlerObject {
    interface: IAvnAppHandler,
    ref_count: AtomicU32,
    callback: Mutex<Option<StartedCallback>>,
}

static APP_HANDLER_VTBL: IAvnAppHandlerVtbl = IAvnAppHandlerVtbl {
    query_interface,
    add_ref,
    release,
    on_started,
};

pub fn app_handler(
    callback: impl FnOnce() -> Result<()> + Send + 'static,
) -> ComPtr<IAvnAppHandler> {
    let object = Box::new(AppHandlerObject {
        interface: IAvnAppHandler {
            vtbl: &APP_HANDLER_VTBL,
        },
        ref_count: AtomicU32::new(1),
        callback: Mutex::new(Some(Box::new(callback))),
    });
    unsafe {
        ComPtr::from_raw(Box::into_raw(object).cast())
            .expect("Box allocation cannot produce a null pointer")
    }
}

unsafe extern "system" fn query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    *result = ptr::null_mut();
    if *iid != Guid::IUNKNOWN && *iid != IAVN_APP_HANDLER_IID {
        return hresult::E_NOINTERFACE;
    }
    add_ref(this);
    *result = this.cast();
    hresult::S_OK
}

unsafe extern "system" fn add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<AppHandlerObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<AppHandlerObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn on_started(this: *mut IAvnAppHandler) -> i32 {
    let object = this.cast::<AppHandlerObject>();
    match catch_unwind(AssertUnwindSafe(|| {
        let callback = (*object)
            .callback
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?
            .take()
            .ok_or(hresult::Error(hresult::E_FAIL))?;
        callback()
    })) {
        Ok(Ok(())) => hresult::S_OK,
        Ok(Err(error)) => error.0,
        Err(_) => hresult::E_FAIL,
    }
}

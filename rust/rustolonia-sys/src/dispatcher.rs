use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::guid::Guid;
use crate::hresult::{self, Error, Result};
use std::ffi::c_void;

const IAVN_DISPATCHER_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x14],
};

const IAVN_ACTION_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x15],
};

#[repr(C)]
struct IAvnActionVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    invoke: unsafe extern "system" fn(*mut IAvnAction) -> i32,
}

#[repr(C)]
pub struct IAvnAction {
    vtbl: *const IAvnActionVtbl,
}

unsafe impl ComInterface for IAvnAction {
    const IID: Guid = IAVN_ACTION_IID;
}

static ACTION_VTBL: IAvnActionVtbl = IAvnActionVtbl {
    query_interface: action_query_interface,
    add_ref: action_add_ref,
    release: action_release,
    invoke: action_invoke,
};

pub fn action(callback: impl FnOnce() -> Result<()> + Send + 'static) -> ComPtr<IAvnAction> {
    let mut callback = Some(callback);
    crate::event_callback::create::<IAvnAction, ()>(IAvnAction { vtbl: &ACTION_VTBL }, move |_| {
        callback.take().ok_or(Error(hresult::E_FAIL))?()
    })
}

unsafe extern "system" fn action_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnAction, ()>(this, iid, result)
}

unsafe extern "system" fn action_add_ref(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnAction, ()>(this)
}

unsafe extern "system" fn action_release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnAction, ()>(this)
}

unsafe extern "system" fn action_invoke(this: *mut IAvnAction) -> i32 {
    crate::event_callback::invoke::<IAvnAction, ()>(this, &mut ())
}

#[repr(C)]
struct IAvnDispatcherVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    check_access: unsafe extern "system" fn(*mut IAvnDispatcher, *mut i32) -> i32,
    post: unsafe extern "system" fn(*mut IAvnDispatcher, *mut IAvnAction) -> i32,
}

#[repr(C)]
pub struct IAvnDispatcher {
    vtbl: *const IAvnDispatcherVtbl,
}

unsafe impl ComInterface for IAvnDispatcher {
    const IID: Guid = IAVN_DISPATCHER_IID;
}

impl ComPtr<IAvnDispatcher> {
    pub fn check_access(&self) -> Result<bool> {
        unsafe {
            let mut value = 0;
            let hr =
                ((*self.as_raw()).vtbl.as_ref().unwrap().check_access)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value != 0)
        }
    }

    pub fn post(&self, action: &ComPtr<IAvnAction>) -> Result<()> {
        unsafe {
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().post)(self.as_raw(), action.as_raw());
            hresult::check(hr)
        }
    }
}

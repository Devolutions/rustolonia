use crate::{ComInterface, ComPtr, Guid, IUnknown, Result};
use std::ffi::c_void;

#[repr(C)]
struct ApplicationVtbl {
    query: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set: unsafe extern "system" fn(*mut IAvnApplication11, *mut IAvnTerminationHandler) -> i32,
    complete: unsafe extern "system" fn(*mut IAvnApplication11, i64, i32) -> i32,
}
#[repr(C)]
pub struct IAvnApplication11 {
    vtbl: *const ApplicationVtbl,
}
unsafe impl ComInterface for IAvnApplication11 {
    const IID: Guid = Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA4],
    };
}
impl ComPtr<IAvnApplication11> {
    pub fn set_termination_handler(
        &self,
        handler: Option<&ComPtr<IAvnTerminationHandler>>,
    ) -> Result<()> {
        unsafe {
            crate::hresult::check(((*(*self.as_raw()).vtbl).set)(
                self.as_raw(),
                handler.map_or(std::ptr::null_mut(), |handler| handler.as_raw()),
            ))
        }
    }
    pub fn complete_termination(&self, token: i64, accept: bool) -> Result<()> {
        unsafe {
            crate::hresult::check(((*(*self.as_raw()).vtbl).complete)(
                self.as_raw(),
                token,
                accept as i32,
            ))
        }
    }
}
#[repr(C)]
struct HandlerVtbl {
    query: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    requested: unsafe extern "system" fn(*mut IAvnTerminationHandler, i64, i32) -> i32,
}
#[repr(C)]
pub struct IAvnTerminationHandler {
    vtbl: *const HandlerVtbl,
}
unsafe impl ComInterface for IAvnTerminationHandler {
    const IID: Guid = Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA5],
    };
}
#[derive(Clone, Copy, Debug)]
pub struct TerminationRequest {
    pub token: i64,
    pub flags: i32,
}
unsafe extern "system" fn query(
    this: *mut IUnknown,
    iid: *const Guid,
    out: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnTerminationHandler, TerminationRequest>(
        this, iid, out,
    )
}
unsafe extern "system" fn add(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnTerminationHandler, TerminationRequest>(this)
}
unsafe extern "system" fn release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnTerminationHandler, TerminationRequest>(this)
}
unsafe extern "system" fn requested(
    this: *mut IAvnTerminationHandler,
    token: i64,
    flags: i32,
) -> i32 {
    crate::event_callback::invoke::<IAvnTerminationHandler, TerminationRequest>(
        this,
        &mut TerminationRequest { token, flags },
    )
}
static HANDLER: HandlerVtbl = HandlerVtbl {
    query,
    add,
    release,
    requested,
};
pub fn termination_handler(
    mut callback: impl FnMut(TerminationRequest) -> Result<()> + Send + 'static,
) -> ComPtr<IAvnTerminationHandler> {
    crate::event_callback::create(
        IAvnTerminationHandler { vtbl: &HANDLER },
        move |request: &mut TerminationRequest| callback(*request),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn additive_vtable_shapes() {
        assert_eq!(
            std::mem::size_of::<ApplicationVtbl>(),
            5 * std::mem::size_of::<usize>()
        );
        assert_eq!(
            std::mem::size_of::<HandlerVtbl>(),
            4 * std::mem::size_of::<usize>()
        );
    }
}

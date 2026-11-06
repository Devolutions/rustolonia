use crate::{ComInterface, ComPtr, Guid, IAvnApplication, IAvnWindow, IUnknown, Result};
use std::ffi::c_void;

impl ComPtr<IAvnApplication> {
    pub fn set_native_popup_gate(
        &self,
        window: &ComPtr<IAvnWindow>,
        handler: Option<&ComPtr<IAvnNativePopupHandler>>,
    ) -> Result<()> {
        unsafe {
            crate::hresult::check((self.vtbl().set_native_popup_gate)(
                self.as_raw(),
                window.as_raw(),
                handler.map_or(std::ptr::null_mut(), |handler| handler.as_raw()),
            ))
        }
    }
    pub fn acknowledge_native_popup(&self, token: i64) -> Result<()> {
        unsafe {
            crate::hresult::check((self.vtbl().acknowledge_native_popup)(self.as_raw(), token))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum NativePopupEvent {
    Preparing { token: i64, window_number: i32 },
    Closed { token: i64 },
}
#[repr(C)]
struct HandlerVtbl {
    query: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    preparing: unsafe extern "system" fn(*mut IAvnNativePopupHandler, i64, i32) -> i32,
    closed: unsafe extern "system" fn(*mut IAvnNativePopupHandler, i64) -> i32,
}
#[repr(C)]
pub struct IAvnNativePopupHandler {
    vtbl: *const HandlerVtbl,
}
unsafe impl ComInterface for IAvnNativePopupHandler {
    const IID: Guid = Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA3],
    };
}
unsafe extern "system" fn query(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnNativePopupHandler, NativePopupEvent>(
        this, iid, result,
    )
}
unsafe extern "system" fn add(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnNativePopupHandler, NativePopupEvent>(this)
}
unsafe extern "system" fn release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnNativePopupHandler, NativePopupEvent>(this)
}
unsafe extern "system" fn preparing(
    this: *mut IAvnNativePopupHandler,
    token: i64,
    window_number: i32,
) -> i32 {
    crate::event_callback::invoke::<IAvnNativePopupHandler, NativePopupEvent>(
        this,
        &mut NativePopupEvent::Preparing {
            token,
            window_number,
        },
    )
}
unsafe extern "system" fn closed(this: *mut IAvnNativePopupHandler, token: i64) -> i32 {
    crate::event_callback::invoke::<IAvnNativePopupHandler, NativePopupEvent>(
        this,
        &mut NativePopupEvent::Closed { token },
    )
}
static HANDLER: HandlerVtbl = HandlerVtbl {
    query,
    add,
    release,
    preparing,
    closed,
};
pub fn native_popup_handler(
    mut callback: impl FnMut(NativePopupEvent) -> Result<()> + Send + 'static,
) -> ComPtr<IAvnNativePopupHandler> {
    crate::event_callback::create(
        IAvnNativePopupHandler { vtbl: &HANDLER },
        move |event: &mut NativePopupEvent| callback(*event),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handler_shape_is_stable() {
        assert_eq!(
            5 * std::mem::size_of::<usize>(),
            std::mem::size_of::<HandlerVtbl>()
        );
        assert_eq!(0xA3, IAvnNativePopupHandler::IID.data4[7]);
    }
}

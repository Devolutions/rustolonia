use crate::{ComInterface, ComPtr, Guid, IAvnApplication, IAvnWindow, IUnknown, Result};
use std::ffi::c_void;

#[cfg(test)]
fn iid(last: u8) -> Guid {
    Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, last],
    }
}

#[repr(C)]
struct ApplicationVtbl {
    query: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set: unsafe extern "system" fn(
        *mut IAvnApplication10,
        *mut IAvnWindow,
        *mut IAvnNativePopupHandler,
    ) -> i32,
    acknowledge: unsafe extern "system" fn(*mut IAvnApplication10, i64) -> i32,
}
#[repr(C)]
pub struct IAvnApplication10 {
    vtbl: *const ApplicationVtbl,
}
unsafe impl ComInterface for IAvnApplication10 {
    const IID: Guid = Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA2],
    };
}
impl ComPtr<IAvnApplication> {
    pub fn popup_gate(&self) -> Result<ComPtr<IAvnApplication10>> {
        self.query_interface()
    }
}
impl ComPtr<IAvnApplication10> {
    pub fn set(
        &self,
        window: &ComPtr<IAvnWindow>,
        handler: Option<&ComPtr<IAvnNativePopupHandler>>,
    ) -> Result<()> {
        unsafe {
            crate::hresult::check(((*(*self.as_raw()).vtbl).set)(
                self.as_raw(),
                window.as_raw(),
                handler.map_or(std::ptr::null_mut(), |handler| handler.as_raw()),
            ))
        }
    }
    pub fn acknowledge(&self, token: i64) -> Result<()> {
        unsafe {
            crate::hresult::check(((*(*self.as_raw()).vtbl).acknowledge)(self.as_raw(), token))
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
    fn stable_capability_shape() {
        assert_eq!(
            5 * std::mem::size_of::<usize>(),
            std::mem::size_of::<ApplicationVtbl>()
        );
        assert_eq!(iid(0xA2), IAvnApplication10::IID);
        assert_eq!(iid(0xA3), IAvnNativePopupHandler::IID);
    }
}

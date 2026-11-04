use crate::generated::IAvnControl;
use crate::{ComInterface, ComPtr, Guid, IUnknown, Result};
use std::ffi::c_void;

#[repr(C)]
struct ApplicationVtbl {
    query: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set_main_view: unsafe extern "system" fn(*mut IAvnApplication13, *mut IAvnControl) -> i32,
}

/// Single-view application capability: hosts whose lifetime has one main
/// view instead of windows (the browser host) accept the root control here.
/// Desktop hosts return `E_NOTIMPL`.
#[repr(C)]
pub struct IAvnApplication13 {
    vtbl: *const ApplicationVtbl,
}

unsafe impl ComInterface for IAvnApplication13 {
    const IID: Guid = Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA8],
    };
}

impl ComPtr<IAvnApplication13> {
    pub fn set_main_view(&self, view: Option<&ComPtr<IAvnControl>>) -> Result<()> {
        unsafe {
            crate::hresult::check(((*(*self.as_raw()).vtbl).set_main_view)(
                self.as_raw(),
                view.map_or(std::ptr::null_mut(), |view| view.as_raw()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn additive_vtable_shape() {
        assert_eq!(
            std::mem::size_of::<ApplicationVtbl>(),
            4 * std::mem::size_of::<usize>()
        );
    }
}

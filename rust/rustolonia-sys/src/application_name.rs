use crate::{ComInterface, ComPtr, Guid, IUnknown, Result};
use std::ffi::c_void;

#[repr(C)]
struct ApplicationVtbl {
    query: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set_application_name: unsafe extern "system" fn(*mut IAvnApplication14, *const u16) -> i32,
}

/// Application naming before Avalonia initialization.
#[repr(C)]
pub struct IAvnApplication14 {
    vtbl: *const ApplicationVtbl,
}

unsafe impl ComInterface for IAvnApplication14 {
    const IID: Guid = Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA9],
    };
}

impl ComPtr<crate::IAvnApplication> {
    pub fn application_name(&self) -> Result<ComPtr<IAvnApplication14>> {
        self.query_interface()
    }
}

impl ComPtr<IAvnApplication14> {
    pub fn set_application_name(&self, name: Option<&[u16]>) -> Result<()> {
        let name = name.map(crate::terminated_utf16);
        unsafe {
            crate::hresult::check(((*(*self.as_raw()).vtbl).set_application_name)(
                self.as_raw(),
                name.as_ref().map_or(std::ptr::null(), |name| name.as_ptr()),
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

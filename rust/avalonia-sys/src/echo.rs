use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use crate::Host;
use std::ffi::c_void;
use std::ptr;

#[repr(C)]
struct IAvnEchoVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    ping: unsafe extern "system" fn(*mut IAvnEcho, i32, *mut i32) -> i32,
    echo_string: unsafe extern "system" fn(*mut IAvnEcho, *const u16, *mut *mut u16) -> i32,
    fail: unsafe extern "system" fn(*mut IAvnEcho) -> i32,
}

#[repr(C)]
pub struct IAvnEcho {
    vtbl: *const IAvnEchoVtbl,
}

unsafe impl ComInterface for IAvnEcho {
    const IID: Guid = Guid::IAVN_ECHO;
}

impl ComPtr<IAvnEcho> {
    pub fn ping(&self, value: i32) -> Result<i32> {
        unsafe {
            let mut result = 0;
            let hr =
                ((*self.as_raw()).vtbl.as_ref().unwrap().ping)(self.as_raw(), value, &mut result);
            hresult::check(hr).map(|_| result)
        }
    }

    pub fn echo_string(&self, host: &Host, input: &str) -> Result<String> {
        let utf16: Vec<u16> = input.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let mut out = ptr::null_mut();
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().echo_string)(
                self.as_raw(),
                utf16.as_ptr(),
                &mut out,
            );
            hresult::check(hr)?;
            Ok(read_and_free_utf16(host, out))
        }
    }

    pub fn fail(&self) -> i32 {
        unsafe { ((*self.as_raw()).vtbl.as_ref().unwrap().fail)(self.as_raw()) }
    }
}

unsafe fn read_and_free_utf16(host: &Host, ptr: *mut u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    let s = String::from_utf16_lossy(slice);
    host.free(ptr.cast());
    s
}

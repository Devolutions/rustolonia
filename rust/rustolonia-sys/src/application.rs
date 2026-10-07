use crate::app_handler::IAvnAppHandler;
use crate::async_completion::IAvnAsyncCompletion;
use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::IAvnWindow;
use crate::guid::Guid;
use crate::hresult::{self, Result};
use crate::rust_vm::IAvnRustViewModel;
use crate::terminated_utf16;
use crate::value_converter::IAvnRustValueConverterProvider;
use std::ffi::c_void;
use std::ptr;

#[cfg(test)]
mod string_tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn empty_and_unterminated_application_arguments_are_terminated() {
        for value in [&[][..], &[65, 66][..], &[0xd800][..]] {
            let buffer = terminated_utf16(value);
            assert_eq!(&buffer[..value.len()], value);
            assert_eq!(buffer.last(), Some(&0));
            assert_eq!(buffer.len(), value.len() + 1);
        }
    }

    #[test]
    fn terminated_arguments_preserve_existing_nul_semantics_without_allocation() {
        for value in [&[0][..], &[65, 0][..], &[65, 0, 66][..]] {
            let buffer = terminated_utf16(value);
            assert!(matches!(buffer, Cow::Borrowed(_)));
            assert_eq!(buffer.as_ptr(), value.as_ptr());
        }
    }
}

const IAVN_APPLICATION2_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x41],
};

const IAVN_RESOURCE_VALUE_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x16],
};

#[repr(C)]
struct IAvnResourceValueVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_kind: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i32) -> i32,
    get_boolean: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i32) -> i32,
    get_integer: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i64) -> i32,
    get_double: unsafe extern "system" fn(*mut IAvnResourceValue, *mut f64) -> i32,
    get_string: unsafe extern "system" fn(*mut IAvnResourceValue, *mut *mut u16) -> i32,
    get_color: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i32) -> i32,
}

#[repr(C)]
pub struct IAvnResourceValue {
    vtbl: *const IAvnResourceValueVtbl,
}

unsafe impl ComInterface for IAvnResourceValue {
    const IID: Guid = IAVN_RESOURCE_VALUE_IID;
}

impl ComPtr<IAvnResourceValue> {
    fn get_i32(
        &self,
        method: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i32) -> i32,
    ) -> Result<i32> {
        unsafe {
            let mut value = 0;
            hresult::check(method(self.as_raw(), &mut value)).map(|_| value)
        }
    }

    pub fn kind(&self) -> Result<i32> {
        unsafe { self.get_i32((*self.as_raw()).vtbl.as_ref().unwrap().get_kind) }
    }

    pub fn boolean(&self) -> Result<bool> {
        unsafe {
            self.get_i32((*self.as_raw()).vtbl.as_ref().unwrap().get_boolean)
                .map(|value| value != 0)
        }
    }

    pub fn integer(&self) -> Result<i64> {
        unsafe {
            let mut value = 0;
            let hr =
                ((*self.as_raw()).vtbl.as_ref().unwrap().get_integer)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn double(&self) -> Result<f64> {
        unsafe {
            let mut value = 0.0;
            let hr =
                ((*self.as_raw()).vtbl.as_ref().unwrap().get_double)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn string(&self) -> Result<*mut u16> {
        unsafe {
            let mut value = ptr::null_mut();
            let hr =
                ((*self.as_raw()).vtbl.as_ref().unwrap().get_string)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn color(&self) -> Result<u32> {
        unsafe {
            self.get_i32((*self.as_raw()).vtbl.as_ref().unwrap().get_color)
                .map(|value| value as u32)
        }
    }
}

#[repr(C)]
struct IAvnApplicationVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    run: unsafe extern "system" fn(*mut IAvnApplication, *mut c_void) -> i32,
    shutdown: unsafe extern "system" fn(*mut IAvnApplication) -> i32,
    get_requested_theme_variant: unsafe extern "system" fn(*mut IAvnApplication, *mut i32) -> i32,
    set_requested_theme_variant: unsafe extern "system" fn(*mut IAvnApplication, i32) -> i32,
    get_actual_theme_variant: unsafe extern "system" fn(*mut IAvnApplication, *mut i32) -> i32,
    try_get_resource: unsafe extern "system" fn(
        *mut IAvnApplication,
        *const u16,
        i32,
        *mut i32,
        *mut *mut IAvnResourceValue,
    ) -> i32,
    start_delay: unsafe extern "system" fn(
        *mut IAvnApplication,
        i32,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
    start_clipboard_set_text: unsafe extern "system" fn(
        *mut IAvnApplication,
        *mut IAvnWindow,
        *const u16,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
    start_clipboard_get_text: unsafe extern "system" fn(
        *mut IAvnApplication,
        *mut IAvnWindow,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
    cancel_async_operation: unsafe extern "system" fn(*mut IAvnApplication, i64) -> i32,
    create_rust_vm_window: unsafe extern "system" fn(
        *mut IAvnApplication,
        i32,
        *mut IAvnRustViewModel,
        *mut *mut IAvnWindow,
    ) -> i32,
}

#[repr(C)]
pub struct IAvnApplication {
    vtbl: *const IAvnApplicationVtbl,
}

unsafe impl ComInterface for IAvnApplication {
    const IID: Guid = Guid::IAVN_APPLICATION;
}

#[repr(C)]
struct IAvnApplication2Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set_value_converter_provider: unsafe extern "system" fn(
        *mut IAvnApplication2,
        *mut IAvnRustValueConverterProvider,
    ) -> i32,
}

#[repr(C)]
struct IAvnApplication2 {
    vtbl: *const IAvnApplication2Vtbl,
}

unsafe impl ComInterface for IAvnApplication2 {
    const IID: Guid = IAVN_APPLICATION2_IID;
}

impl ComPtr<IAvnApplication> {
    pub fn run(&self, handler: &ComPtr<IAvnAppHandler>) -> Result<()> {
        unsafe {
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().run)(
                self.as_raw(),
                handler.as_raw().cast(),
            );
            hresult::check(hr)
        }
    }

    pub fn shutdown(&self) -> Result<()> {
        unsafe {
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().shutdown)(self.as_raw());
            hresult::check(hr)
        }
    }

    pub fn requested_theme_variant(&self) -> Result<i32> {
        unsafe {
            let mut value = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .get_requested_theme_variant)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn set_requested_theme_variant(&self, value: i32) -> Result<()> {
        unsafe {
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .set_requested_theme_variant)(self.as_raw(), value);
            hresult::check(hr)
        }
    }

    pub fn actual_theme_variant(&self) -> Result<i32> {
        unsafe {
            let mut value = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .get_actual_theme_variant)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn try_get_resource(
        &self,
        key: &[u16],
        theme_variant: i32,
    ) -> Result<Option<ComPtr<IAvnResourceValue>>> {
        let key = terminated_utf16(key);
        unsafe {
            let mut found = 0;
            let mut value = ptr::null_mut();
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().try_get_resource)(
                self.as_raw(),
                key.as_ptr(),
                theme_variant,
                &mut found,
                &mut value,
            );
            hresult::check(hr)?;
            if found == 0 {
                Ok(None)
            } else {
                ComPtr::from_raw(value)
                    .map(Some)
                    .ok_or(crate::Error(hresult::E_POINTER))
            }
        }
    }

    pub fn start_delay(
        &self,
        milliseconds: i32,
        completion: &ComPtr<IAvnAsyncCompletion>,
    ) -> Result<i64> {
        unsafe {
            let mut operation_id = 0;
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().start_delay)(
                self.as_raw(),
                milliseconds,
                completion.as_raw(),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }

    pub fn start_clipboard_set_text(
        &self,
        window: &ComPtr<IAvnWindow>,
        text: &[u16],
        completion: &ComPtr<IAvnAsyncCompletion>,
    ) -> Result<i64> {
        let text = terminated_utf16(text);
        unsafe {
            let mut operation_id = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_clipboard_set_text)(
                self.as_raw(),
                window.as_raw(),
                text.as_ptr(),
                completion.as_raw(),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }

    pub fn start_clipboard_get_text(
        &self,
        window: &ComPtr<IAvnWindow>,
        completion: &ComPtr<IAvnAsyncCompletion>,
    ) -> Result<i64> {
        unsafe {
            let mut operation_id = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_clipboard_get_text)(
                self.as_raw(),
                window.as_raw(),
                completion.as_raw(),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }

    pub fn cancel_async_operation(&self, operation_id: i64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .cancel_async_operation)(
                self.as_raw(), operation_id
            ))
        }
    }

    pub fn create_rust_vm_window(
        &self,
        view_id: i32,
        model: &ComPtr<IAvnRustViewModel>,
    ) -> Result<ComPtr<IAvnWindow>> {
        unsafe {
            let mut window = ptr::null_mut();
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .create_rust_vm_window)(
                self.as_raw(), view_id, model.as_raw(), &mut window
            );
            hresult::check(hr)?;
            ComPtr::from_projected_raw(window)
        }
    }

    /// Registers (or clears, when `provider` is `None`) the single
    /// application-scoped Rust value-converter provider.
    pub fn set_value_converter_provider(
        &self,
        provider: Option<&ComPtr<IAvnRustValueConverterProvider>>,
    ) -> Result<()> {
        unsafe {
            let extension = self.query_interface::<IAvnApplication2>()?;
            let raw = provider.map_or(ptr::null_mut(), ComPtr::as_raw);
            let hr = ((*extension.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .set_value_converter_provider)(extension.as_raw(), raw);
            hresult::check(hr)
        }
    }
}

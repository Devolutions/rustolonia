use crate::application::IAvnApplication;
use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::dispatcher::IAvnDispatcher;
use crate::echo::IAvnEcho;
use crate::generated::IAvnControlFactory;
use crate::guid::Guid;
use crate::hresult::{self, Error, Result};
use std::ffi::c_void;
use std::ptr;

#[repr(C)]
struct IAvnActivationFactoryVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    create_echo: unsafe extern "system" fn(*mut IAvnActivationFactory, *mut *mut IAvnEcho) -> i32,
    create_application:
        unsafe extern "system" fn(*mut IAvnActivationFactory, *mut *mut IAvnApplication) -> i32,
    create_control_factory:
        unsafe extern "system" fn(*mut IAvnActivationFactory, *mut *mut IAvnControlFactory) -> i32,
    create_dispatcher:
        unsafe extern "system" fn(*mut IAvnActivationFactory, *mut *mut IAvnDispatcher) -> i32,
}

#[repr(C)]
pub struct IAvnActivationFactory {
    vtbl: *const IAvnActivationFactoryVtbl,
}

unsafe impl ComInterface for IAvnActivationFactory {
    const IID: Guid = Guid::IAVN_ACTIVATION_FACTORY;
}

impl ComPtr<IAvnActivationFactory> {
    pub fn create_echo(&self) -> Result<ComPtr<IAvnEcho>> {
        unsafe {
            let mut echo = ptr::null_mut();
            let hr =
                ((*self.as_raw()).vtbl.as_ref().unwrap().create_echo)(self.as_raw(), &mut echo);
            hresult::check(hr)?;
            ComPtr::from_raw(echo).ok_or(Error(hresult::E_POINTER))
        }
    }

    pub fn create_application(&self) -> Result<ComPtr<IAvnApplication>> {
        unsafe {
            let mut app = ptr::null_mut();
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().create_application)(
                self.as_raw(),
                &mut app,
            );
            hresult::check(hr)?;
            ComPtr::from_raw(app).ok_or(Error(hresult::E_POINTER))
        }
    }

    pub fn create_control_factory(&self) -> Result<ComPtr<IAvnControlFactory>> {
        unsafe {
            let mut factory = ptr::null_mut();
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .create_control_factory)(self.as_raw(), &mut factory);
            hresult::check(hr)?;
            ComPtr::from_raw(factory).ok_or(Error(hresult::E_POINTER))
        }
    }

    pub fn create_dispatcher(&self) -> Result<ComPtr<IAvnDispatcher>> {
        unsafe {
            let mut dispatcher = ptr::null_mut();
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().create_dispatcher)(
                self.as_raw(),
                &mut dispatcher,
            );
            hresult::check(hr)?;
            ComPtr::from_raw(dispatcher).ok_or(Error(hresult::E_POINTER))
        }
    }
}

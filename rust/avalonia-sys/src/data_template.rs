//! Rust-side `IAvnDataTemplate` implementation.
//!
//! `data_template()` builds a ref-counted CCW the NativeAOT host consumes as an
//! `IDataTemplate`: `Match` receives the item as an `AvnVariant` and reports
//! whether this template builds a control for it, and `Build` hands back a
//! freshly created control. The host frees the variant payload after the call.

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::{AvnVariant, IAvnControl, IAvnDataTemplate};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Mutex;

type MatchCallback = Box<dyn FnMut(AvnVariant) -> Result<bool> + Send>;
type BuildCallback = Box<dyn FnMut() -> Result<ComPtr<IAvnControl>> + Send>;

#[repr(C)]
struct DataTemplateVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    match_: unsafe extern "system" fn(*mut IAvnDataTemplate, AvnVariant, *mut i32) -> i32,
    build: unsafe extern "system" fn(*mut IAvnDataTemplate, *mut *mut IAvnControl) -> i32,
}

#[repr(C)]
struct DataTemplateObject {
    vtbl: *const DataTemplateVtbl,
    ref_count: AtomicU32,
    matches: Mutex<Option<MatchCallback>>,
    build: Mutex<Option<BuildCallback>>,
}

/// Builds an `IAvnDataTemplate` from Rust closures. `matches` receives the
/// item as the raw ABI variant (its UTF-16 payload is freed after the call),
/// and `build` returns the control to realize for a matching item.
pub fn data_template(
    matches: impl FnMut(AvnVariant) -> Result<bool> + Send + 'static,
    build: impl FnMut() -> Result<ComPtr<IAvnControl>> + Send + 'static,
) -> DataTemplate {
    let object = Box::into_raw(Box::new(DataTemplateObject {
        vtbl: &DATA_TEMPLATE_VTBL,
        ref_count: AtomicU32::new(1),
        matches: Mutex::new(Some(Box::new(matches))),
        build: Mutex::new(Some(Box::new(build))),
    }));
    DataTemplate {
        ptr: unsafe {
            ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
        },
    }
}

/// An owned handle to a Rust-built data template, safe to hand to the
/// ItemTemplate/HeaderTemplate/ContentTemplate setters.
#[derive(Clone, Debug)]
pub struct DataTemplate {
    ptr: ComPtr<IAvnDataTemplate>,
}

impl DataTemplate {
    pub fn as_com_ptr(&self) -> &ComPtr<IAvnDataTemplate> {
        &self.ptr
    }
}

fn invoke_callback<T: ?Sized>(
    lock: &Mutex<Option<Box<T>>>,
    invoke: impl FnOnce(&mut T) -> Result<()>,
) -> i32 {
    let callback = {
        let mut slot = match lock.lock() {
            Ok(slot) => slot,
            Err(_) => return hresult::E_FAIL,
        };
        match slot.take() {
            Some(callback) => callback,
            None => return hresult::E_FAIL,
        }
    };
    let mut callback = callback;
    let result = catch_unwind(AssertUnwindSafe(|| invoke(callback.as_mut())));
    if let Ok(mut slot) = lock.lock() {
        *slot = Some(callback);
    }
    match result {
        Ok(Ok(())) => hresult::S_OK,
        Ok(Err(error)) => error.0,
        Err(_) => hresult::E_FAIL,
    }
}

unsafe extern "system" fn data_template_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    unsafe {
        *result = ptr::null_mut();
        if *iid != Guid::IUNKNOWN && *iid != IAvnDataTemplate::IID {
            return hresult::E_NOINTERFACE;
        }
        data_template_add_ref(this);
        *result = this.cast();
        hresult::S_OK
    }
}

unsafe extern "system" fn data_template_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<DataTemplateObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn data_template_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<DataTemplateObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn data_template_match(
    this: *mut IAvnDataTemplate,
    data: AvnVariant,
    value: *mut i32,
) -> i32 {
    if value.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<DataTemplateObject>();
    let mut result = 0i32;
    let hr = invoke_callback(&(*object).matches, |matches| {
        matches(data).map(|matched| result = i32::from(matched))
    });
    if data.tag == AvnVariant::TAG_UTF16 && !data.utf16.is_null() {
        crate::free_utf16_if_host(data.utf16);
    }
    if hr == 0 {
        *value = result;
    }
    hr
}

unsafe extern "system" fn data_template_build(
    this: *mut IAvnDataTemplate,
    value: *mut *mut IAvnControl,
) -> i32 {
    if value.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<DataTemplateObject>();
    let mut built: Option<ComPtr<IAvnControl>> = None;
    let hr = invoke_callback(&(*object).build, |build| {
        build().map(|control| built = Some(control))
    });
    if hr == 0 {
        // The host takes ownership of the interface pointer.
        *value = built
            .map(|control| {
                let raw = control.as_raw();
                core::mem::forget(control);
                raw
            })
            .unwrap_or(ptr::null_mut());
    }
    hr
}

#[rustfmt::skip]
static DATA_TEMPLATE_VTBL: DataTemplateVtbl = DataTemplateVtbl {
    query_interface: data_template_query_interface,
    add_ref: data_template_add_ref,
    release: data_template_release,
    match_: data_template_match,
    build: data_template_build,
};

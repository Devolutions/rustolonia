//! Rust-side `IAvnItemFilter`/`IAvnTextFilter` implementations for
//! AutoCompleteBox's filter predicates. `item_filter()` and `text_filter()`
//! build ref-counted CCWs the NativeAOT host consumes as delegates; the host
//! frees the variant payload after each invoke.

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::{AvnVariant, IAvnItemFilter, IAvnTextFilter};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Mutex;

type ItemFilterCallback = Box<dyn FnMut(*const u16, AvnVariant) -> Result<bool> + Send>;
type TextFilterCallback = Box<dyn FnMut(*const u16, *const u16) -> Result<bool> + Send>;

#[repr(C)]
struct ItemFilterVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    invoke: unsafe extern "system" fn(*mut IAvnItemFilter, *const u16, AvnVariant, *mut i32) -> i32,
}

#[repr(C)]
struct ItemFilterObject {
    vtbl: *const ItemFilterVtbl,
    ref_count: AtomicU32,
    callback: Mutex<Option<ItemFilterCallback>>,
}

/// Builds an `IAvnItemFilter` from a Rust closure receiving the search text as
/// a borrowed NUL-terminated UTF-16 buffer and the item as the raw variant
/// (whose UTF-16 payload is freed after the call).
pub fn item_filter(
    callback: impl FnMut(*const u16, AvnVariant) -> Result<bool> + Send + 'static,
) -> ItemFilter {
    let object = Box::into_raw(Box::new(ItemFilterObject {
        vtbl: &ITEM_FILTER_VTBL,
        ref_count: AtomicU32::new(1),
        callback: Mutex::new(Some(Box::new(callback))),
    }));
    ItemFilter {
        ptr: unsafe {
            ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
        },
    }
}

/// An owned handle to a Rust-built item filter, safe to hand to the
/// ItemFilter setter.
#[derive(Clone, Debug)]
pub struct ItemFilter {
    ptr: ComPtr<IAvnItemFilter>,
}

impl ItemFilter {
    pub fn as_com_ptr(&self) -> &ComPtr<IAvnItemFilter> {
        &self.ptr
    }
}

#[repr(C)]
struct TextFilterVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    invoke: unsafe extern "system" fn(*mut IAvnTextFilter, *const u16, *const u16, *mut i32) -> i32,
}

#[repr(C)]
struct TextFilterObject {
    vtbl: *const TextFilterVtbl,
    ref_count: AtomicU32,
    callback: Mutex<Option<TextFilterCallback>>,
}

/// Builds an `IAvnTextFilter` from a Rust closure receiving both the search
/// text and the item as borrowed NUL-terminated UTF-16 buffers.
pub fn text_filter(
    callback: impl FnMut(*const u16, *const u16) -> Result<bool> + Send + 'static,
) -> TextFilter {
    let object = Box::into_raw(Box::new(TextFilterObject {
        vtbl: &TEXT_FILTER_VTBL,
        ref_count: AtomicU32::new(1),
        callback: Mutex::new(Some(Box::new(callback))),
    }));
    TextFilter {
        ptr: unsafe {
            ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
        },
    }
}

/// An owned handle to a Rust-built text filter, safe to hand to the
/// TextFilter setter.
#[derive(Clone, Debug)]
pub struct TextFilter {
    ptr: ComPtr<IAvnTextFilter>,
}

impl TextFilter {
    pub fn as_com_ptr(&self) -> &ComPtr<IAvnTextFilter> {
        &self.ptr
    }
}

fn invoke_callback<A, T: ?Sized>(
    lock: &Mutex<Option<Box<T>>>,
    arguments: &mut A,
    invoke: impl FnOnce(&mut T, &mut A) -> Result<()>,
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
    let result = catch_unwind(AssertUnwindSafe(|| invoke(callback.as_mut(), arguments)));
    if let Ok(mut slot) = lock.lock() {
        *slot = Some(callback);
    }
    match result {
        Ok(Ok(())) => hresult::S_OK,
        Ok(Err(error)) => error.0,
        Err(_) => hresult::E_FAIL,
    }
}

unsafe extern "system" fn shared_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
    expected: Guid,
) -> i32 {
    if iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    unsafe {
        *result = ptr::null_mut();
        if *iid != Guid::IUNKNOWN && *iid != expected {
            return hresult::E_NOINTERFACE;
        }
        *result = this.cast();
        hresult::S_OK
    }
}

unsafe extern "system" fn item_filter_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    let hr = shared_query_interface(this, iid, result, IAvnItemFilter::IID);
    if hr == 0 {
        item_filter_add_ref(this);
    }
    hr
}

unsafe extern "system" fn item_filter_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<ItemFilterObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn item_filter_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<ItemFilterObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn item_filter_invoke(
    this: *mut IAvnItemFilter,
    search: *const u16,
    item: AvnVariant,
    result: *mut i32,
) -> i32 {
    if result.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<ItemFilterObject>();
    let mut value = 0i32;
    let mut arguments = (search, item);
    let hr = invoke_callback(
        &(*object).callback,
        &mut arguments,
        |callback, arguments| {
            callback(arguments.0, arguments.1).map(|matched| value = i32::from(matched))
        },
    );
    let (search, item) = arguments;
    let _ = search;
    if item.tag == AvnVariant::TAG_UTF16 && !item.utf16.is_null() {
        crate::free_utf16_if_host(item.utf16);
    }
    if hr == 0 {
        *result = value;
    }
    hr
}

unsafe extern "system" fn text_filter_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    let hr = shared_query_interface(this, iid, result, IAvnTextFilter::IID);
    if hr == 0 {
        text_filter_add_ref(this);
    }
    hr
}

unsafe extern "system" fn text_filter_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<TextFilterObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn text_filter_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<TextFilterObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn text_filter_invoke(
    this: *mut IAvnTextFilter,
    search: *const u16,
    item: *const u16,
    result: *mut i32,
) -> i32 {
    if result.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<TextFilterObject>();
    let mut value = 0i32;
    let hr = invoke_callback(
        &(*object).callback,
        &mut (search, item),
        |callback, arguments| {
            callback(arguments.0, arguments.1).map(|matched| value = i32::from(matched))
        },
    );
    if hr == 0 {
        *result = value;
    }
    hr
}

#[rustfmt::skip]
static ITEM_FILTER_VTBL: ItemFilterVtbl = ItemFilterVtbl {
    query_interface: item_filter_query_interface,
    add_ref: item_filter_add_ref,
    release: item_filter_release,
    invoke: item_filter_invoke,
};

#[rustfmt::skip]
static TEXT_FILTER_VTBL: TextFilterVtbl = TextFilterVtbl {
    query_interface: text_filter_query_interface,
    add_ref: text_filter_add_ref,
    release: text_filter_release,
    invoke: text_filter_invoke,
};

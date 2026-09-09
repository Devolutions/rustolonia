//! Rust-side `IAvnItemSelector`/`IAvnTextSelector` implementations for
//! AutoCompleteBox's selector delegates. `item_selector()` and
//! `text_selector()` build ref-counted CCWs the NativeAOT host consumes as
//! delegates; the returned string is host-allocated (the host frees it), and
//! the variant payload is freed after each invoke when a host is loaded.

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::{AvnVariant, IAvnItemSelector, IAvnTextSelector};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Mutex;

type ItemSelectorCallback = Box<dyn FnMut(*const u16, AvnVariant) -> Result<String> + Send>;
type TextSelectorCallback = Box<dyn FnMut(*const u16, *const u16) -> Result<String> + Send>;

#[repr(C)]
struct ItemSelectorVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    invoke: unsafe extern "system" fn(
        *mut IAvnItemSelector,
        *const u16,
        AvnVariant,
        *mut *mut u16,
    ) -> i32,
}

#[repr(C)]
struct ItemSelectorObject {
    vtbl: *const ItemSelectorVtbl,
    ref_count: AtomicU32,
    callback: Mutex<Option<ItemSelectorCallback>>,
}

/// Builds an `IAvnItemSelector` from a Rust closure receiving the search text
/// as a borrowed NUL-terminated UTF-16 buffer and the item as the raw variant,
/// returning the display text (host-allocated on return).
pub fn item_selector(
    callback: impl FnMut(*const u16, AvnVariant) -> Result<String> + Send + 'static,
) -> ItemSelector {
    let object = Box::into_raw(Box::new(ItemSelectorObject {
        vtbl: &ITEM_SELECTOR_VTBL,
        ref_count: AtomicU32::new(1),
        callback: Mutex::new(Some(Box::new(callback))),
    }));
    ItemSelector {
        ptr: unsafe {
            ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
        },
    }
}

/// An owned handle to a Rust-built item selector, safe to hand to the
/// ItemSelector setter.
#[derive(Clone, Debug)]
pub struct ItemSelector {
    ptr: ComPtr<IAvnItemSelector>,
}

impl ItemSelector {
    pub fn as_com_ptr(&self) -> &ComPtr<IAvnItemSelector> {
        &self.ptr
    }
}

#[repr(C)]
struct TextSelectorVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    invoke: unsafe extern "system" fn(
        *mut IAvnTextSelector,
        *const u16,
        *const u16,
        *mut *mut u16,
    ) -> i32,
}

#[repr(C)]
struct TextSelectorObject {
    vtbl: *const TextSelectorVtbl,
    ref_count: AtomicU32,
    callback: Mutex<Option<TextSelectorCallback>>,
}

/// Builds an `IAvnTextSelector` from a Rust closure receiving both the search
/// text and the item as borrowed NUL-terminated UTF-16 buffers, returning the
/// display text (host-allocated on return).
pub fn text_selector(
    callback: impl FnMut(*const u16, *const u16) -> Result<String> + Send + 'static,
) -> TextSelector {
    let object = Box::into_raw(Box::new(TextSelectorObject {
        vtbl: &TEXT_SELECTOR_VTBL,
        ref_count: AtomicU32::new(1),
        callback: Mutex::new(Some(Box::new(callback))),
    }));
    TextSelector {
        ptr: unsafe {
            ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
        },
    }
}

/// An owned handle to a Rust-built text selector, safe to hand to the
/// TextSelector setter.
#[derive(Clone, Debug)]
pub struct TextSelector {
    ptr: ComPtr<IAvnTextSelector>,
}

impl TextSelector {
    pub fn as_com_ptr(&self) -> &ComPtr<IAvnTextSelector> {
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

unsafe fn allocate_utf16(
    alloc: unsafe extern "C" fn(i32) -> *mut u16,
    text: &str,
) -> Option<*mut u16> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let Ok(length) = i32::try_from(units.len()) else {
        return None;
    };
    unsafe {
        let buffer = alloc(length);
        if buffer.is_null() {
            return None;
        }
        for (index, unit) in units.iter().enumerate() {
            *buffer.add(index) = *unit;
        }
        *buffer.add(units.len()) = 0;
        Some(buffer)
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

unsafe extern "system" fn item_selector_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    let hr = shared_query_interface(this, iid, result, IAvnItemSelector::IID);
    if hr == 0 {
        item_selector_add_ref(this);
    }
    hr
}

unsafe extern "system" fn item_selector_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<ItemSelectorObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn item_selector_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<ItemSelectorObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn item_selector_invoke(
    this: *mut IAvnItemSelector,
    search: *const u16,
    item: AvnVariant,
    text: *mut *mut u16,
) -> i32 {
    if text.is_null() {
        return hresult::E_POINTER;
    }
    let Some(alloc) = crate::alloc_utf16_provider() else {
        return hresult::E_NOTIMPL;
    };
    let object = this.cast::<ItemSelectorObject>();
    let mut selected = String::new();
    let mut arguments = (search, item);
    let hr = invoke_callback(
        &(*object).callback,
        &mut arguments,
        |callback, arguments| callback(arguments.0, arguments.1).map(|value| selected = value),
    );
    let (_, item) = arguments;
    if item.tag == AvnVariant::TAG_UTF16 && !item.utf16.is_null() {
        crate::free_utf16_if_host(item.utf16);
    }
    if hr != 0 {
        return hr;
    }
    match allocate_utf16(alloc, &selected) {
        Some(buffer) => {
            *text = buffer;
            hresult::S_OK
        }
        None => 0x8007_0000u32 as i32,
    }
}

unsafe extern "system" fn text_selector_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    let hr = shared_query_interface(this, iid, result, IAvnTextSelector::IID);
    if hr == 0 {
        text_selector_add_ref(this);
    }
    hr
}

unsafe extern "system" fn text_selector_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<TextSelectorObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn text_selector_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<TextSelectorObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn text_selector_invoke(
    this: *mut IAvnTextSelector,
    search: *const u16,
    item: *const u16,
    text: *mut *mut u16,
) -> i32 {
    if text.is_null() {
        return hresult::E_POINTER;
    }
    let Some(alloc) = crate::alloc_utf16_provider() else {
        return hresult::E_NOTIMPL;
    };
    let object = this.cast::<TextSelectorObject>();
    let mut selected = String::new();
    let hr = invoke_callback(
        &(*object).callback,
        &mut (search, item),
        |callback, arguments| callback(arguments.0, arguments.1).map(|value| selected = value),
    );
    if hr != 0 {
        return hr;
    }
    match allocate_utf16(alloc, &selected) {
        Some(buffer) => {
            *text = buffer;
            hresult::S_OK
        }
        None => 0x8007_0000u32 as i32,
    }
}

#[rustfmt::skip]
static ITEM_SELECTOR_VTBL: ItemSelectorVtbl = ItemSelectorVtbl {
    query_interface: item_selector_query_interface,
    add_ref: item_selector_add_ref,
    release: item_selector_release,
    invoke: item_selector_invoke,
};

#[rustfmt::skip]
static TEXT_SELECTOR_VTBL: TextSelectorVtbl = TextSelectorVtbl {
    query_interface: text_selector_query_interface,
    add_ref: text_selector_add_ref,
    release: text_selector_release,
    invoke: text_selector_invoke,
};

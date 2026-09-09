//! Rust-side `IAvnNotification` implementation.
//!
//! `notification()` builds a ref-counted CCW the NativeAOT host consumes as an
//! `INotification`: the host reads Title, Message, Type and Expiration through
//! the getter slots, and invokes OnClick/OnClose through the handler the CCW
//! returns from those slots. Title and Message strings are allocated with the
//! host allocator (the host releases them) and cached so repeated reads work.

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::{IAvnNotification, IAvnNotificationActionHandler};
use crate::guid::Guid;
use crate::hresult;
use std::ffi::c_void;
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Mutex;

/// A notification handed to `WindowNotificationManager::show`.
#[derive(Clone, Debug)]
pub struct NotificationSpec {
    pub title: String,
    pub message: String,
    /// NotificationType: 0 Information, 1 Success, 2 Warning, 3 Error.
    pub kind: i32,
    /// Expiration as .NET ticks (100ns units).
    pub expiration_ticks: i64,
    pub on_click: Option<ComPtr<IAvnNotificationActionHandler>>,
    pub on_close: Option<ComPtr<IAvnNotificationActionHandler>>,
}

/// Builds an `IAvnNotification` from a spec. The click/close handlers come
/// from `notification_action_handler`, the generated CCW constructor.
pub fn notification(spec: NotificationSpec) -> Notification {
    let object = Box::into_raw(Box::new(NotificationObject {
        vtbl: &NOTIFICATION_VTBL,
        ref_count: AtomicU32::new(1),
        spec: Mutex::new(spec),
    }));
    Notification {
        ptr: unsafe {
            ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
        },
    }
}

/// An owned handle to a Rust-built notification, safe to pass to the
/// notification manager's Show slot.
#[derive(Clone, Debug)]
pub struct Notification {
    ptr: ComPtr<IAvnNotification>,
}

impl Notification {
    pub fn as_com_ptr(&self) -> &ComPtr<IAvnNotification> {
        &self.ptr
    }
}

#[repr(C)]
struct NotificationVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_title: unsafe extern "system" fn(*mut IAvnNotification, *mut *mut u16) -> i32,
    get_message: unsafe extern "system" fn(*mut IAvnNotification, *mut *mut u16) -> i32,
    get_type: unsafe extern "system" fn(*mut IAvnNotification, *mut i32) -> i32,
    get_expiration: unsafe extern "system" fn(*mut IAvnNotification, *mut i64) -> i32,
    get_on_click: unsafe extern "system" fn(
        *mut IAvnNotification,
        *mut *mut IAvnNotificationActionHandler,
    ) -> i32,
    get_on_close: unsafe extern "system" fn(
        *mut IAvnNotification,
        *mut *mut IAvnNotificationActionHandler,
    ) -> i32,
}

#[repr(C)]
struct NotificationObject {
    vtbl: *const NotificationVtbl,
    ref_count: AtomicU32,
    spec: Mutex<NotificationSpec>,
}

unsafe extern "system" fn notification_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    unsafe {
        *result = std::ptr::null_mut();
        if *iid != Guid::IUNKNOWN && *iid != IAvnNotification::IID {
            return hresult::E_NOINTERFACE;
        }
        notification_add_ref(this);
        *result = this.cast();
        hresult::S_OK
    }
}

unsafe extern "system" fn notification_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<NotificationObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn notification_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<NotificationObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn notification_get_title(
    this: *mut IAvnNotification,
    value: *mut *mut u16,
) -> i32 {
    if value.is_null() {
        return hresult::E_POINTER;
    }
    // Strings must come from the host allocator so the host can free them;
    // without a loaded host (unit tests) there is no allocator to answer to.
    let Some(alloc) = crate::alloc_utf16_provider() else {
        return hresult::E_NOTIMPL;
    };
    let object = this.cast::<NotificationObject>();
    let spec = match (*object).spec.lock() {
        Ok(spec) => spec,
        Err(_) => return hresult::E_FAIL,
    };
    let buffer = match allocate_utf16(alloc, &spec.title) {
        Some(buffer) => buffer,
        None => return hresult::E_FAIL,
    };
    *value = buffer;
    hresult::S_OK
}

/// Allocates a host buffer and copies the string plus its terminator.
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

unsafe extern "system" fn notification_get_message(
    this: *mut IAvnNotification,
    value: *mut *mut u16,
) -> i32 {
    if value.is_null() {
        return hresult::E_POINTER;
    }
    let Some(alloc) = crate::alloc_utf16_provider() else {
        return hresult::E_NOTIMPL;
    };
    let object = this.cast::<NotificationObject>();
    let spec = match (*object).spec.lock() {
        Ok(spec) => spec,
        Err(_) => return hresult::E_FAIL,
    };
    let buffer = match allocate_utf16(alloc, &spec.message) {
        Some(buffer) => buffer,
        None => return hresult::E_FAIL,
    };
    *value = buffer;
    hresult::S_OK
}

unsafe extern "system" fn notification_get_type(
    this: *mut IAvnNotification,
    value: *mut i32,
) -> i32 {
    if value.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<NotificationObject>();
    let spec = match (*object).spec.lock() {
        Ok(spec) => spec,
        Err(_) => return hresult::E_FAIL,
    };
    *value = spec.kind;
    hresult::S_OK
}

unsafe extern "system" fn notification_get_expiration(
    this: *mut IAvnNotification,
    value: *mut i64,
) -> i32 {
    if value.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<NotificationObject>();
    let spec = match (*object).spec.lock() {
        Ok(spec) => spec,
        Err(_) => return hresult::E_FAIL,
    };
    *value = spec.expiration_ticks;
    hresult::S_OK
}

unsafe extern "system" fn notification_get_on_click(
    this: *mut IAvnNotification,
    value: *mut *mut IAvnNotificationActionHandler,
) -> i32 {
    if value.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<NotificationObject>();
    let spec = match (*object).spec.lock() {
        Ok(spec) => spec,
        Err(_) => return hresult::E_FAIL,
    };
    match &spec.on_click {
        Some(handler) => {
            handler.add_ref();
            *value = handler.as_raw();
        }
        None => *value = std::ptr::null_mut(),
    }
    hresult::S_OK
}

unsafe extern "system" fn notification_get_on_close(
    this: *mut IAvnNotification,
    value: *mut *mut IAvnNotificationActionHandler,
) -> i32 {
    if value.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<NotificationObject>();
    let spec = match (*object).spec.lock() {
        Ok(spec) => spec,
        Err(_) => return hresult::E_FAIL,
    };
    match &spec.on_close {
        Some(handler) => {
            handler.add_ref();
            *value = handler.as_raw();
        }
        None => *value = std::ptr::null_mut(),
    }
    hresult::S_OK
}

#[rustfmt::skip]
static NOTIFICATION_VTBL: NotificationVtbl = NotificationVtbl {
    query_interface: notification_query_interface,
    add_ref: notification_add_ref,
    release: notification_release,
    get_title: notification_get_title,
    get_message: notification_get_message,
    get_type: notification_get_type,
    get_expiration: notification_get_expiration,
    get_on_click: notification_get_on_click,
    get_on_close: notification_get_on_close,
};

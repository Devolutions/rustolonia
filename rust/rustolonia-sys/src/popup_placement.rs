//! Rust-side `IAvnPopupPlacementCallback` implementation for custom popup
//! placement. `popup_placement()` builds a ref-counted CCW the NativeAOT host
//! consumes as the managed `CustomPopupPlacementCallback` delegate; the host
//! passes the popup geometry and reads the mutated placement back through the
//! out-parameters.

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::IAvnPopupPlacementCallback;
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Mutex;

/// The placement the callback produced.
pub struct PopupPlacementResult {
    pub offset_x: f64,
    pub offset_y: f64,
    pub anchor: i32,
    pub gravity: i32,
    pub constraint_adjustment: i32,
}

type PopupPlacementCallback =
    Box<dyn FnMut(f64, f64, f64, f64, f64, f64) -> Result<PopupPlacementResult> + Send>;

#[repr(C)]
struct PopupPlacementVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    invoke: unsafe extern "system" fn(
        *mut IAvnPopupPlacementCallback,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        *mut f64,
        *mut f64,
        *mut i32,
        *mut i32,
        *mut i32,
    ) -> i32,
}

#[repr(C)]
struct PopupPlacementObject {
    vtbl: *const PopupPlacementVtbl,
    ref_count: AtomicU32,
    callback: Mutex<Option<PopupPlacementCallback>>,
}

/// Builds an `IAvnPopupPlacementCallback` from a Rust closure receiving the
/// popup size and anchor rectangle and returning the placement to apply.
pub fn popup_placement(
    callback: impl FnMut(f64, f64, f64, f64, f64, f64) -> Result<PopupPlacementResult> + Send + 'static,
) -> PopupPlacement {
    let object = Box::into_raw(Box::new(PopupPlacementObject {
        vtbl: &POPUP_PLACEMENT_VTBL,
        ref_count: AtomicU32::new(1),
        callback: Mutex::new(Some(Box::new(callback))),
    }));
    PopupPlacement {
        ptr: unsafe {
            ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
        },
    }
}

/// An owned handle to a Rust-built placement callback, safe to hand to the
/// CustomPopupPlacementCallback setter.
#[derive(Clone, Debug)]
pub struct PopupPlacement {
    ptr: ComPtr<IAvnPopupPlacementCallback>,
}

impl PopupPlacement {
    pub fn as_com_ptr(&self) -> &ComPtr<IAvnPopupPlacementCallback> {
        &self.ptr
    }
}

fn invoke_callback(
    lock: &Mutex<Option<PopupPlacementCallback>>,
    arguments: &mut (f64, f64, f64, f64, f64, f64),
    result: &mut Option<PopupPlacementResult>,
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
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        callback(
            arguments.0,
            arguments.1,
            arguments.2,
            arguments.3,
            arguments.4,
            arguments.5,
        )
    }));
    if let Ok(mut slot) = lock.lock() {
        *slot = Some(callback);
    }
    match outcome {
        Ok(Ok(placement)) => {
            *result = Some(placement);
            hresult::S_OK
        }
        Ok(Err(error)) => error.0,
        Err(_) => hresult::E_FAIL,
    }
}

unsafe extern "system" fn shared_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    unsafe {
        *result = std::ptr::null_mut();
        if *iid != Guid::IUNKNOWN && *iid != IAvnPopupPlacementCallback::IID {
            return hresult::E_NOINTERFACE;
        }
        *result = this.cast();
        hresult::S_OK
    }
}

unsafe extern "system" fn popup_placement_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    let hr = shared_query_interface(this, iid, result);
    if hr == 0 {
        popup_placement_add_ref(this);
    }
    hr
}

unsafe extern "system" fn popup_placement_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<PopupPlacementObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn popup_placement_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<PopupPlacementObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn popup_placement_invoke(
    this: *mut IAvnPopupPlacementCallback,
    popup_width: f64,
    popup_height: f64,
    anchor_x: f64,
    anchor_y: f64,
    anchor_width: f64,
    anchor_height: f64,
    offset_x: *mut f64,
    offset_y: *mut f64,
    anchor: *mut i32,
    gravity: *mut i32,
    constraint_adjustment: *mut i32,
) -> i32 {
    if offset_x.is_null()
        || offset_y.is_null()
        || anchor.is_null()
        || gravity.is_null()
        || constraint_adjustment.is_null()
    {
        return hresult::E_POINTER;
    }
    let object = this.cast::<PopupPlacementObject>();
    let mut arguments = (
        popup_width,
        popup_height,
        anchor_x,
        anchor_y,
        anchor_width,
        anchor_height,
    );
    let mut placement: Option<PopupPlacementResult> = None;
    let hr = invoke_callback(&(*object).callback, &mut arguments, &mut placement);
    if hr == 0 {
        if let Some(placement) = placement {
            unsafe {
                *offset_x = placement.offset_x;
                *offset_y = placement.offset_y;
                *anchor = placement.anchor;
                *gravity = placement.gravity;
                *constraint_adjustment = placement.constraint_adjustment;
            }
        }
    }
    hr
}

#[rustfmt::skip]
static POPUP_PLACEMENT_VTBL: PopupPlacementVtbl = PopupPlacementVtbl {
    query_interface: popup_placement_query_interface,
    add_ref: popup_placement_add_ref,
    release: popup_placement_release,
    invoke: popup_placement_invoke,
};

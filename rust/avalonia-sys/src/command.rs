//! Rust-side `IAvnCommand` implementation.
//!
//! `command()` builds a ref-counted CCW that the NativeAOT host consumes as an
//! `ICommand`: `execute` and `can_execute` cross into Rust closures, and
//! `advise_can_execute_changed` / `notify()` / `unadvise_can_execute_changed`
//! carry the `CanExecuteChanged` notification back to the host. The handler
//! interface the host hands us is kept alive until the matching unadvise or the
//! command's final release, mirroring the host-side `AvnCommand` contract.
//!
//! The vtable here is a layout-compatible mirror of the generated `IAvnCommand`
//! vtable, the same way `dispatcher::IAvnAction` pairs with its local vtable:
//! the object's first field is the vtable pointer, so the host sees a
//! well-formed `IAvnCommand`.

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::{AvnVariant, IAvnCommand, IAvnCommandCanExecuteChangedHandler};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::collections::HashMap;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{fence, AtomicI64, AtomicU32, Ordering};
use std::sync::Mutex;

type ExecuteCallback = Box<dyn FnMut(AvnVariant) -> Result<()> + Send>;
type CanExecuteCallback = Box<dyn FnMut(AvnVariant) -> Result<bool> + Send>;

#[repr(C)]
struct CommandVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    execute: unsafe extern "system" fn(*mut IAvnCommand, AvnVariant) -> i32,
    can_execute: unsafe extern "system" fn(*mut IAvnCommand, AvnVariant, *mut i32) -> i32,
    advise_can_execute_changed: unsafe extern "system" fn(
        *mut IAvnCommand,
        *mut IAvnCommandCanExecuteChangedHandler,
        *mut i64,
    ) -> i32,
    unadvise_can_execute_changed: unsafe extern "system" fn(*mut IAvnCommand, i64) -> i32,
}

#[repr(C)]
struct CommandObject {
    vtbl: *const CommandVtbl,
    ref_count: AtomicU32,
    execute: Mutex<Option<ExecuteCallback>>,
    can_execute: Mutex<Option<CanExecuteCallback>>,
    subscriptions: Mutex<HashMap<i64, ComPtr<IAvnCommandCanExecuteChangedHandler>>>,
    next_subscription_id: AtomicI64,
}

/// Builds an `IAvnCommand` from Rust closures.
///
/// Both closures must be `Send`; they are invoked on whatever thread calls
/// into the command, which for Avalonia controls is the UI thread. The
/// parameter arrives as the raw ABI variant: the UTF-16 payload is borrowed
/// for the duration of the call only, so read it (or copy it) before
/// returning.
pub fn command(
    execute: impl FnMut(AvnVariant) -> Result<()> + Send + 'static,
    can_execute: impl FnMut(AvnVariant) -> Result<bool> + Send + 'static,
) -> Command {
    let object = Box::into_raw(Box::new(CommandObject {
        vtbl: &COMMAND_VTBL,
        ref_count: AtomicU32::new(1),
        execute: Mutex::new(Some(Box::new(execute))),
        can_execute: Mutex::new(Some(Box::new(can_execute))),
        subscriptions: Mutex::new(HashMap::new()),
        next_subscription_id: AtomicI64::new(1),
    }));
    Command {
        ptr: unsafe {
            ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
        },
    }
}

/// An owned handle to a Rust-built command, safe to hand to control setters.
#[derive(Clone, Debug)]
pub struct Command {
    ptr: ComPtr<IAvnCommand>,
}

impl Command {
    /// Fires every live `CanExecuteChanged` subscription. Call this whenever
    /// the value `can_execute` would return has changed.
    pub fn notify(&self) {
        let handlers: Vec<ComPtr<IAvnCommandCanExecuteChangedHandler>> = unsafe {
            let object = self.ptr.as_raw().cast::<CommandObject>();
            let subscriptions = match (*object).subscriptions.lock() {
                Ok(subscriptions) => subscriptions,
                Err(_) => return,
            };
            subscriptions.values().cloned().collect()
        };
        for handler in handlers {
            let _ = handler.invoke();
        }
    }

    pub fn as_com_ptr(&self) -> &ComPtr<IAvnCommand> {
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

unsafe extern "system" fn command_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    unsafe {
        *result = ptr::null_mut();
        if *iid != Guid::IUNKNOWN && *iid != IAvnCommand::IID {
            return hresult::E_NOINTERFACE;
        }
        command_add_ref(this);
        *result = this.cast();
        hresult::S_OK
    }
}

unsafe extern "system" fn command_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<CommandObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn command_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<CommandObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn command_execute(this: *mut IAvnCommand, parameter: AvnVariant) -> i32 {
    let object = this.cast::<CommandObject>();
    let hr = invoke_callback(&(*object).execute, |execute| execute(parameter));
    if parameter.tag == AvnVariant::TAG_UTF16 && !parameter.utf16.is_null() {
        crate::free_utf16_if_host(parameter.utf16);
    }
    hr
}

unsafe extern "system" fn command_can_execute(
    this: *mut IAvnCommand,
    parameter: AvnVariant,
    value: *mut i32,
) -> i32 {
    if value.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<CommandObject>();
    let mut result = 0i32;
    let hr = invoke_callback(&(*object).can_execute, |can_execute| {
        can_execute(parameter).map(|can| result = i32::from(can))
    });
    if parameter.tag == AvnVariant::TAG_UTF16 && !parameter.utf16.is_null() {
        crate::free_utf16_if_host(parameter.utf16);
    }
    if hr == 0 {
        *value = result;
    }
    hr
}

unsafe extern "system" fn command_advise_can_execute_changed(
    this: *mut IAvnCommand,
    handler: *mut IAvnCommandCanExecuteChangedHandler,
    subscription_id: *mut i64,
) -> i32 {
    if handler.is_null() || subscription_id.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<CommandObject>();
    // The caller keeps ownership of the handler it passed; add-ref our own
    // reference so both sides can release independently.
    let handler = match ComPtr::from_borrowed(handler) {
        Some(handler) => handler,
        None => return hresult::E_POINTER,
    };
    let mut subscriptions = match (*object).subscriptions.lock() {
        Ok(subscriptions) => subscriptions,
        Err(_) => return hresult::E_FAIL,
    };
    let id = (*object)
        .next_subscription_id
        .fetch_add(1, Ordering::Relaxed);
    subscriptions.insert(id, handler);
    *subscription_id = id;
    hresult::S_OK
}

unsafe extern "system" fn command_unadvise_can_execute_changed(
    this: *mut IAvnCommand,
    subscription_id: i64,
) -> i32 {
    let object = this.cast::<CommandObject>();
    let mut subscriptions = match (*object).subscriptions.lock() {
        Ok(subscriptions) => subscriptions,
        Err(_) => return hresult::E_FAIL,
    };
    match subscriptions.remove(&subscription_id) {
        Some(_) => hresult::S_OK,
        None => hresult::E_INVALIDARG,
    }
}

#[rustfmt::skip]
static COMMAND_VTBL: CommandVtbl = CommandVtbl {
    query_interface: command_query_interface,
    add_ref: command_add_ref,
    release: command_release,
    execute: command_execute,
    can_execute: command_can_execute,
    advise_can_execute_changed: command_advise_can_execute_changed,
    unadvise_can_execute_changed: command_unadvise_can_execute_changed,
};

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{fence, AtomicI64, AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

const IAVN_RUST_VIEW_MODEL_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x24],
};

const IAVN_RUST_VM_SINK_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x25],
};

/// A second, independently versioned sink interface carrying the transport
/// added for nested view models, nullable values, collection insert/remove/
/// replace/move/clear, command `CanExecute` state, and validation-error
/// projection. Kept separate from `IAvnRustVmSink` (rather than extending its
/// vtable) so an older generated adapter that only implements v1 fails a
/// `QueryInterface` for this IID with a normal, explicit ABI error instead of
/// corrupting an existing call site.
const IAVN_RUST_VM_SINK2_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x26],
};

const IAVN_RUST_VM_SINK3_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x27],
};

const IAVN_RUST_VM_UPDATE_BATCH_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x28],
};

const IAVN_RUST_VM_UPDATE_OPERATION_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x29],
};

/// The batch's separately versioned ownership-commit capability. It is a
/// distinct IID and vtable rather than an extension of
/// `IAvnRustVmUpdateBatch`, so the immutable batch contract that already
/// shipped is untouched. Managed code calls it exactly once, after every
/// notification-free state store and before any notification is published.
const IAVN_RUST_VM_UPDATE_BATCH2_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x2A],
};

/// Stage 30 sink capability: observable keyed maps, structured command
/// results, async command progress and windowed range publication. A new IID
/// and vtable, so every already-published sink contract is untouched.
const IAVN_RUST_VM_SINK4_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x2B],
};

/// Scalar-number collection capability: appending, inserting and replacing
/// `i64`/`f64` elements. A new IID and vtable, so every already-published sink
/// contract is untouched. Removal, movement and clearing carry no element
/// value and therefore stay on `IAvnRustVmSink2`.
const IAVN_RUST_VM_SINK5_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x2F],
};

/// One immutable realized range (or range reset) of a windowed collection.
const IAVN_RUST_VM_RANGE_BATCH_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x2C],
};

/// The Rust-implemented range capability managed code queries from
/// `IAvnRustViewModel`. Its methods must never take the view-model lock: they
/// are called on the UI thread and only enqueue work.
const IAVN_RUST_RANGE_SOURCE_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x2D],
};

/// The Rust-implemented tracked-async capability managed code queries from
/// `IAvnRustViewModel`. `cancel_async` deliberately does not take the
/// view-model lock, so cancelling never blocks the UI thread behind a running
/// worker.
const IAVN_RUST_VIEW_MODEL2_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x2E],
};

#[repr(C)]
struct IAvnRustVmSinkVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set_string: unsafe extern "system" fn(*mut IAvnRustVmSink, i32, *const u16) -> i32,
    set_integer: unsafe extern "system" fn(*mut IAvnRustVmSink, i32, i64) -> i32,
    set_boolean: unsafe extern "system" fn(*mut IAvnRustVmSink, i32, i32) -> i32,
    set_double: unsafe extern "system" fn(*mut IAvnRustVmSink, i32, f64) -> i32,
    add_string: unsafe extern "system" fn(*mut IAvnRustVmSink, i32, *const u16) -> i32,
}

#[repr(C)]
pub struct IAvnRustVmSink {
    vtbl: *const IAvnRustVmSinkVtbl,
}

unsafe impl ComInterface for IAvnRustVmSink {
    const IID: Guid = IAVN_RUST_VM_SINK_IID;
}

impl ComPtr<IAvnRustVmSink> {
    pub fn set_string(&self, property_id: i32, value: &[u16]) -> Result<()> {
        let value = crate::terminated_utf16(value);
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().set_string)(
                self.as_raw(),
                property_id,
                value.as_ptr(),
            ))
        }
    }

    pub fn set_integer(&self, property_id: i32, value: i64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().set_integer)(
                self.as_raw(),
                property_id,
                value,
            ))
        }
    }

    pub fn set_boolean(&self, property_id: i32, value: bool) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().set_boolean)(
                self.as_raw(),
                property_id,
                i32::from(value),
            ))
        }
    }

    pub fn set_double(&self, property_id: i32, value: f64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().set_double)(
                self.as_raw(),
                property_id,
                value,
            ))
        }
    }

    pub fn add_string(&self, collection_id: i32, value: &[u16]) -> Result<()> {
        let value = crate::terminated_utf16(value);
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().add_string)(
                self.as_raw(),
                collection_id,
                value.as_ptr(),
            ))
        }
    }
}

#[repr(C)]
struct IAvnRustVmSink2Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set_null: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32) -> i32,
    set_model: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, *mut IAvnRustViewModel) -> i32,
    add_model: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, *mut IAvnRustViewModel) -> i32,
    insert_string: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, i32, *const u16) -> i32,
    insert_model:
        unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, i32, *mut IAvnRustViewModel) -> i32,
    replace_string: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, i32, *const u16) -> i32,
    replace_model:
        unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, i32, *mut IAvnRustViewModel) -> i32,
    remove_at: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, i32) -> i32,
    move_item: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, i32, i32) -> i32,
    clear_collection: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32) -> i32,
    set_command_enabled: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, i32) -> i32,
    set_property_error: unsafe extern "system" fn(*mut IAvnRustVmSink2, i32, *const u16) -> i32,
}

#[repr(C)]
pub struct IAvnRustVmSink2 {
    vtbl: *const IAvnRustVmSink2Vtbl,
}

unsafe impl ComInterface for IAvnRustVmSink2 {
    const IID: Guid = IAVN_RUST_VM_SINK2_IID;
}

impl ComPtr<IAvnRustVmSink2> {
    pub fn set_null(&self, property_id: i32) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().set_null)(
                self.as_raw(),
                property_id,
            ))
        }
    }
}

#[repr(C)]
struct IAvnRustVmSink3Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    submit_batch:
        unsafe extern "system" fn(*mut IAvnRustVmSink3, *mut IAvnRustVmUpdateBatch) -> i32,
}

#[repr(C)]
pub struct IAvnRustVmSink3 {
    vtbl: *const IAvnRustVmSink3Vtbl,
}

unsafe impl ComInterface for IAvnRustVmSink3 {
    const IID: Guid = IAVN_RUST_VM_SINK3_IID;
}

impl ComPtr<IAvnRustVmSink3> {
    pub fn submit_batch(&self, batch: &ComPtr<IAvnRustVmUpdateBatch>) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().submit_batch)(
                self.as_raw(),
                batch.as_raw(),
            ))
        }
    }
}

#[repr(C)]
struct IAvnRustVmSink4Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    map_set_string:
        unsafe extern "system" fn(*mut IAvnRustVmSink4, i32, *const u16, i64, *const u16) -> i32,
    map_set_integer:
        unsafe extern "system" fn(*mut IAvnRustVmSink4, i32, *const u16, i64, i64) -> i32,
    map_set_boolean:
        unsafe extern "system" fn(*mut IAvnRustVmSink4, i32, *const u16, i64, i32) -> i32,
    map_set_double:
        unsafe extern "system" fn(*mut IAvnRustVmSink4, i32, *const u16, i64, f64) -> i32,
    map_set_model: unsafe extern "system" fn(
        *mut IAvnRustVmSink4,
        i32,
        *const u16,
        i64,
        *mut IAvnRustViewModel,
    ) -> i32,
    map_remove: unsafe extern "system" fn(*mut IAvnRustVmSink4, i32, *const u16, i64) -> i32,
    map_clear: unsafe extern "system" fn(*mut IAvnRustVmSink4, i32) -> i32,
    set_command_progress:
        unsafe extern "system" fn(*mut IAvnRustVmSink4, i32, i32, f64, *const u16) -> i32,
    set_command_result:
        unsafe extern "system" fn(*mut IAvnRustVmSink4, i32, *mut IAvnRustViewModel) -> i32,
    set_command_running: unsafe extern "system" fn(*mut IAvnRustVmSink4, i32, i32) -> i32,
    publish_range:
        unsafe extern "system" fn(*mut IAvnRustVmSink4, *mut IAvnRustVmRangeBatch) -> i32,
}

#[repr(C)]
pub struct IAvnRustVmSink4 {
    vtbl: *const IAvnRustVmSink4Vtbl,
}

unsafe impl ComInterface for IAvnRustVmSink4 {
    const IID: Guid = IAVN_RUST_VM_SINK4_IID;
}

#[repr(C)]
struct IAvnRustVmSink5Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    add_integer: unsafe extern "system" fn(*mut IAvnRustVmSink5, i32, i64) -> i32,
    insert_integer: unsafe extern "system" fn(*mut IAvnRustVmSink5, i32, i32, i64) -> i32,
    replace_integer: unsafe extern "system" fn(*mut IAvnRustVmSink5, i32, i32, i64) -> i32,
    add_double: unsafe extern "system" fn(*mut IAvnRustVmSink5, i32, f64) -> i32,
    insert_double: unsafe extern "system" fn(*mut IAvnRustVmSink5, i32, i32, f64) -> i32,
    replace_double: unsafe extern "system" fn(*mut IAvnRustVmSink5, i32, i32, f64) -> i32,
}

#[repr(C)]
pub struct IAvnRustVmSink5 {
    vtbl: *const IAvnRustVmSink5Vtbl,
}

unsafe impl ComInterface for IAvnRustVmSink5 {
    const IID: Guid = IAVN_RUST_VM_SINK5_IID;
}

impl ComPtr<IAvnRustVmSink5> {
    pub fn add_integer(&self, collection_id: i32, value: i64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().add_integer)(
                self.as_raw(),
                collection_id,
                value,
            ))
        }
    }

    pub fn insert_integer(&self, collection_id: i32, index: i32, value: i64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().insert_integer)(
                self.as_raw(),
                collection_id,
                index,
                value,
            ))
        }
    }

    pub fn replace_integer(&self, collection_id: i32, index: i32, value: i64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().replace_integer)(
                self.as_raw(),
                collection_id,
                index,
                value,
            ))
        }
    }

    pub fn add_double(&self, collection_id: i32, value: f64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().add_double)(
                self.as_raw(),
                collection_id,
                value,
            ))
        }
    }

    pub fn insert_double(&self, collection_id: i32, index: i32, value: f64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().insert_double)(
                self.as_raw(),
                collection_id,
                index,
                value,
            ))
        }
    }

    pub fn replace_double(&self, collection_id: i32, index: i32, value: f64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().replace_double)(
                self.as_raw(),
                collection_id,
                index,
                value,
            ))
        }
    }
}

/// The transported form of a map key. The schema fixes which representation a
/// given map uses, so the generated named APIs never expose this to
/// application authors.
#[derive(Clone, Debug)]
pub enum MapKey {
    Text(Vec<u16>),
    Integer(i64),
}

impl MapKey {
    fn parts(&self) -> (Option<std::borrow::Cow<'_, [u16]>>, i64) {
        match self {
            Self::Text(value) => (Some(crate::terminated_utf16(value)), 0),
            Self::Integer(value) => (None, *value),
        }
    }
}

impl ComPtr<IAvnRustVmSink4> {
    pub fn map_set_string(&self, map_id: i32, key: &MapKey, value: &[u16]) -> Result<()> {
        let value = crate::terminated_utf16(value);
        let (text, integer) = key.parts();
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().map_set_string)(
                self.as_raw(),
                map_id,
                text.as_deref().map_or(ptr::null(), <[u16]>::as_ptr),
                integer,
                value.as_ptr(),
            ))
        }
    }

    pub fn map_set_integer(&self, map_id: i32, key: &MapKey, value: i64) -> Result<()> {
        let (text, integer) = key.parts();
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().map_set_integer)(
                self.as_raw(),
                map_id,
                text.as_deref().map_or(ptr::null(), <[u16]>::as_ptr),
                integer,
                value,
            ))
        }
    }

    pub fn map_set_boolean(&self, map_id: i32, key: &MapKey, value: bool) -> Result<()> {
        let (text, integer) = key.parts();
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().map_set_boolean)(
                self.as_raw(),
                map_id,
                text.as_deref().map_or(ptr::null(), <[u16]>::as_ptr),
                integer,
                i32::from(value),
            ))
        }
    }

    pub fn map_set_double(&self, map_id: i32, key: &MapKey, value: f64) -> Result<()> {
        let (text, integer) = key.parts();
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().map_set_double)(
                self.as_raw(),
                map_id,
                text.as_deref().map_or(ptr::null(), <[u16]>::as_ptr),
                integer,
                value,
            ))
        }
    }

    pub fn map_set_model(
        &self,
        map_id: i32,
        key: &MapKey,
        value: &ComPtr<IAvnRustViewModel>,
    ) -> Result<()> {
        let (text, integer) = key.parts();
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().map_set_model)(
                self.as_raw(),
                map_id,
                text.as_deref().map_or(ptr::null(), <[u16]>::as_ptr),
                integer,
                value.as_raw(),
            ))
        }
    }

    pub fn map_remove(&self, map_id: i32, key: &MapKey) -> Result<()> {
        let (text, integer) = key.parts();
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().map_remove)(
                self.as_raw(),
                map_id,
                text.as_deref().map_or(ptr::null(), <[u16]>::as_ptr),
                integer,
            ))
        }
    }

    pub fn map_clear(&self, map_id: i32) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().map_clear)(
                self.as_raw(),
                map_id,
            ))
        }
    }

    pub fn set_command_progress(
        &self,
        command_id: i32,
        value: Option<f64>,
        message: Option<&[u16]>,
    ) -> Result<()> {
        let message = message.map(crate::terminated_utf16);
        unsafe {
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .set_command_progress)(
                self.as_raw(),
                command_id,
                i32::from(value.is_some()),
                value.unwrap_or(0.0),
                message.as_deref().map_or(ptr::null(), <[u16]>::as_ptr),
            ))
        }
    }

    pub fn set_command_result(
        &self,
        command_id: i32,
        result: Option<&ComPtr<IAvnRustViewModel>>,
    ) -> Result<()> {
        unsafe {
            hresult::check(
                ((*self.as_raw()).vtbl.as_ref().unwrap().set_command_result)(
                    self.as_raw(),
                    command_id,
                    result.map_or(ptr::null_mut(), ComPtr::as_raw),
                ),
            )
        }
    }

    pub fn set_command_running(&self, command_id: i32, running: bool) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .set_command_running)(
                self.as_raw(), command_id, i32::from(running)
            ))
        }
    }

    pub fn publish_range(&self, batch: &ComPtr<IAvnRustVmRangeBatch>) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().publish_range)(
                self.as_raw(),
                batch.as_raw(),
            ))
        }
    }
}

impl ComPtr<IAvnRustVmUpdateBatch> {
    /// Reports a terminal outcome. Only managed code calls this in production;
    /// it is exposed so the Rust side can be exercised without a managed host.
    pub fn complete(&self, outcome: i32, error: i32) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().complete)(
                self.as_raw(),
                outcome,
                error,
            ))
        }
    }
}

impl ComPtr<IAvnRustVmUpdateBatch2> {
    /// Hands the batch's nested ownership to its producer. Managed code calls
    /// this exactly once, between the state commit and the notifications.
    pub fn commit_ownership(&self) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().commit_ownership)(
                self.as_raw(),
            ))
        }
    }
}

#[repr(C)]
pub struct IAvnRustVmUpdateBatch {
    vtbl: *const IAvnRustVmUpdateBatchVtbl,
}

unsafe impl ComInterface for IAvnRustVmUpdateBatch {
    const IID: Guid = IAVN_RUST_VM_UPDATE_BATCH_IID;
}

#[repr(C)]
pub struct IAvnRustVmUpdateOperation {
    vtbl: *const IAvnRustVmUpdateOperationVtbl,
}

unsafe impl ComInterface for IAvnRustVmUpdateOperation {
    const IID: Guid = IAVN_RUST_VM_UPDATE_OPERATION_IID;
}

/// The batch's ownership-commit interface. Deliberately a separate vtable from
/// [`IAvnRustVmUpdateBatch`]; the two are exposed by the same object.
#[repr(C)]
pub struct IAvnRustVmUpdateBatch2 {
    vtbl: *const IAvnRustVmUpdateBatch2Vtbl,
}

unsafe impl ComInterface for IAvnRustVmUpdateBatch2 {
    const IID: Guid = IAVN_RUST_VM_UPDATE_BATCH2_IID;
}

#[repr(C)]
struct IAvnRustVmUpdateBatch2Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    commit_ownership: unsafe extern "system" fn(*mut IAvnRustVmUpdateBatch2) -> i32,
}

#[repr(C)]
struct IAvnRustVmUpdateBatchVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_generation: unsafe extern "system" fn(*mut IAvnRustVmUpdateBatch, *mut i64) -> i32,
    get_operation_count: unsafe extern "system" fn(*mut IAvnRustVmUpdateBatch, *mut i32) -> i32,
    get_operation: unsafe extern "system" fn(
        *mut IAvnRustVmUpdateBatch,
        i32,
        *mut *mut IAvnRustVmUpdateOperation,
    ) -> i32,
    get_snapshot_item_count:
        unsafe extern "system" fn(*mut IAvnRustVmUpdateBatch, i32, *mut i32) -> i32,
    get_snapshot_string_length:
        unsafe extern "system" fn(*mut IAvnRustVmUpdateBatch, i32, i32, *mut i32) -> i32,
    copy_snapshot_string:
        unsafe extern "system" fn(*mut IAvnRustVmUpdateBatch, i32, i32, *mut u16, i32) -> i32,
    get_snapshot_model: unsafe extern "system" fn(
        *mut IAvnRustVmUpdateBatch,
        i32,
        i32,
        *mut *mut IAvnRustViewModel,
    ) -> i32,
    complete: unsafe extern "system" fn(*mut IAvnRustVmUpdateBatch, i32, i32) -> i32,
}

#[repr(C)]
struct IAvnRustVmUpdateOperationVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_kind: unsafe extern "system" fn(*mut IAvnRustVmUpdateOperation, *mut i32) -> i32,
    get_target_id: unsafe extern "system" fn(*mut IAvnRustVmUpdateOperation, *mut i32) -> i32,
    get_index: unsafe extern "system" fn(*mut IAvnRustVmUpdateOperation, *mut i32) -> i32,
    get_index2: unsafe extern "system" fn(*mut IAvnRustVmUpdateOperation, *mut i32) -> i32,
    get_integer: unsafe extern "system" fn(*mut IAvnRustVmUpdateOperation, *mut i64) -> i32,
    get_double: unsafe extern "system" fn(*mut IAvnRustVmUpdateOperation, *mut f64) -> i32,
    get_boolean: unsafe extern "system" fn(*mut IAvnRustVmUpdateOperation, *mut i32) -> i32,
    get_text_length: unsafe extern "system" fn(*mut IAvnRustVmUpdateOperation, *mut i32) -> i32,
    copy_text: unsafe extern "system" fn(*mut IAvnRustVmUpdateOperation, *mut u16, i32) -> i32,
    get_model: unsafe extern "system" fn(
        *mut IAvnRustVmUpdateOperation,
        *mut *mut IAvnRustViewModel,
    ) -> i32,
}

impl ComPtr<IAvnRustVmSink2> {
    pub fn set_model(
        &self,
        property_id: i32,
        model: Option<&ComPtr<IAvnRustViewModel>>,
    ) -> Result<()> {
        unsafe {
            let raw = model.map_or(ptr::null_mut(), ComPtr::as_raw);
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().set_model)(
                self.as_raw(),
                property_id,
                raw,
            ))
        }
    }

    pub fn add_model(&self, collection_id: i32, model: &ComPtr<IAvnRustViewModel>) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().add_model)(
                self.as_raw(),
                collection_id,
                model.as_raw(),
            ))
        }
    }

    pub fn insert_string(&self, collection_id: i32, index: i32, value: &[u16]) -> Result<()> {
        let value = crate::terminated_utf16(value);
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().insert_string)(
                self.as_raw(),
                collection_id,
                index,
                value.as_ptr(),
            ))
        }
    }

    pub fn insert_model(
        &self,
        collection_id: i32,
        index: i32,
        model: &ComPtr<IAvnRustViewModel>,
    ) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().insert_model)(
                self.as_raw(),
                collection_id,
                index,
                model.as_raw(),
            ))
        }
    }

    pub fn replace_string(&self, collection_id: i32, index: i32, value: &[u16]) -> Result<()> {
        let value = crate::terminated_utf16(value);
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().replace_string)(
                self.as_raw(),
                collection_id,
                index,
                value.as_ptr(),
            ))
        }
    }

    pub fn replace_model(
        &self,
        collection_id: i32,
        index: i32,
        model: &ComPtr<IAvnRustViewModel>,
    ) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().replace_model)(
                self.as_raw(),
                collection_id,
                index,
                model.as_raw(),
            ))
        }
    }

    pub fn remove_at(&self, collection_id: i32, index: i32) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().remove_at)(
                self.as_raw(),
                collection_id,
                index,
            ))
        }
    }

    pub fn move_item(&self, collection_id: i32, from_index: i32, to_index: i32) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().move_item)(
                self.as_raw(),
                collection_id,
                from_index,
                to_index,
            ))
        }
    }

    pub fn clear_collection(&self, collection_id: i32) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().clear_collection)(
                self.as_raw(),
                collection_id,
            ))
        }
    }

    pub fn set_command_enabled(&self, command_id: i32, enabled: bool) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .set_command_enabled)(
                self.as_raw(), command_id, i32::from(enabled)
            ))
        }
    }

    pub fn set_property_error(&self, property_id: i32, message: Option<&[u16]>) -> Result<()> {
        let message = message.map(crate::terminated_utf16);
        unsafe {
            let raw = message.as_deref().map_or(ptr::null(), <[u16]>::as_ptr);
            hresult::check(
                ((*self.as_raw()).vtbl.as_ref().unwrap().set_property_error)(
                    self.as_raw(),
                    property_id,
                    raw,
                ),
            )
        }
    }
}

pub struct RustViewModelCallbacks {
    pub attach: Box<dyn FnMut(ComPtr<IAvnRustVmSink>) -> Result<()> + Send>,
    pub detach: Box<dyn FnMut() -> Result<()> + Send>,
    pub set_string: Box<dyn FnMut(i32, String) -> Result<()> + Send>,
    pub set_integer: Box<dyn FnMut(i32, i64) -> Result<()> + Send>,
    pub set_boolean: Box<dyn FnMut(i32, bool) -> Result<()> + Send>,
    pub set_double: Box<dyn FnMut(i32, f64) -> Result<()> + Send>,
    pub execute: Box<dyn FnMut(i32, Option<String>) -> Result<()> + Send>,
    pub begin_async: Box<dyn FnMut(i32, Option<String>) -> Result<()> + Send>,
}

/// The collection ID and offset of a request the bounded range queue evicted
/// to make room, or an offset of -1. The queue is shared by every windowed
/// collection on a model, so the ID matters as much as the offset.
pub type RustVmDroppedRange = (i32, i64);

/// Stage 30 control callbacks. They are stored behind their *own* lock,
/// separate from `RustViewModelCallbacks`, precisely because managed code
/// invokes them on the UI thread while a Rust worker may be holding the
/// view-model lock. Implementations must only enqueue or flip a flag; they
/// must never run application model code inline.
pub struct RustViewModelControlCallbacks {
    /// Reads a windowed collection's current generation and total count.
    pub get_range_state: Box<dyn FnMut(i32) -> Result<(i64, i64)> + Send>,

    /// Enqueues a range request. Must return immediately. Returns the
    /// collection ID and offset of any request the bounded queue evicted to
    /// make room, or an offset of -1.
    pub request_range: Box<dyn FnMut(i32, i64, i32, i64) -> Result<RustVmDroppedRange> + Send>,

    /// Signals cancellation for one tracked async invocation. Must return immediately.
    pub cancel_async: Box<dyn FnMut(i32, i64) -> Result<()> + Send>,
}

impl Default for RustViewModelControlCallbacks {
    fn default() -> Self {
        Self {
            get_range_state: Box::new(|_| Err(hresult::Error(hresult::E_NOTIMPL))),
            request_range: Box::new(|_, _, _, _| Err(hresult::Error(hresult::E_NOTIMPL))),
            cancel_async: Box::new(|_, _| Ok(())),
        }
    }
}

/// Starts a tracked async invocation. Kept on the model-locked callback set
/// (like `begin_async`) because starting work legitimately touches the model.
pub type RustViewModelBeginTracked = Box<dyn FnMut(i32, Option<String>, i64) -> Result<()> + Send>;

#[repr(C)]
struct IAvnRustRangeSourceVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_range_state:
        unsafe extern "system" fn(*mut IAvnRustRangeSource, i32, *mut i64, *mut i64) -> i32,
    request_range: unsafe extern "system" fn(
        *mut IAvnRustRangeSource,
        i32,
        i64,
        i32,
        i64,
        *mut i32,
        *mut i64,
    ) -> i32,
}

#[repr(C)]
pub struct IAvnRustRangeSource {
    vtbl: *const IAvnRustRangeSourceVtbl,
}

unsafe impl ComInterface for IAvnRustRangeSource {
    const IID: Guid = IAVN_RUST_RANGE_SOURCE_IID;
}

impl ComPtr<IAvnRustRangeSource> {
    /// Reads a windowed collection's dataset identity. Only managed code calls
    /// this in production; it is exposed so the Rust side can be exercised
    /// without a managed host.
    pub fn get_range_state(&self, collection_id: i32) -> Result<(i64, i64)> {
        unsafe {
            let mut generation = 0i64;
            let mut total = 0i64;
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().get_range_state)(
                self.as_raw(),
                collection_id,
                &mut generation,
                &mut total,
            ))?;
            Ok((generation, total))
        }
    }

    /// Posts a range request. Returns as soon as the request is enqueued, and
    /// identifies any request the bounded queue evicted to make room (offset
    /// -1 when nothing was evicted).
    pub fn request_range(
        &self,
        collection_id: i32,
        offset: i64,
        length: i32,
        generation: i64,
    ) -> Result<(i32, i64)> {
        unsafe {
            let mut dropped_collection = 0i32;
            let mut dropped = -1i64;
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().request_range)(
                self.as_raw(),
                collection_id,
                offset,
                length,
                generation,
                &mut dropped_collection,
                &mut dropped,
            ))?;
            Ok((dropped_collection, dropped))
        }
    }
}

#[repr(C)]
struct IAvnRustViewModel2Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    begin_async_tracked:
        unsafe extern "system" fn(*mut IAvnRustViewModel2, i32, *const u16, *mut i64) -> i32,
    cancel_async: unsafe extern "system" fn(*mut IAvnRustViewModel2, i32, i64) -> i32,
}

#[repr(C)]
pub struct IAvnRustViewModel2 {
    vtbl: *const IAvnRustViewModel2Vtbl,
}

unsafe impl ComInterface for IAvnRustViewModel2 {
    const IID: Guid = IAVN_RUST_VIEW_MODEL2_IID;
}

impl ComPtr<IAvnRustViewModel2> {
    /// Starts a tracked async invocation and returns its never-reused handle.
    pub fn begin_async_tracked(&self, command_id: i32, parameter: Option<&[u16]>) -> Result<i64> {
        let parameter = parameter.map(crate::terminated_utf16);
        unsafe {
            let mut operation_id = 0i64;
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .begin_async_tracked)(
                self.as_raw(),
                command_id,
                parameter.as_deref().map_or(ptr::null(), <[u16]>::as_ptr),
                &mut operation_id,
            ))?;
            Ok(operation_id)
        }
    }

    /// Requests cancellation of one in-flight invocation. Never blocks.
    pub fn cancel_async(&self, command_id: i32, operation_id: i64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().cancel_async)(
                self.as_raw(),
                command_id,
                operation_id,
            ))
        }
    }
}

#[repr(C)]
struct IAvnRustViewModelVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    attach: unsafe extern "system" fn(*mut IAvnRustViewModel, *mut IAvnRustVmSink) -> i32,
    detach: unsafe extern "system" fn(*mut IAvnRustViewModel) -> i32,
    set_string: unsafe extern "system" fn(*mut IAvnRustViewModel, i32, *const u16) -> i32,
    set_integer: unsafe extern "system" fn(*mut IAvnRustViewModel, i32, i64) -> i32,
    set_boolean: unsafe extern "system" fn(*mut IAvnRustViewModel, i32, i32) -> i32,
    set_double: unsafe extern "system" fn(*mut IAvnRustViewModel, i32, f64) -> i32,
    execute: unsafe extern "system" fn(*mut IAvnRustViewModel, i32, *const u16) -> i32,
    begin_async: unsafe extern "system" fn(*mut IAvnRustViewModel, i32, *const u16) -> i32,
}

#[repr(C)]
pub struct IAvnRustViewModel {
    vtbl: *const IAvnRustViewModelVtbl,
}

unsafe impl ComInterface for IAvnRustViewModel {
    const IID: Guid = IAVN_RUST_VIEW_MODEL_IID;
}

#[repr(C)]
struct RustViewModelObject {
    interface: IAvnRustViewModel,
    range_interface: IAvnRustRangeSource,
    model2_interface: IAvnRustViewModel2,
    references: AtomicU32,
    callbacks: Mutex<RustViewModelCallbacks>,
    /// Deliberately a second lock: UI-thread control calls must never queue
    /// behind a Rust worker that is holding `callbacks`.
    control: Mutex<RustViewModelControlCallbacks>,
    begin_tracked: Mutex<Option<RustViewModelBeginTracked>>,
    next_operation: AtomicI64,
}

static RUST_VIEW_MODEL_VTBL: IAvnRustViewModelVtbl = IAvnRustViewModelVtbl {
    query_interface,
    add_ref,
    release,
    attach,
    detach,
    set_string,
    set_integer,
    set_boolean,
    set_double,
    execute,
    begin_async,
};

static RUST_RANGE_SOURCE_VTBL: IAvnRustRangeSourceVtbl = IAvnRustRangeSourceVtbl {
    query_interface: range_query_interface,
    add_ref: range_add_ref,
    release: range_release,
    get_range_state,
    request_range,
};

static RUST_VIEW_MODEL2_VTBL: IAvnRustViewModel2Vtbl = IAvnRustViewModel2Vtbl {
    query_interface: model2_query_interface,
    add_ref: model2_add_ref,
    release: model2_release,
    begin_async_tracked,
    cancel_async,
};

pub fn rust_view_model(callbacks: RustViewModelCallbacks) -> ComPtr<IAvnRustViewModel> {
    rust_view_model_with_control(callbacks, RustViewModelControlCallbacks::default(), None)
}

/// Creates a view model that also exposes the stage 30 range-source and
/// tracked-async capabilities. Both are separate IIDs on the same object and
/// share its single reference count; the published `IAvnRustViewModel` vtable
/// is untouched.
pub fn rust_view_model_with_control(
    callbacks: RustViewModelCallbacks,
    control: RustViewModelControlCallbacks,
    begin_tracked: Option<RustViewModelBeginTracked>,
) -> ComPtr<IAvnRustViewModel> {
    let object = Box::new(RustViewModelObject {
        interface: IAvnRustViewModel {
            vtbl: &RUST_VIEW_MODEL_VTBL,
        },
        range_interface: IAvnRustRangeSource {
            vtbl: &RUST_RANGE_SOURCE_VTBL,
        },
        model2_interface: IAvnRustViewModel2 {
            vtbl: &RUST_VIEW_MODEL2_VTBL,
        },
        references: AtomicU32::new(1),
        callbacks: Mutex::new(callbacks),
        control: Mutex::new(control),
        begin_tracked: Mutex::new(begin_tracked),
        next_operation: AtomicI64::new(1),
    });
    unsafe {
        ComPtr::from_raw(Box::into_raw(object).cast())
            .expect("Box allocation cannot produce a null pointer")
    }
}

unsafe fn model_from_range(this: *mut c_void) -> *mut RustViewModelObject {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(RustViewModelObject, range_interface))
        .cast::<RustViewModelObject>()
}

unsafe fn model_from_model2(this: *mut c_void) -> *mut RustViewModelObject {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(RustViewModelObject, model2_interface))
        .cast::<RustViewModelObject>()
}

unsafe fn model_query(
    object: *mut RustViewModelObject,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    *result = ptr::null_mut();
    if *iid == Guid::IUNKNOWN || *iid == IAVN_RUST_VIEW_MODEL_IID {
        (*object).references.fetch_add(1, Ordering::Relaxed);
        *result = ptr::addr_of_mut!((*object).interface).cast();
        return hresult::S_OK;
    }
    if *iid == IAVN_RUST_RANGE_SOURCE_IID {
        (*object).references.fetch_add(1, Ordering::Relaxed);
        *result = ptr::addr_of_mut!((*object).range_interface).cast();
        return hresult::S_OK;
    }
    if *iid == IAVN_RUST_VIEW_MODEL2_IID {
        (*object).references.fetch_add(1, Ordering::Relaxed);
        *result = ptr::addr_of_mut!((*object).model2_interface).cast();
        return hresult::S_OK;
    }
    hresult::E_NOINTERFACE
}

unsafe extern "system" fn query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if this.is_null() || iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    model_query(this.cast::<RustViewModelObject>(), iid, result)
}

unsafe extern "system" fn range_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if this.is_null() || iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    model_query(model_from_range(this.cast()), iid, result)
}

unsafe extern "system" fn model2_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if this.is_null() || iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    model_query(model_from_model2(this.cast()), iid, result)
}

unsafe extern "system" fn add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<RustViewModelObject>();
    (*object).references.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn range_add_ref(this: *mut IUnknown) -> u32 {
    add_ref(model_from_range(this.cast()).cast())
}

unsafe extern "system" fn model2_add_ref(this: *mut IUnknown) -> u32 {
    add_ref(model_from_model2(this.cast()).cast())
}

unsafe extern "system" fn release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<RustViewModelObject>();
    let remaining = (*object).references.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn range_release(this: *mut IUnknown) -> u32 {
    release(model_from_range(this.cast()).cast())
}

unsafe extern "system" fn model2_release(this: *mut IUnknown) -> u32 {
    release(model_from_model2(this.cast()).cast())
}

/// Reads a windowed collection's identity. Takes only the control lock, so it
/// cannot queue behind a Rust worker that holds the view-model lock.
unsafe extern "system" fn get_range_state(
    this: *mut IAvnRustRangeSource,
    collection_id: i32,
    generation: *mut i64,
    total_count: *mut i64,
) -> i32 {
    ffi(|| {
        if this.is_null() || generation.is_null() || total_count.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        let object = model_from_range(this.cast());
        let mut control = (*object)
            .control
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?;
        let (current, total) = (control.get_range_state)(collection_id)?;
        *generation = current;
        *total_count = total;
        Ok(())
    })
}

/// Enqueues a range request. Never produces a range inline and never blocks.
/// Reports the offset of any request the bounded queue evicted to make room,
/// so managed code stops waiting for a batch that will never arrive.
unsafe extern "system" fn request_range(
    this: *mut IAvnRustRangeSource,
    collection_id: i32,
    offset: i64,
    length: i32,
    generation: i64,
    dropped_collection_id: *mut i32,
    dropped_offset: *mut i64,
) -> i32 {
    ffi(|| {
        if this.is_null() || dropped_collection_id.is_null() || dropped_offset.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        *dropped_collection_id = 0;
        *dropped_offset = -1;
        if offset < 0 || length <= 0 {
            return Err(hresult::Error(hresult::E_INVALIDARG));
        }
        let object = model_from_range(this.cast());
        let mut control = (*object)
            .control
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?;
        let (dropped_collection, dropped) =
            (control.request_range)(collection_id, offset, length, generation)?;
        *dropped_collection_id = dropped_collection;
        *dropped_offset = dropped;
        Ok(())
    })
}

/// Starts one tracked async invocation and hands back its never-reused handle.
unsafe extern "system" fn begin_async_tracked(
    this: *mut IAvnRustViewModel2,
    command_id: i32,
    parameter: *const u16,
    operation_id: *mut i64,
) -> i32 {
    if this.is_null() || operation_id.is_null() {
        return hresult::E_POINTER;
    }
    *operation_id = 0;
    let parameter = crate::clone_utf16(parameter);
    let object = model_from_model2(this.cast());
    let handle = (*object).next_operation.fetch_add(1, Ordering::Relaxed);
    let result = ffi(|| {
        let mut tracked = (*object)
            .begin_tracked
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?;
        if let Some(begin) = tracked.as_mut() {
            return begin(command_id, parameter, handle);
        }
        drop(tracked);
        // No tracked entry point: fall back to the untracked one so a producer
        // that only supports stage 27 async still runs, simply without
        // cancellation.
        let mut callbacks = (*object)
            .callbacks
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?;
        (callbacks.begin_async)(command_id, parameter)
    });
    if result >= 0 {
        *operation_id = handle;
    }
    result
}

/// Signals cancellation. Takes only the control lock, so cancelling never
/// blocks the UI thread behind the running worker it is cancelling.
unsafe extern "system" fn cancel_async(
    this: *mut IAvnRustViewModel2,
    command_id: i32,
    operation_id: i64,
) -> i32 {
    ffi(|| {
        if this.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        let object = model_from_model2(this.cast());
        let mut control = (*object)
            .control
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?;
        (control.cancel_async)(command_id, operation_id)
    })
}

unsafe extern "system" fn attach(this: *mut IAvnRustViewModel, sink: *mut IAvnRustVmSink) -> i32 {
    let Some(sink) = ComPtr::from_borrowed(sink) else {
        return hresult::E_POINTER;
    };
    invoke(this, |callbacks| (callbacks.attach)(sink))
}

unsafe extern "system" fn detach(this: *mut IAvnRustViewModel) -> i32 {
    invoke(this, |callbacks| (callbacks.detach)())
}

unsafe extern "system" fn set_string(
    this: *mut IAvnRustViewModel,
    property_id: i32,
    value: *const u16,
) -> i32 {
    let value = crate::clone_utf16(value).unwrap_or_default();
    invoke(this, |callbacks| (callbacks.set_string)(property_id, value))
}

unsafe extern "system" fn set_integer(
    this: *mut IAvnRustViewModel,
    property_id: i32,
    value: i64,
) -> i32 {
    invoke(this, |callbacks| {
        (callbacks.set_integer)(property_id, value)
    })
}

unsafe extern "system" fn set_boolean(
    this: *mut IAvnRustViewModel,
    property_id: i32,
    value: i32,
) -> i32 {
    invoke(this, |callbacks| {
        (callbacks.set_boolean)(property_id, value != 0)
    })
}

unsafe extern "system" fn set_double(
    this: *mut IAvnRustViewModel,
    property_id: i32,
    value: f64,
) -> i32 {
    invoke(this, |callbacks| (callbacks.set_double)(property_id, value))
}

unsafe extern "system" fn execute(
    this: *mut IAvnRustViewModel,
    command_id: i32,
    parameter: *const u16,
) -> i32 {
    let parameter = crate::clone_utf16(parameter);
    invoke(this, |callbacks| (callbacks.execute)(command_id, parameter))
}

unsafe extern "system" fn begin_async(
    this: *mut IAvnRustViewModel,
    command_id: i32,
    parameter: *const u16,
) -> i32 {
    let parameter = crate::clone_utf16(parameter);
    invoke(this, |callbacks| {
        (callbacks.begin_async)(command_id, parameter)
    })
}

unsafe fn invoke(
    this: *mut IAvnRustViewModel,
    callback: impl FnOnce(&mut RustViewModelCallbacks) -> Result<()>,
) -> i32 {
    let object = this.cast::<RustViewModelObject>();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut callbacks = (*object)
            .callbacks
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?;
        callback(&mut callbacks)
    }));
    match result {
        Ok(Ok(())) => hresult::S_OK,
        Ok(Err(error)) => error.0,
        Err(_) => hresult::E_FAIL,
    }
}

/// Immutable update operation used by the v3 batch transport. The numeric tags
/// intentionally match `Avalonia.Rust.RustVmUpdateKind`; unknown tags are
/// rejected by managed code before it mutates presentation state.
#[derive(Clone)]
pub struct RustVmUpdate {
    pub kind: i32,
    pub target_id: i32,
    pub index: i32,
    pub index2: i32,
    pub integer: i64,
    pub double: f64,
    pub boolean: i32,
    pub text: Option<String>,
    pub model: Option<ComPtr<IAvnRustViewModel>>,
    pub snapshot_strings: Option<Vec<String>>,
    pub snapshot_models: Option<Vec<ComPtr<IAvnRustViewModel>>>,
}

impl RustVmUpdate {
    pub fn new(kind: i32, target_id: i32) -> Self {
        Self {
            kind,
            target_id,
            index: 0,
            index2: 0,
            integer: 0,
            double: 0.0,
            boolean: 0,
            text: None,
            model: None,
            snapshot_strings: None,
            snapshot_models: None,
        }
    }
}

pub type RustVmBatchCompletion = Box<dyn FnOnce(i32, i32) + Send>;

/// Invoked exactly once, by managed code, after every notification-free state
/// store and before any notification is published. This is where the batch's
/// nested ownership is handed to the producer.
pub type RustVmBatchOwnershipCommit = Box<dyn FnOnce() + Send>;

struct RustVmBatchState {
    generation: i64,
    operations: Vec<RustVmUpdate>,
    ownership: Mutex<Option<RustVmBatchOwnershipCommit>>,
    completion: Mutex<Option<RustVmBatchCompletion>>,
}

/// One allocation exposing two interfaces: the immutable batch itself and the
/// separately versioned ownership-commit capability. Both share the object's
/// single reference count.
#[repr(C)]
struct RustVmBatchObject {
    interface: IAvnRustVmUpdateBatch,
    ownership_interface: IAvnRustVmUpdateBatch2,
    references: AtomicU32,
    state: Arc<RustVmBatchState>,
}

#[repr(C)]
struct RustVmBatchOperationObject {
    interface: IAvnRustVmUpdateOperation,
    references: AtomicU32,
    state: Arc<RustVmBatchState>,
    index: usize,
}

static RUST_VM_BATCH_VTBL: IAvnRustVmUpdateBatchVtbl = IAvnRustVmUpdateBatchVtbl {
    query_interface: batch_query_interface,
    add_ref: batch_add_ref,
    release: batch_release,
    get_generation: batch_get_generation,
    get_operation_count: batch_get_operation_count,
    get_operation: batch_get_operation,
    get_snapshot_item_count: batch_get_snapshot_item_count,
    get_snapshot_string_length: batch_get_snapshot_string_length,
    copy_snapshot_string: batch_copy_snapshot_string,
    get_snapshot_model: batch_get_snapshot_model,
    complete: batch_complete,
};

static RUST_VM_BATCH_OWNERSHIP_VTBL: IAvnRustVmUpdateBatch2Vtbl = IAvnRustVmUpdateBatch2Vtbl {
    query_interface: ownership_query_interface,
    add_ref: ownership_add_ref,
    release: ownership_release,
    commit_ownership: batch_commit_ownership,
};

static RUST_VM_BATCH_OPERATION_VTBL: IAvnRustVmUpdateOperationVtbl =
    IAvnRustVmUpdateOperationVtbl {
        query_interface: operation_query_interface,
        add_ref: operation_add_ref,
        release: operation_release,
        get_kind: operation_get_kind,
        get_target_id: operation_get_target_id,
        get_index: operation_get_index,
        get_index2: operation_get_index2,
        get_integer: operation_get_integer,
        get_double: operation_get_double,
        get_boolean: operation_get_boolean,
        get_text_length: operation_get_text_length,
        copy_text: operation_copy_text,
        get_model: operation_get_model,
    };

/// Creates an immutable nano-COM batch exposing both the batch interface and
/// its separately versioned ownership-commit capability. `ownership` is taken
/// exactly once, by managed code, between the state commit and the
/// notifications; `completion` is taken exactly once after apply/stale/cancel/
/// error. Neither is ever invoked by SubmitBatch. Dropping the batch without an
/// ownership commit simply drops the callback (and everything it owns).
pub fn rust_vm_update_batch(
    generation: i64,
    operations: Vec<RustVmUpdate>,
    ownership: Option<RustVmBatchOwnershipCommit>,
    completion: Option<RustVmBatchCompletion>,
) -> ComPtr<IAvnRustVmUpdateBatch> {
    let object = Box::new(RustVmBatchObject {
        interface: IAvnRustVmUpdateBatch {
            vtbl: &RUST_VM_BATCH_VTBL,
        },
        ownership_interface: IAvnRustVmUpdateBatch2 {
            vtbl: &RUST_VM_BATCH_OWNERSHIP_VTBL,
        },
        references: AtomicU32::new(1),
        state: Arc::new(RustVmBatchState {
            generation,
            operations,
            ownership: Mutex::new(ownership),
            completion: Mutex::new(completion),
        }),
    });
    unsafe {
        ComPtr::from_raw(Box::into_raw(object).cast())
            .expect("Box allocation cannot produce a null pointer")
    }
}

/// Resolves the owning object from either of the two interface pointers it
/// exposes.
unsafe fn batch_object_from_ownership(this: *mut c_void) -> *mut RustVmBatchObject {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(RustVmBatchObject, ownership_interface))
        .cast::<RustVmBatchObject>()
}

unsafe fn batch_query(
    object: *mut RustVmBatchObject,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    *result = ptr::null_mut();
    if *iid == Guid::IUNKNOWN || *iid == IAVN_RUST_VM_UPDATE_BATCH_IID {
        (*object).references.fetch_add(1, Ordering::Relaxed);
        *result = ptr::addr_of_mut!((*object).interface).cast();
        return hresult::S_OK;
    }
    if *iid == IAVN_RUST_VM_UPDATE_BATCH2_IID {
        (*object).references.fetch_add(1, Ordering::Relaxed);
        *result = ptr::addr_of_mut!((*object).ownership_interface).cast();
        return hresult::S_OK;
    }
    hresult::E_NOINTERFACE
}

unsafe extern "system" fn batch_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if this.is_null() || iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    batch_query(this.cast::<RustVmBatchObject>(), iid, result)
}

unsafe extern "system" fn ownership_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if this.is_null() || iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    batch_query(batch_object_from_ownership(this.cast()), iid, result)
}

unsafe extern "system" fn batch_add_ref(this: *mut IUnknown) -> u32 {
    (*this.cast::<RustVmBatchObject>())
        .references
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

unsafe extern "system" fn ownership_add_ref(this: *mut IUnknown) -> u32 {
    batch_add_ref(batch_object_from_ownership(this.cast()).cast())
}

unsafe extern "system" fn batch_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<RustVmBatchObject>();
    let remaining = (*object).references.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn ownership_release(this: *mut IUnknown) -> u32 {
    batch_release(batch_object_from_ownership(this.cast()).cast())
}

/// Hands the batch's nested ownership to the producer. The callback is taken
/// under the state lock, so it runs at most once no matter how many times
/// managed code calls this.
unsafe extern "system" fn batch_commit_ownership(this: *mut IAvnRustVmUpdateBatch2) -> i32 {
    ffi(|| {
        if this.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        let state = (*batch_object_from_ownership(this.cast())).state.clone();
        let ownership = state
            .ownership
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?
            .take();
        if let Some(ownership) = ownership {
            ownership();
        }
        Ok(())
    })
}

unsafe fn batch_state(this: *mut IAvnRustVmUpdateBatch) -> Result<Arc<RustVmBatchState>> {
    if this.is_null() {
        return Err(hresult::Error(hresult::E_POINTER));
    }
    Ok((*this.cast::<RustVmBatchObject>()).state.clone())
}

unsafe extern "system" fn batch_get_generation(
    this: *mut IAvnRustVmUpdateBatch,
    result: *mut i64,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        *result = batch_state(this)?.generation;
        Ok(())
    })
}

unsafe extern "system" fn batch_get_operation_count(
    this: *mut IAvnRustVmUpdateBatch,
    result: *mut i32,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        *result = i32::try_from(batch_state(this)?.operations.len())
            .map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
        Ok(())
    })
}

unsafe extern "system" fn batch_get_operation(
    this: *mut IAvnRustVmUpdateBatch,
    index: i32,
    result: *mut *mut IAvnRustVmUpdateOperation,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        *result = ptr::null_mut();
        let state = batch_state(this)?;
        let index = usize::try_from(index).map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
        if index >= state.operations.len() {
            return Err(hresult::Error(hresult::E_INVALIDARG));
        }
        let operation = Box::new(RustVmBatchOperationObject {
            interface: IAvnRustVmUpdateOperation {
                vtbl: &RUST_VM_BATCH_OPERATION_VTBL,
            },
            references: AtomicU32::new(1),
            state,
            index,
        });
        *result = Box::into_raw(operation).cast();
        Ok(())
    })
}

unsafe fn snapshot_operation(
    this: *mut IAvnRustVmUpdateBatch,
    operation: i32,
) -> Result<(Arc<RustVmBatchState>, usize)> {
    let state = batch_state(this)?;
    let index = usize::try_from(operation).map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
    if index >= state.operations.len() {
        return Err(hresult::Error(hresult::E_INVALIDARG));
    }
    Ok((state, index))
}

unsafe extern "system" fn batch_get_snapshot_item_count(
    this: *mut IAvnRustVmUpdateBatch,
    operation: i32,
    result: *mut i32,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        let (state, index) = snapshot_operation(this, operation)?;
        let update = &state.operations[index];
        let length = update
            .snapshot_strings
            .as_ref()
            .map(Vec::len)
            .or_else(|| update.snapshot_models.as_ref().map(Vec::len))
            .ok_or(hresult::Error(hresult::E_INVALIDARG))?;
        *result = i32::try_from(length).map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
        Ok(())
    })
}

unsafe extern "system" fn batch_get_snapshot_string_length(
    this: *mut IAvnRustVmUpdateBatch,
    operation: i32,
    item: i32,
    result: *mut i32,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        let (state, index) = snapshot_operation(this, operation)?;
        let item = usize::try_from(item).map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
        let value = state.operations[index]
            .snapshot_strings
            .as_ref()
            .and_then(|items| items.get(item))
            .ok_or(hresult::Error(hresult::E_INVALIDARG))?;
        *result = i32::try_from(value.encode_utf16().count())
            .map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
        Ok(())
    })
}

unsafe extern "system" fn batch_copy_snapshot_string(
    this: *mut IAvnRustVmUpdateBatch,
    operation: i32,
    item: i32,
    destination: *mut u16,
    capacity: i32,
) -> i32 {
    ffi(|| {
        let (state, index) = snapshot_operation(this, operation)?;
        let item = usize::try_from(item).map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
        let value = state.operations[index]
            .snapshot_strings
            .as_ref()
            .and_then(|items| items.get(item))
            .ok_or(hresult::Error(hresult::E_INVALIDARG))?;
        copy_utf16(value, destination, capacity)
    })
}

unsafe extern "system" fn batch_get_snapshot_model(
    this: *mut IAvnRustVmUpdateBatch,
    operation: i32,
    item: i32,
    result: *mut *mut IAvnRustViewModel,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        *result = ptr::null_mut();
        let (state, index) = snapshot_operation(this, operation)?;
        let item = usize::try_from(item).map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
        let model = state.operations[index]
            .snapshot_models
            .as_ref()
            .and_then(|items| items.get(item))
            .ok_or(hresult::Error(hresult::E_INVALIDARG))?;
        add_ref(model.as_raw().cast());
        *result = model.as_raw();
        Ok(())
    })
}

unsafe extern "system" fn batch_complete(
    this: *mut IAvnRustVmUpdateBatch,
    outcome: i32,
    error: i32,
) -> i32 {
    ffi(|| {
        let state = batch_state(this)?;
        let completion = state
            .completion
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?
            .take();
        if let Some(completion) = completion {
            completion(outcome, error);
        }
        Ok(())
    })
}

unsafe extern "system" fn operation_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if this.is_null() || iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    *result = ptr::null_mut();
    if *iid != Guid::IUNKNOWN && *iid != IAVN_RUST_VM_UPDATE_OPERATION_IID {
        return hresult::E_NOINTERFACE;
    }
    operation_add_ref(this);
    *result = this.cast();
    hresult::S_OK
}

unsafe extern "system" fn operation_add_ref(this: *mut IUnknown) -> u32 {
    (*this.cast::<RustVmBatchOperationObject>())
        .references
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

unsafe extern "system" fn operation_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<RustVmBatchOperationObject>();
    let remaining = (*object).references.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe fn operation(this: *mut IAvnRustVmUpdateOperation) -> Result<Arc<RustVmBatchState>> {
    if this.is_null() {
        return Err(hresult::Error(hresult::E_POINTER));
    }
    Ok((*this.cast::<RustVmBatchOperationObject>()).state.clone())
}
unsafe fn operation_value(
    this: *mut IAvnRustVmUpdateOperation,
) -> Result<(Arc<RustVmBatchState>, usize)> {
    let object = this.cast::<RustVmBatchOperationObject>();
    let state = operation(this)?;
    Ok((state, (*object).index))
}
macro_rules! operation_get {
    ($name:ident, $type:ty, $field:ident) => {
        unsafe extern "system" fn $name(
            this: *mut IAvnRustVmUpdateOperation,
            result: *mut $type,
        ) -> i32 {
            ffi(|| {
                if result.is_null() {
                    return Err(hresult::Error(hresult::E_POINTER));
                }
                let (state, index) = operation_value(this)?;
                *result = state.operations[index].$field;
                Ok(())
            })
        }
    };
}
operation_get!(operation_get_kind, i32, kind);
operation_get!(operation_get_target_id, i32, target_id);
operation_get!(operation_get_index, i32, index);
operation_get!(operation_get_index2, i32, index2);
operation_get!(operation_get_integer, i64, integer);
operation_get!(operation_get_double, f64, double);
operation_get!(operation_get_boolean, i32, boolean);

unsafe extern "system" fn operation_get_text_length(
    this: *mut IAvnRustVmUpdateOperation,
    result: *mut i32,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        let (state, index) = operation_value(this)?;
        *result = match &state.operations[index].text {
            Some(value) => i32::try_from(value.encode_utf16().count())
                .map_err(|_| hresult::Error(hresult::E_INVALIDARG))?,
            None => 0,
        };
        Ok(())
    })
}

unsafe fn copy_utf16(value: &str, destination: *mut u16, capacity: i32) -> Result<()> {
    if destination.is_null() || capacity < 0 {
        return Err(hresult::Error(hresult::E_POINTER));
    }
    let units: Vec<u16> = value.encode_utf16().collect();
    if usize::try_from(capacity).map_err(|_| hresult::Error(hresult::E_INVALIDARG))?
        < units.len() + 1
    {
        return Err(hresult::Error(hresult::E_INVALIDARG));
    }
    ptr::copy_nonoverlapping(units.as_ptr(), destination, units.len());
    *destination.add(units.len()) = 0;
    Ok(())
}

unsafe extern "system" fn operation_copy_text(
    this: *mut IAvnRustVmUpdateOperation,
    destination: *mut u16,
    capacity: i32,
) -> i32 {
    ffi(|| {
        let (state, index) = operation_value(this)?;
        copy_utf16(
            state.operations[index].text.as_deref().unwrap_or(""),
            destination,
            capacity,
        )
    })
}

unsafe extern "system" fn operation_get_model(
    this: *mut IAvnRustVmUpdateOperation,
    result: *mut *mut IAvnRustViewModel,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        *result = ptr::null_mut();
        let (state, index) = operation_value(this)?;
        if let Some(model) = &state.operations[index].model {
            add_ref(model.as_raw().cast());
            *result = model.as_raw();
        }
        Ok(())
    })
}

unsafe fn ffi(action: impl FnOnce() -> Result<()>) -> i32 {
    match catch_unwind(AssertUnwindSafe(action)) {
        Ok(Ok(())) => hresult::S_OK,
        Ok(Err(error)) => error.0,
        Err(_) => hresult::E_FAIL,
    }
}

/// Wire tag for a range batch: `Reset` republishes the dataset identity
/// (generation and total count), `Fill` realizes one page of it. They are
/// distinct kinds rather than an implicit "empty range means reset" so a
/// legitimately empty page can never be mistaken for a reset.
pub const RUST_VM_RANGE_RESET: i32 = 0;
pub const RUST_VM_RANGE_FILL: i32 = 1;
pub const RUST_VM_RANGE_INVALIDATE: i32 = 2;

/// One element of a realized range. Exactly one of the two is set: a string
/// window carries text, a nested-model window carries the element's model.
#[derive(Clone)]
pub struct RustVmRangeItem {
    pub text: Option<String>,
    pub model: Option<ComPtr<IAvnRustViewModel>>,
}

struct RustVmRangeState {
    kind: i32,
    collection_id: i32,
    generation: i64,
    total_count: i64,
    offset: i64,
    items: Vec<RustVmRangeItem>,
    completion: Mutex<Option<RustVmBatchCompletion>>,
}

#[repr(C)]
struct RustVmRangeObject {
    interface: IAvnRustVmRangeBatch,
    references: AtomicU32,
    state: Arc<RustVmRangeState>,
}

#[repr(C)]
struct IAvnRustVmRangeBatchVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_kind: unsafe extern "system" fn(*mut IAvnRustVmRangeBatch, *mut i32) -> i32,
    get_collection_id: unsafe extern "system" fn(*mut IAvnRustVmRangeBatch, *mut i32) -> i32,
    get_generation: unsafe extern "system" fn(*mut IAvnRustVmRangeBatch, *mut i64) -> i32,
    get_total_count: unsafe extern "system" fn(*mut IAvnRustVmRangeBatch, *mut i64) -> i32,
    get_offset: unsafe extern "system" fn(*mut IAvnRustVmRangeBatch, *mut i64) -> i32,
    get_item_count: unsafe extern "system" fn(*mut IAvnRustVmRangeBatch, *mut i32) -> i32,
    get_item_model: unsafe extern "system" fn(
        *mut IAvnRustVmRangeBatch,
        i32,
        *mut *mut IAvnRustViewModel,
    ) -> i32,
    get_item_string_length:
        unsafe extern "system" fn(*mut IAvnRustVmRangeBatch, i32, *mut i32) -> i32,
    copy_item_string:
        unsafe extern "system" fn(*mut IAvnRustVmRangeBatch, i32, *mut u16, i32) -> i32,
    complete: unsafe extern "system" fn(*mut IAvnRustVmRangeBatch, i32, i32) -> i32,
}

#[repr(C)]
pub struct IAvnRustVmRangeBatch {
    vtbl: *const IAvnRustVmRangeBatchVtbl,
}

unsafe impl ComInterface for IAvnRustVmRangeBatch {
    const IID: Guid = IAVN_RUST_VM_RANGE_BATCH_IID;
}

static RUST_VM_RANGE_VTBL: IAvnRustVmRangeBatchVtbl = IAvnRustVmRangeBatchVtbl {
    query_interface: range_batch_query_interface,
    add_ref: range_batch_add_ref,
    release: range_batch_release,
    get_kind: range_get_kind,
    get_collection_id: range_get_collection_id,
    get_generation: range_get_generation,
    get_total_count: range_get_total_count,
    get_offset: range_get_offset,
    get_item_count: range_get_item_count,
    get_item_model: range_get_item_model,
    get_item_string_length: range_get_item_string_length,
    copy_item_string: range_copy_item_string,
    complete: range_complete,
};

/// Creates an immutable nano-COM range batch. `completion` is taken exactly
/// once after managed code reports applied/stale/cancelled/error; dropping the
/// batch without a completion simply drops the callback and every element
/// model it owns, which is what keeps a rejected range from leaking adapters.
#[allow(clippy::too_many_arguments)]
pub fn rust_vm_range_batch(
    kind: i32,
    collection_id: i32,
    generation: i64,
    total_count: i64,
    offset: i64,
    items: Vec<RustVmRangeItem>,
    completion: Option<RustVmBatchCompletion>,
) -> ComPtr<IAvnRustVmRangeBatch> {
    let object = Box::new(RustVmRangeObject {
        interface: IAvnRustVmRangeBatch {
            vtbl: &RUST_VM_RANGE_VTBL,
        },
        references: AtomicU32::new(1),
        state: Arc::new(RustVmRangeState {
            kind,
            collection_id,
            generation,
            total_count,
            offset,
            items,
            completion: Mutex::new(completion),
        }),
    });
    unsafe {
        ComPtr::from_raw(Box::into_raw(object).cast())
            .expect("Box allocation cannot produce a null pointer")
    }
}

impl ComPtr<IAvnRustVmRangeBatch> {
    /// Reports a terminal outcome. Only managed code calls this in production;
    /// it is exposed so the Rust side can be exercised without a managed host.
    pub fn complete(&self, outcome: i32, error: i32) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw()).vtbl.as_ref().unwrap().complete)(
                self.as_raw(),
                outcome,
                error,
            ))
        }
    }
}

unsafe extern "system" fn range_batch_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if this.is_null() || iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    *result = ptr::null_mut();
    if *iid != Guid::IUNKNOWN && *iid != IAVN_RUST_VM_RANGE_BATCH_IID {
        return hresult::E_NOINTERFACE;
    }
    range_batch_add_ref(this);
    *result = this.cast();
    hresult::S_OK
}

unsafe extern "system" fn range_batch_add_ref(this: *mut IUnknown) -> u32 {
    (*this.cast::<RustVmRangeObject>())
        .references
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

unsafe extern "system" fn range_batch_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<RustVmRangeObject>();
    let remaining = (*object).references.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe fn range_state(this: *mut IAvnRustVmRangeBatch) -> Result<Arc<RustVmRangeState>> {
    if this.is_null() {
        return Err(hresult::Error(hresult::E_POINTER));
    }
    Ok((*this.cast::<RustVmRangeObject>()).state.clone())
}

macro_rules! range_get {
    ($name:ident, $type:ty, $field:ident) => {
        unsafe extern "system" fn $name(
            this: *mut IAvnRustVmRangeBatch,
            result: *mut $type,
        ) -> i32 {
            ffi(|| {
                if result.is_null() {
                    return Err(hresult::Error(hresult::E_POINTER));
                }
                *result = range_state(this)?.$field;
                Ok(())
            })
        }
    };
}
range_get!(range_get_kind, i32, kind);
range_get!(range_get_collection_id, i32, collection_id);
range_get!(range_get_generation, i64, generation);
range_get!(range_get_total_count, i64, total_count);
range_get!(range_get_offset, i64, offset);

unsafe extern "system" fn range_get_item_count(
    this: *mut IAvnRustVmRangeBatch,
    result: *mut i32,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        *result = i32::try_from(range_state(this)?.items.len())
            .map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
        Ok(())
    })
}

unsafe fn range_item_index(this: *mut IAvnRustVmRangeBatch, index: i32) -> Result<usize> {
    let state = range_state(this)?;
    let index = usize::try_from(index).map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
    if index >= state.items.len() {
        return Err(hresult::Error(hresult::E_INVALIDARG));
    }
    Ok(index)
}

unsafe extern "system" fn range_get_item_model(
    this: *mut IAvnRustVmRangeBatch,
    index: i32,
    result: *mut *mut IAvnRustViewModel,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        *result = ptr::null_mut();
        let index = range_item_index(this, index)?;
        let state = range_state(this)?;
        if let Some(model) = &state.items[index].model {
            add_ref(model.as_raw().cast());
            *result = model.as_raw();
        }
        Ok(())
    })
}

unsafe extern "system" fn range_get_item_string_length(
    this: *mut IAvnRustVmRangeBatch,
    index: i32,
    result: *mut i32,
) -> i32 {
    ffi(|| {
        if result.is_null() {
            return Err(hresult::Error(hresult::E_POINTER));
        }
        let index = range_item_index(this, index)?;
        let state = range_state(this)?;
        *result = match &state.items[index].text {
            Some(value) => i32::try_from(value.encode_utf16().count())
                .map_err(|_| hresult::Error(hresult::E_INVALIDARG))?,
            None => 0,
        };
        Ok(())
    })
}

unsafe extern "system" fn range_copy_item_string(
    this: *mut IAvnRustVmRangeBatch,
    index: i32,
    destination: *mut u16,
    capacity: i32,
) -> i32 {
    ffi(|| {
        let index = range_item_index(this, index)?;
        let state = range_state(this)?;
        copy_utf16(
            state.items[index].text.as_deref().unwrap_or(""),
            destination,
            capacity,
        )
    })
}

unsafe extern "system" fn range_complete(
    this: *mut IAvnRustVmRangeBatch,
    outcome: i32,
    error: i32,
) -> i32 {
    ffi(|| {
        let state = range_state(this)?;
        let completion = state
            .completion
            .lock()
            .map_err(|_| hresult::Error(hresult::E_FAIL))?
            .take();
        if let Some(completion) = completion {
            completion(outcome, error);
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI32;

    #[test]
    fn public_map_keys_are_normalized_before_pointer_extraction() {
        for value in [vec![], vec![65], vec![65, 0, 66]] {
            let key = MapKey::Text(value.clone());
            let (text, integer) = key.parts();
            let text = text.unwrap();
            assert!(text.contains(&0));
            assert_eq!(&text[..value.len()], &value);
            assert_eq!(integer, 0);
        }
        let (text, integer) = MapKey::Integer(42).parts();
        assert!(text.is_none());
        assert_eq!(integer, 42);
    }

    fn callbacks() -> RustViewModelCallbacks {
        RustViewModelCallbacks {
            attach: Box::new(|_| Ok(())),
            detach: Box::new(|| Ok(())),
            set_string: Box::new(|_, _| Ok(())),
            set_integer: Box::new(|_, _| Ok(())),
            set_boolean: Box::new(|_, _| Ok(())),
            set_double: Box::new(|_, _| Ok(())),
            execute: Box::new(|_, _| Ok(())),
            begin_async: Box::new(|_, _| Ok(())),
        }
    }

    /// The stage 30 capabilities are separate IIDs on the same object, so a
    /// query must hand back a distinct interface pointer that shares the
    /// object's single reference count.
    #[test]
    fn the_view_model_object_exposes_the_stage_thirty_capabilities() {
        let model = rust_view_model(callbacks());
        let range = model
            .query_interface::<IAvnRustRangeSource>()
            .expect("range source");
        let tracked = model
            .query_interface::<IAvnRustViewModel2>()
            .expect("tracked async");

        assert_ne!(model.as_raw().cast::<u8>(), range.as_raw().cast::<u8>());
        assert_ne!(model.as_raw().cast::<u8>(), tracked.as_raw().cast::<u8>());

        // Round-tripping back to the primary interface must land on the same
        // pointer, proving the identity rule holds through the new vtables.
        let round_trip = range
            .query_interface::<IAvnRustViewModel>()
            .expect("primary interface");
        assert_eq!(model.as_raw(), round_trip.as_raw());
    }

    #[test]
    fn an_unknown_iid_is_rejected_from_every_interface() {
        let model = rust_view_model(callbacks());
        let range = model
            .query_interface::<IAvnRustRangeSource>()
            .expect("range source");
        let unknown = Guid {
            data1: 0xDEAD_BEEF,
            data2: 0,
            data3: 0,
            data4: [0; 8],
        };
        let mut result = ptr::null_mut();
        unsafe {
            assert_eq!(
                hresult::E_NOINTERFACE,
                model_query(model.as_raw().cast(), &unknown, &mut result)
            );
            assert_eq!(
                hresult::E_NOINTERFACE,
                model_query(
                    model_from_range(range.as_raw().cast()),
                    &unknown,
                    &mut result
                )
            );
        }
        assert!(result.is_null());
    }

    /// `request_range` must not touch the view-model lock; it only enqueues.
    /// The default control callbacks report `E_NOTIMPL`, which is what a model
    /// declaring no windowed collection should answer.
    #[test]
    fn a_model_without_a_range_capability_reports_not_implemented() {
        let model = rust_view_model(callbacks());
        let range = model
            .query_interface::<IAvnRustRangeSource>()
            .expect("range source");
        unsafe {
            let mut generation = 0i64;
            let mut total = 0i64;
            let mut dropped_collection = 0i32;
            let mut dropped = 0i64;
            assert_eq!(
                hresult::E_NOTIMPL,
                get_range_state(range.as_raw(), 1, &mut generation, &mut total)
            );
            assert_eq!(
                hresult::E_NOTIMPL,
                request_range(
                    range.as_raw(),
                    1,
                    0,
                    8,
                    1,
                    &mut dropped_collection,
                    &mut dropped
                )
            );
            assert_eq!(
                hresult::E_INVALIDARG,
                request_range(
                    range.as_raw(),
                    1,
                    -1,
                    8,
                    1,
                    &mut dropped_collection,
                    &mut dropped
                )
            );
            assert_eq!(
                hresult::E_INVALIDARG,
                request_range(
                    range.as_raw(),
                    1,
                    0,
                    0,
                    1,
                    &mut dropped_collection,
                    &mut dropped
                )
            );
            assert_eq!(
                hresult::E_POINTER,
                request_range(
                    range.as_raw(),
                    1,
                    0,
                    8,
                    1,
                    &mut dropped_collection,
                    ptr::null_mut()
                )
            );
        }
    }

    #[test]
    fn tracked_async_hands_back_a_fresh_never_reused_handle() {
        let started = Arc::new(Mutex::new(Vec::<(i32, i64)>::new()));
        let recorder = started.clone();
        let model = rust_view_model_with_control(
            callbacks(),
            RustViewModelControlCallbacks::default(),
            Some(Box::new(move |command, _, operation| {
                recorder
                    .lock()
                    .expect("recorder")
                    .push((command, operation));
                Ok(())
            })),
        );
        let tracked = model
            .query_interface::<IAvnRustViewModel2>()
            .expect("tracked async");

        let mut first = 0i64;
        let mut second = 0i64;
        unsafe {
            assert_eq!(
                hresult::S_OK,
                begin_async_tracked(tracked.as_raw(), 7, ptr::null(), &mut first)
            );
            assert_eq!(
                hresult::S_OK,
                begin_async_tracked(tracked.as_raw(), 7, ptr::null(), &mut second)
            );
        }

        assert_ne!(first, second);
        assert_eq!(
            vec![(7, first), (7, second)],
            *started.lock().expect("recorder")
        );
    }

    #[test]
    fn a_failed_tracked_start_reports_no_handle() {
        let model = rust_view_model_with_control(
            callbacks(),
            RustViewModelControlCallbacks::default(),
            Some(Box::new(|_, _, _| Err(hresult::Error(hresult::E_FAIL)))),
        );
        let tracked = model
            .query_interface::<IAvnRustViewModel2>()
            .expect("tracked async");

        let mut operation = 99i64;
        unsafe {
            assert_eq!(
                hresult::E_FAIL,
                begin_async_tracked(tracked.as_raw(), 1, ptr::null(), &mut operation)
            );
        }
        assert_eq!(0, operation, "a failed start must not publish a handle");
    }

    #[test]
    fn cancellation_never_takes_the_view_model_lock() {
        let cancelled = Arc::new(AtomicI32::new(0));
        let recorder = cancelled.clone();
        let model = rust_view_model_with_control(
            callbacks(),
            RustViewModelControlCallbacks {
                cancel_async: Box::new(move |_, operation| {
                    recorder.store(operation as i32, Ordering::SeqCst);
                    Ok(())
                }),
                ..Default::default()
            },
            None,
        );
        let tracked = model
            .query_interface::<IAvnRustViewModel2>()
            .expect("tracked async");

        // Hold the model lock exactly as a running worker would; cancelling
        // must still complete rather than deadlocking.
        let object = model.as_raw().cast::<RustViewModelObject>();
        let guard = unsafe { (*object).callbacks.lock().expect("model lock") };
        unsafe {
            assert_eq!(hresult::S_OK, cancel_async(tracked.as_raw(), 3, 42));
        }
        drop(guard);

        assert_eq!(42, cancelled.load(Ordering::SeqCst));
    }

    #[test]
    fn a_range_batch_is_immutable_and_completes_exactly_once() {
        let completions = Arc::new(AtomicI32::new(0));
        let recorder = completions.clone();
        let batch = rust_vm_range_batch(
            RUST_VM_RANGE_FILL,
            5,
            11,
            100,
            64,
            vec![
                RustVmRangeItem {
                    text: Some("first".to_owned()),
                    model: None,
                },
                RustVmRangeItem {
                    text: Some("second".to_owned()),
                    model: None,
                },
            ],
            Some(Box::new(move |outcome, _| {
                recorder.fetch_add(outcome + 1, Ordering::SeqCst);
            })),
        );

        unsafe {
            let mut kind = 0i32;
            let mut collection = 0i32;
            let mut generation = 0i64;
            let mut total = 0i64;
            let mut offset = 0i64;
            let mut count = 0i32;
            assert_eq!(hresult::S_OK, range_get_kind(batch.as_raw(), &mut kind));
            assert_eq!(
                hresult::S_OK,
                range_get_collection_id(batch.as_raw(), &mut collection)
            );
            assert_eq!(
                hresult::S_OK,
                range_get_generation(batch.as_raw(), &mut generation)
            );
            assert_eq!(
                hresult::S_OK,
                range_get_total_count(batch.as_raw(), &mut total)
            );
            assert_eq!(hresult::S_OK, range_get_offset(batch.as_raw(), &mut offset));
            assert_eq!(
                hresult::S_OK,
                range_get_item_count(batch.as_raw(), &mut count)
            );
            assert_eq!(
                (RUST_VM_RANGE_FILL, 5, 11, 100, 64, 2),
                (kind, collection, generation, total, offset, count)
            );

            let mut length = 0i32;
            assert_eq!(
                hresult::S_OK,
                range_get_item_string_length(batch.as_raw(), 1, &mut length)
            );
            assert_eq!(6, length);
            let mut buffer = vec![0u16; 7];
            assert_eq!(
                hresult::S_OK,
                range_copy_item_string(batch.as_raw(), 1, buffer.as_mut_ptr(), 7)
            );
            assert_eq!("second", String::from_utf16_lossy(&buffer[..6]));

            // A too-small buffer and an out-of-range index are rejected, not
            // silently truncated.
            assert_eq!(
                hresult::E_INVALIDARG,
                range_copy_item_string(batch.as_raw(), 1, buffer.as_mut_ptr(), 3)
            );
            assert_eq!(
                hresult::E_INVALIDARG,
                range_get_item_string_length(batch.as_raw(), 9, &mut length)
            );
        }

        assert_eq!(hresult::S_OK, batch.complete(0, 0).map_or(-1, |()| 0));
        assert_eq!(hresult::S_OK, batch.complete(3, -1).map_or(-1, |()| 0));
        assert_eq!(
            1,
            completions.load(Ordering::SeqCst),
            "only the first completion is delivered"
        );
    }

    /// A rejected range batch must release the element models it carried,
    /// which is what keeps a stale window from leaking managed adapters.
    #[test]
    fn dropping_a_range_batch_releases_its_element_models() {
        let element = rust_view_model(callbacks());
        let raw = element.as_raw();
        let batch = rust_vm_range_batch(
            RUST_VM_RANGE_FILL,
            5,
            1,
            1,
            0,
            vec![RustVmRangeItem {
                text: None,
                model: Some(element),
            }],
            None,
        );

        unsafe {
            let references = (*raw.cast::<RustViewModelObject>())
                .references
                .load(Ordering::SeqCst);
            assert_eq!(1, references, "the batch owns the only reference");

            let mut out = ptr::null_mut();
            assert_eq!(
                hresult::S_OK,
                range_get_item_model(batch.as_raw(), 0, &mut out)
            );
            assert_eq!(raw, out);
            assert_eq!(
                2,
                (*raw.cast::<RustViewModelObject>())
                    .references
                    .load(Ordering::SeqCst),
                "reading a model hands out a counted reference"
            );

            drop(batch);
            assert_eq!(
                1,
                (*raw.cast::<RustViewModelObject>())
                    .references
                    .load(Ordering::SeqCst),
                "dropping the batch releases its own reference"
            );
            release(out.cast());
        }
    }
    /// Cross-language vtable tripwire. Managed `[GeneratedComInterface]`
    /// dispatch is purely positional, so a method added on one side only would
    /// silently shift every later slot. `RustDataShapeTests` asserts the same
    /// counts from the managed side; changing either without the other fails.
    #[test]
    fn stage_thirty_vtables_have_their_published_slot_counts() {
        const PTR: usize = std::mem::size_of::<usize>();
        const IUNKNOWN: usize = 3;
        assert_eq!(
            (IUNKNOWN + 11) * PTR,
            std::mem::size_of::<IAvnRustVmSink4Vtbl>()
        );
        assert_eq!(
            (IUNKNOWN + 10) * PTR,
            std::mem::size_of::<IAvnRustVmRangeBatchVtbl>()
        );
        assert_eq!(
            (IUNKNOWN + 2) * PTR,
            std::mem::size_of::<IAvnRustRangeSourceVtbl>()
        );
        assert_eq!(
            (IUNKNOWN + 2) * PTR,
            std::mem::size_of::<IAvnRustViewModel2Vtbl>()
        );
        assert_eq!(
            (IUNKNOWN + 6) * PTR,
            std::mem::size_of::<IAvnRustVmSink5Vtbl>()
        );
    }
}

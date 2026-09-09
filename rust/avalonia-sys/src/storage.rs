//! Stage 29 desktop file integration ABI.
//!
//! These interfaces form a separately versioned capability that is queried from
//! `IAvnApplication`; nothing here is ever added to an already published
//! vtable. Host-implemented interfaces (`IAvnApplication3`,
//! `IAvnFilePickerOptions`, `IAvnStorageItem`, `IAvnStorageItemList`) are
//! called from Rust; Rust-implemented interfaces (`IAvnStorageCompletion`,
//! `IAvnFileDropHandler`, `IAvnActivationHandler`) are called by the host.

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::{IAvnControl, IAvnWindow};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::ptr;

const IAVN_APPLICATION3_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x50],
};

const IAVN_FILE_PICKER_OPTIONS_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x51],
};

const IAVN_STORAGE_ITEM_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x52],
};

const IAVN_STORAGE_ITEM_LIST_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x53],
};

const IAVN_STORAGE_COMPLETION_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x54],
};

const IAVN_FILE_DROP_HANDLER_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x55],
};

const IAVN_ACTIVATION_HANDLER_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x56],
};

/// A storage item read across the ABI and copied into Rust-owned memory.
///
/// `uri` is always populated by the host. `local_path` is `None` for items the
/// platform cannot map to a filesystem path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageItemData {
    pub kind: i32,
    pub name: Option<String>,
    pub uri: Option<String>,
    pub local_path: Option<String>,
}

/// Arguments of one `IAvnStorageCompletion::Complete` call.
#[derive(Debug)]
pub struct StorageCompletionArgs {
    pub operation_id: i64,
    pub hresult: i32,
    pub outcome: i32,
    pub items: Vec<StorageItemData>,
    pub error: Option<String>,
}

/// Arguments of one `IAvnFileDropHandler::OnDragEvent` call.
#[derive(Debug)]
pub struct FileDropArgs {
    pub subscription_id: i64,
    pub kind: i32,
    pub allowed_effects: i32,
    pub effective_effects: i32,
    pub items: Vec<StorageItemData>,
}

/// Arguments of one `IAvnActivationHandler::OnActivated` call.
#[derive(Debug)]
pub struct ActivationArgs {
    pub kind: i32,
    pub items: Vec<StorageItemData>,
}

#[repr(C)]
struct IAvnStorageItemVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_kind: unsafe extern "system" fn(*mut IAvnStorageItem, *mut i32) -> i32,
    get_name: unsafe extern "system" fn(*mut IAvnStorageItem, *mut *mut u16) -> i32,
    get_uri: unsafe extern "system" fn(*mut IAvnStorageItem, *mut *mut u16) -> i32,
    try_get_local_path:
        unsafe extern "system" fn(*mut IAvnStorageItem, *mut i32, *mut *mut u16) -> i32,
}

#[repr(C)]
pub struct IAvnStorageItem {
    vtbl: *const IAvnStorageItemVtbl,
}

unsafe impl ComInterface for IAvnStorageItem {
    const IID: Guid = IAVN_STORAGE_ITEM_IID;
}

impl ComPtr<IAvnStorageItem> {
    pub fn read(&self) -> Result<StorageItemData> {
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            let mut kind = 0;
            hresult::check((vtbl.get_kind)(self.as_raw(), &mut kind))?;

            // Each string is taken as soon as it is produced: an out-parameter
            // that already succeeded is owned by this side, so a later failure
            // must not leak it.
            let mut name = ptr::null_mut();
            hresult::check((vtbl.get_name)(self.as_raw(), &mut name))?;
            let name = crate::take_utf16(name);

            let mut uri = ptr::null_mut();
            hresult::check((vtbl.get_uri)(self.as_raw(), &mut uri))?;
            let uri = crate::take_utf16(uri);

            let mut found = 0;
            let mut local_path = ptr::null_mut();
            hresult::check((vtbl.try_get_local_path)(
                self.as_raw(),
                &mut found,
                &mut local_path,
            ))?;
            let local_path = crate::take_utf16(local_path);
            Ok(StorageItemData {
                kind,
                name,
                uri,
                local_path: if found == 0 { None } else { local_path },
            })
        }
    }
}

#[repr(C)]
struct IAvnStorageItemListVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_count: unsafe extern "system" fn(*mut IAvnStorageItemList, *mut i32) -> i32,
    get_item:
        unsafe extern "system" fn(*mut IAvnStorageItemList, i32, *mut *mut IAvnStorageItem) -> i32,
}

#[repr(C)]
pub struct IAvnStorageItemList {
    vtbl: *const IAvnStorageItemListVtbl,
}

unsafe impl ComInterface for IAvnStorageItemList {
    const IID: Guid = IAVN_STORAGE_ITEM_LIST_IID;
}

impl ComPtr<IAvnStorageItemList> {
    pub fn count(&self) -> Result<i32> {
        unsafe {
            let mut value = 0;
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().get_count)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn item(&self, index: i32) -> Result<ComPtr<IAvnStorageItem>> {
        unsafe {
            let mut value = ptr::null_mut();
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().get_item)(
                self.as_raw(),
                index,
                &mut value,
            );
            hresult::check(hr)?;
            ComPtr::from_raw(value).ok_or(crate::Error(hresult::E_POINTER))
        }
    }

    /// Copies the whole list into Rust-owned memory.
    pub fn read_all(&self) -> Result<Vec<StorageItemData>> {
        let count = self.count()?;
        let mut items = Vec::with_capacity(count.max(0) as usize);
        for index in 0..count {
            items.push(self.item(index)?.read()?);
        }
        Ok(items)
    }
}

/// Reads a borrowed `IAvnStorageItemList` in-parameter without taking ownership.
///
/// The host owns the reference for the duration of the call, exactly like any
/// other COM in-parameter, so the list is materialized eagerly and the Rust
/// callback only ever sees owned data.
///
/// # Safety
/// `list` must be null or a valid `IAvnStorageItemList` owned by the caller.
unsafe fn read_borrowed_items(list: *mut IAvnStorageItemList) -> Vec<StorageItemData> {
    let Some(list) = ComPtr::from_borrowed(list) else {
        return Vec::new();
    };
    list.read_all().unwrap_or_default()
}

#[repr(C)]
struct IAvnFilePickerOptionsVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set_title: unsafe extern "system" fn(*mut IAvnFilePickerOptions, *const u16) -> i32,
    set_allow_multiple: unsafe extern "system" fn(*mut IAvnFilePickerOptions, i32) -> i32,
    set_suggested_file_name:
        unsafe extern "system" fn(*mut IAvnFilePickerOptions, *const u16) -> i32,
    set_suggested_start_location:
        unsafe extern "system" fn(*mut IAvnFilePickerOptions, *const u16) -> i32,
    set_suggested_start_well_known_folder:
        unsafe extern "system" fn(*mut IAvnFilePickerOptions, i32) -> i32,
    set_default_extension: unsafe extern "system" fn(*mut IAvnFilePickerOptions, *const u16) -> i32,
    set_show_overwrite_prompt: unsafe extern "system" fn(*mut IAvnFilePickerOptions, i32) -> i32,
    add_file_type:
        unsafe extern "system" fn(*mut IAvnFilePickerOptions, *const u16, *mut i32) -> i32,
    add_file_type_pattern:
        unsafe extern "system" fn(*mut IAvnFilePickerOptions, i32, *const u16) -> i32,
    add_file_type_mime_type:
        unsafe extern "system" fn(*mut IAvnFilePickerOptions, i32, *const u16) -> i32,
    add_file_type_apple_uniform_type_identifier:
        unsafe extern "system" fn(*mut IAvnFilePickerOptions, i32, *const u16) -> i32,
    set_suggested_file_type_index:
        unsafe extern "system" fn(*mut IAvnFilePickerOptions, i32) -> i32,
}

#[repr(C)]
pub struct IAvnFilePickerOptions {
    vtbl: *const IAvnFilePickerOptionsVtbl,
}

unsafe impl ComInterface for IAvnFilePickerOptions {
    const IID: Guid = IAVN_FILE_PICKER_OPTIONS_IID;
}

fn optional_utf16(value: Option<&[u16]>) -> *const u16 {
    value.map_or(ptr::null(), <[u16]>::as_ptr)
}

/// Null for a missing interface argument.
///
/// The host contract for every stage 29 interface in-parameter is `E_POINTER`,
/// so the raw layer can express (and conformance tests can exercise) a null
/// argument. The safe API always passes `Some`.
fn optional_raw<T: ComInterface>(value: Option<&ComPtr<T>>) -> *mut T {
    value.map_or(ptr::null_mut(), ComPtr::as_raw)
}

impl ComPtr<IAvnFilePickerOptions> {
    pub fn set_title(&self, value: Option<&[u16]>) -> Result<()> {
        let value = value.map(crate::terminated_utf16);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_title)(
                self.as_raw(),
                optional_utf16(value.as_deref()),
            ))
        }
    }

    pub fn set_allow_multiple(&self, value: bool) -> Result<()> {
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_allow_multiple)(self.as_raw(), i32::from(value)))
        }
    }

    pub fn set_suggested_file_name(&self, value: Option<&[u16]>) -> Result<()> {
        let value = value.map(crate::terminated_utf16);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_suggested_file_name)(
                self.as_raw(),
                optional_utf16(value.as_deref()),
            ))
        }
    }

    pub fn set_suggested_start_location(&self, value: Option<&[u16]>) -> Result<()> {
        let value = value.map(crate::terminated_utf16);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_suggested_start_location)(
                self.as_raw(),
                optional_utf16(value.as_deref()),
            ))
        }
    }

    pub fn set_suggested_start_well_known_folder(&self, value: i32) -> Result<()> {
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_suggested_start_well_known_folder)(
                self.as_raw(),
                value,
            ))
        }
    }

    pub fn set_default_extension(&self, value: Option<&[u16]>) -> Result<()> {
        let value = value.map(crate::terminated_utf16);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_default_extension)(
                self.as_raw(),
                optional_utf16(value.as_deref()),
            ))
        }
    }

    /// `-1` keeps the platform default, `0`/`1` force the prompt off/on.
    pub fn set_show_overwrite_prompt(&self, value: i32) -> Result<()> {
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_show_overwrite_prompt)(self.as_raw(), value))
        }
    }

    pub fn add_file_type(&self, name: Option<&[u16]>) -> Result<i32> {
        let name = name.map(crate::terminated_utf16);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            let mut index = 0;
            let hr =
                (vtbl.add_file_type)(self.as_raw(), optional_utf16(name.as_deref()), &mut index);
            hresult::check(hr).map(|_| index)
        }
    }

    pub fn add_file_type_pattern(&self, index: i32, value: &[u16]) -> Result<()> {
        let value = crate::terminated_utf16(value);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.add_file_type_pattern)(
                self.as_raw(),
                index,
                value.as_ptr(),
            ))
        }
    }

    pub fn add_file_type_mime_type(&self, index: i32, value: &[u16]) -> Result<()> {
        let value = crate::terminated_utf16(value);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.add_file_type_mime_type)(
                self.as_raw(),
                index,
                value.as_ptr(),
            ))
        }
    }

    pub fn add_file_type_apple_uniform_type_identifier(
        &self,
        index: i32,
        value: &[u16],
    ) -> Result<()> {
        let value = crate::terminated_utf16(value);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.add_file_type_apple_uniform_type_identifier)(
                self.as_raw(),
                index,
                value.as_ptr(),
            ))
        }
    }

    pub fn set_suggested_file_type_index(&self, index: i32) -> Result<()> {
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_suggested_file_type_index)(self.as_raw(), index))
        }
    }
}

#[repr(C)]
struct IAvnStorageCompletionVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    complete: unsafe extern "system" fn(
        *mut IAvnStorageCompletion,
        i64,
        i32,
        i32,
        *mut IAvnStorageItemList,
        *const u16,
    ) -> i32,
}

#[repr(C)]
pub struct IAvnStorageCompletion {
    vtbl: *const IAvnStorageCompletionVtbl,
}

unsafe impl ComInterface for IAvnStorageCompletion {
    const IID: Guid = IAVN_STORAGE_COMPLETION_IID;
}

static STORAGE_COMPLETION_VTBL: IAvnStorageCompletionVtbl = IAvnStorageCompletionVtbl {
    query_interface: storage_completion_query_interface,
    add_ref: storage_completion_add_ref,
    release: storage_completion_release,
    complete: storage_completion_complete,
};

/// Creates a Rust-implemented storage completion object.
pub fn storage_completion(
    callback: impl FnMut(&mut StorageCompletionArgs) -> Result<()> + Send + 'static,
) -> ComPtr<IAvnStorageCompletion> {
    crate::event_callback::create(
        IAvnStorageCompletion {
            vtbl: &STORAGE_COMPLETION_VTBL,
        },
        callback,
    )
}

unsafe extern "system" fn storage_completion_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnStorageCompletion, StorageCompletionArgs>(
        this, iid, result,
    )
}

unsafe extern "system" fn storage_completion_add_ref(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnStorageCompletion, StorageCompletionArgs>(this)
}

unsafe extern "system" fn storage_completion_release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnStorageCompletion, StorageCompletionArgs>(this)
}

unsafe extern "system" fn storage_completion_complete(
    this: *mut IAvnStorageCompletion,
    operation_id: i64,
    hresult: i32,
    outcome: i32,
    items: *mut IAvnStorageItemList,
    error: *const u16,
) -> i32 {
    let mut arguments = StorageCompletionArgs {
        operation_id,
        hresult,
        outcome,
        items: read_borrowed_items(items),
        error: crate::clone_utf16(error),
    };
    crate::event_callback::invoke::<IAvnStorageCompletion, StorageCompletionArgs>(
        this,
        &mut arguments,
    )
}

#[repr(C)]
struct IAvnFileDropHandlerVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    on_drag_event: unsafe extern "system" fn(
        *mut IAvnFileDropHandler,
        i64,
        i32,
        i32,
        i32,
        *mut IAvnStorageItemList,
    ) -> i32,
}

#[repr(C)]
pub struct IAvnFileDropHandler {
    vtbl: *const IAvnFileDropHandlerVtbl,
}

unsafe impl ComInterface for IAvnFileDropHandler {
    const IID: Guid = IAVN_FILE_DROP_HANDLER_IID;
}

static FILE_DROP_HANDLER_VTBL: IAvnFileDropHandlerVtbl = IAvnFileDropHandlerVtbl {
    query_interface: file_drop_query_interface,
    add_ref: file_drop_add_ref,
    release: file_drop_release,
    on_drag_event: file_drop_on_drag_event,
};

/// Creates a Rust-implemented incoming drag-and-drop sink.
pub fn file_drop_handler(
    callback: impl FnMut(&mut FileDropArgs) -> Result<()> + Send + 'static,
) -> ComPtr<IAvnFileDropHandler> {
    crate::event_callback::create(
        IAvnFileDropHandler {
            vtbl: &FILE_DROP_HANDLER_VTBL,
        },
        callback,
    )
}

unsafe extern "system" fn file_drop_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnFileDropHandler, FileDropArgs>(this, iid, result)
}

unsafe extern "system" fn file_drop_add_ref(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnFileDropHandler, FileDropArgs>(this)
}

unsafe extern "system" fn file_drop_release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnFileDropHandler, FileDropArgs>(this)
}

unsafe extern "system" fn file_drop_on_drag_event(
    this: *mut IAvnFileDropHandler,
    subscription_id: i64,
    kind: i32,
    allowed_effects: i32,
    effective_effects: i32,
    items: *mut IAvnStorageItemList,
) -> i32 {
    let mut arguments = FileDropArgs {
        subscription_id,
        kind,
        allowed_effects,
        effective_effects,
        items: read_borrowed_items(items),
    };
    crate::event_callback::invoke::<IAvnFileDropHandler, FileDropArgs>(this, &mut arguments)
}

#[repr(C)]
struct IAvnActivationHandlerVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    on_activated:
        unsafe extern "system" fn(*mut IAvnActivationHandler, i32, *mut IAvnStorageItemList) -> i32,
}

#[repr(C)]
pub struct IAvnActivationHandler {
    vtbl: *const IAvnActivationHandlerVtbl,
}

unsafe impl ComInterface for IAvnActivationHandler {
    const IID: Guid = IAVN_ACTIVATION_HANDLER_IID;
}

static ACTIVATION_HANDLER_VTBL: IAvnActivationHandlerVtbl = IAvnActivationHandlerVtbl {
    query_interface: activation_query_interface,
    add_ref: activation_add_ref,
    release: activation_release,
    on_activated: activation_on_activated,
};

/// Creates a Rust-implemented post-startup activation sink.
pub fn activation_handler(
    callback: impl FnMut(&mut ActivationArgs) -> Result<()> + Send + 'static,
) -> ComPtr<IAvnActivationHandler> {
    crate::event_callback::create(
        IAvnActivationHandler {
            vtbl: &ACTIVATION_HANDLER_VTBL,
        },
        callback,
    )
}

unsafe extern "system" fn activation_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnActivationHandler, ActivationArgs>(
        this, iid, result,
    )
}

unsafe extern "system" fn activation_add_ref(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnActivationHandler, ActivationArgs>(this)
}

unsafe extern "system" fn activation_release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnActivationHandler, ActivationArgs>(this)
}

unsafe extern "system" fn activation_on_activated(
    this: *mut IAvnActivationHandler,
    kind: i32,
    items: *mut IAvnStorageItemList,
) -> i32 {
    let mut arguments = ActivationArgs {
        kind,
        items: read_borrowed_items(items),
    };
    crate::event_callback::invoke::<IAvnActivationHandler, ActivationArgs>(this, &mut arguments)
}

#[repr(C)]
struct IAvnApplication3Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    create_picker_options:
        unsafe extern "system" fn(*mut IAvnApplication3, *mut *mut IAvnFilePickerOptions) -> i32,
    get_storage_capabilities:
        unsafe extern "system" fn(*mut IAvnApplication3, *mut IAvnWindow, *mut i32) -> i32,
    start_open_file_picker: unsafe extern "system" fn(
        *mut IAvnApplication3,
        *mut IAvnWindow,
        *mut IAvnFilePickerOptions,
        *mut IAvnStorageCompletion,
        *mut i64,
    ) -> i32,
    start_open_folder_picker: unsafe extern "system" fn(
        *mut IAvnApplication3,
        *mut IAvnWindow,
        *mut IAvnFilePickerOptions,
        *mut IAvnStorageCompletion,
        *mut i64,
    ) -> i32,
    start_save_file_picker: unsafe extern "system" fn(
        *mut IAvnApplication3,
        *mut IAvnWindow,
        *mut IAvnFilePickerOptions,
        *mut IAvnStorageCompletion,
        *mut i64,
    ) -> i32,
    subscribe_file_drop: unsafe extern "system" fn(
        *mut IAvnApplication3,
        *mut IAvnControl,
        i32,
        *mut IAvnFileDropHandler,
        *mut i64,
    ) -> i32,
    unsubscribe_file_drop: unsafe extern "system" fn(*mut IAvnApplication3, i64) -> i32,
    clear_startup_arguments: unsafe extern "system" fn(*mut IAvnApplication3) -> i32,
    add_startup_argument: unsafe extern "system" fn(*mut IAvnApplication3, *const u16) -> i32,
    get_startup_argument_count: unsafe extern "system" fn(*mut IAvnApplication3, *mut i32) -> i32,
    get_startup_argument:
        unsafe extern "system" fn(*mut IAvnApplication3, i32, *mut *mut u16) -> i32,
    get_activation_items:
        unsafe extern "system" fn(*mut IAvnApplication3, *mut *mut IAvnStorageItemList) -> i32,
    advise_activation: unsafe extern "system" fn(
        *mut IAvnApplication3,
        *mut IAvnActivationHandler,
        *mut i64,
    ) -> i32,
    unadvise_activation: unsafe extern "system" fn(*mut IAvnApplication3, i64) -> i32,
}

#[repr(C)]
pub struct IAvnApplication3 {
    vtbl: *const IAvnApplication3Vtbl,
}

unsafe impl ComInterface for IAvnApplication3 {
    const IID: Guid = IAVN_APPLICATION3_IID;
}

impl ComPtr<IAvnApplication3> {
    pub fn create_picker_options(&self) -> Result<ComPtr<IAvnFilePickerOptions>> {
        unsafe {
            let mut value = ptr::null_mut();
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .create_picker_options)(self.as_raw(), &mut value);
            hresult::check(hr)?;
            ComPtr::from_raw(value).ok_or(crate::Error(hresult::E_POINTER))
        }
    }

    pub fn storage_capabilities(&self, window: Option<&ComPtr<IAvnWindow>>) -> Result<i32> {
        unsafe {
            let mut value = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .get_storage_capabilities)(
                self.as_raw(), optional_raw(window), &mut value
            );
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn start_open_file_picker(
        &self,
        window: Option<&ComPtr<IAvnWindow>>,
        options: Option<&ComPtr<IAvnFilePickerOptions>>,
        completion: Option<&ComPtr<IAvnStorageCompletion>>,
    ) -> Result<i64> {
        unsafe {
            let start = (*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_open_file_picker;
            self.start_picker(start, window, options, completion)
        }
    }

    pub fn start_open_folder_picker(
        &self,
        window: Option<&ComPtr<IAvnWindow>>,
        options: Option<&ComPtr<IAvnFilePickerOptions>>,
        completion: Option<&ComPtr<IAvnStorageCompletion>>,
    ) -> Result<i64> {
        unsafe {
            let start = (*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_open_folder_picker;
            self.start_picker(start, window, options, completion)
        }
    }

    pub fn start_save_file_picker(
        &self,
        window: Option<&ComPtr<IAvnWindow>>,
        options: Option<&ComPtr<IAvnFilePickerOptions>>,
        completion: Option<&ComPtr<IAvnStorageCompletion>>,
    ) -> Result<i64> {
        unsafe {
            let start = (*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_save_file_picker;
            self.start_picker(start, window, options, completion)
        }
    }

    unsafe fn start_picker(
        &self,
        start: unsafe extern "system" fn(
            *mut IAvnApplication3,
            *mut IAvnWindow,
            *mut IAvnFilePickerOptions,
            *mut IAvnStorageCompletion,
            *mut i64,
        ) -> i32,
        window: Option<&ComPtr<IAvnWindow>>,
        options: Option<&ComPtr<IAvnFilePickerOptions>>,
        completion: Option<&ComPtr<IAvnStorageCompletion>>,
    ) -> Result<i64> {
        let mut operation_id = 0;
        let hr = start(
            self.as_raw(),
            optional_raw(window),
            optional_raw(options),
            optional_raw(completion),
            &mut operation_id,
        );
        hresult::check(hr).map(|_| operation_id)
    }

    pub fn subscribe_file_drop(
        &self,
        target: Option<&ComPtr<IAvnControl>>,
        accepted_effects: i32,
        handler: Option<&ComPtr<IAvnFileDropHandler>>,
    ) -> Result<i64> {
        unsafe {
            let mut subscription_id = 0;
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().subscribe_file_drop)(
                self.as_raw(),
                optional_raw(target),
                accepted_effects,
                optional_raw(handler),
                &mut subscription_id,
            );
            hresult::check(hr).map(|_| subscription_id)
        }
    }

    pub fn unsubscribe_file_drop(&self, subscription_id: i64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .unsubscribe_file_drop)(
                self.as_raw(), subscription_id
            ))
        }
    }

    pub fn clear_startup_arguments(&self) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .clear_startup_arguments)(self.as_raw()))
        }
    }

    pub fn add_startup_argument(&self, value: Option<&[u16]>) -> Result<()> {
        let value = value.map(crate::terminated_utf16);
        unsafe {
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .add_startup_argument)(
                self.as_raw(), optional_utf16(value.as_deref())
            ))
        }
    }

    pub fn startup_argument_count(&self) -> Result<i32> {
        unsafe {
            let mut value = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .get_startup_argument_count)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn startup_argument(&self, index: i32) -> Result<Option<String>> {
        unsafe {
            let mut value = ptr::null_mut();
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().get_startup_argument)(
                self.as_raw(),
                index,
                &mut value,
            );
            hresult::check(hr)?;
            Ok(crate::take_utf16(value))
        }
    }

    pub fn activation_items(&self) -> Result<Vec<StorageItemData>> {
        unsafe {
            let mut value = ptr::null_mut();
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().get_activation_items)(
                self.as_raw(),
                &mut value,
            );
            hresult::check(hr)?;
            let list = ComPtr::from_raw(value).ok_or(crate::Error(hresult::E_POINTER))?;
            list.read_all()
        }
    }

    pub fn advise_activation(
        &self,
        handler: Option<&ComPtr<IAvnActivationHandler>>,
    ) -> Result<i64> {
        unsafe {
            let mut subscription_id = 0;
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().advise_activation)(
                self.as_raw(),
                optional_raw(handler),
                &mut subscription_id,
            );
            hresult::check(hr).map(|_| subscription_id)
        }
    }

    pub fn unadvise_activation(&self, subscription_id: i64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .unadvise_activation)(
                self.as_raw(), subscription_id
            ))
        }
    }
}

impl ComPtr<crate::application::IAvnApplication> {
    /// Queries the optional stage 29 desktop file integration capability.
    pub fn desktop_files(&self) -> Result<ComPtr<IAvnApplication3>> {
        self.query_interface::<IAvnApplication3>()
    }
}

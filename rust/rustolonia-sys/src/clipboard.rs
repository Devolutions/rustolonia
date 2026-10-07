//! Stage 31 clipboard command ABI.
//!
//! `IAvnApplication4` and `IAvnClipboardData` are a separately versioned
//! capability queried from `IAvnApplication`; nothing here is added to an
//! already published vtable. Plain-text write/read stay on the frozen
//! `IAvnApplication` vtable and are not duplicated. Reading file entries reuses
//! the stage 29 `IAvnStorageCompletion`/`IAvnStorageItemList` snapshots.

use crate::application::IAvnApplication;
use crate::async_completion::IAvnAsyncCompletion;
use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::IAvnWindow;
use crate::guid::Guid;
use crate::hresult::{self, Result};
use crate::storage::IAvnStorageCompletion;
use std::ffi::c_void;
use std::ptr;

const IAVN_APPLICATION4_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x60],
};

const IAVN_CLIPBOARD_DATA_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x61],
};

#[repr(C)]
struct IAvnClipboardDataVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set_text: unsafe extern "system" fn(*mut IAvnClipboardData, *const u16) -> i32,
    add_file_uri: unsafe extern "system" fn(*mut IAvnClipboardData, *const u16) -> i32,
}

#[repr(C)]
pub struct IAvnClipboardData {
    vtbl: *const IAvnClipboardDataVtbl,
}

unsafe impl ComInterface for IAvnClipboardData {
    const IID: Guid = IAVN_CLIPBOARD_DATA_IID;
}

impl ComPtr<IAvnClipboardData> {
    pub fn set_text(&self, value: Option<&[u16]>) -> Result<()> {
        let value = value.map(crate::terminated_utf16);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_text)(
                self.as_raw(),
                optional_utf16(value.as_deref()),
            ))
        }
    }

    pub fn add_file_uri(&self, value: Option<&[u16]>) -> Result<()> {
        let value = value.map(crate::terminated_utf16);
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.add_file_uri)(
                self.as_raw(),
                optional_utf16(value.as_deref()),
            ))
        }
    }
}

#[repr(C)]
struct IAvnApplication4Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    create_clipboard_data:
        unsafe extern "system" fn(*mut IAvnApplication4, *mut *mut IAvnClipboardData) -> i32,
    get_clipboard_capabilities:
        unsafe extern "system" fn(*mut IAvnApplication4, *mut IAvnWindow, *mut i32) -> i32,
    start_clipboard_write: unsafe extern "system" fn(
        *mut IAvnApplication4,
        *mut IAvnWindow,
        *mut IAvnClipboardData,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
    start_clipboard_clear: unsafe extern "system" fn(
        *mut IAvnApplication4,
        *mut IAvnWindow,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
    start_clipboard_read_files: unsafe extern "system" fn(
        *mut IAvnApplication4,
        *mut IAvnWindow,
        *mut IAvnStorageCompletion,
        *mut i64,
    ) -> i32,
}

#[repr(C)]
pub struct IAvnApplication4 {
    vtbl: *const IAvnApplication4Vtbl,
}

unsafe impl ComInterface for IAvnApplication4 {
    const IID: Guid = IAVN_APPLICATION4_IID;
}

fn optional_utf16(value: Option<&[u16]>) -> *const u16 {
    value.map_or(ptr::null(), <[u16]>::as_ptr)
}

fn optional_raw<T: ComInterface>(value: Option<&ComPtr<T>>) -> *mut T {
    value.map_or(ptr::null_mut(), ComPtr::as_raw)
}

impl ComPtr<IAvnApplication4> {
    pub fn create_clipboard_data(&self) -> Result<ComPtr<IAvnClipboardData>> {
        unsafe {
            let mut value = ptr::null_mut();
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .create_clipboard_data)(self.as_raw(), &mut value);
            hresult::check(hr)?;
            ComPtr::from_raw(value).ok_or(crate::Error(hresult::E_POINTER))
        }
    }

    pub fn clipboard_capabilities(&self, window: Option<&ComPtr<IAvnWindow>>) -> Result<i32> {
        unsafe {
            let mut value = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .get_clipboard_capabilities)(
                self.as_raw(), optional_raw(window), &mut value
            );
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn start_clipboard_write(
        &self,
        window: Option<&ComPtr<IAvnWindow>>,
        data: Option<&ComPtr<IAvnClipboardData>>,
        completion: Option<&ComPtr<IAvnAsyncCompletion>>,
    ) -> Result<i64> {
        unsafe {
            let mut operation_id = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_clipboard_write)(
                self.as_raw(),
                optional_raw(window),
                optional_raw(data),
                optional_raw(completion),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }

    pub fn start_clipboard_clear(
        &self,
        window: Option<&ComPtr<IAvnWindow>>,
        completion: Option<&ComPtr<IAvnAsyncCompletion>>,
    ) -> Result<i64> {
        unsafe {
            let mut operation_id = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_clipboard_clear)(
                self.as_raw(),
                optional_raw(window),
                optional_raw(completion),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }

    pub fn start_clipboard_read_files(
        &self,
        window: Option<&ComPtr<IAvnWindow>>,
        completion: Option<&ComPtr<IAvnStorageCompletion>>,
    ) -> Result<i64> {
        unsafe {
            let mut operation_id = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_clipboard_read_files)(
                self.as_raw(),
                optional_raw(window),
                optional_raw(completion),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }
}

impl ComPtr<IAvnApplication> {
    /// Queries the stage 31 clipboard command capability.
    pub fn clipboard(&self) -> Result<ComPtr<IAvnApplication4>> {
        self.query_interface::<IAvnApplication4>()
    }
}

/// Number of methods `IAvnApplication4` publishes after `IUnknown`.
///
/// A cross-language tripwire: the managed interface and this vtable must stay
/// the same shape, and a published capability is never widened in place.
pub const IAVN_APPLICATION4_METHOD_COUNT: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn application4_vtable_has_expected_slot_count() {
        let slots = size_of::<IAvnApplication4Vtbl>() / size_of::<usize>();
        assert_eq!(3 + IAVN_APPLICATION4_METHOD_COUNT, slots);
    }

    #[test]
    fn clipboard_data_vtable_has_expected_slot_count() {
        let slots = size_of::<IAvnClipboardDataVtbl>() / size_of::<usize>();
        assert_eq!(3 + 2, slots);
    }

    #[test]
    fn capability_iids_are_distinct_and_stable() {
        assert_ne!(IAVN_APPLICATION4_IID, IAVN_CLIPBOARD_DATA_IID);
        assert_eq!(0x60, IAVN_APPLICATION4_IID.data4[7]);
        assert_eq!(0x61, IAVN_CLIPBOARD_DATA_IID.data4[7]);
    }

    #[test]
    fn stage_29_capability_stays_separately_versioned() {
        // Stage 31 never widens the stage 29 vtable; the two capabilities carry
        // distinct IIDs and are queried independently.
        assert_ne!(crate::storage::IAvnApplication3::IID, IAVN_APPLICATION4_IID);
    }

    #[test]
    fn clipboard_string_arguments_are_terminated_and_keep_null_distinct() {
        use std::cell::RefCell;
        thread_local! {
            static OBSERVED: RefCell<Option<String>> = const { RefCell::new(None) };
        }
        unsafe extern "system" fn query(
            _: *mut IUnknown,
            _: *const Guid,
            _: *mut *mut c_void,
        ) -> i32 {
            hresult::E_NOINTERFACE
        }
        unsafe extern "system" fn reference(_: *mut IUnknown) -> u32 {
            1
        }
        unsafe extern "system" fn read(_: *mut IAvnClipboardData, value: *const u16) -> i32 {
            OBSERVED.with(|observed| *observed.borrow_mut() = crate::clone_utf16(value));
            0
        }
        let vtbl = IAvnClipboardDataVtbl {
            query_interface: query,
            add_ref: reference,
            release: reference,
            set_text: read,
            add_file_uri: read,
        };
        let mut object = IAvnClipboardData { vtbl: &vtbl };
        let object = unsafe { ComPtr::from_raw(&mut object).unwrap() };
        for value in [
            None,
            Some(&[][..]),
            Some(&[65, 66][..]),
            Some(&[65, 0, 66][..]),
        ] {
            let expected =
                value.map(|v| String::from_utf16_lossy(v.split(|c| *c == 0).next().unwrap()));
            object.set_text(value).unwrap();
            OBSERVED.with(|observed| assert_eq!(*observed.borrow(), expected));
            object.add_file_uri(value).unwrap();
            OBSERVED.with(|observed| assert_eq!(*observed.borrow(), expected));
        }
    }
}

//! Stage 33 desktop shell integration ABI.
//!
//! `IAvnApplication6` is a separately versioned capability queried from
//! `IAvnApplication`; nothing here is added to an already published vtable. It
//! sets how the application lifetime ends, shows the tray icons a view-model
//! schema declares, and carries the window-level settings a tray or capture
//! tool needs: the native handle, topmost, per-pixel transparency and capture
//! exclusion.

use crate::application::IAvnApplication;
use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::IAvnWindow;
use crate::guid::Guid;
use crate::hresult::{self, Result};
use crate::rust_vm::IAvnRustViewModel;
use std::ffi::c_void;

const IAVN_APPLICATION6_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x80],
};

#[repr(C)]
struct IAvnApplication6Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set_shutdown_mode: unsafe extern "system" fn(*mut IAvnApplication6, i32) -> i32,
    create_rust_vm_tray:
        unsafe extern "system" fn(*mut IAvnApplication6, i32, *mut IAvnRustViewModel) -> i32,
    get_platform_handle:
        unsafe extern "system" fn(*mut IAvnApplication6, *mut IAvnWindow, *mut i64) -> i32,
    set_window_topmost:
        unsafe extern "system" fn(*mut IAvnApplication6, *mut IAvnWindow, i32) -> i32,
    set_window_transparent:
        unsafe extern "system" fn(*mut IAvnApplication6, *mut IAvnWindow, i32) -> i32,
    set_window_excluded_from_capture:
        unsafe extern "system" fn(*mut IAvnApplication6, *mut IAvnWindow, i32) -> i32,
}

#[repr(C)]
pub struct IAvnApplication6 {
    vtbl: *const IAvnApplication6Vtbl,
}

unsafe impl ComInterface for IAvnApplication6 {
    const IID: Guid = IAVN_APPLICATION6_IID;
}

impl ComPtr<IAvnApplication6> {
    fn vtbl(&self) -> &IAvnApplication6Vtbl {
        unsafe { (*self.as_raw()).vtbl.as_ref().unwrap() }
    }

    /// Sets when the application exits: 0 on last window close (the default),
    /// 1 on main window close, 2 only on an explicit shutdown.
    pub fn set_shutdown_mode(&self, mode: i32) -> Result<()> {
        unsafe { hresult::check((self.vtbl().set_shutdown_mode)(self.as_raw(), mode)) }
    }

    /// Shows the tray icon declared as `tray_id`, bound to `model`.
    pub fn create_rust_vm_tray(
        &self,
        tray_id: i32,
        model: &ComPtr<IAvnRustViewModel>,
    ) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().create_rust_vm_tray)(
                self.as_raw(),
                tray_id,
                model.as_raw(),
            ))
        }
    }

    /// The window's native handle (an `HWND` on Windows), 0 when it has none yet.
    pub fn platform_handle(&self, window: &ComPtr<IAvnWindow>) -> Result<i64> {
        unsafe {
            let mut handle = 0;
            let hr = (self.vtbl().get_platform_handle)(self.as_raw(), window.as_raw(), &mut handle);
            hresult::check(hr).map(|_| handle)
        }
    }

    pub fn set_window_topmost(&self, window: &ComPtr<IAvnWindow>, topmost: bool) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().set_window_topmost)(
                self.as_raw(),
                window.as_raw(),
                topmost as i32,
            ))
        }
    }

    pub fn set_window_transparent(
        &self,
        window: &ComPtr<IAvnWindow>,
        transparent: bool,
    ) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().set_window_transparent)(
                self.as_raw(),
                window.as_raw(),
                transparent as i32,
            ))
        }
    }

    /// Windows only; `E_NOTIMPL` elsewhere.
    pub fn set_window_excluded_from_capture(
        &self,
        window: &ComPtr<IAvnWindow>,
        excluded: bool,
    ) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().set_window_excluded_from_capture)(
                self.as_raw(),
                window.as_raw(),
                excluded as i32,
            ))
        }
    }
}

impl ComPtr<IAvnApplication> {
    /// Queries the separately versioned desktop shell capability.
    pub fn shell(&self) -> Result<ComPtr<IAvnApplication6>> {
        self.query_interface::<IAvnApplication6>()
    }
}

/// Number of methods `IAvnApplication6` publishes after `IUnknown`.
///
/// A cross-language tripwire: the managed interface and this vtable must stay
/// the same shape, and a published capability is never widened in place.
pub const IAVN_APPLICATION6_METHOD_COUNT: usize = 6;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn application6_vtable_has_expected_slot_count() {
        let slots = size_of::<IAvnApplication6Vtbl>() / size_of::<usize>();
        assert_eq!(3 + IAVN_APPLICATION6_METHOD_COUNT, slots);
    }

    #[test]
    fn application6_iid_is_distinct_and_in_the_stage_33_slot() {
        assert_eq!(0x80, IAVN_APPLICATION6_IID.data4[7]);
        assert_ne!(crate::dialogs::IAvnApplication5::IID, IAVN_APPLICATION6_IID);
        assert_ne!(
            crate::clipboard::IAvnApplication4::IID,
            IAVN_APPLICATION6_IID
        );
    }
}

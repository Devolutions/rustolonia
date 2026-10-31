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

const IAVN_APPLICATION8_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA0],
};

#[repr(C)]
struct IAvnApplication8Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_native_window_handle: unsafe extern "system" fn(
        *mut IAvnApplication8,
        *mut IAvnWindow,
        *mut i64,
        *mut i32,
    ) -> i32,
    get_macos_window_number:
        unsafe extern "system" fn(*mut IAvnApplication8, *mut IAvnWindow, *mut i32) -> i32,
    set_macos_window_ignores_mouse_events:
        unsafe extern "system" fn(*mut IAvnApplication8, *mut IAvnWindow, i32) -> i32,
    show_window: unsafe extern "system" fn(*mut IAvnApplication8, *mut IAvnWindow, i32) -> i32,
    activate_window: unsafe extern "system" fn(*mut IAvnApplication8, *mut IAvnWindow) -> i32,
}

#[repr(C)]
pub struct IAvnApplication8 {
    vtbl: *const IAvnApplication8Vtbl,
}

unsafe impl ComInterface for IAvnApplication8 {
    const IID: Guid = IAVN_APPLICATION8_IID;
}

impl ComPtr<IAvnApplication8> {
    fn vtbl(&self) -> &IAvnApplication8Vtbl {
        unsafe { (*self.as_raw()).vtbl.as_ref().unwrap() }
    }

    /// Gets the platform handle and its Avalonia handle descriptor. A zero
    /// handle and kind zero mean the window has no native handle yet.
    pub fn native_window_handle(&self, window: &ComPtr<IAvnWindow>) -> Result<(i64, i32)> {
        unsafe {
            let mut handle = 0;
            let mut kind = 0;
            let hr = (self.vtbl().get_native_window_handle)(
                self.as_raw(),
                window.as_raw(),
                &mut handle,
                &mut kind,
            );
            hresult::check(hr).map(|_| (handle, kind))
        }
    }

    /// Returns AppKit's `NSWindow.windowNumber` on macOS; other platforms
    /// return `E_NOTIMPL`.
    pub fn macos_window_number(&self, window: &ComPtr<IAvnWindow>) -> Result<i32> {
        unsafe {
            let mut number = 0;
            let hr =
                (self.vtbl().get_macos_window_number)(self.as_raw(), window.as_raw(), &mut number);
            hresult::check(hr).map(|_| number)
        }
    }

    /// Sets AppKit's `NSWindow.ignoresMouseEvents`; other platforms return
    /// `E_NOTIMPL`.
    pub fn set_macos_window_ignores_mouse_events(
        &self,
        window: &ComPtr<IAvnWindow>,
        ignores: bool,
    ) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().set_macos_window_ignores_mouse_events)(
                self.as_raw(),
                window.as_raw(),
                i32::from(ignores),
            ))
        }
    }

    /// Shows a hidden window and optionally activates it. If already visible,
    /// only `activate = true` has an effect.
    pub fn show_window(&self, window: &ComPtr<IAvnWindow>, activate: bool) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().show_window)(
                self.as_raw(),
                window.as_raw(),
                i32::from(activate),
            ))
        }
    }

    /// Activates a visible window.
    pub fn activate_window(&self, window: &ComPtr<IAvnWindow>) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().activate_window)(
                self.as_raw(),
                window.as_raw(),
            ))
        }
    }
}

impl ComPtr<IAvnApplication> {
    /// Queries the separately versioned native-window capability.
    pub fn native_windows(&self) -> Result<ComPtr<IAvnApplication8>> {
        self.query_interface::<IAvnApplication8>()
    }
}

/// Number of methods `IAvnApplication8` publishes after `IUnknown`.
pub const IAVN_APPLICATION8_METHOD_COUNT: usize = 5;

#[repr(C)]
struct IAvnApplication9Vtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    set_window_overlay_popups:
        unsafe extern "system" fn(*mut IAvnApplication9, *mut IAvnWindow, i32) -> i32,
}

#[repr(C)]
pub struct IAvnApplication9 {
    vtbl: *const IAvnApplication9Vtbl,
}

unsafe impl ComInterface for IAvnApplication9 {
    const IID: Guid = Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA1],
    };
}

impl ComPtr<IAvnApplication9> {
    pub fn set_window_overlay_popups(
        &self,
        window: &ComPtr<IAvnWindow>,
        enabled: bool,
    ) -> Result<()> {
        unsafe {
            let vtbl = (*self.as_raw()).vtbl.as_ref().unwrap();
            hresult::check((vtbl.set_window_overlay_popups)(
                self.as_raw(),
                window.as_raw(),
                i32::from(enabled),
            ))
        }
    }
}

impl ComPtr<IAvnApplication> {
    pub fn popup_hosting(&self) -> Result<ComPtr<IAvnApplication9>> {
        self.query_interface()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn application9_is_an_additive_single_method_capability() {
        assert_eq!(4 * size_of::<usize>(), size_of::<IAvnApplication9Vtbl>());
        assert_eq!(0xA1, IAvnApplication9::IID.data4[7]);
        assert_ne!(IAvnApplication8::IID, IAvnApplication9::IID);
    }

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

    #[test]
    fn application8_vtable_has_expected_slot_count() {
        let slots = size_of::<IAvnApplication8Vtbl>() / size_of::<usize>();
        assert_eq!(3 + IAVN_APPLICATION8_METHOD_COUNT, slots);
    }

    #[test]
    fn application8_iid_uses_the_next_capability_slot() {
        assert_eq!(
            Guid {
                data1: 0x6B2E8F10,
                data2: 0x4C91,
                data3: 0x4E3A,
                data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA0],
            },
            IAVN_APPLICATION8_IID
        );
        assert_ne!(IAvnApplication6::IID, IAVN_APPLICATION8_IID);
    }
}

//! Desktop shell integration.
//!
//! These `IAvnApplication` operations set how the application lifetime ends,
//! show the tray icons a view-model schema declares, and carry the window-level
//! settings a tray, capture or overlay tool needs: native handles, topmost,
//! per-pixel transparency, capture exclusion, show/activate and overlay popups.

use crate::application::IAvnApplication;
use crate::com::ComPtr;
use crate::generated::IAvnWindow;
use crate::hresult::{self, Result};
use crate::rust_vm::IAvnRustViewModel;

impl ComPtr<IAvnApplication> {
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

    pub fn set_window_overlay_popups(
        &self,
        window: &ComPtr<IAvnWindow>,
        enabled: bool,
    ) -> Result<()> {
        unsafe {
            let vtbl = self.vtbl();
            hresult::check((vtbl.set_window_overlay_popups)(
                self.as_raw(),
                window.as_raw(),
                i32::from(enabled),
            ))
        }
    }
}

//! Code-first native menus.
//!
//! `IAvnApplication` builds Avalonia `NativeMenu`s from Rust (`IAvnNativeMenu`, reporting clicks
//! to a Rust-implemented `IAvnNativeMenuHandler`) and attaches them as a
//! window's menu bar or as the macOS application menu.

use crate::application::IAvnApplication;
use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::{IAvnControl, IAvnWindow};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;

const fn iid(last: u8) -> Guid {
    Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, last],
    }
}

type QueryInterface =
    unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32;
type AddRef = unsafe extern "system" fn(*mut IUnknown) -> u32;

/// `IAvnNativeMenu::AddMenuItem` kinds.
pub const NATIVE_MENU_KIND_COMMAND: i32 = 0;
pub const NATIVE_MENU_KIND_CHECK_BOX: i32 = 1;
pub const NATIVE_MENU_KIND_SEPARATOR: i32 = 2;
pub const NATIVE_MENU_KIND_SUBMENU: i32 = 3;
pub const NATIVE_MENU_KIND_RADIO: i32 = 4;
/// `IAvnNativeMenu` item flags.
pub const NATIVE_MENU_FLAG_CHECKED: i32 = 1;
pub const NATIVE_MENU_FLAG_DISABLED: i32 = 2;
pub const NATIVE_MENU_FLAG_HIDDEN: i32 = 4;

impl ComPtr<IAvnApplication> {
    /// Creates an empty menu reporting clicks to `handler`, or one that does nothing.
    pub fn create_native_menu(
        &self,
        handler: Option<&ComPtr<IAvnNativeMenuHandler>>,
    ) -> Result<ComPtr<IAvnNativeMenu>> {
        unsafe {
            let mut menu = std::ptr::null_mut();
            hresult::check((self.vtbl().create_native_menu)(
                self.as_raw(),
                handler.map_or(std::ptr::null_mut(), |h| h.as_raw()),
                &mut menu,
            ))?;
            ComPtr::from_raw(menu).ok_or(hresult::Error(hresult::E_POINTER))
        }
    }

    /// Puts `menu`'s items at the top of the application menu; `None` removes them.
    pub fn set_application_menu(&self, menu: Option<&ComPtr<IAvnNativeMenu>>) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().set_application_menu)(
                self.as_raw(),
                menu.map_or(std::ptr::null_mut(), |m| m.as_raw()),
            ))
        }
    }

    /// Makes `menu` the window's menu bar; `None` removes it.
    pub fn set_window_menu(
        &self,
        window: &ComPtr<IAvnWindow>,
        menu: Option<&ComPtr<IAvnNativeMenu>>,
    ) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().set_window_menu)(
                self.as_raw(),
                window.as_raw(),
                menu.map_or(std::ptr::null_mut(), |m| m.as_raw()),
            ))
        }
    }

    /// Whether the platform shows the window's menu outside the window (macOS).
    pub fn is_native_menu_exported(&self, window: &ComPtr<IAvnWindow>) -> Result<bool> {
        let mut exported = 0;
        unsafe {
            hresult::check((self.vtbl().is_native_menu_exported)(
                self.as_raw(),
                window.as_raw(),
                &mut exported,
            ))?;
        }
        Ok(exported != 0)
    }

    /// A control drawing its window's menu in-window unless it is exported.
    pub fn create_native_menu_bar(&self) -> Result<ComPtr<IAvnControl>> {
        unsafe {
            let mut bar = std::ptr::null_mut();
            hresult::check((self.vtbl().create_native_menu_bar)(
                self.as_raw(),
                &mut bar,
            ))?;
            ComPtr::from_raw(bar).ok_or(hresult::Error(hresult::E_POINTER))
        }
    }
}

#[repr(C)]
struct IAvnNativeMenuVtbl {
    query_interface: QueryInterface,
    add_ref: AddRef,
    release: AddRef,
    clear_menu: unsafe extern "system" fn(*mut IAvnNativeMenu) -> i32,
    add_menu_item: unsafe extern "system" fn(
        *mut IAvnNativeMenu,
        i32,
        i32,
        i32,
        *const u16,
        *const u16,
        i32,
    ) -> i32,
    update_menu_item: unsafe extern "system" fn(*mut IAvnNativeMenu, i32, *const u16, i32) -> i32,
}

/// A native menu whose items Rust numbers itself. UI thread only.
#[repr(C)]
pub struct IAvnNativeMenu {
    vtbl: *const IAvnNativeMenuVtbl,
}

unsafe impl ComInterface for IAvnNativeMenu {
    const IID: Guid = iid(0xAB);
}

fn utf16(value: Option<&str>) -> Option<Vec<u16>> {
    value.map(|v| v.encode_utf16().chain(Some(0)).collect())
}

fn utf16_ptr(value: &Option<Vec<u16>>) -> *const u16 {
    value.as_ref().map_or(std::ptr::null(), |v| v.as_ptr())
}

impl ComPtr<IAvnNativeMenu> {
    fn vtbl(&self) -> &IAvnNativeMenuVtbl {
        unsafe { (*self.as_raw()).vtbl.as_ref().unwrap() }
    }

    pub fn clear_menu(&self) -> Result<()> {
        unsafe { hresult::check((self.vtbl().clear_menu)(self.as_raw())) }
    }

    /// Appends item `id` (positive, unique) under `parent` (a submenu id, or 0
    /// for the top level). `kind` is a `NATIVE_MENU_KIND_*`, `gesture` a key
    /// gesture such as `Ctrl+O`, `flags` a set of `NATIVE_MENU_FLAG_*`.
    pub fn add_menu_item(
        &self,
        parent: i32,
        id: i32,
        kind: i32,
        header: Option<&str>,
        gesture: Option<&str>,
        flags: i32,
    ) -> Result<()> {
        let header = utf16(header);
        let gesture = utf16(gesture);
        unsafe {
            hresult::check((self.vtbl().add_menu_item)(
                self.as_raw(),
                parent,
                id,
                kind,
                utf16_ptr(&header),
                utf16_ptr(&gesture),
                flags,
            ))
        }
    }

    /// Changes an item's header (unless `None`) and flags.
    pub fn update_menu_item(&self, id: i32, header: Option<&str>, flags: i32) -> Result<()> {
        let header = utf16(header);
        unsafe {
            hresult::check((self.vtbl().update_menu_item)(
                self.as_raw(),
                id,
                utf16_ptr(&header),
                flags,
            ))
        }
    }
}

#[repr(C)]
struct IAvnNativeMenuHandlerVtbl {
    query_interface: QueryInterface,
    add_ref: AddRef,
    release: AddRef,
    on_menu_item: unsafe extern "system" fn(*mut IAvnNativeMenuHandler, i32) -> i32,
}

/// Implemented by Rust ([`native_menu_handler`]); the host calls it on the UI thread.
#[repr(C)]
pub struct IAvnNativeMenuHandler {
    vtbl: *const IAvnNativeMenuHandlerVtbl,
}

unsafe impl ComInterface for IAvnNativeMenuHandler {
    const IID: Guid = iid(0xAC);
}

static NATIVE_MENU_HANDLER_VTBL: IAvnNativeMenuHandlerVtbl = IAvnNativeMenuHandlerVtbl {
    query_interface: handler_query_interface,
    add_ref: handler_add_ref,
    release: handler_release,
    on_menu_item: handler_on_menu_item,
};

unsafe extern "system" fn handler_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnNativeMenuHandler, i32>(this, iid, result)
}

unsafe extern "system" fn handler_add_ref(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnNativeMenuHandler, i32>(this)
}

unsafe extern "system" fn handler_release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnNativeMenuHandler, i32>(this)
}

unsafe extern "system" fn handler_on_menu_item(this: *mut IAvnNativeMenuHandler, id: i32) -> i32 {
    crate::event_callback::invoke::<IAvnNativeMenuHandler, i32>(this, &mut { id })
}

/// Builds an `IAvnNativeMenuHandler` from a closure receiving each clicked item id.
pub fn native_menu_handler(
    mut callback: impl FnMut(i32) -> Result<()> + Send + 'static,
) -> ComPtr<IAvnNativeMenuHandler> {
    crate::event_callback::create(
        IAvnNativeMenuHandler {
            vtbl: &NATIVE_MENU_HANDLER_VTBL,
        },
        move |id: &mut i32| callback(*id),
    )
}

/// Number of methods `IAvnNativeMenu` publishes after `IUnknown`.
pub const IAVN_NATIVE_MENU_METHOD_COUNT: usize = 3;
/// Number of methods `IAvnNativeMenuHandler` publishes after `IUnknown`.
pub const IAVN_NATIVE_MENU_HANDLER_METHOD_COUNT: usize = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use std::sync::{Arc, Mutex};

    fn slots<T>() -> usize {
        size_of::<T>() / size_of::<usize>()
    }

    #[test]
    fn vtables_have_expected_slot_counts() {
        assert_eq!(
            3 + IAVN_NATIVE_MENU_METHOD_COUNT,
            slots::<IAvnNativeMenuVtbl>()
        );
        assert_eq!(
            3 + IAVN_NATIVE_MENU_HANDLER_METHOD_COUNT,
            slots::<IAvnNativeMenuHandlerVtbl>()
        );
    }

    #[test]
    fn iids_are_stable() {
        assert_eq!(0xAB, IAvnNativeMenu::IID.data4[7]);
        assert_eq!(0xAC, IAvnNativeMenuHandler::IID.data4[7]);
    }

    #[test]
    fn handler_reports_clicked_ids() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        let handler = native_menu_handler(move |id| {
            log.lock().unwrap().push(id);
            Ok(())
        });
        let raw = handler.as_raw();
        unsafe {
            let vtbl = &*(*raw).vtbl;
            assert_eq!(0, (vtbl.on_menu_item)(raw, 7));
            assert_eq!(0, (vtbl.on_menu_item)(raw, 9));
        }
        assert_eq!(vec![7, 9], *seen.lock().unwrap());
    }
}

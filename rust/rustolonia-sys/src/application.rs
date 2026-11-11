use crate::app_handler::IAvnAppHandler;
use crate::async_completion::IAvnAsyncCompletion;
use crate::clipboard::IAvnClipboardData;
use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::file_drag::IAvnFileDragHandler;
use crate::generated::{AvnVariant, IAvnCommand, IAvnControl, IAvnWindow};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use crate::native_menu::{IAvnNativeMenu, IAvnNativeMenuHandler};
use crate::pixel_surface::{
    IAvnPixelSurface, IAvnPixelSurfaceHandler, IAvnShellTray, IAvnShellTrayHandler,
};
use crate::popup_gate::IAvnNativePopupHandler;
use crate::rust_vm::IAvnRustViewModel;
use crate::storage::{
    IAvnActivationHandler, IAvnFileDropHandler, IAvnFilePickerOptions, IAvnStorageCompletion,
    IAvnStorageItemList,
};
use crate::terminated_utf16;
use crate::termination::IAvnTerminationHandler;
use crate::value_converter::IAvnRustValueConverterProvider;
use std::ffi::c_void;
use std::ptr;

#[cfg(test)]
mod string_tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn empty_and_unterminated_application_arguments_are_terminated() {
        for value in [&[][..], &[65, 66][..], &[0xd800][..]] {
            let buffer = terminated_utf16(value);
            assert_eq!(&buffer[..value.len()], value);
            assert_eq!(buffer.last(), Some(&0));
            assert_eq!(buffer.len(), value.len() + 1);
        }
    }

    #[test]
    fn terminated_arguments_preserve_existing_nul_semantics_without_allocation() {
        for value in [&[0][..], &[65, 0][..], &[65, 0, 66][..]] {
            let buffer = terminated_utf16(value);
            assert!(matches!(buffer, Cow::Borrowed(_)));
            assert_eq!(buffer.as_ptr(), value.as_ptr());
        }
    }
}

const IAVN_RESOURCE_VALUE_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x16],
};

#[repr(C)]
struct IAvnResourceValueVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_kind: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i32) -> i32,
    get_boolean: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i32) -> i32,
    get_integer: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i64) -> i32,
    get_double: unsafe extern "system" fn(*mut IAvnResourceValue, *mut f64) -> i32,
    get_string: unsafe extern "system" fn(*mut IAvnResourceValue, *mut *mut u16) -> i32,
    get_color: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i32) -> i32,
}

#[repr(C)]
pub struct IAvnResourceValue {
    vtbl: *const IAvnResourceValueVtbl,
}

unsafe impl ComInterface for IAvnResourceValue {
    const IID: Guid = IAVN_RESOURCE_VALUE_IID;
}

impl ComPtr<IAvnResourceValue> {
    fn get_i32(
        &self,
        method: unsafe extern "system" fn(*mut IAvnResourceValue, *mut i32) -> i32,
    ) -> Result<i32> {
        unsafe {
            let mut value = 0;
            hresult::check(method(self.as_raw(), &mut value)).map(|_| value)
        }
    }

    pub fn kind(&self) -> Result<i32> {
        unsafe { self.get_i32((*self.as_raw()).vtbl.as_ref().unwrap().get_kind) }
    }

    pub fn boolean(&self) -> Result<bool> {
        unsafe {
            self.get_i32((*self.as_raw()).vtbl.as_ref().unwrap().get_boolean)
                .map(|value| value != 0)
        }
    }

    pub fn integer(&self) -> Result<i64> {
        unsafe {
            let mut value = 0;
            let hr =
                ((*self.as_raw()).vtbl.as_ref().unwrap().get_integer)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn double(&self) -> Result<f64> {
        unsafe {
            let mut value = 0.0;
            let hr =
                ((*self.as_raw()).vtbl.as_ref().unwrap().get_double)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn string(&self) -> Result<*mut u16> {
        unsafe {
            let mut value = ptr::null_mut();
            let hr =
                ((*self.as_raw()).vtbl.as_ref().unwrap().get_string)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn color(&self) -> Result<u32> {
        unsafe {
            self.get_i32((*self.as_raw()).vtbl.as_ref().unwrap().get_color)
                .map(|value| value as u32)
        }
    }
}

/// Number of methods `IAvnApplication` declares after `IUnknown`.
///
/// A cross-language tripwire: the managed `IAvnApplication` and
/// [`IAvnApplicationVtbl`] must list the same methods in the same order.
pub const IAVN_APPLICATION_METHOD_COUNT: usize = 64;

type App = IAvnApplication;

/// The one application vtable. Its slot order matches the managed
/// `Avalonia.Host.Com.IAvnApplication` declaration exactly; the interface is
/// changed in place (host and crates ship together), never versioned.
#[repr(C)]
pub(crate) struct IAvnApplicationVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    run: unsafe extern "system" fn(*mut App, *mut c_void) -> i32,
    shutdown: unsafe extern "system" fn(*mut App) -> i32,
    get_requested_theme_variant: unsafe extern "system" fn(*mut App, *mut i32) -> i32,
    set_requested_theme_variant: unsafe extern "system" fn(*mut App, i32) -> i32,
    get_actual_theme_variant: unsafe extern "system" fn(*mut App, *mut i32) -> i32,
    try_get_resource: unsafe extern "system" fn(
        *mut App,
        *const u16,
        i32,
        *mut i32,
        *mut *mut IAvnResourceValue,
    ) -> i32,
    start_delay:
        unsafe extern "system" fn(*mut App, i32, *mut IAvnAsyncCompletion, *mut i64) -> i32,
    start_clipboard_set_text: unsafe extern "system" fn(
        *mut App,
        *mut IAvnWindow,
        *const u16,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
    start_clipboard_get_text: unsafe extern "system" fn(
        *mut App,
        *mut IAvnWindow,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
    cancel_async_operation: unsafe extern "system" fn(*mut App, i64) -> i32,
    create_rust_vm_window: unsafe extern "system" fn(
        *mut App,
        i32,
        *mut IAvnRustViewModel,
        *mut *mut IAvnWindow,
    ) -> i32,
    set_value_converter_provider:
        unsafe extern "system" fn(*mut App, *mut IAvnRustValueConverterProvider) -> i32,

    // Desktop file integration (storage.rs).
    pub(crate) create_picker_options:
        unsafe extern "system" fn(*mut App, *mut *mut IAvnFilePickerOptions) -> i32,
    pub(crate) get_storage_capabilities:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, *mut i32) -> i32,
    pub(crate) start_open_file_picker: StartPicker,
    pub(crate) start_open_folder_picker: StartPicker,
    pub(crate) start_save_file_picker: StartPicker,
    pub(crate) subscribe_file_drop: unsafe extern "system" fn(
        *mut App,
        *mut IAvnControl,
        i32,
        *mut IAvnFileDropHandler,
        *mut i64,
    ) -> i32,
    pub(crate) unsubscribe_file_drop: unsafe extern "system" fn(*mut App, i64) -> i32,
    pub(crate) clear_startup_arguments: unsafe extern "system" fn(*mut App) -> i32,
    pub(crate) add_startup_argument: unsafe extern "system" fn(*mut App, *const u16) -> i32,
    pub(crate) get_startup_argument_count: unsafe extern "system" fn(*mut App, *mut i32) -> i32,
    pub(crate) get_startup_argument: unsafe extern "system" fn(*mut App, i32, *mut *mut u16) -> i32,
    pub(crate) get_activation_items:
        unsafe extern "system" fn(*mut App, *mut *mut IAvnStorageItemList) -> i32,
    pub(crate) advise_activation:
        unsafe extern "system" fn(*mut App, *mut IAvnActivationHandler, *mut i64) -> i32,
    pub(crate) unadvise_activation: unsafe extern "system" fn(*mut App, i64) -> i32,

    // Clipboard commands (clipboard.rs).
    pub(crate) create_clipboard_data:
        unsafe extern "system" fn(*mut App, *mut *mut IAvnClipboardData) -> i32,
    pub(crate) get_clipboard_capabilities:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, *mut i32) -> i32,
    pub(crate) start_clipboard_write: unsafe extern "system" fn(
        *mut App,
        *mut IAvnWindow,
        *mut IAvnClipboardData,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
    pub(crate) start_clipboard_clear: unsafe extern "system" fn(
        *mut App,
        *mut IAvnWindow,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,
    pub(crate) start_clipboard_read_files: unsafe extern "system" fn(
        *mut App,
        *mut IAvnWindow,
        *mut IAvnStorageCompletion,
        *mut i64,
    ) -> i32,

    // Modal dialogs (dialogs.rs).
    pub(crate) start_show_dialog: unsafe extern "system" fn(
        *mut App,
        *mut IAvnWindow,
        *mut IAvnWindow,
        *mut IAvnAsyncCompletion,
        *mut i64,
    ) -> i32,

    // Desktop shell (shell.rs).
    pub(crate) set_shutdown_mode: unsafe extern "system" fn(*mut App, i32) -> i32,
    pub(crate) create_rust_vm_tray:
        unsafe extern "system" fn(*mut App, i32, *mut IAvnRustViewModel) -> i32,
    pub(crate) get_platform_handle:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, *mut i64) -> i32,
    pub(crate) set_window_topmost: unsafe extern "system" fn(*mut App, *mut IAvnWindow, i32) -> i32,
    pub(crate) set_window_transparent:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, i32) -> i32,
    pub(crate) set_window_excluded_from_capture:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, i32) -> i32,

    // Pixel surfaces and code-first tray icons (pixel_surface.rs).
    pub(crate) create_pixel_surface: unsafe extern "system" fn(
        *mut App,
        *mut IAvnPixelSurfaceHandler,
        *mut *mut IAvnPixelSurface,
    ) -> i32,
    pub(crate) create_tray_icon: unsafe extern "system" fn(
        *mut App,
        *mut IAvnShellTrayHandler,
        *mut *mut IAvnShellTray,
    ) -> i32,

    // Native-window operations (shell.rs).
    pub(crate) get_native_window_handle:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, *mut i64, *mut i32) -> i32,
    pub(crate) get_macos_window_number:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, *mut i32) -> i32,
    pub(crate) set_macos_window_ignores_mouse_events:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, i32) -> i32,
    pub(crate) show_window: unsafe extern "system" fn(*mut App, *mut IAvnWindow, i32) -> i32,
    pub(crate) activate_window: unsafe extern "system" fn(*mut App, *mut IAvnWindow) -> i32,
    pub(crate) set_window_overlay_popups:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, i32) -> i32,

    // Native popup gate (popup_gate.rs).
    pub(crate) set_native_popup_gate:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, *mut IAvnNativePopupHandler) -> i32,
    pub(crate) acknowledge_native_popup: unsafe extern "system" fn(*mut App, i64) -> i32,

    // Termination requests (termination.rs).
    pub(crate) set_termination_handler:
        unsafe extern "system" fn(*mut App, *mut IAvnTerminationHandler) -> i32,
    pub(crate) complete_termination: unsafe extern "system" fn(*mut App, i64, i32) -> i32,

    // Outgoing file drags (file_drag.rs).
    pub(crate) set_file_drag_handler:
        unsafe extern "system" fn(*mut App, *mut IAvnControl, *mut IAvnFileDragHandler) -> i32,
    pub(crate) complete_file_drag: unsafe extern "system" fn(*mut App, i64, *const u16) -> i32,
    pub(crate) cancel_file_drag: unsafe extern "system" fn(*mut App, i64) -> i32,

    // Single-view lifetimes (main_view.rs).
    pub(crate) set_main_view: unsafe extern "system" fn(*mut App, *mut IAvnControl) -> i32,

    // Application naming (application_name.rs).
    pub(crate) set_application_name: unsafe extern "system" fn(*mut App, *const u16) -> i32,

    // Code-first native menus (native_menu.rs).
    pub(crate) create_native_menu: unsafe extern "system" fn(
        *mut App,
        *mut IAvnNativeMenuHandler,
        *mut *mut IAvnNativeMenu,
    ) -> i32,
    pub(crate) set_application_menu:
        unsafe extern "system" fn(*mut App, *mut IAvnNativeMenu) -> i32,
    pub(crate) set_window_menu:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, *mut IAvnNativeMenu) -> i32,
    pub(crate) is_native_menu_exported:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, *mut i32) -> i32,
    pub(crate) create_native_menu_bar:
        unsafe extern "system" fn(*mut App, *mut *mut IAvnControl) -> i32,

    // Code-first styles and window key bindings.
    pub(crate) apply_styles:
        unsafe extern "system" fn(*mut App, *mut IAvnControl, *const u16, *mut i64) -> i32,
    pub(crate) remove_styles: unsafe extern "system" fn(*mut App, i64) -> i32,
    pub(crate) add_window_key_binding: unsafe extern "system" fn(
        *mut App,
        *mut IAvnWindow,
        *const u16,
        *mut IAvnCommand,
        AvnVariant,
        *mut i64,
    ) -> i32,
    pub(crate) remove_window_key_binding:
        unsafe extern "system" fn(*mut App, *mut IAvnWindow, i64) -> i32,
}

pub(crate) type StartPicker = unsafe extern "system" fn(
    *mut App,
    *mut IAvnWindow,
    *mut IAvnFilePickerOptions,
    *mut IAvnStorageCompletion,
    *mut i64,
) -> i32;

#[repr(C)]
pub struct IAvnApplication {
    vtbl: *const IAvnApplicationVtbl,
}

unsafe impl ComInterface for IAvnApplication {
    const IID: Guid = Guid::IAVN_APPLICATION;
}

impl ComPtr<IAvnApplication> {
    pub(crate) fn vtbl(&self) -> &IAvnApplicationVtbl {
        unsafe { (*self.as_raw()).vtbl.as_ref().unwrap() }
    }
}

impl ComPtr<IAvnApplication> {
    pub fn run(&self, handler: &ComPtr<IAvnAppHandler>) -> Result<()> {
        unsafe {
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().run)(
                self.as_raw(),
                handler.as_raw().cast(),
            );
            hresult::check(hr)
        }
    }

    pub fn shutdown(&self) -> Result<()> {
        unsafe {
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().shutdown)(self.as_raw());
            hresult::check(hr)
        }
    }

    pub fn requested_theme_variant(&self) -> Result<i32> {
        unsafe {
            let mut value = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .get_requested_theme_variant)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn set_requested_theme_variant(&self, value: i32) -> Result<()> {
        unsafe {
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .set_requested_theme_variant)(self.as_raw(), value);
            hresult::check(hr)
        }
    }

    pub fn actual_theme_variant(&self) -> Result<i32> {
        unsafe {
            let mut value = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .get_actual_theme_variant)(self.as_raw(), &mut value);
            hresult::check(hr).map(|_| value)
        }
    }

    pub fn try_get_resource(
        &self,
        key: &[u16],
        theme_variant: i32,
    ) -> Result<Option<ComPtr<IAvnResourceValue>>> {
        let key = terminated_utf16(key);
        unsafe {
            let mut found = 0;
            let mut value = ptr::null_mut();
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().try_get_resource)(
                self.as_raw(),
                key.as_ptr(),
                theme_variant,
                &mut found,
                &mut value,
            );
            hresult::check(hr)?;
            if found == 0 {
                Ok(None)
            } else {
                ComPtr::from_raw(value)
                    .map(Some)
                    .ok_or(crate::Error(hresult::E_POINTER))
            }
        }
    }

    pub fn start_delay(
        &self,
        milliseconds: i32,
        completion: &ComPtr<IAvnAsyncCompletion>,
    ) -> Result<i64> {
        unsafe {
            let mut operation_id = 0;
            let hr = ((*self.as_raw()).vtbl.as_ref().unwrap().start_delay)(
                self.as_raw(),
                milliseconds,
                completion.as_raw(),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }

    pub fn start_clipboard_set_text(
        &self,
        window: &ComPtr<IAvnWindow>,
        text: &[u16],
        completion: &ComPtr<IAvnAsyncCompletion>,
    ) -> Result<i64> {
        let text = terminated_utf16(text);
        unsafe {
            let mut operation_id = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_clipboard_set_text)(
                self.as_raw(),
                window.as_raw(),
                text.as_ptr(),
                completion.as_raw(),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }

    pub fn start_clipboard_get_text(
        &self,
        window: &ComPtr<IAvnWindow>,
        completion: &ComPtr<IAvnAsyncCompletion>,
    ) -> Result<i64> {
        unsafe {
            let mut operation_id = 0;
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .start_clipboard_get_text)(
                self.as_raw(),
                window.as_raw(),
                completion.as_raw(),
                &mut operation_id,
            );
            hresult::check(hr).map(|_| operation_id)
        }
    }

    pub fn cancel_async_operation(&self, operation_id: i64) -> Result<()> {
        unsafe {
            hresult::check(((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .cancel_async_operation)(
                self.as_raw(), operation_id
            ))
        }
    }

    pub fn create_rust_vm_window(
        &self,
        view_id: i32,
        model: &ComPtr<IAvnRustViewModel>,
    ) -> Result<ComPtr<IAvnWindow>> {
        unsafe {
            let mut window = ptr::null_mut();
            let hr = ((*self.as_raw())
                .vtbl
                .as_ref()
                .unwrap()
                .create_rust_vm_window)(
                self.as_raw(), view_id, model.as_raw(), &mut window
            );
            hresult::check(hr)?;
            ComPtr::from_projected_raw(window)
        }
    }

    /// Registers (or clears, when `provider` is `None`) the single
    /// application-scoped Rust value-converter provider.
    pub fn set_value_converter_provider(
        &self,
        provider: Option<&ComPtr<IAvnRustValueConverterProvider>>,
    ) -> Result<()> {
        let raw = provider.map_or(ptr::null_mut(), ComPtr::as_raw);
        unsafe {
            hresult::check((self.vtbl().set_value_converter_provider)(
                self.as_raw(),
                raw,
            ))
        }
    }

    pub fn apply_styles(
        &self,
        scope_control: Option<&ComPtr<IAvnControl>>,
        rules_json: &[u16],
    ) -> Result<i64> {
        let rules_json = terminated_utf16(rules_json);
        let scope_raw = scope_control.map_or(ptr::null_mut(), ComPtr::as_raw);
        unsafe {
            let mut handle = 0i64;
            let hr = (self.vtbl().apply_styles)(
                self.as_raw(),
                scope_raw,
                rules_json.as_ptr(),
                &mut handle,
            );
            hresult::check(hr).map(|_| handle)
        }
    }

    pub fn remove_styles(&self, style_handle: i64) -> Result<()> {
        unsafe { hresult::check((self.vtbl().remove_styles)(self.as_raw(), style_handle)) }
    }

    pub fn add_window_key_binding(
        &self,
        window: &ComPtr<IAvnWindow>,
        gesture: &[u16],
        command: &ComPtr<IAvnCommand>,
        parameter: AvnVariant,
    ) -> Result<i64> {
        let gesture = terminated_utf16(gesture);
        unsafe {
            let mut binding_id = 0i64;
            let hr = (self.vtbl().add_window_key_binding)(
                self.as_raw(),
                window.as_raw(),
                gesture.as_ptr(),
                command.as_raw(),
                parameter,
                &mut binding_id,
            );
            hresult::check(hr).map(|_| binding_id)
        }
    }

    pub fn remove_window_key_binding(
        &self,
        window: &ComPtr<IAvnWindow>,
        binding_id: i64,
    ) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().remove_window_key_binding)(
                self.as_raw(),
                window.as_raw(),
                binding_id,
            ))
        }
    }
}

#[cfg(test)]
mod vtable_tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn application_vtable_has_one_slot_per_managed_method() {
        assert_eq!(
            3 + IAVN_APPLICATION_METHOD_COUNT,
            size_of::<IAvnApplicationVtbl>() / size_of::<usize>()
        );
    }
}

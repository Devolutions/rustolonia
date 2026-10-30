//! Stage 34 code-first desktop ABI.
//!
//! `IAvnApplication7` is a separately versioned capability queried from
//! `IAvnApplication`; nothing here is added to an already published vtable. It
//! creates pixel surfaces: host controls that draw bitmap layers Rust fills
//! (`IAvnPixelSurface`) and report their raw pointer and keyboard input to a
//! Rust-implemented `IAvnPixelSurfaceHandler`; and tray icons whose menu Rust
//! builds (`IAvnShellTray`, reporting to an `IAvnShellTrayHandler`).

use crate::application::IAvnApplication;
use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::IAvnControl;
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Mutex;

const fn stage34_iid(last: u8) -> Guid {
    Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, last],
    }
}

const IAVN_APPLICATION7_IID: Guid = stage34_iid(0x90);
const IAVN_PIXEL_SURFACE_IID: Guid = stage34_iid(0x91);
const IAVN_PIXEL_SURFACE_HANDLER_IID: Guid = stage34_iid(0x92);
const IAVN_SHELL_TRAY_IID: Guid = stage34_iid(0x93);
const IAVN_SHELL_TRAY_HANDLER_IID: Guid = stage34_iid(0x94);

type QueryInterface =
    unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32;
type AddRef = unsafe extern "system" fn(*mut IUnknown) -> u32;

#[repr(C)]
struct IAvnApplication7Vtbl {
    query_interface: QueryInterface,
    add_ref: AddRef,
    release: AddRef,
    create_pixel_surface: unsafe extern "system" fn(
        *mut IAvnApplication7,
        *mut IAvnPixelSurfaceHandler,
        *mut *mut IAvnPixelSurface,
    ) -> i32,
    create_tray_icon: unsafe extern "system" fn(
        *mut IAvnApplication7,
        *mut IAvnShellTrayHandler,
        *mut *mut IAvnShellTray,
    ) -> i32,
}

#[repr(C)]
pub struct IAvnApplication7 {
    vtbl: *const IAvnApplication7Vtbl,
}

unsafe impl ComInterface for IAvnApplication7 {
    const IID: Guid = IAVN_APPLICATION7_IID;
}

impl ComPtr<IAvnApplication7> {
    fn vtbl(&self) -> &IAvnApplication7Vtbl {
        unsafe { (*self.as_raw()).vtbl.as_ref().unwrap() }
    }

    /// Creates a pixel surface reporting its input to `handler`, or a
    /// display-only one.
    pub fn create_pixel_surface(
        &self,
        handler: Option<&ComPtr<IAvnPixelSurfaceHandler>>,
    ) -> Result<ComPtr<IAvnPixelSurface>> {
        unsafe {
            let mut surface = std::ptr::null_mut();
            hresult::check((self.vtbl().create_pixel_surface)(
                self.as_raw(),
                handler.map_or(std::ptr::null_mut(), |h| h.as_raw()),
                &mut surface,
            ))?;
            ComPtr::from_raw(surface).ok_or(hresult::Error(hresult::E_POINTER))
        }
    }

    /// Shows a tray icon reporting clicks to `handler`, or one that does nothing.
    pub fn create_tray_icon(
        &self,
        handler: Option<&ComPtr<IAvnShellTrayHandler>>,
    ) -> Result<ComPtr<IAvnShellTray>> {
        unsafe {
            let mut tray = std::ptr::null_mut();
            hresult::check((self.vtbl().create_tray_icon)(
                self.as_raw(),
                handler.map_or(std::ptr::null_mut(), |h| h.as_raw()),
                &mut tray,
            ))?;
            ComPtr::from_raw(tray).ok_or(hresult::Error(hresult::E_POINTER))
        }
    }
}

impl ComPtr<IAvnApplication> {
    /// Queries the separately versioned Rust-drawn surfaces capability.
    pub fn pixel_surfaces(&self) -> Result<ComPtr<IAvnApplication7>> {
        self.query_interface::<IAvnApplication7>()
    }
}

#[repr(C)]
struct IAvnPixelSurfaceVtbl {
    query_interface: QueryInterface,
    add_ref: AddRef,
    release: AddRef,
    get_control: unsafe extern "system" fn(*mut IAvnPixelSurface, *mut *mut IAvnControl) -> i32,
    set_layer: unsafe extern "system" fn(*mut IAvnPixelSurface, i32, i32, i32, i32) -> i32,
    write_layer: unsafe extern "system" fn(
        *mut IAvnPixelSurface,
        i32,
        i32,
        i32,
        i32,
        i32,
        *const u8,
        i32,
    ) -> i32,
    place_layer: unsafe extern "system" fn(
        *mut IAvnPixelSurface,
        i32,
        f64,
        f64,
        f64,
        f64,
        i32,
        i32,
        f64,
    ) -> i32,
    set_background: unsafe extern "system" fn(*mut IAvnPixelSurface, u32) -> i32,
    set_cursor: unsafe extern "system" fn(*mut IAvnPixelSurface, i32) -> i32,
    set_desired_size: unsafe extern "system" fn(*mut IAvnPixelSurface, f64, f64) -> i32,
    focus: unsafe extern "system" fn(*mut IAvnPixelSurface) -> i32,
    get_metrics:
        unsafe extern "system" fn(*mut IAvnPixelSurface, *mut f64, *mut f64, *mut f64) -> i32,
    detach: unsafe extern "system" fn(*mut IAvnPixelSurface) -> i32,
}

/// A host-owned control drawing bitmap layers. UI thread only.
#[repr(C)]
pub struct IAvnPixelSurface {
    vtbl: *const IAvnPixelSurfaceVtbl,
}

unsafe impl ComInterface for IAvnPixelSurface {
    const IID: Guid = IAVN_PIXEL_SURFACE_IID;
}

impl ComPtr<IAvnPixelSurface> {
    fn vtbl(&self) -> &IAvnPixelSurfaceVtbl {
        unsafe { (*self.as_raw()).vtbl.as_ref().unwrap() }
    }

    pub fn control(&self) -> Result<ComPtr<IAvnControl>> {
        unsafe {
            let mut control = std::ptr::null_mut();
            hresult::check((self.vtbl().get_control)(self.as_raw(), &mut control))?;
            ComPtr::from_raw(control).ok_or(hresult::Error(hresult::E_POINTER))
        }
    }

    /// Allocates (or with a zero size removes) a layer: format 0 is
    /// premultiplied RGBA8888, 1 premultiplied BGRA8888.
    pub fn set_layer(&self, layer: i32, width: i32, height: i32, format: i32) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().set_layer)(
                self.as_raw(),
                layer,
                width,
                height,
                format,
            ))
        }
    }

    /// Copies `width` by `height` pixels, `stride` bytes per row, into the
    /// layer at (`x`, `y`).
    ///
    /// # Errors
    ///
    /// `E_INVALIDARG` when `pixels` is too short for the rectangle.
    #[allow(clippy::too_many_arguments)]
    pub fn write_layer(
        &self,
        layer: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        pixels: &[u8],
        stride: i32,
    ) -> Result<()> {
        if width < 0 || height < 0 || stride < width.saturating_mul(4) {
            return Err(hresult::Error(hresult::E_INVALIDARG));
        }
        if height > 0 && width > 0 {
            let needed = (height as usize - 1) * stride as usize + width as usize * 4;
            if pixels.len() < needed {
                return Err(hresult::Error(hresult::E_INVALIDARG));
            }
        }
        unsafe {
            hresult::check((self.vtbl().write_layer)(
                self.as_raw(),
                layer,
                x,
                y,
                width,
                height,
                pixels.as_ptr(),
                stride,
            ))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn place_layer(
        &self,
        layer: i32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: bool,
        interpolation: i32,
        opacity: f64,
    ) -> Result<()> {
        unsafe {
            hresult::check((self.vtbl().place_layer)(
                self.as_raw(),
                layer,
                x,
                y,
                width,
                height,
                fill as i32,
                interpolation,
                opacity,
            ))
        }
    }

    pub fn set_background(&self, argb: u32) -> Result<()> {
        unsafe { hresult::check((self.vtbl().set_background)(self.as_raw(), argb)) }
    }

    pub fn set_cursor(&self, cursor: i32) -> Result<()> {
        unsafe { hresult::check((self.vtbl().set_cursor)(self.as_raw(), cursor)) }
    }

    pub fn set_desired_size(&self, width: f64, height: f64) -> Result<()> {
        unsafe { hresult::check((self.vtbl().set_desired_size)(self.as_raw(), width, height)) }
    }

    pub fn focus(&self) -> Result<()> {
        unsafe { hresult::check((self.vtbl().focus)(self.as_raw())) }
    }

    /// The control's size in device-independent pixels and its render scaling.
    pub fn metrics(&self) -> Result<(f64, f64, f64)> {
        unsafe {
            let (mut width, mut height, mut scaling) = (0.0, 0.0, 1.0);
            hresult::check((self.vtbl().get_metrics)(
                self.as_raw(),
                &mut width,
                &mut height,
                &mut scaling,
            ))?;
            Ok((width, height, scaling))
        }
    }

    pub fn detach(&self) -> Result<()> {
        unsafe { hresult::check((self.vtbl().detach)(self.as_raw())) }
    }
}

/// One input notification from a pixel surface, as the host reports it.
#[derive(Clone, Debug, PartialEq)]
pub enum PixelSurfaceInput {
    /// `kind`: 0 pressed, 1 moved, 2 released, 3 wheel, 4 entered, 5 exited,
    /// 6 capture lost.
    Pointer {
        kind: i32,
        x: f64,
        y: f64,
        button: i32,
        buttons: i32,
        modifiers: i32,
        click_count: i32,
        delta_x: f64,
        delta_y: f64,
    },
    /// `kind`: 0 down, 1 up.
    Key {
        kind: i32,
        key: i32,
        modifiers: i32,
    },
    Text(String),
    Resize {
        width: f64,
        height: f64,
        scaling: f64,
    },
}

type HandlerCallback = Box<dyn FnMut(&PixelSurfaceInput) -> Result<bool> + Send>;

#[repr(C)]
struct IAvnPixelSurfaceHandlerVtbl {
    query_interface: QueryInterface,
    add_ref: AddRef,
    release: AddRef,
    on_pointer: unsafe extern "system" fn(
        *mut IAvnPixelSurfaceHandler,
        i32,
        f64,
        f64,
        i32,
        i32,
        i32,
        i32,
        f64,
        f64,
        *mut i32,
    ) -> i32,
    on_key: unsafe extern "system" fn(*mut IAvnPixelSurfaceHandler, i32, i32, i32, *mut i32) -> i32,
    on_text_input:
        unsafe extern "system" fn(*mut IAvnPixelSurfaceHandler, *const u16, *mut i32) -> i32,
    on_resize: unsafe extern "system" fn(*mut IAvnPixelSurfaceHandler, f64, f64, f64) -> i32,
}

/// Implemented by Rust ([`pixel_surface_handler`]); the host calls it on the
/// UI thread.
#[repr(C)]
pub struct IAvnPixelSurfaceHandler {
    vtbl: *const IAvnPixelSurfaceHandlerVtbl,
}

unsafe impl ComInterface for IAvnPixelSurfaceHandler {
    const IID: Guid = IAVN_PIXEL_SURFACE_HANDLER_IID;
}

#[repr(C)]
struct HandlerObject {
    vtbl: *const IAvnPixelSurfaceHandlerVtbl,
    ref_count: AtomicU32,
    callback: Mutex<Option<HandlerCallback>>,
}

static HANDLER_VTBL: IAvnPixelSurfaceHandlerVtbl = IAvnPixelSurfaceHandlerVtbl {
    query_interface: handler_query_interface,
    add_ref: handler_add_ref,
    release: handler_release,
    on_pointer: handler_on_pointer,
    on_key: handler_on_key,
    on_text_input: handler_on_text_input,
    on_resize: handler_on_resize,
};

/// Builds an `IAvnPixelSurfaceHandler` from a closure that receives each input
/// notification and returns whether it handled it. A notification raised while
/// the closure is already running (re-entrantly) is reported unhandled.
pub fn pixel_surface_handler(
    callback: impl FnMut(&PixelSurfaceInput) -> Result<bool> + Send + 'static,
) -> ComPtr<IAvnPixelSurfaceHandler> {
    let object = Box::into_raw(Box::new(HandlerObject {
        vtbl: &HANDLER_VTBL,
        ref_count: AtomicU32::new(1),
        callback: Mutex::new(Some(Box::new(callback))),
    }));
    unsafe {
        ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
    }
}

unsafe fn dispatch(this: *mut IAvnPixelSurfaceHandler, input: &PixelSurfaceInput) -> Result<bool> {
    let object = this.cast::<HandlerObject>();
    let taken = match (*object).callback.lock() {
        Ok(mut slot) => slot.take(),
        Err(_) => return Err(hresult::Error(hresult::E_FAIL)),
    };
    // Re-entered from inside the callback: nothing to call.
    let Some(mut callback) = taken else {
        return Ok(false);
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| callback(input)));
    if let Ok(mut slot) = (*object).callback.lock() {
        *slot = Some(callback);
    }
    outcome.unwrap_or(Err(hresult::Error(hresult::E_FAIL)))
}

unsafe fn report(
    this: *mut IAvnPixelSurfaceHandler,
    input: PixelSurfaceInput,
    handled: *mut i32,
) -> i32 {
    match dispatch(this, &input) {
        Ok(done) => {
            if !handled.is_null() {
                *handled = done as i32;
            }
            hresult::S_OK
        }
        Err(error) => error.0,
    }
}

unsafe extern "system" fn handler_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    *result = std::ptr::null_mut();
    if *iid != Guid::IUNKNOWN && *iid != IAvnPixelSurfaceHandler::IID {
        return hresult::E_NOINTERFACE;
    }
    handler_add_ref(this);
    *result = this.cast();
    hresult::S_OK
}

unsafe extern "system" fn handler_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<HandlerObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn handler_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<HandlerObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn handler_on_pointer(
    this: *mut IAvnPixelSurfaceHandler,
    kind: i32,
    x: f64,
    y: f64,
    button: i32,
    buttons: i32,
    modifiers: i32,
    click_count: i32,
    delta_x: f64,
    delta_y: f64,
    handled: *mut i32,
) -> i32 {
    let input = PixelSurfaceInput::Pointer {
        kind,
        x,
        y,
        button,
        buttons,
        modifiers,
        click_count,
        delta_x,
        delta_y,
    };
    report(this, input, handled)
}

unsafe extern "system" fn handler_on_key(
    this: *mut IAvnPixelSurfaceHandler,
    kind: i32,
    key: i32,
    modifiers: i32,
    handled: *mut i32,
) -> i32 {
    report(
        this,
        PixelSurfaceInput::Key {
            kind,
            key,
            modifiers,
        },
        handled,
    )
}

unsafe extern "system" fn handler_on_text_input(
    this: *mut IAvnPixelSurfaceHandler,
    text: *const u16,
    handled: *mut i32,
) -> i32 {
    let text = crate::clone_utf16(text).unwrap_or_default();
    report(this, PixelSurfaceInput::Text(text), handled)
}

unsafe extern "system" fn handler_on_resize(
    this: *mut IAvnPixelSurfaceHandler,
    width: f64,
    height: f64,
    scaling: f64,
) -> i32 {
    let input = PixelSurfaceInput::Resize {
        width,
        height,
        scaling,
    };
    report(this, input, std::ptr::null_mut())
}

#[repr(C)]
struct IAvnShellTrayVtbl {
    query_interface: QueryInterface,
    add_ref: AddRef,
    release: AddRef,
    set_icon: unsafe extern "system" fn(*mut IAvnShellTray, *const u16) -> i32,
    set_tool_tip: unsafe extern "system" fn(*mut IAvnShellTray, *const u16) -> i32,
    set_visible: unsafe extern "system" fn(*mut IAvnShellTray, i32) -> i32,
    clear_menu: unsafe extern "system" fn(*mut IAvnShellTray) -> i32,
    add_menu_item:
        unsafe extern "system" fn(*mut IAvnShellTray, i32, i32, i32, *const u16, i32) -> i32,
    update_menu_item: unsafe extern "system" fn(*mut IAvnShellTray, i32, *const u16, i32) -> i32,
    remove: unsafe extern "system" fn(*mut IAvnShellTray) -> i32,
}

/// A notification-area icon whose menu Rust builds. UI thread only.
#[repr(C)]
pub struct IAvnShellTray {
    vtbl: *const IAvnShellTrayVtbl,
}

unsafe impl ComInterface for IAvnShellTray {
    const IID: Guid = IAVN_SHELL_TRAY_IID;
}

fn utf16(value: Option<&str>) -> Option<Vec<u16>> {
    value.map(|v| v.encode_utf16().chain(Some(0)).collect())
}

fn utf16_ptr(value: &Option<Vec<u16>>) -> *const u16 {
    value.as_ref().map_or(std::ptr::null(), |v| v.as_ptr())
}

impl ComPtr<IAvnShellTray> {
    fn vtbl(&self) -> &IAvnShellTrayVtbl {
        unsafe { (*self.as_raw()).vtbl.as_ref().unwrap() }
    }

    /// A file path or `avares://` URI; `None` removes the image.
    pub fn set_icon(&self, source: Option<&str>) -> Result<()> {
        let source = utf16(source);
        unsafe { hresult::check((self.vtbl().set_icon)(self.as_raw(), utf16_ptr(&source))) }
    }

    pub fn set_tool_tip(&self, text: Option<&str>) -> Result<()> {
        let text = utf16(text);
        unsafe { hresult::check((self.vtbl().set_tool_tip)(self.as_raw(), utf16_ptr(&text))) }
    }

    pub fn set_visible(&self, visible: bool) -> Result<()> {
        unsafe { hresult::check((self.vtbl().set_visible)(self.as_raw(), visible as i32)) }
    }

    pub fn clear_menu(&self) -> Result<()> {
        unsafe { hresult::check((self.vtbl().clear_menu)(self.as_raw())) }
    }

    /// `kind`: 0 command, 1 check box, 2 separator, 3 submenu; `flags`: bit 0
    /// checked, bit 1 disabled; `parent` 0 for the top level.
    pub fn add_menu_item(
        &self,
        parent: i32,
        id: i32,
        kind: i32,
        header: Option<&str>,
        flags: i32,
    ) -> Result<()> {
        let header = utf16(header);
        unsafe {
            hresult::check((self.vtbl().add_menu_item)(
                self.as_raw(),
                parent,
                id,
                kind,
                utf16_ptr(&header),
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

    pub fn remove(&self) -> Result<()> {
        unsafe { hresult::check((self.vtbl().remove)(self.as_raw())) }
    }
}

/// What was clicked on a tray icon: the icon itself, or menu item `id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayInput {
    Activate,
    MenuItem(i32),
}

#[repr(C)]
struct IAvnShellTrayHandlerVtbl {
    query_interface: QueryInterface,
    add_ref: AddRef,
    release: AddRef,
    on_activate: unsafe extern "system" fn(*mut IAvnShellTrayHandler) -> i32,
    on_menu_item: unsafe extern "system" fn(*mut IAvnShellTrayHandler, i32) -> i32,
}

/// Implemented by Rust ([`tray_handler`]); the host calls it on the UI thread.
#[repr(C)]
pub struct IAvnShellTrayHandler {
    vtbl: *const IAvnShellTrayHandlerVtbl,
}

unsafe impl ComInterface for IAvnShellTrayHandler {
    const IID: Guid = IAVN_SHELL_TRAY_HANDLER_IID;
}

static TRAY_HANDLER_VTBL: IAvnShellTrayHandlerVtbl = IAvnShellTrayHandlerVtbl {
    query_interface: tray_query_interface,
    add_ref: tray_add_ref,
    release: tray_release,
    on_activate: tray_on_activate,
    on_menu_item: tray_on_menu_item,
};

unsafe extern "system" fn tray_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnShellTrayHandler, TrayInput>(this, iid, result)
}

unsafe extern "system" fn tray_add_ref(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnShellTrayHandler, TrayInput>(this)
}

unsafe extern "system" fn tray_release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnShellTrayHandler, TrayInput>(this)
}

/// Builds an `IAvnShellTrayHandler` from a closure receiving each click.
pub fn tray_handler(
    mut callback: impl FnMut(TrayInput) -> Result<()> + Send + 'static,
) -> ComPtr<IAvnShellTrayHandler> {
    crate::event_callback::create(
        IAvnShellTrayHandler {
            vtbl: &TRAY_HANDLER_VTBL,
        },
        move |input: &mut TrayInput| callback(*input),
    )
}

unsafe extern "system" fn tray_on_activate(this: *mut IAvnShellTrayHandler) -> i32 {
    crate::event_callback::invoke::<IAvnShellTrayHandler, TrayInput>(this, &mut TrayInput::Activate)
}

unsafe extern "system" fn tray_on_menu_item(this: *mut IAvnShellTrayHandler, id: i32) -> i32 {
    crate::event_callback::invoke::<IAvnShellTrayHandler, TrayInput>(
        this,
        &mut TrayInput::MenuItem(id),
    )
}

/// Number of methods `IAvnApplication7` publishes after `IUnknown`.
pub const IAVN_APPLICATION7_METHOD_COUNT: usize = 2;
/// Number of methods `IAvnShellTray` publishes after `IUnknown`.
pub const IAVN_SHELL_TRAY_METHOD_COUNT: usize = 7;
/// Number of methods `IAvnShellTrayHandler` publishes after `IUnknown`.
pub const IAVN_SHELL_TRAY_HANDLER_METHOD_COUNT: usize = 2;
/// Number of methods `IAvnPixelSurface` publishes after `IUnknown`.
pub const IAVN_PIXEL_SURFACE_METHOD_COUNT: usize = 10;
/// Number of methods `IAvnPixelSurfaceHandler` publishes after `IUnknown`.
pub const IAVN_PIXEL_SURFACE_HANDLER_METHOD_COUNT: usize = 4;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    fn slots<T>() -> usize {
        size_of::<T>() / size_of::<usize>()
    }

    #[test]
    fn vtables_have_expected_slot_counts() {
        assert_eq!(
            3 + IAVN_APPLICATION7_METHOD_COUNT,
            slots::<IAvnApplication7Vtbl>()
        );
        assert_eq!(
            3 + IAVN_PIXEL_SURFACE_METHOD_COUNT,
            slots::<IAvnPixelSurfaceVtbl>()
        );
        assert_eq!(
            3 + IAVN_PIXEL_SURFACE_HANDLER_METHOD_COUNT,
            slots::<IAvnPixelSurfaceHandlerVtbl>()
        );
        assert_eq!(
            3 + IAVN_SHELL_TRAY_METHOD_COUNT,
            slots::<IAvnShellTrayVtbl>()
        );
        assert_eq!(
            3 + IAVN_SHELL_TRAY_HANDLER_METHOD_COUNT,
            slots::<IAvnShellTrayHandlerVtbl>()
        );
    }

    #[test]
    fn tray_handler_reports_clicks() {
        let seen = std::sync::Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        let handler = tray_handler(move |input| {
            log.lock().unwrap().push(input);
            Ok(())
        });
        let raw = handler.as_raw();
        unsafe {
            let vtbl = &*(*raw).vtbl;
            assert_eq!(0, (vtbl.on_activate)(raw));
            assert_eq!(0, (vtbl.on_menu_item)(raw, 7));
        }
        assert_eq!(
            vec![TrayInput::Activate, TrayInput::MenuItem(7)],
            *seen.lock().unwrap()
        );
    }

    #[test]
    fn iids_are_in_the_stage_34_slot() {
        assert_eq!(0x90, IAVN_APPLICATION7_IID.data4[7]);
        assert_ne!(crate::shell::IAvnApplication6::IID, IAVN_APPLICATION7_IID);
    }

    #[test]
    fn handler_reports_input_and_handled() {
        let seen = std::sync::Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        let handler = pixel_surface_handler(move |input| {
            log.lock().unwrap().push(input.clone());
            Ok(matches!(input, PixelSurfaceInput::Key { .. }))
        });
        let raw = handler.as_raw();
        unsafe {
            let vtbl = &*(*raw).vtbl;
            let mut handled = -1;
            assert_eq!(0, (vtbl.on_key)(raw, 0, 44, 2, &mut handled));
            assert_eq!(1, handled);
            assert_eq!(
                0,
                (vtbl.on_pointer)(raw, 0, 1.5, 2.5, 1, 1, 0, 1, 0.0, 0.0, &mut handled)
            );
            assert_eq!(0, handled);
            assert_eq!(0, (vtbl.on_resize)(raw, 10.0, 20.0, 1.5));
        }
        let seen = seen.lock().unwrap();
        assert_eq!(3, seen.len());
        assert_eq!(
            PixelSurfaceInput::Resize {
                width: 10.0,
                height: 20.0,
                scaling: 1.5
            },
            seen[2]
        );
    }
}

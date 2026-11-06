use crate::{ComInterface, ComPtr, Guid, IAvnApplication, IAvnControl, IUnknown, Result};
use std::ffi::c_void;

impl ComPtr<IAvnApplication> {
    pub fn set_file_drag_handler(
        &self,
        control: &ComPtr<IAvnControl>,
        handler: Option<&ComPtr<IAvnFileDragHandler>>,
    ) -> Result<()> {
        unsafe {
            crate::hresult::check((self.vtbl().set_file_drag_handler)(
                self.as_raw(),
                control.as_raw(),
                handler.map_or(std::ptr::null_mut(), |value| value.as_raw()),
            ))
        }
    }
    pub fn complete_file_drag(&self, token: i64, json: &str) -> Result<()> {
        let json: Vec<_> = json.encode_utf16().chain(Some(0)).collect();
        unsafe {
            crate::hresult::check((self.vtbl().complete_file_drag)(
                self.as_raw(),
                token,
                json.as_ptr(),
            ))
        }
    }
    pub fn cancel_file_drag(&self, token: i64) -> Result<()> {
        unsafe { crate::hresult::check((self.vtbl().cancel_file_drag)(self.as_raw(), token)) }
    }
}
#[repr(C)]
struct HandlerVtbl {
    query: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    preparing: unsafe extern "system" fn(*mut IAvnFileDragHandler, i64) -> i32,
    completed: unsafe extern "system" fn(*mut IAvnFileDragHandler, i64, i32, i32) -> i32,
}
#[repr(C)]
pub struct IAvnFileDragHandler {
    vtbl: *const HandlerVtbl,
}
unsafe impl ComInterface for IAvnFileDragHandler {
    const IID: Guid = Guid {
        data1: 0x6B2E8F10,
        data2: 0x4C91,
        data3: 0x4E3A,
        data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0xA7],
    };
}
#[derive(Clone, Copy, Debug)]
pub enum FileDragEvent {
    Preparing {
        token: i64,
    },
    Completed {
        token: i64,
        effect: i32,
        status: i32,
    },
}
unsafe extern "system" fn query(
    this: *mut IUnknown,
    iid: *const Guid,
    out: *mut *mut c_void,
) -> i32 {
    crate::event_callback::query_interface::<IAvnFileDragHandler, FileDragEvent>(this, iid, out)
}
unsafe extern "system" fn add(this: *mut IUnknown) -> u32 {
    crate::event_callback::add_ref::<IAvnFileDragHandler, FileDragEvent>(this)
}
unsafe extern "system" fn release(this: *mut IUnknown) -> u32 {
    crate::event_callback::release::<IAvnFileDragHandler, FileDragEvent>(this)
}
unsafe extern "system" fn preparing(this: *mut IAvnFileDragHandler, token: i64) -> i32 {
    crate::event_callback::invoke::<IAvnFileDragHandler, FileDragEvent>(
        this,
        &mut FileDragEvent::Preparing { token },
    )
}
unsafe extern "system" fn completed(
    this: *mut IAvnFileDragHandler,
    token: i64,
    effect: i32,
    status: i32,
) -> i32 {
    crate::event_callback::invoke::<IAvnFileDragHandler, FileDragEvent>(
        this,
        &mut FileDragEvent::Completed {
            token,
            effect,
            status,
        },
    )
}
static HANDLER: HandlerVtbl = HandlerVtbl {
    query,
    add,
    release,
    preparing,
    completed,
};
pub fn file_drag_handler(
    mut callback: impl FnMut(FileDragEvent) -> Result<()> + Send + 'static,
) -> ComPtr<IAvnFileDragHandler> {
    crate::event_callback::create(
        IAvnFileDragHandler { vtbl: &HANDLER },
        move |event: &mut FileDragEvent| callback(*event),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handler_vtable_shape() {
        assert_eq!(
            std::mem::size_of::<HandlerVtbl>(),
            5 * std::mem::size_of::<usize>()
        );
    }

    #[test]
    fn handler_projection_retains_callback_and_dispatches_both_slots() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Lifetime(Arc<AtomicUsize>);
        impl Drop for Lifetime {
            fn drop(&mut self) {
                self.0.fetch_add(100, Ordering::SeqCst);
            }
        }
        let count = Arc::new(AtomicUsize::new(0));
        let probe = Lifetime(count.clone());
        let handler = file_drag_handler(move |event| {
            let _ = &probe;
            match event {
                FileDragEvent::Preparing { token: 7 }
                | FileDragEvent::Completed {
                    token: 7,
                    effect: 1,
                    status: 0,
                } => {
                    probe.0.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
                _ => panic!("unexpected callback ABI arguments"),
            }
        });
        let retained = handler.query_interface::<IAvnFileDragHandler>().unwrap();
        drop(handler);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        unsafe {
            let vtbl = &*(*retained.as_raw()).vtbl;
            assert_eq!((vtbl.preparing)(retained.as_raw(), 7), 0);
            assert_eq!((vtbl.completed)(retained.as_raw(), 7, 1, 0), 0);
        }
        assert_eq!(count.load(Ordering::SeqCst), 2);
        drop(retained);
        assert_eq!(count.load(Ordering::SeqCst), 102);
    }
}

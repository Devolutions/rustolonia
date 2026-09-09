//! Rust-side `IAvnAsyncPopulator` implementation for AutoCompleteBox's async
//! population. `async_populator()` builds a ref-counted CCW the NativeAOT host
//! consumes as the managed delegate; the closure receives the search text and a
//! completion reporter whose `complete` slot carries the items back.

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::generated::{
    AvnVariant, IAvnAsyncPopulator, IAvnAsyncPopulatorCompletion, IAvnVariantList,
};
use crate::guid::Guid;
use crate::hresult::{self, Result};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Mutex;

/// Reports the populated items back to the host: one variant list per request.
pub struct PopulateCompletion {
    ptr: ComPtr<IAvnAsyncPopulatorCompletion>,
}

impl PopulateCompletion {
    /// Reports an empty population result.
    pub fn complete(&self, request_id: i64) -> Result<()> {
        self.ptr.complete(request_id, 0, std::ptr::null_mut())
    }

    pub fn complete_with(&self, request_id: i64, hresult_value: i32) -> Result<()> {
        self.ptr
            .complete(request_id, hresult_value, std::ptr::null_mut())
    }

    /// Reports suggestions from an ABI variant list. The list is borrowed for
    /// this call, not transferred; a receiver retaining it must AddRef it.
    /// The host materializes the list synchronously, releasing each owned
    /// UTF-16 payload returned by GetAt. Input strings passed to the list's Add
    /// method remain borrowed and must stay alive until Add returns.
    pub fn complete_items(&self, request_id: i64, items: &ComPtr<IAvnVariantList>) -> Result<()> {
        self.ptr.complete(request_id, hresult::S_OK, items.as_raw())
    }

    /// Reports Rust-owned string suggestions. Each GetAt allocates a fresh
    /// host-owned UTF-16 buffer; the receiver must release it with avn_free.
    /// The list itself remains valid if the receiver retains it with AddRef.
    pub fn complete_strings(&self, request_id: i64, items: Vec<String>) -> Result<()> {
        let alloc = crate::alloc_utf16_provider().ok_or(hresult::Error(hresult::E_NOTIMPL))?;
        let items = string_list(items, alloc)?;
        self.complete_items(request_id, &items)
    }
}

type AsyncPopulatorCallback =
    Box<dyn FnMut(i64, *const u16, PopulateCompletion) -> Result<()> + Send>;

#[repr(C)]
struct AsyncPopulatorVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    begin_populate: unsafe extern "system" fn(
        *mut IAvnAsyncPopulator,
        i64,
        *mut IAvnAsyncPopulatorCompletion,
        *const u16,
    ) -> i32,
}

#[repr(C)]
struct AsyncPopulatorObject {
    vtbl: *const AsyncPopulatorVtbl,
    ref_count: AtomicU32,
    callback: Mutex<Option<AsyncPopulatorCallback>>,
}

/// Builds an `IAvnAsyncPopulator` from a Rust closure receiving the request id,
/// the borrowed search text, and a completion reporter.
pub fn async_populator(
    callback: impl FnMut(i64, *const u16, PopulateCompletion) -> Result<()> + Send + 'static,
) -> AsyncPopulator {
    let object = Box::into_raw(Box::new(AsyncPopulatorObject {
        vtbl: &ASYNC_POPULATOR_VTBL,
        ref_count: AtomicU32::new(1),
        callback: Mutex::new(Some(Box::new(callback))),
    }));
    AsyncPopulator {
        ptr: unsafe {
            ComPtr::from_raw(object.cast()).expect("Box allocation cannot produce a null pointer")
        },
    }
}

/// An owned handle to a Rust-built populator, safe to hand to the AsyncPopulator
/// setter.
#[derive(Clone, Debug)]
pub struct AsyncPopulator {
    ptr: ComPtr<IAvnAsyncPopulator>,
}

impl AsyncPopulator {
    pub fn as_com_ptr(&self) -> &ComPtr<IAvnAsyncPopulator> {
        &self.ptr
    }
}

fn invoke_callback(
    lock: &Mutex<Option<AsyncPopulatorCallback>>,
    request_id: i64,
    search: *const u16,
    completion: &mut PopulateCompletion,
) -> i32 {
    let callback = {
        let mut slot = match lock.lock() {
            Ok(slot) => slot,
            Err(_) => return hresult::E_FAIL,
        };
        match slot.take() {
            Some(callback) => callback,
            None => return hresult::E_FAIL,
        }
    };
    let mut callback = callback;
    let result = catch_unwind(AssertUnwindSafe(|| {
        // The callback borrows the completion; the PopulateCompletion handle it
        // receives clones the ComPtr, so the host-owned reference is preserved.
        let reporter = PopulateCompletion {
            ptr: completion.ptr.clone(),
        };
        callback(request_id, search, reporter)
    }));
    if let Ok(mut slot) = lock.lock() {
        *slot = Some(callback);
    }
    match result {
        Ok(Ok(())) => hresult::S_OK,
        Ok(Err(error)) => error.0,
        Err(_) => hresult::E_FAIL,
    }
}

unsafe extern "system" fn shared_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    unsafe {
        *result = std::ptr::null_mut();
        if *iid != Guid::IUNKNOWN && *iid != IAvnAsyncPopulator::IID {
            return hresult::E_NOINTERFACE;
        }
        *result = this.cast();
        hresult::S_OK
    }
}

unsafe extern "system" fn async_populator_query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    let hr = shared_query_interface(this, iid, result);
    if hr == 0 {
        async_populator_add_ref(this);
    }
    hr
}

unsafe extern "system" fn async_populator_add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<AsyncPopulatorObject>();
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn async_populator_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<AsyncPopulatorObject>();
    let remaining = (*object).ref_count.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn async_populator_begin_populate(
    this: *mut IAvnAsyncPopulator,
    request_id: i64,
    completion: *mut IAvnAsyncPopulatorCompletion,
    search: *const u16,
) -> i32 {
    if completion.is_null() {
        return hresult::E_POINTER;
    }
    let object = this.cast::<AsyncPopulatorObject>();
    let completion = unsafe { ComPtr::from_borrowed(completion) };
    let Some(completion) = completion else {
        return hresult::E_POINTER;
    };
    let mut completion = PopulateCompletion { ptr: completion };
    invoke_callback(&(*object).callback, request_id, search, &mut completion)
}

#[rustfmt::skip]
static ASYNC_POPULATOR_VTBL: AsyncPopulatorVtbl = AsyncPopulatorVtbl {
    query_interface: async_populator_query_interface,
    add_ref: async_populator_add_ref,
    release: async_populator_release,
    begin_populate: async_populator_begin_populate,
};

#[repr(C)]
struct StringListVtbl {
    query: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    count: unsafe extern "system" fn(*mut IAvnVariantList, *mut i32) -> i32,
    get: unsafe extern "system" fn(*mut IAvnVariantList, i32, *mut AvnVariant) -> i32,
    add: unsafe extern "system" fn(*mut IAvnVariantList, AvnVariant) -> i32,
    index: unsafe extern "system" fn(*mut IAvnVariantList, AvnVariant, *mut i32) -> i32,
    remove: unsafe extern "system" fn(*mut IAvnVariantList, i32) -> i32,
    clear: unsafe extern "system" fn(*mut IAvnVariantList) -> i32,
}

#[repr(C)]
struct StringList {
    vtbl: *const StringListVtbl,
    refs: AtomicU32,
    items: Vec<Vec<u16>>,
    alloc: crate::AllocUtf16Fn,
}

fn string_list(items: Vec<String>, alloc: crate::AllocUtf16Fn) -> Result<ComPtr<IAvnVariantList>> {
    i32::try_from(items.len()).map_err(|_| hresult::Error(hresult::E_INVALIDARG))?;
    let items: Vec<Vec<u16>> = items.iter().map(|s| s.encode_utf16().collect()).collect();
    if items.iter().any(|s| s.len() >= i32::MAX as usize) {
        return Err(hresult::Error(hresult::E_INVALIDARG));
    }
    let object = Box::new(StringList {
        vtbl: &STRING_LIST_VTBL,
        refs: AtomicU32::new(1),
        items,
        alloc,
    });
    Ok(unsafe { ComPtr::from_raw(Box::into_raw(object).cast()).unwrap() })
}

unsafe extern "system" fn list_query(
    this: *mut IUnknown,
    iid: *const Guid,
    out: *mut *mut c_void,
) -> i32 {
    if iid.is_null() || out.is_null() {
        return hresult::E_POINTER;
    }
    *out = std::ptr::null_mut();
    if *iid != Guid::IUNKNOWN && *iid != IAvnVariantList::IID {
        return hresult::E_NOINTERFACE;
    }
    *out = this.cast();
    list_add_ref(this);
    hresult::S_OK
}

unsafe extern "system" fn list_add_ref(this: *mut IUnknown) -> u32 {
    (*this.cast::<StringList>())
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

unsafe extern "system" fn list_release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<StringList>();
    let remaining = (*object).refs.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

unsafe extern "system" fn list_count(this: *mut IAvnVariantList, out: *mut i32) -> i32 {
    if out.is_null() {
        return hresult::E_POINTER;
    }
    *out = (*this.cast::<StringList>()).items.len() as i32;
    hresult::S_OK
}

unsafe extern "system" fn list_get(
    this: *mut IAvnVariantList,
    index: i32,
    out: *mut AvnVariant,
) -> i32 {
    if out.is_null() {
        return hresult::E_POINTER;
    }
    *out = AvnVariant::default();
    let object = &*this.cast::<StringList>();
    let Some(text) = object.items.get(index as usize) else {
        return hresult::E_INVALIDARG;
    };
    let buffer = (object.alloc)(text.len() as i32);
    if buffer.is_null() {
        return hresult::E_FAIL;
    }
    std::ptr::copy_nonoverlapping(text.as_ptr(), buffer, text.len());
    *buffer.add(text.len()) = 0;
    *out = AvnVariant {
        tag: AvnVariant::TAG_UTF16,
        utf16: buffer,
        ..Default::default()
    };
    hresult::S_OK
}

unsafe extern "system" fn list_index(
    this: *mut IAvnVariantList,
    value: AvnVariant,
    out: *mut i32,
) -> i32 {
    if out.is_null() {
        return hresult::E_POINTER;
    }
    *out = -1;
    if value.tag == AvnVariant::TAG_UTF16 && !value.utf16.is_null() {
        let mut len = 0;
        while *value.utf16.add(len) != 0 {
            len += 1;
        }
        let text = std::slice::from_raw_parts(value.utf16, len);
        if let Some(index) = (*this.cast::<StringList>())
            .items
            .iter()
            .position(|s| s == text)
        {
            *out = index as i32;
        }
    }
    hresult::S_OK
}

unsafe extern "system" fn list_add(_: *mut IAvnVariantList, _: AvnVariant) -> i32 {
    hresult::E_NOTIMPL
}
unsafe extern "system" fn list_remove(_: *mut IAvnVariantList, _: i32) -> i32 {
    hresult::E_NOTIMPL
}
unsafe extern "system" fn list_clear(_: *mut IAvnVariantList) -> i32 {
    hresult::E_NOTIMPL
}

static STRING_LIST_VTBL: StringListVtbl = StringListVtbl {
    query: list_query,
    add_ref: list_add_ref,
    release: list_release,
    count: list_count,
    get: list_get,
    add: list_add,
    index: list_index,
    remove: list_remove,
    clear: list_clear,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    unsafe extern "C" fn test_alloc(len: i32) -> *mut u16 {
        Box::into_raw(vec![0u16; len as usize + 1].into_boxed_slice()).cast()
    }

    unsafe extern "C" fn fail_alloc(_: i32) -> *mut u16 {
        std::ptr::null_mut()
    }

    #[test]
    fn string_suggestions_return_independently_owned_variants() {
        let list = string_list(vec!["日本語 😀".into(), "".into()], test_alloc).unwrap();
        assert_eq!(list.len().unwrap(), 2);
        let retained = list.query_interface::<IAvnVariantList>().unwrap();
        drop(list);
        for index in 0..2 {
            let first = retained.get(index).unwrap();
            let second = retained.get(index).unwrap();
            assert_eq!(first.tag, AvnVariant::TAG_UTF16);
            assert_ne!(first.utf16, second.utf16);
            assert_eq!(retained.index_of(first).unwrap(), Some(index));
            let expected = if index == 0 { "日本語 😀" } else { "" };
            unsafe {
                assert_eq!(crate::clone_utf16(first.utf16).unwrap(), expected);
                assert_eq!(crate::clone_utf16(second.utf16).unwrap(), expected);
                let len = expected.encode_utf16().count() + 1;
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    first.utf16,
                    len,
                )));
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    second.utf16,
                    len,
                )));
            }
        }
        assert_eq!(retained.get(2).unwrap_err().0, hresult::E_INVALIDARG);
        assert_eq!(retained.clear().unwrap_err().0, hresult::E_NOTIMPL);
        assert!(string_list(vec![], test_alloc).unwrap().is_empty().unwrap());
        let failing = string_list(vec!["failure".into()], fail_alloc).unwrap();
        assert_eq!(failing.get(0).unwrap_err().0, hresult::E_FAIL);
    }

    #[repr(C)]
    struct CompletionVtbl {
        query: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
        add: unsafe extern "system" fn(*mut IUnknown) -> u32,
        release: unsafe extern "system" fn(*mut IUnknown) -> u32,
        complete: unsafe extern "system" fn(
            *mut IAvnAsyncPopulatorCompletion,
            i64,
            i32,
            *mut IAvnVariantList,
        ) -> i32,
    }

    #[repr(C)]
    struct Completion {
        vtbl: *const CompletionVtbl,
        refs: AtomicU32,
        calls: AtomicU32,
        result: i32,
        expected_items: *mut IAvnVariantList,
    }

    unsafe extern "system" fn query(_: *mut IUnknown, _: *const Guid, _: *mut *mut c_void) -> i32 {
        hresult::E_NOINTERFACE
    }

    unsafe extern "system" fn add(this: *mut IUnknown) -> u32 {
        (*this.cast::<Completion>())
            .refs
            .fetch_add(1, Ordering::SeqCst)
            + 1
    }

    unsafe extern "system" fn release(this: *mut IUnknown) -> u32 {
        (*this.cast::<Completion>())
            .refs
            .fetch_sub(1, Ordering::SeqCst)
            - 1
    }

    unsafe extern "system" fn complete(
        this: *mut IAvnAsyncPopulatorCompletion,
        request: i64,
        hr: i32,
        items: *mut IAvnVariantList,
    ) -> i32 {
        let state = &*this.cast::<Completion>();
        assert_eq!(request, 42);
        assert_eq!(hr, 0);
        assert_eq!(items, state.expected_items);
        state.calls.fetch_add(1, Ordering::SeqCst);
        state.result
    }

    static VTABLE: CompletionVtbl = CompletionVtbl {
        query,
        add,
        release,
        complete,
    };

    fn completion() -> Completion {
        Completion {
            vtbl: &VTABLE,
            refs: AtomicU32::new(1),
            calls: AtomicU32::new(0),
            result: 0,
            expected_items: std::ptr::null_mut(),
        }
    }

    #[test]
    fn synchronous_completion_preserves_host_reference() {
        let mut host = completion();
        let populator = async_populator(|request, _, completion| completion.complete(request));
        unsafe {
            assert_eq!(
                async_populator_begin_populate(
                    populator.ptr.as_raw(),
                    42,
                    (&mut host as *mut Completion).cast(),
                    std::ptr::null(),
                ),
                0
            );
        }
        assert_eq!(host.refs.load(Ordering::SeqCst), 1);
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    }

    thread_local! {
        static RETAINED: RefCell<Option<PopulateCompletion>> = const { RefCell::new(None) };
    }

    #[test]
    fn retained_completion_owns_reference_until_dropped() {
        let mut host = completion();
        let populator = async_populator(|_, _, completion| {
            RETAINED.with(|slot| *slot.borrow_mut() = Some(completion));
            Ok(())
        });
        unsafe {
            assert_eq!(
                async_populator_begin_populate(
                    populator.ptr.as_raw(),
                    42,
                    (&mut host as *mut Completion).cast(),
                    std::ptr::null(),
                ),
                0
            );
        }
        assert_eq!(host.refs.load(Ordering::SeqCst), 2);
        RETAINED.with(|slot| {
            let reporter = slot.borrow_mut().take().unwrap();
            reporter.complete(42).unwrap();
        });
        assert_eq!(host.refs.load(Ordering::SeqCst), 1);
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn suggestions_list_is_borrowed_on_success_and_failure() {
        // Only IUnknown is invoked on the list by this forwarding API.
        let mut list = completion();
        let list_ptr = unsafe {
            ComPtr::<IAvnVariantList>::from_borrowed((&mut list as *mut Completion).cast()).unwrap()
        };
        let mut host = completion();
        host.expected_items = list_ptr.as_raw();
        let reporter = PopulateCompletion {
            ptr: unsafe { ComPtr::from_borrowed((&mut host as *mut Completion).cast()).unwrap() },
        };
        reporter.complete_items(42, &list_ptr).unwrap();
        assert_eq!(list.refs.load(Ordering::SeqCst), 2);
        host.result = hresult::E_FAIL;
        assert_eq!(
            reporter.complete_items(42, &list_ptr).unwrap_err().0,
            hresult::E_FAIL
        );
        assert_eq!(list.refs.load(Ordering::SeqCst), 2);
        drop(list_ptr);
        drop(reporter);
        assert_eq!(list.refs.load(Ordering::SeqCst), 1);
        assert_eq!(host.refs.load(Ordering::SeqCst), 1);
    }
}

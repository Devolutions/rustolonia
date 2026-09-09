use crate::guid::Guid;
use crate::hresult::{self, Error, Result};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

#[repr(C)]
struct IUnknownVtbl {
    query_interface:
        unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut std::ffi::c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
}

#[repr(C)]
struct ProjectedObjectVtbl {
    query_interface:
        unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut std::ffi::c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    get_object_id: unsafe extern "system" fn(*mut std::ffi::c_void, *mut i64) -> i32,
    get_lifetime_token: unsafe extern "system" fn(*mut std::ffi::c_void, *mut i64) -> i32,
}

#[repr(C)]
pub struct IUnknown {
    vtbl: *const IUnknownVtbl,
}

/// Types that can be `QueryInterface`'d. `IID` is the interface GUID.
///
/// # Safety
/// `Self` must be a COM interface whose vtable starts with IUnknown.
pub unsafe trait ComInterface {
    const IID: Guid;
}

unsafe impl ComInterface for IUnknown {
    const IID: Guid = Guid::IUNKNOWN;
}

pub struct ComPtr<T: ComInterface> {
    ptr: NonNull<T>,
    lifetime: Option<NonNull<IUnknown>>,
    _marker: PhantomData<T>,
}

impl<T: ComInterface> ComPtr<T> {
    /// Takes ownership of an already AddRef'd interface pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must either be null or point to a valid interface whose reference
    /// count is already owned by the caller.
    pub unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            lifetime: None,
            _marker: PhantomData,
        })
    }

    pub(crate) unsafe fn from_borrowed(ptr: *mut T) -> Option<Self> {
        let result = Self::from_raw(ptr)?;
        result.add_ref();
        Some(result)
    }

    pub(crate) unsafe fn from_projected_raw(ptr: *mut T) -> Result<Self> {
        let mut result = Self::from_raw(ptr).ok_or(Error(hresult::E_POINTER))?;
        let vtable = *(ptr.cast::<*const ProjectedObjectVtbl>());
        let mut lifetime = 0i64;
        let hr = ((*vtable).get_lifetime_token)(ptr.cast(), &mut lifetime);
        hresult::check(hr)?;
        let lifetime = lifetime as usize as *mut IUnknown;
        result.lifetime = Some(NonNull::new(lifetime).ok_or(Error(hresult::E_POINTER))?);
        Ok(result)
    }

    pub fn as_raw(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    pub fn as_unknown(&self) -> *mut IUnknown {
        self.ptr.as_ptr().cast()
    }

    pub fn query_interface<U: ComInterface>(&self) -> Result<ComPtr<U>> {
        unsafe {
            let mut out = ptr::null_mut();
            let hr = (((*(self.as_unknown())).vtbl)
                .as_ref()
                .unwrap()
                .query_interface)(self.as_unknown(), &U::IID, &mut out);
            hresult::check(hr)?;
            let mut result = ComPtr::from_raw(out.cast()).ok_or(Error(hresult::E_POINTER))?;
            result.lifetime = self.clone_lifetime();
            Ok(result)
        }
    }

    pub fn add_ref(&self) -> u32 {
        unsafe { (((*(self.as_unknown())).vtbl).as_ref().unwrap().add_ref)(self.as_unknown()) }
    }

    pub fn as_iunknown(&self) -> Result<ComPtr<IUnknown>> {
        self.query_interface::<IUnknown>()
    }

    fn clone_lifetime(&self) -> Option<NonNull<IUnknown>> {
        self.lifetime.inspect(|lifetime| unsafe {
            let unknown = lifetime.as_ptr();
            (((*unknown).vtbl).as_ref().unwrap().add_ref)(unknown);
        })
    }
}

impl<T: ComInterface> std::fmt::Debug for ComPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComPtr")
            .field("ptr", &self.ptr.as_ptr())
            .field("iid", &T::IID)
            .finish()
    }
}

impl<T: ComInterface> Clone for ComPtr<T> {
    fn clone(&self) -> Self {
        self.add_ref();
        Self {
            ptr: self.ptr,
            lifetime: self.clone_lifetime(),
            _marker: PhantomData,
        }
    }
}

impl<T: ComInterface> Drop for ComPtr<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = (((*(self.as_unknown())).vtbl).as_ref().unwrap().release)(self.as_unknown());
            if let Some(lifetime) = self.lifetime {
                let unknown = lifetime.as_ptr();
                let _ = (((*unknown).vtbl).as_ref().unwrap().release)(unknown);
            }
        }
    }
}

unsafe impl<T: ComInterface> Send for ComPtr<T> {}
unsafe impl<T: ComInterface> Sync for ComPtr<T> {}

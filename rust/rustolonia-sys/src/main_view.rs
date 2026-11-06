use crate::generated::IAvnControl;
use crate::{ComPtr, IAvnApplication, Result};

impl ComPtr<IAvnApplication> {
    /// Sets the root control of a single-view lifetime (the browser host).
    /// Desktop hosts, whose lifetime has windows instead, return `E_NOTIMPL`.
    pub fn set_main_view(&self, view: Option<&ComPtr<IAvnControl>>) -> Result<()> {
        unsafe {
            crate::hresult::check((self.vtbl().set_main_view)(
                self.as_raw(),
                view.map_or(std::ptr::null_mut(), |view| view.as_raw()),
            ))
        }
    }
}

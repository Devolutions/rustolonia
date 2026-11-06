use crate::{ComPtr, IAvnApplication, Result};

impl ComPtr<IAvnApplication> {
    /// Names the application; call before Avalonia initializes.
    pub fn set_application_name(&self, name: Option<&[u16]>) -> Result<()> {
        let name = name.map(crate::terminated_utf16);
        unsafe {
            crate::hresult::check((self.vtbl().set_application_name)(
                self.as_raw(),
                name.as_ref().map_or(std::ptr::null(), |name| name.as_ptr()),
            ))
        }
    }
}

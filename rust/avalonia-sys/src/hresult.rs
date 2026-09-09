pub const S_OK: i32 = 0;
pub const E_POINTER: i32 = 0x8000_4003u32 as i32;
pub const E_FAIL: i32 = 0x8000_4005u32 as i32;
pub const E_INVALIDARG: i32 = 0x8007_0057u32 as i32;
pub const E_NOINTERFACE: i32 = 0x8000_4002u32 as i32;
pub const E_NOTIMPL: i32 = 0x8000_4001u32 as i32;
pub const AVN_E_FIXTURE: i32 = 0xA7A7_0001u32 as i32;

pub fn succeeded(hr: i32) -> bool {
    hr >= 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(pub i32);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HRESULT 0x{:08X}", self.0 as u32)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub fn check(hr: i32) -> Result<()> {
    if succeeded(hr) {
        Ok(())
    } else {
        Err(Error(hr))
    }
}

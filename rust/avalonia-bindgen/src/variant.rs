//! The tagged scalar carrying `object?` command parameters across the ABI.
//!
//! Mirrors `AvnVariant` from the C header and the C# projection: `tag` selects
//! the payload slot — 0 none, 1 utf16, 2 i32, 3 f64, 4 bool. Strings are
//! host-allocated in the getter direction (release with `take_utf16`) and
//! borrowed in the setter direction (the host reads them synchronously).

/// `#[repr(C)]` ABI struct plus tag constants for the `avalonia-sys` crate.
pub fn emit_sys_struct() -> String {
    format!(
        "/// Tagged scalar carrying object command parameters. tag: 0 none,\n\
         /// 1 utf16, 2 i32, 3 f64, 4 bool. The utf16 pointer is host-allocated\n\
         /// for getters (release with `take_utf16`) and borrowed for setters.\n\
         #[repr(C)]\n\
         #[derive(Clone, Copy, Debug)]\n\
         pub struct AvnVariant {{\n\
         \x20   pub tag: i32,\n\
         \x20   pub utf16: *mut u16,\n\
         \x20   pub i32: i32,\n\
         \x20   pub f64: f64,\n\
         }}\n\n\
         impl Default for AvnVariant {{\n\
         \x20   fn default() -> Self {{\n\
         \x20       Self {{ tag: 0, utf16: std::ptr::null_mut(), i32: 0, f64: 0.0 }}\n\
         \x20   }}\n\
         }}\n\n\
         impl AvnVariant {{\n\
         \x20   pub const TAG_NONE: i32 = 0;\n\
         \x20   pub const TAG_UTF16: i32 = 1;\n\
         \x20   pub const TAG_I32: i32 = 2;\n\
         \x20   pub const TAG_F64: i32 = 3;\n\
         \x20   pub const TAG_BOOL: i32 = 4;\n\
         }}\n"
    )
}

/// The ergonomic `Variant` enum plus ABI conversions for the safe `avalonia`
/// crate. `to_abi` allocates UTF-16 with the host allocator when the payload is
/// a string; callers release it after the call — setters read the payload
/// synchronously, so releasing immediately after the call is correct.
pub fn emit_safe_enum() -> String {
    format!(
        "/// A command parameter: the closed set an `object` slot can carry.\n\
         #[derive(Clone, Debug, PartialEq)]\n\
         pub enum Variant {{\n\
         \x20   None,\n\
         \x20   Utf16(String),\n\
         \x20   I32(i32),\n\
         \x20   F64(f64),\n\
         \x20   Bool(bool),\n\
         }}\n\n\
         impl From<&str> for Variant {{\n\
         \x20   fn from(value: &str) -> Self {{\n\
         \x20       Variant::Utf16(value.to_string())\n\
         \x20   }}\n\
         }}\n\n\
         impl From<String> for Variant {{\n\
         \x20   fn from(value: String) -> Self {{\n\
         \x20       Variant::Utf16(value)\n\
         \x20   }}\n\
         }}\n\n\
         impl From<i32> for Variant {{\n\
         \x20   fn from(value: i32) -> Self {{\n\
         \x20       Variant::I32(value)\n\
         \x20   }}\n\
         }}\n\n\
         impl From<f64> for Variant {{\n\
         \x20   fn from(value: f64) -> Self {{\n\
         \x20       Variant::F64(value)\n\
         \x20   }}\n\
         }}\n\n\
         impl From<bool> for Variant {{\n\
         \x20   fn from(value: bool) -> Self {{\n\
         \x20       Variant::Bool(value)\n\
         \x20   }}\n\
         }}\n\n\
         impl Variant {{\n\
         \x20   /// Converts to a guard holding the ABI struct, allocating UTF-16\n\
         \x20   /// with the host allocator when the payload is a string. The guard\n\
         \x20   /// releases the allocation when dropped, after the call returns.\n\
         \x20   pub(crate) fn to_abi(&self) -> Result<VariantAbi> {{\n\
         \x20       match self {{\n\
         \x20           Variant::None => Ok(VariantAbi::default()),\n\
         \x20           Variant::I32(value) => Ok(VariantAbi {{ inner: sys::AvnVariant {{ tag: sys::AvnVariant::TAG_I32, i32: *value, ..Default::default() }} }}),\n\
         \x20           Variant::F64(value) => Ok(VariantAbi {{ inner: sys::AvnVariant {{ tag: sys::AvnVariant::TAG_F64, f64: *value, ..Default::default() }} }}),\n\
         \x20           Variant::Bool(value) => Ok(VariantAbi {{ inner: sys::AvnVariant {{ tag: sys::AvnVariant::TAG_BOOL, i32: i32::from(*value), ..Default::default() }} }}),\n\
         \x20           Variant::Utf16(value) => {{\n\
         \x20               let ptr = sys::alloc_utf16_raw(value.len() as i32)\n\
         \x20                   .ok_or_else(|| sys::Error(sys::E_FAIL))?;\n\
         \x20               let mut cursor = ptr;\n\
         \x20               for unit in value.encode_utf16().chain(Some(0)) {{\n\
         \x20                   unsafe {{ *cursor = unit; cursor = cursor.add(1); }}\n\
         \x20               }}\n\
         \x20               Ok(VariantAbi {{ inner: sys::AvnVariant {{ tag: sys::AvnVariant::TAG_UTF16, utf16: ptr, ..Default::default() }} }})\n\
         \x20           }}\n\
         \x20       }}\n\
         \x20   }}\n\n\
         \x20   /// Reads an ABI struct, releasing a host-allocated UTF-16 payload.\n\
         \x20   pub(crate) fn from_abi(value: sys::AvnVariant) -> Self {{\n\
         \x20       match value.tag {{\n\
         \x20           sys::AvnVariant::TAG_UTF16 => unsafe {{\n\
         \x20               Variant::Utf16(sys::take_utf16(value.utf16).unwrap_or_default())\n\
         \x20           }},\n\
         \x20           sys::AvnVariant::TAG_I32 => Variant::I32(value.i32),\n\
         \x20           sys::AvnVariant::TAG_F64 => Variant::F64(value.f64),\n\
         \x20           sys::AvnVariant::TAG_BOOL => Variant::Bool(value.i32 != 0),\n\
         \x20           _ => Variant::None,\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n\n\
         /// Owns an ABI variant, releasing its UTF-16 allocation on drop.\n\
         pub struct VariantAbi {{\n\
         \x20   inner: sys::AvnVariant,\n\
         }}\n\n\
         impl Default for VariantAbi {{\n\
         \x20   fn default() -> Self {{\n\
         \x20       Self {{ inner: sys::AvnVariant::default() }}\n\
         \x20   }}\n\
         }}\n\n\
         impl std::ops::Deref for VariantAbi {{\n\
         \x20   type Target = sys::AvnVariant;\n\
         \x20   fn deref(&self) -> &sys::AvnVariant {{\n\
         \x20       &self.inner\n\
         \x20   }}\n\
         }}\n\n\
         impl Drop for VariantAbi {{\n\
         \x20   fn drop(&mut self) {{\n\
         \x20       if self.inner.tag == sys::AvnVariant::TAG_UTF16 && !self.inner.utf16.is_null() {{\n\
         \x20           unsafe {{ sys::free_utf16(self.inner.utf16) }};\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    )
}

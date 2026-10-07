//! Raw nano-COM object exposing a Rust-authored value-converter provider to
//! the managed host. Handwritten like `rust_vm.rs`: the vtable is a stable,
//! versionable tagged-scalar transport, not a copy of the CLR `object`
//! model. Dispatch is lock-free by construction — the provider only stores
//! an immutable `Arc<dyn Fn(...) + Send + Sync>`, never a `Mutex`, because
//! converters must be pure and must not touch `ViewModel` state.

use crate::com::{ComInterface, ComPtr, IUnknown};
use crate::guid::Guid;
use crate::hresult;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Arc;

const IAVN_RUST_VALUE_CONVERTER_PROVIDER_IID: Guid = Guid {
    data1: 0x6B2E8F10,
    data2: 0x4C91,
    data3: 0x4E3A,
    data4: [0x9A, 0x77, 0x1F, 0x0C, 0x2B, 0x3A, 0x4D, 0x40],
};

/// Direction of a conversion. Mirrors `RustConversionDirection` in
/// `Avalonia.Rust`.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionDirection {
    Convert = 0,
    ConvertBack = 1,
}

impl ConversionDirection {
    fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Convert),
            1 => Some(Self::ConvertBack),
            _ => None,
        }
    }
}

/// Tag for a scalar carried across the value-converter ABI. Mirrors
/// `AvnValueKind` in `Avalonia.Rust`. `Any` is only ever used as a
/// target-kind hint, never as an actual value/parameter/result kind.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarKind {
    Null = 0,
    String = 1,
    Int64 = 2,
    Boolean = 3,
    Double = 4,
    Unset = 5,
    DoNothing = 6,
    Any = 7,
}

impl ScalarKind {
    fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Null),
            1 => Some(Self::String),
            2 => Some(Self::Int64),
            3 => Some(Self::Boolean),
            4 => Some(Self::Double),
            5 => Some(Self::Unset),
            6 => Some(Self::DoNothing),
            7 => Some(Self::Any),
            _ => None,
        }
    }
}

/// A single scalar value carried across the value-converter ABI. Explicitly
/// scalar-only: null, string, 64-bit integer, Boolean, double, `Unset`
/// (`AvaloniaProperty.UnsetValue`), and `DoNothing`
/// (`BindingOperations.DoNothing`). Arbitrary managed/host objects are out of
/// scope and rejected before crossing the ABI on the managed side.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarValue {
    Null,
    String(String),
    Int64(i64),
    Boolean(bool),
    Double(f64),
    Unset,
    DoNothing,
}

/// A converter failure: an HRESULT plus an optional human-readable message
/// surfaced to the managed caller (and, from there, into a
/// `BindingNotification`).
#[derive(Debug, Clone)]
pub struct ConverterAbiError {
    pub hresult: i32,
    pub message: Option<String>,
}

impl ConverterAbiError {
    pub fn new(hresult: i32, message: impl Into<String>) -> Self {
        Self {
            hresult,
            message: Some(message.into()),
        }
    }
}

impl From<hresult::Error> for ConverterAbiError {
    fn from(value: hresult::Error) -> Self {
        Self {
            hresult: value.0,
            message: None,
        }
    }
}

/// Immutable, `Send + Sync` dispatch function invoked without any
/// provider-level lock. `target_kind` and `culture` are threaded through the
/// full ABI for forward compatibility even though the generated scalar
/// converters in this stage do not use them yet.
pub type ConvertFn = dyn Fn(
        i32,
        ConversionDirection,
        ScalarValue,
        ScalarValue,
        ScalarKind,
        &str,
    ) -> std::result::Result<ScalarValue, ConverterAbiError>
    + Send
    + Sync;

#[repr(C)]
struct IAvnRustValueConverterProviderVtbl {
    query_interface: unsafe extern "system" fn(*mut IUnknown, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    #[allow(clippy::type_complexity)]
    convert: unsafe extern "system" fn(
        *mut IAvnRustValueConverterProvider,
        i32,
        i32,
        i32,
        i64,
        f64,
        i32,
        *const u16,
        i32,
        i64,
        f64,
        i32,
        *const u16,
        i32,
        *const u16,
        *mut i32,
        *mut i64,
        *mut f64,
        *mut i32,
        *mut *mut u16,
        *mut *mut u16,
    ) -> i32,
}

#[repr(C)]
pub struct IAvnRustValueConverterProvider {
    vtbl: *const IAvnRustValueConverterProviderVtbl,
}

unsafe impl ComInterface for IAvnRustValueConverterProvider {
    const IID: Guid = IAVN_RUST_VALUE_CONVERTER_PROVIDER_IID;
}

#[repr(C)]
struct RustValueConverterProviderObject {
    interface: IAvnRustValueConverterProvider,
    references: AtomicU32,
    convert: Arc<ConvertFn>,
}

static VTBL: IAvnRustValueConverterProviderVtbl = IAvnRustValueConverterProviderVtbl {
    query_interface,
    add_ref,
    release,
    convert: convert_thunk,
};

/// Builds the nano-COM object the managed host resolves converters through.
/// `convert` must be pure: it is invoked without any provider-level lock and
/// may be called concurrently from any thread that evaluates a binding.
pub fn rust_value_converter_provider(
    convert: Arc<ConvertFn>,
) -> ComPtr<IAvnRustValueConverterProvider> {
    let object = Box::new(RustValueConverterProviderObject {
        interface: IAvnRustValueConverterProvider { vtbl: &VTBL },
        references: AtomicU32::new(1),
        convert,
    });
    unsafe {
        ComPtr::from_raw(Box::into_raw(object).cast())
            .expect("Box allocation cannot produce a null pointer")
    }
}

unsafe extern "system" fn query_interface(
    this: *mut IUnknown,
    iid: *const Guid,
    result: *mut *mut c_void,
) -> i32 {
    if iid.is_null() || result.is_null() {
        return hresult::E_POINTER;
    }
    *result = ptr::null_mut();
    if *iid != Guid::IUNKNOWN && *iid != IAVN_RUST_VALUE_CONVERTER_PROVIDER_IID {
        return hresult::E_NOINTERFACE;
    }
    add_ref(this);
    *result = this.cast();
    hresult::S_OK
}

unsafe extern "system" fn add_ref(this: *mut IUnknown) -> u32 {
    let object = this.cast::<RustValueConverterProviderObject>();
    (*object).references.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn release(this: *mut IUnknown) -> u32 {
    let object = this.cast::<RustValueConverterProviderObject>();
    let remaining = (*object).references.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(object));
    }
    remaining
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn convert_thunk(
    this: *mut IAvnRustValueConverterProvider,
    converter_id: i32,
    direction: i32,
    value_kind: i32,
    value_int64: i64,
    value_double: f64,
    value_boolean: i32,
    value_string: *const u16,
    parameter_kind: i32,
    parameter_int64: i64,
    parameter_double: f64,
    parameter_boolean: i32,
    parameter_string: *const u16,
    target_kind: i32,
    culture: *const u16,
    result_kind: *mut i32,
    result_int64: *mut i64,
    result_double: *mut f64,
    result_boolean: *mut i32,
    result_string: *mut *mut u16,
    error: *mut *mut u16,
) -> i32 {
    if this.is_null()
        || result_kind.is_null()
        || result_int64.is_null()
        || result_double.is_null()
        || result_boolean.is_null()
        || result_string.is_null()
        || error.is_null()
    {
        return hresult::E_POINTER;
    }

    *result_kind = ScalarKind::Null as i32;
    *result_int64 = 0;
    *result_double = 0.0;
    *result_boolean = 0;
    *result_string = ptr::null_mut();
    *error = ptr::null_mut();

    let object = this.cast::<RustValueConverterProviderObject>();
    let Some(direction) = ConversionDirection::from_i32(direction) else {
        write_error(error, Some("Invalid conversion direction."));
        return hresult::E_INVALIDARG;
    };
    let Some(value) = decode_scalar(
        value_kind,
        value_int64,
        value_double,
        value_boolean,
        value_string,
    ) else {
        write_error(error, Some("Invalid converter value."));
        return hresult::E_INVALIDARG;
    };
    let Some(parameter) = decode_scalar(
        parameter_kind,
        parameter_int64,
        parameter_double,
        parameter_boolean,
        parameter_string,
    ) else {
        write_error(error, Some("Invalid converter parameter."));
        return hresult::E_INVALIDARG;
    };
    let Some(target_kind) = ScalarKind::from_i32(target_kind) else {
        write_error(error, Some("Invalid converter target kind."));
        return hresult::E_INVALIDARG;
    };
    let culture = crate::clone_utf16(culture).unwrap_or_default();

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        ((*object).convert)(
            converter_id,
            direction,
            value,
            parameter,
            target_kind,
            &culture,
        )
    }));

    match outcome {
        Ok(Ok(value)) => match encode_scalar(
            value,
            result_kind,
            result_int64,
            result_double,
            result_boolean,
            result_string,
        ) {
            Ok(()) => hresult::S_OK,
            Err(failure) => {
                write_error(error, failure.message.as_deref());
                failure.hresult
            }
        },
        Ok(Err(failure)) => {
            write_error(error, failure.message.as_deref());
            failure.hresult
        }
        Err(_) => {
            write_error(error, Some("The Rust value converter panicked."));
            hresult::E_FAIL
        }
    }
}

unsafe fn decode_scalar(
    kind: i32,
    int64_value: i64,
    double_value: f64,
    boolean_value: i32,
    string_value: *const u16,
) -> Option<ScalarValue> {
    match ScalarKind::from_i32(kind)? {
        ScalarKind::Null => Some(ScalarValue::Null),
        ScalarKind::String => Some(ScalarValue::String(crate::clone_utf16(string_value)?)),
        ScalarKind::Int64 => Some(ScalarValue::Int64(int64_value)),
        ScalarKind::Boolean => Some(ScalarValue::Boolean(boolean_value != 0)),
        ScalarKind::Double => Some(ScalarValue::Double(double_value)),
        ScalarKind::Unset => Some(ScalarValue::Unset),
        ScalarKind::DoNothing => Some(ScalarValue::DoNothing),
        ScalarKind::Any => None,
    }
}

unsafe fn encode_scalar(
    value: ScalarValue,
    kind: *mut i32,
    int64_value: *mut i64,
    double_value: *mut f64,
    boolean_value: *mut i32,
    string_value: *mut *mut u16,
) -> std::result::Result<(), ConverterAbiError> {
    match value {
        ScalarValue::Null => *kind = ScalarKind::Null as i32,
        ScalarValue::String(value) => {
            *kind = ScalarKind::String as i32;
            *string_value = crate::alloc_utf16(&value).ok_or_else(|| {
                ConverterAbiError::new(hresult::E_FAIL, "Failed to allocate the converted string.")
            })?;
        }
        ScalarValue::Int64(value) => {
            *kind = ScalarKind::Int64 as i32;
            *int64_value = value;
        }
        ScalarValue::Boolean(value) => {
            *kind = ScalarKind::Boolean as i32;
            *boolean_value = value as i32;
        }
        ScalarValue::Double(value) => {
            *kind = ScalarKind::Double as i32;
            *double_value = value;
        }
        ScalarValue::Unset => *kind = ScalarKind::Unset as i32,
        ScalarValue::DoNothing => *kind = ScalarKind::DoNothing as i32,
    }
    Ok(())
}

unsafe fn write_error(slot: *mut *mut u16, message: Option<&str>) {
    if let Some(message) = message {
        if let Some(allocated) = crate::alloc_utf16(message) {
            *slot = allocated;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_rejects_null_required_pointers() {
        let result = unsafe {
            convert_thunk(
                ptr::null_mut(),
                0,
                0,
                0,
                0,
                0.0,
                0,
                ptr::null(),
                0,
                0,
                0.0,
                0,
                ptr::null(),
                0,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        assert_eq!(hresult::E_POINTER, result);
    }
}

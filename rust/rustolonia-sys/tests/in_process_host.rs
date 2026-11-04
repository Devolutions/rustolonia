use rustolonia_sys::{Host, HostExports, HostInfoNative, HostLoadError, ABI_FINGERPRINT};
use std::ffi::{c_void, CString};
use std::sync::OnceLock;

unsafe extern "C" fn activation_factory(_: *mut *mut c_void) -> i32 {
    rustolonia_sys::E_NOTIMPL
}

unsafe extern "C" fn free(_: *mut c_void) {}
unsafe extern "C" fn other_free(_: *mut c_void) {}
unsafe extern "C" fn alloc(_: i32) -> *mut u16 {
    std::ptr::null_mut()
}
unsafe extern "C" fn last_error(_: *mut *mut u16) -> i32 {
    rustolonia_sys::S_OK
}

unsafe extern "C" fn host_info(info: *mut HostInfoNative) -> i32 {
    static FINGERPRINT: OnceLock<CString> = OnceLock::new();
    (*info).version = c"test".as_ptr();
    (*info).abi_fingerprint = FINGERPRINT
        .get_or_init(|| CString::new(ABI_FINGERPRINT).unwrap())
        .as_ptr();
    rustolonia_sys::S_OK
}

unsafe extern "C" fn incompatible_info(info: *mut HostInfoNative) -> i32 {
    (*info).version = c"test".as_ptr();
    (*info).abi_fingerprint = c"wrong".as_ptr();
    rustolonia_sys::S_OK
}

fn exports() -> HostExports {
    HostExports {
        struct_size: std::mem::size_of::<HostExports>() as u32,
        reserved: 0,
        get_activation_factory: Some(activation_factory),
        free: Some(free),
        alloc_utf16: Some(alloc),
        get_last_error: Some(last_error),
        get_host_info: Some(host_info),
    }
}

#[test]
fn export_table_has_the_c_layout_used_by_the_embedded_host() {
    let pointer_size = std::mem::size_of::<usize>();
    assert_eq!(std::mem::offset_of!(HostExports, struct_size), 0);
    assert_eq!(std::mem::offset_of!(HostExports, reserved), 4);
    assert_eq!(std::mem::offset_of!(HostExports, get_activation_factory), 8);
    assert_eq!(std::mem::offset_of!(HostExports, free), 8 + pointer_size);
    assert_eq!(
        std::mem::offset_of!(HostExports, alloc_utf16),
        8 + 2 * pointer_size
    );
    assert_eq!(
        std::mem::offset_of!(HostExports, get_last_error),
        8 + 3 * pointer_size
    );
    assert_eq!(
        std::mem::offset_of!(HostExports, get_host_info),
        8 + 4 * pointer_size
    );
    assert_eq!(std::mem::size_of::<HostExports>(), 8 + 5 * pointer_size);
}

#[test]
fn in_process_host_validates_the_table_and_abi_before_publishing_allocators() {
    let mut invalid = exports();
    invalid.struct_size = 0;
    assert!(matches!(
        unsafe { Host::from_in_process(&invalid) },
        Err(HostLoadError::InvalidExportsSize(0))
    ));

    for (invalid, expected) in [
        (
            HostExports {
                get_activation_factory: None,
                ..exports()
            },
            "avn_get_activation_factory",
        ),
        (
            HostExports {
                free: None,
                ..exports()
            },
            "avn_free",
        ),
        (
            HostExports {
                alloc_utf16: None,
                ..exports()
            },
            "avn_alloc_utf16",
        ),
        (
            HostExports {
                get_last_error: None,
                ..exports()
            },
            "avn_get_last_error",
        ),
        (
            HostExports {
                get_host_info: None,
                ..exports()
            },
            "avn_get_host_info",
        ),
    ] {
        assert!(matches!(
            unsafe { Host::from_in_process(&invalid) },
            Err(HostLoadError::MissingExport(name)) if name == expected
        ));
    }

    invalid = exports();
    invalid.get_host_info = Some(incompatible_info);
    assert!(matches!(
        unsafe { Host::from_in_process(&invalid) },
        Err(HostLoadError::IncompatibleHost { .. })
    ));

    let host = unsafe { Host::from_in_process(&exports()) }.unwrap();
    assert_eq!(host.info().abi_fingerprint, ABI_FINGERPRINT);
    assert_eq!(host.info().version, "test");
    assert!(host.activation_factory().is_err());

    invalid = exports();
    invalid.free = Some(other_free);
    assert!(matches!(
        unsafe { Host::from_in_process(&invalid) },
        Err(HostLoadError::IncompatibleAllocator)
    ));
}

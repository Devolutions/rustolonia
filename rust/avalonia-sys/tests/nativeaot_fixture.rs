use avalonia_sys::{ComPtr, Host, IUnknown, AVN_E_FIXTURE, E_NOINTERFACE};
use libloading::Library;
use std::ffi::c_void;
use std::path::PathBuf;

fn host_path() -> PathBuf {
    if let Ok(p) = std::env::var("AVN_HOST_NATIVE_LIB") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    #[cfg(target_os = "windows")]
    let candidates = [
        "host/bin/Release/net10.0/win-x64/publish/Avalonia.Host.dll",
        "host/bin/Release/net10.0/win-arm64/publish/Avalonia.Host.dll",
        "host/bin/Debug/net10.0/win-x64/publish/Avalonia.Host.dll",
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        "rust/target/dotnet-linux-x64/publish/Avalonia.Host/release_linux-x64/Avalonia.Host.so",
        "rust/target/dotnet-linux-arm64/publish/Avalonia.Host/release_linux-arm64/Avalonia.Host.so",
    ];
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let candidates: [&str; 0] = [];
    for rel in candidates {
        let p = root.join(rel);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Avalonia.Host native library not found. Publish with \
         `rust/build.ps1` on Windows or `rust/build.sh` on Linux, or set \
         AVN_HOST_NATIVE_LIB."
    );
}

fn load() -> Host {
    let path = host_path();
    assert!(path.exists(), "missing {}", path.display());
    Host::load(&path).unwrap_or_else(|e| panic!("load {}: {e}", path.display()))
}

#[repr(C)]
struct MicroComOwnershipProbeVtbl {
    query_interface: unsafe extern "system" fn(
        *mut MicroComOwnershipProbe,
        *const c_void,
        *mut *mut c_void,
    ) -> i32,
    add_ref: unsafe extern "system" fn(*mut MicroComOwnershipProbe) -> u32,
    release: unsafe extern "system" fn(*mut MicroComOwnershipProbe) -> u32,
    get_value: unsafe extern "system" fn(*mut MicroComOwnershipProbe, *mut i32) -> i32,
}

#[repr(C)]
struct MicroComOwnershipProbe {
    vtbl: *const MicroComOwnershipProbeVtbl,
}

#[test]
fn ping_increments() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let echo = factory.create_echo().unwrap();
    assert_eq!(echo.ping(41).unwrap(), 42);
}

#[test]
fn microcom_probe_releases_through_nativeaot_vtable() {
    let _host = load();
    let path = host_path();
    let library = unsafe { Library::new(path).unwrap() };
    let get_probe = unsafe {
        library
            .get::<unsafe extern "C" fn(*mut *mut MicroComOwnershipProbe) -> i32>(
                b"avn_get_microcom_ownership_probe",
            )
            .unwrap()
    };
    let mut probe = std::ptr::null_mut();
    assert_eq!(unsafe { get_probe(&mut probe) }, 0);
    assert!(!probe.is_null());

    let mut value = 0;
    assert_eq!(
        unsafe { ((*probe).vtbl.as_ref().unwrap().get_value)(probe, &mut value) },
        0
    );
    assert_eq!(value, 42);
    assert_eq!(
        unsafe { ((*probe).vtbl.as_ref().unwrap().release)(probe) },
        0
    );
}

#[test]
fn echo_string_roundtrips() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let echo = factory.create_echo().unwrap();
    assert_eq!(
        echo.echo_string(&host, "hello nano-COM").unwrap(),
        "hello nano-COM"
    );
}

#[test]
fn fail_returns_fixture_hresult() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let echo = factory.create_echo().unwrap();
    assert_eq!(echo.fail(), AVN_E_FIXTURE);
}

#[test]
fn query_interface_identity() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let echo = factory.create_echo().unwrap();
    let unk1 = echo.as_iunknown().unwrap();
    let unk2 = echo.as_iunknown().unwrap();
    assert_eq!(unk1.as_raw() as usize, unk2.as_raw() as usize);
}

#[test]
fn unknown_iid_returns_e_nointerface() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let echo = factory.create_echo().unwrap();
    #[repr(C)]
    struct Bogus;
    unsafe impl avalonia_sys::ComInterface for Bogus {
        const IID: avalonia_sys::Guid = avalonia_sys::Guid {
            data1: 0x11111111,
            data2: 0x2222,
            data3: 0x3333,
            data4: [0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB],
        };
    }
    let err = echo.query_interface::<Bogus>().unwrap_err();
    assert_eq!(err.0, E_NOINTERFACE);
}

#[test]
fn drop_releases_without_crash() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let echo = factory.create_echo().unwrap();
    let extra = echo.clone();
    drop(echo);
    assert_eq!(extra.ping(0).unwrap(), 1);
    drop(extra);
}

#[test]
fn two_echo_instances_are_distinct() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let a = factory.create_echo().unwrap().as_iunknown().unwrap();
    let b = factory.create_echo().unwrap().as_iunknown().unwrap();
    assert_ne!(a.as_raw() as usize, b.as_raw() as usize);
}

#[test]
fn factory_creates_application() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let app = factory.create_application().unwrap();
    let _ = app.as_iunknown().unwrap();
}

#[test]
fn factory_unknown_qi_succeeds() {
    let host = load();
    let factory = host.activation_factory().unwrap();
    let unk: ComPtr<IUnknown> = factory.as_iunknown().unwrap();
    let _again = unk
        .query_interface::<avalonia_sys::IAvnActivationFactory>()
        .unwrap();
}

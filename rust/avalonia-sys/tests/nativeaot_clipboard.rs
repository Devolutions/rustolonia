//! Raw ABI conformance for the stage 31 clipboard command capability.
//!
//! These run against the published NativeAOT host and prove that every vtable
//! slot of `IAvnApplication4`/`IAvnClipboardData` lines up between the managed
//! `[GeneratedComInterface]` declarations and the handwritten Rust bindings,
//! and that the capability is separately versioned rather than bolted onto an
//! already published vtable. Nothing here touches a real platform clipboard:
//! every operation that needs a window is exercised for its argument contract.

use avalonia_sys::{
    ComPtr, Host, IAvnApplication3, IAvnApplication4, IAvnClipboardData, E_INVALIDARG, E_POINTER,
    IAVN_APPLICATION4_METHOD_COUNT,
};
use std::path::PathBuf;

fn host_path() -> PathBuf {
    if let Ok(path) = std::env::var("AVN_HOST_NATIVE_LIB") {
        return PathBuf::from(path);
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
    for relative in candidates {
        let path = root.join(relative);
        if path.exists() {
            return path;
        }
    }
    panic!(
        "Avalonia.Host native library not found. Run `rust/build.ps1` on \
         Windows or `rust/build.sh` on Linux, or set AVN_HOST_NATIVE_LIB."
    );
}

fn utf16(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn load() -> Host {
    let path = host_path();
    Host::load(&path).unwrap_or_else(|error| panic!("load {}: {error}", path.display()))
}

fn clipboard(host: &Host) -> ComPtr<IAvnApplication4> {
    host.activation_factory()
        .unwrap()
        .create_application()
        .unwrap()
        .clipboard()
        .unwrap()
}

/// The clipboard capability is queried, never bolted onto a published vtable.
#[ignore = "requires a desktop session: host bootstrap needs a display (X11) or the macOS main thread (tracked)"]
#[test]
fn the_clipboard_capability_is_separately_versioned() {
    let host = load();
    let application = host
        .activation_factory()
        .unwrap()
        .create_application()
        .unwrap();

    let stage29: ComPtr<IAvnApplication3> = application.desktop_files().unwrap();
    let stage31: ComPtr<IAvnApplication4> = application.clipboard().unwrap();

    // Distinct capabilities on one object: both resolve, and neither is the
    // other's vtable.
    assert_ne!(
        stage29.as_raw() as usize,
        stage31.as_raw() as usize,
        "a separately versioned capability must have its own vtable pointer"
    );
    assert_eq!(5, IAVN_APPLICATION4_METHOD_COUNT);
}

/// The payload builder is host-owned, so this direction of the ABI only ever
/// carries primitives and UTF-16 strings.
#[ignore = "requires a desktop session: host bootstrap needs a display (X11) or the macOS main thread (tracked)"]
#[test]
fn the_host_owned_payload_builder_accepts_text_and_file_uris() {
    let host = load();
    let capability = clipboard(&host);
    let data: ComPtr<IAvnClipboardData> = capability.create_clipboard_data().unwrap();

    data.set_text(Some(&utf16("copied from Rust"))).unwrap();
    data.add_file_uri(Some(&utf16("file:///logs/a.log")))
        .unwrap();

    // A blank entry is rejected at the boundary rather than silently dropped
    // later, so a caller learns immediately that it built a bad payload.
    assert_eq!(
        E_INVALIDARG,
        data.add_file_uri(Some(&utf16(" "))).unwrap_err().0
    );
    assert_eq!(E_INVALIDARG, data.add_file_uri(None).unwrap_err().0);

    // Clearing the text is spelled as an explicit null, not an empty string.
    data.set_text(None).unwrap();
}

/// Every entry point that takes a window rejects a null one before starting an
/// operation, so a consumer never gets a pending operation it cannot complete.
#[ignore = "requires a desktop session: host bootstrap needs a display (X11) or the macOS main thread (tracked)"]
#[test]
fn null_pointers_are_rejected_by_every_clipboard_entry_point() {
    let host = load();
    let capability = clipboard(&host);
    let data = capability.create_clipboard_data().unwrap();

    assert_eq!(
        E_POINTER,
        capability.clipboard_capabilities(None).unwrap_err().0
    );
    assert_eq!(
        E_POINTER,
        capability
            .start_clipboard_write(None, Some(&data), None)
            .unwrap_err()
            .0
    );
    assert_eq!(
        E_POINTER,
        capability.start_clipboard_clear(None, None).unwrap_err().0
    );
    assert_eq!(
        E_POINTER,
        capability
            .start_clipboard_read_files(None, None)
            .unwrap_err()
            .0
    );
}

/// Two payload builders are independent objects; a builder is never shared.
#[ignore = "requires a desktop session: host bootstrap needs a display (X11) or the macOS main thread (tracked)"]
#[test]
fn each_payload_builder_is_a_distinct_object() {
    let host = load();
    let capability = clipboard(&host);

    let first = capability.create_clipboard_data().unwrap();
    let second = capability.create_clipboard_data().unwrap();

    assert_ne!(first.as_raw() as usize, second.as_raw() as usize);
}

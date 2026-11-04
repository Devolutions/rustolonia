# Experimental iOS sample

This sample builds an iOS NativeAOT application that owns the UIKit and
Avalonia single-view lifetime and links the Rust UI as a static library. The
desktop NativeAOT shared library and its download/discovery mechanism are not
used. For a Rust-only consumer that links a prebuilt host without a .NET SDK,
see the [standalone sample](../ios-standalone/README.md); its iOS assets will
be available only after a future release.

**Status:** experimental. The sample has run on an Apple Silicon iOS
simulator, and unsigned ARM64 device and Intel-simulator builds have compiled.
Neither a physical device nor an Intel simulator has been tested. .NET marks
the `ComWrappers` APIs used by the host as unsupported on iOS; the regular
.NET iOS Mono runtime does not provide the required implementation.

Use a .NET 10 SDK with a patched NativeAOT runtime (10.0.14 or later). Older
runtimes, including the 10.0.9 runtime used in initial testing, are affected by
[GHSA-p3jp-9v83-76w9](https://github.com/advisories/GHSA-p3jp-9v83-76w9)
when processing untrusted Brotli data. The 10.0.403 SDK includes the patched
runtime.

## Requirements

- macOS with Xcode, its command-line tools, and an iOS simulator runtime.
- The repository's .NET 10 SDK and the `ios`, `maccatalyst`, and `tvos`
  workloads. Avalonia's iOS producer project targets all three platforms
  during restore.
- Rust and the desired target: `aarch64-apple-ios-sim` for Apple Silicon
  simulators, `x86_64-apple-ios` for Intel simulators, or `aarch64-apple-ios`
  for devices.
- PowerShell 7 to apply the repository's Avalonia patches.

For Xcode 26.6 and .NET SDK 10.0.403, install the compatible workload set with:

```bash
dotnet workload install ios maccatalyst tvos --version 10.0.401
```

Other SDK and Xcode combinations need matching workloads. Check the version
selected by `dotnet --version` if multiple SDKs are installed.

From the repository root, initialize the producer and build for the host's
simulator architecture:

```bash
git submodule update --init --recursive
pwsh ./avalonia-patches/apply-avalonia-patches.ps1 -AvaloniaRoot ./avalonia-src
rustup target add aarch64-apple-ios-sim  # use x86_64-apple-ios on Intel
./samples/ios/build.sh simulator
```

The script prints the resulting `.app` path. Install and launch it on a booted
simulator with:

```bash
xcrun simctl install booted samples/ios/managed/bin/Release/net10.0-ios/iossimulator-arm64/Rustolonia.IosSample.app
xcrun simctl launch booted org.rustolonia.iossample
```

Use `iossimulator-x64` in the path on Intel. The simulator build uses
`dotnet build -p:_IsPublishing=true` to enable NativeAOT because
`dotnet publish` rejects simulator runtime identifiers. `_IsPublishing` is an
SDK implementation detail and may change.

Run `./samples/ios/smoke-test.sh` for an automated simulator check. It builds a
test configuration, creates and removes a dedicated simulator, verifies
layout, a posted Rust callback, button clicks, and key-event marshalling after
garbage collection. It also checks thread and lifetime guards,
unsubscription, shutdown, and restart. Set `RUSTOLONIA_IOS_SIMULATOR_UDID` to
use an already-booted simulator instead.

For an iOS device, install `aarch64-apple-ios`, configure a unique bundle
identifier and Apple signing/provisioning for your development team, then run
`./samples/ios/build.sh device` with any required `dotnet publish` signing
properties. Installing or distributing the app requires valid signing and
provisioning. The script does not produce an IPA by default.
To compile without signing or provisioning for a build-only check, use
`./samples/ios/build.sh device -p:EnableCodeSigning=false`; the resulting
unsigned application cannot be installed on a device.

## Using the host

The managed application calls the exported `rustolonia_ios_main` after
Avalonia initializes. Rust receives an in-process host function table, creates
an `App` with `App::from_in_process`, and calls `start_single_view` on the UI
thread. Keep the returned `SingleViewSession` alive for as long as the view is
in use; call `close` on that same thread when the app is gracefully stopped.
The host function pointers and allocator must remain valid until process exit.
See `rust/src/lib.rs` and `managed/` for the complete bootstrap and error
handling.

The root is a projected `Control`, not a `Window`. UIKit owns the event loop,
and desktop lifecycle, window, tray, and window-dependent file APIs are not
supported by this sample. iOS may terminate a suspended process without a
graceful callback, so persist important state before suspension.

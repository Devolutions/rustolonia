# Experimental standalone iOS app

This code-first app builds its UI in Rust and links against a prebuilt
Rustolonia NativeAOT iOS host. The host contains precompiled managed code and
the .NET runtime; building the app does **not** compile C# or require a .NET
SDK. Xcode still performs the final native link and app packaging.

The iOS host assets will be available only in a future release that includes
this feature; existing releases do not contain them. The repository sample
uses the local crates so it can also test a locally produced host.

**Status:** experimental. The Apple Silicon simulator path has been exercised.
Unsigned ARM64 device and Intel-simulator builds are build-only checks; neither
physical-device execution nor Intel-simulator execution has been verified.
.NET marks the `ComWrappers` APIs used by the host unsupported on iOS.

## Requirements

- macOS with Xcode 26.6 and its command-line tools. A simulator build also
  requires an installed iOS simulator runtime.
- Rust 1.88 or newer and the appropriate target: `aarch64-apple-ios-sim`,
  `x86_64-apple-ios`, or `aarch64-apple-ios`.
- Python 3, `curl`, `shasum`, and the Apple command-line tools.
- A matching iOS host archive from a release that includes these assets.
  The minimum supported iOS version is 13.0.

For the in-repository sample, after a matching release has been published:

```bash
rustup target add aarch64-apple-ios-sim  # x86_64-apple-ios on Intel
./samples/ios-standalone/build.sh simulator
```

The script selects the exact host version from the resolved `rustolonia-sys`
crate, downloads the matching release archive, verifies its SHA-256 against
that crate's `host-checksums.txt` before extraction, builds the Rust static
library, and links an ad-hoc-signed simulator `.app`. It also verifies the
host's RID, version, ABI fingerprint, and every packaged file. Install and
launch the printed app path with:

```bash
xcrun simctl install booted samples/ios-standalone/target/aarch64-apple-ios-sim/release/RustoloniaStandalone.app
xcrun simctl launch booted org.rustolonia.standalone
```

Use the `x86_64-apple-ios` path on Intel. The output directory is owned by the
build script and may be replaced on later builds; it refuses to replace an app
without its ownership marker.

To copy the sample into an independent Rust project, copy `Cargo.toml`,
`src/`, `Info.plist`, and `build.sh`, then replace the path dependency with
the tag of a release containing iOS hosts:

```toml
[dependencies]
rustolonia = { git = "https://github.com/Devolutions/rustolonia", tag = "vX.Y.Z", default-features = false }
```

Replace `X.Y.Z` with the actual matching release version. If that release is
also published to crates.io, `rustolonia = { version = "=X.Y.Z",
default-features = false }` is an alternative. Run `cargo generate-lockfile`
once after changing the dependency and commit the resulting `Cargo.lock`;
subsequent builds use `--locked`. The app exports its startup callbacks with
`rustolonia::ios_application!(build_view)` and has `crate-type = ["staticlib"]`.
The copied `build.sh` does not depend on the repository or the source-built
sample.

For an offline archive, set `RUSTOLONIA_IOS_HOST_ARCHIVE` to its path. The
normal release checksum from `rustolonia-sys` is still required. For a local
producer build before the release exists, additionally set
`RUSTOLONIA_IOS_HOST_SHA256` to the trusted archive's lowercase SHA-256. This
explicit override replaces the release-pinned checksum; obtain it from a
trusted source.

`./build.sh device` cross-links an unsigned ARM64 device app. Installing or
distributing it requires a unique bundle identifier, a provisioning profile,
entitlements, and Apple signing of the embedded frameworks and app. The script
does not create an IPA or perform device signing. The build script packages the
host's runtime resources and license notices, but does not compile custom
AXAML or asset catalogs.

The UIKit event loop owns a single Avalonia view. Desktop window, tray, and
window-dependent file APIs are not supported. iOS may terminate a suspended
process without a graceful shutdown callback; persist important state before
suspension.

## Producing a host locally

Host production requires the patched Avalonia checkout, .NET SDK, and iOS
workloads described in the [source-built sample](../ios/README.md). For
example:

```bash
pwsh ./rust/package-ios-host.ps1 -Rid iossimulator-arm64 -OutputRoot ./rust/artifacts/host
version="$(python3 -c 'import json; print(json.load(open("rust/release-manifest.json"))["rustoloniaVersion"])')"
export RUSTOLONIA_IOS_HOST_ARCHIVE="$PWD/rust/artifacts/host/rustolonia-host-$version-iossimulator-arm64.tar.gz"
export RUSTOLONIA_IOS_HOST_SHA256="$(cut -d ' ' -f 1 "$RUSTOLONIA_IOS_HOST_ARCHIVE.sha256")"
./samples/ios-standalone/smoke-test.sh
```

The smoke test builds a different Rust UI against the production host,
exercises a posted callback and several projected controls after the view
loads, and checks a simulator screenshot for rendered content. Set
`RUSTOLONIA_IOS_SIMULATOR_UDID` to use an already-booted simulator.

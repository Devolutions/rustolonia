# Platform hosts

The Rust ABI and generated view-model pipeline are platform-neutral. The
NativeAOT host selects one Avalonia windowing backend at publish time:

| Host platform | Runtime identifiers | Backend | Build entry point |
| --- | --- | --- | --- |
| Windows | `win-x64`, `win-arm64` | Win32 | `build.ps1` |
| Linux | `linux-x64`, `linux-arm64` | X11 | `build.ps1` |
| macOS | `osx-x64`, `osx-arm64` | Avalonia.Native | `build.ps1` |

`AvaloniaRustHostPlatform` controls the compile-time selection. The shared
host contains the nano-COM ABI, ownership runtime, generated control
projection, and generated view registry. `RustHostPlatform` contains only
backend setup and the Win32 OLE thread scope.

Linux publishes the host with a `$ORIGIN` runpath. Consequently
`libSkiaSharp.so` and `libHarfBuzzSharp.so` are resolved beside
`librustolonia_host.so` without requiring a process-wide `LD_LIBRARY_PATH`.

macOS publishes against `Avalonia.Native` and loads
`libAvaloniaNative.dylib` from `@loader_path`. `build.ps1`, `package.ps1`,
and `build-app.ps1` generate the native COM headers and invoke the producer's
`xcodebuild` project with an explicit target `ARCHS` and
`CONFIGURATION_BUILD_DIR=Build/Products/Release`, so the generated
`libAvalonia.Native.OSX.dylib` is included by `Avalonia.Native` as
`libAvaloniaNative.dylib` in the NativeAOT publish output.

`build.ps1` publishes the host and runs `cargo test --workspace`. It requires
the requested architecture to match `RuntimeInformation.OSArchitecture` on
all three operating systems. `regenerate-and-build.ps1` regenerates sources,
builds managed code unless skipped, and runs tests only with `-Test`.

`package.ps1` and `build-app.ps1` build delivery bundles; they do not run the
workspace tests. Both allow cross-architecture packaging on the same OS,
with a warning that the target binary cannot be smoke-launched on the build
runner. Install the requested Rust target and its native linker/toolchain.
Linux cross-builds also require the target `objcopy` (for example,
`aarch64-linux-gnu-objcopy`) and a configured Cargo cross-linker. Cross-OS
packaging is rejected. macOS packages use the requested target architecture
for Xcode even when the build runner has a different architecture.

Bundle construction, signing, SBOM generation, and checksums happen in an
isolated staging directory. Only a completed bundle replaces an existing
empty or Rustolonia-owned destination; a failed build leaves the previous
bundle intact. External consumer manifests must name their
`generatedRegistryFile` **`RustViewRegistry.g.cs`**, in any chosen directory.

CI uploads `.tar.gz` bundles rather than raw directories, preserving the hidden
ownership marker and Unix execute permissions. It downloads and extracts the
actual uploaded archives before checking checksums. Native jobs smoke-launch
the extracted sample; cross-build jobs verify checksums and Unix execute
permissions without attempting to execute the target binary. CI also runs
the patched Avalonia table accessibility, column-width, and viewport tests.

## Linux build

Initialize the producer and its dependencies from the Rustolonia root:

```bash
git submodule update --init --recursive
```

Then publish the X11 NativeAOT host and run the complete Rust workspace:

```bash
pwsh ./rust/build.ps1
```

Pass `-Architecture arm64` for native Linux ARM64. The PowerShell entry points
invoke `dotnet` from `PATH`; they do not read a `DOTNET` override. WSL users can
select a local SDK through `PATH` and keep intermediate files on the Linux
filesystem:

```bash
PATH="$HOME/.dotnet:$PATH" \
AVN_DOTNET_ARTIFACTS="$HOME/.cache/avalonia-rust/dotnet-linux-x64" \
CARGO_TARGET_DIR="$HOME/.cache/avalonia-rust/cargo-linux-x64" \
pwsh ./rust/build.ps1
```

The X11 host requires the standard Avalonia Linux runtime libraries, including
X11, fontconfig, and OpenGL/EGL or software-rendering dependencies.

## macOS native lifecycle tests

For macOS desktop lifecycle regression coverage, build and stage the host with
`pwsh ./rust/build.ps1 -Architecture arm64` (or `x64` on an Intel runner), then
set `RUSTOLONIA_HOST_LIB` to the published `librustolonia_host.dylib` and
`RUSTOLONIA_HOST_DIR` to its directory. Run
`cargo test --manifest-path rust/Cargo.toml -p rustolonia --test native_host_shutdown -- --ignored`.
The tests launch the host on a child process's main thread, check process-exit
shutdown, and exercise a mounted window with tray icon/menu setup and cleanup.
The tray fixture uses the initialized Avalonia submodule's `Star512.png` test asset.

The popup fixture opens real macOS tooltips and `MenuFlyout`s on two top-levels:
`AppContext::set_overlay_popups(&window, true)` keeps the protected window's
popups embedded, while the normal window continues to use native popups.
AppKit visible-window counts verify that the embedded opens allocate no visible
popup window. Set the policy before opening any popup; changing it does not
migrate an already-open native popup. Nested Avalonia popups use the same host
selection path. System-owned dialogs and native menu-bar/tray menus are outside
this policy.

For ScreenCaptureKit, Jiffshot must still cloak the owner, register its window
number and await every active stream's filter acknowledgement before revealing
it. This policy does not perform capture exclusion or rely on NSWindow sharing
flags. Because embedded popups draw into the excluded owner rather than another
NSWindow, they introduce no separate first-frame exclusion acknowledgement.
Overlay content cannot extend outside the owner's client area; toolbars may
need a larger host layout. Direct first-frame recording assertions remain the
capture application's responsibility.

### Native popup pre-show gate

Compact excluded windows can instead use
`AppContext::set_native_popup_gate(&window, handler)` on macOS. Set this
independently on each protected owner (toolbar, Capture Widget, etc.) before
opening popups. Leave `set_overlay_popups` disabled on those owners.
`NativePopupEvent::Preparing { token, window_number }` arrives on the UI thread
after the native popup exists but before its first show. Return promptly,
resolve its hidden `SCWindow` with `onScreenWindowsOnly: false`, and update
every active stream's filter asynchronously. Only after all filter updates
acknowledge success, post `acknowledge_native_popup(token)` onto the UI thread.
Until then there is no native show, focus transfer, or light-dismiss overlay.
Normal popup size, screen placement and input resume at acknowledgement.

`Closed { token }` invalidates the token. Closed/recreated popups use distinct
tokens; stale and duplicate acknowledgements return `E_INVALIDARG`. Do not
remove the exclusion until in-flight filter operations have completed safely.
Close all owner popups before clearing/replacing the handler. There is no
implicit timeout/reveal on failure. The contract exchanges values only, never
borrowed NSWindow pointers, and applies to nested Avalonia popup owners.
Native menu-bar/tray menus and system dialogs remain outside this policy.

For an auxiliary-application COM bridge, close owner popups (or close the
owner) on the UI thread, process their `Closed` callbacks, then call
`SetNativePopupGate(owner, null)` on that same auxiliary instance before
dropping its COM reference and shutting down the running application.
Clearing while a popup is logically open is rejected. An owned projected
`IAvnWindow` can clear the handler after `Window.close()`: the managed owner
is still retained, although its borrowed NSWindow handle is no longer valid.
Do not use this as permission for native-window operations after closure.
The native auxiliary teardown matrix covers pending/revealed tooltips and
menus with both owner close and application shutdown. All emit one `Closed`
per token before those calls return, and clearing then succeeds. Cancellation
of owner/application closure must still be handled by the caller.
Managed COM wrappers may retain the callback until garbage collection even
after clearing and releasing caller references; callback state must remain
owned by its COM reference count, not freed manually at shutdown. Host loading
is process-lifetime; neither this sequence nor `Closed` permits unloading the
host library or proves a ScreenCaptureKit frame-drain fence.

The v12.1.2 Rust application handler exposes startup only, not the managed
lifetime's `Exit` event. For automatic shutdown, register each protected
owner's public `on_unloaded(scope, callback)` before mounting. Clear that
owner's gate synchronously in the UI-thread callback, not with `post`.
The native fixture verifies popup `Closed` precedes owner `Unloaded` and
null-handler clearing succeeds there for pending/revealed tooltips and menus
with `OnLastWindowClose`, without calling `AppScope.shutdown()`. This is a
per-owner notification, not a global after-all-windows-close hook. Ordinary
managed OS quit follows the same owner-close path in Avalonia; actual OS quit
was not exercised. Forced process termination provides no cleanup guarantee.

The macOS fixture checks hidden-before-ack, real native tooltip/MenuFlyout
show after ack, asynchronous posted acknowledgement, and close/recreation
races. A public ScreenCaptureKit probe confirmed hidden NSWindow discovery on
the development Mac; the capture application must still validate this on its
supported macOS versions and assert first-frame stream pixels. No sharing-flag
capture guarantee is claimed.

Tooltips default to `PlacementMode::Pointer`, using the owner's last pointer
event position (desktop origin if none has arrived). A programmatic
`set_tool_tip_is_open(true)` does not simulate a hover or anchor to the button.
For a deterministic button-anchored tooltip, first call
`set_tool_tip_placement(PlacementMode::Bottom)`. The native placement fixture
compares actual AppKit frames with/without gating after moving an alpha-zero
owner from offscreen, including a dispatcher-posted acknowledgement:
`cargo test --manifest-path rust/Cargo.toml -p rustolonia --test native_host_shutdown native_tooltip_placement -- --ignored --nocapture`.

To repeat the discovery probe without requesting new privacy permission:
`swiftc -parse-as-library rust/rustolonia/tests/native_popup_capture_probe.swift -o rust/target/popup-capture-probe`
then run `rust/target/popup-capture-probe`. It exits with code 2 when permission
is absent and code 1 if hidden-window discovery fails.

## Prebuilt host platform floors

Release tarballs (`rust/package-host.ps1`) define the minimum OS a published
`rustolonia-sys` supports:

| RID | Build environment | Floor |
| --- | --- | --- |
| `win-x64`, `win-arm64` | `windows-latest` / `windows-11-arm` runners | Windows 10 1607 / Server 2016 (.NET 10 support floor) |
| `linux-x64`, `linux-arm64` | `ubuntu-22.04` / `ubuntu-22.04-arm` runners | glibc 2.35 |
| `osx-x64`, `osx-arm64` | `macos-14` (Apple silicon) runners for both; osx-x64 is cross-built and smoke-tested under Rosetta 2; `MACOSX_DEPLOYMENT_TARGET=12.0` | macOS 12 |

Linux hosts are glibc-only; musl RIDs are not published. They need an X11
display (XWayland works), the X11 client libraries (`libX11`, `libICE`,
`libSM`) and fontconfig at runtime.

## Cross-platform gate result

The generated `rust_vm_axaml` application was published and launched through
the Linux X11 NativeAOT host under WSLg. The same generated Rust model and
managed AXAML presentation handled:

- two-way model text editing
- synchronous command dispatch
- observable collection insertion
- Rust worker-thread status publication
- normal window shutdown

No platform-specific ABI or generated model code was added. The tested
`linux-x64` host is 21.8 MB, plus 11.2 MB for Skia and 2.8 MB for HarfBuzz.
The corresponding `win-x64` host is approximately 20 MB before its two native
rendering dependencies.

The sample interaction performs six calls for initial attachment/state and
twelve calls for the edit, increment, add, and async-save flow. Platform
selection does not add boundary calls. Its application schema is 72 lines and
generates 216 lines of Rust model API, managed adapter, and host registry.

**Decision:** Rust-owned view models with managed compiled AXAML remain the
recommended full-application architecture. The platform gate no longer blocks
model work. Code-first control projection should still expand only when either
application mode demonstrates a concrete missing capability.

## Desktop file integration (stage 29)

Pickers, incoming file drag-and-drop and "open with" activation are
platform-neutral: they use `TopLevel.StorageProvider`, the `DragDrop` routed
events and the desktop lifetime, so one Rust API covers all three hosts and no
platform dialog code was added. Design and rules are in
[DESKTOP_FILES.md](DESKTOP_FILES.md); what differs per host is only what the
platform itself supports:

| Capability | Windows (Win32) | Linux (X11) | macOS (Avalonia.Native) |
| --- | --- | --- | --- |
| Open / folder / save pickers | yes | yes | yes |
| Folder multi-select | reported by `StorageCapabilities`; honoured where the platform picker supports it |||
| File type filters | glob patterns | glob patterns and MIME types | uniform type identifiers |
| Incoming file drop | yes | yes | yes |
| Startup "open with" | command line | command line | command line |
| Later activation (`on_activation`) | not raised | not raised | `Files`, `OpenUri`, `Reopen` |
| Local path for every item | usually | usually | not guaranteed (security-scoped items) |

Consumers always get a URI; `StorageItem::local_path()` is optional by design
and must not be assumed. Where a host has no activation feature the
subscription stays valid and never fires, so no consumer needs a platform
branch.

**Decision:** desktop file integration ships as one separately versioned
capability interface rather than per-platform host methods. Drag-effect
negotiation stays a conservative, subscription-time declaration instead of a
synchronous Rust callback, because calling an external consumer from inside a
platform drag loop is not safe. File type associations are packaging metadata
(template snippets), not application code; MSIX and platform installers remain
out of scope.

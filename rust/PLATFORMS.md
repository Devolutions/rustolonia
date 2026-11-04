# Platform hosts

The Rust ABI and generated view-model pipeline are platform-neutral. The
NativeAOT host selects one Avalonia windowing backend at publish time:

| Host platform | Runtime identifiers | Backend | Build entry point |
| --- | --- | --- | --- |
| Windows | `win-x64`, `win-arm64` | Win32 | `build.ps1` |
| Linux | `linux-x64`, `linux-arm64` | X11 | `build.ps1` |
| macOS | `osx-x64`, `osx-arm64` | Avalonia.Native | `build.ps1` |
| iOS (experimental) | `ios-arm64`, `iossimulator-arm64`, `iossimulator-x64` | Avalonia.iOS | `samples/ios/build.sh` |
| Android | `android-arm64`, `android-x64` | Avalonia.Android (CoreCLR) | `build-android.ps1` (see [Android](#android)) |

`AvaloniaRustHostPlatform` controls the compile-time selection. The shared
host contains the nano-COM ABI, ownership runtime, generated control
projection, and generated view registry. `RustHostPlatform` contains only
backend setup and the Win32 OLE thread scope. The source-built iOS host is not
a shared library: UIKit owns the event loop and a Rust static library supplies
one main view. See the [iOS sample](../samples/ios/README.md) for requirements
and limitations.

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

Outside pull requests, CI uploads `.tar.gz` bundles rather than raw directories, preserving the hidden
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

### Application-wide macOS termination coordination

Optional `IAvnApplication11` can be queried from the running application or
an auxiliary application created through the same loaded host. Install its
handler on the UI thread inside `App::run`, before users can request quit.
This is process/application-wide and does not require a shown window or tray.
Only one application interface can own it. The original v12.1.2 interfaces
and ABI-header fingerprint are unchanged; consumers pinned to those crates
can declare these additional COM types themselves:

| Interface | IID | Methods after IUnknown, in order |
| --- | --- | --- |
| IAvnApplication11 | `6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA4` | `SetTerminationHandler(handler*) -> HRESULT`; `CompleteTermination(i64 token, i32 accept) -> HRESULT` |
| IAvnTerminationHandler | `6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA5` | `OnRequested(i64 token, i32 flags) -> HRESULT` |

Methods use `extern "system"`/PreserveSig and ordinary IUnknown ownership.
`OnRequested` runs on the UI thread before any native-quit window teardown.
Flags are bitwise: 1 means AppKit/native request, 2 means OS logout/restart/
shutdown reason. Zero means managed lifetime `TryShutdown` (including
automatic shutdown). Return promptly, start idempotent async save/capture
finalization, and post completion to the UI dispatcher. Synchronous completion
inside the callback is rejected: AppKit has not yet received its deferred reply.
Repeated requests reuse the pending token, OR their flags, and notify again.
Deduplicate work by token; a request with upgraded flags is not a new cleanup.
Callback failure is reported through host error diagnostics and remains held.

AppKit receives public `NSTerminateLater`; no windows close until the consumer
acknowledges. `CompleteTermination(token, 0)` cancels and sends
`replyToApplicationShouldTerminate:false` for native requests. A later request
gets a fresh token. `CompleteTermination(token, 1)` accepts: after the caller
has finished cleanup, the host closes managed windows/popups synchronously,
removes coordination, then replies true to AppKit. `Closed`/owner `Unloaded`
callbacks can therefore clear popup gates after cleanup but before
`NSApplicationWillTerminateNotification` and platform disposal. A native
accepted quit may exit without returning from `App::run`; no mandatory cleanup
belongs after that call. Managed acceptance instead unwinds the managed loop.

Unknown/duplicate tokens and accept values other than 0/1 return E_INVALIDARG.
Reconfiguration/removal with a pending or completing request is rejected;
cancel it first, then set a null handler. Retain the same owning I11 reference
through completion and release it afterward, without strong callback-reference
cycles. Managed wrappers can retain the callback until GC; never manually free
its state independently of COM Release. Explicit `AppScope.shutdown()` is a
forced shutdown and intentionally bypasses this coordination. Last-window
automatic shutdown is intercepted only after that window has already closed;
save presentation state in window Closing when this mode is used.

Both the new host and rebuilt `libAvaloniaNative.dylib` must be deployed
together. Installation verifies the companion's additive
`avn_supports_deferred_termination` export and fails on an older companion
rather than accidentally accepting quit. Apply the hash-pinned
`avalonia-termination.patch` before rebuilding producer native code.

The desktop fixture tests managed automatic requests, tray-only native quit,
repeated requests, a real deferred cancel/retry, and cleanup-before-popup/
window/WillTerminate ordering. Its OS case sends a quit AppleEvent with a
logout reason to its own child PID; it does not log out the machine.
ARM64 and Rosetta x86_64 are covered; actual machine logout/restart, Intel
hardware, macOS 13, AppKit's finite OS-termination deadline, and force-kill
cannot be inferred from those results. No UI-thread blocking wait or timeout
that accepts failed cleanup is introduced.

To repeat the discovery probe without requesting new privacy permission:
`swiftc -parse-as-library rust/rustolonia/tests/native_popup_capture_probe.swift -o rust/target/popup-capture-probe`
then run `rust/target/popup-capture-probe`. It exits with code 2 when permission
is absent and code 1 if hidden-window discovery fails.

## Additive outgoing file drags (macOS and Linux X11)

Linux X11 hosts implement the same ABI on top of Avalonia's XDND drag source
(`X11DragSource`): no native companion, retained event or capability probe is
needed, and `DragDrop.DoDragDropAsync` starts from the recorded pointer press.
Windows still returns `E_NOTIMPL`. The rest of this section describes the
macOS-specific parts (retained AppKit event, `libAvaloniaNative.dylib` probe).

The local host optionally implements `IAvnApplication12`, IID
`6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA6`. It leaves the published v12.1.2
interfaces and crate pins unchanged. Consumers may declare the following
`extern "system"` vtable slots after `QueryInterface`, `AddRef`, `Release`:

```text
HRESULT SetFileDragHandler(IAvnControl* target, IAvnFileDragHandler* handler);
HRESULT CompleteFileDrag(int64_t token, const uint16_t* fileUrlsJson);
HRESULT CancelFileDrag(int64_t token);
```

`IAvnFileDragHandler`, IID `6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA7`, has
these slots after the same three IUnknown slots:

```text
HRESULT OnPreparing(int64_t token);
HRESULT OnCompleted(int64_t token, int32_t effect, HRESULT status);
```

Use owned COM projections and an ordinary reference-counted callback CCW.
An auxiliary application obtained through the public v12.1.2 activation
factory can QI I12; keep it alive and use that same application for the entire
registration/token lifetime. All methods and callbacks run on the UI thread.
The host retains the control, owner and callback while registered. Dropping
the caller's callback reference after successful installation is valid;
clearing a registration is not a synchronous managed-GC destruction fence.
Targets must already be mounted in a Window. Overlapping targets, including
registrations on ancestors/descendants from another auxiliary application,
are rejected. A null handler removes a registration, cancelling a pending
gesture; clearing/replacing during its callback or native drag is rejected.

A genuine AppKit left mouse-down is retained without handling the press.
Below five DIPs of movement, ordinary Button click/capture behavior is
unchanged. Crossing the threshold resets pressed state through Avalonia's
capture-lost path, recaptures the target while preparing, suppresses its click,
and emits `OnPreparing`. Post work/completion rather than mutating UI or
calling the bridge synchronously from callbacks. Return promptly, prepare
edited snapshots on a worker, then post `CompleteFileDrag`. It consumes a
NUL-terminated UTF-16 JSON array of one or more absolute, existing, local file
URLs. It copies the string before returning; no borrowed event/control/native
pointer is exported. Invalid payloads fail synchronously and leave the token
pending for correction or cancellation.

Successful completion submission means accepted, **not dropped**. The host
resolves files, revalidates the gesture after each await, and invokes Avalonia's
outgoing drag with the retained original native event and Copy only. A second
submission while resolving/dragging fails. Release, capture loss, Escape,
detachment or owner closure cancels preparation. Tokens are never reused on
that application; stale, duplicate, callback-reentrant and already-started
operations return `E_INVALIDARG`. `OnCompleted` arrives once per preparing
token: effect 1 is copied, 0 is cancelled/no drop; a negative status is a host
failure. A failing preparing callback cancels with that HRESULT. A native
session is owned by AppKit and cannot be programmatically cancelled with this
ABI; native completion remains authoritative even if its owner closes.
Keep snapshots and the auxiliary application alive until completion. Do not
dispose the host while a native drag is outstanding; finish/cancel through
AppKit and clear registrations first. Forced process exit cannot promise
completion. No move/link effect is offered, so original files are not removed.

The host requires the hash-pinned `avalonia-file-drag.patch` in the native
companion. The no-UI preflight is to load the exact staged
`libAvaloniaNative.dylib` in a dedicated child and call its default-visible C
export `int avn_supports_retained_file_drag_event(void)`, requiring 1. I12 QI
alone cannot identify an old native companion. The companion also exports
UI-thread-only `void* avn_retain_file_drag_event(int windowNumber)`,
`int avn_set_file_drag_event(void* retainedEvent)` and
`void avn_release_file_drag_event(void* retainedEvent)` for the host's scoped
event selection. They are not a consumer drag API; do not change the selected
event while the host is starting a drag.

Build into separate drag-specific artifact directories, not an existing
termination bundle:

```sh
pwsh ./avalonia-patches/apply-avalonia-patches.ps1 -AvaloniaRoot ./avalonia-src
dotnet publish host/Avalonia.Host.csproj -c Release -r osx-arm64 \
  -p:AvaloniaRustHostPlatform=OSX --artifacts-path rust/target/dotnet-drag-osx-arm64
dotnet publish host/Avalonia.Host.csproj -c Release -r osx-x64 \
  -p:AvaloniaRustHostPlatform=OSX --artifacts-path rust/target/dotnet-drag-osx-x64
```

The ignored `file_drag_preserves_clicks_and_rejects_cancelled_gestures`
desktop fixture dispatches self-directed `NSEvent` mouse events after initial
activation, without changing the user's cursor. It covers click preservation,
pending release/cancel/owner-close, callback-reentrant/stale/duplicate operations,
invalid payload retry, delayed two-file preparation and actual AppKit native
session cancellation with Escape, including closing an active session's owner.
A child-only NSView method observer calls the original AppKit implementation
and verifies retained mouse-down, Copy in both dragging contexts, and both file
URLs through an isolated NSPasteboard NSURL reader. This is real native
session/cancellation plus receiving-data coverage, **not** a successful
cross-application drop into Finder or a hardware-pointer gesture. Manual
Finder/other-app successful-drop coverage, native Intel hardware and execution
on macOS 13 remain separate integration checks.

## Android

Android inverts the desktop bootstrap. The Android runtime owns the process,
so the Rust application is a `cdylib` that the host starts, not an executable
that loads the host:

1. The APK's `[Application]` subclass of `RustoloniaAndroidApplication`
   (`host/Android`) sets Avalonia up with `UseAndroid` and replaces
   Avalonia.Android's windowing stub with *logical windows*.
2. It loads `lib<crate>.so` and calls the `rustolonia_android_main` export
   that `rustolonia::android_main!(ui::build)` declares, on the UI thread,
   passing the same in-process `HostExports` function-pointer table the iOS
   host uses (`avn_get_activation_factory`, `avn_free`, `avn_alloc_utf16`,
   `avn_get_last_error`, `avn_get_host_info`; `struct_size`-versioned).
3. `App::from_in_process` (`rustolonia_sys::Host::from_in_process`) applies
   the same ABI fingerprint and allocator checks as `Host::load`. `App::run` calls the same `ui::build`
   the desktop `main.rs` uses, then returns: the activity owns the UI loop,
   and the application stays alive for the process lifetime.

The ABI header and fingerprint are unchanged; `HostExports` is a bootstrap
table, not a COM interface. `host/Android/Avalonia.Host.Android.csproj`
compiles the same shared host sources as the desktop host (assembly identity
`Avalonia.Host`) with `AVN_HOST_ANDROID`; the desktop project is unaffected.

**Runtime.** The APK must use CoreCLR (`<UseMonoRuntime>false</UseMonoRuntime>`,
.NET 10+): the nano-COM ABI relies on `ComWrappers`, which Mono rejects with
`PlatformNotSupportedException`. CoreCLR on Android supports arm64 and x64.

**Windows.** `new Window()` creates a real Avalonia `Window` over a surfaceless
platform impl. Shown windows form a stack presented by the activity:

- the top window's `Content` is moved into the activity (setting it again
  later moves the new content too), with its `DataContext`; its `Title`
  becomes the activity title;
- Back closes the top window while more than one is shown (its `Closing`
  handlers can cancel); closing the last window finishes the activity;
- `AppScope::set_main_view` (`IAvnApplication13`, as in the browser) sets a
  root that is shown whenever no window is; Back and closing a window then
  return to it instead of finishing the activity;
- size, position, owner, topmost, decorations and window state are ignored;
- clipboard, storage pickers, launcher and insets resolve through the
  activity's top level, so they become available once the activity exists.

**Capabilities.** Tray icons, native window handles, topmost/transparency/
capture exclusion, outgoing file drags, deferred termination and the native
popup gate report `E_NOTIMPL` (or "unsupported"), as on desktop hosts without
them. Startup arguments are always empty; desktop `on_activation` never fires.

**Build.** Install the .NET `android` workload, an Android SDK with
`platforms;android-36`, `build-tools`, `platform-tools` and an NDK, and
`rustup target add aarch64-linux-android x86_64-linux-android`. Then:

```pwsh
pwsh ./rust/build-android.ps1 -Abi x86_64 -Install -Launch   # emulator
pwsh ./rust/build-android.ps1                                # arm64-v8a + x86_64 APK
```

`apps/android-hello` is the reference layout: `Cargo.toml` builds
`crate-type = ["cdylib", "rlib"]`, `src/lib.rs` holds `android_main!`,
`src/main.rs` keeps the desktop entry, and `android/` holds the APK project
(`ApplicationId`, theme, `[Application]`/`[Activity]` subclasses and one
`AndroidNativeLibrary` per ABI). Rust `stdout`/`stderr` and panics go to logcat
(`adb logcat -s rustolonia`). `adb shell setprop log.tag.rustolonia DEBUG`
additionally logs every first-chance managed exception, since projected calls
report failures to Rust only as HRESULTs.

**Verified.** The `android-hello` Release APK (arm64-v8a + x86_64, 55 MB)
launched on an API 36 x86_64 emulator, rendered the Rust-built window, ran
the Rust click handler on tap, and exited cleanly on Back. Not yet covered:
physical arm64 devices, clipboard and pickers at run time, multiple windows,
compiled AXAML / Rust view-model apps (`build-app.ps1`), NativeAOT for
Android, and CI.

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
| Outgoing file drag (`IAvnApplication12`) | no | yes | yes |
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

## Browser (WebAssembly)

Avalonia runs in the browser on the .NET WebAssembly runtime (Mono), and so
does Rustolonia. Browser hosting inverts the desktop architecture:

| | Desktop | Browser |
| --- | --- | --- |
| Process owner | the Rust executable | the .NET WebAssembly runtime |
| Host | NativeAOT `Avalonia.Host` shared library, loaded with `libloading` | `host/Browser/Avalonia.Host.Browser.csproj`, a `Microsoft.NET.Sdk.WebAssembly` app |
| Rust app | executable | `wasm32-unknown-emscripten` **staticlib** linked into `dotnet.native.wasm` |
| Entry point | Rust `main` | the host's `Main` calls the Rust `extern "C" fn rustolonia_main()` |
| Host exports | resolved with `dlsym` | resolved statically by `wasm-ld` (`Host::load_static`) |
| Lifetime | classic desktop, `App::run` blocks | single view in `<div id="out">`, `App::run` returns once started |

### Requirements

- `rustup target add wasm32-unknown-emscripten`
- the .NET 10 SDK with the `wasm-tools` workload
  (`dotnet workload install wasm-tools`); it brings the Emscripten SDK and
  `llvm-objcopy` used below
- network access on the first build: Avalonia.Browser bundles its JavaScript
  modules (`avalonia.js`) with Bun, which it runs through `dnx` from NuGet

### Build and run

```powershell
pwsh rust/build-browser.ps1 -Serve            # the bundled counter sample
pwsh rust/build-browser.ps1 -Package my-app   # any staticlib crate in the workspace
```

The script:

1. builds the crate for `wasm32-unknown-emscripten` with `panic = "abort"`,
   because Rust's default unwinding uses wasm exception handling that the
   .NET runtime's Emscripten link does not enable;
2. removes the `target_features` section from the archive with
   `llvm-objcopy`, because the .NET runtime's `wasm-ld` rejects objects that
   declare features it was not built with (for example `bulk-memory` from
   recent Rust toolchains), writing `rust/target/browser/rustolonia_app.a`;
3. bundles Avalonia.Browser's JavaScript modules (`BunRunBuild`);
4. publishes the browser host with `NativeFileReference` to the archive into
   `rust/target/browser/publish/wwwroot`, which any static file server can
   serve. `-Serve` starts `python3 -m http.server` on `-Port` (default 8080).

`-DotnetArgs` forwards extra arguments (for example `-p:RestoreConfigFile=...`)
to the `dotnet` invocations. The host project accepts
`-p:RustoloniaBrowserApp=<path to rustolonia_app.a>` directly; the archive
file name must be `rustolonia_app.a` because the host imports
`rustolonia_main` from the `rustolonia_app` module.

### Application shape

A browser app is a `staticlib` crate exporting `rustolonia_main`. It shows its
UI with `AppScope::set_main_view` instead of a window:

```rust
#[no_mangle]
pub extern "C" fn rustolonia_main() -> i32 {
    match App::load_from_env().and_then(|app| app.run(build_ui)) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

fn build_ui(scope: &AppScope) -> Result<()> {
    let root = StackPanel::new()?;
    // ... build controls and subscribe to events ...
    scope.set_main_view(&root)
}
```

`set_main_view` uses the separately versioned `IAvnApplication13` capability
(IID `6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA8`), which is outside the generated
ABI header and fingerprint. It returns `E_NOTIMPL` on desktop hosts, whose
lifetime has no main view. On the browser `App::run` returns as soon as the single-view lifetime has
started, so the startup callback runs later on the page's event loop. The
control factory, the application scope (and every subscription it retains)
and the host stay alive for the lifetime of the page. See
`rust/browser-sample` for a complete example.

### COM interop on the browser runtime

The host and interop assemblies use source-generated COM
(`[GeneratedComInterface]`, `[GeneratedComClass]`, `ComWrappers`), which the
.NET WebAssembly runtime does not support. For the browser target framework,
`build/BrowserCom.targets` compiles shadow copies of the
`System.Runtime.InteropServices.Marshalling` types that the generator targets
(`interop/Avalonia.Rust.BrowserCom/Shadow`) and references
`Avalonia.Rust.BrowserCom`, a small COM runtime that builds the managed object
wrappers and native object proxies from the generator's own vtables. A build-time
generator adapts the .NET SDK's COM source generator and emits the proxy types,
interface registrations and class registrations ahead of time. The deployed COM
shim does not emit types, scan assemblies or discover attributes at runtime.
The generator loads the SDK generator in the compiler process; that build-time
reflection is not part of the browser app.

The ABI seen by Rust is identical on both platforms. Optional casts between
versioned interfaces continue to follow `QueryInterface`: the generator emits
a proxy for each possible combination of supported interfaces in a family.
This has an exponential build-time and size cost (the 13-interface application
family needs 8,191 proxy types); families are currently limited to 16
interfaces and produce a build error beyond that limit. Projects that implement
or consume Rustolonia COM interfaces must multi-target
`$(AvsCurrentBrowserTargetFramework)` and import `BrowserCom.targets`, so their
registrations and all referenced interface families are available at build time.
Families split across assemblies that are not referenced by the browser host
cannot be combined dynamically.

Removing runtime reflection from this shim does not make browser NativeAOT a
supported .NET target. The current browser host still uses Mono WebAssembly
and requires the .NET SDK and `wasm-tools` to build.

### Limitations

- The page has a single view: windows, the tray icon and other
  desktop-lifetime features are unavailable, and creating a `Window` fails.
- WebAssembly threads are disabled (`WasmEnableThreads=false`), so Rust code
  must not spawn `std::thread`s; use the UI-thread async runtime instead.
- Generated MVVM presentation projects are not yet built for the browser
  target framework; the browser host supports code-first applications.
- The host renders into the element with id `out`
  (`AvnApplication.BrowserMainDivId`); customize `host/Browser/wwwroot`
  to restyle the page or its splash screen.

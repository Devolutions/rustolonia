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
`Avalonia.Host.so` without requiring a process-wide `LD_LIBRARY_PATH`.

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

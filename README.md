# rustolonia

First-class Rust bindings for [Avalonia](https://avaloniaui.net/), projected
over a nano-COM ABI served by a NativeAOT host. This repository contains the
complete bindings effort; the Avalonia framework itself is consumed from the
pinned `avalonia-src` producer submodule at commit
`9654332a79f637473da96054f75b2a16deaa557e`.

## Layout

| Directory | Contents |
|---|---|
| `avalonia-src/` | Avalonia producer pinned to `9654332a79f637473da96054f75b2a16deaa557e`, currently cloned from the `mamoreau-devolutions/Avalonia` fork |
| `avalonia-patches/` | Additive framework patches applied onto the pinned checkout (see its README + UPSTREAM.md) |
| `rust/` | The Rust workspace: `avalonia` (safe bindings), `avalonia-sys` (ABI bindings), `avalonia-bindgen` (IR to Rust generator), `avalonia-sample` (flagship sample's application-owned view-model API), templates, build scripts, and the checked-in IR |
| `host/` | `Avalonia.Host` - the C# NativeAOT host that serves the ABI, plus its generated object model |
| `projection/` | The projection pipeline: IR extraction, C#/header emitters, and the generator tools |
| `interop/` | `Avalonia.Rust` and `Avalonia.Rust.Interop` - the managed-side view-model interop layer |
| `apps/system-monitor/` | NeoHtop, a real Rust-owned system monitor with compiled AXAML presentation and a packaged NativeAOT host |
| `apps/pdf-viewer/` | A PDF Oxide-powered sample viewer with rendered page navigation and extracted text |
| `tests/` | Host, IR, and generator test suites |
| `samples/` | `RustViewModelSample.Managed` - the sample presentation project the host consumes |
| `build/` | Vendored MSBuild configuration (versioning, signing, analyzers, xunit) |

## Getting started

```pwsh
git clone --recurse-submodules https://github.com/mamoreau-devolutions/rustolonia
cd rustolonia
pwsh ./avalonia-patches/apply-avalonia-patches.ps1 -AvaloniaRoot ./avalonia-src
pwsh ./rust/regenerate-and-build.ps1 -Configuration Release
```

The regeneration pipeline (IR, generated C#, native header, Rust bindings) is
deterministic; CI fails if regenerating changes the checkout. Generated outputs
have checked-in ownership manifests so stale cleanup does not erase another
generator's files. Generator CLIs also provide non-mutating `--check` modes.

Open `Rustolonia.slnx` for the managed projects. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the contributor workflow and code-generation
boundaries.

## Creating a new app

```pwsh
pwsh ./rust/new-app.ps1 -Name my_app -Destination ../my_app -ProducerRoot ./avalonia-src -RustoloniaRoot .
pwsh ./rust/build-app.ps1 -ProducerRoot ./avalonia-src -Manifest ../my_app/avalonia-app.json -UpdateLockFile
```

The generated app keeps the producer root and Rustolonia root separate, allows
paths with spaces, and validates the declared roots before writing any files.
References are relative when the checkouts share a volume; the manifest defaults
to the current OS/architecture and `global.json` pins the supported SDK policy.
Commit `Cargo.lock` after the first build and omit `-UpdateLockFile` thereafter.
Normal builds require matching producer revisions/applied patches and initialized
submodules, run Cargo locked, and never format handwritten Rust.

The in-repository system monitor uses the same consumer path without a vendored
checkout:

```pwsh
pwsh ./apps/system-monitor/build.ps1
```

## CI

`.github/workflows/avalonia-rust.yml` keeps the native release and cross-build
gates running automatically on pull requests and pushes to `main`. Native
execution covers Windows/Linux x64 and macOS x64/arm64; Windows/Linux arm64
have cross-build packaging coverage, not native execution coverage. The
original flagship samples remain part of the release gate.
Artifacts are named `avalonia-rust-native-<rid>` for native jobs and
`avalonia-rust-cross-<rid>` for cross-build jobs, so their provenance remains
unambiguous when both jobs target the same RID.

The quick helper/scaffold suite does not build or launch an application:

```pwsh
pwsh ./rust/tests/test-build-app.ps1
```

Native jobs additionally build and launch a fresh external consumer:

```pwsh
pwsh ./rust/tests/test-build-app.ps1 -RunNativeSmoke
```

Linux native smoke execution needs a display, for example
`xvfb-run -a pwsh ./rust/tests/test-build-app.ps1 -RunNativeSmoke`.

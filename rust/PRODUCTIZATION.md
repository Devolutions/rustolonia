# Productizing the Rust application workflow

This documents the reusable, scripted path from "empty directory" to a
packaged Rust Avalonia application: the copyable project template, the
one-command regeneration/build loop, native host discovery, the deterministic
per-runtime-identifier (RID) artifact layout with checksums and an optional
signing hook, source-only crate packaging readiness, and the EU CRA CycloneDX
SBOM scope decision for everything this workflow ships.

Nothing here changes the generated ABI, the ownership contract, or the
view-model transport; see [README.md](README.md), [OWNERSHIP.md](OWNERSHIP.md),
[ASYNC.md](ASYNC.md), [VIEW_MODELS.md](VIEW_MODELS.md),
[DESKTOP_FILES.md](DESKTOP_FILES.md), and [PLATFORMS.md](PLATFORMS.md) for
those. Release compatibility, RID artifact integrity, signing, and future SBOM
requirements are defined in [COMPATIBILITY.md](COMPATIBILITY.md).

## Application template

[`templates/avalonia-app`](templates/avalonia-app) is a minimal, copyable
Cargo project (a `Cargo.toml` with a path dependency on `avalonia`, plus a
`src/main.rs` that opens a window) meant to be copied outside this repository
to bootstrap a new application. `new-app.ps1` performs the copy and
package rename:

```powershell
.\rust\new-app.ps1 -Name my_app -Destination .\my_app
```

```bash
pwsh ./rust/new-app.ps1 -Name my_app -Destination ~/src/my_app
```

The template is excluded from the `rust` Cargo workspace (see the `exclude`
entry in `Cargo.toml`) and declares an empty workspace of its own. The template is an external consumer, including a managed AXAML project,
versioned view-model IR, and `avalonia-app.json`. It also carries copyable
Windows/Linux/macOS file type association metadata in `file-associations/`, and
a `main.rs` that already surfaces startup "open with" documents through
`AppScope::activation_items()` (see
[DESKTOP_FILES.md](DESKTOP_FILES.md#file-type-associations)). `new-app`
substitutes the Cargo package name and separate Rustolonia/producer roots,
including those snippets. Cargo paths are consumer-relative and managed roots
are anchored to the presentation project directory. Moving the entire layout
preserves these references; roots on different volumes retain absolute paths
with a warning. Explicit MSBuild root overrides remain supported; update Cargo
paths too when deliberately changing the layout.
The scaffold copies Rustolonia's supported `global.json` policy and defaults the
manifest RID/output directory to the current OS/architecture (`-Rid` overrides
this). Commit Rustolonia and the producer as a
submodule (or pin a checkout to an immutable commit); do not point a release
consumer at an unpinned branch.
Initialize the pinned producer recursively before generation or publishing:

```bash
git submodule update --init --recursive
```

This is required because the producer build consumes its XamlX and platform
submodules. The vendored template declares an empty `[workspace]`, so it does
not become an accidental member of an enclosing consumer Cargo workspace.
`regenerate-and-build.ps1 -ValidateTemplate` /
`pwsh ./regenerate-and-build.ps1 -ValidateTemplate` copies it through `new-app`
before running `cargo check`.

## External consumer build and package

`consumer-app-manifest.schema.json` defines manifest version 1. The required
fields name the consumer presentation `.csproj`, IR, generated adapter,
registry, Rust and contract outputs, Cargo manifest/package, RID,
configuration, and output directory; `binary` optionally selects a normal
Cargo binary and defaults to `packageName`.

```powershell
pwsh .\rustolonia\rust\build-app.ps1 -ProducerRoot .\rustolonia\avalonia-src `
  -Manifest .\consumer\avalonia-app.json -UpdateLockFile
```

The cross-platform `build-app.ps1` (PowerShell 7) validates the
manifest and paths, then checks the producer HEAD against `release-manifest.json`,
patch file hashes and applied patch content, and recursive submodule revisions.
Apply patches explicitly with `avalonia-patches/apply-avalonia-patches.ps1`;
preflight never modifies or downloads source/tool prerequisites.
It requires the supported .NET SDK (respecting `global.json` feature-band roll
forward), Cargo/Rust/rustup, the RID's installed Rust target and native tools:
Visual Studio C++ plus Windows SDK on Windows, clang/cc/binutils/pkg-config and
zlib development files on Linux, or Xcode on macOS. Packaging remains same-OS,
with cross-architecture builds requiring their target toolchains.

On first build, `-UpdateLockFile` explicitly permits Cargo metadata resolution
to create/update the workspace `Cargo.lock`. Commit that lockfile and
`global.json`; omit the switch for subsequent builds. Metadata resolution and
the actual Cargo build are then both locked, so missing/stale lockfiles fail
before generation. Compatible existing pins are retained during resolution;
upgrading dependencies is a separate deliberate Cargo operation.

After preflight it runs Rustolonia's
`Avalonia.ViewModelProjection.Tool` against consumer outputs. It then runs
the consumer AXAML build, builds the declared Cargo `--bin --locked`, and
publishes `Avalonia.Host` with
`AvaloniaRustPresentationProjects` and `AvaloniaRustViewRegistryFile`.
The external ProjectReference and linked generated registry are therefore
compiled statically into NativeAOT; no application-specific ABI is introduced.
Normal consumer builds never run `cargo fmt` or rewrite handwritten Rust.
External Rust output uses a crate-root compatibility bridge. Consumers must
re-export `DynamicViewModel`, `ViewModelSink`, `ViewModelBatch`, and
`BatchCompletion` from `avalonia::view_model`; the shipped template already
does so.
Finally it writes the host, published native DLLs/shared libraries, consumer
executable, the producer's `licence.md`, Rustolonia's `LICENSE` and
`THIRD-PARTY-NOTICES.txt`, deterministic CycloneDX delivery inventory, and SHA-256
checksums to the manifest's adjacent output directory. A local
`AVALONIA_RUST_SIGN_COMMAND` wrapper may sign final binaries before SBOM and
checksums; it is never downloaded or shell-expanded.
The optional `noticeFiles` manifest array adds application-specific license or
notice files to that bundle before its SBOM and checksums are generated. Entries
must be relative files contained beneath the manifest directory, cannot traverse
symbolic links or reparse points, and must have case-insensitively distinct,
non-reserved basenames.

Both packaging entrypoints share `package-shared.ps1`: target mappings, native
preparation, command construction, native-library copying, signing and checksums.
Consumer publication uses a unique temporary staging directory, and cleans only
that invocation's directory. A `.rustolonia-bundle-owner` marker identifies
disposable output bundles. A nonempty directory without the valid marker is
rejected rather than erased; choose a new output directory for existing bundles
created before this ownership mechanism.

Windows MSVC Rust binaries are built with `target-feature=+crt-static`, so the
adjacent bundle does not require a separately installed Visual C++ runtime.
The external template carries the same target-specific Cargo configuration;
`build-app` also enforces it for existing consumer workspaces.

## One-command developer workflow

`regenerate-and-build.ps1` replaces the four
previously separate, manually copy-pasted commands from README.md's
"Regenerate bindings" section with one:

1. Regenerate the object-model projection IR, generated C# COM sources, and
   the native ABI header from the current `AvaloniaObject`/`Control`
   assemblies (`Avalonia.Projection.Tool`).
2. Regenerate the Rust `avalonia-sys`/`avalonia` bindings from that IR
   (`avalonia-bindgen`), then `cargo fmt --all`.
3. Regenerate the managed adapters, application view registry, Rust
   view-model API, and `view-model.contract.md` from the sample-owned
   `rust/avalonia-sample/view-model.ir.json` (`Avalonia.ViewModelProjection.Tool`
   with `--external-rust`).
4. Build the code-first host (no sample presentation), then the
   sample-composed host (`AvaloniaRustPresentationProjects` +
   `AvaloniaRustViewRegistryFile`), then the Rust workspace
   (`cargo build --workspace`).

```powershell
.\rust\regenerate-and-build.ps1
```

```bash
pwsh ./rust/regenerate-and-build.ps1
```

Both scripts are idempotent when the IR hasn't changed (running them against
an unmodified `projection.ir.json`/`view-model.ir.json` reproduces byte-identical
generated output) and fail fast: each step's exit code stops the script before
the next one runs.

## Tabular presentation

Schema version 3 table metadata is generated with the normal consumer
presentation sources. Consumers use built-in `TableView` columns and compiled
AXAML cell bindings; no DataGrid package, runtime binding reflection, or
application-specific host code is required. A snapshot-backed model still has
one managed row adapter per delivered row, while `TableView` virtualizes visual
rows. Schema-v4 `window` metadata provides range-backed collections: the
projection reports the whole Rust dataset size while keeping live element
objects bounded by `pageSize * maxLivePages`, and both shapes ship in the
sample so the difference is measurable rather than asserted.

Useful switches (the same PowerShell arguments work on Windows, Linux, and macOS):

| Switch | Effect |
| --- | --- |
| `-Configuration Debug` | Build configuration for the .NET regeneration tools and managed build (default `Release`). |
| `-SkipManagedBuild` | Skip the code-first and sample-composed `dotnet build` calls; generation and the Rust workspace build still run. |
| `-Test` | Run `cargo test --workspace` instead of `cargo build --workspace`. Requires a host discoverable per [Host discovery](#host-discovery) below (`rust/build.ps1` publishes one; set `AVN_HOST_NATIVE_LIB` otherwise). |
| `-ValidateTemplate` | Scaffold a temporary external consumer, generate its view-model sources, run `cargo check`, and remove it on success. |
| `-PackageRid <rid>` | Additionally run [`package.ps1`](#deterministic-per-rid-artifact-layout) for that RID. |

For example, from the repository root:

```bash
pwsh ./rust/regenerate-and-build.ps1 -SkipManagedBuild -ValidateTemplate
```

This script intentionally does not replace `rust/build.ps1`
(full RID publish plus `cargo test --workspace` against the exact published
host) or `package.ps1` (below): it is the fast inner
regenerate/compile loop, and delegates to those for anything that needs a
real NativeAOT publish.

## Host discovery

`avalonia::App::load_from_env()` (used by every example and the template)
resolves the native `Avalonia.Host` library through `avalonia::discover_host_path()`:

1. **`AVN_HOST_NATIVE_LIB`** (the `avalonia::HOST_NATIVE_LIB_ENV_VAR` constant) -- an explicit
   override. If set, its value is used as-is, even if nothing exists at that
   path yet, so `Host::load` can surface a precise loader error instead of
   this function silently falling back to the next mechanism. This remains
   how `rust/build.ps1` point the workspace test suite at a
   freshly published host, and how you point a running app at a different
   host during development.
2. **Adjacent to the executable** -- otherwise, the platform host file name
   (`Avalonia.Host.dll` on Windows, `Avalonia.Host.so` on Linux,
   `Avalonia.Host.dylib` on macOS) is looked up next to
   `std::env::current_exe()`. This is what lets a packaged application run
   with no environment variable at all: [`package.ps1`](#deterministic-per-rid-artifact-layout)
   copy the host and the application binary into the same directory.

If neither resolves, the error names both the environment variable and the
host file name it looked for next to the executable's directory. See
`rust/avalonia/src/runtime.rs` (`discover_host_path`, `HOST_NATIVE_LIB_ENV_VAR`)
for the implementation, `rust/avalonia/src/runtime.rs`'s
`host_discovery_tests` module for unit tests of the override/adjacent-lookup
precedence and error message, and `rust/avalonia/tests/host_discovery.rs` for
the same behavior exercised through the crate's public API.

## Deterministic per-RID artifact layout

`package.ps1` publish the NativeAOT host for
one RID, build a Rust binary next to it, and lay both out identically
regardless of platform, under `rust/artifacts/<rid>/`:

```powershell
.\rust\package.ps1 -Rid win-x64
```

```bash
pwsh ./rust/package.ps1 linux-x64
```

```bash
pwsh ./rust/package.ps1 osx-arm64
```

Both produce, for every supported RID:

- `Avalonia.Host.<dll|so|dylib>` -- the published NativeAOT host.
- Its native rendering dependencies that publish alongside it
  (`libSkiaSharp`/`libHarfBuzzSharp`, plus `libAvaloniaNative.dylib` on
  macOS), copied only if present. `package.ps1` copies every published `.so`,
  versioned `.so.*`, and `.dylib` dependency rather than maintaining a
  platform-specific allow-list.
- The requested Rust binary (`hello_world` by default; pass `-Example`/an
  extra argument to package a different example, or point `-OutputRoot`
  /`AVN_PACKAGE_OUTPUT` and build the copied application template the same
  way), placed **next to** the host so [host discovery](#host-discovery)
  finds it with no environment variable.
- `checksums.sha256` -- a `sha256sum -c`-compatible SHA-256 manifest of every
  other file in the directory, generated last so it never hashes itself.
- `licence.md`, `LICENSE`, `THIRD-PARTY-NOTICES.txt` -- producer and project
  licensing notices copied into every delivery bundle.
- `.rustolonia-bundle-owner` -- the marker authorizing replacement of a previously
  generated bundle; it is included in the delivery inventory and checksums.
- `sbom.cdx.json` -- a deterministic CycloneDX 1.5 delivery inventory,
  generated after optional signing and before checksums. It records SHA-256
  hashes for the host, Rust executable, bundled native libraries, and licence;
  it intentionally excludes itself and the checksum manifest to avoid a
  recursive hash. It also records the resolved third-party dependency graph:
  NuGet packages from the host's already-restored `project.assets.json` (no
  network access; `type: "project"` entries such as in-repo project
  references are excluded) and Cargo crates from `Cargo.lock` (workspace-local
  crates with no `[source]`, like `avalonia`, are excluded as not third-party).
  Each resolved dependency is a CycloneDX `library` component with a `purl`
  (`pkg:nuget/...`/`pkg:cargo/...`). `metadata.properties` records the producer
  git pin used for the build and, when a dependency source path could not be
  supplied, an explicit note that dependency data for that ecosystem is
  unavailable rather than silently omitting it. A supplied path that does not
  exist is also recorded as unavailable. Both packaging entrypoints query
  MSBuild's `ProjectAssetsFile` with the publish configuration, RID, and output
  overrides rather than assuming `host/obj`; missing publish restore metadata
  fails packaging. This is still a delivery
  inventory of resolved packages and their identities, not a NVD/OSV
  vulnerability scan or license-compatibility check.

The layout is deterministic: the same RID with the same configuration and
example always produces the same file set at the same relative paths, which
is what makes the adjacent-host discovery above reliable and what a release
pipeline can zip and publish unchanged.

Each supported RID has an explicit native Cargo target; package scripts reject
an absent target with the corresponding `rustup target add` command:

| RID | Cargo target |
| --- | --- |
| `win-x64` | `x86_64-pc-windows-msvc` |
| `win-arm64` | `aarch64-pc-windows-msvc` |
| `linux-x64` | `x86_64-unknown-linux-gnu` |
| `linux-arm64` | `aarch64-unknown-linux-gnu` |
| `osx-x64` | `x86_64-apple-darwin` |
| `osx-arm64` | `aarch64-apple-darwin` |

Packaging supports same-OS cross-architecture builds when the required native
and Rust toolchains are available. Executing a packaged application requires a
matching native runner. Cross-build artifacts are therefore not evidence of
native runtime coverage.

### Signing hook

Neither script downloads or bundles a signing tool -- that would be exactly
the kind of unverified-binary shortcut this workflow avoids. Instead, before
the SBOM and checksums are computed, both scripts check for
`AVALONIA_RUST_SIGN_COMMAND`. If set, it must be the path to a trusted local
wrapper executable (or executable script); the script invokes it once per
binary artifact with that artifact's path as a separate argument. The wrapper
owns all signer options and identity selection. No shell evaluation or command
template expansion is performed. If unset, signing is skipped with a message
explaining how to opt in; either way, the SBOM and checksums describe final
(optionally signed) bytes.

The entrypoints explicitly identify the application executable, including
extensionless Linux/macOS binaries. Licensing files and ownership markers are
not signing inputs. Missing explicitly requested signing inputs fail the build.

## Source-only crate packaging

`avalonia`, `avalonia-sys`, and `avalonia-bindgen` all set `publish = false`:
they are pinned to a matching `Avalonia.Host` build from the same checkout,
not to a versioned ABI contract suitable for crates.io. That is a deliberate
choice, not a gap -- but the crates are kept in a state where `cargo package`
would succeed if that ever changed: each has `description`, `license`,
`repository`, and (for the two application-facing crates) a `readme`
pointing at a real `README.md`. This is checked with:

```bash
cargo package --list -p avalonia-sys --allow-dirty
cargo package --list -p avalonia --allow-dirty
cargo package --list -p avalonia-bindgen --allow-dirty
```

## SBOM (EU CRA) scope

Rustolonia does not carry the upstream Avalonia producer's NUKE build or its
`SbomGenerator.cs`/Numerge infrastructure; none of that exists in this
repository. Rustolonia is not a NuGet package producer at all: every managed
project in this repository (`Avalonia.Host`, `Avalonia.Rust`,
`Avalonia.Rust.Interop`, `Avalonia.Projection.*`,
`Avalonia.ViewModelProjection.Tool`) is `IsPackable=false`, and the `rust/*`
crates are `publish = false` (see [Source-only crate
packaging](#source-only-crate-packaging) above). There is no shipped `.nupkg`
or published crate for a package-level SBOM generator to cover.

What Rustolonia does ship is the packaged NativeAOT bundle produced by
[`package.ps1`](#deterministic-per-rid-artifact-layout) or `build-app.ps1`
for an external consumer, and that delivery is what
[`sbom.cdx.json`](#deterministic-per-rid-artifact-layout) inventories: every
delivered file's SHA-256 hash, plus the resolved third-party NuGet package
graph (from the host's already-restored `project.assets.json`) and Cargo
crate graph (from `Cargo.lock`), each recorded as a CycloneDX component with
a `purl`. This is a delivery-content and resolved-dependency-identity record,
not a NuGet-package-level SBOM generator and not a vulnerability or
license-compatibility scan; treat it as the inventory an EU CRA delivery
process consumes, not as the whole of that process.

## Tests

- `pwsh ./rust/tests/test-build-app.ps1` runs quick script-parser, helper,
  path/ownership, signing, manifest and scaffold regressions without a native
  build. All fixtures are created in an owned temporary directory.
- Add `-RunNativeSmoke` to build, package and launch a fresh external consumer
  for the current OS/architecture. On Linux, run under a display such as
  `xvfb-run -a pwsh ./rust/tests/test-build-app.ps1 -RunNativeSmoke`. Smoke builds
  do not invoke a user-configured signing service and clear the host override
  when launching so that the adjacent packaged host is exercised.
- `rust/avalonia/src/runtime.rs` (`host_discovery_tests` module) and
  `rust/avalonia/tests/host_discovery.rs` cover `discover_host_path`: the
  explicit override always winning (even to a nonexistent path), the
  adjacent-file lookup succeeding and failing, and the combined error naming
  both mechanisms -- all without requiring a published host.
- `pwsh ./rust/regenerate-and-build.ps1 -ValidateTemplate` scaffolds a temporary
  external consumer and generates its sources before compile-checking it as a
  standalone crate. The checked-in template contains placeholders and should
  not be compiled directly.
- `cargo package --list` (see [Source-only crate packaging](#source-only-crate-packaging))
  is the packaging-readiness check for all three workspace crates.
- `package.ps1` self-verify their own output shape (the
  publish step fails the script if the host file is missing, the cargo build
  step fails it if the binary is missing) and were run end to end for
  `win-x64` while developing this stage, confirming the host, its native
  dependencies, the Rust binary, and a matching `checksums.sha256` land
  together in one deterministic directory.
- Desktop file integration is covered end to end from the managed picker core
  through the raw nano-COM vtables to the safe Rust API; the full table is in
  [DESKTOP_FILES.md](DESKTOP_FILES.md#tests).

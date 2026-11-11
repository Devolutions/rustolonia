# Standalone app templates

Each template is an independent Cargo workspace. These examples use APIs from
the current Rustolonia source revision; the published release tags do not yet
contain those APIs.

| Template | Starting point | Structure |
| --- | --- | --- |
| [minimal](minimal/README.md) | One window, `rsx!`, `StyleSheet`, `color!`, `fmt!`, and a reactive `Signal` counter | `main.rs` bootstrap, `ui.rs` composition |
| [form](form/README.md) | Two-way `TextBox`, `#[derive(Lens)]`, `#[component]`, validation, Submit and Clear actions | `main.rs`, `ui.rs`, UI-independent `model.rs` |
| [multi-window](multi-window/README.md) | Main workspace opening independent `rsx!` text-preview windows | `main.rs`, `ui.rs`, `ui/preview.rs` |
| [axaml](axaml/README.md) | `.axaml` compiled into Rust builders during Cargo builds (development) | `build.rs`, AXAML views, Rust components and signals |

The manifests pin the latest published **v12.1.6** as a lockfile seed. That
release predates these examples, so building them requires a local source
override and a host built from the same revision. Follow the development
setup below before building; do not mix bindings and hosts with different ABI
fingerprints.

## Development setup

In the copied app, append this patch to `.cargo/config.toml`, replacing the
paths with the absolute paths to the matching checkout:

```toml
[patch."https://github.com/Devolutions/rustolonia"]
rustolonia = { path = "/path/to/rustolonia/rust/rustolonia" }
rustolonia-macros = { path = "/path/to/rustolonia/rust/rustolonia-macros" }
rustolonia-bindgen = { path = "/path/to/rustolonia/rust/rustolonia-bindgen" }
rustolonia-sys = { path = "/path/to/rustolonia/rust/rustolonia-sys" }
```

Update the lockfile and set the matching NativeAOT host before running:

```sh
cargo update -p rustolonia -p rustolonia-macros -p rustolonia-bindgen -p rustolonia-sys
export RUSTOLONIA_HOST_DIR=/path/to/matching-host
cargo run
```

The host must be built from the same source revision as the Rust crates. To
type-check without a host, use `RUSTOLONIA_NO_DOWNLOAD=1 cargo check`.

## Copy and run

Install Rust 1.88 or newer and the native linker toolchain for your platform.
The Rust templates do not need an initialized Avalonia producer checkout. The
matching NativeAOT host must be built separately from the same source revision;
building it requires the .NET SDK.

From the repository root, choose a code-first template:

```powershell
$template = 'minimal'
$destination = Join-Path $HOME 'my-rustolonia-app'
if (Test-Path -LiteralPath $destination) { throw "Destination already exists: $destination" }
New-Item -ItemType Directory -Path $destination | Out-Null
Get-ChildItem -LiteralPath (Join-Path .\templates $template) -Force |
    Where-Object Name -ne 'target' |
    Copy-Item -Destination $destination -Recurse
Set-Location $destination
cargo run
```

On macOS/Linux, copying the directory with `cp -R` preserves its dotfiles;
remove any copied `target` directory before starting your own project.
Keep `.cargo/config.toml`, `.gitignore`, and `Cargo.lock` alongside the sources.
Complete the development setup above before running Cargo.

The AXAML template compiles its supported markup subset as part of Cargo's
build step; it needs no .NET SDK, Avalonia checkout, or separate generator
installation. See its [README](axaml/README.md) for the source override, host
setup, subset, and limitations.

You can also try a template in place:

```powershell
cd templates\minimal
cargo run
```

Run Cargo from the template directory, not from the repository root using
`--manifest-path`: Cargo discovers configuration from its working directory.
Each template overrides the source checkout's host-download opt-out.
Explicit environment variables still win; unset `RUSTOLONIA_NO_DOWNLOAD`,
`RUSTOLONIA_HOST_DIR`, `RUSTOLONIA_HOST_LIB`, `RUSTOLONIA_HOST_BASE_URL`, and
`RUSTOLONIA_HOST_CHECKSUMS` if you previously selected a local host or mirror.

With a matching source override, `RUSTOLONIA_NO_DOWNLOAD=1 cargo check` can
type-check a template without a host. Running it requires the matching host
directory described in the development setup.

## Customize

Rename the package and update its description in `Cargo.toml`, then change
the window title and UI in `src/ui.rs` (code-first) or `src/MainWindow.axaml`
(AXAML). `main.rs` bootstraps the runtime and wires Rust state to the view.
Keep business logic in separate modules, as illustrated by `form/src/model.rs`.
Register UI subscriptions through `AppScope` to retain them for the app lifetime.
Startup errors propagate from `main`; callbacks report runtime failures to stderr.

After renaming the package, run `cargo build` to refresh `Cargo.lock`, then
commit it in your new app's repository. Normal builds should use `--locked`.

```powershell
cargo fmt
cargo clippy --locked -- -D warnings
cargo build --release --locked
```

To update Rustolonia, consult
[the latest release](https://github.com/Devolutions/rustolonia/releases/latest),
change the exact `tag` in `Cargo.toml`, run `cargo update -p rustolonia`, and
commit the manifest and lockfile together. Do not move an existing release tag
or mix bindings with a host from a different version.
Once a release includes the AXAML changes, update both `rustolonia` and
`rustolonia-bindgen` in that template to that release and refresh its lockfile.

## Platforms and shipping

Release hosts support Windows, macOS, and Linux (glibc), on x64 and arm64.
Linux needs X11/XWayland and fontconfig. See the
[platform requirements](https://github.com/Devolutions/rustolonia/blob/v12.1.6/rust/PLATFORMS.md#prebuilt-host-platform-floors).
Build on the target platform with its native Rust toolchain.

These are development skeletons, not installer projects. A release-profile
build still embeds the development cache location by default and is **not**
portable on its own. For distribution, disable the dependency's default
features, rebuild, and ship the matching extracted release host, native
dependencies, and notices beside the executable (or in `../Frameworks` inside
a macOS bundle). Follow the
[host distribution guide](https://github.com/Devolutions/rustolonia/blob/v12.1.6/rust/rustolonia/README.md).

For full Avalonia-compiled AXAML and generated managed view-model adapters, use
the separate
[source-build scaffold](../rust/templates/avalonia-app/README.md) through
`rust/new-app.ps1`. It intentionally remains under `rust/templates/`: its
placeholders and producer-checkout requirements are different from these
copy-out release consumers.

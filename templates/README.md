# Standalone app templates

Each template is an independent Cargo workspace. The code-first templates use
published Rustolonia bindings and a prebuilt NativeAOT host. The AXAML template
currently uses this Rustolonia source revision with a matching prebuilt host.

| Template | Starting point | Structure |
| --- | --- | --- |
| [minimal](minimal/README.md) | One window, a label, and a button callback | `main.rs` bootstrap, `ui.rs` composition |
| [form](form/README.md) | Text input, validation, Submit and Clear actions | `main.rs`, `ui.rs`, UI-independent `model.rs` |
| [multi-window](multi-window/README.md) | Main workspace opening independent text-preview windows | `main.rs`, `ui.rs`, `ui/preview.rs` |
| [axaml](axaml/README.md) | `.axaml` compiled into Rust builders during Cargo builds (development) | `build.rs`, AXAML views, Rust components and signals |

The three code-first templates pin **v12.1.0**. The AXAML template's
**v12.1.5** tag predates its compiler and runtime additions, so it requires the
local source override described in its README until those additions are
released. Its projection catalog is generated from the same metadata as the
host. Do not mix bindings and hosts with different ABI fingerprints.

## Copy and run

Install Rust 1.88 or newer and the native linker toolchain for your platform.
No .NET SDK or initialized Avalonia producer checkout is needed. Cargo may
fetch the tagged source's submodules, but the templates never build them.

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
cargo run --locked
```

On macOS/Linux, copying the directory with `cp -R` preserves its dotfiles;
remove any copied `target` directory before starting your own project.
Keep `.cargo/config.toml`, `.gitignore`, and `Cargo.lock` alongside the sources.
No sibling template or repository file is required by a copied code-first app.

The AXAML template compiles its supported markup subset as part of Cargo's
build step; it needs no .NET SDK, Avalonia checkout, or separate generator
installation. Until released, it also needs a matching prebuilt development
host. See its [README](axaml/README.md) for the source override, host setup,
subset, and limitations.

You can also try a template in place:

```powershell
cd templates\minimal
cargo run --locked
```

Run Cargo from the template directory, not from the repository root using
`--manifest-path`: Cargo discovers configuration from its working directory.
Each template overrides the source checkout's host-download opt-out.
Explicit environment variables still win; unset `RUSTOLONIA_NO_DOWNLOAD`,
`RUSTOLONIA_HOST_DIR`, `RUSTOLONIA_HOST_LIB`, `RUSTOLONIA_HOST_BASE_URL`, and
`RUSTOLONIA_HOST_CHECKSUMS` if you previously selected a local host or mirror.

For the released code-first templates, the first build downloads the release
host for your platform, checks its SHA-256 against the checksum embedded in the
tagged bindings, and caches it. The default `dev-host-path` feature finds that
cache when the app starts. The AXAML development template instead requires
the matching local host described in its README.

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
[platform requirements](https://github.com/Devolutions/rustolonia/blob/v12.1.0/rust/PLATFORMS.md#prebuilt-host-platform-floors).
Build on the target platform with its native Rust toolchain.

These are development skeletons, not installer projects. A release-profile
build still embeds the development cache location by default and is **not**
portable on its own. For distribution, disable the dependency's default
features, rebuild, and ship the matching extracted release host, native
dependencies, and notices beside the executable (or in `../Frameworks` inside
a macOS bundle). Follow the
[host distribution guide](https://github.com/Devolutions/rustolonia/blob/v12.1.0/rust/rustolonia/README.md).

For full Avalonia-compiled AXAML and generated managed view-model adapters, use
the separate
[source-build scaffold](../rust/templates/avalonia-app/README.md) through
`rust/new-app.ps1`. It intentionally remains under `rust/templates/`: its
placeholders and producer-checkout requirements are different from these
copy-out release consumers.

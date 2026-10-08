# Minimal desktop app

A standalone, code-first starting point: one window, a greeting, and a
**Say hello** button. Uses the published Rustolonia **v12.1.0** bindings and
prebuilt host. No .NET SDK, local checkout dependency, or generation required.

## Start

Copy this whole directory anywhere, including `.cargo`, `.gitignore`, and
`Cargo.lock`, but excluding `target`. Install Rust 1.88 or newer and the native
linker toolchain for your platform, then run from your copied directory:

```powershell
cargo run --locked
```

The first build fetches the tagged Rust bindings and downloads the matching
host from GitHub, verifies its SHA-256, and caches it. Cargo may fetch the
tagged repository's submodules but does not build them.

Run Cargo from this directory so `.cargo/config.toml` enables release host
downloads even inside the original repository. If you previously configured
a local host or mirror, unset `RUSTOLONIA_NO_DOWNLOAD`, `RUSTOLONIA_HOST_DIR`,
`RUSTOLONIA_HOST_LIB`, `RUSTOLONIA_HOST_BASE_URL`, and
`RUSTOLONIA_HOST_CHECKSUMS`.

## Extend

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Load the native runtime and start the application |
| `src/ui.rs` | Compose the window and register its button callback |
| `Cargo.toml` | Your package metadata and pinned release dependency |
| `Cargo.lock` | Reproducible dependency graph; keep in version control |

Change the package name and description, window title, and greeting. Run
`cargo build` once after renaming the package to refresh the lockfile.
Callbacks use `AppScope` to keep their subscriptions alive, and report UI
errors to stderr. Startup failures propagate out of `main`.

```powershell
cargo fmt
cargo clippy --locked -- -D warnings
cargo build --release --locked
```

To upgrade, change the exact dependency `tag` to a newer
[published release](https://github.com/Devolutions/rustolonia/releases/latest),
run `cargo update -p rustolonia`, and commit both manifest and lockfile.
Avoid moving branch dependencies: bindings and host versions must match.

## Ship

Supports Windows/macOS/Linux glibc, x64 and arm64. Linux requires X11/XWayland
and fontconfig; see the
[platform requirements](https://github.com/Devolutions/rustolonia/blob/v12.1.0/rust/PLATFORMS.md#prebuilt-host-platform-floors).

The default development feature loads the host from its build-time cache,
even in release-profile builds. For a portable app, set
`default-features = false` on the dependency, rebuild, and ship the extracted
matching host, native dependencies, and notices beside the executable (or
`../Frameworks` in a macOS bundle). See the
[distribution guide](https://github.com/Devolutions/rustolonia/blob/v12.1.0/rust/rustolonia/README.md).
This template is code-first; compiled AXAML requires a source-built host.

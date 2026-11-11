# Minimal desktop app

A code-first starting point: one window, a greeting, and a **Say hello**
button. It uses pre-release APIs from the current Rustolonia source revision;
the published **v12.1.6** dependency tag does not contain those APIs.

## Start

Copy this whole directory anywhere, including `.cargo`, `.gitignore`, and
`Cargo.lock`, but excluding `target`. Before building, complete the
[template development setup](../README.md#development-setup). Then install
Rust 1.88 or newer and the native linker toolchain, and run:

```powershell
cargo run
```

The pinned release tag is only a dependency seed; the local source override and
host must be from the same revision. Set `RUSTOLONIA_HOST_DIR` to the matching
host directory before running. Run Cargo from this directory so its
`.cargo/config.toml` is loaded.

## Extend

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Load the native runtime and start the application |
| `src/ui.rs` | Compose the window and register its button callback |
| `Cargo.toml` | Your package metadata and pinned dependency seed |
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
[platform requirements](https://github.com/Devolutions/rustolonia/blob/v12.1.6/rust/PLATFORMS.md#prebuilt-host-platform-floors).

The default development feature loads the host from its build-time cache,
even in release-profile builds. For a portable app, set
`default-features = false` on the dependency, rebuild, and ship the extracted
matching host, native dependencies, and notices beside the executable (or
`../Frameworks` in a macOS bundle). See the
[distribution guide](https://github.com/Devolutions/rustolonia/blob/v12.1.6/rust/rustolonia/README.md).
This template is code-first; compiled AXAML requires a source-built host.

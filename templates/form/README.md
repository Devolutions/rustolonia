# Input form app

A standalone form skeleton with a name field, **Submit** and **Clear**
actions, and inline validation. It uses the published Rustolonia **v12.1.0**
bindings and prebuilt host, without the .NET SDK or local source dependencies.

## Start

Copy this entire directory outside the repository, including `.cargo`,
`.gitignore`, and `Cargo.lock`, but excluding `target`. Install Rust 1.88 or
newer and your platform's native linker toolchain. From the copied directory:

```powershell
cargo run --locked
```

The first build fetches the tagged bindings, downloads the matching host from
GitHub, verifies its SHA-256, and caches it. Tagged source submodules may be
fetched by Cargo but are not built. Run from this directory for Cargo to load
the included release-download configuration.
If you configured a local host or mirror, unset `RUSTOLONIA_NO_DOWNLOAD`,
`RUSTOLONIA_HOST_DIR`, `RUSTOLONIA_HOST_LIB`, `RUSTOLONIA_HOST_BASE_URL`, and
`RUSTOLONIA_HOST_CHECKSUMS` first.

## Extend

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Runtime bootstrap |
| `src/ui.rs` | Input controls, callbacks, status display |
| `src/model.rs` | UI-independent validation and greeting logic |

Submit trims surrounding whitespace and shows a validation message for empty
input; Clear resets both input and status. Keep application rules in
`model.rs` or additional domain modules rather than mixing them into control
composition. Callbacks are retained by `AppScope`. Startup failures propagate
from `main`, while runtime UI failures are reported to stderr.

Rename the package in `Cargo.toml` and change the window title in `src/ui.rs`.
Run `cargo build` once after renaming to refresh `Cargo.lock`, and commit it.

```powershell
cargo fmt
cargo clippy --locked -- -D warnings
cargo build --release --locked
```

For upgrades, select an exact
[published release tag](https://github.com/Devolutions/rustolonia/releases/latest),
edit `Cargo.toml`, run `cargo update -p rustolonia`, and commit the manifest
and lockfile. Do not track `master` or mix host and bindings versions.

## Ship

Supported release platforms are Windows/macOS/Linux glibc on x64 and arm64.
Linux needs X11/XWayland and fontconfig; see the
[platform requirements](https://github.com/Devolutions/rustolonia/blob/v12.1.0/rust/PLATFORMS.md#prebuilt-host-platform-floors).

The development host-cache feature is enabled even for release builds.
To distribute, set `default-features = false` on the Rustolonia dependency,
rebuild, and copy the matching release host, native dependencies, and notices
beside the executable (or `../Frameworks` in a macOS bundle). Follow the
[distribution guide](https://github.com/Devolutions/rustolonia/blob/v12.1.0/rust/rustolonia/README.md).
Compiled AXAML is not part of this code-first template.

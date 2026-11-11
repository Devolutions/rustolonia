# Input form app

A form skeleton with a name field, **Submit** and **Clear** actions, and inline
validation. It uses pre-release APIs from the current Rustolonia source
revision; the published **v12.1.6** dependency tag does not contain those APIs.

## Start

Copy this entire directory outside the repository, including `.cargo`,
`.gitignore`, and `Cargo.lock`, but excluding `target`. Before building,
complete the source override and matching-host steps in the
[template development setup](../README.md#development-setup). Install Rust
1.88 or newer and your platform's native linker toolchain, then run:

```powershell
cargo run
```

The pinned release tag is only a dependency seed; the local source override and
host must be from the same revision. Set `RUSTOLONIA_HOST_DIR` to the matching
host directory before running. Run Cargo from this directory for its
`.cargo/config.toml` to be loaded.

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
[platform requirements](https://github.com/Devolutions/rustolonia/blob/v12.1.6/rust/PLATFORMS.md#prebuilt-host-platform-floors).

The development host-cache feature is enabled even for release builds.
To distribute, set `default-features = false` on the Rustolonia dependency,
rebuild, and copy the matching release host, native dependencies, and notices
beside the executable (or `../Frameworks` in a macOS bundle). Follow the
[distribution guide](https://github.com/Devolutions/rustolonia/blob/v12.1.6/rust/rustolonia/README.md).
Compiled AXAML is not part of this code-first template.

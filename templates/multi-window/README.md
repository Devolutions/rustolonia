# Multi-window app

A standalone workspace skeleton that opens text-preview windows. Uses the
published Rustolonia **v12.1.0** bindings and prebuilt host: no .NET SDK,
local source dependency, or AXAML compilation.

## Start

Copy this whole directory elsewhere, including `.cargo`, `.gitignore`, and
`Cargo.lock`, but excluding `target`. Install Rust 1.88 or newer and your
platform's native linker toolchain, then run from the copied directory:

```powershell
cargo run --locked
```

The first build fetches the release bindings and downloads the matching
host, verifies its SHA-256, and caches it. Cargo may fetch tagged submodules
but never builds them. Always invoke Cargo from this directory to load the
included release-download configuration. Unset `RUSTOLONIA_NO_DOWNLOAD`,
`RUSTOLONIA_HOST_DIR`, `RUSTOLONIA_HOST_LIB`, `RUSTOLONIA_HOST_BASE_URL`, and
`RUSTOLONIA_HOST_CHECKSUMS` if you previously selected a local host or mirror.

## Extend

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Runtime bootstrap |
| `src/ui.rs` | Main window, input, open action, and status |
| `src/ui/preview.rs` | Compose and mount each secondary window |

Each **Open preview** action opens a new window with a snapshot of the
current input. Later edits do not change existing previews. Closing a preview
leaves the other windows running. The application exits when the last window
closes, so previews also remain open if you close the main window first.
The previews are independent windows, not modal dialogs.

Use this structure to add document windows or tool palettes. Mount windows
through `AppScope`; register subscriptions through the same scope so they stay
alive. Startup errors propagate from `main`; callback failures are logged to
stderr and preview failures also appear in the main window's status.

Change the package name and description in `Cargo.toml`, and window titles
in the UI modules. Run `cargo build` after renaming to refresh `Cargo.lock`,
then commit the lockfile.

```powershell
cargo fmt
cargo clippy --locked -- -D warnings
cargo build --release --locked
```

To upgrade, change the dependency to a newer exact
[published tag](https://github.com/Devolutions/rustolonia/releases/latest),
run `cargo update -p rustolonia`, and commit the manifest and lockfile together.
Do not substitute a moving branch or use a differently versioned host.

## Ship

Supports Windows/macOS/Linux glibc on x64 and arm64. Linux requires
X11/XWayland and fontconfig; see the
[platform requirements](https://github.com/Devolutions/rustolonia/blob/v12.1.0/rust/PLATFORMS.md#prebuilt-host-platform-floors).

Release-profile builds still load the development host cache by default.
For distribution, set `default-features = false` on the Rustolonia dependency,
rebuild, and ship the matching extracted host, native dependencies, and
notices beside the executable (or `../Frameworks` in a macOS bundle).
Follow the
[distribution guide](https://github.com/Devolutions/rustolonia/blob/v12.1.0/rust/rustolonia/README.md).

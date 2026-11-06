# rustolonia

Idiomatic Rust API for building GUI applications with
[Avalonia](https://avaloniaui.net/), backed by a .NET NativeAOT host.

```toml
[dependencies]
rustolonia = { git = "https://github.com/Devolutions/rustolonia", tag = "v12.1.0" }
```

Use this dependency once the `v12.1.0` GitHub release is published. Cargo
resolves `rustolonia-sys` from the same git revision automatically; no
registry patches or crates.io publication are required. The final tag includes
the checksums of the actual release build.
Rust 1.88 or newer is required. Rustolonia `12.1.0` uses Avalonia `12.1.3`:
major/minor identify the Avalonia line, while the patch counts Rustolonia
releases independently. The API is still experimental; use an exact release
tag to avoid unintended upgrades. After crates.io publication,
`rustolonia = "=12.1.0"` is also supported.

```rust,no_run
use rustolonia::{App, Button, Orientation, StackPanel, TextBlock, Window};

fn main() -> rustolonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        scope.mount(
            Window::new()?.title("Hello from Rust")?.content(Some(
                &StackPanel::new()?
                    .orientation(Orientation::Vertical)?
                    .child(TextBlock::new()?.text("Welcome to Avalonia!")?)?
                    .child(Button::new()?.content(Some(&TextBlock::new()?.text("Hello")?))?)?,
            ))?,
        )
    })
}
```

`cargo run` is all it takes.

Set the application name before `run` when shipping a desktop application:

```rust,no_run
use rustolonia::App;

fn main() -> rustolonia::Result<()> {
    App::load_from_env()?
        .with_application_name("Jiffshot")?
        .run(|scope| {
            // Create and mount your windows here.
            scope.shutdown()
        })
}
```

This sets Avalonia's `Application.Name` before initialization. On macOS,
Avalonia uses it for the running-process name, application-menu title and
`Hide Jiffshot` item, including when switching between windows. Window titles
and packaged app metadata (`CFBundleName` / `CFBundleDisplayName`) remain
separate; set those consistently when packaging. Omitting the option preserves
Avalonia's default name. The name must not be empty, whitespace-only or contain
NUL. The call is fallible: older hosts without the naming capability report
`E_NOINTERFACE`, and Android/iOS hosts report `E_NOTIMPL` because their platform
initializes Avalonia before Rust starts. Desktop, browser and Linux DRM hosts
accept the option before `run`; do not call it inside the startup callback.

On the first build, `rustolonia-sys` downloads the prebuilt
`rustolonia_host` library for your target from the matching
[GitHub release](https://github.com/Devolutions/rustolonia/releases),
checks it against the SHA-256 baked into the crate, and caches it. No .NET SDK
is needed. Supported targets: Windows, macOS, and Linux (glibc) on x64 and
arm64. To use a locally built host, offline builds, or a different mirror, see
the `rustolonia-sys` README (`RUSTOLONIA_HOST_DIR`, `RUSTOLONIA_NO_DOWNLOAD`,
`RUSTOLONIA_CACHE_DIR`, `RUSTOLONIA_HOST_BASE_URL`).

At runtime the host is found in this order: `RUSTOLONIA_HOST_LIB`, then next to
the executable (or `../Frameworks` in a macOS bundle), and then the build-time
host directory when the default `dev-host-path` feature is enabled. When you
ship an application, copy the host directory next to your executable and build
with `default-features = false` so that no build machine paths end up in the
binary.

The Rust and native host versions are locked to each other: `rustolonia X.Y.Z`
pins `rustolonia-sys =X.Y.Z`, which only downloads the `X.Y.Z` host release
built from the recorded Actions build revision. The `vX.Y.Z` consumer tag differs
from that revision only by the committed host checksums. When the app starts, the host's ABI fingerprint is
checked against the one the crate was built with.

Desktop Linux needs X11 (Wayland sessions work through XWayland) and fontconfig;
minimum OS versions are listed in
[PLATFORMS.md](https://github.com/Devolutions/rustolonia/blob/master/rust/PLATFORMS.md#prebuilt-host-platform-floors).
The prebuilt release covers code-first applications. Applications with
compiled AXAML and view-model IR still build their own host from a source
checkout.

For an embedded Linux appliance, enable `features = ["linux-drm"]` on this
dependency. Cargo then downloads a separate DRM/KMS host for the same Linux
GNU target, with no .NET SDK or local source checkout. Mount a fullscreen root
with `scope.set_main_view(&root)` rather than creating a `Window`. This requires
a new release containing the DRM assets; existing releases remain X11-only.
See the [standalone embedded Linux workflow](https://github.com/Devolutions/rustolonia/blob/master/rust/PLATFORMS.md#standalone-rust-apps-with-prebuilt-hosts)
for a complete dependency/example, deployment packaging, permissions, and
native runtime requirements.

The experimental [standalone iOS sample](https://github.com/Devolutions/rustolonia/tree/master/samples/ios-standalone)
uses a Rust static library and a prebuilt NativeAOT host, without building C#
or installing .NET on the consumer machine. It requires a future release
containing the iOS host assets; existing releases do not have them. iOS owns
the single-view lifetime and does not use desktop host discovery. Desktop-only
window and shell APIs are not supported.

Two things the projected controls cannot express are covered from Rust
directly: `pixel_surface::PixelSurface` is a control that draws bitmap layers
Rust fills and reports the raw pointer stream (press, move, release, wheel)
and keys to a Rust closure, for canvases, overlays and custom charts (see
`examples/pixel_surface.rs`); `ShellTray` is a notification-area icon whose
menu Rust builds, with no schema compiled into the host. Both need a host with
the stage 34 capability.

Application view-model schemas are owned by the consuming crate. In this
repository the flagship sample is [`../avalonia-sample`](../avalonia-sample),
and [`../templates/avalonia-app`](../templates/avalonia-app) (or
`../new-app.ps1`) is a starting point for new applications.

See [`../PRODUCTIZATION.md`](../PRODUCTIZATION.md) for the productized
workflow (templates, one-command build, host discovery, packaging, checksums,
and SBOM scope).
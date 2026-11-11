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
use rustolonia::{
    color, fmt, rsx, App, Button, Orientation, StackPanel, Style, StyleSheet, TextBlock, Window,
};

fn main() -> rustolonia::Result<()> {
    let styles = StyleSheet::from([
        Style::select("Button.primary")
            .background(color!("#2563eb"))
            .foreground(color!("#ffffff"))
            .corner_radius(6.0),
        Style::select("Button.primary:pointerover").background(color!("#1d4ed8")),
    ]);

    App::load_from_env()?.run_with_styles(styles, |scope| {
        let clicks = scope.signal(0_u32);
        scope.mount_view(rsx! {
            Window {
                title: "Hello from Rust",
                StackPanel {
                    orientation: Orientation::Vertical,
                    margin: 24.0,
                    spacing: 12.0,
                    TextBlock { "Welcome to Avalonia!" }
                    Button {
                        class: "primary",
                        on_click: move |_| clicks.update(|c| *c += 1),
                        text: fmt!("Clicked {} times", clicks.get())
                    }
                }
            }
        })?;
        Ok(())
    })
}
```

`cargo run` is all it takes.

## Declarative UI, Reactivity, and Async

- **Declarative `rsx!` & `#[component]`**: Author control trees with `rsx!`, inline `"string"` children, `class:`, `ref:`, `show cond { ... } else { ... }`, `for item in signal key |x| x.id { ... }`, compile-time `color!("#rrggbb")`, `tracks![auto, 1*, 240px]`, reactive `fmt!(...)`, and `#[derive(Lens)]` / `lens!`.
- **UI-thread reactive state**: `Signal<T>`, `Memo<T>`, and `NodeRef<T>` are `Copy` UI-thread handles, so UI event closures need no `Send` bounds or `Arc<Mutex<...>>`. Background threads update UI state through `signal.writer(scope)` (`SignalWriter<T>`) or `scope.channel(...)` (`UiSender<M>`).
- **Single-step async & off-thread work**: `scope.spawn_local(...)` polls `!Send` UI futures on the dispatcher; `scope.run_blocking(...)` / `scope.spawn_blocking(...)` run CPU or blocking I/O work off the UI thread; `scope.open_files(...)`, `scope.save_file(...)`, `scope.clipboard_text()`, and `scope.show_modal(&dialog)` resolve the active window automatically and return a single `Future<Output = Result<T>>`.
- **Pure-Rust styling & menus**: `StyleSheet` / `Style::select(...)` target Avalonia selector classes and pseudo-classes (`:pointerover`, `:pressed`, `:focus`, `:disabled`) with `Paint` (solid or linear/radial gradients), metrics, shadows, and property transitions. `MenuBar` and `Shortcut` bind native menus and window `KeyBinding`s reactively without integer command IDs.
- **Optional ecosystem features**: Enable `chrono`, `time`, `rust_decimal`, or `tokio` on `rustolonia` for seamless conversions with `CivilDate`, `CivilTime`, `Decimal`, and `create_resource`.

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
NUL. The call is fallible: Android/iOS hosts report `E_NOTIMPL` because their platform
initializes Avalonia before Rust starts. Desktop, browser and Linux DRM hosts
accept the option before `run`; do not call it inside the startup callback.

On the first build, `rustolonia-sys` downloads the prebuilt
`rustolonia_host` library for your target from the matching
[GitHub release](https://github.com/Devolutions/rustolonia/releases),
checks it against the SHA-256 baked into the crate, and caches it. No .NET SDK
is needed. Supported targets: Windows, macOS, and Linux (glibc or static musl)
on x64 and arm64. To use a locally built host, offline builds, or a different mirror, see
the `rustolonia-sys` README (`RUSTOLONIA_HOST_DIR`, `RUSTOLONIA_NO_DOWNLOAD`,
`RUSTOLONIA_CACHE_DIR`, `RUSTOLONIA_HOST_BASE_URL`).

At runtime the host is found in this order: `RUSTOLONIA_HOST_LIB`, then next to
the executable (or `../Frameworks` in a macOS bundle), and then the build-time
host directory when the default `dev-host-path` feature is enabled. When you
ship an application, copy the host directory next to your executable and build
with `default-features = false` so that no build machine paths end up in the
binary.

For an executable with the host and graphics libraries linked in, enable
`features = ["static-host"]` on the dependency and use a release containing
the static assets. No Rustolonia, Avalonia.Native, SkiaSharp, HarfBuzzSharp or
ANGLE shared library needs to accompany the executable. On Windows also build
with `-C target-feature=+crt-static`; on macOS set
`MACOSX_DEPLOYMENT_TARGET=12.0` or newer. On those platforms and Linux GNU,
operating-system libraries and frameworks remain dynamic.

For a fully static Linux executable, use a musl target and a release that
includes the separate musl assets:

```bash
rustup target add x86_64-unknown-linux-musl
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
RUSTFLAGS="-C target-feature=+crt-static -C relocation-model=static" \
cargo build --release --target x86_64-unknown-linux-musl
```

Enable `static-host` on the dependency and install a matching musl linker.
ARM64 uses `aarch64-unknown-linux-musl`. The static relocation model avoids a
`musl-gcc` static-PIE issue and produces a non-PIE executable. With the
application's other native dependencies also statically linked, the binary
needs an X server and fonts but no shared libraries. It uses software
rendering; see the
[`rustolonia-sys` README](../rustolonia-sys/README.md) for limitations.

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

`native_menu::MenuBar` and `native_menu::NativeMenu` build Avalonia's
[`NativeMenu`](https://docs.avaloniaui.net/controls/menus/nativemenu) from
Rust: window menus (the macOS menu bar, or an in-window `NativeMenuBar`
elsewhere) and the macOS application menu, with typed `Shortcut`s and reactive
check/radio items whose state Rust owns (see `examples/native_menu.rs` and
[MENUS.md](https://github.com/Devolutions/rustolonia/blob/master/rust/MENUS.md#code-first-native-menus)).

Application view-model schemas are owned by the consuming crate. In this
repository the flagship sample is [`../avalonia-sample`](../avalonia-sample),
and [`../templates/avalonia-app`](../templates/avalonia-app) (or
`../new-app.ps1`) is a starting point for new applications.

See [`../PRODUCTIZATION.md`](../PRODUCTIZATION.md) for the productized
workflow (templates, one-command build, host discovery, packaging, checksums,
and SBOM scope).
# Avalonia Rust external consumer

Two source checkouts are required: Rustolonia (Rust crates, interop, projection
tool and NativeAOT host) and its pinned Avalonia producer (framework and XAML
compiler). The producer normally lives at `avalonia-src` inside Rustolonia.
From this consumer directory, initialize dependencies once:

```powershell
pwsh "__RUSTOLONIA_ROOT__/rust/build-app.ps1" -ProducerRoot "__AVALONIA_PRODUCER_ROOT__" -Manifest ./avalonia-app.json -UpdateLockFile
```

Commit `Cargo.lock` and `global.json`. Subsequent builds use the same command
without `-UpdateLockFile`; Cargo runs with `--locked`. That switch explicitly
resolves a missing or changed lockfile, not a request to upgrade all dependencies.
Normal builds never format handwritten Rust. Run `cargo fmt` separately.

Keep the relative layout of consumer and source checkouts when moving them.
MSBuild roots can be overridden with `AvaloniaProducerRoot` and `RustoloniaRoot`;
Cargo path dependencies must also be updated if the checkout layout changes.
Pin Rustolonia as a submodule/revision and initialize its producer recursively,
then apply `avalonia-patches/apply-avalonia-patches.ps1` from Rustolonia.
Preflight requires the producer revision and applied patch content recorded in
Rustolonia's `rust/release-manifest.json`, initialized submodules, the supported
.NET SDK selected by `global.json`, Rust/Cargo/rustup and native build tools.
It does not download tools or repair source checkouts.

The manifest defaults to the scaffolding machine's OS and architecture; use
`new-app.ps1 -Rid` or edit `rid` and `outputDirectory` for another target.
Packaging must run on the target OS; same-OS cross-architecture builds still
require the target Rust/native toolchain and cannot be smoke-run on this host.

`build-app` regenerates the managed adapters, NativeAOT registry, and Rust
view-model API from `view-model.ir.json`, compiles this AXAML project and Rust
binary, and writes the adjacent runnable bundle declared by the manifest.

Generated external bindings use a crate-root compatibility bridge. Keep the
`view_model` exports in `src/main.rs`, including `DynamicViewModel`,
`ViewModelSink`, `ViewModelBatch`, and `BatchCompletion`.

The scaffold's schema declares a stage 31 `Main` menu with a copy command
(`Ctrl+C`), a recent-file submenu and `Exit` (`Ctrl+Q`). `MainWindow.axaml.cs`
attaches it through the generated `MainViewModelMenus.AttachMain`, which sets
the top-level's `NativeMenu` and installs the declared gestures as key
bindings; the `NativeMenuBar` in `MainWindow.axaml` renders that same menu
in-window on platforms without a native menu bar. Recent files are Rust-owned
storage URIs published through the generated `publish_recent_files`. See
`rust/MENUS.md` in the Rustolonia checkout.

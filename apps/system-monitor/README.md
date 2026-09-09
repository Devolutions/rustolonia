# NeoHtop system monitor

This is the repository's real Rustolonia application: an Avalonia port of
[NeoHtop](https://github.com/Abdenasser/neohtop). It was extracted from
`mamoreau-devolutions/neohtop` commit
`ba01de85f0b9543fe01338c7f0fabd957d5079f9`.

Rust owns process monitoring, refresh work, sorting, filtering, selection and
commands. `view-model.ir.json` is the application-owned schema. Rustolonia
generates the Rust dispatch, managed adapters and host view registry, while the
presentation project compiles AXAML and bindings into the NativeAOT host.

## Build on Windows x64

Initialize and patch the producer once from the repository root:

```powershell
git submodule update --init --recursive
pwsh ./avalonia-patches/apply-avalonia-patches.ps1 -AvaloniaRoot ./avalonia-src
```

Build the application:

```powershell
pwsh ./apps/system-monitor/build.ps1
```

The portable bundle is written to `apps/system-monitor/artifacts/win-x64`.
Launch `neohtop-avalonia.exe` with all adjacent files present. Use
`-UpdateLockFile` only when intentionally updating dependencies, then review
and commit `NeoHtop.App/Cargo.lock`.

## Opt-in developer tools (MCP)

Build and launch a development-only NativeAOT bundle from the repository root:

```powershell
pwsh .\apps\system-monitor\build.ps1 -DeveloperTools
.\apps\system-monitor\artifacts\win-x64-devtools\neohtop-avalonia.exe
```

The switch works with the manifest's **Release** configuration. It adds
`AvaloniaUI.DiagnosticsSupport` only to the host and attaches once in
`HostApplication.Initialize`, with diagnostic console logging enabled.
The shared `rust\build-app.ps1` accepts the same switch; its output directory
is the manifest output directory plus `-devtools`. Normal builds explicitly
disable diagnostics and leave development bundles separate from release bundles.
Do not distribute the instrumented bundle: the connected tools can inspect and
modify application state.

Use the already configured Avalonia DevTools MCP server to `attach-to-app`
(select this executable's PID if several apps are running), then inspect its
tree and properties. No F12 keypress is required for MCP discovery.
See the [MCP setup guide](https://docs.avaloniaui.net/tools/developer-tools/mcp).
The server requires the `AvaloniaUI.DeveloperTools` global tool and an Avalonia
Plus license supplied privately through `AVALONIA_TOOLS_LICENSE_KEY` for Avalonia
12. Do not put license keys in this repository.

This remains a source-built Avalonia NativeAOT host, not a managed debug
executable. DiagnosticsSupport 2.2.3 supplies an Avalonia 12 implementation
without introducing binary Avalonia packages. Runtime XAML loading/hot reload
is not supported by this NativeAOT bundle. MCP attachment, tree inspection,
property reads, screenshots and search-box input work with this configuration,
including filtering through the Rust-owned view model. NativeAOT publication
still warns about a bitmap encoder ABI reference in DiagnosticsSupport 2.2.3;
the exercised MCP screenshot path works, but other image paths may be affected.

## Coverage

The process list keeps row models keyed by PID and start time across refreshes.
It updates row properties in batches and reconciles additions, removals and
sort moves instead of invalidating windowed pages on every sample. This avoids
refresh-time placeholder churn and preserves process identity across PID reuse.
Unlike the previous paged model, it retains a model for every filtered process;
the TableView still virtualizes its visual rows.

Summary cards use a bounded, scrollable CPU-core list, and process rows use
36-pixel spacing. Account names are resolved when available, with identifiers
as a fallback and tooltips for truncated user and process names.

```powershell
Push-Location ./apps/system-monitor
cargo test --locked --manifest-path ./NeoHtop.App/Cargo.toml
Pop-Location

powershell.exe -NoProfile -STA -ExecutionPolicy Bypass `
  -File ./apps/system-monitor/tests/test-bundle.ps1
```

The Rust tests cover filtering, sorting, selection safety, monitoring and
joined refresh-worker teardown. The interactive Windows UI Automation smoke
checks populated statistics, search filtering, own-process selection,
freeze/resume and natural shutdown. It never terminates a process.

The checked-in application manifest and native acceptance currently target
Windows x64. Linux and macOS execution are not claimed. The produced directory
is a portable bundle, not an installer, and is unsigned unless
`AVALONIA_RUST_SIGN_COMMAND` is configured.

NeoHtop-derived source remains under its MIT license; see
[`LICENSE.neohtop`](LICENSE.neohtop).

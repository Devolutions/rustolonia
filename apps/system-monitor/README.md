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

## Coverage

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

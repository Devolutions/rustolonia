# Rustolonia archive explorer

This sample is a sibling external consumer under `apps/` that is a dual-pane
archive manager. Rust owns format detection, listing, zip-slip path checks and
extraction; the generated view-model bridge exposes that state to the compiled
Avalonia presentation. Listing and extraction run on worker threads so the UI
stays responsive.

Supported formats: `.zip`, `.tar`, `.tar.gz` / `.tgz`, `.tar.bz2`, `.tar.xz`,
`.tar.zst`, `.gz`, `.bz2`, `.xz`, `.zst`, `.lz4`, `.7z`, `.cab`. RAR is out of
scope. The app opens on your home folder and Downloads. Open an archive in the
active pane, copy selected items to the other pane, and delete only on the
filesystem side.

## Build on Windows x64

From the repository root:

```powershell
pwsh ./rust/build-app.ps1 `
  -ProducerRoot ./avalonia-src `
  -Manifest ./apps/archive-tool/avalonia-app.json `
  -UpdateLockFile
```

The portable bundle is written to `apps/archive-tool/artifacts/win-x64`.
Subsequent locked builds omit `-UpdateLockFile`.

## Opt-in developer tools (MCP)

```powershell
pwsh ./rust/build-app.ps1 `
  -ProducerRoot ./avalonia-src `
  -Manifest ./apps/archive-tool/avalonia-app.json `
  -DeveloperTools
.\apps\archive-tool\artifacts\win-x64-devtools\archive-tool.exe
```

This attaches Avalonia DevTools on the NativeAOT host. Use the DevTools MCP
`attach-to-app` tool against the process. Do not distribute the instrumented
bundle.

## Tests

```powershell
Push-Location ./apps/archive-tool
cargo test --locked
Pop-Location
```

After building, the Windows UI Automation smoke test checks the idle window
and natural shutdown. It never opens a picker:

```powershell
powershell.exe -NoProfile -STA -ExecutionPolicy Bypass `
  -File ./apps/archive-tool/tests/test-bundle.ps1
```

The app is a portable Win32 GUI bundle, not an installer. File-association
metadata snippets are under `file-associations/`.

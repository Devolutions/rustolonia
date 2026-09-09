# Rustolonia archive explorer

This sample is a sibling external consumer under `apps/` that lists zip and tar
archives and extracts selected entries. Rust owns format detection, listing,
zip-slip path checks and extraction; the generated view-model bridge exposes
that state to the compiled Avalonia presentation. Listing and extraction run on
worker threads so the UI stays responsive.

Supported formats: `.zip`, `.tar`, `.tar.gz` / `.tgz` (no 7z/RAR codecs). With
no arguments the app waits for **Open**, a dropped file, or an "open with"
activation. Folder browsing, CRC, Test, Extract here, smart extract, New archive
from a folder, and opening nested zip/tar entries stay on those crates.

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

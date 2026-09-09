# Rustolonia hash calculator

This sample is a sibling external consumer under `apps/` that combines
Rustolonia, Avalonia and streaming checksum crates (`sha2`, `sha1`, `md-5`,
`blake3`). Rust owns file hashing, algorithm selection, digest comparison and
the file list; the generated view-model bridge exposes that state to the
compiled Avalonia presentation. Hashing runs on a dedicated worker thread and
reads files in 64 KiB chunks so large inputs keep the UI responsive.

With no arguments, the app writes a small sample file to the system temporary
directory and hashes it with SHA-256. Passing local file paths hashes those
instead. **Open files** uses Avalonia's platform picker (multi-select). Toggle
MD5, SHA-1, SHA-256, SHA-512 and BLAKE3 independently. Paste an expected digest
to mark matching files; **Copy** / **Copy report** write hex to the clipboard.

## Build on Windows x64

From the repository root:

```powershell
pwsh ./rust/build-app.ps1 `
  -ProducerRoot ./avalonia-src `
  -Manifest ./apps/hash-calculator/avalonia-app.json `
  -UpdateLockFile
```

The portable bundle is written to `apps/hash-calculator/artifacts/win-x64`.
Subsequent locked builds omit `-UpdateLockFile`.

## Tests

Run the Rust hashing tests:

```powershell
Push-Location ./apps/hash-calculator
cargo test --locked
Pop-Location
```

After building, the Windows UI Automation smoke test exercises startup sample
generation, SHA-256 output, enabling MD5, compare-against-digest and natural
shutdown:

```powershell
powershell.exe -NoProfile -STA -ExecutionPolicy Bypass `
  -File ./apps/hash-calculator/tests/test-bundle.ps1
```

The app is a portable Win32 GUI bundle, not an installer. File-association
metadata snippets are under `file-associations/`.

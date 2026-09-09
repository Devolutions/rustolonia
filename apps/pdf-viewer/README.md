# Rustolonia PDF viewer

This sample is a sibling external consumer under `apps/` that combines
Rustolonia, Avalonia and [`pdf_oxide`](https://crates.io/crates/pdf_oxide).
Rust owns PDF parsing, page rasterization, text extraction and navigation state;
the generated view-model bridge exposes that state to the compiled Avalonia
presentation. PDF work runs on a dedicated worker thread, reuses the parsed
document while paging, bounds preview raster sizes, and caps the extracted-text
preview so large or image-heavy documents keep the UI responsive. The Find bar
uses PDF Oxide's positioned text search and seeks to the page containing each
match; the previous/next match controls wrap through the result set.

With no arguments, the app creates a small multi-page sample PDF in the system
temporary directory and renders its first page. Passing a local PDF path opens
that document instead. The **Open PDF** command uses Avalonia's platform file
picker, and **Previous**/**Next** navigate rendered pages.

## Build on Windows x64

From the repository root:

```powershell
pwsh ./rust/build-app.ps1 `
  -ProducerRoot ./avalonia-src `
  -Manifest ./apps/pdf-viewer/avalonia-app.json `
  -UpdateLockFile
```

The portable bundle is written to `apps/pdf-viewer/artifacts/win-x64`.
Subsequent locked builds omit `-UpdateLockFile`.

## Tests

Run the Rust PDF generation/rendering test:

```powershell
Push-Location ./apps/pdf-viewer
cargo test --locked
Pop-Location
```

After building, the Windows UI Automation smoke test exercises startup sample
generation, page rendering, next/previous navigation and natural shutdown:

```powershell
powershell.exe -NoProfile -STA -ExecutionPolicy Bypass `
  -File ./apps/pdf-viewer/tests/test-bundle.ps1
```

The app is a portable Win32 GUI bundle, not an installer. File-association
metadata snippets are under `file-associations/`.

# File type association metadata

These are packaging metadata snippets, not installers. They register a document
type so the desktop shell launches this application with the selected file path
as a command-line argument; the runtime side of "open with" is already wired,
because `App::run` forwards this process's arguments to the managed desktop
lifetime and `AppScope::activation_items()` returns them normalized (see
`rust/DESKTOP_FILES.md` in the pinned Avalonia producer checkout).

Deliberately out of scope here: MSIX packaging, `.msi`/`.pkg`/`.deb`/`.rpm`
installers, notarization, and any store submission. Those belong to whatever
distribution channel a consumer chooses; this workflow ships a deterministic
per-RID directory (see `PRODUCTIZATION.md`).

Replace `__AVALONIA_APP_NAME__` (already substituted by `new-app`), the
extension `.myapp`, and the install path before shipping any of these.

| Platform | File | Applied by |
| --- | --- | --- |
| Windows | `windows-file-association.reg` | Your installer writing the same keys, or `reg import` for local testing. |
| Linux | `linux-desktop-entry.desktop`, `linux-mime-type.xml` | `desktop-file-install` / `xdg-mime install` from your package's post-install step. |
| macOS | `macos-Info.plist.snippet` | Merged into the `.app` bundle's `Info.plist`. |

## Verifying the runtime side

Once an association is registered, launching a file through the shell reaches
Rust as an activation item:

```rust
for item in scope.activation_items()? {
    // `uri()` is always present; `local_path()` may be `None` on platforms
    // that hand the application a non-local document reference.
    println!("open with: {} ({})", item.uri(), item.name());
}
```

macOS additionally raises later activations while the application is already
running; subscribe with `AppScope::on_activation` to receive them.

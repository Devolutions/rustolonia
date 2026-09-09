# File type association metadata

These are packaging metadata snippets, not installers. They register archive
types so the desktop shell launches this application with the selected file
path as a command-line argument; the runtime side of "open with" is already
wired, because `App::run` forwards this process's arguments to the managed
desktop lifetime and `AppScope::activation_items()` returns them normalized
(see `rust/DESKTOP_FILES.md` in the pinned Avalonia producer checkout).

Deliberately out of scope here: MSIX packaging, `.msi`/`.pkg`/`.deb`/`.rpm`
installers, notarization, and any store submission. Those belong to whatever
distribution channel a consumer chooses; this workflow ships a deterministic
per-RID directory (see `PRODUCTIZATION.md`).

These snippets cover `.zip`, `.tar`, and `.tgz`. A real installer should still
own the association lifecycle and should avoid claiming archives without an
explicit user choice.

| Platform | File | Applied by |
| --- | --- | --- |
| Windows | `windows-file-association.reg` | Your installer writing the same keys, or `reg import` for local testing. |
| Linux | `linux-desktop-entry.desktop`, `linux-mime-type.xml` | `desktop-file-install` / `xdg-mime install` from your package's post-install step. |
| macOS | `macos-Info.plist.snippet` | Merged into the `.app` bundle's `Info.plist`. |

# Jiffshot

A screen capture tool and image editor for Windows, written entirely in Rust
with its UI on Rustolonia's code-first API (no AXAML, no C#). It runs as one
program, `jiffshot.exe`: the tray icon and capture side on their own thread,
the editor and every other window on Avalonia.

## Features

**Capture**
- Region, window (the mouse wheel walks up and down child elements), screen
  and all screens, from a frozen overlay with a magnifier, size labels and
  aspect locks (Shift square, Ctrl 16:9, Ctrl+Shift 4:3).
- Scrolling (vertical, horizontal or both), panoramic, freehand, multiple
  areas, fixed size, open menus, Grab Text (Windows OCR) and Step Capture (a
  guide of numbered screenshots, one per click).
- Time delay, time-lapse and a daily schedule; presets with their own
  hotkeys; the Capture window (All-in-One, Image and Video tabs) and a small
  Capture Widget at the top of the screen.
- After a capture: capture effects, the clipboard, the editor, and share
  destinations (file, email, Office apps, any program, FTP, print).

**Recording**
- An area, a window or a screen, with system sound, the microphone or both,
  the webcam in a corner, pointer highlight and click rings, and Screen Draw.
- A toolbar under the area: stop, pause, restart, minimize, cancel, and the
  pointer, webcam and sound switches, which change the recording as it runs.
- In the editor: trim, cut, frame stepping, GIF export and videos made from
  images.

**Editor**
- Arrow, line, shape, text, callout, step, pen, highlight, blur, magnify,
  spotlight, stamps, fill, crop, cut out, selection, magic wand, eraser,
  eyedropper, Smart Move and Text Replace, with quick styles.
- Effects (borders, shadows, edges, perspective, page curl and more),
  templates with drop zones, and combined images.
- A library of captures with search, filters, tags and pins; export to PNG,
  JPEG, PDF, BMP, GIF and WebP; editable `.jsd` documents.

One Preferences dialog (Capture, Editor, Library, Share, Hotkeys) and one
color theme cover every window; settings are `%APPDATA%\Jiffshot\jiffshot.json`
and captures go to `Pictures\Jiffshot` unless changed.
[docs/architecture.md](docs/architecture.md) describes how it fits together.

## Layout

| Path | Contents |
|---|---|
| `crates/jiffshot-capture/` | Monitors and DPI, screen grabs, window targets, clipboard, OCR, media (recording and playback), the library, settings |
| `crates/jiffshot-doc/` | Documents: annotation objects, rendering, editing and undo, effects, smart tools, `.jsd` and image files |
| `crates/jiffshot-workflow/` | The capture side: tray, hotkeys, selection overlay, recording, capture panels, post-capture workflow, command-line modes |
| `app/` | `jiffshot.exe`: the editor's model (`src/app.rs`, view models in `src/vm.rs`), every window (`src/ui/`) and the capture side, in one process |

## Building

Jiffshot needs Rust 1.88+ and nothing else: it depends on Rustolonia's
`v12.1.2` release, and the first build downloads that release's prebuilt
host (checked against the checksums pinned at the tag). No .NET SDK or
Rustolonia checkout is involved.

```powershell
cd apps/jiffshot
cargo run --release -p jiffshot
```

The build copies the host next to `jiffshot.exe`, so `target/release` runs as
it is; zip `jiffshot.exe` and the host DLLs beside it, with
`app/README-portable.txt`, for the portable app.

The `apps/jiffshot` folder builds the same way outside this repository: copy
it anywhere. Its `.cargo/config.toml` only undoes this repository's
`RUSTOLONIA_NO_DOWNLOAD=1` for the folder. To move to a newer Rustolonia,
change the tag on the two lines in `Cargo.toml`'s `[workspace.dependencies]`.

With no arguments Jiffshot starts in the tray. `jiffshot.exe FILE` opens a
file in the editor, `--capture`, `--record`, `--window` and the other capture
flags start a capture, and `--quit` ends it; a second launch hands its request
to the one already running. Closing the editor keeps Jiffshot in the tray.

## Testing

From `apps/jiffshot`:

```powershell
cargo test --release --workspace                          # capture side, documents, media, library, editor
./target/release/jiffshot.exe --selftest DIR                  # overlay logic, offscreen
./target/release/jiffshot.exe --record-selftest DIR 4         # records its own test window
```

The editor drives its own UI when `JIFFSHOT_UITEST=DIR` is set (with
`JIFFSHOT_UITEST_IMAGE`, `JIFFSHOT_UITEST_VIDEO`, an isolated
`JIFFSHOT_LIBRARY` and, to leave your settings alone, scratch `APPDATA` and
`LOCALAPPDATA`), writing screenshots of every window and `ui-log.txt` to
`DIR`. `cargo run -p jiffshot-doc --example synthetic -- OUT.png` writes the
test image its coordinates assume.
The capture panels and outlines are left out of screen captures; set
`JIFFSHOT_HUD_CAPTURABLE=1` to see them in screenshots while checking them.
`JIFFSHOT_CAPTURE_DRY_RUN=1` runs the selection overlay for real but only logs
the selection (nothing is saved or copied), for checking it live;
`JIFFSHOT_RECORD_DRAW=1` and `JIFFSHOT_RECORD_SWITCH=1` make
`--record-selftest` check Screen Draw marks and switching sound and the
pointer part-way.

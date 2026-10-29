# Jiffshot

A screen capture tool and image editor for Windows, written in Rust with its
UI on Rustolonia. It runs as one program, `jiffshot.exe`: the tray icon and
capture side on their own thread, the editor and every other window on
Avalonia.

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
| `app/` | `jiffshot.exe`: the editor (Rust model in `src/`, Avalonia presentation in `managed/`, view-model IR from `tools/gen_ir.py`) and the capture side, in one process |

## Building

Jiffshot builds against this repository: Rust 1.88+, the .NET 10 SDK,
Python (for the view-model IR) and the `avalonia-src` submodule. From the
repository root:

```powershell
git submodule update --init avalonia-src
pwsh apps/jiffshot/app/build.ps1          # -Rid win-arm64 for ARM64; -ProducerRoot for another producer checkout
./apps/jiffshot/app/artifacts/win-x64/jiffshot.exe
```

`app/build.ps1` regenerates `view-model.ir.json` from `app/tools/gen_ir.py`
and runs Rustolonia's `build-app.ps1`. `app/README-portable.txt` is the
README that goes into a portable zip of the build output.

With no arguments Jiffshot starts in the tray. `jiffshot.exe FILE` opens a
file in the editor, `--capture`, `--record`, `--window` and the other capture
flags start a capture, and `--quit` ends it; a second launch hands its request
to the one already running. Closing the editor keeps Jiffshot in the tray.

## Testing

From `apps/jiffshot`:

```powershell
cargo test --release --workspace                          # capture side, documents, media, library
cd app; cargo test --release; cd ..                       # editor model
./app/artifacts/win-x64/jiffshot.exe --selftest DIR           # overlay logic, offscreen
./app/artifacts/win-x64/jiffshot.exe --record-selftest DIR 4  # records its own test window
```

The editor drives its own UI when `JIFFSHOT_UITEST=DIR` is set (with
`JIFFSHOT_UITEST_IMAGE`, `JIFFSHOT_UITEST_VIDEO` and an isolated
`JIFFSHOT_LIBRARY`), writing screenshots and `ui-log.txt` to `DIR`.
The capture panels and outlines are left out of screen captures; set
`JIFFSHOT_HUD_CAPTURABLE=1` to see them in screenshots while checking them.
`JIFFSHOT_CAPTURE_DRY_RUN=1` runs the selection overlay for real but only logs
the selection (nothing is saved or copied), for checking it live;
`JIFFSHOT_RECORD_DRAW=1` and `JIFFSHOT_RECORD_SWITCH=1` make
`--record-selftest` check Screen Draw marks and switching sound and the
pointer part-way.

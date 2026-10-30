# Architecture

Jiffshot is one executable and one process, `jiffshot.exe`, written in Rust
only, with every window on Rustolonia's code-first API (Avalonia): the editor,
the tray icon, and the capture side's overlay, panels and dialogs.

## Windows and view models

`app/src/vm.rs` holds the view models: each window's values are signals the
model code publishes to (`app.rs` and friends call the sinks, from any
thread), and each window sends what the user does back through the model
traits. `app/src/ui/` builds the windows from those signals. Two Rustolonia
pieces cover what ordinary controls cannot: pixel surfaces (the editor
canvas, the selection overlay and Screen Draw draw Rust pixels and take the
raw pointer stream) and a Rust-built tray icon.

## Threads

- **UI thread (Avalonia).** Every window: the editor, the Capture window, the
  tray icon and its menu, toasts, the Capture Widget, Grab Text, the capture
  panels and outline, the selection overlay, Screen Draw and Preferences. The
  editor's `App` lives here (`app/src/app.rs`, reached with `app::post`).
- **Capture thread** (`jiffshot-capture`, `crates/jiffshot-workflow`). A
  message-only window for hotkeys, timers (delay, time-lapse, schedule), the
  Step Capture mouse hook, and the capture flows. It talks to the app through
  `Hooks` (open a capture in the editor, toasts, tray state, the HUD and the
  overlay as JSON) and takes work through `on_capture_thread`.
- **Workers.** Finishing captures, recording (`recorder::record_to`) and
  scrolling captures.

A second launch hands its request to the running one (`--capture`, a file to
open), passing on its right to take the foreground.

## The overlay

`overlay.rs` is the selection logic only; `Overlay::view` says what to show.
`ui_api.rs` is the table of calls the app's overlay windows use
(`app/src/ui/overlay_api.rs`): input in (desktop pixels), state, texts and
points out, and the snapshot copied straight into each window's pixel
surface. One window per monitor (`app/src/ui/overlay.rs`) is made at startup
and only hidden natively between captures; a capture loads the new
screenshot, waits about a frame, then shows the windows natively in physical
pixels.

Screen Draw's marks live in Rust and render to one image: its window shows
it (left out of captures) and the recorder blends it into every frame.

## Settings

One file, `%APPDATA%\Jiffshot\jiffshot.json`
(`crates/jiffshot-capture/src/config.rs`):

```json
{ "library": "D:\\Captures", "theme": "Dark", "capture": { }, "editor": { } }
```

Each side owns its section; the library folder and the color theme are shared
at the top, so captures are saved where the editor lists them and every
window has the same theme. Reads and writes share one lock.
`JIFFSHOT_LIBRARY` overrides the library for both sides without being saved.

## Preferences

One dialog (`app/src/ui/prefs_window.rs`): Capture, Editor, Library, Share and
Hotkeys. Rust describes each tab as rows (`src/prefs.rs`, and
`jiffshot_workflow::prefs` for the capture side's tabs) and applies the values
the dialog sends back. It opens from the editor's Edit menu (Ctrl+,), the
Capture window's File and Edit menus, and the tray.

## Checking it

See the README's Testing section; for live checks, `JIFFSHOT_CAPTURE_DRY_RUN`
runs the overlay without saving anything, and `JIFFSHOT_HUD_CAPTURABLE` lets
screenshots see the panels.

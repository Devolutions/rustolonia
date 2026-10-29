# Architecture

Jiffshot is one executable and one process, `jiffshot.exe`, with every window
on Rustolonia (Avalonia): the editor, the tray icon, and the capture side's
overlay, panels and dialogs.

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
`ui_api.rs` is a C function table (the `UiApi` property) the Avalonia side
calls: input in (desktop pixels), state, texts and points out, and the
snapshot copied straight into each window's bitmap. One window per monitor
(`OverlayWindows.cs`) is made at startup and only hidden natively between
captures; a capture loads the new screenshot, waits for that frame to be
rendered, then shows the windows natively in physical pixels. On a
three-screen desktop the overlay is up 105–130 ms after the hotkey, of which
the GDI grab is 70–85 ms.

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

One dialog (`PreferencesWindow.cs`): Capture, Editor, Library, Share and
Hotkeys. Rust describes each tab as rows (`src/prefs.rs`, and
`jiffshot_workflow::prefs` for the capture side's tabs) and applies the values
the dialog sends back. It opens from the editor's Edit menu (Ctrl+,), the
Capture window's File and Edit menus, and the tray.

## Checking it

See the README's Testing section; for live checks, `JIFFSHOT_CAPTURE_DRY_RUN`
runs the overlay without saving anything, and `JIFFSHOT_HUD_CAPTURABLE` lets
screenshots see the panels.

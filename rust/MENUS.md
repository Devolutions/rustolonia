# Menus, accelerators, recent files and clipboard commands

Stage 31 projects a declared command surface into Rust applications: an
application menu, context menus, keyboard accelerators, a recent-file list, and
clipboard operations. Everything is IR-driven and generalized -- the schema
declares the shape, the generator emits named APIs, and no application name or
application-specific method ever reaches the ABI.

## What is presentation and what is state

The split is the same one the rest of this design uses, and it decides where
each half lives.

| Concern | Owner | Mechanism |
| --- | --- | --- |
| Menu structure, headers, gestures, check marks | Managed presentation | Generated `NativeMenu`/`ContextMenu` factories |
| Commands, toggles, radio values, recent-file entries | Rust state | Already generated command and property surface |
| Clipboard read/write/clear | Host, on Avalonia's `IClipboard` | New `IAvnApplication4` capability |

Because a menu only binds already generated commands and properties, **menus
add no view-model ABI at all**. The clipboard does need the host, so it gets one
new, separately versioned capability. The published `IAvnApplication`,
`IAvnApplication2` and `IAvnApplication3` vtables are untouched.

## Schema version 5

Menus, recent files and display paths are schema-v5 members. A version 4
document that declares any of them is rejected with an explicit upgrade message
rather than silently downgraded:

```
View-model IR version 4 does not support menus, keyboard accelerators,
recent-file lists or display paths; upgrade to version 5.
```

A model may declare menus, one recent-file list, and one display path:

```json
{
  "menus": [
    {
      "id": 1,
      "name": "Main",
      "kind": "Application",
      "items": [
        { "id": 1, "name": "File", "kind": "Submenu", "header": "_File", "items": [
          { "id": 2, "name": "Open", "kind": "Command", "header": "_Open files...",
            "command": "OpenFiles", "gesture": "Ctrl+O" },
          { "id": 3, "name": "Recent", "kind": "RecentFiles", "header": "Recent _files" },
          { "id": 4, "name": "FileSeparator", "kind": "Separator" },
          { "id": 5, "name": "Exit", "kind": "Command", "header": "E_xit",
            "command": "ExitApplication", "gesture": "Ctrl+Q" }
        ] }
      ]
    }
  ],
  "recentFiles": {
    "id": 1,
    "collection": "RecentFiles",
    "activateCommand": "OpenRecentFile",
    "capacity": 8
  },
  "displayPath": "Message"
}
```

Item kinds are `Command`, `Separator`, `Toggle`, `Radio`, `Submenu` and
`RecentFiles`. Validation is deliberately strict, because a menu that silently
does nothing is worse than a schema error: a command must exist, a toggle must
name a *writable* Boolean property, a radio value must be a member of the bound
enum, a separator may declare nothing else, and every item in an
accelerator-only menu must declare a gesture.

## Menu kinds

### `Application`

Projected as one Avalonia `NativeMenu` attached to a top-level:

```csharp
_menu = SampleViewModelMenus.AttachMain(this, adapter);
```

There is no platform branch. On macOS the platform exports the same
`NativeMenu` as the real application menu; everywhere else a `NativeMenuBar`
control in the window renders it:

```xml
<DockPanel>
  <NativeMenuBar DockPanel.Dock="Top" />
  ...
</DockPanel>
```

### `Context`

Projected as a generated `ContextMenu` subclass, so compiled AXAML attaches it
declaratively and it inherits its target control's data context:

```xml
<TableView.ContextMenu>
  <vm:SampleViewModelTraceRowsContextMenu />
</TableView.ContextMenu>
```

### `Accelerators`

Projected only as key bindings, for shortcuts that appear on no menu:

```csharp
_shortcuts = SampleViewModelMenus.AttachShortcuts(this, adapter);
```

## Accelerators

A declared gesture is carried through to `KeyGesture.Parse`, so the whole
Avalonia syntax (`Ctrl+O`, `Ctrl+Shift+S`, `Cmd+O`) is available and the
platform-specific meaning stays Avalonia's concern.

**A declared gesture always becomes a real key binding.** Only a native menu bar
handles its own shortcuts, and `NativeMenuBar` deliberately maps
`NativeMenuItem.Gesture` to `MenuItem.InputGesture`, which *displays* the
shortcut without handling it. `AttachMain` therefore also installs every
declared gesture as a `KeyBinding` on the top-level, bound to the same command
object as the menu item. That is why `Ctrl+O` works identically on Windows,
Linux and macOS.

## Enabled state

A menu item's enabled state is **not** assigned. Both `NativeMenuItem` and
`MenuItem` derive `IsEnabled` from their command's `CanExecute` and overwrite
any local value when it changes, so an assignment would be silently undone.
The generated `RustMenuCommand` instead composes:

- the underlying generated command's `CanExecute` (which is what Rust's
  `set_{command}_enabled` drives), and
- an optional declared `isEnabledProperty`.

A submenu is the one exception: it carries no command, so its `IsEnabled` is
observed directly from the declared property.

## Toggles and radio groups

A toggle item mirrors a writable Boolean property and writes its inverse when
invoked. A radio item mirrors `property == value` and writes `value`. The
property is written **first**, then any declared command runs, so Rust observes
the new state.

Both are driven from the command rather than from `IsChecked`, because whether
the platform toggles its own item is platform-specific. The check mark is always
recomputed from the model, so the model stays authoritative.

## Recent files

The recent-file list is a bounded, de-duplicated, most-recently-used list of
**stage 29 storage URIs**. That choice is the whole design:

- a URI is the only member `IStorageItem` guarantees, so the same value
  identifies a picked file, a dropped file and an "open with" activation;
- it rides the already published string-collection transport, so recent files
  need **no new ABI**;
- the menu header is derived from the URI's last path segment, and the URI
  itself is the command parameter, so activation needs no side table.

This is deliberately *not* a Windows jump list: there is no shell registration
and nothing platform-specific.

Rust owns the list:

```rust
let mut recent = RecentFileList::with_capacity(SAMPLE_VIEW_MODEL_RECENT_FILES_CAPACITY);
if recent.push_item(&item).is_changed() {
    sink.publish_recent_files(&recent)?;
}
```

`push` reports `Unchanged` when the entry is already at the front, so
re-opening the current file does not republish.

## Clipboard

Plain-text write and read already exist on the frozen `IAvnApplication` vtable
and are unchanged. Stage 31 adds `IAvnApplication4`:

| Method | Purpose |
| --- | --- |
| `CreateClipboardData` | Creates a host-owned payload builder |
| `GetClipboardCapabilities` | Reports availability and file support |
| `StartClipboardWrite` | Writes text and/or file entries |
| `StartClipboardClear` | Clears the clipboard |
| `StartClipboardReadFiles` | Reads file entries as storage snapshots |

The payload builder is host-owned for the same reason picker options are: that
direction of the ABI then carries only primitives and UTF-16 strings, and the
real `DataTransfer` is built on the UI thread when the write runs. The payload
is snapshotted at start, so mutating the builder afterwards cannot change an
in-flight write.

Reading file entries reuses the published `IAvnStorageCompletion` and returns
the same immutable snapshots pickers and drops produce. A clipboard carrying no
files is a **successful empty result**, not a cancellation and not an error --
"nothing to paste" must stay distinguishable from "the user cancelled".

Every clipboard operation is asynchronous and executor-neutral:

```rust
let operation = scope.clipboard_write(&window, &ClipboardData::text(text))?;
scope.spawn(async move {
    let _ = sink.set_clipboard_status(match operation.await {
        Ok(()) => "Copied".to_string(),
        Err(error) => format!("Clipboard write failed: {error}"),
    });
});
```

A synchronous clipboard API would be a UI-thread hazard: a clipboard read can
block for as long as the owning application takes to render the requested
format.

"Cut" is not a clipboard primitive. It is a copy plus an application-defined
removal, so it is an ordinary generated command; the sample's Edit menu shows
both spellings against the same clipboard write.

## Accessible row names

A data row's accessible name used to fall back to `Content.ToString()`, which
reports the adapter's CLR type name for any view model that does not override
it. Two additive changes fix it:

- `TableViewRow` now creates a `TableViewRowAutomationPeer` that composes the
  name from the row's realized cells -- the text the user actually sees. This
  applies to every `TableView`, not only generated ones.
- A model may declare `displayPath`, a dotted path ending in a string property,
  which the generated adapter projects as `ToString()`. That makes the fallback
  deterministic and declarable for list items and tree nodes too.

`TableViewColumn` also gained `MinWidth`/`MaxWidth`. They apply to every width
mode and to interactive resizing, so the schema's declared `minWidth` is now
honoured by compiled AXAML instead of being metadata-only.

## Honest platform limits

- **Native menu bar**: only macOS has one. Elsewhere the same `NativeMenu` is
  rendered in-window by `NativeMenuBar`, which is a real difference in
  appearance, not in behaviour.
- **Accelerators**: handled by the platform menu on macOS and by the installed
  key bindings everywhere; the binding is installed unconditionally, so a
  shortcut never silently stops working.
- **Clipboard file entries**: require a storage provider on the same top-level.
  `ClipboardCapabilities::supports_files` reports whether one exists, and a
  write without one still writes its text.
- **Unresolvable file entries** are dropped rather than failing the copy: a
  recent-file URI may point at something that has since been deleted.
- **Validation was Windows-only.** Linux and macOS behaviour rests on Avalonia's
  own `NativeMenu` exporters and clipboard implementations and on the stage 26
  cross-platform CI gate, not on a local run.

# Desktop file integration

Stage 29 gives external Rust consumers first-class, platform-neutral desktop
file integration: an open-file picker (multi-select), a folder picker, a
save/export picker, incoming file drag-and-drop, and startup/"open with"
activation. Everything goes through Avalonia's own abstractions --
`TopLevel.StorageProvider`, `IStorageFile`/`IStorageFolder`, the `DragDrop`
routed events, and the desktop lifetime -- so nothing here is a platform dialog
call and nothing is application specific.

## Capability, not a vtable extension

The published `IAvnApplication` and `IAvnApplication2` vtables are unchanged.
The whole feature is a separately versioned optional capability that Rust
queries:

| Interface | IID suffix | Implemented by |
| --- | --- | --- |
| `IAvnApplication3` | `...4D50` | host |
| `IAvnFilePickerOptions` | `...4D51` | host |
| `IAvnStorageItem` | `...4D52` | host |
| `IAvnStorageItemList` | `...4D53` | host |
| `IAvnStorageCompletion` | `...4D54` | Rust |
| `IAvnFileDropHandler` | `...4D55` | Rust |
| `IAvnActivationHandler` | `...4D56` | Rust |

Options are a *host-owned builder* rather than a Rust-implemented interface, so
the ABI only ever carries primitives and UTF-16 strings in that direction. See
[COMPATIBILITY.md](COMPATIBILITY.md#native-abi) for the rules this follows.

## Storage items are snapshots

Every item that crosses the ABI is an immutable snapshot: kind (file/folder),
name, URI, and an *optional* local path.

```rust
for item in outcome.items() {
    let where_it_is = match item.local_path() {
        Some(path) => path.display().to_string(),
        None => item.uri().to_string(),   // content:, browser handles, ...
    };
}
```

Snapshots are required, not a convenience. A drag payload's `IDataTransfer` is
only valid while the managed event is on the stack, and a picker result must
outlive the window that produced it. `uri()` is always populated;
`local_path()` is `None` whenever the platform has no filesystem path, so code
must never assume one exists.

## Pickers

```rust
let outcome = scope
    .open_file_picker(
        &window,
        &OpenFilePickerOptions::new()
            .title("Open files")
            .allow_multiple(true)
            .start_in(WellKnownFolder::Documents)
            .file_type(
                FileTypeFilter::new("Log files")
                    .with_extension("log")             // Windows, Linux, browser
                    .with_mime_type("text/plain")      // Linux, Android, browser
                    .with_apple_uniform_type_identifier("public.plain-text"),
            ),
    )?
    .await?;
```

`AppScope::open_folder_picker` and `AppScope::save_file_picker` follow the same
shape. `allow_multiple` on the folder picker is honoured only where the platform
folder picker supports multi-selection.

The save picker returns the same snapshot as the open pickers -- a name, a URI
and an optional local path. It deliberately does **not** hand back a managed
stream: writing stays in Rust, which keeps large exports and non-local targets
working.

### Cancellation is not an error

```rust
match operation.await {
    Ok(PickerOutcome::Cancelled) => { /* user dismissed the dialog */ }
    Ok(PickerOutcome::Selected(items)) => { /* ... */ }
    Err(error) => { /* a real failure, or E_ABORT for an aborted operation */ }
}
```

Three outcomes are kept distinct on purpose:

- **Cancelled** -- the dialog was dismissed. Success HRESULT, `Cancelled`
  outcome tag, empty item list.
- **Aborted** -- the future was dropped, the parent window closed, or the
  application exited while the dialog was open. `E_ABORT` through the existing
  async error mapping.
- **Failed** -- anything else, with the host's error text attached.

### Executor neutrality and lifetime

`StoragePickerOperation` is a plain `std::future::Future`, like
`AsyncOperation<T>` (see [ASYNC.md](ASYNC.md)). It stores its single completion
under a mutex, wakes the registered waker, and never assumes a runtime.
Completion crosses the ABI exactly once.

Pickers are tied to a `Window`, and both the storage provider and a copy of the
options are resolved on the UI thread when the operation starts, so a consumer
that keeps mutating its options builder cannot change an in-flight dialog. The
operation is then bounded by three things, all of which resolve the consumer's
future exactly once with `E_ABORT`:

- **Dropping the pending future** cancels through the same operation registry
  the rest of the async ABI uses.
- **Closing the parent window** cancels the operation. A dialog whose owner is
  gone is a lifetime event, not a user choice, which is why it aborts rather
  than reporting the `Cancelled` outcome.
- **Application exit** -- `Shutdown`, or simply the last window closing --
  aborts everything still pending. Teardown delivers those completions itself,
  because the dispatcher continuation that normally carries a result can no
  longer run once the application loop has stopped.

A storage provider's picker task has no cancellation token of its own: it
completes when the dialog closes and not before. The host therefore races that
task against the cancellation token instead of waiting for it, so none of the
three cases above can leave a consumer waiting on a dialog that may never be
dismissed. The abandoned dialog task is explicitly observed, and the Rust
completion object stays alive until the host releases it, so a late result
never touches freed memory and never produces a second completion.

`AppScope::mount` tracks its windows (`AppScope::windows`, `main_window`) so a
Rust-owned view model, which owns state but not presentation, can still parent a
picker to the window the host created for it.

## Incoming drag and drop

```rust
scope.on_file_drop(&window, DragDropEffects::COPY, |event| match event {
    FileDropEvent::Enter { items, .. } => { /* ... */ }
    FileDropEvent::Over { .. } => {}
    FileDropEvent::Leave => {}
    FileDropEvent::Drop { items, .. } => { /* ... */ }
})?;
```

**Effect negotiation is deliberately not a Rust callback.** Calling an external
consumer from inside a platform drag loop would let arbitrary Rust code stall
the compositor (and, on Windows, run inside an OLE modal loop), so the ABI does
not offer a synchronous "what effect do you want?" call. Instead:

1. The subscriber declares one conservative accepted-effect mask at subscription
   time.
2. While the managed event is still on the stack, the host answers the platform
   with `allowed & accepted`, and refuses outright (`None`) when the payload
   carries no file/folder items.
3. The notification -- kind, allowed effects, effective effect, and the
   snapshotted item list -- is then posted to the dispatcher.

The payload is snapshotted once per drag and reused, because `DragOver` fires
continuously while the pointer moves; re-materializing it per notification would
allocate a snapshot (and re-probe a local path) for every file, many times a
second, with the platform drag loop still on the stack.

Posting is what keeps the drag loop responsive regardless of what the consumer
does, and the dispatcher's FIFO ordering at one priority preserves
enter/over/leave/drop order. A consumer that returns a failure HRESULT is
recorded and ignored; it cannot tear down the UI thread. A notification already
queued when the subscription is removed (or when the application exits) is
dropped rather than delivered, so unsubscribing really does stop delivery.

Consumers that need finer-grained accept/reject can subscribe on a narrower
control, or use separate subscriptions with different masks on different targets.
A subscription on a window accepts drops anywhere inside it, which is what the
flagship sample does: Rust owns state rather than presentation, so it has no
handle to a control declared inside the compiled AXAML.

## Startup and "open with"

A Rust executable owns `argv`, so the managed host cannot see it. `App::run`
forwards this process's own arguments (minus the executable name) to the desktop
lifetime by default; `App::with_startup_arguments` and
`App::without_startup_arguments` override that. Arguments are read with
`args_os` and converted lossily, because a Unix filename is an arbitrary byte
sequence and a document path is exactly where a non-Unicode argument shows up --
losing part of one path is a far better outcome than panicking at launch.

```rust
let arguments = scope.startup_arguments()?;   // verbatim, ordered
let documents = scope.activation_items()?;    // normalized, de-duplicated
```

Normalization rules, all platform neutral:

- Arguments beginning with `-` are treated as option switches and skipped.
  `/` is **not** a switch prefix, because it introduces every absolute path on
  Unix.
- An argument with an explicit URI scheme is kept verbatim. Non-`file` schemes
  keep their URI and report no local path. A bare `C:\dir\file` is *not* treated
  as a URI even though it parses as one on every OS, so paths take one code path
  everywhere.
- Everything else is made absolute against the current directory.
- Duplicates are dropped by URI; first-seen order is preserved.

`AppScope::on_activation` receives later activations where the desktop lifetime
supports them (macOS "open with" while already running, protocol activation,
dock reopen). Where the platform has no activation feature the subscription
stays valid and simply never fires, so consumers need no platform branches.

| Platform | Startup documents | Later activation |
| --- | --- | --- |
| Windows (Win32) | command line | not raised |
| Linux (X11) | command line | not raised |
| macOS (Avalonia.Native) | command line | `ActivationEvent::Files` / `OpenUri` / `Reopen` |

## File type associations

Registering a document type is packaging metadata, not application code. The
external application template ships copyable snippets in
[`templates/avalonia-app/file-associations`](templates/avalonia-app/file-associations):
a Windows `.reg` file, a Linux `.desktop` entry plus a shared-mime-info
declaration, and a macOS `Info.plist` fragment with `CFBundleDocumentTypes` and
`UTExportedTypeDeclarations`.

MSIX packaging and platform installers are explicitly out of scope for this
stage. The runtime half of "open with" is already wired: once an association
launches the executable with a path, that path arrives through
`AppScope::activation_items()`.

## Tests

| Area | Where |
| --- | --- |
| Picker flows through a fake `IStorageProvider` (open, folder, save, cancel, failure, filters, start location, capabilities) | `tests/Avalonia.Host.Tests/Desktop/DesktopFilePickerTests.cs` |
| Window-scoped lifetime: closing the parent window aborts a pending picker, and the observer is detached afterwards | `tests/Avalonia.Host.Tests/Desktop/DesktopWindowLifetimeTests.cs` |
| Activation argument normalization, de-duplication, URI preservation, non-local items | `tests/Avalonia.Host.Tests/Desktop/StorageActivationTests.cs` |
| Effect negotiation and payload capture | `tests/Avalonia.Host.Tests/Desktop/DesktopFileDropTests.cs` |
| Routed-event wiring, synchronous effect vs. asynchronous delivery, ordering, unsubscribe | `tests/Avalonia.Host.Tests/Desktop/DesktopFileDropRegistryTests.cs` |
| ABI surface: option marshalling, item lists, completion exactly once, cancel/abort/error | `tests/Avalonia.Host.Tests/Desktop/DesktopFileAbiTests.cs` |
| Raw nano-COM vtable conformance against the published host, including null-pointer rejection for every picker start | `rust/avalonia-sys/tests/nativeaot_desktop_files.rs` |
| Safe API decoding, outcomes, effect masks, event kinds | `rust/avalonia/src/storage.rs` unit tests |
| End-to-end capability discovery, subscriptions, activation, thread-affinity error mapping | `rust/avalonia/tests/desktop_files.rs`, `rust/avalonia/tests/desktop_files_without_arguments.rs` |

The flagship sample stays automation safe: the only thing that opens a platform
dialog is an explicit button click, so UI automation can drive the drop panel,
the activation status and every other surface without a modal dialog appearing.

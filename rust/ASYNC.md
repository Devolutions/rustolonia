# Asynchronous interop

The async ABI is executor-neutral. Avalonia starts an operation and retains a
Rust COM completion object. Completion crosses the ABI once with:

- operation ID
- HRESULT
- tagged primitive value
- copied UTF-16 string value
- copied error text

`AsyncOperation<T>` implements `std::future::Future`; it does not depend on
Tokio or another runtime. Completion stores the result under a mutex, removes
the registered waker, and calls `wake`. The caller's executor decides where
the continuation is polled.

Dropping a pending future calls `CancelAsyncOperation`. Managed operations own
a `CancellationTokenSource`, complete with `E_ABORT`, and remove their registry
entry exactly once. A native completion failure is captured but never causes a
second completion attempt. Consuming a result keeps the operation completed:
dropping it does not request cancellation, and duplicate completions remain
rejected.

`AppScope::spawn` is a small optional executor for UI work. It schedules polls
through `IAvnDispatcher`, retains only active tasks, and releases completed task
entries as soon as their polls finish. Scope cleanup closes the task registry
and drops pending futures outside its locks. External executors can poll
`AsyncOperation<T>` directly instead.

Clipboard text read/write is the first real consumer. On Windows the host
initializes OLE around the Avalonia application lifetime because a Rust
executable has no managed `[STAThread]` entry point.

Clipboard commands extend that first consumer without touching the
frozen text methods: clearing and multi-format writes complete through the same
`IAvnAsyncCompletion`, while reading file entries reuses
`IAvnStorageCompletion` because the result is a list of storage items. They stay
asynchronous for a concrete reason: a platform clipboard read can block for as
long as the owning application takes to render the requested format, so a
synchronous clipboard API would be a UI-thread hazard. See
[MENUS.md](MENUS.md#clipboard).

Storage pickers are the second. They reuse the same operation
registry (so `CancelAsyncOperation` and shutdown cancellation behave
identically) but complete through their own separately versioned
`IAvnStorageCompletion` interface, because a picker result is a list of storage
items rather than one tagged primitive. A dismissed dialog is reported as a
successful completion with a `Cancelled` outcome tag, which keeps "the user
said no" distinguishable from `E_ABORT` and from a real failure. See
[DESKTOP_FILES.md](DESKTOP_FILES.md).

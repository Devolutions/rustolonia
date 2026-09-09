# Rust interop ownership

The first prototype used .NET source-generated `ComWrappers` CCWs for every
projected Avalonia object. Rust `ComPtr::drop` called `Release`, but managed
code received no synchronous notification when the final native reference was
released. That prevented deterministic identity retirement and subscription
cleanup.

Two replacement models were prototyped against the same diagnostics:

- A generation-counted handle table with explicit strong roots, retain/release,
  active-call leases, stale-generation rejection, and deferred cleanup.
- MicroCom shadows using `IMicroComShadowContainer` and
  `OnUnreferencedFromNative`, with an additional managed active-call lease.

MicroCom was selected for migration because it:

- Preserves the existing COM vtable and Rust `ComPtr` architecture.
- Reports the zero-native-reference transition synchronously.
- Generates AOT-safe unmanaged function-pointer vtables.
- Executes successfully from Rust through the NativeAOT shared library.
- Reuses Avalonia's established native interop infrastructure.

The handle-table fixture remains as comparison evidence and as a fallback if a
later projected feature cannot be represented safely by MicroCom.

Generated `ComWrappers` CCWs now act only as dispatch shells. Every projected
object state owns a canonical MicroCom lifetime token, and every Rust
`ComPtr<T>` returned for a projected object retains that token. Releasing the
last Rust pointer retires the object ID, weak identity entry, managed root, and
host-owned subscriptions immediately without waiting for GC.

## Contract

Final native `Release` immediately retires Rust ownership. Managed cleanup is
scheduled separately when thread affinity requires it. Releasing Rust
ownership does not remove a control from its Avalonia parent or force the
managed control to be garbage collected.

Every generated method must hold an active-call lease. If the final native
release occurs from a reentrant callback, shadow disposal and subscription
cleanup wait until that lease exits.

`subscribe_*` returns an RAII ownership object and unsubscribes when dropped.
Builder-style `on_*` requires an explicit `&AppScope` and transfers that guard
to scope-owned state so handlers remain active after temporary builder values
move into the managed tree.

Top-level windows are mounted with `scope.mount(window)`. Unlike child
controls, a shown Window cannot rely on a parent control to provide its managed
root. `AppScope` clears subscriptions first and mounted objects second when the
application lifetime exits.

Rust application startup runs from
`ClassicDesktopStyleApplicationLifetime.Startup`, after Avalonia installs its
global window-open/window-close tracking. Windows shown before that point are
not visible to `OnLastWindowClose` accounting and must never be created by the
host bootstrap.

The authoritative ABI is generated at
`avalonia-sys/include/avalonia-rust-abi.h`. Interface ABI versions and IIDs are
independent from the projection IR schema version.

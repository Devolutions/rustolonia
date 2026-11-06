# Agent instructions

Read [CONTRIBUTING.md](CONTRIBUTING.md) for setup, validation and the
source-of-truth rules. The rules below are the ones agents most often get wrong.

## COM interfaces are unversioned

The host (`Avalonia.Host`) and the Rust crates (`rustolonia`, `rustolonia-sys`,
`rustolonia-bindgen`) always ship together from one revision, so no interface
has to stay compatible with an older build of the other side.

- Change an interface in place: add, remove or reorder members on the existing
  interface, and update the C# interface, the Rust vtable and wrapper, and every
  implementation in the same change.
- Never create a numbered successor (`IAvnApplication2`, `IAvnRustVmSink6`,
  `IFoo2`), never change an IID to mark a change, and never put a version in an
  interface name, an IID seed or the projection IR. `UnversionedInterfaceTests`
  rejects interface names that end in a digit.
- A new IID is only for a genuinely new interface with its own descriptive name.
- For functionality an implementation may lack, return `E_NOTIMPL` or report
  capability flags on the same interface (see `IAvnRustVmSink.GetCapabilities`).
- Don't describe changes as "moves from version N to N+1" or "republishes
  under a fresh IID" in docs, comments or test names.

The full policy is in
[rust/COMPATIBILITY.md](rust/COMPATIBILITY.md#native-abi).

## Generated code

- Edit projection policy, schemas and generators, never generated outputs such
  as `host/Generated/`, `rust/rustolonia-sys/src/generated.rs`,
  `rust/rustolonia/src/generated.rs` or
  `rust/rustolonia-sys/include/avalonia-rust-abi.h`.
- Regenerate with
  `pwsh ./rust/regenerate-and-build.ps1 -Configuration Release -SkipManagedBuild`.
  CI reruns it and fails on any diff.

## Validation

- The repository uses Microsoft.Testing.Platform, so run a test project with
  `dotnet test --project tests/<Project> -c Release`.
- Rust, from `rust/`: `cargo fmt --all`, then
  `cargo test -p <crate> --locked`.

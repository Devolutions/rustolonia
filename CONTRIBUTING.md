# Contributing to Rustolonia

Rustolonia owns the Rust bindings, projection tools, managed interop and NativeAOT
host. Avalonia itself is a pinned producer dependency, not another copy of our
source tree.

## Prepare a checkout

Use Git with recursive submodules, PowerShell 7, the .NET SDK selected by
`global.json`, and a stable Rust toolchain with `rustfmt`. NativeAOT publishing also
needs the platform's native compiler/linker; see `rust/PLATFORMS.md` and
`rust/PRODUCTIZATION.md` for platform details.

```powershell
git submodule update --init --recursive
pwsh .\avalonia-patches\apply-avalonia-patches.ps1 -AvaloniaRoot .\avalonia-src
pwsh .\rust\regenerate-and-build.ps1 -Configuration Release
```

The Rust build entry points also initialize the in-repository producer at the
revision recorded by the checkout, so a fresh or reused checkout cannot build
against a different `avalonia-src` revision.

Open `Rustolonia.slnx` to navigate the eleven Rustolonia-owned managed projects.
Producer projects remain transitive project references, with their own build
configuration. When adding a project to the solution, pass
`--include-references false` so platform-specific producer projects do not become
unconditional solution entries.

## Choose the smallest relevant validation

Quick script, package-helper and scaffold cases do not compile native code:

```powershell
pwsh .\rust\tests\test-build-app.ps1
```

Managed tests and the Rust generator can run independently:

```powershell
dotnet test .\Rustolonia.slnx -c Release
cargo test --manifest-path .\rust\Cargo.toml -p avalonia-bindgen --locked
cargo check --manifest-path .\rust\Cargo.toml -p avalonia --examples --locked
```

For native integration, `rust/build.ps1` publishes a host and runs the Rust
workspace against it. The explicit consumer smoke flow builds an application
outside the repository and launches its adjacent bundle:

```powershell
pwsh .\rust\build.ps1
pwsh .\rust\tests\test-build-app.ps1 -RunNativeSmoke
```

Native execution needs a matching OS/architecture and a desktop display. On a
headless Linux runner, use Xvfb for the consumer smoke flow. Cross-compilation
alone does not establish native runtime coverage.

## Change the source of truth

Edit projection policy, schemas and generators rather than generated C#, Rust
or native headers. Regenerate related outputs together, preserve canonical LF
line endings, and inspect the resulting diff for unintended API changes.

Each generator records its files and content hashes in a checked-in
`.*.owned.json` sidecar. Keep these manifests with their outputs. Regeneration
prunes only previously owned, unmodified obsolete files and refuses conflicting
claims from another generator. Do not manually delete or rewrite the manifests
to bypass a conflict.

The two managed projection tools and `avalonia-bindgen` accept `--check` before
their normal positional arguments. This compares outputs without creating or
rewriting them. `--check --normalize` is rejected rather than silently modifying
the view-model input.

Published IIDs, vtable order, calling conventions and ownership semantics must
not change under an existing identity. Follow `rust/COMPATIBILITY.md`; preserve
regression coverage when reorganizing old wave-named tests.

Application-specific schema, presentation and generated registry changes must
remain compatible with the explicit external-consumer path. Exercise both the
existing samples and an external consumer when changing that boundary.

## Keep producer and generated artifacts contained

Do not commit temporary compiler outputs, instantiated consumer applications,
or package bundles. Do not edit an unrelated checkout to make a local build
work. Producer changes belong in the documented patch set and upstreaming
tracker; keep the submodule revision unchanged unless intentionally updating the
supported producer contract.

The crates remain source-pinned and `publish = false`. Changing publication,
versioning or delivery policy requires a matching host/ABI compatibility plan
and dependency/license provenance, not just a manifest toggle.

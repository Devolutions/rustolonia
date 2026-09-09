# avalonia

Idiomatic Rust API for Avalonia, generated on top of `avalonia-sys` from the
same `projection.ir.json` that generates the managed C# COM wrappers.
Application view-model schemas are owned by the consuming crate — the flagship
sample uses [`../avalonia-sample`](../avalonia-sample) — not by this generic
API crate.

This crate is source-only: `publish = false` because it only works against a
matching `Avalonia.Host` NativeAOT build produced from the same checkout, not
against a versioned ABI contract suitable for crates.io. Copy
[`../templates/avalonia-app`](../templates/avalonia-app) (or run
`../new-app.ps1`) to start a new application against this
crate.

See [`../PRODUCTIZATION.md`](../PRODUCTIZATION.md) for the productized
workflow (templates, one-command build, host discovery, packaging,
checksums, and SBOM scope), and [`../README.md`](../README.md) for examples.

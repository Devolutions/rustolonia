# rustolonia

Idiomatic Rust API for building desktop GUI applications with
[Avalonia](https://avaloniaui.net/), backed by a prebuilt .NET NativeAOT host.

```toml
[dependencies]
rustolonia = "0.1"
```

On the first build, `rustolonia-sys` downloads the prebuilt
`rustolonia_host` library for your target from the matching
[GitHub release](https://github.com/Devolutions/rustolonia/releases),
checks it against the SHA-256 baked into the crate, and caches it. No .NET SDK
is needed. Supported targets: Windows, macOS, and Linux (glibc) on x64 and
arm64. To use a locally built host, offline builds, or a different mirror, see
the `rustolonia-sys` README (`RUSTOLONIA_HOST_DIR`, `RUSTOLONIA_NO_DOWNLOAD`,
`RUSTOLONIA_CACHE_DIR`, `RUSTOLONIA_HOST_BASE_URL`).

At runtime the host is found in this order: `RUSTOLONIA_HOST_LIB`, then next to
the executable (or `../Frameworks` in a macOS bundle), and then the build-time
host directory when the default `dev-host-path` feature is enabled. When you
ship an application, copy the host directory next to your executable and build
with `default-features = false` so that no build machine paths end up in the
binary.

The Rust and native host versions are locked to each other:
`rustolonia 0.1.x` only works with the `0.1.x` host release built from the same
tag. Both sides check the ABI fingerprint when the app starts.

Application view-model schemas are owned by the consuming crate. In this
repository the flagship sample is [`../avalonia-sample`](../avalonia-sample),
and [`../templates/avalonia-app`](../templates/avalonia-app) (or
`../new-app.ps1`) is a starting point for new applications.

See [`../PRODUCTIZATION.md`](../PRODUCTIZATION.md) for the productized
workflow (templates, one-command build, host discovery, packaging, checksums,
and SBOM scope).
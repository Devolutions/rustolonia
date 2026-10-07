# rustolonia-sys

Raw nano-COM bindings for the Avalonia NativeAOT host (`Avalonia.Host`).

This crate is handwritten plus IR-generated vtables/GUIDs (see
`../projection.ir.json` and `rustolonia-bindgen`), and is consumed almost
exclusively through the safe `rustolonia` crate. It is published to crates.io
in lockstep with the prebuilt `rustolonia_host` release assets of the same
version (see "Prebuilt host" below). The crate and host must match exactly;
`rustolonia` pins this crate with `=x.y.z`.

See [`../PRODUCTIZATION.md`](../PRODUCTIZATION.md) and
[`../README.md`](../README.md) for the full workflow, and
[`../OWNERSHIP.md`](../OWNERSHIP.md) for the ownership contract this crate
implements.

## Host lifetime

`Host::load` resolves every required export before publishing the process-wide
UTF-16 allocation callbacks. Successful host libraries intentionally remain
loaded for the process lifetime because returned ABI strings can outlive a
`Host` value. A later load whose `avn_free` or `avn_alloc_utf16` exports differ
is rejected, preserving the single allocator-host invariant.

## Prebuilt host

`build.rs` stages the NativeAOT host (`rustolonia_host.dll`,
`librustolonia_host.dylib` or `librustolonia_host.so`) for the target, in this
order:

1. `RUSTOLONIA_HOST_DIR`: use this directory as-is (local builds, CI,
   vendored or distro-packaged hosts).
2. Skip staging when building on docs.rs, with the `no-download` feature, or
   with `RUSTOLONIA_NO_DOWNLOAD` set to a truthy value. The application must
   then ship the host itself.
3. The cache: `RUSTOLONIA_CACHE_DIR`, or `<user cache dir>/rustolonia/host`,
   keyed by `<version>/<rid>`.
4. Download `rustolonia-host-<version>-<rid>.tar.gz` from the GitHub release
   (`RUSTOLONIA_HOST_BASE_URL` overrides the base URL; `file://` works), verify
   it against `host-checksums.txt` (or the file named by
   `RUSTOLONIA_HOST_CHECKSUMS`), then cache it. `CARGO_NET_OFFLINE` forbids the
   download. Proxies come from the standard proxy environment variables.

Prebuilt hosts exist for `x86_64`/`aarch64` `windows-msvc`, `apple-darwin` and
`linux-gnu`. The staged directory is exported to dependent build scripts as
`DEP_RUSTOLONIA_HOST_HOST_DIR`. With the default `dev-host-path` feature it is
also baked into the binary as a last-resort runtime search path, so `cargo run`
works without copying files; disable it for distributable builds.

Builds inside this repository default to `RUSTOLONIA_NO_DOWNLOAD=1` through
the repo-root `.cargo/config.toml`, since the matching release does not exist
yet.

## Host compatibility check

Before resolving any other export, `Host::load` calls `avn_get_host_info`,
which reports the host's release version and ABI fingerprint (the SHA-256 of
`include/avalonia-rust-abi.h` at host build time). `build.rs` hashes the same
header into `ABI_FINGERPRINT`. If the fingerprints differ, or the host is too
old to export `avn_get_host_info`, loading fails with
`HostLoadError::IncompatibleHost`. The version is reported in the error and
through `Host::info()`; only the fingerprint decides compatibility.

## String arguments

Safe wrappers accepting UTF-16 slices ensure that a NUL terminator exists
before passing their pointer to the host. Already terminated slices are
borrowed; others, including empty slices, are copied with a terminator.
Embedded NULs keep their ABI meaning of ending the string. Nullable arguments
still distinguish `None` from an empty string. Raw vtable calls remain unsafe
and require the caller to uphold the ABI string contract.

## Date and duration helpers

`AvnOptionalDateTime` converts signed offsets from the Unix epoch, including
pre-1970 dates, without overflowing an intermediate nanosecond count.
`AvnOptionalTimeSpan` likewise converts large nonnegative durations using
seconds and subsecond ticks. Both use .NET's 100ns resolution; finer precision
is truncated toward the Unix epoch for dates and toward zero for durations.

Use the `try_from_date_time`, `try_to_date_time`, `try_from_duration`, and
`try_to_duration` helpers to receive an error for values outside the .NET or
platform range, or a negative TimeSpan that Rust's `Duration` cannot represent.
The original infallible helpers remain available and panic on those errors
rather than silently changing the value.

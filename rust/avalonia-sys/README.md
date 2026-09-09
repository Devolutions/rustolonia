# avalonia-sys

Raw nano-COM bindings for the Avalonia NativeAOT host (`Avalonia.Host`).

This crate is handwritten plus IR-generated vtables/GUIDs (see
`../projection.ir.json` and `avalonia-bindgen`), and is consumed almost
exclusively through the safe `avalonia` crate. It is source-only: `publish =
false` because it is pinned to a matching `Avalonia.Host` build produced from
the same checkout, not to a versioned ABI contract suitable for crates.io.

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

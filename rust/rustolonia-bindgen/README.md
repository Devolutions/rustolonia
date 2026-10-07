# rustolonia-bindgen

Code generator for [rustolonia](https://github.com/Devolutions/rustolonia). It
reads the projection IR (`projection.ir.json`) exported from the patched
Avalonia host and emits the raw vtables and GUIDs in `rustolonia-sys` and the
safe wrappers in `rustolonia`.

Application authors do not need this crate; it is published so the generated
sources in the released crates can be reproduced.

```text
rustolonia-bindgen [--check] <projection.ir.json> <sys-output> [safe-output]
rustolonia-bindgen --write-baseline <projection.ir.json> <header> <baseline.json>
```

`--check` compares instead of writing and fails if the outputs are stale.

Licensed under MIT.

# android-hello

A code-first Rustolonia app that runs as an Android APK and on the desktop
from one UI module. It is the reference layout for Android apps; the design,
window mapping and limits are in
[rust/PLATFORMS.md](../../rust/PLATFORMS.md#android).

| File | Role |
| --- | --- |
| `src/ui.rs` | The UI, shared by both entry points |
| `src/lib.rs` | Android entry: `rustolonia::android_main!(ui::build)` |
| `src/main.rs` | Desktop entry: `App::load_from_env()?.run(ui::build)` |
| `android/AndroidHello.csproj` | APK project: references `host/Android`, packages `libandroid_hello.so` per ABI |
| `android/MainActivity.cs` | `[Application]` and launcher `[Activity]` subclasses |
| `Cargo.toml` `[package.metadata.rustolonia.android]` | Package name and label for `cargo rustolonia-apk` |

## Android, Cargo only

No .NET tooling is needed: `cargo rustolonia-apk` downloads the prebuilt host
APK of the release matching the app's `rustolonia-sys` and verifies it against
that crate's `host-checksums.txt`:

```pwsh
cargo install --path ../../rust/rustolonia-apk
cargo rustolonia-apk --abi x86_64 --launch
```

Until a release includes the Android host, build one in this checkout with
`pwsh ./rust/package-android-host.ps1` and pass it with
`--host-apk ../../rust/target/android-host/rustolonia-host-<version>-android.apk`.
The tool reads `[package.metadata.rustolonia.android]` from `Cargo.toml`.

## Android, with an APK project

Prerequisites: the .NET `android` workload, an Android SDK with an NDK, and
`rustup target add aarch64-linux-android x86_64-linux-android`. From the
repository root, with an emulator or device connected:

```pwsh
pwsh ./rust/build-android.ps1 -Abi x86_64 -Install -Launch
```

Omit `-Abi` to build both `arm64-v8a` and `x86_64`. Logs:
`adb logcat -s rustolonia`.

## Desktop

Build a host from this checkout (`pwsh ./rust/build.ps1`), point
`RUSTOLONIA_HOST_LIB` at it, then run `cargo run` here.

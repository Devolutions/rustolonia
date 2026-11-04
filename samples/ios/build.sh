#!/usr/bin/env bash
set -euo pipefail

sample_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$sample_dir/../.." && pwd)"
mode="${1:-simulator}"
if [ "$#" -gt 0 ]; then
    shift
fi

case "$mode" in
    simulator)
        case "$(uname -m)" in
            arm64) rust_target=aarch64-apple-ios-sim; rid=iossimulator-arm64 ;;
            x86_64) rust_target=x86_64-apple-ios; rid=iossimulator-x64 ;;
            *) echo "Unsupported simulator host architecture: $(uname -m)" >&2; exit 1 ;;
        esac
        ;;
    simulator-arm64) rust_target=aarch64-apple-ios-sim; rid=iossimulator-arm64 ;;
    simulator-x64) rust_target=x86_64-apple-ios; rid=iossimulator-x64 ;;
    device) rust_target=aarch64-apple-ios; rid=ios-arm64 ;;
    *) echo "Usage: $0 [simulator|simulator-arm64|simulator-x64|device] [dotnet options...]" >&2; exit 1 ;;
esac

if ! rustup target list --installed | grep -Fxq "$rust_target"; then
    echo "Missing Rust target $rust_target; install it with: rustup target add $rust_target" >&2
    exit 1
fi
if [ ! -f "$repo_root/avalonia-src/NuGet.Config" ]; then
    echo "Initialize the pinned Avalonia producer and apply its patches first; see samples/ios/README.md." >&2
    exit 1
fi

export IPHONEOS_DEPLOYMENT_TARGET=13.0
build_rust() {
    cargo build --manifest-path "$sample_dir/rust/Cargo.toml" --locked \
        --target "$rust_target" --target-dir "$sample_dir/target" --release "$@"
}
if [ "${RUSTOLONIA_IOS_SMOKE:-0}" = 1 ]; then
    build_rust --features smoke
else
    build_rust
fi
rust_library="$sample_dir/target/$rust_target/release/librustolonia_ios_example.a"
project="$sample_dir/managed/Rustolonia.IosSample.csproj"

if [ "$mode" = device ]; then
    dotnet publish "$project" -c Release -r "$rid" \
        "-p:RustoloniaRustLibrary=$rust_library" \
        "-p:RestoreConfigFile=$repo_root/avalonia-src/NuGet.Config" \
        -p:BuildIpa=false "$@"
else
    # The iOS SDK uses Mono for a normal build, but permits NativeAOT simulator
    # builds with _IsPublishing=true. `dotnet publish` rejects simulator RIDs.
    dotnet build "$project" -c Release -r "$rid" -p:_IsPublishing=true \
        "-p:RustoloniaRustLibrary=$rust_library" \
        "-p:RestoreConfigFile=$repo_root/avalonia-src/NuGet.Config" "$@"
fi

printf 'iOS app: %s\n' "$sample_dir/managed/bin/Release/net10.0-ios/$rid/Rustolonia.IosSample.app"

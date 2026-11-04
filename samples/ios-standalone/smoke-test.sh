#!/usr/bin/env bash
set -euo pipefail

sample_dir="$(cd "$(dirname "$0")" && pwd -P)"
case "$(uname -m)" in
    arm64) target=aarch64-apple-ios-sim ;;
    x86_64) target=x86_64-apple-ios ;;
    *) echo "Unsupported simulator host architecture: $(uname -m)" >&2; exit 1 ;;
esac
RUSTOLONIA_IOS_SMOKE=1 "$sample_dir/build.sh" simulator
"$sample_dir/../ios/run-smoke-app.sh" \
    "$sample_dir/target/$target/release/RustoloniaStandalone.app" \
    org.rustolonia.standalone rustolonia-standalone-smoke.txt \
    "$sample_dir/target/ios-standalone-smoke.png" "$sample_dir/check-render.sh"

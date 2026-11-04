#!/usr/bin/env bash
set -euo pipefail

sample_dir="$(cd "$(dirname "$0")" && pwd)"

RUSTOLONIA_IOS_SMOKE=1 "$sample_dir/build.sh" simulator -p:RustoloniaIosSmoke=true "$@"
case "$(uname -m)" in
    arm64) rid=iossimulator-arm64 ;;
    x86_64) rid=iossimulator-x64 ;;
    *) echo "Unsupported simulator host architecture: $(uname -m)" >&2; exit 1 ;;
esac
app="$sample_dir/managed/bin/Release/net10.0-ios/$rid/Rustolonia.IosSample.app"
"$sample_dir/run-smoke-app.sh" "$app" org.rustolonia.iossample \
    rustolonia-ios-smoke.txt "$sample_dir/target/ios-smoke.png"

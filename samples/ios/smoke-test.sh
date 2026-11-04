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
bundle_id=org.rustolonia.iossample
simulator="${RUSTOLONIA_IOS_SIMULATOR_UDID:-}"
created_simulator=

cleanup() {
    local status=$?
    trap - EXIT
    if [ -n "$created_simulator" ]; then
        if ! xcrun simctl shutdown "$created_simulator"; then
            echo "Could not shut down test simulator $created_simulator" >&2
            status=1
        fi
        if ! xcrun simctl delete "$created_simulator"; then
            echo "Could not delete test simulator $created_simulator" >&2
            status=1
        fi
    fi
    if [ "$status" -eq 0 ]; then
        echo "iOS simulator smoke test passed."
    fi
    exit "$status"
}
trap cleanup EXIT

if [ -z "$simulator" ]; then
    runtime="$(xcrun simctl list runtimes --json | python3 -c '
import json, sys
runtimes = [r for r in json.load(sys.stdin)["runtimes"]
            if r["identifier"].startswith("com.apple.CoreSimulator.SimRuntime.iOS-") and r["isAvailable"]]
if not runtimes:
    raise SystemExit("No iOS simulator runtime is installed")
print(max(runtimes, key=lambda r: tuple(int(x) for x in r["version"].split(".")))["identifier"])
')"
    device_type="$(xcrun simctl list devicetypes --json | python3 -c '
import json, sys
devices = [d for d in json.load(sys.stdin)["devicetypes"] if d["name"].startswith("iPhone ")]
if not devices:
    raise SystemExit("No iPhone simulator device type is installed")
print(devices[0]["identifier"])
')"
    simulator="$(xcrun simctl create "Rustolonia iOS smoke" "$device_type" "$runtime")"
    created_simulator="$simulator"
    xcrun simctl boot "$simulator"
fi

xcrun simctl bootstatus "$simulator" -b
xcrun simctl install "$simulator" "$app"
data_dir="$(xcrun simctl get_app_container "$simulator" "$bundle_id" data)"
result="$data_dir/Documents/rustolonia-ios-smoke.txt"
if [ -f "$result" ]; then
    rm "$result"
fi
xcrun simctl launch "$simulator" "$bundle_id"

for _ in $(seq 1 60); do
    if [ -f "$result" ]; then
        break
    fi
    sleep 1
done
if [ ! -f "$result" ]; then
    echo "The iOS application did not report a smoke-test result." >&2
    xcrun simctl spawn "$simulator" log show --last 3m --style compact \
        --predicate 'process == "Rustolonia.IosSample"' | tail -80
    exit 1
fi
if [ "$(cat "$result")" != "ok" ]; then
    cat "$result" >&2
    exit 1
fi
xcrun simctl io "$simulator" screenshot "$sample_dir/target/ios-smoke.png"
xcrun simctl io "$simulator" screenshot "$sample_dir/target/ios-smoke.png"

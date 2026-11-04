#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 4 ] || [ "$#" -gt 5 ]; then
    echo "Usage: $0 <app> <bundle-id> <result-file-name> <screenshot> [render-check]" >&2
    exit 1
fi
app="$1"
bundle_id="$2"
result_name="$3"
screenshot="$4"
executable="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Info.plist")"
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
result="$data_dir/Documents/$result_name"
if [ -f "$result" ]; then
    rm "$result"
fi
xcrun simctl launch --terminate-running-process "$simulator" "$bundle_id"

for _ in $(seq 1 60); do
    if [ -f "$result" ]; then
        break
    fi
    sleep 1
done
if [ ! -f "$result" ]; then
    echo "The iOS application did not report a smoke-test result." >&2
    xcrun simctl spawn "$simulator" log show --last 3m --style compact \
        --predicate "process == \"$executable\"" | tail -80
    exit 1
fi
if [ "$(cat "$result")" != "ok" ]; then
    cat "$result" >&2
    exit 1
fi
if [ "$#" -eq 5 ]; then
    "$5" "$simulator" "$screenshot"
else
    xcrun simctl io "$simulator" screenshot "$screenshot"
fi

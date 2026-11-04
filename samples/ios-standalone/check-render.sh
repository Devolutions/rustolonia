#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <simulator-udid> <screenshot>" >&2
    exit 1
fi
simulator="$1"
screenshot="$2"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/rustolonia-ios-render.XXXXXXXX")"
bitmap="$scratch/screenshot.bmp"
trap 'rm -rf "$scratch"' EXIT

for _ in $(seq 1 30); do
    xcrun simctl io "$simulator" screenshot "$screenshot" >/dev/null
    sips -s format bmp "$screenshot" --out "$bitmap" >/dev/null
    if python3 - "$bitmap" <<'PY'
import struct
import sys

with open(sys.argv[1], "rb") as source:
    data = source.read()
if data[:2] != b"BM" or len(data) < 54:
    raise SystemExit("Invalid simulator screenshot bitmap.")
offset = struct.unpack_from("<I", data, 10)[0]
width, height, planes, bits = struct.unpack_from("<iiHH", data, 18)
if width <= 0 or height == 0 or planes != 1 or bits != 32:
    raise SystemExit("Unexpected simulator screenshot bitmap format.")
height = abs(height)
if len(data) < offset + width * height * 4:
    raise SystemExit("Incomplete simulator screenshot bitmap.")
light = dark = total = 0
for y in range(int(height * 0.35), int(height * 0.65), 4):
    for x in range(int(width * 0.05), int(width * 0.95), 4):
        blue, green, red, _ = data[offset + (y * width + x) * 4:offset + (y * width + x) * 4 + 4]
        light += min(red, green, blue) >= 230
        dark += max(red, green, blue) <= 55
        total += 1
if total == 0:
    raise SystemExit("Simulator screenshot is too small to inspect.")
if not (light / total > 0.005 and dark / total > 0.005
        and (light / total > 0.6 or dark / total > 0.6)):
    raise SystemExit(1)
PY
    then
        echo "Standalone iOS view rendered: $screenshot"
        exit 0
    fi
    sleep 1
done
echo "The standalone iOS view did not render; screenshot: $screenshot" >&2
exit 1

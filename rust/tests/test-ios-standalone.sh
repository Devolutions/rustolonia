#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <host-archive> <simulator-arm64|simulator-x64|device>" >&2
    exit 1
fi
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
archive="$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")"
mode="$2"
case "$mode" in
    simulator-arm64) rid=iossimulator-arm64; target=aarch64-apple-ios-sim ;;
    simulator-x64) rid=iossimulator-x64; target=x86_64-apple-ios ;;
    device) rid=ios-arm64; target=aarch64-apple-ios ;;
    *) echo "Unsupported iOS test mode: $mode" >&2; exit 1 ;;
esac
if [ ! -f "$archive" ]; then
    echo "Host archive does not exist: $archive" >&2
    exit 1
fi
version="$(python3 - "$root/rust/release-manifest.json" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as source:
    print(json.load(source)["rustoloniaVersion"])
PY
)"
asset="rustolonia-host-$version-$rid.tar.gz"
if [ "$(basename "$archive")" != "$asset" ]; then
    echo "Expected the $asset archive." >&2
    exit 1
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/rustolonia-ios-consumer.XXXXXXXX")"
trap 'rm -rf "$scratch"' EXIT
work="$scratch/external consumer"
mkdir -p "$work/crates" "$work/app/src"
cargo package --locked --allow-dirty --no-verify --manifest-path "$root/rust/Cargo.toml" \
    -p rustolonia-sys -p rustolonia-macros -p rustolonia --target-dir "$work/package-target"
for crate in rustolonia-sys rustolonia-macros rustolonia; do
    tar -xzf "$work/package-target/package/$crate-$version.crate" -C "$work/crates"
done

checksums="$work/crates/rustolonia-sys-$version/host-checksums.txt"
actual_hash="$(shasum -a 256 "$archive")"
actual_hash="${actual_hash%% *}"
python3 - "$checksums" "$asset" "$actual_hash" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
lines = [line for line in path.read_text(encoding="utf-8").splitlines()
         if not line.rstrip().endswith("  " + sys.argv[2])]
lines.append(f"{sys.argv[3]}  {sys.argv[2]}")
path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
cp "$root/samples/ios-standalone/src/lib.rs" "$work/app/src/lib.rs"
cp "$root/samples/ios-standalone/Info.plist" "$work/app/Info.plist"
cp "$root/samples/ios-standalone/build.sh" "$work/app/build.sh"
cat > "$work/app/Cargo.toml" <<EOF
[package]
name = "rustolonia-ios-standalone"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib"]

[features]
smoke = []

[dependencies]
rustolonia = { version = "=$version", default-features = false }

[patch.crates-io]
rustolonia = { path = "../crates/rustolonia-$version" }
rustolonia-macros = { path = "../crates/rustolonia-macros-$version" }
rustolonia-sys = { path = "../crates/rustolonia-sys-$version" }

[workspace]
EOF
cargo generate-lockfile --offline --manifest-path "$work/app/Cargo.toml"
export RUSTOLONIA_IOS_HOST_ARCHIVE="$archive" RUSTOLONIA_IOS_SMOKE=1
unset RUSTOLONIA_IOS_HOST_SHA256

cp "$checksums" "$scratch/trusted-checksums.txt"
python3 - "$checksums" "$asset" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
lines = [line for line in text.splitlines() if not line.rstrip().endswith("  " + sys.argv[2])]
lines.append(f'{"0" * 64}  {sys.argv[2]}')
path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
if output="$(bash "$work/app/build.sh" "$mode" 2>&1)"; then
    echo "A host archive with the wrong pinned checksum was accepted." >&2
    exit 1
fi
if ! printf '%s\n' "$output" | grep -Fq 'SHA-256 mismatch'; then
    printf 'Expected a checksum failure, got:\n%s\n' "$output" >&2
    exit 1
fi
cp "$scratch/trusted-checksums.txt" "$checksums"

bash "$work/app/build.sh" "$mode"
if { [ "$mode" = simulator-arm64 ] && [ "$(uname -m)" = arm64 ]; } ||
    { [ "$mode" = simulator-x64 ] && [ "$(uname -m)" = x86_64 ]; }; then
    "$root/samples/ios/run-smoke-app.sh" \
        "$work/app/target/$target/release/RustoloniaStandalone.app" \
        org.rustolonia.standalone rustolonia-standalone-smoke.txt \
        "$scratch/standalone-smoke.png" "$root/samples/ios-standalone/check-render.sh"
fi
echo "External iOS consumer passed for $rid using the packaged crates and pinned host checksum."

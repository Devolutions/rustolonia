#!/usr/bin/env bash
set -euo pipefail

sample_dir="$(cd "$(dirname "$0")" && pwd -P)"
mode="${1:-simulator}"
if [ "$#" -gt 0 ]; then
    shift
fi
case "$mode" in
    simulator)
        case "$(uname -m)" in
            arm64) target=aarch64-apple-ios-sim; rid=iossimulator-arm64 ;;
            x86_64) target=x86_64-apple-ios; rid=iossimulator-x64 ;;
            *) echo "Unsupported simulator host architecture: $(uname -m)" >&2; exit 1 ;;
        esac
        ;;
    simulator-arm64) target=aarch64-apple-ios-sim; rid=iossimulator-arm64 ;;
    simulator-x64) target=x86_64-apple-ios; rid=iossimulator-x64 ;;
    device) target=aarch64-apple-ios; rid=ios-arm64 ;;
    *) echo "Usage: $0 [simulator|simulator-arm64|simulator-x64|device]" >&2; exit 1 ;;
esac
if [ "$#" -ne 0 ]; then
    echo "Unexpected arguments: $*" >&2
    exit 1
fi
if ! rustup target list --installed | grep -Fxq "$target"; then
    echo "Missing Rust target $target; install it with: rustup target add $target" >&2
    exit 1
fi

metadata="$(cargo metadata --locked --format-version 1 --manifest-path "$sample_dir/Cargo.toml")"
host_info="$(printf '%s' "$metadata" | python3 -c '
import hashlib, json, pathlib, sys
packages = [p for p in json.load(sys.stdin)["packages"] if p["name"] == "rustolonia-sys"]
if len(packages) != 1:
    raise SystemExit("Expected exactly one rustolonia-sys package in the Cargo dependency graph.")
package = packages[0]
root = pathlib.Path(package["manifest_path"]).parent
print(package["version"])
print(hashlib.sha256((root / "include" / "avalonia-rust-abi.h").read_bytes()).hexdigest())
print(root / "host-checksums.txt")
')"
version="${host_info%%$'\n'*}"
host_info="${host_info#*$'\n'}"
abi="${host_info%%$'\n'*}"
checksums="${host_info#*$'\n'}"
asset="rustolonia-host-$version-$rid.tar.gz"
expected_hash="${RUSTOLONIA_IOS_HOST_SHA256:-}"
if [ -z "$expected_hash" ]; then
    expected_hash="$(python3 - "$checksums" "$asset" <<'PY'
import re
import sys

matches = []
with open(sys.argv[1], encoding="utf-8") as source:
    for line in source:
        match = re.fullmatch(r"\s*([0-9a-f]{64})\s+\*?(\S+)\s*", line)
        if match and match.group(2) == sys.argv[2]:
            matches.append(match.group(1))
if len(matches) != 1:
    raise SystemExit(f"No unique trusted checksum for {sys.argv[2]} in {sys.argv[1]}. "
                     "Use a release that includes iOS hosts, or explicitly provide "
                     "RUSTOLONIA_IOS_HOST_ARCHIVE and RUSTOLONIA_IOS_HOST_SHA256 for a local build.")
print(matches[0])
PY
)"
fi
if [[ ! "$expected_hash" =~ ^[0-9a-f]{64}$ ]]; then
    echo "RUSTOLONIA_IOS_HOST_SHA256 must be a lowercase SHA-256 hash." >&2
    exit 1
fi

mkdir -p "$sample_dir/target"
stage="$(mktemp -d "$sample_dir/target/.ios-host-stage.XXXXXXXX")"
cleanup() {
    local status=$?
    trap - EXIT
    rm -rf "$stage"
    exit "$status"
}
trap cleanup EXIT

archive="${RUSTOLONIA_IOS_HOST_ARCHIVE:-}"
if [ -z "$archive" ]; then
    archive="$stage/$asset"
    url="https://github.com/Devolutions/rustolonia/releases/download/v$version/$asset"
    echo "Downloading $asset" >&2
    curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error "$url" --output "$archive"
fi
if [ ! -f "$archive" ]; then
    echo "The iOS host archive does not exist: $archive" >&2
    exit 1
fi
actual_hash="$(shasum -a 256 "$archive")"
actual_hash="${actual_hash%% *}"
if [ "$actual_hash" != "$expected_hash" ]; then
    echo "SHA-256 mismatch for $archive: expected $expected_hash, got $actual_hash" >&2
    exit 1
fi

host_dir="$stage/host"
mkdir "$host_dir"
python3 - "$archive" "$host_dir" <<'PY'
import os
from pathlib import Path, PurePosixPath
import shutil
import sys
import tarfile

root = Path(sys.argv[2])
with tarfile.open(sys.argv[1], "r:gz") as archive:
    for member in archive:
        relative = PurePosixPath(member.name)
        if (not member.isfile() or relative.is_absolute() or "\\" in member.name
                or any(part in {"", ".", ".."} for part in relative.parts)):
            raise SystemExit(f"Unsafe iOS host archive entry: {member.name!r}")
        destination = root.joinpath(*relative.parts)
        destination.parent.mkdir(parents=True, exist_ok=True)
        source = archive.extractfile(member)
        if source is None:
            raise SystemExit(f"Cannot extract iOS host archive entry: {member.name!r}")
        with source, destination.open("xb") as target:
            shutil.copyfileobj(source, target)
        os.chmod(destination, 0o755 if member.mode & 0o111 else 0o644)
PY

export IPHONEOS_DEPLOYMENT_TARGET=13.0
build_rust() {
    cargo build --locked --manifest-path "$sample_dir/Cargo.toml" --target "$target" \
        --target-dir "$sample_dir/target" --release "$@"
}
if [ "${RUSTOLONIA_IOS_SMOKE:-0}" = 1 ]; then
    build_rust --features smoke
else
    build_rust
fi
bash "$host_dir/build-app.sh" \
    "$sample_dir/target/$target/release/librustolonia_ios_standalone.a" \
    "$sample_dir/Info.plist" \
    "$sample_dir/target/$target/release/RustoloniaStandalone.app" \
    "$version" "$abi" "$rid"

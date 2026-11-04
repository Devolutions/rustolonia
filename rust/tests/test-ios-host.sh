#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd -P)"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/rustolonia-ios-host-test.XXXXXXXX")"
trap 'rm -rf "$scratch"' EXIT
host="$scratch/host"
mkdir "$host"
cp "$root/rust/ios/build-app.sh" "$host/build-app.sh"
printf 'host input\n' > "$host/input.txt"
: > "$scratch/libapp.a"
printf 'not a plist\n' > "$scratch/Info.plist"
abi="$(printf 'a%.0s' {1..64})"
other_abi="$(printf 'b%.0s' {1..64})"
python3 - "$host" "$abi" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
files = [
    {"name": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    for path in sorted(root.iterdir())
]
manifest = {
    "schemaVersion": 1,
    "kind": "rustolonia-ios-host",
    "version": "1.2.3",
    "rid": "iossimulator-arm64",
    "abiFingerprint": sys.argv[2],
    "files": files,
}
(root / "host-manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
PY

expect_failure() {
    local expected="$1"
    shift
    local output
    if output="$("$@" 2>&1)"; then
        echo "Expected failure containing: $expected" >&2
        exit 1
    fi
    if ! printf '%s\n' "$output" | grep -Fq "$expected"; then
        printf 'Expected failure containing: %s\nGot: %s\n' "$expected" "$output" >&2
        exit 1
    fi
}

app="$scratch/Test.app"
command=(bash "$host/build-app.sh" "$scratch/libapp.a" "$scratch/Info.plist" "$app")
expect_failure 'does not match rustolonia-sys' "${command[@]}" 9.9.9 "$abi" iossimulator-arm64
expect_failure 'ABI fingerprint does not match' "${command[@]}" 1.2.3 "$other_abi" iossimulator-arm64
expect_failure 'does not match the requested RID' "${command[@]}" 1.2.3 "$abi" ios-arm64

printf 'tampered\n' > "$host/input.txt"
expect_failure 'Checksum mismatch' "${command[@]}" 1.2.3 "$abi" iossimulator-arm64
printf 'host input\n' > "$host/input.txt"
printf 'extra\n' > "$host/extra.txt"
expect_failure 'file inventory differs' "${command[@]}" 1.2.3 "$abi" iossimulator-arm64
rm "$host/extra.txt"
ln -s input.txt "$host/link"
expect_failure 'Symbolic links are not allowed' "${command[@]}" 1.2.3 "$abi" iossimulator-arm64
rm "$host/link"

mkdir "$app"
printf 'keep me\n' > "$app/user-data"
expect_failure 'Refusing to replace' "${command[@]}" 1.2.3 "$abi" iossimulator-arm64
test "$(cat "$app/user-data")" = 'keep me'
printf 'rustolonia-ios-app-v1\n' > "$app/.rustolonia-ios-owner"
expect_failure 'Cannot read Info.plist' "${command[@]}" 1.2.3 "$abi" iossimulator-arm64
test "$(cat "$app/user-data")" = 'keep me'

if [ "$(uname -s)" = Darwin ]; then
    mkdir -p "$host/native" "$host/bundle/Frameworks/Fake.framework" "$host/link" "$host/licenses"
    for file in entry.o registrar.o host.o libbootstrapperdll.o libRuntime.WorkstationGC.a \
        libaotminipal.a libeventpipe-disabled.a libstandalonegc-disabled.a libxamarin-dotnet-nativeaot.a; do
        : > "$host/native/$file"
    done
    for file in LICENSE licence.md THIRD-PARTY-NOTICES.txt; do
        printf 'notice\n' > "$host/$file"
    done
    : > "$host/licenses/dependency.txt"
    : > "$host/bundle/runtimeconfig.bin"
    : > "$host/bundle/Frameworks/Fake.framework/Fake"
    : > "$host/link/dylibs.txt"
    printf 'strong Foundation\n' > "$host/link/frameworks.txt"
    printf '_test_symbol\n' > "$host/link/symbols.txt"
    python3 - "$host" "$scratch/Info.plist" <<'PY'
import hashlib
import json
import plistlib
from pathlib import Path
import sys

root = Path(sys.argv[1])
manifest_path = root / "host-manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
manifest["files"] = [
    {"name": path.relative_to(root).as_posix(), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    for path in sorted(root.rglob("*")) if path.is_file() and path != manifest_path
]
manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
with open(sys.argv[2], "wb") as target:
    plistlib.dump({"CFBundleExecutable": "Test", "CFBundleIdentifier": "org.rustolonia.test",
                   "CFBundleName": "Test", "CFBundleVersion": "1",
                   "CFBundleShortVersionString": "1.0", "CFBundlePackageType": "APPL"}, target)
PY
    fake_bin="$scratch/bin"
    mkdir "$fake_bin"
    cat > "$fake_bin/xcrun" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
shift 2
if [ "$1" = --show-sdk-path ]; then
    echo /tmp
    exit
fi
if [ "$1" != clang++ ]; then
    echo "Unexpected xcrun invocation: $*" >&2
    exit 1
fi
shift
while [ "$#" -gt 0 ]; do
    if [ "$1" = -o ]; then shift; executable="$1"; fi
    shift
done
: > "$executable"
if [ -n "${RUSTOLONIA_TEST_RACE_OUTPUT:-}" ]; then
    mkdir "$RUSTOLONIA_TEST_RACE_OUTPUT"
    printf 'concurrent data\n' > "$RUSTOLONIA_TEST_RACE_OUTPUT/user-data"
fi
SH
    cat > "$fake_bin/ditto" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$2"
cp -R "$1/." "$2/"
SH
    cat > "$fake_bin/codesign" <<'SH'
#!/usr/bin/env bash
exit 0
SH
    cat > "$fake_bin/xcode-select" <<'SH'
#!/usr/bin/env bash
echo /tmp
SH
    chmod +x "$fake_bin/xcrun" "$fake_bin/ditto" "$fake_bin/codesign" "$fake_bin/xcode-select"
    PATH="$fake_bin:$PATH" "${command[@]}" 1.2.3 "$abi" iossimulator-arm64 >/dev/null
    test -f "$app/Test"
    mv "$app" "$scratch/owned.app"
    expect_failure 'Refusing to replace' env PATH="$fake_bin:$PATH" RUSTOLONIA_TEST_RACE_OUTPUT="$app" \
        "${command[@]}" 1.2.3 "$abi" iossimulator-arm64
    test "$(cat "$app/user-data")" = 'concurrent data'
    test -f "$scratch/owned.app/Test"
fi

echo 'iOS host validation regressions passed.'

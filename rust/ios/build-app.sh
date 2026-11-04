#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 6 ]; then
    echo "Usage: bash build-app.sh <rust-staticlib.a> <Info.plist> <output.app> <rustolonia-version> <abi-sha256> <rid>" >&2
    exit 1
fi

host_dir="$(cd "$(dirname "$0")" && pwd -P)"
rust_library="$1"
source_plist="$2"
output="$3"
expected_version="$4"
expected_abi="$5"
expected_rid="$6"

if [ ! -f "$rust_library" ] || [ ! -f "$source_plist" ]; then
    echo "The Rust static library and Info.plist must both exist." >&2
    exit 1
fi
case "$output" in
    *.app) ;;
    *) echo "The output path must end in .app." >&2; exit 1 ;;
esac

rid="$(python3 - "$host_dir" "$expected_version" "$expected_abi" "$expected_rid" <<'PY'
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys

root = Path(sys.argv[1]).resolve()
try:
    manifest = json.loads((root / "host-manifest.json").read_text(encoding="utf-8"))
except (OSError, ValueError) as error:
    raise SystemExit(f"Cannot read the iOS host manifest: {error}")

if manifest.get("schemaVersion") != 1 or manifest.get("kind") != "rustolonia-ios-host":
    raise SystemExit("Unsupported iOS host manifest.")
if manifest.get("version") != sys.argv[2]:
    raise SystemExit(f"Host version {manifest.get('version')!r} does not match rustolonia-sys {sys.argv[2]!r}.")
if not re.fullmatch(r"[0-9a-f]{64}", sys.argv[3]) or manifest.get("abiFingerprint") != sys.argv[3]:
    raise SystemExit("The host ABI fingerprint does not match rustolonia-sys.")
rid = manifest.get("rid")
if rid not in {"ios-arm64", "iossimulator-arm64", "iossimulator-x64"}:
    raise SystemExit(f"Unsupported iOS host RID: {rid!r}.")
if rid != sys.argv[4]:
    raise SystemExit(f"Host RID {rid!r} does not match the requested RID {sys.argv[4]!r}.")

entries = manifest.get("files")
if not isinstance(entries, list) or not entries:
    raise SystemExit("The iOS host manifest has no file inventory.")
expected = set()
for entry in entries:
    if not isinstance(entry, dict):
        raise SystemExit("Invalid iOS host file inventory entry.")
    name, digest = entry.get("name"), entry.get("sha256")
    if not isinstance(name, str) or not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise SystemExit("Invalid iOS host file name or checksum.")
    relative = PurePosixPath(name)
    if relative.is_absolute() or any(part in {"", ".", ".."} for part in relative.parts) or "\\" in name:
        raise SystemExit(f"Invalid iOS host file path: {name!r}.")
    if name in expected or name == "host-manifest.json":
        raise SystemExit(f"Duplicate or reserved iOS host file: {name!r}.")
    expected.add(name)
    path = root.joinpath(*relative.parts)
    if any(root.joinpath(*relative.parts[:index]).is_symlink() for index in range(1, len(relative.parts) + 1)):
        raise SystemExit(f"Symbolic links are not allowed in the iOS host: {name!r}.")
    try:
        hasher = hashlib.sha256()
        with path.open("rb") as source:
            for block in iter(lambda: source.read(1024 * 1024), b""):
                hasher.update(block)
    except OSError as error:
        raise SystemExit(f"Cannot read iOS host file {name!r}: {error}")
    if hasher.hexdigest() != digest:
        raise SystemExit(f"Checksum mismatch for iOS host file {name!r}.")

actual = set()
for directory, dirs, files in os.walk(root, followlinks=False):
    for name in dirs:
        if (Path(directory) / name).is_symlink():
            raise SystemExit(f"Symbolic links are not allowed in the iOS host: {name!r}.")
    for name in files:
        path = Path(directory) / name
        if path.is_symlink():
            raise SystemExit(f"Symbolic links are not allowed in the iOS host: {path}.")
        relative = path.relative_to(root).as_posix()
        if relative != "host-manifest.json":
            actual.add(relative)
if actual != expected:
    raise SystemExit(f"iOS host file inventory differs: missing={sorted(expected - actual)}, extra={sorted(actual - expected)}")
print(rid)
PY
)"

case "$rid" in
    ios-arm64) sdk_name=iphoneos; arch=arm64; minimum_flag=-miphoneos-version-min ;;
    iossimulator-arm64) sdk_name=iphonesimulator; arch=arm64; minimum_flag=-mios-simulator-version-min ;;
    iossimulator-x64) sdk_name=iphonesimulator; arch=x86_64; minimum_flag=-mios-simulator-version-min ;;
esac

output_parent="$(dirname "$output")"
output_name="$(basename "$output")"
mkdir -p "$output_parent"
output_parent="$(cd "$output_parent" && pwd -P)"
output="$output_parent/$output_name"
is_owned_app() {
    local path="$1"
    [ ! -L "$path" ] && [ -d "$path" ] && [ ! -L "$path/.rustolonia-ios-owner" ] &&
        [ -f "$path/.rustolonia-ios-owner" ] &&
        [ "$(cat "$path/.rustolonia-ios-owner")" = "rustolonia-ios-app-v1" ]
}
assert_safe_output() {
    if { [ -e "$output" ] || [ -L "$output" ]; } && ! is_owned_app "$output"; then
        echo "Refusing to replace an app not created by this script: $output" >&2
        return 1
    fi
}
rename_exclusive() {
    python3 - "$1" "$2" <<'PY'
import ctypes
import os
import sys

libc = ctypes.CDLL(None, use_errno=True)
rename = libc.renamex_np
rename.argtypes = (ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint)
rename.restype = ctypes.c_int
if rename(os.fsencode(sys.argv[1]), os.fsencode(sys.argv[2]), 4) != 0:
    error = ctypes.get_errno()
    raise OSError(error, os.strerror(error), sys.argv[2])
PY
}
assert_safe_output

stage="$(mktemp -d "$output_parent/.rustolonia-ios-stage.XXXXXXXX")"
previous="$stage/previous.app"
cleanup() {
    local status=$?
    trap - EXIT
    if { [ -e "$previous" ] || [ -L "$previous" ]; } && [ "$status" -ne 0 ]; then
        if ! rename_exclusive "$previous" "$output"; then
            echo "The previous app was retained for recovery at $previous" >&2
            exit "$status"
        fi
    fi
    rm -rf "$stage"
    exit "$status"
}
trap cleanup EXIT

app="$stage/$output_name"
mkdir -p "$app/Frameworks" "$app/Licenses"
plist_info="$(python3 - "$source_plist" "$app/Info.plist" "$sdk_name" <<'PY'
import plistlib
import re
import sys

try:
    with open(sys.argv[1], "rb") as source:
        info = plistlib.load(source)
except (OSError, ValueError, plistlib.InvalidFileException) as error:
    raise SystemExit(f"Cannot read Info.plist: {error}")
if not isinstance(info, dict):
    raise SystemExit("Info.plist must contain a dictionary.")
for key in ("CFBundleExecutable", "CFBundleIdentifier", "CFBundleName",
            "CFBundleVersion", "CFBundleShortVersionString"):
    if not isinstance(info.get(key), str) or not info[key]:
        raise SystemExit(f"Info.plist must contain a nonempty {key}.")
executable = info["CFBundleExecutable"]
if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", executable):
    raise SystemExit("CFBundleExecutable must be a simple file name.")
if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.-]*", info["CFBundleIdentifier"]):
    raise SystemExit("CFBundleIdentifier contains unsupported characters.")
if info.get("CFBundlePackageType") != "APPL":
    raise SystemExit("CFBundlePackageType must be APPL.")
minimum = info.get("MinimumOSVersion", "13.0")
if not isinstance(minimum, str) or not re.fullmatch(r"\d+\.\d+(?:\.\d+)?", minimum):
    raise SystemExit("MinimumOSVersion must be an iOS version such as 13.0.")
parts = tuple(int(part) for part in minimum.split("."))
if parts + (0,) * (3 - len(parts)) < (13, 0, 0):
    raise SystemExit("The prebuilt host requires iOS 13.0 or later.")
info["MinimumOSVersion"] = minimum
info["CFBundleSupportedPlatforms"] = ["iPhoneSimulator" if sys.argv[3] == "iphonesimulator" else "iPhoneOS"]
with open(sys.argv[2], "wb") as target:
    plistlib.dump(info, target, sort_keys=False)
print(executable)
print(minimum)
PY
)"
executable="${plist_info%%$'\n'*}"
minimum="${plist_info#*$'\n'}"

ditto "$host_dir/bundle/Frameworks" "$app/Frameworks"
cp "$host_dir/bundle/runtimeconfig.bin" "$app/runtimeconfig.bin"
cp "$host_dir/LICENSE" "$app/Licenses/Rustolonia-LICENSE.txt"
cp "$host_dir/licence.md" "$app/Licenses/Avalonia-LICENSE.md"
cp "$host_dir/THIRD-PARTY-NOTICES.txt" "$app/Licenses/THIRD-PARTY-NOTICES.txt"
ditto "$host_dir/licenses" "$app/Licenses/Dependencies"
printf 'rustolonia-ios-app-v1\n' > "$app/.rustolonia-ios-owner"

sdk="$(xcrun --sdk "$sdk_name" --show-sdk-path)"
developer="${DEVELOPER_DIR:-$(xcode-select -p)}"
case "$developer" in
    *.app) developer="$developer/Contents/Developer" ;;
esac
objects=(
    "$host_dir/native/entry.o"
    "$host_dir/native/registrar.o"
    "$host_dir/native/host.o"
    "$host_dir/native/libbootstrapperdll.o"
)
libraries=(
    "$host_dir/native/libRuntime.WorkstationGC.a"
    "$host_dir/native/libaotminipal.a"
    "$host_dir/native/libeventpipe-disabled.a"
    "$host_dir/native/libstandalonegc-disabled.a"
)
if [ "$arch" = x86_64 ]; then
    libraries+=("$host_dir/native/libRuntime.VxsortEnabled.a")
fi
if [ "$sdk_name" = iphoneos ]; then
    for name in \
        libSystem.Globalization.Native.a libSystem.IO.Compression.Native.a \
        libSystem.Native.a libSystem.Net.Security.Native.a \
        libSystem.Security.Cryptography.Native.Apple.a \
        libbrotlienc.a libbrotlidec.a libbrotlicommon.a; do
        libraries+=("$host_dir/native/$name")
    done
else
    while IFS= read -r name; do
        [ -n "$name" ] || continue
        if [[ ! "$name" =~ ^lib[A-Za-z0-9._-]+\.dylib$ ]]; then
            echo "Invalid host dynamic-library name: $name" >&2
            exit 1
        fi
        cp "$host_dir/bundle/$name" "$app/$name"
        libraries+=("$host_dir/bundle/$name")
    done < "$host_dir/link/dylibs.txt"
fi

frameworks=()
while IFS=' ' read -r strength name extra; do
    [ -n "$strength" ] || continue
    if [[ ! "$name" =~ ^[A-Za-z0-9._-]+$ ]] || [ -n "${extra:-}" ]; then
        echo "Invalid iOS host framework entry: $strength $name ${extra:-}" >&2
        exit 1
    fi
    case "$strength" in
        strong) frameworks+=(-framework "$name") ;;
        weak) frameworks+=(-weak_framework "$name") ;;
        *) echo "Invalid iOS host framework strength: $strength" >&2; exit 1 ;;
    esac
done < "$host_dir/link/frameworks.txt"

symbols=()
while IFS= read -r symbol; do
    [ -n "$symbol" ] || continue
    if [[ ! "$symbol" =~ ^_[A-Za-z0-9_\$]+$ ]]; then
        echo "Invalid iOS host linker symbol: $symbol" >&2
        exit 1
    fi
    symbols+=(-Xlinker -u -Xlinker "$symbol")
done < "$host_dir/link/symbols.txt"

xcrun --sdk "$sdk_name" clang++ -arch "$arch" -isysroot "$sdk" \
    "$minimum_flag=$minimum" -ObjC -Wl,-dead_strip \
    -Xlinker -rpath -Xlinker @executable_path/Frameworks \
    -Xlinker -rpath -Xlinker /usr/lib/swift \
    "${objects[@]}" -Xlinker -force_load -Xlinker "$rust_library" \
    -Xlinker -force_load -Xlinker "$host_dir/native/libxamarin-dotnet-nativeaot.a" \
    "${libraries[@]}" -F "$host_dir/bundle/Frameworks" "${frameworks[@]}" "${symbols[@]}" \
    -L "$developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/$sdk_name" \
    -L "$sdk/usr/lib/swift" -lz -liconv -lcompression -licucore -lobjc \
    -o "$app/$executable"

if [ "$sdk_name" = iphonesimulator ]; then
    for framework in "$app"/Frameworks/*.framework; do
        [ -d "$framework" ] || continue
        codesign --force --sign - --timestamp=none "$framework" >/dev/null
    done
    for dylib in "$app"/*.dylib; do
        [ -f "$dylib" ] || continue
        codesign --force --sign - --timestamp=none "$dylib" >/dev/null
    done
    codesign --force --sign - --timestamp=none "$app" >/dev/null
fi

assert_safe_output
if [ -e "$output" ] || [ -L "$output" ]; then
    rename_exclusive "$output" "$previous"
    if ! is_owned_app "$previous"; then
        echo "The output changed during the build; refusing to replace it." >&2
        exit 1
    fi
fi
rename_exclusive "$app" "$output"
printf 'iOS app: %s\n' "$output"
if [ "$sdk_name" = iphoneos ]; then
    echo "The device app still requires Apple signing and provisioning."
fi

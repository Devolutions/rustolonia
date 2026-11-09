#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <linux-x64|linux-arm64|linux-musl-x64|linux-musl-arm64> <new-build-directory>" >&2
    exit 1
fi

rid=$1
workspace=$2
case "$rid:$(uname -m)" in
    linux-x64:x86_64) cpu=x64 ;;
    linux-arm64:aarch64) cpu=arm64 ;;
    linux-musl-x64:x86_64) cpu=x64 ;;
    linux-musl-arm64:aarch64) cpu=arm64 ;;
    *) echo "Build $rid on a native Linux machine of the matching architecture." >&2; exit 1 ;;
esac
if [ "$(uname -s)" != Linux ]; then
    echo 'Static Linux graphics must be built on Linux.' >&2
    exit 1
fi
for tool in git python3 gn ninja clang clang++ sha256sum nm getconf; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Static Linux graphics build requires $tool." >&2
        exit 1
    fi
done
case "$rid" in
    linux-musl-*)
        if [ ! -f /etc/alpine-release ] || ! command -v apk >/dev/null 2>&1; then
            echo 'Build static Linux musl graphics on native Alpine Linux.' >&2
            exit 1
        fi
        ;;
    *)
        if [ "$(getconf GNU_LIBC_VERSION)" != 'glibc 2.35' ]; then
            echo 'Build static Linux graphics on the supported glibc 2.35 baseline (Ubuntu 22.04).' >&2
            exit 1
        fi
        ;;
esac
jobs=${RUSTOLONIA_STATIC_BUILD_JOBS:-$(getconf _NPROCESSORS_ONLN)}
case "$jobs" in
    ''|0|*[!0-9]*) echo "Invalid RUSTOLONIA_STATIC_BUILD_JOBS: $jobs" >&2; exit 1 ;;
esac

mkdir -p "$workspace"
workspace=$(cd "$workspace" && pwd -P)
source_dir="$workspace/src_skiasharp"
build_dir="$workspace/out/$rid"
if [ -e "$source_dir" ] || [ -e "$build_dir" ]; then
    echo "Build directory already contains source or output: $workspace" >&2
    exit 1
fi
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
skiasharp_commit=f568ac94dd768ef9a2f593537cfde2dd0d348ef5
skia_commit=7dbfc07dd33181f84e0958afb7ee805c6c769f0b

git clone --quiet --depth 1 --branch v3.119.4 https://github.com/mono/SkiaSharp.git "$source_dir"
if [ "$(git -C "$source_dir" rev-parse HEAD)" != "$skiasharp_commit" ]; then
    echo 'The SkiaSharp tag does not match the pinned commit.' >&2
    exit 1
fi
git -C "$source_dir" submodule update --init --depth 1 externals/skia
skia_dir="$source_dir/externals/skia"
if [ "$(git -C "$skia_dir" rev-parse HEAD)" != "$skia_commit" ]; then
    echo 'The Skia submodule does not match the pinned commit.' >&2
    exit 1
fi
(cd "$skia_dir" && python3 tools/git-sync-deps)
case "$rid" in
    linux-musl-*)
        dng_dir="$skia_dir/third_party/externals/dng_sdk"
        git -C "$dng_dir" apply --check "$script_dir/static-skia-musl-dng.patch"
        git -C "$dng_dir" apply "$script_dir/static-skia-musl-dng.patch"
        ;;
esac
mkdir -p "$build_dir"
cat "$script_dir/static-skia-linux.gn" > "$build_dir/args.gn"
printf '\ntarget_cpu = "%s"\n' "$cpu" >> "$build_dir/args.gn"
(cd "$skia_dir" && gn gen "$build_dir")
ninja -C "$build_dir" -j "$jobs" skia SkiaSharp HarfBuzzSharp
sh "$script_dir/collect-linux-graphics-notices.sh" "$source_dir" "$build_dir/STATIC-SKIASHARP-SOURCE-NOTICES.txt"

for archive in libSkiaSharp.a libskia.a libHarfBuzzSharp.a; do
    if [ ! -f "$build_dir/$archive" ]; then
        echo "Static graphics build did not produce $archive." >&2
        exit 1
    fi
    if ! symbols=$(nm -u "$build_dir/$archive" 2>&1); then
        echo "Cannot inspect undefined symbols in $archive: $symbols" >&2
        exit 1
    fi
    if printf '%s\n' "$symbols" | grep -q '__isoc23_'; then
        echo "$archive references glibc symbols newer than the supported baseline." >&2
        exit 1
    fi
done
hash() { sha256sum "$1" | cut -d ' ' -f 1; }
case "$rid" in
    linux-musl-*) libc_provenance="\"libc\": \"musl\", \"dngPatchSha256\": \"$(hash "$script_dir/static-skia-musl-dng.patch")\"" ;;
    *) libc_provenance='"glibcBuild": "2.35"' ;;
esac
cat > "$build_dir/STATIC-SKIASHARP-PROVENANCE.json" <<EOF
{
  "component": "skiasharp",
  "version": "3.119.4",
  "rid": "$rid",
  "skiasharpCommit": "$skiasharp_commit",
  "skiaCommit": "$skia_commit",
  $libc_provenance,
  "skiaDepsSha256": "$(hash "$skia_dir/DEPS")",
  "gnArgsSha256": "$(hash "$build_dir/args.gn")",
  "noticesSha256": "$(hash "$build_dir/STATIC-SKIASHARP-SOURCE-NOTICES.txt")",
  "archives": {
    "libSkiaSharp.a": "$(hash "$build_dir/libSkiaSharp.a")",
    "libskia.a": "$(hash "$build_dir/libskia.a")",
    "libHarfBuzzSharp.a": "$(hash "$build_dir/libHarfBuzzSharp.a")"
  }
}
EOF
echo "Static Linux graphics are ready in $build_dir."

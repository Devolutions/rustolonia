#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <new-build-directory>" >&2
    exit 1
fi
case "$(cat /etc/alpine-release 2>/dev/null || true)" in
    3.24.*) ;;
    *) echo 'Build the static musl X11 dependencies on Alpine 3.24.' >&2; exit 1 ;;
esac
for tool in awk curl git meson ninja pkg-config python3 sha256sum sha512sum tar; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Static musl X11 build requires $tool." >&2
        exit 1
    fi
done
workspace=$1
mkdir -p "$workspace"
workspace=$(cd "$workspace" && pwd -P)
prefix="$workspace/x11"
if [ -e "$prefix" ] || [ -e "$workspace/build" ] || [ -e "$workspace/source" ]; then
    echo "Build directory already contains X11 source or output: $workspace" >&2
    exit 1
fi
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
musl_source_manifest="$script_dir/linux-musl-static-sources.tsv"
RUSTOLONIA_MUSL_SOURCE_CACHE=${RUSTOLONIA_MUSL_SOURCE_CACHE:-"$workspace/sources"}
export RUSTOLONIA_MUSL_SOURCE_CACHE
. "$script_dir/linux-musl-static-source.sh"
mkdir -p "$workspace/source" "$workspace/build" "$prefix"
provenance="$prefix/RUSTOLONIA-X11-PROVENANCE.tsv"
printf '# archive\tsha256\tsource\tsha512\n' > "$provenance"
export PKG_CONFIG_PATH="$prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

for library in libXau.a libXrandr.a; do
    row=$(musl_source_row "$library") || { echo "Missing source pin for $library." >&2; exit 1; }
    set -- $row
    source_name=$3
    source_hash=$4
    source_archive=$(fetch_musl_source "$source_name" "$source_hash")
    source_dir="$workspace/source/${source_name%.tar.xz}"
    tar -xf "$source_archive" -C "$workspace/source"
    build_dir="$workspace/build/${source_name%.tar.xz}"
    meson setup "$build_dir" "$source_dir" --prefix "$prefix" --libdir lib \
        --buildtype release --default-library static -Db_staticpic=true
    ninja -C "$build_dir" install
    archive="$prefix/lib/$library"
    if [ ! -s "$archive" ]; then
        echo "The X11 source build did not produce $archive." >&2
        exit 1
    fi
    printf '%s\t%s\t%s\t%s\n' "$library" "$(sha256sum "$archive" | cut -d ' ' -f 1)" \
        "$source_name" "$source_hash" >> "$provenance"
done
echo "Static X11 source libraries are ready in $prefix."

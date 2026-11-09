#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <nativeaot-publish-directory> <linux-musl-x64|linux-musl-arm64>" >&2
    exit 1
fi
publish=$1
rid=$2
case "$rid:$(uname -s):$(uname -m)" in
    linux-musl-x64:Linux:x86_64|linux-musl-arm64:Linux:aarch64) ;;
    *) echo "Package the $rid static host on native Linux with the matching architecture." >&2; exit 1 ;;
esac
case "$(cat /etc/alpine-release 2>/dev/null || true)" in
    3.24.*) ;;
    *) echo 'Package Linux musl static hosts on Alpine 3.24.' >&2; exit 1 ;;
esac
for tool in apk awk cc curl pkg-config python3 readlink sha256sum sha512sum tar; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Static musl host packaging requires $tool." >&2
        exit 1
    fi
done
if [ -z "${RUSTOLONIA_MUSL_X11_DIR:-}" ] || [ ! -f "$RUSTOLONIA_MUSL_X11_DIR/RUSTOLONIA-X11-PROVENANCE.tsv" ]; then
    echo 'Set RUSTOLONIA_MUSL_X11_DIR to the output of rust/build-linux-musl-x11.sh.' >&2
    exit 1
fi
prefix=$(cd "$RUSTOLONIA_MUSL_X11_DIR" && pwd -P)
RUSTOLONIA_MUSL_SOURCE_CACHE=${RUSTOLONIA_MUSL_SOURCE_CACHE:-"$(dirname "$prefix")/sources"}
export RUSTOLONIA_MUSL_SOURCE_CACHE
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
musl_source_manifest="$script_dir/linux-musl-static-sources.tsv"
. "$script_dir/linux-musl-static-source.sh"
export PKG_CONFIG_PATH="$prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
flags=$(pkg-config --static --libs-only-l xrandr xi xcursor xfixes sm ice fontconfig freetype2)
arch=$(apk --print-arch)
link_file="$publish/rustolonia-linux-static-link.txt"
notices="$publish/STATIC-LINUX-NOTICES.txt"
provenance="$publish/STATIC-LINUX-PROVENANCE.tsv"
: > "$link_file"
printf '# archive\tpackage\tversion\tsha256\tdistribution\n' > "$provenance"
seen_libraries=' '
seen_sources=' '

stage_library() {
    name=$1
    case "$name" in
        ''|*[!a-zA-Z0-9_+.-]*) echo "Invalid static library name: $name" >&2; exit 1 ;;
    esac
    case "$seen_libraries" in *" $name "*) return ;; esac
    seen_libraries="$seen_libraries$name "
    file="lib$name.a"
    row=$(musl_source_row "$file") || { echo "No pinned musl source for $file." >&2; exit 1; }
    set -- $row
    owner=$1
    version=$2
    source_name=$3
    source_hash=$4

    case "$owner" in
        xorg-*)
            archive="$prefix/lib/$file"
            [ -s "$archive" ] || { echo "Missing source-built archive $archive." >&2; exit 1; }
            hash=$(sha256sum "$archive" | cut -d ' ' -f 1)
            expected=$(printf '%s\t%s\t%s\t%s' "$file" "$hash" "$source_name" "$source_hash")
            if ! grep -Fx "$expected" "$prefix/RUSTOLONIA-X11-PROVENANCE.tsv" >/dev/null; then
                echo "X11 source provenance does not match $file." >&2
                exit 1
            fi
            distribution=xorg
            ;;
        *)
            archive=$(cc -print-file-name="$file")
            if [ ! -f "$archive" ]; then
                echo "Missing $file; install the pinned Alpine static development packages." >&2
                exit 1
            fi
            canonical=$(readlink -f "$archive")
            actual_owner=$(apk info -W -q "$canonical")
            actual_version=$(apk info -e -v "$owner")
            if [ "$actual_owner" != "$owner" ] || [ "$actual_version" != "$owner-$version" ]; then
                echo "Unexpected Alpine package for $file: $actual_owner $actual_version; expected $owner-$version." >&2
                exit 1
            fi
            hash=$(sha256sum "$archive" | cut -d ' ' -f 1)
            distribution=alpine
            ;;
    esac
    case "$seen_sources" in
        *" $source_name "*) ;;
        *)
            seen_sources="$seen_sources$source_name "
            fetch_musl_source "$source_name" "$source_hash" >/dev/null
            ;;
    esac
    cp "$archive" "$publish/$file"
    printf 'static %s\n' "$file" >> "$link_file"
    printf '%s\t%s:%s\t%s\t%s\t%s\n' "$file" "$owner" "$arch" "$version" "$hash" "$distribution" >> "$provenance"
}

for flag in $flags; do
    case "$flag" in
        -l*) name=${flag#-l} ;;
        *) echo "Unexpected pkg-config library flag: $flag" >&2; exit 1 ;;
    esac
    case "$name" in
        c|m|dl|pthread|z|brotlienc|brotlidec|brotlicommon) ;;
        *) stage_library "$name" ;;
    esac
done
for name in bsd md ssl crypto; do
    stage_library "$name"
done
# NativeAOT's compression archives must follow their consumers.
for name in brotlienc z brotlidec brotlicommon; do
    if [ ! -f "$publish/lib$name.a" ]; then
        echo "NativeAOT did not supply lib$name.a." >&2
        exit 1
    fi
    printf 'static lib%s.a\n' "$name" >> "$link_file"
done
for name in stdc++ gcc gcc_eh; do
    stage_library "$name"
done
python3 "$script_dir/collect-linux-musl-notices.py" "$musl_source_manifest" "$provenance" \
    "$RUSTOLONIA_MUSL_SOURCE_CACHE" "$notices"
cp "$musl_source_manifest" "$publish/STATIC-LINUX-MUSL-SOURCES.tsv"
printf '# notices-sha256\t%s\n' "$(sha256sum "$notices" | cut -d ' ' -f 1)" >> "$provenance"
printf '# sources-sha256\t%s\n' "$(sha256sum "$musl_source_manifest" | cut -d ' ' -f 1)" >> "$provenance"

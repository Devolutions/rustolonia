#!/bin/sh

musl_source_row() {
    awk -F '\t' -v archive="$1" '
        $1 == archive { count++; print $2, $3, $4, $5 }
        END { if (count != 1) exit 1 }
    ' "$musl_source_manifest"
}

fetch_musl_source() (
    set -eu
    name=$1
    expected=$2
    case "$name" in
        ''|*[!A-Za-z0-9._+-]*) echo "Invalid source archive name: $name" >&2; exit 1 ;;
    esac
    mkdir -p "$RUSTOLONIA_MUSL_SOURCE_CACHE"
    archive="$RUSTOLONIA_MUSL_SOURCE_CACHE/$name"
    if [ ! -f "$archive" ]; then
        temporary="$archive.partial.$$"
        trap 'rm -f "$temporary"' EXIT
        echo "Downloading pinned musl source $name" >&2
        curl --fail --silent --show-error --location --retry 2 \
            "https://distfiles.alpinelinux.org/distfiles/v3.24/$name" --output "$temporary"
        actual=$(sha512sum "$temporary" | cut -d ' ' -f 1)
        if [ "$actual" != "$expected" ]; then
            echo "Source checksum mismatch for $name." >&2
            exit 1
        fi
        mv "$temporary" "$archive"
    fi
    actual=$(sha512sum "$archive" | cut -d ' ' -f 1)
    if [ "$actual" != "$expected" ]; then
        echo "Source checksum mismatch for $archive." >&2
        exit 1
    fi
    printf '%s\n' "$archive"
)

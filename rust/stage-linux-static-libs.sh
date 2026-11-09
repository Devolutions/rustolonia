#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <nativeaot-publish-directory> <linux-x64|linux-arm64>" >&2
    exit 1
fi

publish=$1
rid=$2
case "$rid:$(uname -s):$(uname -m)" in
    linux-x64:Linux:x86_64|linux-arm64:Linux:aarch64) ;;
    *) echo "Package the $rid static host on native Linux with the matching architecture." >&2; exit 1 ;;
esac
link_file="$publish/rustolonia-linux-static-link.txt"
notices="$publish/STATIC-LINUX-NOTICES.txt"
provenance="$publish/STATIC-LINUX-PROVENANCE.tsv"

for tool in cc pkg-config dpkg-query sha256sum readlink getconf; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Static Linux host packaging requires $tool and Debian/Ubuntu development packages." >&2
        exit 1
    fi
done

if [ "$(getconf GNU_LIBC_VERSION)" != 'glibc 2.35' ]; then
    echo 'Package static Linux hosts on the supported glibc 2.35 baseline (Ubuntu 22.04).' >&2
    exit 1
fi
flags=$(pkg-config --static --libs-only-l fontconfig freetype2)
distribution=$(sed -n 's/^ID=//p' /etc/os-release | tr -d '"')
case "$distribution" in
    debian|ubuntu) ;;
    *) echo "Static Linux host packaging requires Debian or Ubuntu, not $distribution." >&2; exit 1 ;;
esac
: > "$link_file"
: > "$notices"
printf '# archive\tpackage\tversion\tsha256\tdistribution\n' > "$provenance"
seen_libraries=' '
seen_packages=' '

stage_library() {
    name=$1
    case "$name" in
        ''|*[!a-zA-Z0-9_+.-]*)
            echo "Invalid static library name from pkg-config: $name" >&2
            exit 1
            ;;
    esac
    case "$seen_libraries" in *" $name "*) return ;; esac
    seen_libraries="$seen_libraries$name "

    file="lib$name.a"
    archive=$(cc -print-file-name="$file")
    if [ ! -f "$archive" ]; then
        echo "Missing $file; install the static development libraries required by fontconfig, FreeType, and GCC." >&2
        exit 1
    fi
    canonical=$(readlink -f "$archive")
    owner=$(dpkg-query -S "$canonical" | sed -n '1s/: .*//p')
    if [ -z "$owner" ]; then
        echo "Cannot identify the Debian package that supplies $canonical." >&2
        exit 1
    fi
    package=${owner%%:*}
    version=$(dpkg-query -W -f='${Version}' "$owner")
    notice="/usr/share/doc/$package/copyright"
    if [ -z "$version" ] || [ ! -f "$notice" ]; then
        echo "Missing package version or copyright notice for $owner." >&2
        exit 1
    fi

    cp "$archive" "$publish/$file"
    printf 'static %s\n' "$file" >> "$link_file"
    hash=$(sha256sum "$archive" | cut -d ' ' -f 1)
    printf '%s\t%s\t%s\t%s\t%s\n' "$file" "$owner" "$version" "$hash" "$distribution" >> "$provenance"
    case "$seen_packages" in
        *" $owner "*) ;;
        *)
            seen_packages="$seen_packages$owner "
            printf '===== %s %s =====\n' "$owner" "$version" >> "$notices"
            cat "$notice" >> "$notices"
            printf '\n\n' >> "$notices"
            ;;
    esac
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

# The NativeAOT runtime supplies zlib and Brotli; put them after their
# consumers so one-pass Unix linkers can resolve the static archives.
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

if [ ! -d /usr/share/common-licenses ]; then
    echo 'Missing Debian common license texts.' >&2
    exit 1
fi
for notice in /usr/share/common-licenses/*; do
    if [ -f "$notice" ]; then
        printf '===== %s =====\n' "$notice" >> "$notices"
        cat "$notice" >> "$notices"
        printf '\n\n' >> "$notices"
    fi
done

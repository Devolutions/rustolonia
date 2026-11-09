#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <linux-musl-x64|linux-musl-arm64> <new-build-directory>" >&2
    exit 1
fi

rid=$1
workspace=$2
case "$rid:$(uname -m)" in
    linux-musl-x64:x86_64)
        sdk_arch=x64
        sdk_hash=7c85806aeb15b5c02e282234532ac599f69409e45ba4ef09c13e848441d3a092c4e030efaaecbd271896c4137429c90daf3f44ca9805fa27468cc2323c670ac8
        ;;
    linux-musl-arm64:aarch64)
        sdk_arch=arm64
        sdk_hash=a5a881429e879f0ff01e84a768dbc022e7fed0f4c2fe4936c0bec134fa184d16b42f98e1e43bc1c0fea5e9ce442fe57efc87bdc04023e337f9b0013c849bc2b1
        ;;
    *) echo "Build $rid on a native Linux machine of the matching architecture." >&2; exit 1 ;;
esac
case "$(cat /etc/alpine-release 2>/dev/null || true):$(id -u)" in
    3.24.*:0) ;;
    *) echo 'Run this release builder as root in a disposable Alpine 3.24 container.' >&2; exit 1 ;;
esac

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
repo=$(CDPATH= cd -- "$script_dir/.." && pwd -P)
if [ -e "$workspace" ]; then
    echo "Build directory already exists: $workspace" >&2
    exit 1
fi
mkdir -p "$workspace"
workspace=$(cd "$workspace" && pwd -P)

# Pin every packaged archive to the source/owner versions verified by staging.
awk -F '\t' 'NF == 5 && $1 !~ /^#/ && $2 !~ /^xorg-/ { print $2 "=" $3 }' \
    "$script_dir/linux-musl-static-sources.tsv" | sort -u > "$workspace/apk-pins.txt"
set --
while IFS= read -r pin; do
    set -- "$@" "$pin"
done < "$workspace/apk-pins.txt"
apk add --no-cache "$@" \
    build-base=0.5-r4 clang21=21.1.8-r3 cmake=4.2.3-r0 gn=0_git20260924-r0 \
    meson=1.11.1-r0 samurai=1.2-r8 \
    binutils ca-certificates curl git icu-libs pkgconf python3 xorgproto

mkdir -p "$workspace/bin" "$workspace/dotnet-sdk"
ln -s /usr/bin/clang-21 "$workspace/bin/clang"
ln -s /usr/bin/clang++-21 "$workspace/bin/clang++"
export PATH="$workspace/bin:$workspace/dotnet-sdk:$PATH"
export DOTNET_ROOT="$workspace/dotnet-sdk"
export DOTNET_CLI_HOME="$workspace/dotnet-home"
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
export NUGET_PACKAGES="$workspace/nuget"
export RestoreConfigFile="$repo/avalonia-src/NuGet.Config"

sdk_version=10.0.403
sdk_archive="$workspace/dotnet-sdk.tar.gz"
curl --proto '=https' --tlsv1.2 --fail --location --retry 5 \
    "https://builds.dotnet.microsoft.com/dotnet/Sdk/$sdk_version/dotnet-sdk-$sdk_version-linux-musl-$sdk_arch.tar.gz" \
    --output "$sdk_archive"
printf '%s  %s\n' "$sdk_hash" "$sdk_archive" | sha512sum -c -
tar -xzf "$sdk_archive" -C "$DOTNET_ROOT"
dotnet --version

# The checkout is mounted from the host runner and therefore has a different owner.
git config --global --add safe.directory "$repo"
git config --global --add safe.directory "$repo/avalonia-src"

export RUSTOLONIA_MUSL_SOURCE_CACHE="$workspace/sources"
sh "$script_dir/build-linux-musl-x11.sh" "$workspace/x11-build"
export RUSTOLONIA_MUSL_X11_DIR="$workspace/x11-build/x11"
sh "$script_dir/build-linux-static-graphics.sh" "$rid" "$workspace/graphics"
export RUSTOLONIA_LINUX_STATIC_GRAPHICS_DIR="$workspace/graphics/out/$rid"

tasks="$repo/avalonia-src/src/Avalonia.Build.Tasks/Avalonia.Build.Tasks.csproj"
dotnet restore "$tasks"
dotnet build "$tasks" -c Release --no-restore
dotnet publish "$repo/host/Avalonia.Host.csproj" -c Release -r "$rid" \
    --artifacts-path "$workspace/dotnet" \
    "-p:AvaloniaProducerRoot=$repo/avalonia-src" "-p:RustoloniaRoot=$repo" \
    -p:AvaloniaRustHostPlatform=X11 -p:AvaloniaRustDeveloperTools=false -p:NativeLib=Static

echo "Static musl host is ready in $workspace/dotnet/publish/Avalonia.Host/release_$rid."

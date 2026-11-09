#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <skiasharp-source-directory> <output-file>" >&2
    exit 1
fi

source_dir=$1
output=$2
: > "$output"
for relative in \
    LICENSE.md \
    LICENSE.txt \
    externals/skia/LICENSE \
    externals/skia/third_party/etc1/LICENSE \
    externals/skia/third_party/expat/LICENSE \
    externals/skia/third_party/harfbuzz/LICENSE \
    externals/skia/third_party/vulkanmemoryallocator/LICENSE \
    externals/skia/third_party/wuffs/LICENSE \
    externals/skia/third_party/externals/brotli/LICENSE \
    externals/skia/third_party/externals/dng_sdk/LICENSE \
    externals/skia/third_party/externals/dng_sdk/LICENSE.source_code \
    externals/skia/third_party/externals/dng_sdk/LICENSE.technology \
    externals/skia/third_party/externals/dng_sdk/NOTICE \
    externals/skia/third_party/externals/dng_sdk/PATENTS \
    externals/skia/third_party/externals/expat/COPYING \
    externals/skia/third_party/externals/freetype/LICENSE.TXT \
    externals/skia/third_party/externals/freetype/docs/FTL.TXT \
    externals/skia/third_party/externals/freetype/docs/GPLv2.TXT \
    externals/skia/third_party/externals/harfbuzz/COPYING \
    externals/skia/third_party/externals/libjpeg-turbo/LICENSE.md \
    externals/skia/third_party/externals/libjpeg-turbo/README.ijg \
    externals/skia/third_party/externals/libpng/LICENSE \
    externals/skia/third_party/externals/libwebp/COPYING \
    externals/skia/third_party/externals/libwebp/PATENTS \
    externals/skia/third_party/externals/piex/LICENSE \
    externals/skia/third_party/externals/spirv-cross/LICENSE \
    externals/skia/third_party/externals/spirv-cross/LICENSES/Apache-2.0.txt \
    externals/skia/third_party/externals/spirv-cross/LICENSES/CC-BY-4.0.txt \
    externals/skia/third_party/externals/spirv-cross/LICENSES/LicenseRef-KhronosFreeUse.txt \
    externals/skia/third_party/externals/spirv-cross/LICENSES/MIT.txt \
    externals/skia/third_party/externals/vulkan-headers/LICENSE.md \
    externals/skia/third_party/externals/vulkan-headers/LICENSES/Apache-2.0.txt \
    externals/skia/third_party/externals/vulkan-headers/LICENSES/MIT.txt \
    externals/skia/third_party/externals/vulkanmemoryallocator/LICENSE.txt \
    externals/skia/third_party/externals/wuffs/LICENSE \
    externals/skia/third_party/externals/zlib/LICENSE
do
    path="$source_dir/$relative"
    if [ ! -f "$path" ]; then
        echo "Missing static graphics source notice: $path" >&2
        exit 1
    fi
    printf '===== %s =====\n' "$relative" >> "$output"
    cat "$path" >> "$output"
    printf '\n\n' >> "$output"
done

wuffs="$source_dir/externals/skia/third_party/externals/wuffs/release/c/wuffs-v0.3.c"
if ! grep -q 'Copyright 2017 The Wuffs Authors' "$wuffs"; then
    echo "Missing Wuffs copyright notice: $wuffs" >&2
    exit 1
fi
printf '===== Wuffs copyright =====\nCopyright 2017 The Wuffs Authors.\n' >> "$output"

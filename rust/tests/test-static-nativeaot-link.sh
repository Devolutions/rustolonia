#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

cat > "$scratch/host.c" <<'EOF'
static int module_value = 123;
__attribute__((used, section("__modules")))
void *__Module = &module_value;
EOF
cat > "$scratch/main.c" <<'EOF'
extern void *__start___modules[];
extern void *__stop___modules[];
int main(void)
{
    return (unsigned long)__stop___modules - (unsigned long)__start___modules != sizeof(void *) ||
        *(const int *)__start___modules[0] != 123;
}
EOF

clang -c -O2 -fPIC "$root/host/static-nativeaot-module-anchor.c" -o "$scratch/anchor.o"
clang -c -O2 -fPIC -fdata-sections "$scratch/host.c" -o "$scratch/host.o"
clang -c -O2 -ffunction-sections "$scratch/main.c" -o "$scratch/main.o"
ar crs "$scratch/libbootstrap.a" "$scratch/anchor.o"
ar crs "$scratch/libhost.a" "$scratch/host.o"

host=$(rustc -vV | sed -n 's/^host: //p')
lld_dir="$(rustc --print sysroot)/lib/rustlib/$host/bin/gcc-ld"
if [ ! -x "$lld_dir/ld.lld" ]; then
    echo "The Rust toolchain's LLD linker is required for this regression test." >&2
    exit 1
fi

if ! cc -B "$lld_dir" -fuse-ld=lld "$scratch/main.o" -Wl,--gc-sections \
    -Wl,--whole-archive "$scratch/libhost.a" -Wl,--no-whole-archive \
    -o "$scratch/unanchored" > "$scratch/unanchored.log" 2>&1; then
    if ! grep -q '__start___modules' "$scratch/unanchored.log"; then
        cat "$scratch/unanchored.log" >&2
        exit 1
    fi
fi

for linker in bfd lld; do
    if [ "$linker" = lld ]; then
        set -- -B "$lld_dir" -fuse-ld=lld
    else
        set -- -fuse-ld=bfd
    fi
    cc "$@" "$scratch/main.o" -Wl,--gc-sections \
        -Wl,--whole-archive "$scratch/libbootstrap.a" -Wl,--no-whole-archive \
        "$scratch/libhost.a" -o "$scratch/probe-$linker"
    "$scratch/probe-$linker"
done
echo 'GNU ld and LLD retained and initialized the NativeAOT module section.'

#!/bin/sh
set -eu

scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/src"
cat > "$scratch/Cargo.toml" <<'EOF'
[package]
name = "static-musl-link-probe"
version = "0.1.0"
edition = "2021"

[workspace]
EOF
cat > "$scratch/src/main.rs" <<'EOF'
fn main() {
    println!("static musl executable");
}
EOF
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
RUSTFLAGS="-C target-feature=+crt-static -C relocation-model=static" \
    cargo build --quiet --release --target x86_64-unknown-linux-musl --manifest-path "$scratch/Cargo.toml"
exe="$scratch/target/x86_64-unknown-linux-musl/release/static-musl-link-probe"
readelf -l "$exe" > "$scratch/segments"
readelf -d "$exe" > "$scratch/dynamic"
if grep -q ' INTERP ' "$scratch/segments" || grep -q '(NEEDED)' "$scratch/dynamic"; then
    echo 'The musl-gcc executable has an interpreter or shared-library dependencies.' >&2
    cat "$scratch/segments" "$scratch/dynamic" >&2
    exit 1
fi
"$exe" | grep -q '^static musl executable$'
echo 'musl-gcc produced a runnable executable without an interpreter or shared libraries.'
